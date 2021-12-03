use arangodb_types::constants::MUTEX_ALIVE_INTERVAL;
use arangodb_types::traits::DBCollection;
use arangodb_types::traits::DBDocument;
use arangodb_types::types::DBUuid;
use arangodb_types::utilities::BDMutexGuard;
use std::time::Duration;
use tokio::time::sleep;

use crate::tests::constants::NODE_ID;
use crate::tests::db_mutex::model::{MutexCollection, MutexDBDocument};
use crate::tests::db_mutex::TEST_RWLOCK;
use crate::tests::init_db_connection;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn alive_ok() {
    let _test_lock = TEST_RWLOCK.read().await;
    let _db_info = init_db_connection().await;

    // Preconditions.
    let document_key = DBUuid::new();
    let _document = MutexDBDocument {
        db_key: Some(document_key.clone()),
        ..Default::default()
    }
    .insert(true)
    .await
    .expect("Cannot add preconditions to DB");

    // Execute.
    let (document, _mutex) = BDMutexGuard::<MutexDBDocument>::acquire_document(
        &document_key,
        &NODE_ID.into(),
        None,
        None,
    )
    .await
    .expect("Locking must succeed");

    // Check DB.
    assert!(document.db_mutex.is_value(), "Incorrect mutex");

    let prev_expiration = document.db_mutex.unwrap_as_ref().expiration.clone();

    // Wait until the alive is completed.
    sleep(Duration::from_secs(MUTEX_ALIVE_INTERVAL + 1)).await;

    // Check DB 2.
    let collection = MutexCollection::instance();
    let document = collection
        .get_one_by_key(&document_key, None)
        .await
        .expect("There is an error trying to get the document")
        .expect("The document does not exist in DB");

    assert_eq!(document.db_key, Some(document_key), "Incorrect db_key");
    assert!(document.db_mutex.is_value(), "Incorrect mutex");

    let db_mutex = document.db_mutex.unwrap_as_ref();
    assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
    assert_ne!(db_mutex.expiration, prev_expiration, "Incorrect expiration");
}
