mod info;

use anyhow::{anyhow, Context};
use info::Info;
use std::{fs, io::BufReader, path::Path};

#[derive(Debug)]
pub struct Mods(Vec<Mod>);

#[derive(Debug)]
pub struct Mod {
    info: Info,
}

impl Mods {
    pub fn from_directory<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let pathname = path.as_ref().to_string_lossy();
        let context = format_args!("Failed to load mods from {}", pathname).to_string();

        let mut zips = path.as_ref().to_path_buf();
        zips.push("*.zip");

        Ok(Mods(
            glob::glob(&zips.to_str().unwrap())
                .context(context)?
                .map(|entry| Ok(Mod::from_zip(entry?)?))
                .collect::<anyhow::Result<Vec<Mod>>>()?,
        ))
    }
}

impl Mod {
    pub fn from_zip<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let filename = path
            .as_ref()
            .file_name()
            .ok_or_else(|| anyhow!("given path doesn't have a filename"))?
            .to_string_lossy();
        let context = format_args!("Mod zip {} failed to load", filename).to_string();

        let zipfile = fs::File::open(path).context(context.clone())?;
        let reader = BufReader::new(zipfile);
        let mut archive = zip::ZipArchive::new(reader)?;
        let info = serde_json::from_reader(archive.by_name("info.json").context(context)?)?;

        Ok(Self { info })
    }
}
