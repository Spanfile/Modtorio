mod mods;
mod settings;

use crate::{Cache, Config, ModPortal};
use anyhow::anyhow;
use mods::{Mods, ModsBuilder};
use settings::ServerSettings;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Factorio<'a> {
    pub settings: ServerSettings,
    pub mods: Mods<'a, PathBuf>,
}

pub struct Importer {
    root: Option<PathBuf>,
    settings: PathBuf,
    game_cache_id: Option<i32>,
}

impl Importer {
    pub fn from_root<P>(root: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            root: Some(root.as_ref().to_path_buf()),
            settings: PathBuf::from("server-settings.json"),
            game_cache_id: None,
        }
    }

    pub fn from_cache(game_cache_id: i32) -> Self {
        Self {
            root: None,
            settings: PathBuf::from("server-settings.json"),
            game_cache_id: Some(game_cache_id),
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
        config: &'a Config,
        portal: &'a ModPortal,
        cache: &'a Cache,
    ) -> anyhow::Result<Factorio<'a>> {
        let root_path = self.root;
        let cached_path = self
            .game_cache_id
            .and_then(|id| cache.get_game(id).ok())
            .map(|game| PathBuf::from(game.path));

        let root = root_path.or(cached_path).ok_or_else(|| {
            anyhow!("cannot determine game root: root not set and game_cache_id is invalid")
        })?;

        let mut settings_path = root.clone();
        settings_path.push(self.settings);

        let mut mods_path = root.clone();
        mods_path.push("mods/");

        Ok(Factorio {
            settings: ServerSettings::from_game_json(&fs::read_to_string(settings_path)?)?,
            mods: ModsBuilder::root(mods_path)
                .build(config, portal, cache)
                .await?,
        })
    }
}
