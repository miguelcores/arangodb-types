use std::time::Duration;

use tokio::time::sleep;

use arangodb_types::traits::DBCollection;
use arangodb_types::traits::DBDocument;
use arangodb_types::types::DBUuid;
use arangodb_types::utilities::DBMutexGuard;

use crate::tests::constants::NODE_ID;
use crate::tests::db_mutex::model::MutexDBDocument;
use crate::tests::db_mutex::TEST_RWLOCK;
use crate::tests::init_db_connection;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn release_auto() {
    let _test_lock = TEST_RWLOCK.read().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    let document_key = DBUuid::new();
    let _document = MutexDBDocument {
        db_key: Some(document_key.clone()),
        ..Default::default()
    }
    .insert(true, collection.as_ref())
    .await
    .expect("Cannot add preconditions to DB");

    {
        // Execute.
        let (_document, _mutex) = DBMutexGuard::<MutexDBDocument>::acquire_document(
            &document_key,
            &NODE_ID.into(),
            None,
            None,
            &collection,
        )
        .await
        .expect("Locking must succeed");
    }

    // Wait until the release is completed.
    sleep(Duration::from_secs(3)).await;

    // Check DB.
    let document = collection
        .get_one_by_key(&document_key, None)
        .await
        .expect("There is an error trying to get the document")
        .expect("The document does not exist in DB");

    assert!(!document.db_mutex.is_value(), "Incorrect mutex");
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn release_manually() {
    let _test_lock = TEST_RWLOCK.read().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    let document_key = DBUuid::new();
    let _document = MutexDBDocument {
        db_key: Some(document_key.clone()),
        ..Default::default()
    }
    .insert(true, collection.as_ref())
    .await
    .expect("Cannot add preconditions to DB");

    // Execute.
    let (_document, mutex) = DBMutexGuard::<MutexDBDocument>::acquire_document(
        &document_key,
        &NODE_ID.into(),
        None,
        None,
        &collection,
    )
    .await
    .expect("Locking must succeed");

    mutex.release();

    // Wait until the release is completed.
    sleep(Duration::from_secs(3)).await;

    // Check DB.
    let document = collection
        .get_one_by_key(&document_key, None)
        .await
        .expect("There is an error trying to get the document")
        .expect("The document does not exist in DB");

    assert!(!document.db_mutex.is_value(), "Incorrect mutex");
}
