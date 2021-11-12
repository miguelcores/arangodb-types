use std::borrow::Cow;

use arcstr::ArcStr;
use serde::{Deserialize, Serialize};

use arangodb_models::type_model;

use crate::traits::AQLMapping;
use crate::traits::DBNormalize;
use crate::traits::DBNormalizeResult;
use crate::types::dates::DBDateTime;
use crate::types::DBUuid;

type_model!(
    #![no_imports]

    /// This type stores a mutex for a document.
    pub struct DBMutex {
        #[db_name = "N"]
        pub node: ArcStr,
        #[db_name = "F"]
        pub change_flag: DBUuid,
        #[db_name = "E"]
        pub expiration: DBDateTime,
    }
);
