//! Provides several utilities and helper functions.

pub mod checksum;
pub mod env;
mod human_version;
mod log_level;

use crate::ext::PathExt;
pub use human_version::{Comparator, HumanVersion, HumanVersionReq};
pub use log_level::LogLevel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Retrieves the last segment of a given path as a `PathBuf`. Panics if there is no last component
/// in the path.
pub fn get_last_path_segment<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let component = path
        .as_ref()
        .components()
        .last()
        .expect("no last component in path");
    let last: &Path = component.as_ref();
    last.to_path_buf()
}

/// Returns all entries matched by a given glob pattern (for example `*.txt`).
///
/// Returns an error if:
/// * The glob pattern contains invalid Unicode
/// * The glob pattern is invalid
/// * The matched entries contain invalid Unicode
pub fn glob<P>(pattern: P) -> anyhow::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let mut matches = Vec::new();

    for entry in glob::glob(pattern.as_ref().get_str()?)? {
        matches.push(entry?);
    }

    Ok(matches)
}

/// Represents a range of 64-bit unsigned integers.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Range {
    pub min: u64,
    pub max: u64,
}

/// Represents a limit that is either unbounded ([Unlimited](#variant.Unlimited)) or bounded by a
/// 64-bit unsigned integer ([Limited](#variant.Limited)).
///
/// Conversion to and from `u64` is provided, where 0 is seen as [Unlimited](#variant.Unlimited)
/// and every other value as [Limited](#variant.Limited).
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub enum Limit {
    Unlimited,
    Limited(u64),
}

impl From<u64> for Limit {
    fn from(val: u64) -> Self {
        if val == 0 {
            Self::Unlimited
        } else {
            Self::Limited(val)
        }
    }
}

impl From<Limit> for u64 {
    fn from(val: Limit) -> Self {
        match val {
            Limit::Unlimited => 0,
            Limit::Limited(v) => v,
        }
    }
}
