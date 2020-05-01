mod dependency;
mod info;

use crate::ext::PathExt;
use anyhow::anyhow;
pub use dependency::Dependency;
pub use info::Info;
use log::*;
use std::path::Path;
use tokio::task;

#[derive(Debug)]
pub struct Mod {
    pub info: Info,
}

impl Mod {
    pub async fn from_zip<P>(path: P) -> anyhow::Result<Self>
    where
        P: 'static + AsRef<Path> + Send,
    {
        debug!("Creating mod from zip {}", path.as_ref().display());
        let info = task::spawn_blocking(|| -> anyhow::Result<Info> {
            let zipfile = std::fs::File::open(path)?;
            let mut archive = zip::ZipArchive::new(zipfile)?;

            let mut infopath: Option<String> = None;
            for filepath in archive.file_names() {
                if Path::new(filepath).get_file_name()? == "info.json" {
                    infopath = Some(filepath.to_owned());
                    break;
                }
            }

            let infopath = infopath.ok_or_else(|| anyhow!("no info.json found"))?;
            let info = serde_json::from_reader(archive.by_name(&infopath)?)?;
            Ok(info)
        })
        .await?;

        Ok(Mod { info: info? })
    }

    pub fn get_archive_filename(&self) -> String {
        format!("{}_{}.zip", self.info.name, self.info.version)
    }
}
