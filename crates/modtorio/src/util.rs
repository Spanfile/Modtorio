//! Provides several utilities and helper functions.

pub mod checksum;
mod human_version;

use std::path::{Path, PathBuf};

use crate::ext::PathExt;
pub use human_version::{Comparator, HumanVersion, HumanVersionReq};

/// Returns all environment variables of the current process with a given prefix as a string with
/// each variable on its own line.
pub fn dump_env(prefix: &str) -> String {
    dump_env_lines(prefix).join("\n")
}

/// Returns all environment variables of the current process with a given prefix as a vector with
/// each variable being a single `String` element.
pub fn dump_env_lines(prefix: &str) -> Vec<String> {
    std::env::vars()
        .filter_map(|(k, v)| {
            if k.starts_with(prefix) {
                Some(format!("{}={}", k, v))
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
}

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
