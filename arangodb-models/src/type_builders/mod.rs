use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{File, ItemUse};

pub use build_api_enum::*;
pub use build_api_struct::*;
use build_db_enum::*;
use build_db_struct::*;

use crate::data::{ModelInfo, ModelNode, ModelOptions};

mod build_api_enum;
mod build_api_struct;
mod build_db_enum;
mod build_db_struct;

pub fn process_type(file: File) -> Result<TokenStream, syn::Error> {
    let options = ModelOptions::from_attributes(&file.attrs)?;
    let info = ModelInfo::from_file_for_sub_model(&options, &file)?;
    let mut imports = HashSet::<String>::new();

    let tokens = match &info.item {
        ModelNode::Struct(_) => {
            let db = build_db_struct_type(&options, &info, &mut imports)?;
            let api = if options.build_api {
                build_api_struct_type(&options, &info, &mut imports)?
            } else {
                quote! {}
            };

            let imports = if !options.no_imports {
                imports
                    .into_iter()
                    .map(|v| {
                        syn::parse_str::<ItemUse>(format!("use {};", v.as_str()).as_str()).unwrap()
                    })
                    .collect()
            } else {
                vec![]
            };

            quote! {
                #(#imports)*
                #db
                #api
            }
        }
        ModelNode::Enum(_) => {
            let db = build_db_enum_type(&options, &info, &mut imports)?;
            let api = if options.build_api {
                build_api_enum_type(&options, &info, &mut imports)?
            } else {
                quote! {}
            };

            let imports = if !options.no_imports {
                imports
                    .into_iter()
                    .map(|v| {
                        syn::parse_str::<ItemUse>(format!("use {};", v.as_str()).as_str()).unwrap()
                    })
                    .collect()
            } else {
                vec![]
            };

            quote! {
                #(#imports)*
                #db
                #api
            }
        }
    };

    Ok(tokens)
}
