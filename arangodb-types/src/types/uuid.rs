use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;

use arcstr::ArcStr;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

use crate::traits::{DBNormalize, DBNormalizeResult};

// Char set used to create the codes.
// We do not use the default because it is not correctly sorted in DB.
const ALPHABET: [char; 64] = [
    '-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
    'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '_',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

// Same as ALPHABET but without - and _ characters.
const SIMPLE_ALPHABET: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b',
    'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u',
    'v', 'w', 'x', 'y', 'z',
];

// Same as SIMPLE_ALPHABET but without I and O letters.
const BASE60_ALPHABET: [char; 60] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J',
    'K', 'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd',
    'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w',
    'x', 'y', 'z',
];

// Same as BASE60_ALPHABET but without the 0 number and l letter.
const BASE58_ALPHABET: [char; 58] = [
    '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K',
    'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e',
    'f', 'g', 'h', 'i', 'j', 'k', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y',
    'z',
];

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct DBUuid(ArcStr);

impl DBUuid {
    // CONSTRUCTORS -----------------------------------------------------------

    pub fn new() -> DBUuid {
        Self::new_with_length(22)
    }

    pub fn new_with_length(length: usize) -> DBUuid {
        DBUuid(nanoid!(length, &ALPHABET).into())
    }

    pub fn new_simple() -> DBUuid {
        Self::new_simple_with_length(23)
    }

    pub fn new_simple_with_length(length: usize) -> DBUuid {
        DBUuid(nanoid!(length, &SIMPLE_ALPHABET).into())
    }

    pub fn new_base60() -> DBUuid {
        Self::new_base60_with_length(23)
    }

    pub fn new_base60_with_length(length: usize) -> DBUuid {
        DBUuid(nanoid!(length, &BASE60_ALPHABET).into())
    }

    pub fn new_base58() -> DBUuid {
        Self::new_base58_with_length(23)
    }

    pub fn new_base58_with_length(length: usize) -> DBUuid {
        DBUuid(nanoid!(length, &BASE58_ALPHABET).into())
    }

    // METHODS ----------------------------------------------------------------

    pub fn as_string(&self) -> &ArcStr {
        &self.0
    }
}

impl FromStr for DBUuid {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check_nanoid(s).map(|_| DBUuid(s.into()))
    }
}

impl TryFrom<ArcStr> for DBUuid {
    type Error = &'static str;

    fn try_from(s: ArcStr) -> Result<Self, Self::Error> {
        check_nanoid(s.as_str()).map(|_| DBUuid(s))
    }
}

impl Display for DBUuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for DBUuid {
    fn default() -> Self {
        Self::new()
    }
}

impl DBNormalize for DBUuid {
    fn normalize(&mut self) -> DBNormalizeResult {
        DBNormalizeResult::NotModified
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

fn check_nanoid(s: &str) -> Result<(), &'static str> {
    for c in s.chars() {
        if ALPHABET.binary_search(&c).is_err() {
            return Err("nanoid::decoding::invalid_chars");
        }
    }

    Ok(())
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serialization() {
        let id = DBUuid::from_str("gid000020").unwrap();
        let serialization = serde_json::to_string(&id).unwrap();

        assert_eq!(serialization, "\"gid000020\"");

        let deserialization: DBUuid = serde_json::from_str(&serialization).unwrap();
        assert_eq!(deserialization, id);
    }

    #[test]
    fn test_from_str() {
        let id = DBUuid::from_str("gidMh8J1aB000000000000020").expect("The from_str must succeed");
        let id_ok = DBUuid("gidMh8J1aB000000000000020".into());
        assert_eq!(id, id_ok);

        DBUuid::from_str("gidMh8J1aB00000000000002ñ").expect_err("The id must fail by character");
    }
}
