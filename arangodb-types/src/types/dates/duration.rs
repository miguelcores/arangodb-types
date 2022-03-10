use std::fmt;
use std::ops::Deref;

use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct DBDuration(u64);

impl Serialize for DBDuration {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for DBDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct TimeVisitor;
        impl<'de> Visitor<'de> for TimeVisitor {
            type Value = DBDuration;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an integer between -2^63 and 2^63")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DBDuration(value as u64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DBDuration(value))
            }
        }

        deserializer.deserialize_u64(TimeVisitor)
    }
}

impl Deref for DBDuration {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u8> for DBDuration {
    fn from(v: u8) -> Self {
        DBDuration(v as u64)
    }
}

impl From<u16> for DBDuration {
    fn from(v: u16) -> Self {
        DBDuration(v as u64)
    }
}

impl From<u32> for DBDuration {
    fn from(v: u32) -> Self {
        DBDuration(v as u64)
    }
}

impl From<u64> for DBDuration {
    fn from(v: u64) -> Self {
        DBDuration(v)
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_duration() {
        let time_duration = DBDuration(555687);
        let str_time_duration = serde_json::to_string(&time_duration).unwrap();

        assert_eq!("555687", str_time_duration);
        assert_eq!(
            time_duration,
            serde_json::from_str(str_time_duration.as_str()).unwrap()
        );
    }
}
