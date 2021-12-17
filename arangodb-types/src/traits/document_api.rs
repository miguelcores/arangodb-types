use std::fmt::Debug;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

pub trait APIDocument {
    type Key: Debug
    + ToString
    + Eq
    + PartialEq
    + Clone
    + Hash
    + Send
    + Sync
    + Serialize
    + for<'de> Deserialize<'de>;

    // GETTERS ----------------------------------------------------------------

    fn id(&self) -> &Option<Self::Key>;

    /// Whether all the fields are missing or not.
    fn is_all_missing(&self) -> bool;
}
