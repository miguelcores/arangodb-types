use crate::traits::DBDocument;
use crate::types::DBId;

pub trait DBEdgeDocument: DBDocument {
    // GETTERS ----------------------------------------------------------------

    fn db_from(&self) -> &Option<DBId<Self::Key, Self::CollectionType>>;

    fn db_to(&self) -> &Option<DBId<Self::Key, Self::CollectionType>>;

    // GETTERS ----------------------------------------------------------------

    // METHODS ----------------------------------------------------------------
}
