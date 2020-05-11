use super::PathExt;
use anyhow::anyhow;
use std::{
    io::{Read, Seek},
    path::Path,
};
use zip::{read::ZipFile, ZipArchive};

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
            if Path::new(filepath).get_file_name()? == "info.json" {
                found_path = Some(filepath.to_owned());
                break;
            }
        }

        let found_path =
            found_path.ok_or_else(|| anyhow!("No such file in zip archive: {}", name))?;
        Ok(self.by_name(&found_path)?)
    }
}
