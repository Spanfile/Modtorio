mod settings;

use settings::ServerSettings;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Factorio {
    settings: ServerSettings,
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
    pub fn with_server_settings(self, settings: PathBuf) -> Self {
        Self { settings, ..self }
    }

    pub fn import(self) -> anyhow::Result<Factorio> {
        let mut settings_path = self.root.clone();
        settings_path.push(self.settings);

        Ok(Factorio {
            settings: ServerSettings::from_json(&fs::read_to_string(settings_path)?)?,
        })
    }
}
