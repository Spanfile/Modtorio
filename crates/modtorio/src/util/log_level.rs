//! Provides the [`LogLevel`](LogLevel) enum.

use log::LevelFilter;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};

/// Represents the various logging levels.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, EnumString, Display, EnumVariantNames, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// The `trace` level.
    #[strum(serialize = "trace")]
    Trace,
    /// The `debug` level.
    #[strum(serialize = "debug")]
    Debug,
    /// The `info` level.
    #[strum(serialize = "info")]
    Info,
    /// The `warn` level.
    #[strum(serialize = "warn")]
    Warn,
    /// The `error` level.
    #[strum(serialize = "error")]
    Error,
}

impl LogLevel {
    /// Returns a logging `LevelFilter` based on this `LogLevel`.
    pub fn to_level_filter(&self) -> LevelFilter {
        match self {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}
