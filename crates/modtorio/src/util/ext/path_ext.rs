//! Provides the [`PathExt`](PathExt) trait which provides several commonly used functions on paths
//! and path-like objects.

use crate::error::PathError;
use std::path::Path;

/// Collection of common functions used with paths and path-like objects.
pub trait PathExt {
    /// Extracts the file name from the path and returns it as a `String`. If the path is a
    /// directory, returns the directory name. Returns an error on the following conditions:
    ///  * the path doesn't have a file name
    ///  * the path contains invalid Unicode
    fn get_file_name(&self) -> anyhow::Result<String>;
    /// Extracts the file stem (non-extension) portion from the path and returns it as a `String`.
    /// Returns an error on the following conditions:
    ///  * the path doesn't have a file name (`PathError::NoFilename`)
    ///  * the path contains invalid Unicode (`PathError::InvalidUnicode`))
    fn get_file_stem(&self) -> anyhow::Result<String>;
    /// Borrows the path as an `&str`. Returns `PathError::InvalidUnicode` error if the path
    /// contains invalid Unicode.
    fn get_str(&self) -> anyhow::Result<&str>;
    /// Copies the path as a `String`. Returns `PathError::InvalidUnicode` error if the path
    /// contains invalid Unicode.
    fn get_string(&self) -> anyhow::Result<String>;
}

impl<P> PathExt for P
where
    P: AsRef<Path>,
{
    fn get_file_name(&self) -> anyhow::Result<String> {
        Ok(self
            .as_ref()
            .file_name()
            .ok_or(PathError::NoFilename)?
            .to_str()
            .ok_or(PathError::InvalidUnicode)?
            .to_owned())
    }

    fn get_file_stem(&self) -> anyhow::Result<String> {
        Ok(self
            .as_ref()
            .file_stem()
            .ok_or(PathError::NoFilename)?
            .to_str()
            .ok_or(PathError::InvalidUnicode)?
            .to_owned())
    }

    fn get_str(&self) -> anyhow::Result<&str> {
        Ok(self.as_ref().to_str().ok_or(PathError::InvalidUnicode)?)
    }

    fn get_string(&self) -> anyhow::Result<String> {
        Ok(self.as_ref().to_str().ok_or(PathError::InvalidUnicode)?.to_string())
    }
}
