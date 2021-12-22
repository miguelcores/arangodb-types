use serde::{Deserialize, Serialize};

use crate::traits::{APIDocument, DBDocument};
use crate::types::DBReference;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum APIReference<T: APIDocument> {
    // Keep this order because otherwise Key will always be dereferenced in favour of Document
    // ignoring the rest of the fields.
    Document(Box<T>),
    Key(APIReferenceKey<T::Key>),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct APIReferenceKey<K> {
    id: K,
}

impl<T: APIDocument> APIReference<T> {
    // CONSTRUCTORS -----------------------------------------------------------

    pub fn new_key(id: T::Key) -> Self {
        Self::Key(APIReferenceKey { id })
    }

    // GETTERS ----------------------------------------------------------------

    pub fn key(&self) -> T::Key {
        match self {
            APIReference::Key(v) => v.id.clone(),
            APIReference::Document(v) => v.id().clone().expect("Missing id in reference"),
        }
    }

    pub fn is_key(&self) -> bool {
        matches!(self, APIReference::Key(_))
    }

    pub fn is_document(&self) -> bool {
        matches!(self, APIReference::Document(_))
    }

    pub fn unwrap_document_as_ref(&self) -> &T {
        match self {
            APIReference::Document(v) => v,
            _ => unreachable!("Cannot call 'unwrap_document_as_ref' without a document"),
        }
    }

    pub fn unwrap_document_as_mut_ref(&mut self) -> &mut T {
        match self {
            APIReference::Document(v) => v,
            _ => unreachable!("Cannot call 'unwrap_document_as_mut_ref' without a document"),
        }
    }

    // METHODS ----------------------------------------------------------------

    pub fn unwrap_document(self) -> Box<T> {
        match self {
            APIReference::Document(v) => v,
            _ => unreachable!("Cannot call 'unwrap_document' without a document"),
        }
    }

    pub fn and<F>(&mut self, mapper: F)
    where
        F: FnOnce(&mut Box<T>),
    {
        match self {
            APIReference::Document(v) => {
                mapper(v);
            }
            APIReference::Key(_) => {}
        }
    }

    pub fn map_to_db<F, R>(self, mapper: F) -> DBReference<R>
    where
        F: FnOnce(Box<T>) -> Box<R>,
        R: DBDocument<Key = T::Key>,
    {
        match self {
            APIReference::Document(v) => DBReference::Document(mapper(v)),
            APIReference::Key(v) => DBReference::new_key(v.id),
        }
    }
}

impl<T: APIDocument> PartialEq for APIReference<T> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            APIReference::Key(a) => match other {
                APIReference::Key(b) => a == b,
                APIReference::Document(_) => false,
            },
            APIReference::Document(a) => match other {
                APIReference::Key(_) => false,
                APIReference::Document(b) => a.id() == b.id(),
            },
        }
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

// TODO
// #[cfg(test)]
// mod test {
//     use std::sync::Arc;
//
//     use async_trait::async_trait;
//
//     use arangodb_models::model;
//
//     use crate::database::documents::ChallengeAPIDocument;
//     use crate::database::types::DBUuidType;
//     use crate::traits::AQLMapping;
//     use crate::traits::DBNormalize;
//     use crate::traits::DBNormalizeResult;
//     use crate::types::DBId;
//     use crate::types::DBUuid;
//
//     use super::*;
//
//     model! {
//         #![build_api]
//
//         pub struct Test {
//         }
//     }
//
//     #[test]
//     fn test_serialization() {
//         let key = DBUuid::new();
//         let key_ref = APIReference::<TestAPIDocument>::new_key(key.clone());
//         let doc_ref = APIReference::Document(Box::new(TestAPIDocument {
//             id: Some(key),
//             ..Default::default()
//         }));
//
//         let key_ref_serialization = serde_json::to_string(&key_ref).unwrap();
//         let doc_ref_serialization = serde_json::to_string(&doc_ref).unwrap();
//
//         assert_eq!(key_ref_serialization, doc_ref_serialization);
//     }
// }
