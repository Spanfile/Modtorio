use super::PathExt;
use crate::error::ZipError;
use std::{
    io::{Read, Seek},
    path::Path,
};
use zip::{read::ZipFile, ZipArchive};

/// Collection of common functions used with zip archives
pub trait ZipExt {
    /// Find a file with a certain file name from anywhere in the zip archive. Returns
    /// `ZipError::NoFile` if a file with the given file name isn't found.
    fn find_file(&mut self, name: &str) -> anyhow::Result<ZipFile<'_>>;
}

impl<F> ZipExt for ZipArchive<F>
where
    F: Read + Seek,
{
    fn find_file(&mut self, name: &str) -> anyhow::Result<ZipFile<'_>> {
        let mut found_path: Option<String> = None;
        for filepath in self.file_names() {
            if Path::new(filepath).get_file_name()? == name {
                found_path = Some(filepath.to_owned());
                break;
            }
        }

        let found_path = found_path.ok_or_else(|| ZipError::NoFile(name.to_owned()))?;
        Ok(self.by_name(&found_path)?)
    }
}
