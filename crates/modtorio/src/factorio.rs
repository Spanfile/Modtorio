//! The whole point. Provides the [`Factorio`](Factorio) struct used to interact with a single
//! instance of a Factorio server.

mod mods;
mod settings;

use crate::{
    store::{cache::models, Store},
    util::ext::PathExt,
    Config, ModPortal,
};
use log::*;
use mods::{Mods, ModsBuilder};
use settings::ServerSettings;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

/// The file name of the JSON file used to store a Factorio server's settings.
const SERVER_SETTINGS_FILENAME: &str = "server-settings.json";
/// The path relative to the Factorio server's root directory where the server's mods are stored.
const MODS_PATH: &str = "mods/";

/// The type used to identify games in the program cache.
pub type GameCacheId = i64;

/// Represents a single Factorio server instance.
///
/// Built using an [`Importer`](Importer).
pub struct Factorio {
    /// The server's settings.
    pub settings: ServerSettings,
    /// The server's mods.
    pub mods: Mods,
    /// The server's root directory.
    root: PathBuf,
    /// The program's cache ID.
    cache_id: Mutex<Option<GameCacheId>>,
    /// Reference to the program store.
    store: Arc<Store>,
}

/// Builds a new instance of a [`Factorio`](Factorio) server by importing its information from the
/// filesystem or from the program cache.
pub struct Importer {
    /// The server's root directory.
    root: PathBuf,
    /// The server's `server-settings.json` file's location.
    settings: PathBuf,
    /// The program's cache ID.
    game_cache_id: Option<GameCacheId>,
}

impl Factorio {
    /// Updates all information about this game instance in the program cache.
    pub async fn update_cache(&self) -> anyhow::Result<()> {
        let mut cache_id = self.cache_id.lock().await;

        self.store.begin_transaction()?;

        let id = if let Some(c) = *cache_id {
            self.store
                .cache
                .update_game(models::Game {
                    id: c,
                    path: self.root.get_str()?.to_string(),
                })
                .await?;

            info!("Updating existing game ID {} cache", c);
            c
        } else {
            let new_id = self
                .store
                .cache
                .insert_game(models::Game {
                    id: 0, /* this ID is irrelevant as the actual ID will be dictated by the
                            * database when inserting a new row */
                    path: self.root.get_str()?.to_string(),
                })
                .await?;
            *cache_id = Some(new_id);

            info!("Creating new game cache with ID {}", new_id);
            new_id
        };

        self.mods.update_cache(id).await?;
        self.store.commit_transaction()?;

        info!("Game ID {} cached updated", id);
        Ok(())
    }
}

impl Importer {
    /// Returns a new `Importer` using a certain path as the new Factorio server instance's root
    /// directory.
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

    /// Returns a new `Importer` with information from a cached `Game`.
    pub fn from_cache(cached_game: &models::Game) -> Self {
        Self {
            root: PathBuf::from(&cached_game.path),
            settings: PathBuf::from(SERVER_SETTINGS_FILENAME),
            game_cache_id: Some(cached_game.id),
        }
    }

    /// Specify a custom file to read the server's settings from.
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

    /// Finalise the builder and return the imported Factorio server instance.
    pub async fn import<'a>(
        self,
        config: Arc<Config>,
        portal: Arc<ModPortal>,
        store: Arc<Store>,
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
            mods: mods.build(config, portal, Arc::clone(&store)).await?,
            root: self.root,
            cache_id: Mutex::new(self.game_cache_id),
            store,
        })
    }
}
