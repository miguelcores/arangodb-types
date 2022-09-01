use proc_macro2::TokenStream;
use quote::quote;
use syn::File;

pub use build_api::*;
pub use build_db::*;

use crate::data::{ModelInfo, ModelOptions};

mod build_api;
mod build_db;

pub fn process_model(file: File) -> Result<TokenStream, syn::Error> {
    let options = ModelOptions::from_attributes(&file.attrs)?;
    let info = ModelInfo::from_file_for_model(&options, &file)?;

    let db = build_db_model(&options, &info)?;
    let mut models = Vec::with_capacity(options.build_models.len());

    for model_name in &options.build_models {
        models.push(build_api_model(model_name, &options, &info)?);
    }

    let tokens = quote! {
        #db
        #(#models)*
    };

    // Keep this for debugging purpose.
    // return Err(crate::errors::Error::Message(tokens.to_string()).with_tokens(file));

    Ok(tokens)
}
