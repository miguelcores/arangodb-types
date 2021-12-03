use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;

use crate::data::{
    BaseTypeKind, FieldInfo, FieldTypeKind, InnerModelKind, ModelInfo, ModelOptions,
};
use crate::model_builders::{
    build_api_fields, build_api_struct, build_api_struct_sensible_info_impl, build_from_to,
};

pub fn build_api_struct_type(
    options: &ModelOptions,
    info: &ModelInfo,
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let fields_in_api: Vec<_> = info.fields_in_api().collect();
    let struct_tokens = build_api_struct(options, info, true, &fields_in_api, imports)?;
    let from_to_tokens = build_from_to(options, info, true, &fields_in_api, imports)?;
    let impl_tokens = build_impl(options, info, &fields_in_api, imports)?;
    let api_fields_tokens = build_api_fields(options, info, true, &fields_in_api, imports)?;
    let sensible_info_impl_tokens =
        build_api_struct_sensible_info_impl(options, info, true, &fields_in_api, imports)?;

    // Build result.
    Ok(quote! {
        #struct_tokens
        #from_to_tokens
        #impl_tokens
        #api_fields_tokens
        #sensible_info_impl_tokens
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_impl(
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_api: &[&FieldInfo],
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let api_document_name = &info.api_document_name;

    imports.insert("::arangodb_types::types::NullableOption".to_string());

    // Evaluate map_to_null fields.
    let map_to_null_fields = fields_in_api.iter().filter_map(|field| {
        let name = field.name();

        if field.attributes.api_skip_map_to_null {
            return None;
        }

        match field.attributes.inner_model {
            InnerModelKind::Data => match field.field_type_kind {
                Some(FieldTypeKind::NullableOption) => Some(quote! {
                    if self.#name.is_value() {
                        self.#name = NullableOption::Null;
                    }
                }),
                Some(FieldTypeKind::Option) => Some(quote! {
                    self.#name = None;
                }),
                None => None,
            },
            InnerModelKind::Struct | InnerModelKind::Enum => {
                let base = match field.base_type_kind {
                    BaseTypeKind::Other | BaseTypeKind::Box => Some(quote! {
                        v.map_values_to_null();
                    }),
                    BaseTypeKind::Vec => Some(quote! {
                        for v in v {
                            v.map_values_to_null();
                        }
                    }),
                    BaseTypeKind::VecDBReference => {
                        panic!("Cannot declare a DBReference value as Struct or Enum model")
                    }
                    BaseTypeKind::HashMap => Some(quote! {
                        for (_, v) in v {
                            v.map_values_to_null();
                        }
                    }),
                    BaseTypeKind::DBReference => {
                        panic!("Cannot declare a DBReference value as Struct or Enum model")
                    }
                };

                match field.field_type_kind {
                    Some(FieldTypeKind::NullableOption) => base.map(|base| {
                        quote! {
                            if let NullableOption::Value(v) = &mut self.#name {
                                #base
                            }
                        }
                    }),
                    Some(FieldTypeKind::Option) => base.map(|base| {
                        quote! {
                            if let Some(v) = &mut self.#name {
                                #base
                            }
                        }
                    }),
                    None => base.map(|base| {
                        quote! {
                            {
                                let v = &mut self.#name;
                                #base
                            }
                        }
                    }),
                }
            }
        }
    });

    // Build result.
    Ok(quote! {
        impl #generics #api_document_name #generics {
            pub fn map_values_to_null(&mut self) {
                #(#map_to_null_fields)*
            }
        }
    })
}
