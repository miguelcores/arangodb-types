use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{File, ItemUse};

pub use build_api::*;
pub use build_db::*;

use crate::data::{ModelInfo, ModelOptions};

mod build_api;
mod build_db;

pub fn process_model(file: File) -> Result<TokenStream, syn::Error> {
    let options = ModelOptions::from_attributes(&file.attrs)?;
    let info = ModelInfo::from_file_for_model(&options, &file)?;
    let mut imports = HashSet::<String>::new();

    let db = build_db_model(&options, &info, &mut imports)?;
    let api = if options.build_api {
        build_api_model(&options, &info, &mut imports)?
    } else {
        quote! {}
    };

    let imports = if !options.no_imports {
        imports
            .into_iter()
            .map(|v| syn::parse_str::<ItemUse>(format!("use {};", v.as_str()).as_str()).unwrap())
            .collect()
    } else {
        vec![]
    };

    Ok(quote! {
        #(#imports)*
        #db
        #api
    })
}
