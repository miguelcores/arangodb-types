//! Procedural macro to create a Database model for types.

#![recursion_limit = "128"]

extern crate proc_macro;

use syn::{parse_macro_input, File};

use model_builders::*;
use type_builders::*;

mod constants;
mod data;
mod errors;
mod model_builders;
mod type_builders;
mod utils;

/// Creates a root database model for a type.
#[proc_macro]
pub fn model(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let file = parse_macro_input!(item as File);

    process_model(file)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

/// Creates a type of a database model for a type.
#[proc_macro]
pub fn type_model(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let file = parse_macro_input!(item as File);

    process_type(file)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
