use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use quote::{format_ident, ToTokens};

use crate::data::{FieldInfo, InnerModelKind, ModelInfo, ModelOptions};
use crate::utils::from_pascal_case_to_snake_case;

pub fn build_api_enum_type(
    model: &str,
    options: &ModelOptions,
    info: &ModelInfo,
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let fields_in_model = info.fields_in_model(model);
    let enum_tokens = build_enum(model, options, info, &fields_in_model, imports)?;
    let impl_tokens = if !options.skip_impl {
        build_impl(model, options, info, &fields_in_model, imports)?
    } else {
        quote! {}
    };
    let from_to_tokens = build_from_to(model, options, info, &fields_in_model, imports)?;

    let field_list_tokens = if !options.skip_fields {
        build_field_list(model, options, info, &fields_in_model, imports)?
    } else {
        quote! {}
    };

    // Build result.
    Ok(quote! {
        #enum_tokens
        #impl_tokens
        #from_to_tokens
        #field_list_tokens
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_enum(
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_model: &[&FieldInfo],
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let visibility = info.item.visibility();
    let generics = info.item.generics();
    let document_name = &info.api_document_names.get(model).unwrap();

    let all_variants_are_unit = info.check_all_api_variants_are_unit(model);

    // Evaluate simple attributes.
    let simple_attributes = if all_variants_are_unit {
        quote! {#[derive(Copy, Eq, PartialEq, Hash)]}
    } else {
        quote! {}
    };

    // Evaluate fields.
    let field_list = fields_in_model.iter().map(|field| {
        let name = field.name();

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

        if field.inner_type.is_some() {
            let inner_type = field.build_api_field_type(model);

            quote! {
                #attributes
                #name(#inner_type),
            }
        } else if !all_variants_are_unit {
            quote! {
                #attributes
                #name(Option<()>),
            }
        } else {
            quote! {
                #attributes
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

    imports.insert("::serde::Deserialize".to_string());
    imports.insert("::serde::Serialize".to_string());

    // Build result.
    Ok(quote! {
        #[derive(Debug, Clone, Serialize, Deserialize)]
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
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_model: &[&FieldInfo],
    _imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let document_name = &info.api_document_names.get(model).unwrap();

    let all_variants_are_unit = info.check_all_api_variants_are_unit(model);

    // Evaluate is * method.
    let is_method_list = fields_in_model.iter().map(|field| {
        let name = field.name();
        let fn_name = from_pascal_case_to_snake_case(&name.to_string());
        let fn_name = format_ident!("is_{}", fn_name, span = name.span());

        if field.inner_type.is_some() {
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
        let fields = fields_in_model.iter().map(|field| {
            let name = field.name();

            if field.inner_type.is_none() {
                quote! {
                    #document_name::#name => {}
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

fn build_from_to(
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_model: &[&FieldInfo],
    _imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let document_name = &info.document_name;
    let api_document_name = &info.api_document_names.get(model).unwrap();

    let all_db_variants_are_unit = info.check_all_db_variants_are_unit();

    // Evaluate fields.
    let to_api_field_list = fields_in_model.iter().map(|field| {
        let name = field.name();

        if field.inner_type.is_some() {
            quote! {
                #document_name::#name(v) => #api_document_name::#name(v.into()),
            }
        } else if all_db_variants_are_unit {
            quote! {
                #document_name::#name => #api_document_name::#name,
            }
        } else {
            quote! {
                #document_name::#name(_) => #api_document_name::#name,
            }
        }
    });

    let to_db_field_list = fields_in_model.iter().map(|field| {
        let name = field.name();

        if field.inner_type.is_some() {
            quote! {
                #api_document_name::#name(v) => #document_name::#name(v.into()),
            }
        } else if all_db_variants_are_unit {
            quote! {
                #api_document_name::#name => #document_name::#name,
            }
        } else {
            quote! {
                #api_document_name::#name => #document_name::#name(None),
            }
        }
    });

    // Build result.
    Ok(quote! {
        impl #generics From<#document_name #generics> for #api_document_name #generics {
            fn from(value: #document_name #generics) -> Self {
                match value {
                    #(#to_api_field_list)*
                }
            }
        }

        impl #generics From<#api_document_name #generics> for #document_name #generics {
            fn from(value: #api_document_name #generics) -> Self {
                match value {
                    #(#to_db_field_list)*
                }
            }
        }
    })
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn build_field_list(
    model: &str,
    _options: &ModelOptions,
    info: &ModelInfo,
    fields_in_model: &[&FieldInfo],
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let generics = info.item.generics();
    let api_document_name = &info.api_document_names.get(model).unwrap();
    let api_field_enum_name = &info.api_field_enum_names.get(model).unwrap();
    let visibility = info.item.visibility();

    // Evaluate fields.
    let mut field_names = vec![];
    let mut field_paths = vec![];
    let mut get_variant_list = vec![];

    fields_in_model.iter().for_each(|field| {
        let name = field.name();
        let db_name = &field.db_name;

        match field.attributes.inner_model {
            InnerModelKind::Data => {
                field_names.push(quote! {
                    #name(Option<()>),
                });
                field_paths.push(quote! {
                    #api_field_enum_name::#name(_) => Cow::Borrowed(#db_name),
                });

                if field.inner_type.is_some() {
                    get_variant_list.push(quote! {
                        #api_document_name::#name(_) => #api_field_enum_name::#name(None),
                    });
                } else {
                    get_variant_list.push(quote! {
                        #api_document_name::#name => #api_field_enum_name::#name(None),
                    });
                }
            }
            InnerModelKind::Struct => {
                let inner_api_type = field.attributes.inner_type_by_model.get(model);
                let inner_api_type_name = inner_api_type
                    .map(|v| v.to_token_stream().to_string())
                    .unwrap_or_else(|| field.get_inner_db_type_name());
                let inner_api_type_enum = format_ident!("{}Field", inner_api_type_name);

                field_names.push(quote! {
                    #name(Option<#inner_api_type_enum>),
                });
                field_paths.push(quote! {
                    #api_field_enum_name::#name(v) => if let Some(v) = v {
                        Cow::Owned(format!("V.{}", v.path()))
                    } else {
                        Cow::Borrowed(#db_name)
                    }
                });
                get_variant_list.push(quote! {
                    #api_document_name::#name(_) => #api_field_enum_name::#name(None),
                });
            }
            InnerModelKind::Enum => {
                let inner_api_type = field.attributes.inner_type_by_model.get(model);
                let inner_api_type_name = inner_api_type
                    .map(|v| v.to_token_stream().to_string())
                    .unwrap_or_else(|| field.get_inner_db_type_name());
                let inner_api_type_enum = format_ident!("{}Field", inner_api_type_name);

                field_names.push(quote! {
                    #name(Option<#inner_api_type_enum>),
                });
                field_paths.push(quote! {
                    #api_field_enum_name::#name(v) => if let Some(v) = v {
                        Cow::Owned(format!("V.{}", v.path()))
                    } else {
                        Cow::Borrowed(#db_name)
                    }
                });
                get_variant_list.push(quote! {
                    #api_document_name::#name(v) => #api_field_enum_name::#name(Some(v.variant())),
                });
            }
        }
    });

    imports.insert("::serde::Deserialize".to_string());
    imports.insert("::serde::Serialize".to_string());

    // Build result.
    Ok(quote! {
        impl #generics #api_document_name #generics {
            pub fn variant(&self) -> #api_field_enum_name {
                match self {
                    #(#get_variant_list)*
                }
            }
        }

        #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[serde(tag = "T", content = "V")]
        #visibility enum #api_field_enum_name {
            #[serde(rename = "_type")]
            TypeField(Option<()>),

            #[serde(rename = "_value")]
            ValueField(Option<()>),

            #(#field_names)*
        }

        impl #api_field_enum_name {
            pub fn path(&self) -> Cow<'static, str> {
                match self {
                    #api_field_enum_name::TypeField(_) => Cow::Borrowed("T"),
                    #api_field_enum_name::ValueField(_) => Cow::Borrowed("V"),
                    #(#field_paths)*
                }
            }
        }
    })
}
