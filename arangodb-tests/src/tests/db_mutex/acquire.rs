use arangodb_types::traits::DBCollection;
use arangodb_types::traits::DBDocument;
use arangodb_types::types::{DBUuid, NullableOption};
use arangodb_types::types::dates::DBDateTime;
use arangodb_types::types::DBMutex;
use arangodb_types::utilities::{DBMutexError, DBMutexGuard};

use crate::tests::constants::NODE_ID;
use crate::tests::db_mutex::model::MutexDBDocument;
use crate::tests::db_mutex::TEST_RWLOCK;
use crate::tests::init_db_connection;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acquire_ok() {
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
    let (document, _mutex) = DBMutexGuard::<MutexDBDocument>::acquire_document(
        &document_key,
        &NODE_ID.into(),
        None,
        None,
        &collection,
    )
        .await
        .expect("Locking must succeed");

    // Check DB.
    assert_eq!(document.db_key, Some(document_key), "Incorrect db_key");
    assert!(document.db_mutex.is_value(), "Incorrect mutex");

    let db_mutex = document.db_mutex.unwrap_as_ref();
    assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
    assert!(!db_mutex.expiration.is_expired(), "Incorrect expiration");
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acquire_expired() {
    let _test_lock = TEST_RWLOCK.read().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    let document_key = DBUuid::new();
    let change_flag = DBUuid::new();
    let _document = MutexDBDocument {
        db_key: Some(document_key.clone()),
        db_mutex: NullableOption::Value(DBMutex {
            expiration: DBDateTime::now(),
            change_flag: change_flag.clone(),
            node: NODE_ID.into(),
        }),
        ..Default::default()
    }
        .insert(true, collection.as_ref())
        .await
        .expect("Cannot add preconditions to DB");

    // Execute.
    let (document, _mutex) = DBMutexGuard::<MutexDBDocument>::acquire_document(
        &document_key,
        &NODE_ID.into(),
        None,
        None,
        &collection,
    )
        .await
        .expect("Locking must succeed");

    // Check DB.
    assert_eq!(document.db_key, Some(document_key), "Incorrect db_key");
    assert!(document.db_mutex.is_value(), "Incorrect mutex");

    let db_mutex = document.db_mutex.unwrap_as_ref();
    assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
    assert!(!db_mutex.expiration.is_expired(), "Incorrect expiration");
    assert_ne!(db_mutex.change_flag, change_flag, "Incorrect change_flag");
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acquire_already_locked() {
    let _test_lock = TEST_RWLOCK.read().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    let document_key = DBUuid::new();
    let change_flag = DBUuid::new();
    let expiration = DBDateTime::now().after_seconds(200000);
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

    // Execute.
    let error = DBMutexGuard::<MutexDBDocument>::acquire_document(
        &document_key,
        &NODE_ID.into(),
        None,
        Some(1),
        &collection,
    )
        .await;

    match error {
        Ok(_) => panic!("Locking must fail"),
        Err(DBMutexError::Timeout) => {}
        _ => unreachable!(),
    }

    // Check DB.
    let document = collection
        .get_one_by_key(&document_key, None)
        .await
        .expect("There is an error trying to get the document")
        .expect("The document does not exist in DB");

    assert_eq!(document.db_key, Some(document_key), "Incorrect db_key");
    assert!(document.db_mutex.is_value(), "Incorrect mutex");

    let db_mutex = document.db_mutex.unwrap_as_ref();
    assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
    assert_eq!(db_mutex.expiration, expiration, "Incorrect expiration");
    assert_eq!(db_mutex.change_flag, change_flag, "Incorrect change_flag");
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acquire_missing() {
    let _test_lock = TEST_RWLOCK.read().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    let document_key = DBUuid::new();

    // Execute.
    let error = DBMutexGuard::<MutexDBDocument>::acquire_document(
        &document_key,
        &NODE_ID.into(),
        None,
        None,
        &collection,
    )
        .await;

    match error {
        Ok(_) => panic!("Locking must fail"),
        Err(DBMutexError::NotFound) => {}
        _ => unreachable!(),
    }
}
