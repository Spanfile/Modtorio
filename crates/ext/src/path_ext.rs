use anyhow::anyhow;
use std::path::Path;

pub trait PathExt {
    fn get_file_name(&self) -> anyhow::Result<String>;
    fn get_file_stem(&self) -> anyhow::Result<String>;
}

impl<P> PathExt for P
where
    P: AsRef<Path>,
{
    fn get_file_name(&self) -> anyhow::Result<String> {
        Ok(self
            .as_ref()
            .file_name()
            .ok_or_else(|| anyhow!("given path doesn't have a filename"))?
            .to_str()
            .ok_or_else(|| anyhow!("path isn't valid unicode"))?
            .to_owned())
    }

    fn get_file_stem(&self) -> anyhow::Result<String> {
        Ok(self
            .as_ref()
            .file_stem()
            .ok_or_else(|| anyhow!("given path doesn't have a filename"))?
            .to_str()
            .ok_or_else(|| anyhow!("path isn't valid unicode"))?
            .to_owned())
    }
}
