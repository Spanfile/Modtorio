pub mod checksum;
mod human_version;

use std::path::{Path, PathBuf};

use crate::ext::PathExt;
pub use human_version::{Comparator, HumanVersion, HumanVersionReq};

pub fn dump_env(prefix: &str) -> String {
    dump_env_lines(prefix).join("\n")
}

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
