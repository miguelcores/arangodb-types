use crate::constants::DB_MODEL_TAG;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::spanned::Spanned;

use crate::data::{FieldInfo, InnerModelKind, ModelInfo, ModelOptions};
use crate::utils::from_pascal_case_to_snake_case;

pub fn build_db_enum_type(
    options: &ModelOptions,
    info: &ModelInfo,
) -> Result<TokenStream, syn::Error> {
    let fields_in_db: Vec<_> = info.fields_in_db().collect();
    let enum_tokens = build_enum(options, info, &fields_in_db)?;
    let impl_tokens = if !options.skip_impl {
        build_impl(options, info, &fields_in_db)?
    } else {
        quote! {}
    };

    let field_list_tokens = if !options.skip_fields {
        build_field_list(options, info, &fields_in_db)?
    } else {
        quote! {}
    };

    let aql_mapping_impl_tokens = build_aql_mapping_impl(options, info, &fields_in_db)?;

    // Build result.
    Ok(quote! {
        #enum_tokens
        #impl_tokens
        #field_list_tokens
        #aql_mapping_impl_tokens
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_enum(
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_db: &[&FieldInfo],
) -> Result<TokenStream, syn::Error> {
    let visibility = info.item.visibility();
    let generics = info.item.generics();
    let document_name = &info.document_name;

    let all_variants_are_unit = info.check_all_db_variants_are_unit();

    // Evaluate simple attributes.
    let simple_attributes = if all_variants_are_unit {
        quote! {#[derive(Copy, Eq, PartialEq, Hash)]}
    } else {
        quote! {}
    };

    // Evaluate fields.
    let field_list = fields_in_db.iter().map(|field| {
        let name = field.name();
        let db_name = &field.db_name;

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

        if let Some(inner_type) = &field.inner_type {
            quote! {
                #attributes
                #[serde(rename = #db_name)]
                #name(#inner_type),
            }
        } else if !all_variants_are_unit {
            quote! {
                #attributes
                #[serde(rename = #db_name)]
                #name(Option<()>),
            }
        } else {
            quote! {
                #attributes
                #[serde(rename = #db_name)]
                #name,
            }
        }
    });

    // Process serde tag.
    let serde_tag_attribute = if !all_variants_are_unit {
        quote! {
            #[serde(tag = "T", content = "V")]
        }
    } else {
        quote! {}
    };

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
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
        #simple_attributes
        #[serde(rename_all = "camelCase")]
        #serde_tag_attribute
        #attributes
        #visibility enum #document_name #generics {
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
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let document_name = &info.document_name;

    let all_variants_are_unit = info.check_all_db_variants_are_unit();

    // Evaluate is * method.
    let is_method_list = fields_in_db.iter().map(|field| {
        let name = field.name();
        let fn_name = from_pascal_case_to_snake_case(&name.to_string());
        let fn_name = format_ident!("is_{}", fn_name, span = name.span());

        if field.inner_type.is_some() || !all_variants_are_unit {
            quote! {
                pub fn #fn_name(&self) -> bool {
                    matches!(self, #document_name::#name(_))
                }
            }
        } else {
            quote! {
                pub fn #fn_name(&self) -> bool {
                    matches!(self, #document_name::#name)
                }
            }
        }
    });

    // Evaluate map_values_to_null_method method.
    let map_values_to_null_method_tokens = if all_variants_are_unit {
        quote! {
            pub fn map_values_to_null(&mut self) { }
        }
    } else {
        let fields = fields_in_db.iter().map(|field| {
            let name = field.name();

            if field.inner_type.is_none() {
                quote! {
                    #document_name::#name(_) => {}
                }
            } else {
                match field.attributes.inner_model {
                    InnerModelKind::Struct | InnerModelKind::Enum => quote! {
                        #document_name::#name(v) => v.map_values_to_null(),
                    },
                    InnerModelKind::Data => quote! {
                        #document_name::#name(_) => {}
                    },
                }
            }
        });

        quote! {
            pub fn map_values_to_null(&mut self) {
                match self {
                    #(#fields)*
                }
            }
        }
    };

    // Build result.
    Ok(quote! {
        impl #generics #document_name #generics {
            #(#is_method_list)*
            #map_values_to_null_method_tokens

            pub fn is_all_missing(&self) -> bool {
                false
            }

            pub fn is_all_null(&self) -> bool {
                false
            }
        }
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_field_list(
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_db: &[&FieldInfo],
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let document_name = &info.document_name;
    let enum_name = &info.field_enum_name;
    let visibility = info.item.visibility();

    let all_variants_are_unit = info.check_all_db_variants_are_unit();

    // Evaluate fields.
    let mut field_names = vec![];
    let mut field_paths = vec![];
    let mut get_variant_list = vec![];

    if all_variants_are_unit {
        fields_in_db.iter().for_each(|field| {
            let name = field.name();
            let db_name = &field.db_name;

            field_names.push(quote! {
                #[serde(rename = #db_name)]
                #name(Option<()>),
            });
            field_paths.push(quote! {
                #enum_name::#name(_) => ::std::borrow::Cow::Borrowed(#db_name),
            });
            get_variant_list.push(quote! {
                #document_name::#name => #enum_name::#name(None),
            });
        });
    } else {
        fields_in_db.iter().for_each(|field| {
            let name = field.name();
            let db_name = &field.db_name;

            match field.attributes.inner_model {
                InnerModelKind::Data => {
                    field_names.push(quote! {
                        #[serde(rename = #db_name)]
                        #name(Option<()>),
                    });
                    field_paths.push(quote! {
                        #enum_name::#name(_) => ::std::borrow::Cow::Borrowed(#db_name),
                    });
                    get_variant_list.push(quote! {
                        #document_name::#name(_) => #enum_name::#name(None),
                    });
                }
                InnerModelKind::Struct => {
                    let inner_type = field.inner_type.as_ref().unwrap();
                    let inner_type_name = field.get_inner_db_type_name();
                    let inner_type_enum =
                        format_ident!("{}Field", inner_type_name, span = inner_type.span());

                    field_names.push(quote! {
                        #[serde(rename = #db_name)]
                        #name(Option<#inner_type_enum>),
                    });
                    field_paths.push(quote! {
                        #enum_name::#name(v) => if let Some(v) = v {
                            ::std::borrow::Cow::Owned(format!("V.{}", v.path()))
                        } else {
                            ::std::borrow::Cow::Borrowed(#db_name)
                        },
                    });
                    get_variant_list.push(quote! {
                        #document_name::#name(_) => #enum_name::#name(None),
                    });
                }
                InnerModelKind::Enum => {
                    let inner_type = field.inner_type.as_ref().unwrap();
                    let inner_type_name = field.get_inner_db_type_name();
                    let inner_type_enum =
                        format_ident!("{}Field", inner_type_name, span = inner_type.span());

                    field_names.push(quote! {
                        #[serde(rename = #db_name)]
                        #name(Option<#inner_type_enum>),
                    });
                    field_paths.push(quote! {
                        #enum_name::#name(v) => if let Some(v) = v {
                            ::std::borrow::Cow::Owned(format!("V.{}", v.path()))
                        } else {
                            ::std::borrow::Cow::Borrowed(#db_name)
                        },
                    });
                    get_variant_list.push(quote! {
                        #document_name::#name(v) => #enum_name::#name(Some(v.variant())),
                    });
                }
            }
        });
    }

    // Build result.
    Ok(quote! {
        impl #generics #document_name #generics {
            pub fn variant(&self) -> #enum_name {
                match self {
                    #(#get_variant_list)*
                }
            }
        }

        #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[serde(tag = "T", content = "V")]
        #visibility enum #enum_name {
            #[serde(rename = "_T")]
            TypeField(Option<()>),

            #[serde(rename = "_V")]
            ValueField(Option<()>),

            #(#field_names)*
        }

        impl #enum_name {
            pub fn path(&self) -> ::std::borrow::Cow<'static, str> {
                match self {
                    #enum_name::TypeField(_) => ::std::borrow::Cow::Borrowed("T"),
                    #enum_name::ValueField(_) => ::std::borrow::Cow::Borrowed("V"),
                    #(#field_paths)*
                }
            }
        }
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_aql_mapping_impl(
    options: &ModelOptions,
    info: &ModelInfo,
    fields_in_db: &[&FieldInfo],
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let document_name = &info.document_name;

    let all_variants_are_unit = info.check_all_db_variants_are_unit();

    let include_let_steps_method = if !all_variants_are_unit {
        let fields = fields_in_db.iter().map(|field| {
            let name = field.name();

            match field.attributes.inner_model {
                InnerModelKind::Struct | InnerModelKind::Enum => if field.inner_type.is_some() {
                    quote! {
                        #document_name::#name(v) => v.include_let_steps(aql, sub_path.as_str(), next_id),
                    }
                } else {
                    quote! {
                        #document_name::#name(_) => {}
                    }
                },
                InnerModelKind::Data => quote! {
                    #document_name::#name(_) => {}
                },
            }
        });

        quote! {
            #[allow(unused_variables)]
            fn include_let_steps(&self, aql: &mut ::arangodb_types::aql::AqlBuilder, path: &str, next_id: &mut usize) {
                let sub_path = format!("{}.V", path);

                match self {
                    #(#fields)*
                }
            }
        }
    } else {
        quote! {}
    };

    let map_to_json_method = if !all_variants_are_unit {
        let fields = fields_in_db.iter().map(|field| {
            let name = field.name();

            match field.attributes.inner_model {
                InnerModelKind::Data => quote! {
                    #document_name::#name(_) => {
                        buffer.write_all(b"null").unwrap();
                    }
                },
                InnerModelKind::Struct | InnerModelKind::Enum => if field.inner_type.is_some() {
                    quote! {
                        #document_name::#name(v) => v.map_to_json(buffer, sub_path.as_str(), next_id),
                    }
                } else {
                    quote! {
                        #document_name::#name(_) => {
                            buffer.write_all(b"null").unwrap();
                        }
                    }
                },
            }
        });

        quote! {
            #[allow(unused_variables)]
            fn map_to_json(&self, buffer: &mut Vec<u8>, path: &str, next_id: &mut usize) {
                use std::io::Write;
                let sub_path = format!("{}.V", path);

                buffer.write_all(b"{T:null,V:").unwrap();

                match self {
                    #(#fields)*
                }

                buffer.write_all(b"}").unwrap();
            }
        }
    } else {
        quote! {
            #[allow(unused_variables)]
            fn map_to_json(&self, buffer: &mut Vec<u8>, path: &str, next_id: &mut usize) {
                use std::io::Write;
                buffer.write_all(b"null").unwrap();
            }
        }
    };

    let impl_name = if options.relative_imports {
        quote!(AQLMapping)
    } else {
        quote!(::arangodb_types::traits::AQLMapping)
    };

    Ok(quote! {
        impl #generics #impl_name for #document_name #generics {
            #include_let_steps_method
            #map_to_json_method
        }
    })
}
