use std::collections::{HashMap, HashSet};
use std::iter::Filter;
use std::slice::Iter;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, ToTokens, TokenStreamExt};
use syn::{Attribute, File, Generics, Item, ItemEnum, ItemStruct, Visibility};

use crate::constants::{
    DB_COLLECTION_SUFFIX, DB_DOCUMENT_SUFFIX, DB_MODEL_NAME, DB_MODEL_TAG, FIELDS_SUFFIX,
    MUTEX_FIELD_DB_NAME,
};
use crate::data::{FieldInfo, ModelOptions, StructAttributes};
use crate::errors::Error;
use crate::utils::from_snake_case_to_pascal_case;

pub struct ModelInfo<'a> {
    pub file: &'a File,
    pub item: ModelNode<'a>,
    pub item_attributes: StructAttributes,
    pub item_fields: Vec<FieldInfo<'a>>,
    // Other info
    pub document_name: Ident,
    pub collection_name: Ident,
    pub field_enum_name: Ident,
    pub api_document_names: HashMap<String, Ident>,
    pub api_field_enum_names: HashMap<String, Ident>,
}

impl<'a> ModelInfo<'a> {
    // CONSTRUCTORS -----------------------------------------------------------

    pub fn from_file_for_model(
        options: &ModelOptions,
        file: &'a File,
    ) -> Result<ModelInfo<'a>, syn::Error> {
        let mut items_iter = file.items.iter();

        // Check a struct is present and in the first position.
        let struct_item = match items_iter.next() {
            Some(v) => v,
            None => return Err(Error::MissingStructItem.with_tokens(file)),
        };

        let struct_item = match struct_item {
            Item::Struct(v) => v,
            _ => return Err(Error::MissingStructItem.with_tokens(file)),
        };
        let struct_attributes = StructAttributes::from_attributes(&struct_item.attrs)?;

        // Check struct fields.
        let mut struct_fields = Vec::with_capacity(struct_item.fields.len());
        for field in &struct_item.fields {
            struct_fields.push(FieldInfo::from_field(field)?);
        }

        // Build other info.
        let document_name = format_ident!("{}{}", struct_item.ident, DB_DOCUMENT_SUFFIX);
        let api_document_names: HashMap<_, _> = options
            .build_models
            .iter()
            .map(|v| {
                (
                    v.clone(),
                    Self::compute_api_document_name(&document_name, v),
                )
            })
            .collect();
        let collection_name = if let Some(collection_name) = &options.collection_name {
            format_ident!("{}", collection_name)
        } else {
            format_ident!("{}{}", struct_item.ident, DB_COLLECTION_SUFFIX)
        };
        let field_enum_name = format_ident!(
            "{}{}{}",
            struct_item.ident,
            DB_DOCUMENT_SUFFIX,
            FIELDS_SUFFIX
        );
        let api_field_enum_names = api_document_names
            .iter()
            .map(|(n, v)| (n.clone(), format_ident!("{}{}", v, FIELDS_SUFFIX)))
            .collect();

        // Build result.
        let mut result = ModelInfo {
            file,
            item: ModelNode::Struct(struct_item),
            item_attributes: struct_attributes,
            item_fields: struct_fields,
            document_name,
            collection_name,
            field_enum_name,
            api_document_names,
            api_field_enum_names,
        };

        // Analyze rest functions.
        result.analyze_rest_functions(items_iter)?;

        // Final checks.
        result.check_names(options)?;

        Ok(result)
    }

    pub fn from_file_for_sub_model(
        options: &ModelOptions,
        file: &'a File,
    ) -> Result<ModelInfo<'a>, syn::Error> {
        let mut items_iter = file.items.iter();

        // Check a struct/enum is present and in the first position.
        let item = match items_iter.next() {
            Some(v) => v,
            None => return Err(Error::MissingStructOrEnumItem.with_tokens(file)),
        };

        let item = match item {
            Item::Struct(v) => ModelNode::Struct(v),
            Item::Enum(v) => ModelNode::Enum(v),
            _ => return Err(Error::MissingStructOrEnumItem.with_tokens(file)),
        };
        let item_attributes = StructAttributes::from_attributes(item.attributes())?;

        // Check struct fields.
        let item_fields = match &item {
            ModelNode::Struct(item) => {
                let mut item_fields = Vec::with_capacity(item.fields.len());
                for field in &item.fields {
                    item_fields.push(FieldInfo::from_field(field)?);
                }
                item_fields
            }
            ModelNode::Enum(item) => {
                let mut item_fields = Vec::with_capacity(item.variants.len());
                for variant in &item.variants {
                    item_fields.push(FieldInfo::from_variant(variant)?);
                }
                item_fields
            }
        };

        // Build other info.
        let document_name = item.ident().clone();
        let collection_name = format_ident!("undefined");
        let field_enum_name = format_ident!("{}{}", document_name, FIELDS_SUFFIX);
        let api_document_names: HashMap<_, _> = options
            .build_models
            .iter()
            .map(|v| {
                (
                    v.clone(),
                    Self::compute_api_document_name(&document_name, v),
                )
            })
            .collect();
        let api_field_enum_names = api_document_names
            .iter()
            .map(|(n, v)| (n.clone(), format_ident!("{}{}", v, FIELDS_SUFFIX)))
            .collect();

        // Build result.
        let mut result = ModelInfo {
            file,
            item,
            item_attributes,
            item_fields,
            document_name,
            api_document_names,
            collection_name,
            field_enum_name,
            api_field_enum_names,
        };

        // Analyze rest functions.
        result.analyze_rest_functions(items_iter)?;

        // Final checks.
        result.check_names(options)?;

        Ok(result)
    }

    // GETTERS ----------------------------------------------------------------

    pub fn check_all_db_fields_are_optional_or_properties(&self) -> bool {
        self.fields_in_db()
            .all(|field| field.field_type_kind.is_some())
    }

    pub fn check_all_db_variants_are_unit(&self) -> bool {
        self.fields_in_db().all(|field| field.inner_type.is_none())
    }

    pub fn check_all_api_variants_are_unit(&self, model: &str) -> bool {
        self.fields_in_model(model)
            .iter()
            .all(|field| field.inner_type.is_none())
    }

    pub fn fields_in_db(&self) -> Filter<Iter<'_, FieldInfo<'a>>, fn(&&'a FieldInfo<'a>) -> bool> {
        self.item_fields
            .iter()
            .filter(|field| !field.attributes.skip_in_model.contains(DB_MODEL_TAG))
    }

    pub fn fields_in_model(&self, model: &str) -> Vec<&FieldInfo<'a>> {
        self.item_fields
            .iter()
            .filter(|field| {
                !field.attributes.skip_in_model.contains(model) && field.db_name != "_key"
            })
            .collect()
    }

    pub fn get_key_field(&self) -> Option<&FieldInfo<'a>> {
        self.item_fields
            .iter()
            .find(|field| field.db_name == "_key" && *field.name() == "db_key")
    }

    // METHODS ----------------------------------------------------------------

    fn check_names(&self, options: &ModelOptions) -> Result<(), syn::Error> {
        let mut names = HashSet::with_capacity(self.item_fields.len());

        let rev = "_rev".to_string();
        let mutex = MUTEX_FIELD_DB_NAME.to_string();
        names.insert(&rev);

        if options.sync_level.is_document_active() {
            names.insert(&mutex);
        }

        for field in &self.item_fields {
            let db_name = &field.db_name;
            if names.contains(db_name) {
                return Err(Error::DuplicatedStructName(db_name.clone()).with_tokens(&field.node));
            }

            names.insert(db_name);
        }

        Ok(())
    }

    fn analyze_rest_functions(&mut self, mut items_iter: Iter<'a, Item>) -> Result<(), syn::Error> {
        if let Some(item) = items_iter.next() {
            return Err(Error::UnexpectedItem.with_tokens(item));
        }

        Ok(())
    }

    // STATIC METHODS ---------------------------------------------------------

    fn compute_api_document_name(name: &Ident, model: &str) -> Ident {
        let db_name = name.to_string();
        let model = from_snake_case_to_pascal_case(model);
        let api_name = db_name.replace(DB_MODEL_NAME, model.as_str());

        if db_name == api_name {
            format_ident!("{}{}", model, api_name, span = name.span())
        } else {
            format_ident!("{}", api_name, span = name.span())
        }
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

pub enum ModelNode<'a> {
    Struct(&'a ItemStruct),
    Enum(&'a ItemEnum),
}

impl<'a> ModelNode<'a> {
    // GETTERS ----------------------------------------------------------------

    pub fn ident(&self) -> &Ident {
        match self {
            ModelNode::Struct(v) => &v.ident,
            ModelNode::Enum(v) => &v.ident,
        }
    }

    pub fn visibility(&self) -> &Visibility {
        match self {
            ModelNode::Struct(v) => &v.vis,
            ModelNode::Enum(v) => &v.vis,
        }
    }

    pub fn generics(&self) -> &Generics {
        match self {
            ModelNode::Struct(v) => &v.generics,
            ModelNode::Enum(v) => &v.generics,
        }
    }

    pub fn attributes(&self) -> &Vec<Attribute> {
        match self {
            ModelNode::Struct(v) => &v.attrs,
            ModelNode::Enum(v) => &v.attrs,
        }
    }
}

impl<'a> ToTokens for ModelNode<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(match self {
            ModelNode::Struct(v) => v.to_token_stream(),
            ModelNode::Enum(v) => v.to_token_stream(),
        });
    }
}
