use std::borrow::Cow;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DBDocumentField {
    Key,
    Id,
    Rev,
    To,
    From,
    Mutex,
}

impl DBDocumentField {
    // GETTERS ----------------------------------------------------------------

    pub fn path(&self) -> Cow<'static, str> {
        match self {
            DBDocumentField::Key => "_key".into(),
            DBDocumentField::Id => "_id".into(),
            DBDocumentField::Rev => "_rev".into(),
            DBDocumentField::To => "_to".into(),
            DBDocumentField::From => "_from".into(),
            DBDocumentField::Mutex => "_l".into(),
        }
    }
}
