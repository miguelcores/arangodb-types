use crate::traits::DBDocument;
use crate::types::{DBMutex, NullableOption};

pub trait DBSynchronizedDocument<'a>: DBDocument {
    // GETTERS ----------------------------------------------------------------

    /// The key of the document that represents the collection in the config collection.
    fn collection_key() -> &'a Self::Key;

    // SETTERS ----------------------------------------------------------------

    fn set_mutex(&mut self, mutex: NullableOption<DBMutex>);
}
