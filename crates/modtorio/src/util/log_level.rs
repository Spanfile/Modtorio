use log::LevelFilter;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(
    Debug, Deserialize, Serialize, Eq, PartialEq, EnumString, Display, EnumVariantNames, Copy, Clone,
)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[strum(serialize = "trace")]
    Trace,
    #[strum(serialize = "debug")]
    Debug,
    #[strum(serialize = "info")]
    Info,
    #[strum(serialize = "warn")]
    Warn,
    #[strum(serialize = "error")]
    Error,
}

impl LogLevel {
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
