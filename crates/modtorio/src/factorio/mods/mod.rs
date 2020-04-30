mod info;

use crate::mod_portal::ModPortal;
use anyhow::anyhow;
use ext::PathExt;
use futures::stream::StreamExt;
use glob::glob;
use info::Info;
use log::*;
use std::{fmt::Debug, path::Path};
use tokio::{stream, task};
use util::HumanVersion;

const MOD_LOAD_BUFFER_SIZE: usize = 8;

pub enum ModSource<'a> {
    Portal {
        mod_portal: &'a ModPortal,
        name: String,
        version: Option<HumanVersion>,
    },
    Zip {
        path: &'a (dyn AsRef<Path>),
    },
}

#[derive(Debug)]
pub struct Mods<P>
where
    P: AsRef<Path>,
{
    directory: P,
    mods: Vec<Mod>,
}

#[derive(Debug)]
pub struct Mod {
    info: Info,
}

impl<P> Mods<P>
where
    P: AsRef<Path>,
{
    pub async fn from_directory(path: P) -> anyhow::Result<Self> {
        let zips = path.as_ref().join("*.zip");

        // create a stream of loading individual mods
        let mods = stream::iter(glob(zips.get_str()?)?.map(|entry| async move {
            let entry = entry?;
            Ok(task::spawn(async {
                match Mod::from_zip(entry).await {
                    Ok(m) => Some(m),
                    Err(e) => {
                        warn!("Mod failed to load: {}", e);
                        None
                    }
                }
            })
            .await?)
        }))
        .buffer_unordered(MOD_LOAD_BUFFER_SIZE) // buffer the stream into MOD_LOAD_BUFFER_SIZE parallel tasks
        .collect::<Vec<anyhow::Result<Option<Mod>>>>()
        .await // collect them into a vec of results asynchronously
        .into_iter()
        .filter_map(|m| m.transpose()) // turn each Result<Option<...>> into Option<Result<...>>
        .collect::<anyhow::Result<Vec<Mod>>>()?; // aggregate the results into a final vec

        Ok(Mods {
            directory: path,
            mods,
        })
    }

    pub fn count(&self) -> usize {
        self.mods.len()
    }

    pub async fn add<'a>(&mut self, source: ModSource<'_>) -> anyhow::Result<()> {
        let zipfile = match source {
            ModSource::Portal {
                mod_portal,
                name,
                version,
            } => {
                info!("Retrieve mod '{}' (ver. {:?}) from portal", name, version);

                let (path, written_bytes) = mod_portal
                    .download_mod(&name, version, &self.directory)
                    .await?;

                debug!("Downloaded {} bytes to {}", written_bytes, path.display());
                path
            }
            ModSource::Zip { path: _ } => unimplemented!(),
        };

        let new_mod = Mod::from_zip(zipfile).await?;

        info!(
            "Added new mod '{}' ver. {}",
            new_mod.info.title, new_mod.info.version
        );
        debug!("{:?}", new_mod);

        self.mods.push(new_mod);

        Ok(())
    }
}

impl Mod {
    pub async fn from_zip<P: 'static + AsRef<Path> + Send>(path: P) -> anyhow::Result<Self> {
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
}
