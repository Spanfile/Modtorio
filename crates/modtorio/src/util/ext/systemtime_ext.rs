//! Provides the [`SystemTime`](SystemTime) trait which provides several commonly used functions on SystemTime objects.

use chrono::{DateTime, TimeZone, Utc};
use std::time::SystemTime;

/// Collection of common functions used with `SystemTime` objects.
pub trait SystemTimeExt {
    /// Returns this `SystemTime` as a `String`.
    fn to_string(&self) -> String;
    /// Returns this `SystemTime` as a `chrono::DateTime<Utc>`.
    fn to_chrono(&self) -> DateTime<Utc>;
}

impl SystemTimeExt for SystemTime {
    fn to_string(&self) -> String {
        let duration = self
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("SystemTime is behind Unix epoch");
        Utc.timestamp_millis(duration.as_millis() as i64).to_string()
    }

    fn to_chrono(&self) -> DateTime<Utc> {
        let duration = self
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("SystemTime is behind Unix epoch");
        Utc.timestamp_millis(duration.as_millis() as i64)
    }
}
