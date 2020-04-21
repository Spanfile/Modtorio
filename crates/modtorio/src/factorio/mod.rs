mod mods;
mod settings;

use mods::Mods;
use settings::ServerSettings;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Factorio {
    pub settings: ServerSettings,
    pub mods: Mods,
}

pub struct Importer {
    root: PathBuf,
    settings: PathBuf,
}

impl Importer {
    pub fn from<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            settings: PathBuf::from("server-settings.json"),
        }
    }

    #[allow(dead_code)]
    pub fn with_server_settings<P: AsRef<Path>>(self, settings: P) -> Self {
        Self {
            settings: settings.as_ref().to_path_buf(),
            ..self
        }
    }

    pub fn import(self) -> anyhow::Result<Factorio> {
        let mut settings_path = self.root.clone();
        settings_path.push(self.settings);

        let mut mods_path = self.root;
        mods_path.push("mods/");

        Ok(Factorio {
            settings: ServerSettings::from_json(&fs::read_to_string(settings_path)?)?,
            mods: Mods::from_directory(mods_path)?,
        })
    }
}
