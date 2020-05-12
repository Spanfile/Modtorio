mod mods;
mod settings;

use crate::{Config, ModPortal};
use mods::{Mods, ModsBuilder};
use settings::ServerSettings;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug)]
pub struct Factorio {
    pub settings: ServerSettings,
    pub mods: Mods<PathBuf>,
}

pub struct Importer {
    root: PathBuf,
    settings: PathBuf,
}

impl Importer {
    pub fn from<P: AsRef<Path>>(root: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            root: root.as_ref().to_path_buf(),
            settings: PathBuf::from("server-settings.json"),
        }
    }

    #[allow(dead_code)]
    pub fn with_server_settings<P>(self, settings: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            settings: settings.as_ref().to_path_buf(),
            ..self
        }
    }

    pub async fn import<'a>(
        self,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
    ) -> anyhow::Result<Factorio> {
        let mut settings_path = self.root.clone();
        settings_path.push(self.settings);

        let mut mods_path = self.root;
        mods_path.push("mods/");

        Ok(Factorio {
            settings: ServerSettings::from_game_json(&fs::read_to_string(settings_path)?)?,
            mods: ModsBuilder::root(mods_path).build(config, portal).await?,
        })
    }
}
