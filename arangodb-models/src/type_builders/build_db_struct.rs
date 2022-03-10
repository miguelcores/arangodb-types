use std::collections::HashSet;

use crate::constants::DB_MODEL_TAG;
use proc_macro2::TokenStream;
use quote::quote;

use crate::data::{
    BaseTypeKind, FieldInfo, FieldTypeKind, InnerModelKind, ModelInfo, ModelOptions,
};
use crate::model_builders::{build_db_struct_aql_mapping_impl, build_db_struct_field_list};

pub fn build_db_struct_type(
    options: &ModelOptions,
    info: &ModelInfo,
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let fields_in_db: Vec<_> = info.fields_in_db().collect();
    let struct_tokens = build_struct(options, info, &fields_in_db, imports)?;
    let impl_tokens = if !options.skip_impl {
        build_impl(options, info, &fields_in_db, imports)?
    } else {
        quote! {}
    };

    let field_list_tokens = if !options.skip_fields {
        build_db_struct_field_list(options, info, &fields_in_db, imports)?
    } else {
        quote! {}
    };

    let aql_mapping_impl_tokens =
        build_db_struct_aql_mapping_impl(options, info, true, &fields_in_db, imports)?;

    // Build result.
    Ok(quote! {
        #struct_tokens
        #impl_tokens
        #field_list_tokens
        #aql_mapping_impl_tokens
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_struct(
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_db: &[&FieldInfo],
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let visibility = info.item.visibility();
    let generics = info.item.generics();
    let document_name = &info.document_name;

    let all_fields_are_optional_or_db_properties =
        info.check_all_db_fields_are_optional_or_properties();

    imports.insert("::serde::Deserialize".to_string());
    imports.insert("::serde::Serialize".to_string());

    // Evaluate default attribute.
    let default_attribute =
        if !info.item_attributes.skip_default && all_fields_are_optional_or_db_properties {
            quote! {
                #[derive(Default)]
                #[serde(default)]
            }
        } else {
            quote! {}
        };

    // Evaluate fields.
    let field_list = fields_in_db.iter().map(|field| {
        let node = field.node.as_field().unwrap();
        let visibility = &node.vis;
        let name = field.name();
        let db_name = &field.db_name;
        let field_type = field.build_db_field_type();
        let deserialize_with = field.build_field_deserialize_with(imports);

        let attributes = &field.attributes.attributes;
        let attribute_list = field.attributes.attributes_by_model.get(DB_MODEL_TAG);
        let attributes = if let Some(attribute_list) = attribute_list {
            quote! {
                #(#attributes)*
                #(#attribute_list)*
            }
        } else {
            quote! {
                #(#attributes)*
            }
        };

        quote! {
            #attributes
            #[serde(rename = #db_name)]
            #deserialize_with
            #visibility #name: #field_type,
        }
    });

    let attributes = &info.item_attributes.attributes;
    let attribute_list = info.item_attributes.attributes_by_model.get(DB_MODEL_TAG);
    let attributes = if let Some(attribute_list) = attribute_list {
        quote! {
            #(#attributes)*
            #(#attribute_list)*
        }
    } else {
        quote! {
            #(#attributes)*
        }
    };

    // Build result.
    Ok(quote! {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        #default_attribute
        #attributes
        #visibility struct #document_name #generics {
            #(#field_list)*
        }
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_impl(
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_db: &[&FieldInfo],
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let document_name = &info.document_name;

    let all_fields_are_optional_or_db_properties =
        info.check_all_db_fields_are_optional_or_properties();

    // Evaluate is_all_missing_method_tokens method.
    let is_all_missing_method_tokens = if all_fields_are_optional_or_db_properties {
        let fields = fields_in_db.iter().map(|field| {
            let name = field.name();
            match field.field_type_kind {
                Some(FieldTypeKind::NullableOption) => {
                    quote! {
                        if !self.#name.is_missing() {
                            return false;
                        }
                    }
                }
                Some(FieldTypeKind::Option) => {
                    quote! {
                        if self.#name.is_some() {
                            return false;
                        }
                    }
                }
                None => {
                    unreachable!("Cannot generate is_all_missing for plain fields")
                }
            }
        });

        quote! {
            pub fn is_all_missing(&self) -> bool {
                #(#fields)*

                true
            }
        }
    } else {
        quote! {
            pub fn is_all_missing(&self) -> bool {
                false
            }
        }
    };

    // Evaluate is_all_null_method_tokens method.
    let is_all_null_method_tokens = if all_fields_are_optional_or_db_properties {
        let fields = fields_in_db.iter().map(|field| {
            let name = field.name();
            match field.field_type_kind {
                Some(FieldTypeKind::NullableOption) => {
                    quote! {
                        if !self.#name.is_null() {
                            return false;
                        }
                    }
                }
                Some(FieldTypeKind::Option) => {
                    quote! {
                        if self.#name.is_some() {
                            return false;
                        }
                    }
                }
                None => {
                    unreachable!("Cannot generate is_all_null for plain fields")
                }
            }
        });

        quote! {
            pub fn is_all_null(&self) -> bool {
                #(#fields)*

                true
            }
        }
    } else {
        quote! {
            pub fn is_all_null(&self) -> bool {
                false
            }
        }
    };

    // Evaluate is_all_null_or_missing_method_tokens method.
    let is_all_null_or_missing_method_tokens = if all_fields_are_optional_or_db_properties {
        let fields = fields_in_db.iter().map(|field| {
            let name = field.name();
            match field.field_type_kind {
                Some(FieldTypeKind::NullableOption) => {
                    quote! {
                        if self.#name.is_value() {
                            return false;
                        }
                    }
                }
                Some(FieldTypeKind::Option) => {
                    quote! {
                        if self.#name.is_some() {
                            return false;
                        }
                    }
                }
                None => {
                    unreachable!("Cannot generate is_all_null_or_missing for plain fields")
                }
            }
        });

        quote! {
            pub fn is_all_null_or_missing(&self) -> bool {
                #(#fields)*

                true
            }
        }
    } else {
        quote! {
            pub fn is_all_null_or_missing(&self) -> bool {
                false
            }
        }
    };

    // Evaluate all null method.
    imports.insert("::arangodb_types::types::NullableOption".to_string());

    let all_null_method_tokens = if all_fields_are_optional_or_db_properties {
        let null_field_list = fields_in_db.iter().filter_map(|field| {
            let name = field.name();

            match field.field_type_kind {
                Some(FieldTypeKind::NullableOption) => Some(quote! {
                    #name: NullableOption::Null
                }),
                Some(FieldTypeKind::Option) => Some(quote! {
                    #name: None
                }),
                None => None,
            }
        });

        quote! {
            #[allow(clippy::needless_update)]
            pub fn all_null() -> Self {
                Self {
                    #(#null_field_list,)*
                    ..Default::default()
                }
            }
        }
    } else {
        quote! {}
    };

    // Evaluate map_values_to_null_method method.
    let map_values_to_null_method_tokens = {
        let fields = fields_in_db.iter().filter_map(|field| {
            let name = field.name();

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
                            panic!("Cannot declare a VecDBReference value as Struct or Enum model")
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

        quote! {
            pub fn map_values_to_null(&mut self) {
                #(#fields)*
            }
        }
    };

    // Build result.
    Ok(quote! {
        impl #generics #document_name #generics {
            #is_all_missing_method_tokens
            #is_all_null_method_tokens
            #is_all_null_or_missing_method_tokens
            #all_null_method_tokens
            #map_values_to_null_method_tokens
        }
    })
}
