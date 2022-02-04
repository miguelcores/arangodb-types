use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use arcstr::ArcStr;
use rand::Rng;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;

pub use errors::*;

use crate::aql::{
    AQL_DOCUMENT_ID, AQL_NEW_ID, AqlBuilder, AqlLet, AqlLetKind, AqlLimit, AqlReturn, AqlSort,
    AqlUpdate,
};
use crate::constants::{
    MUTEX_ACQUIRE_MAX_INTERVAL, MUTEX_ACQUIRE_MIN_INTERVAL, MUTEX_ALIVE_INTERVAL, MUTEX_EXPIRATION,
};
use crate::documents::DBDocumentField;
use crate::traits::{DBCollection, DBSynchronizedDocument};
use crate::types::{DBMutex, DBMutexField, DBUuid, NullableOption};
use crate::types::dates::DBDateTime;

mod errors;

pub struct DBMutexGuard<T: 'static + DBSynchronizedDocument<'static>> {
    inner: Arc<Mutex<BDMutexGuardInner<T>>>,
}

struct BDMutexGuardInner<T: 'static + DBSynchronizedDocument<'static>> {
    node_id: ArcStr,
    elements: HashSet<T::Key>,
    change_flag: DBUuid,
    alive_job: Option<JoinHandle<()>>,
    collection: Arc<T::Collection>,
}

impl<T: 'static + DBSynchronizedDocument<'static>> DBMutexGuard<T> {
    // CONSTRUCTORS -----------------------------------------------------------

    /// # Safety
    /// This method won't panic but can cause incorrect behaviour if not used wisely.
    pub async unsafe fn new(
        key: &T::Key,
        node_id: &ArcStr,
        change_flag: DBUuid,
        collection: &Arc<T::Collection>,
    ) -> DBMutexGuard<T> {
        let guard = Self {
            inner: Arc::new(Mutex::new(BDMutexGuardInner {
                node_id: node_id.clone(),
                elements: {
                    let mut set = HashSet::new();
                    set.insert(key.clone());
                    set
                },
                change_flag,
                alive_job: None,
                collection: collection.clone(),
            })),
        };

        // Launch alive action.
        {
            let mut lock = guard.inner.lock().await;
            lock.alive_job = Some(tokio::spawn(Self::alive_action(guard.inner.clone())));
        }

        guard
    }

    /// Acquires a single document optionally with a timeout.
    pub async fn acquire_document(
        key: &T::Key,
        node_id: &ArcStr,
        fields: Option<&T>,
        timeout: Option<u64>,
        collection: &Arc<T::Collection>,
    ) -> Result<(T, DBMutexGuard<T>), DBMutexError> {
        let time_out = timeout.map(|v| DBDateTime::now().after_seconds(v));
        let mut checked_doc_exists = false;

        loop {
            // Check timeout.
            if time_out.as_ref().map(|v| v.is_expired()).unwrap_or(false) {
                return Err(DBMutexError::Timeout);
            }

            // Prepare filter.
            let (mut list, mutex) = Self::acquire_list(&[key.clone()], node_id, fields, collection).await?;

            let value = list.pop().unwrap();

            match value {
                Some(v) => return Ok((v, mutex)),
                None => {
                    if !checked_doc_exists {
                        // Check the document exists and exit if not.
                        // This prevents waiting until timeout when the document
                        // is not present in the DB.
                        let exists_in_db = collection.exists_by_key(key).await?;

                        if !exists_in_db {
                            return Err(DBMutexError::NotFound);
                        }

                        checked_doc_exists = true;
                    }

                    // Sleep for a while to retry later.
                    let time = {
                        let mut rng = rand::thread_rng();
                        rng.gen_range(MUTEX_ACQUIRE_MIN_INTERVAL..MUTEX_ACQUIRE_MAX_INTERVAL)
                    };
                    sleep(Duration::from_millis(time)).await;
                }
            }
        }
    }

    /// Acquires a single document optionally with a timeout.
    pub async fn acquire_or_create_document<F: FnOnce() -> T>(
        key: &T::Key,
        node_id: &ArcStr,
        fields: Option<&T>,
        timeout: Option<u64>,
        collection: &Arc<T::Collection>,
        default: F,
    ) -> Result<(T, DBMutexGuard<T>), DBMutexError> {
        match Self::acquire_document(key, node_id, fields, timeout, collection).await {
            Ok(v) => Ok(v),
            Err(e) => {
                match e {
                    DBMutexError::NotFound => {
                        // Persist document with mutex.
                        let mut document = default();
                        let now = DBDateTime::now();
                        let expiration = now.after_seconds(MUTEX_EXPIRATION);
                        let change_flag = DBUuid::new();

                        document.set_mutex(NullableOption::Value(DBMutex {
                            node: node_id.clone(),
                            expiration,
                            change_flag: change_flag.clone(),
                        }));

                        let final_document = document.insert(false, collection).await?;

                        let guard = Self {
                            inner: Arc::new(Mutex::new(BDMutexGuardInner {
                                node_id: node_id.clone(),
                                elements: {
                                    let mut set = HashSet::new();
                                    set.insert(final_document.db_key().clone().unwrap());
                                    set
                                },
                                change_flag,
                                alive_job: None,
                                collection: collection.clone(),
                            })),
                        };

                        // Launch alive action.
                        {
                            let mut lock = guard.inner.lock().await;
                            lock.alive_job = Some(tokio::spawn(Self::alive_action(guard.inner.clone())));
                        }

                        Ok((final_document, guard))
                    }
                    DBMutexError::Timeout => Err(DBMutexError::Timeout),
                    DBMutexError::Other(e) => Err(DBMutexError::Other(e))
                }
            }
        }
    }

    /// Acquires a list of documents, locking them in the process. If any of the documents couldn't
    /// be locked, a None is returned.
    pub async fn acquire_list(
        keys: &[T::Key],
        node_id: &ArcStr,
        fields: Option<&T>,
        collection: &Arc<T::Collection>,
    ) -> Result<(Vec<Option<T>>, DBMutexGuard<T>), anyhow::Error> {
        // Shortcut for empty sets.
        if keys.is_empty() {
            return Ok((
                Vec::new(),
                Self {
                    inner: Arc::new(Mutex::new(BDMutexGuardInner {
                        node_id: node_id.clone(),
                        elements: HashSet::new(),
                        change_flag: DBUuid::new(),
                        alive_job: Some(tokio::spawn(async {})),
                        collection: collection.clone(),
                    })),
                },
            ));
        }

        let collection_name = T::Collection::name();
        let mutex_path = DBDocumentField::Mutex.path();

        let now = DBDateTime::now();
        let expiration = now.after_seconds(MUTEX_EXPIRATION);

        // FOR i IN <keys>
        //     LET o = Document(<collection>, i)
        //     FILTER o != null && o.<mutex.expiration> <= <now>
        //     UPDATE i WITH { <mutex>: { <node>: <node_id>, <expiration>: <expiration>, <change_flag>: <change_flag> } } IN <collection> OPTIONS { mergeObjects: true, ignoreErrors: true }
        //     RETURN NEW
        let document_key = "o";
        let change_flag = DBUuid::new();
        let mut aql = AqlBuilder::new_for_in_list(AQL_DOCUMENT_ID, keys);
        aql.let_step(AqlLet {
            variable: document_key,
            expression: AqlLetKind::Expression(
                format!("DOCUMENT({}, {})", collection_name, AQL_DOCUMENT_ID).into(),
            ),
        });
        aql.filter_step(
            format!(
                "{} != null && {}.{}.{} <= {}",
                document_key,
                document_key,
                mutex_path,
                DBMutexField::Expiration(None).path(),
                serde_json::to_string(&now).unwrap()
            ).into(),
        );
        aql.update_step(
            AqlUpdate::new(
                AQL_DOCUMENT_ID.into(),
                collection_name,
                format!(
                    "{{ {}: {{ {}: {}, {}: {}, {}: {} }} }}",
                    mutex_path,
                    DBMutexField::Node(None).path(),
                    serde_json::to_string(node_id).unwrap(),
                    DBMutexField::Expiration(None).path(),
                    serde_json::to_string(&expiration).unwrap(),
                    DBMutexField::ChangeFlag(None).path(),
                    serde_json::to_string(&change_flag).unwrap()
                ).into(),
            ).apply_ignore_errors(true),
        );

        if let Some(fields) = fields {
            aql.return_step_with_fields(AQL_NEW_ID, fields);
        } else {
            aql.return_step(AqlReturn::new_updated());
        }

        let result = collection.send_generic_aql::<Option<T>>(&aql).await?;
        let result_ids = result.results.iter().filter_map(|v| match v {
            Some(v) => v.db_key().clone(),
            None => None,
        }).collect();

        let guard = Self {
            inner: Arc::new(Mutex::new(BDMutexGuardInner {
                node_id: node_id.clone(),
                elements: result_ids,
                change_flag,
                alive_job: None,
                collection: collection.clone(),
            })),
        };

        // Adjust the result list to contain every element in its position.
        let mut index = 0;
        let mut results: Vec<Option<T>> = result.results;
        for key in keys {
            let result = match results.get(index) {
                Some(Some(v)) => v,
                Some(None) => {
                    continue;
                }
                None => {
                    results.push(None);
                    continue;
                }
            };

            if result.db_key().as_ref() != Some(key) {
                results.insert(index, None);
            }

            index += 1;
        }

        // Launch alive action.
        {
            let mut lock = guard.inner.lock().await;
            lock.alive_job = Some(tokio::spawn(Self::alive_action(guard.inner.clone())));
        }

        Ok((results, guard))
    }

    /// Acquires a list of documents filtering them using a limited AQL.
    pub async fn acquire_aql(
        filter: Option<&str>,
        sort: Option<Vec<AqlSort<'_>>>,
        limits: Option<AqlLimit>,
        node_id: &ArcStr,
        fields: Option<&T>,
        collection: &Arc<T::Collection>,
    ) -> Result<(Vec<T>, DBMutexGuard<T>), anyhow::Error> {
        let collection_name = T::Collection::name();
        let mutex_path = DBDocumentField::Mutex.path();

        let now = DBDateTime::now();
        let expiration = now.after_seconds(MUTEX_EXPIRATION);

        // FOR i IN <collection>
        //     <custom_filter>
        //     FILTER i.<mutex.expiration> <= <now>
        //     <custom_sort>
        //     <custom_limit>
        //     UPDATE i WITH { <mutex>: { <node>: <node_id>, <expiration>: <expiration>, <change_flag>: <change_flag> } } IN <collection> OPTIONS { mergeObjects: true, ignoreErrors: true }
        //     FILTER NEW != null
        //     RETURN NEW
        let change_flag = DBUuid::new();
        let mut aql = AqlBuilder::new_for_in_collection(AQL_DOCUMENT_ID, collection_name);

        if let Some(filter) = filter {
            aql.filter_step(filter.into());
        }
        aql.filter_step(
            format!(
                "{}.{}.{} <= {}",
                AQL_DOCUMENT_ID,
                mutex_path,
                DBMutexField::Expiration(None).path(),
                serde_json::to_string(&now).unwrap()
            ).into(),
        );

        if let Some(sort) = sort {
            aql.sort_step(sort);
        }

        if let Some(limits) = limits {
            aql.limit_step(limits);
        }

        aql.update_step(
            AqlUpdate::new_document(
                collection_name,
                format!(
                    "{{ {}: {{ {}: {}, {}: {}, {}: {} }} }}",
                    mutex_path,
                    DBMutexField::Node(None).path(),
                    serde_json::to_string(&node_id).unwrap(),
                    DBMutexField::Expiration(None).path(),
                    serde_json::to_string(&expiration).unwrap(),
                    DBMutexField::ChangeFlag(None).path(),
                    serde_json::to_string(&change_flag).unwrap()
                ).into(),
            ).apply_ignore_errors(true),
        );
        aql.filter_step(format!("{} != null", AQL_NEW_ID).into());

        if let Some(fields) = fields {
            aql.return_step_with_fields(AQL_NEW_ID, fields);
        } else {
            aql.return_step(AqlReturn::new_updated());
        }

        let result = collection.send_generic_aql::<T>(&aql).await?;
        let result_ids = result.results.iter().map(|v| v.db_key().as_ref().unwrap().clone()).collect();

        let guard = Self {
            inner: Arc::new(Mutex::new(BDMutexGuardInner {
                node_id: node_id.clone(),
                elements: result_ids,
                change_flag,
                alive_job: None,
                collection: collection.clone(),
            })),
        };

        // Launch alive action.
        {
            let mut lock = guard.inner.lock().await;
            lock.alive_job = Some(tokio::spawn(Self::alive_action(guard.inner.clone())));
        }

        Ok((result.results, guard))
    }

    // GETTERS ----------------------------------------------------------------

    /// Whether the mutex is locking any document or not.
    pub async fn is_empty(&self) -> bool {
        let lock = self.inner.lock().await;
        lock.elements.is_empty()
    }

    // METHODS ----------------------------------------------------------------

    /// Checks whether a document is locked or not.
    pub async fn contains_key(&self, key: &T::Key) -> bool {
        let lock = self.inner.lock().await;
        lock.elements.get(key).is_some()
    }

    /// This method removes the keys from the lock. It is useful to prevent errors when
    /// locked documents are removed before releasing the lock.
    ///
    /// # Safety
    /// This method can cause documents to be locked during minutes.
    pub async unsafe fn remove_keys(&self, keys: &[T::Key]) {
        let mut lock = self.inner.lock().await;

        for key in keys {
            lock.elements.remove(key);
        }

        // Abort alive job if empty.
        if lock.elements.is_empty() {
            if let Some(alive_job) = lock.alive_job.take() {
                alive_job.abort();
            }
        }
    }

    /// This method removes all the keys from the lock. It is useful to prevent errors when
    /// locked documents are removed before releasing the lock.
    ///
    /// # Safety
    /// This method can cause documents to be locked during minutes.
    pub async unsafe fn clear_keys(&self) {
        let mut lock = self.inner.lock().await;
        lock.elements.clear();

        // Abort alive job if empty.
        if let Some(alive_job) = lock.alive_job.take() {
            alive_job.abort();
        }
    }

    /// Moves the keys from the current mutex into another one.
    pub async fn pop(&mut self, keys: &[T::Key]) -> Option<DBMutexGuard<T>> {
        let mut lock = self.inner.lock().await;

        let new_elements = keys.iter().filter_map(|key| {
            if lock.elements.remove(key) {
                Some(key.clone())
            } else {
                None
            }
        }).collect::<HashSet<_>>();

        // Abort alive job if empty.
        if lock.elements.is_empty() {
            if let Some(alive_job) = lock.alive_job.take() {
                alive_job.abort();
            }
        }

        let guard = if new_elements.is_empty() {
            Self {
                inner: Arc::new(Mutex::new(BDMutexGuardInner {
                    node_id: lock.node_id.clone(),
                    elements: new_elements,
                    change_flag: DBUuid::new(),
                    alive_job: Some(tokio::spawn(async {})),
                    collection: lock.collection.clone(),
                })),
            }
        } else {
            let guard = Self {
                inner: Arc::new(Mutex::new(BDMutexGuardInner {
                    node_id: lock.node_id.clone(),
                    elements: new_elements,
                    change_flag: lock.change_flag.clone(),
                    alive_job: None,
                    collection: lock.collection.clone(),
                })),
            };

            // Launch alive action.
            {
                let mut lock = guard.inner.lock().await;
                lock.alive_job = Some(tokio::spawn(Self::alive_action(guard.inner.clone())));
            }

            guard
        };

        Some(guard)
    }

    /// Manually releases the mutex.
    pub fn release(self) {
        tokio::spawn(Self::release_action(self.inner.clone()));
    }

    // STATIC METHODS ---------------------------------------------------------

    async fn alive_action(mutex: Arc<Mutex<BDMutexGuardInner<T>>>) {
        loop {
            // Sleep for interval.
            sleep(Duration::from_secs(MUTEX_ALIVE_INTERVAL)).await;

            let mut lock = mutex.lock().await;
            if lock.alive_job.is_none() {
                // The mutex has been already released.
                return;
            }

            // Avoid doing unnecessary DB requests.
            if lock.elements.is_empty() {
                return;
            }

            let collection = &lock.collection;
            let node_id = &lock.node_id;
            let now = DBDateTime::now();
            let expiration = now.after_seconds(MUTEX_EXPIRATION);
            let keys = &lock.elements;

            // FOR i IN <keys>
            //     LET o = Document(<collection>, i)
            //     FILTER o != null && o.<mutex.node> == <node> && o.<mutex.change_flag> == <change_flag>
            //     UPDATE i WITH { <mutex>: { <expiration>: <expiration> } } IN <collection> OPTIONS { mergeObjects: true, ignoreErrors: true }
            //     FILTER NEW != null
            //     RETURN i
            let document_key = "o";
            let collection_name = T::Collection::name();
            let mutex_path = DBDocumentField::Mutex.path();
            let mut aql = AqlBuilder::new_for_in_set(AQL_DOCUMENT_ID, keys);
            aql.let_step(AqlLet {
                variable: document_key,
                expression: AqlLetKind::Expression(
                    format!("DOCUMENT({}, {})", collection_name, AQL_DOCUMENT_ID).into(),
                ),
            });
            aql.filter_step(
                format!(
                    "{} != null && {}.{}.{} == {} && {}.{}.{} == {}",
                    document_key,
                    document_key,
                    mutex_path,
                    DBMutexField::Node(None).path(),
                    serde_json::to_string(&node_id).unwrap(),
                    document_key,
                    mutex_path,
                    DBMutexField::ChangeFlag(None).path(),
                    serde_json::to_string(&lock.change_flag).unwrap(),
                ).into(),
            );
            aql.update_step(
                AqlUpdate::new_document(
                    collection_name,
                    format!(
                        "{{ {}: {{ {}: {} }} }}",
                        mutex_path,
                        DBMutexField::Expiration(None).path(),
                        serde_json::to_string(&expiration).unwrap(),
                    ).into(),
                ).apply_ignore_errors(true),
            );
            aql.filter_step(format!("{} != null", AQL_NEW_ID).into());
            aql.return_step(AqlReturn::new_document());

            let result = match collection.send_generic_aql::<T::Key>(&aql).await {
                Ok(v) => v.results,
                Err(e) => {
                    let keys = keys.iter().map(|v| v.to_string()).collect::<Vec<_>>();
                    lock.alive_job.take().unwrap().abort();
                    log::error!(
                        "Error while keeping alive document mutexes in DB. Keys: {:?}, Error: {}",
                        keys,
                        e
                    );
                    return;
                }
            };
            let result: HashSet<_> = result.into_iter().collect();

            if result.is_empty() {
                lock.alive_job.take().unwrap().abort();
                return;
            }

            lock.elements = result;
        }
    }

    async fn release_action(mutex: Arc<Mutex<BDMutexGuardInner<T>>>) {
        let mut lock = mutex.lock().await;
        if lock.alive_job.is_none() {
            // The mutex has been already released.
            return;
        }

        // Abort the alive job.
        lock.alive_job.take().unwrap().abort();

        // Avoid doing unnecessary DB requests.
        if lock.elements.is_empty() {
            return;
        }

        let collection = &lock.collection;
        let node_id = &lock.node_id;
        let keys = &lock.elements;

        // FOR i IN <keys>
        //     LET o = Document(<collection>, i)
        //     FILTER o != null && o.<mutex.node> == <node> && o.<mutex.change_flag> == <change_flag>
        //     UPDATE i WITH { <mutex>: null } IN <collection> OPTIONS { mergeObjects: true, keepNulls: false, ignoreErrors: true }
        //     FILTER NEW != null
        //     RETURN i
        let document_key = "o";
        let collection_name = T::Collection::name();
        let mutex_path = DBDocumentField::Mutex.path();
        let mut aql = AqlBuilder::new_for_in_set(AQL_DOCUMENT_ID, keys);
        aql.let_step(AqlLet {
            variable: document_key,
            expression: AqlLetKind::Expression(
                format!("DOCUMENT({}, {})", collection_name, AQL_DOCUMENT_ID).into(),
            ),
        });
        aql.filter_step(
            format!(
                "{} != null && {}.{}.{} == {} && {}.{}.{} == {}",
                document_key,
                document_key,
                mutex_path,
                DBMutexField::Node(None).path(),
                serde_json::to_string(node_id).unwrap(),
                document_key,
                mutex_path,
                DBMutexField::ChangeFlag(None).path(),
                serde_json::to_string(&lock.change_flag).unwrap(),
            ).into(),
        );
        aql.update_step(
            AqlUpdate::new_document(
                collection_name,
                format!("{{ {}: null }}", mutex_path).into(),
            ).apply_ignore_errors(true),
        );
        aql.filter_step(format!("{} != null", AQL_NEW_ID).into());
        aql.return_step(AqlReturn::new_document());

        let result = match collection.send_generic_aql::<T::Key>(&aql).await {
            Ok(v) => v.results,
            Err(e) => {
                let keys = keys.iter().map(|v| v.to_string()).collect::<Vec<_>>();
                log::error!(
                    "Error while releasing document mutexes in DB. Keys: {:?}, Error: {}",
                    keys,
                    e
                );
                return;
            }
        };
        let result: HashSet<_> = result.iter().collect();

        for element_id in keys {
            if !result.contains(element_id) {
                log::error!(
                    "The mutex (Collection: {}, Id: {}, ChangeFlag: {}) couldn't be released",
                    collection_name,
                    element_id.to_string(),
                    lock.change_flag
                );
            }
        }
    }

    pub async fn release_all_mutexes(node_id: &str, collection: &Arc<T::Collection>) {
        // FOR i IN <collection>
        //     FILTER i.<mutex.node> == <node>
        //     UPDATE i WITH { <mutex>: null } IN <collection> OPTIONS { mergeObjects: true, keepNulls: false, ignoreErrors: true }
        let mutex_path = DBDocumentField::Mutex.path();
        let collection_name = T::Collection::name();
        let mut aql = AqlBuilder::new_for_in_collection(AQL_DOCUMENT_ID, collection_name);
        aql.filter_step(
            format!(
                "{}.{}.{} == {}",
                AQL_DOCUMENT_ID,
                mutex_path,
                DBMutexField::Node(None).path(),
                serde_json::to_string(node_id).unwrap(),
            )
                .into(),
        );
        aql.update_step(
            AqlUpdate::new(
                AQL_DOCUMENT_ID.into(),
                collection_name,
                format!("{{ {}: null }}", mutex_path).into(),
            )
                .apply_ignore_errors(true),
        );

        if let Err(e) = collection.send_generic_aql::<DBUuid>(&aql).await {
            log::error!(
                "Error while releasing all collection mutexes in DB. Error: {}",
                e
            );
        }
    }
}

impl<T: 'static + DBSynchronizedDocument<'static>> Drop for DBMutexGuard<T> {
    fn drop(&mut self) {
        tokio::spawn(Self::release_action(self.inner.clone()));
    }
}
