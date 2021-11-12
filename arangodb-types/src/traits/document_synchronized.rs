use crate::traits::DBDocument;

pub trait DBSynchronizedDocument<'a>: DBDocument {
    /// The key of the document that represents the collection in the config collection.
    fn collection_key() -> &'a Self::Key;
}
