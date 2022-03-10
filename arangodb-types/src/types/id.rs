use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// The id of a collection.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DBId<K, C> {
    key: K,
    collection: C,
}

impl<K, C> DBId<K, C> {
    // CONSTRUCTORS -----------------------------------------------------------

    pub fn new(key: K, collection: C) -> Self {
        DBId { key, collection }
    }

    // GETTERS ----------------------------------------------------------------

    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn collection(&self) -> &C {
        &self.collection
    }
}

impl<K: ToString, C: ToString> Serialize for DBId<K, C> {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(
            format!("{}/{}", self.collection.to_string(), self.key.to_string()).as_str(),
        )
    }
}

impl<'de, K: FromStr, C: FromStr> Deserialize<'de> for DBId<K, C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct DBIdVisitor<K, C> {
            phantom_key: PhantomData<K>,
            phantom_collection: PhantomData<C>,
        }

        impl<'de, K: FromStr, C: FromStr> Visitor<'de> for DBIdVisitor<K, C> {
            type Value = DBId<K, C>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with the format: <collection-name>/<document-id>")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let mut value = v.split('/');
                let collection = match value.next() {
                    Some(v) => match C::from_str(v) {
                        Ok(v) => v,
                        Err(_) => {
                            return Err(E::custom(format!("Incorrect value for a DBId: {}", v)));
                        }
                    },
                    None => return Err(E::custom(format!("Incorrect value for a DBId: {}", v))),
                };

                let key = match value.next() {
                    Some(v) => match K::from_str(v) {
                        Ok(v) => v,
                        Err(_) => {
                            return Err(E::custom(format!("Incorrect value for a DBId: {}", v)));
                        }
                    },
                    None => return Err(E::custom(format!("Incorrect value for a DBId: {}", v))),
                };

                // Too many values.
                if value.next().is_some() {
                    return Err(E::custom(format!("Incorrect value for a DBId: {}", v)));
                }

                Ok(DBId { key, collection })
            }
        }

        deserializer.deserialize_string(DBIdVisitor {
            phantom_key: PhantomData::default(),
            phantom_collection: PhantomData::default(),
        })
    }
}
