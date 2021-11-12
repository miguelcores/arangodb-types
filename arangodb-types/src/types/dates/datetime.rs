use std::fmt;
use std::ops::Deref;

use chrono::{Datelike, LocalResult, TimeZone, Timelike, Utc};
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::traits::{DBNormalize, DBNormalizeResult};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DBDateTime(pub chrono::DateTime<Utc>);

impl DBDateTime {
    // CONSTRUCTORS -----------------------------------------------------------

    pub fn new(date: chrono::DateTime<Utc>) -> Self {
        DBDateTime(date.date().and_hms_milli(
            date.hour(),
            date.minute(),
            date.second(),
            date.timestamp_subsec_millis(),
        ))
    }

    pub fn now() -> Self {
        Self::new(Utc::now())
    }

    pub fn current_minute() -> Self {
        let now = Utc::now();
        DBDateTime(now.date().and_hms(now.hour(), now.minute(), 0))
    }

    pub fn current_hour() -> Self {
        let now = Utc::now();
        DBDateTime(now.date().and_hms(now.hour(), 0, 0))
    }

    pub fn max_datetime() -> Self {
        Self::new(chrono::MAX_DATETIME)
    }

    // GETTERS ----------------------------------------------------------------

    /// Checks this datetime against now as if it is an expiration.
    pub fn is_expired(&self) -> bool {
        let now = DBDateTime::default();
        self.0 <= now.0
    }

    // METHODS ----------------------------------------------------------------

    /// Creates a new DateTime from the current one after `duration` seconds.
    pub fn after_seconds(&self, duration: u64) -> DBDateTime {
        self.after_seconds_checked(duration as i64).unwrap()
    }

    /// Creates a new DateTime from the current one after `duration` seconds.
    pub fn after_seconds_checked(&self, duration: i64) -> Option<DBDateTime> {
        self.0
            .checked_add_signed(chrono::Duration::seconds(duration))
            .map(DBDateTime)
    }

    /// Creates a new DateTime from the current one before `duration` seconds.
    pub fn before_seconds(&self, duration: u64) -> DBDateTime {
        DBDateTime(self.0 - chrono::Duration::seconds(duration as i64))
    }

    /// Creates a new DateTime from the current one after `duration` days.
    pub fn after_days(&self, duration: u64) -> DBDateTime {
        self.after_days_checked(duration as i64).unwrap()
    }

    /// Creates a new DateTime from the current one after `duration` days.
    pub fn after_days_checked(&self, duration: i64) -> Option<DBDateTime> {
        self.0
            .checked_add_signed(chrono::Duration::days(duration))
            .map(DBDateTime)
    }

    /// Creates a new DateTime from the current one after `duration` months.
    pub fn after_months_checked(&self, duration: u32) -> Option<DBDateTime> {
        let mut final_months = match (self.0.year() as i64).checked_mul(12) {
            Some(v) => v,
            None => return None,
        };
        final_months = match final_months.checked_add(self.0.month0() as i64) {
            Some(v) => v,
            None => return None,
        };
        final_months = match final_months.checked_add(duration as i64) {
            Some(v) => v,
            None => return None,
        };

        let year = final_months / 12;
        let month = final_months % 12;

        match Utc
            .ymd_opt(year as i32, month as u32 + 1, self.0.day())
            .map(|v| {
                v.and_hms_milli_opt(
                    self.0.hour(),
                    self.0.minute(),
                    self.0.second(),
                    self.0.timestamp_subsec_millis(),
                )
                .map(DBDateTime)
            }) {
            LocalResult::Single(v) => v,
            _ => None,
        }
    }

    /// Creates a new DateTime from the current one after `duration` years.
    pub fn after_years_checked(&self, duration: i32) -> Option<DBDateTime> {
        let mut years = self.0.year();
        years = match years.checked_add(duration) {
            Some(v) => v,
            None => return None,
        };

        match Utc.ymd_opt(years, self.0.month(), self.0.day()).map(|v| {
            v.and_hms_milli_opt(
                self.0.hour(),
                self.0.minute(),
                self.0.second(),
                self.0.timestamp_subsec_millis(),
            )
            .map(DBDateTime)
        }) {
            LocalResult::Single(v) => v,
            _ => None,
        }
    }

    /// Creates a new DateTime from the current one before `duration` months.
    pub fn before_months(&self, duration: u64) -> DBDateTime {
        let mut final_months = self.0.year() * 12;
        final_months += self.0.month0() as i32;
        final_months -= duration as i32;

        let year = final_months / 12;
        let month = final_months % 12;

        DBDateTime(Utc.ymd(year, month as u32 + 1, self.0.day()).and_hms_milli(
            self.0.hour(),
            self.0.minute(),
            self.0.second(),
            self.0.timestamp_subsec_millis(),
        ))
    }

    pub fn min(self, other: DBDateTime) -> DBDateTime {
        DBDateTime(self.0.min(other.0))
    }

    pub fn max(self, other: DBDateTime) -> DBDateTime {
        DBDateTime(self.0.max(other.0))
    }
}

impl Serialize for DBDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.0.timestamp_millis())
    }
}

impl<'de> Deserialize<'de> for DBDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct DateTimeVisitor;
        impl<'de> Visitor<'de> for DateTimeVisitor {
            type Value = DBDateTime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an integer between -2^63 and 2^63")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DBDateTime::new(Utc::timestamp_millis(&Utc, value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DBDateTime::new(Utc::timestamp_millis(&Utc, value as i64)))
            }
        }

        deserializer.deserialize_i64(DateTimeVisitor)
    }
}

impl Deref for DBDateTime {
    type Target = chrono::DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<chrono::DateTime<Utc>> for DBDateTime {
    fn from(v: chrono::DateTime<Utc>) -> Self {
        DBDateTime::new(v)
    }
}

impl DBNormalize for DBDateTime {
    fn normalize(&mut self) -> DBNormalizeResult {
        DBNormalizeResult::NotModified
    }
}

impl Default for DBDateTime {
    fn default() -> Self {
        Self::now()
    }
}

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_datetime() {
        let date = DBDateTime(Utc.ymd(1970, 12, 7).and_hms_milli(5, 23, 30, 500));
        let str_date = serde_json::to_string(&date).unwrap();

        assert_eq!("29395410500", str_date);
        assert_eq!(date, serde_json::from_str(str_date.as_str()).unwrap());
    }

    #[test]
    fn test_datetime_after_months() {
        let original_date = DBDateTime(Utc.ymd(2021, 12, 1).and_hms(0, 0, 0));
        let final_date = original_date.after_months_checked(1).unwrap();

        assert_eq!(final_date.0.year(), 2022, "The year is incorrect");
        assert_eq!(final_date.0.month(), 1, "The month is incorrect");

        let original_date = DBDateTime(Utc.ymd(2021, 5, 1).and_hms(0, 0, 0));
        let final_date = original_date.after_months_checked(20).unwrap();

        assert_eq!(final_date.0.year(), 2023, "The year is incorrect");
        assert_eq!(final_date.0.month(), 1, "The month is incorrect");
    }

    #[test]
    fn test_datetime_before_months() {
        let original_date = DBDateTime(Utc.ymd(2021, 1, 1).and_hms(0, 0, 0));
        let final_date = original_date.before_months(1);

        assert_eq!(final_date.0.year(), 2020, "The year is incorrect");
        assert_eq!(final_date.0.month(), 12, "The month is incorrect");

        let original_date = DBDateTime(Utc.ymd(2021, 5, 1).and_hms(0, 0, 0));
        let final_date = original_date.before_months(20);

        assert_eq!(final_date.0.year(), 2019, "The year is incorrect");
        assert_eq!(final_date.0.month(), 9, "The month is incorrect");
    }
}
