use std::fmt::Formatter;
use std::sync::Arc;

use arangodb_types::models::model;

use arangodb_types::traits::DBCollection;
use arangodb_types::types::DBInfo;
use arangodb_types::types::DBUuid;

#[derive(Debug)]
pub struct MutexCollection {
    db_info: Arc<DBInfo>,
}

impl MutexCollection {
    // CONSTRUCTORS -----------------------------------------------------------

    pub async fn new(db_info: &Arc<DBInfo>) -> Result<Arc<Self>, anyhow::Error> {
        let database = &db_info.database;

        // Initialize collection.
        let collection = Arc::new(MutexCollection {
            db_info: db_info.clone(),
        });
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
