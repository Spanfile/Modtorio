//! Provides several utilities and helper functions.

pub mod async_status;
pub mod checksum;
pub mod env;
pub mod ext;
pub mod file;
mod human_version;
mod limit;
mod log_level;

use ext::PathExt;
pub use human_version::{Comparator, HumanVersion, HumanVersionReq};
pub use limit::Limit;
pub use log_level::LogLevel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Retrieves the last segment of a given path as a `PathBuf`. Panics if there is no last component
/// in the path.
pub fn get_last_path_segment<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let component = path.as_ref().components().last().expect("no last component in path");
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

/// Lossily percent decode a given URL segment into UTF-8.
pub fn decode_url(s: &str) -> String {
    percent_encoding::percent_decode_str(s).decode_utf8_lossy().to_string()
}

/// Represents a range of 64-bit unsigned integers.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Range {
    /// The range's lower bound.
    pub min: u64,
    /// The range's upper bound.
    pub max: u64,
}
