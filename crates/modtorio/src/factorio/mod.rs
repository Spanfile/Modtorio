mod mods;
mod settings;

use crate::{cache::models, ext::PathExt, Cache, Config, ModPortal};
use anyhow::anyhow;
use log::*;
use mods::{Mods, ModsBuilder};
use settings::ServerSettings;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

pub type GameCacheId = i64;

pub struct Factorio {
    pub settings: ServerSettings,
    pub mods: Mods,
    root: PathBuf,
    cache_id: Mutex<Option<GameCacheId>>,
    cache: Arc<Cache>,
}

pub struct Importer {
    root: Option<PathBuf>,
    settings: PathBuf,
    game_cache_id: Option<GameCacheId>,
}

impl Factorio {
    pub async fn update_cache(&self) -> anyhow::Result<()> {
        let mut cache_id = self.cache_id.lock().await;

        self.cache.begin_transaction()?;

        let id = if let Some(c) = *cache_id {
            self.cache
                .update_game(models::Game {
                    id: c,
                    path: self.root.get_str()?.to_string(),
                })
                .await?;

            debug!("Updating existing game cache (id {})", c);
            c
        } else {
            let new_id = self
                .cache
                .insert_game(models::Game {
                    id: 0,
                    path: self.root.get_str()?.to_string(),
                })
                .await?;
            *cache_id = Some(new_id);

            debug!("Inserting new game cache (id {})", new_id);
            new_id
        };

        self.mods.update_cache(id).await?;
        self.cache.commit_transaction()
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

    pub fn from_cache(game_cache_id: GameCacheId) -> Self {
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
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        cache: Arc<Cache>,
    ) -> anyhow::Result<Factorio> {
        let (root, cache_id) = match self.game_cache_id {
            Some(id) => {
                let cached_game = cache.get_game(id).await?;
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
                .build(config, portal, Arc::clone(&cache))
                .await?,
            root,
            cache_id: Mutex::new(cache_id),
            cache,
        })
    }
}
