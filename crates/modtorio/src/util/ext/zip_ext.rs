//! Provides the [`ZipExt`](ZipExt) trait which provides several commonly used functions on zip
//! archives

use super::PathExt;
use std::{
    io::{Read, Seek},
    path::Path,
};
use zip::ZipArchive;

/// Collection of common functions used with zip archives
pub trait ZipExt {
    /// Find a file with a certain file name from anywhere in the zip archive. Returns
    /// `ZipError::NoFile` if a file with the given file name isn't found.
    fn find_files(&mut self, names: &[&str]) -> anyhow::Result<Vec<(String, Vec<u8>)>>;
}

impl<F> ZipExt for ZipArchive<F>
where
    F: Read + Seek,
{
    // (file.name().to_string(), file.bytes().filter_map(Result::ok).collect())
    fn find_files(&mut self, names: &[&str]) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
        let names = names.to_owned();
        let mut found_files = Vec::new();
        'file_loop: for i in 0..self.len() {
            // opening a file `by_index()` means reading its header from the zip reader, which is safe to assume here is
            // a fast operation
            let file = self.by_index(i)?;
            let file_name = Path::new(file.name()).get_file_name()?;

            for name in &names {
                if file_name == *name {
                    found_files.push((file_name.to_string(), file.bytes().filter_map(Result::ok).collect()));

                    if found_files.len() == names.len() {
                        // all the files have been found
                        break 'file_loop;
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(found_files)
    }
}
