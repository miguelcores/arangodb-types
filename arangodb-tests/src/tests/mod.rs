use std::sync::Arc;

use arangodb_types::types::DBInfo;

use crate::tests::constants::{DB_NAME, DB_PASSWORD, DB_URL, DB_USERNAME};
use crate::tests::db_mutex::model::MutexCollection;

pub mod constants;
pub mod db_mutex;

async fn init_db_connection() -> Arc<DBInfo> {
    let db_info = DBInfo::connect(
        DB_URL.into(),
        DB_NAME.into(),
        DB_USERNAME.into(),
        DB_PASSWORD.into(),
    )
    .await
    .expect("Cannot connect with DB");

    let db_info = Arc::new(db_info);
    let _collection = MutexCollection::new(&db_info)
        .await
        .expect("Cannot create collection");

    db_info
}
