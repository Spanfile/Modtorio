mod info;

use crate::mod_portal::ModPortal;
use anyhow::anyhow;
use ext::PathExt;
use info::Info;
use log::*;
use std::{fmt::Debug, fs, io::BufReader, path::Path};
use util::HumanVersion;

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
    pub fn from_directory(path: P) -> anyhow::Result<Self> {
        let pathname = path.get_str()?;
        let zips = path.as_ref().join("*.zip");

        let mods = macros::with_context!(
        format!("Failed to load mods from {}", pathname),
            Vec<Mod>: {
            let mut mods = Vec::new();

            for entry in glob::glob(zips.get_str()?)? {
                let fact_mod = Mod::from_zip(entry?);
                match fact_mod {
                    Ok(m) => mods.push(m),
                    Err(e) => {
                        warn!("Mod {} failed to load: {}", pathname, e);
                    }
                }
            }

            Ok(mods)
        })?;

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

        let new_mod = Mod::from_zip(zipfile)?;

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
    pub fn from_zip<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let filename = path.get_file_name()?;
        macros::with_context!(format_args!("Failed to load mod zip {}", filename).to_string(),
            Self: {
            let zipfile = fs::File::open(path)?;
            let reader = BufReader::new(zipfile);
            let mut archive = zip::ZipArchive::new(reader)?;

            let mut infopath: Option<String> = None;
            for filepath in archive.file_names() {
                if Path::new(filepath).get_file_name()? == "info.json" {
                    infopath = Some(filepath.to_owned());
                    break;
                }
            }

            let infopath = infopath.ok_or_else(|| anyhow!("no info.json found"))?;
            let info = serde_json::from_reader(archive.by_name(&infopath)?)?;

            Ok(Mod { info })
        })
    }
}
