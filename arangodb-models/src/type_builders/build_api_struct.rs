use proc_macro2::TokenStream;
use quote::quote;

use crate::data::{
    BaseTypeKind, FieldInfo, FieldTypeKind, InnerModelKind, ModelInfo, ModelOptions,
};
use crate::model_builders::{build_api_fields, build_api_struct, build_from_to};

pub fn build_api_struct_type(
    model: &str,
    options: &ModelOptions,
    info: &ModelInfo,
) -> Result<TokenStream, syn::Error> {
    let fields_in_model = info.fields_in_model(model);
    let struct_tokens = build_api_struct(model, options, info, true, &fields_in_model)?;
    let from_to_tokens = build_from_to(model, options, info, true, &fields_in_model)?;
    let impl_tokens = build_impl(model, options, info, &fields_in_model)?;
    let api_fields_tokens = build_api_fields(model, options, info, true, &fields_in_model)?;

    // Build result.
    Ok(quote! {
        #struct_tokens
        #from_to_tokens
        #api_fields_tokens
        #impl_tokens
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_impl(
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_model: &[&FieldInfo],
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let api_document_name = &info.api_document_names.get(model).unwrap();

    // Evaluate map_values_to_null fields.
    let map_to_null_fields = fields_in_model.iter().filter_map(|field| {
        let name = field.name();

        match field.attributes.inner_model {
            InnerModelKind::Data => match field.field_type_kind {
                Some(FieldTypeKind::NullableOption) => Some(quote! {
                    if self.#name.is_value() {
                        self.#name = ::arangodb_types::types::NullableOption::Null;
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
                            if let ::arangodb_types::types::NullableOption::Value(v) = &mut self.#name {
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
