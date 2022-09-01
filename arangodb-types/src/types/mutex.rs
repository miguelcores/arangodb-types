use crate::traits::AQLMapping;
use crate::types::{DBDateTime, DBUuid};
use arangodb_models::type_model;
use arcstr::ArcStr;

type_model!(
    #![relative_imports]

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
