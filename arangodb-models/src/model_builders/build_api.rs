use std::collections::HashSet;

use crate::constants::DB_MODEL_TAG;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::{quote, ToTokens};

use crate::data::{
    BaseTypeKind, FieldInfo, FieldTypeKind, InnerModelKind, ModelInfo, ModelOptions,
};
use crate::utils::from_snake_case_to_pascal_case;

pub fn build_api_model(
    model: &str,
    options: &ModelOptions,
    info: &ModelInfo,
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let fields_in_model = info.fields_in_model(model);
    let struct_tokens = build_api_struct(model, options, info, false, &fields_in_model, imports)?;
    let from_to_tokens = build_from_to(model, options, info, false, &fields_in_model, imports)?;
    let api_fields_tokens =
        build_api_fields(model, options, info, false, &fields_in_model, imports)?;

    let impl_tokens = if !options.skip_impl {
        build_api_document_impl(model, options, info, &fields_in_model, imports)?
    } else {
        quote! {}
    };

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

pub fn build_api_struct(
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    is_sub_model: bool,
    fields_in_model: &[&FieldInfo],
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let visibility = info.item.visibility();
    let generics = info.item.generics();
    let api_document_name = &info.api_document_names.get(model).unwrap();

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
    let field_list = fields_in_model.iter().map(|field| {
        let node = field.node.as_field().unwrap();
        let visibility = &node.vis;
        let name = field.name();
        let field_type = field.build_api_field_type(model);
        let deserialize_with = field.build_field_deserialize_with(imports);

        let attributes = &field.attributes.attributes;
        let attribute_list = field.attributes.attributes_by_model.get(model);
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
            #deserialize_with
            #visibility #name: #field_type,
        }
    });

    // Id field.
    let id_field = if !is_sub_model {
        let field = info.get_key_field().unwrap();
        let node = field.node.as_field().unwrap();
        let visibility = &node.vis;
        let field_type = field.build_api_field_type(model);

        let attributes = &field.attributes.attributes;
        let attribute_list = field.attributes.attributes_by_model.get(model);
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
            #visibility id: #field_type,
        }
    } else {
        quote! {}
    };

    let attributes = &info.item_attributes.attributes;
    let attribute_list = info.item_attributes.attributes_by_model.get(model);
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
        #visibility struct #api_document_name #generics {
            #id_field

            #(#field_list)*
        }
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

pub fn build_from_to(
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    is_sub_model: bool,
    fields_in_model: &[&FieldInfo],
    _imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let document_name = &info.document_name;
    let api_document_name = &info.api_document_names.get(model).unwrap();

    let all_fields_are_optional_or_db_properties =
        info.check_all_db_fields_are_optional_or_properties();

    // Evaluate fields.
    let to_api_field_list = fields_in_model.iter().filter_map(|field| {
        let name = field.name();

        if field.attributes.skip_in_model.contains(model)
            || field.attributes.skip_in_model.contains(DB_MODEL_TAG)
        {
            return None;
        }

        let apply_into = field.attributes.inner_type_by_model.get(model).is_some();

        let base = match field.base_type_kind {
            BaseTypeKind::Other => {
                if apply_into {
                    quote! {
                        v.into()
                    }
                } else {
                    quote! {
                        v
                    }
                }
            }
            BaseTypeKind::Box => {
                if apply_into {
                    quote! {
                        Box::new((*v).into())
                    }
                } else {
                    quote! {
                        v
                    }
                }
            }
            BaseTypeKind::Vec => {
                if apply_into {
                    quote! {
                        v.into_iter().map(|v| v.into()).collect()
                    }
                } else {
                    quote! {
                        v
                    }
                }
            }
            BaseTypeKind::VecDBReference => {
                if apply_into {
                    quote! {
                        v.into_iter().map(|v| v.map_to_api(|v| Box::new((*v).into()))).collect()
                    }
                } else {
                    quote! {
                        v.into_iter().map(|v| v.map_to_api(|v| Box::new(v))).collect()
                    }
                }
            }
            BaseTypeKind::HashMap => {
                if apply_into {
                    quote! {
                        v.into_iter().map(|(k, v)| (k, v.into())).collect()
                    }
                } else {
                    quote! {
                        v
                    }
                }
            }
            BaseTypeKind::DBReference => {
                if apply_into {
                    quote! {
                        v.map_to_api(|v| Box::new((*v).into()))
                    }
                } else {
                    quote! {
                        v.map_to_api(|v| Box::new(v))
                    }
                }
            }
        };

        let result = match field.field_type_kind {
            Some(FieldTypeKind::NullableOption) | Some(FieldTypeKind::Option) => {
                if apply_into {
                    quote! {
                        #name: value.#name.map(|v| #base),
                    }
                } else {
                    quote! {
                        #name: {
                            let v = value.#name;
                            #base
                        },
                    }
                }
            }
            None => quote! {
                #name: {
                    let v = value.#name;
                    #base
                },
            },
        };

        Some(result)
    });

    let to_db_field_list = fields_in_model.iter().filter_map(|field| {
        let name = field.name();

        if field.attributes.skip_in_model.contains(model)
            || field.attributes.skip_in_model.contains(DB_MODEL_TAG)
        {
            return None;
        }

        let apply_into = field.attributes.inner_type_by_model.get(model).is_some();

        let base = match field.base_type_kind {
            BaseTypeKind::Other => {
                if apply_into {
                    quote! {
                        v.into()
                    }
                } else {
                    quote! {
                        v
                    }
                }
            }
            BaseTypeKind::Box => {
                if apply_into {
                    quote! {
                        Box::new((*v).into())
                    }
                } else {
                    quote! {
                        v
                    }
                }
            }
            BaseTypeKind::Vec => {
                if apply_into {
                    quote! {
                        v.into_iter().map(|v| v.into()).collect()
                    }
                } else {
                    quote! {
                        v
                    }
                }
            }
            BaseTypeKind::VecDBReference => {
                if apply_into {
                    quote! {
                        v.into_iter().map(|v| v.map_to_db(|v| Box::new((*v).into()))).collect()
                    }
                } else {
                    quote! {
                        v.into_iter().map(|v| v.map_to_db(|v| Box::new(v))).collect()
                    }
                }
            }
            BaseTypeKind::HashMap => {
                if apply_into {
                    quote! {
                        v.into_iter().map(|(k, v)| (k, v.into())).collect()
                    }
                } else {
                    quote! {
                        v
                    }
                }
            }
            BaseTypeKind::DBReference => {
                if apply_into {
                    quote! {
                        v.map_to_db(|v| Box::new((*v).into()))
                    }
                } else {
                    quote! {
                        v.map_to_db(|v| Box::new(v))
                    }
                }
            }
        };

        let result = match field.field_type_kind {
            Some(FieldTypeKind::NullableOption) | Some(FieldTypeKind::Option) => {
                if apply_into {
                    quote! {
                        #name: value.#name.map(|v| #base),
                    }
                } else {
                    quote! {
                        #name: {
                            let v = value.#name;
                            #base
                        },
                    }
                }
            }
            None => quote! {
                #name: {
                    let v = value.#name;
                    #base
                },
            },
        };

        Some(result)
    });

    let (to_api_id_field, to_db_key_field) = if !is_sub_model {
        (
            quote! {
                id: value.db_key,
            },
            quote! {
                db_key: value.id,
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    // Evaluate default fields.
    let default_rest = if all_fields_are_optional_or_db_properties {
        quote! { ..Default::default() }
    } else {
        quote! {}
    };

    // Build result.
    Ok(quote! {
        impl #generics From<#document_name #generics> for #api_document_name #generics {
            #[allow(clippy::needless_update)]
            fn from(value: #document_name #generics) -> Self {
                Self {
                    #to_api_id_field
                    #(#to_api_field_list)*
                    #default_rest
                }
            }
        }

        impl #generics From<#api_document_name #generics> for #document_name #generics {
            #[allow(clippy::needless_update)]
            fn from(value: #api_document_name #generics) -> Self {
                Self {
                    #to_db_key_field
                    #(#to_db_field_list)*
                    #default_rest
                }
            }
        }
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

pub fn build_api_fields(
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    is_sub_model: bool,
    fields_in_model: &[&FieldInfo],
    _imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let visibility = info.item.visibility();
    let api_field_enum_name = &info.api_field_enum_names.get(model).unwrap();

    // Evaluate fields.
    let mut enum_fields = vec![];
    let mut path_fields = vec![];

    fields_in_model.iter().for_each(|field| {
        let name_str = from_snake_case_to_pascal_case(&field.name().to_string());
        let name = format_ident!("{}", name_str, span = field.name().span());
        let db_name = &field.db_name;

        match field.attributes.inner_model {
            InnerModelKind::Struct => match field.base_type_kind {
                BaseTypeKind::DBReference => {
                    let key_path = format!("{}._key", db_name);

                    enum_fields.push(quote! {
                        #name,
                    });
                    path_fields.push(quote! {
                        #api_field_enum_name::#name => #key_path.into(),
                    });
                }
                _ => {
                    let inner_api_type = field.attributes.inner_type_by_model.get(model);
                    let inner_api_type_name = inner_api_type
                        .map(|v| v.to_token_stream().to_string())
                        .unwrap_or_else(|| field.get_inner_db_type_name());
                    let inner_api_type_enum = format_ident!("{}Field", inner_api_type_name);
                    let sub_pattern = format!("{}.{{}}", db_name);

                    enum_fields.push(quote! {
                        #name(Option<#inner_api_type_enum>),
                    });
                    path_fields.push(quote! {
                        #api_field_enum_name::#name(v) => if let Some(v) = v {
                            format!(#sub_pattern, v.path()).into()
                        } else {
                            #db_name.into()
                        }
                    });
                }
            },
            InnerModelKind::Data | InnerModelKind::Enum => match field.base_type_kind {
                BaseTypeKind::DBReference => {
                    let key_path = format!("{}._key", db_name);

                    enum_fields.push(quote! {
                        #name,
                    });
                    path_fields.push(quote! {
                        #api_field_enum_name::#name => #key_path.into(),
                    });
                }
                _ => {
                    enum_fields.push(quote! {
                        #name,
                    });
                    path_fields.push(quote! {
                        #api_field_enum_name::#name => #db_name.into(),
                    });
                }
            },
        }
    });

    // Check it is empty.
    if enum_fields.is_empty() {
        return Ok(quote! {});
    }

    // Id field.
    let (id_field, id_field_path) = if !is_sub_model {
        (
            quote! {
                Id,
            },
            quote! {
                #api_field_enum_name::Id => "_key".into(),
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    // Build result.
    Ok(quote! {
        #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[serde(tag = "T", content = "V")]
        #visibility enum #api_field_enum_name {
            #id_field
            #(#enum_fields)*
        }

        impl #api_field_enum_name {
            pub fn path(&self) -> Cow<'static, str> {
                match self {
                    #id_field_path
                    #(#path_fields)*
                }
            }
        }
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_api_document_impl(
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_model: &[&FieldInfo],
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let api_document_name = &info.api_document_names.get(model);

    imports.insert("::arangodb_types::traits::APIDocument".to_string());

    // Evaluate map_to_null.
    let map_to_null_fields = fields_in_model.iter().filter_map(|field| {
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

    // Build result.
    let key_field = info.get_key_field().unwrap();
    let inner_api_type = key_field.attributes.inner_type_by_model.get(model);
    let key_type = inner_api_type
        .map(|v| v.to_token_stream())
        .unwrap_or_else(|| key_field.inner_type.clone().unwrap());
    Ok(quote! {
        impl APIDocument for #api_document_name {
            type Id = #key_type;

            // GETTERS --------------------------------------------------------

            fn id(&self) -> &Option<Self::Id> {
                &self.id
            }

            // METHODS --------------------------------------------------------

            fn map_values_to_null(&mut self) {
                #(#map_to_null_fields)*
            }
        }
    })
}
