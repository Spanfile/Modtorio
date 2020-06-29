use super::PathExt;
use crate::error::ZipError;
use std::{
    io::{Read, Seek},
    path::Path,
};
use zip::{read::ZipFile, ZipArchive};

const INFO_JSON_FILENAME: &str = "info.json";

pub trait ZipExt {
    fn find_file(&mut self, name: &str) -> anyhow::Result<ZipFile<'_>>;
}

impl<F> ZipExt for ZipArchive<F>
where
    F: Read + Seek,
{
    fn find_file(&mut self, name: &str) -> anyhow::Result<ZipFile<'_>> {
        let mut found_path: Option<String> = None;
        for filepath in self.file_names() {
            if Path::new(filepath).get_file_name()? == INFO_JSON_FILENAME {
                found_path = Some(filepath.to_owned());
                break;
            }
        }

        let found_path = found_path.ok_or(ZipError::NoFile(INFO_JSON_FILENAME))?;
        Ok(self.by_name(&found_path)?)
    }
}
