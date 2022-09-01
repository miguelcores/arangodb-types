use proc_macro2::TokenStream;
use quote::quote;
use syn::File;

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

    let tokens = match &info.item {
        ModelNode::Struct(_) => {
            let db = build_db_struct_type(&options, &info)?;
            let mut models = Vec::with_capacity(options.build_models.len());

            for model in &options.build_models {
                models.push(build_api_struct_type(model, &options, &info)?);
            }

            quote! {
                #db
                #(#models)*
            }
        }
        ModelNode::Enum(_) => {
            let db = build_db_enum_type(&options, &info)?;
            let mut models = Vec::with_capacity(options.build_models.len());

            for model in &options.build_models {
                models.push(build_api_enum_type(model, &options, &info)?);
            }

            quote! {
                #db
                #(#models)*
            }
        }
    };

    // Keep this for debugging purpose.
    // return Err(crate::errors::Error::Message(tokens.to_string()).with_tokens(file));

    Ok(tokens)
}
