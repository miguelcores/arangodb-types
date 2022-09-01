use arangodb_types::traits::DBCollection;
use arangodb_types::traits::DBDocument;
use arangodb_types::types::{DBDateTime, DBMutex};
use arangodb_types::types::{DBUuid, NullableOption};
use arangodb_types::utilities::DBMutexGuard;

use crate::tests::constants::NODE_ID;
use crate::tests::db_mutex::model::MutexDBDocument;
use crate::tests::db_mutex::TEST_RWLOCK;
use crate::tests::init_db_connection;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acquire_list_ok() {
    let _test_lock = TEST_RWLOCK.read().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    let mut document_keys = Vec::new();

    for _ in 0..100_u8 {
        let document_key = DBUuid::new();
        let _document = MutexDBDocument {
            db_key: Some(document_key.clone()),
            ..Default::default()
        }
        .insert(true, collection.as_ref())
        .await
        .expect("Cannot add preconditions to DB");

        document_keys.push(document_key);
    }

    // Execute.
    let (documents, _mutex) = DBMutexGuard::<MutexDBDocument>::acquire_list(
        &document_keys,
        &NODE_ID.into(),
        None,
        &collection,
    )
    .await
    .expect("Locking must succeed");

    assert_eq!(documents.len(), document_keys.len(), "Incorrect length");

    // Check DB.
    for (document_key, document) in document_keys.iter().zip(documents) {
        let document = document.expect("Incorrect document");

        assert_eq!(
            document.db_key,
            Some(document_key.clone()),
            "Incorrect db_key"
        );
        assert!(document.db_mutex.is_value(), "Incorrect mutex");

        let db_mutex = document.db_mutex.unwrap_as_ref();
        assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
        assert!(!db_mutex.expiration.is_expired(), "Incorrect expiration");
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acquire_list_mix() {
    let _test_lock = TEST_RWLOCK.read().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    let mut document_keys = Vec::new();
    let expiration = DBDateTime::now().after_seconds(200000);
    let change_flag = DBUuid::new();

    for i in 0..100_u8 {
        let document_key = DBUuid::new();

        match i % 3 {
            0 => {
                // Active.
                let _document = MutexDBDocument {
                    db_key: Some(document_key.clone()),
                    ..Default::default()
                }
                .insert(true, collection.as_ref())
                .await
                .expect("Cannot add preconditions to DB");
            }
            1 => {
                let _document = MutexDBDocument {
                    db_key: Some(document_key.clone()),
                    db_mutex: NullableOption::Value(DBMutex {
                        expiration: expiration.clone(),
                        change_flag: change_flag.clone(),
                        node: NODE_ID.into(),
                    }),
                    ..Default::default()
                }
                .insert(true, collection.as_ref())
                .await
                .expect("Cannot add preconditions to DB");
            }
            _ => {}
        }

        document_keys.push(document_key);
    }

    // Execute.
    let (documents, _mutex) = DBMutexGuard::<MutexDBDocument>::acquire_list(
        &document_keys,
        &NODE_ID.into(),
        None,
        &collection,
    )
    .await
    .expect("Locking must succeed");

    assert_eq!(documents.len(), document_keys.len(), "Incorrect length");

    // Check DB.
    for (i, (document_key, document)) in document_keys.iter().zip(documents).enumerate() {
        match i % 3 {
            0 => {
                let document = document.expect("Incorrect available document");

                assert_eq!(
                    document.db_key,
                    Some(document_key.clone()),
                    "Incorrect db_key"
                );
                assert!(document.db_mutex.is_value(), "Incorrect mutex");

                let db_mutex = document.db_mutex.unwrap_as_ref();
                assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
                assert!(!db_mutex.expiration.is_expired(), "Incorrect expiration");
            }
            1 => {
                assert!(document.is_none(), "Incorrect already locked document");

                let document = collection
                    .get_one_by_key(document_key, None)
                    .await
                    .expect("There is an error trying to get the document")
                    .expect("The document does not exist in DB");

                assert_eq!(
                    document.db_key,
                    Some(document_key.clone()),
                    "Incorrect db_key"
                );
                assert!(document.db_mutex.is_value(), "Incorrect mutex");

                let db_mutex = document.db_mutex.unwrap_as_ref();
                assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
                assert_eq!(db_mutex.expiration, expiration, "Incorrect expiration");
                assert_eq!(db_mutex.change_flag, change_flag, "Incorrect change_flag");
            }
            _ => {
                assert!(document.is_none(), "Incorrect missing document");
            }
        }
    }
}
