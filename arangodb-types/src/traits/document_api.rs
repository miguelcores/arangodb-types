use std::fmt::Debug;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

pub trait APIDocument {
    type Id: Debug
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

    fn id(&self) -> &Option<Self::Id>;
}
