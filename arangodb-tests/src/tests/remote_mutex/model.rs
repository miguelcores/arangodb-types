use std::error::Error;
use std::fmt::Formatter;
use std::ops::Deref;
use std::sync::Arc;

use arcstr::ArcStr;
use lazy_static::lazy_static;

use arangodb_models::model;
use arangodb_types::traits::DBCollection;
use arangodb_types::types::DBInfo;
use arangodb_types::types::DBUuid;

lazy_static! {
    static ref COLLECTION: std::sync::Mutex<Option<Arc<MutexCollection>>> =
        std::sync::Mutex::new(None);
}

#[derive(Debug)]
pub struct MutexCollection {
    db_info: Arc<DBInfo>,
}

impl MutexCollection {
    // CONSTRUCTORS -----------------------------------------------------------

    pub async fn new(db_info: &Arc<DBInfo>) -> Result<Arc<Self>, Box<dyn Error>> {
        let database = &db_info.database;

        // Initialize collection.
        let mut collection = COLLECTION.lock().unwrap();
        let collection = match collection.deref() {
            Some(v) => v.clone(),
            None => {
                let value = Arc::new(MutexCollection {
                    db_info: db_info.clone(),
                });

                *collection = Some(value.clone());

                value
            }
        };
        let _ = database.create_collection(MutexCollection::name()).await; // Ignore error because it means already created.

        Ok(collection)
    }
}

impl DBCollection for MutexCollection {
    type Document = MutexDBDocument;

    fn name() -> &'static str {
        "Mutexes"
    }

    fn db_info(&self) -> &Arc<DBInfo> {
        &self.db_info
    }

    fn instance() -> Arc<MutexCollection> {
        COLLECTION.lock().unwrap().as_ref().unwrap().clone()
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum CollectionKind {
    Mutexes,
}

impl std::fmt::Display for CollectionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectionKind::Mutexes => write!(f, "Mutexes"),
        }
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

model!(
    #![sync_level = "document"]
    #![collection_kind = "Mutexes"]

    pub struct Mutex {
        #[db_name = "_key"]
        pub db_key: Option<DBUuid>,

        #[db_name = "V"]
        pub value: NullableOption<u64>,
    }
);
