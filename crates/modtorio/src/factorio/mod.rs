mod mods;
mod settings;

use crate::{ext::PathExt, Cache, Config, ModPortal};
use anyhow::anyhow;
use log::*;
use mods::{Mods, ModsBuilder};
use settings::ServerSettings;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Factorio<'a> {
    pub settings: ServerSettings,
    pub mods: Mods<'a, PathBuf>,
    root: PathBuf,
    cache_id: Option<i32>,
    cache: &'a Cache,
}

pub struct Importer {
    root: Option<PathBuf>,
    settings: PathBuf,
    game_cache_id: Option<i32>,
}

impl Factorio<'_> {
    pub fn update_cache(&mut self) -> anyhow::Result<()> {
        if let Some(cache_id) = self.cache_id {
            self.cache.update_game(cache_id, self.root.get_str()?)?;

            debug!("Updated existing game cache (id {})", cache_id);
        } else {
            let new_id = self.cache.insert_game(self.root.get_str()?)?;
            self.cache_id = Some(new_id);

            debug!("Inserted new game cache (id {})", new_id);
        }

        Ok(())
    }
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
        let (root, cache_id) = match self.game_cache_id {
            Some(id) => {
                let cached_game = cache.get_game(id)?;
                (Some(PathBuf::from(cached_game.path)), Some(cached_game.id))
            }
            None => (self.root, None),
        };

        let root = root.ok_or_else(|| anyhow!("no valid game root"))?;

        let mut settings_path = root.clone();
        settings_path.push(self.settings);

        let mut mods_path = root.clone();
        mods_path.push("mods/");

        Ok(Factorio {
            settings: ServerSettings::from_game_json(&fs::read_to_string(settings_path)?)?,
            mods: ModsBuilder::root(mods_path)
                .build(config, portal, cache)
                .await?,
            root,
            cache_id,
            cache,
        })
    }
}
