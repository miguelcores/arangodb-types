use crate::types::NullableOption;
use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

macro_rules! unsigned_types {
    ($from_type:ty, $method:ident, $method_literal:literal, $null_method:ident) => {
        pub fn $method<'de, D>(
            deserializer: D,
        ) -> Result<$from_type, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            struct DBVisitor;

            impl<'de> Visitor<'de> for DBVisitor {
                type Value = $from_type;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a string with the format: <collection-name>/<document-id>")
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    if v < 0 || v > <$from_type>::MAX as i64 {
                        return Err(Error::invalid_type(Unexpected::Signed(v), &self));
                    }

                    Ok(v as $from_type)
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    if v > <$from_type>::MAX as u64 {
                        return Err(Error::invalid_type(Unexpected::Unsigned(v), &self));
                    }

                    Ok(v as $from_type)
                }

                fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    if v < 0.0 || v > <$from_type>::MAX as f64 {
                        return Err(Error::invalid_type(Unexpected::Float(v), &self));
                    }

                    let v2 = v.trunc();

                    if v != v2 {
                        Err(Error::invalid_type(Unexpected::Float(v), &self))
                    } else {
                        self.visit_u64(v2 as u64)
                    }
                }
            }

            deserializer.deserialize_u64(DBVisitor)
        }

        pub fn $null_method<'de, D>(
            deserializer: D,
        ) -> Result<NullableOption<$from_type>, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            struct Aux(#[serde(deserialize_with = $method_literal)] pub $from_type);
            let result = <NullableOption<Aux>>::deserialize(deserializer)?;

            match result {
                NullableOption::Value(v) => Ok(NullableOption::Value(v.0)),
                NullableOption::Missing => Ok(NullableOption::Missing),
                NullableOption::Null => Ok(NullableOption::Null),
            }
        }
    };
}

macro_rules! signed_types {
    ($from_type:ty, $method:ident, $method_literal:literal, $null_method:ident) => {
        pub fn $method<'de, D>(
            deserializer: D,
        ) -> Result<$from_type, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            struct DBVisitor;

            impl<'de> Visitor<'de> for DBVisitor {
                type Value = $from_type;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a signed value")
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    if v < <$from_type>::MIN as i64 || v > <$from_type>::MAX as i64 {
                        return Err(Error::invalid_type(Unexpected::Signed(v), &self));
                    }

                    Ok(v as $from_type)
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    if v > <$from_type>::MAX as u64 {
                        return Err(Error::invalid_type(Unexpected::Unsigned(v), &self));
                    }

                    Ok(v as $from_type)
                }

                fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    if v < <$from_type>::MIN as f64 || v > <$from_type>::MAX as f64 {
                        return Err(Error::invalid_type(Unexpected::Float(v), &self));
                    }

                    let v2 = v.trunc();

                    if v != v2 {
                        Err(Error::invalid_type(Unexpected::Float(v), &self))
                    } else {
                        self.visit_i64(v2 as i64)
                    }
                }
            }

            deserializer.deserialize_i64(DBVisitor)
        }

        pub fn $null_method<'de, D>(
            deserializer: D,
        ) -> Result<NullableOption<$from_type>, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            struct Aux(#[serde(deserialize_with = $method_literal)] pub $from_type);
            let result = <NullableOption<Aux>>::deserialize(deserializer)?;

            match result {
                NullableOption::Value(v) => Ok(NullableOption::Value(v.0)),
                NullableOption::Missing => Ok(NullableOption::Missing),
                NullableOption::Null => Ok(NullableOption::Null),
            }
        }
    };
}

unsigned_types!(
    u8,
    deserialize_u8,
    "deserialize_u8",
    deserialize_nullable_u8
);
unsigned_types!(
    u16,
    deserialize_u16,
    "deserialize_u16",
    deserialize_nullable_u16
);
unsigned_types!(
    u32,
    deserialize_u32,
    "deserialize_u32",
    deserialize_nullable_u32
);
unsigned_types!(
    u64,
    deserialize_u64,
    "deserialize_u64",
    deserialize_nullable_u64
);
signed_types!(
    i8,
    deserialize_i8,
    "deserialize_i8",
    deserialize_nullable_i8
);
signed_types!(
    i16,
    deserialize_i16,
    "deserialize_i16",
    deserialize_nullable_i16
);
signed_types!(
    i32,
    deserialize_i32,
    "deserialize_i32",
    deserialize_nullable_i32
);
signed_types!(
    i64,
    deserialize_i64,
    "deserialize_i64",
    deserialize_nullable_i64
);

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_deserialize_struct() {
        #[derive(Deserialize)]
        struct Demo {
            #[serde(deserialize_with = "deserialize_u64")]
            value: u64,
        }

        let value = "{ \"value\": 1234 }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, 1234);

        let value = "{ \"value\": 1234e+3 }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, 1234000);
    }

    #[test]
    fn test_deserialize_nullable_struct() {
        #[derive(Deserialize)]
        struct Demo {
            #[serde(default)]
            #[serde(deserialize_with = "deserialize_nullable_u32")]
            value: NullableOption<u32>,
        }

        let value = "{ \"value\": 1234 }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, NullableOption::Value(1234));

        let value = "{ \"value\": 1234e+3 }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, NullableOption::Value(1234000));

        let value = "{ \"value\": null }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, NullableOption::Null);

        let value = "{ }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, NullableOption::Missing);
    }

    #[test]
    fn test_deserialize_nullable_struct2() {
        #[derive(Default, Deserialize)]
        struct Demo {
            #[serde(rename = "Q")]
            #[serde(default)]
            #[serde(deserialize_with = "deserialize_nullable_u32")]
            value: NullableOption<u32>,
        }

        let value = "{ \"Q\": 1234 }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, NullableOption::Value(1234));

        let value = "{ \"Q\": 1234e+3 }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, NullableOption::Value(1234000));

        let value = "{ \"Q\": null }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, NullableOption::Null);

        let value = "{ }";
        let deserialize: Demo = serde_json::from_str(value).unwrap();
        assert_eq!(deserialize.value, NullableOption::Missing);
    }
}
