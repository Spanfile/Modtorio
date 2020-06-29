use crate::error::PathError;
use std::path::Path;

pub trait PathExt {
    fn get_file_name(&self) -> anyhow::Result<String>;
    fn get_file_stem(&self) -> anyhow::Result<String>;
    fn get_str(&self) -> anyhow::Result<&str>;
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
}
