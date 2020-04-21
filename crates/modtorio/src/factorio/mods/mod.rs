mod info;

use anyhow::anyhow;
use ext::PathExt;
use info::Info;
use log::*;
use std::{fs, io::BufReader, path::Path};

#[derive(Debug)]
pub struct Mods {
    pub mods: Vec<Mod>,
}

#[derive(Debug)]
pub struct Mod {
    info: Info,
}

impl Mods {
    pub fn from_directory<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let pathname = path.get_str()?;
        macros::with_context!(
            format_args!("Failed to load mods from {}", pathname).to_string(),
            Self: {
            let zips = path.as_ref().join("*.zip");

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

            Ok(Mods { mods })
        })
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
