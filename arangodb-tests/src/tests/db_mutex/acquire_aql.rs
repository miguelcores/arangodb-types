use arangodb_types::aql::{AqlLimit, AQL_DOCUMENT_ID};
use arangodb_types::traits::DBCollection;
use arangodb_types::traits::DBDocument;
use arangodb_types::types::{DBUuid, NullableOption};
use arangodb_types::utilities::BDMutexGuard;

use crate::tests::constants::NODE_ID;
use crate::tests::db_mutex::model::MutexDBDocument;
use crate::tests::db_mutex::model::MutexDBDocumentField;
use crate::tests::db_mutex::TEST_RWLOCK;
use crate::tests::init_db_connection;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acquire_user_aql_ok() {
    let _test_lock = TEST_RWLOCK.write().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    collection
        .truncate()
        .await
        .expect("Cannot truncate the collection");

    for _ in 0..15_u8 {
        let document_key = DBUuid::new();
        let _document = MutexDBDocument {
            db_key: Some(document_key.clone()),
            value: NullableOption::Value(15),
            ..Default::default()
        }
        .insert(true, collection.as_ref())
        .await
        .expect("Cannot add preconditions to DB");
    }

    for _ in 0..20_u8 {
        let document_key = DBUuid::new();
        let _document = MutexDBDocument {
            db_key: Some(document_key.clone()),
            value: NullableOption::Value(20),
            ..Default::default()
        }
        .insert(true, collection.as_ref())
        .await
        .expect("Cannot add preconditions to DB");
    }

    // FILTER i.<state> == <banned>
    let filter = format!(
        "{}.{} == 20",
        AQL_DOCUMENT_ID,
        MutexDBDocumentField::Value(None).path(),
    );

    // Execute.
    let (documents, _mutex) = BDMutexGuard::<MutexDBDocument>::acquire_aql(
        Some(filter.as_str()),
        None,
        None,
        &NODE_ID.into(),
        None,
        &collection,
    )
    .await
    .expect("Locking must succeed");

    assert_eq!(documents.len(), 20, "Incorrect length");

    // Check DB.
    for document in documents {
        assert!(document.db_mutex.is_value(), "Incorrect mutex");
        assert_eq!(document.value, NullableOption::Value(20), "Incorrect value");

        let db_mutex = document.db_mutex.unwrap_as_ref();
        assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
        assert!(!db_mutex.expiration.is_expired(), "Incorrect expiration");
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acquire_user_aql_with_limits_ok() {
    let _test_lock = TEST_RWLOCK.write().await;
    let (_db_info, collection) = init_db_connection().await;

    // Preconditions.
    collection
        .truncate()
        .await
        .expect("Cannot truncate the collection");

    for _ in 0..20_u8 {
        let document_key = DBUuid::new();
        let _document = MutexDBDocument {
            db_key: Some(document_key.clone()),
            value: NullableOption::Value(20),
            ..Default::default()
        }
        .insert(true, collection.as_ref())
        .await
        .expect("Cannot add preconditions to DB");
    }

    // Limits.
    let limits = AqlLimit {
        offset: None,
        count: 10,
    };

    // Execute.
    let (documents, _mutex) = BDMutexGuard::<MutexDBDocument>::acquire_aql(
        None,
        None,
        Some(limits),
        &NODE_ID.into(),
        None,
        &collection,
    )
    .await
    .expect("Locking must succeed");

    assert_eq!(documents.len(), 10, "Incorrect length");

    // Check DB.
    for document in documents {
        assert!(document.db_mutex.is_value(), "Incorrect mutex");
        assert_eq!(document.value, NullableOption::Value(20), "Incorrect value");

        let db_mutex = document.db_mutex.unwrap_as_ref();
        assert_eq!(&db_mutex.node, &NODE_ID, "Incorrect node");
        assert!(!db_mutex.expiration.is_expired(), "Incorrect expiration");
    }
}
