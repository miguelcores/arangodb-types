use convert_case::{Case, Casing};

pub fn from_snake_case_to_pascal_case(input: &str) -> String {
    input.from_case(Case::Snake).to_case(Case::Pascal)
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

pub fn from_snake_case_to_camel_case(input: &str) -> String {
    input.from_case(Case::Snake).to_case(Case::Camel)
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

pub fn from_pascal_case_to_snake_case(input: &str) -> String {
    input.from_case(Case::Pascal).to_case(Case::Snake)
}
