use std::time::Duration;

use tokio::time::sleep;

use arangodb_types::traits::DBCollection;
use arangodb_types::traits::DBDocument;
use arangodb_types::types::DBUuid;
use arangodb_types::utilities::BDMutexGuard;

use crate::tests::constants::NODE_ID;
use crate::tests::init_db_connection;
use crate::tests::remote_mutex::model::{MutexCollection, MutexDBDocument};
use crate::tests::remote_mutex::TEST_RWLOCK;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn release_list_auto() {
    let _test_lock = TEST_RWLOCK.read().await;
    let _db_info = init_db_connection().await;

    // Preconditions.
    let mut document_keys = Vec::new();

    for _ in 0..100_u8 {
        let document_key = DBUuid::new();
        let _document = MutexDBDocument {
            db_key: Some(document_key.clone()),
            ..Default::default()
        }
        .insert(true)
        .await
        .expect("Cannot add preconditions to DB");

        document_keys.push(document_key);
    }

    // Execute.
    {
        let (documents, _mutex) =
            BDMutexGuard::<MutexDBDocument>::acquire_list(&document_keys, &NODE_ID.into(), None)
                .await
                .expect("Locking must succeed");

        assert_eq!(documents.len(), document_keys.len(), "Incorrect length");
    }

    // Wait until the release is completed.
    sleep(Duration::from_secs(3)).await;

    // Check DB.
    let collection = MutexCollection::instance();
    for document_key in document_keys {
        let document = collection
            .get_one_by_key(&document_key, None)
            .await
            .expect("There is an error trying to get the document")
            .expect("The document does not exist in DB");

        assert!(!document.db_mutex.is_value(), "Incorrect mutex");
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn release_list_manually() {
    let _test_lock = TEST_RWLOCK.read().await;
    let _db_info = init_db_connection().await;

    // Preconditions.
    let mut document_keys = Vec::new();

    for _ in 0..100_u8 {
        let document_key = DBUuid::new();
        let _document = MutexDBDocument {
            db_key: Some(document_key.clone()),
            ..Default::default()
        }
        .insert(true)
        .await
        .expect("Cannot add preconditions to DB");

        document_keys.push(document_key);
    }

    // Execute.
    let (documents, mutex) =
        BDMutexGuard::<MutexDBDocument>::acquire_list(&document_keys, &NODE_ID.into(), None)
            .await
            .expect("Locking must succeed");

    assert_eq!(documents.len(), document_keys.len(), "Incorrect length");

    mutex.release();

    // Wait until the release is completed.
    sleep(Duration::from_secs(3)).await;

    // Check DB.
    let collection = MutexCollection::instance();
    for document_key in document_keys {
        let document = collection
            .get_one_by_key(&document_key, None)
            .await
            .expect("There is an error trying to get the document")
            .expect("The document does not exist in DB");

        assert!(!document.db_mutex.is_value(), "Incorrect mutex");
    }
}
