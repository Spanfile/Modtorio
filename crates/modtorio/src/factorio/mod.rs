mod mods;
mod settings;

use crate::{cache::models, ext::PathExt, Cache, Config, ModPortal};
use log::*;
use mods::{Mods, ModsBuilder};
use settings::ServerSettings;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

const SERVER_SETTINGS_FILENAME: &str = "server-settings.json";
const MODS_PATH: &str = "mods/";

pub type GameCacheId = i64;

pub struct Factorio {
    pub settings: ServerSettings,
    pub mods: Mods,
    root: PathBuf,
    cache_id: Mutex<Option<GameCacheId>>,
    cache: Arc<Cache>,
}

pub struct Importer {
    root: PathBuf,
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

            info!("Updating existing game ID {} cache", c);
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

            info!("Creating new game cache with ID {}", new_id);
            new_id
        };

        self.mods.update_cache(id).await?;
        self.cache.commit_transaction()?;

        info!("Game ID {} cached updated", id);
        Ok(())
    }
}

impl Importer {
    pub fn from_root<P>(root: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            root: root.as_ref().to_path_buf(),
            settings: PathBuf::from(SERVER_SETTINGS_FILENAME),
            game_cache_id: None,
        }
    }

    pub fn from_cache(cached_game: &models::Game) -> Self {
        Self {
            root: PathBuf::from(&cached_game.path),
            settings: PathBuf::from(SERVER_SETTINGS_FILENAME),
            game_cache_id: Some(cached_game.id),
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
        let mut settings_path = self.root.clone();
        settings_path.push(self.settings);

        let mut mods_path = self.root.clone();
        mods_path.push(MODS_PATH);

        let mut mods = ModsBuilder::root(mods_path);
        if let Some(game_cache_id) = self.game_cache_id {
            mods = mods.with_game_cache_id(game_cache_id);
        }

        Ok(Factorio {
            settings: ServerSettings::from_game_json(&fs::read_to_string(settings_path)?)?,
            mods: mods.build(config, portal, Arc::clone(&cache)).await?,
            root: self.root,
            cache_id: Mutex::new(self.game_cache_id),
            cache,
        })
    }
}
