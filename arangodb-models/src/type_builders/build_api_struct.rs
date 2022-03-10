use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;

use crate::data::{ModelInfo, ModelOptions};
use crate::model_builders::{build_api_fields, build_api_struct, build_from_to};

pub fn build_api_struct_type(
    model: &str,
    options: &ModelOptions,
    info: &ModelInfo,
    imports: &mut HashSet<String>,
) -> Result<TokenStream, syn::Error> {
    let fields_in_model = info.fields_in_model(model);
    let struct_tokens = build_api_struct(model, options, info, true, &fields_in_model, imports)?;
    let from_to_tokens = build_from_to(model, options, info, true, &fields_in_model, imports)?;
    let api_fields_tokens =
        build_api_fields(model, options, info, true, &fields_in_model, imports)?;

    // Build result.
    Ok(quote! {
        #struct_tokens
        #from_to_tokens
        #api_fields_tokens
    })
}
