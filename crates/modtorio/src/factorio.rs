//! The whole point. Provides the [`Factorio`](Factorio) struct used to interact with a single
//! instance of a Factorio server.

pub mod executable;
pub mod mods;
pub mod settings;

use crate::{
    store::{models, Store},
    util::{ext::PathExt, status, status::AsyncProgressChannelExt},
    Config, ModPortal,
};
use executable::Executable;
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

/// The type used to identify games in the program store.
pub type GameStoreId = i64;

/// Represents a single Factorio server instance.
///
/// Built using an [`Importer`](Importer).
pub struct Factorio {
    /// The server's settings.
    settings: ServerSettings,
    /// The server's mods.
    mods: Mods,
    /// The server's executable.
    executable: Executable,
    /// The server's root directory.
    root: PathBuf,
    /// The program's store ID.
    store_id: Mutex<Option<GameStoreId>>,
    /// Reference to the program store.
    store: Arc<Store>,
}

/// Builds a new instance of a [`Factorio`](Factorio) server by importing its information from the
/// filesystem or from the program store.
pub struct Importer {
    /// The server's root directory.
    root: PathBuf,
    /// The server's `server-settings.json` file's location.
    settings: PathBuf,
    /// The server executable's location.
    executable: PathBuf,
    /// The program's store ID.
    game_store_id: Option<GameStoreId>,
    /// A status update channel.
    prog_tx: Option<status::AsyncProgressChannel>,
}

impl Factorio {
    /// Updates all information about the instance in the program store.
    pub async fn update_store(&self, prog_tx: Option<status::AsyncProgressChannel>) -> anyhow::Result<()> {
        let mut store_id = self.store_id.lock().await;

        self.store.begin_transaction()?;

        let id = if let Some(c) = *store_id {
            self.store
                .update_game(models::Game {
                    id: c,
                    path: self.root.get_str()?.to_string(),
                })
                .await?;

            info!("Updating existing game ID {} store", c);
            prog_tx
                .send_status(status::indefinite("Updating existing stored game..."))
                .await?;
            c
        } else {
            let new_id = self
                .store
                .insert_game(models::Game {
                    id: 0, /* this ID is irrelevant as the actual ID will be dictated by the
                            * database when inserting a new row */
                    path: self.root.get_str()?.to_string(),
                })
                .await?;
            *store_id = Some(new_id);

            info!("Creating new game store with ID {}", new_id);
            prog_tx
                .send_status(status::indefinite("Creating new stored game..."))
                .await?;
            new_id
        };

        self.mods.update_store(id, prog_tx).await?;
        self.store.commit_transaction()?;

        info!("Game ID {} stored updated", id);
        Ok(())
    }

    /// Returns the instance's root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Immutably borrows the server's mods.
    pub fn mods(&self) -> &Mods {
        &self.mods
    }

    /// Mutably borrows the server's mods.
    pub fn mods_mut(&mut self) -> &mut Mods {
        &mut self.mods
    }

    /// Immutably borrows the server's settings.
    pub fn settings(&self) -> &ServerSettings {
        &self.settings
    }

    /// Mutably borrows the server's settings.
    pub fn settings_mut(&mut self) -> &mut ServerSettings {
        &mut self.settings
    }

    /// Immutably borrows the server's executable.
    pub fn executable(&self) -> &Executable {
        &self.executable
    }
}

impl Importer {
    /// Returns a new `Importer` using a certain path as the new Factorio server instance's root
    /// directory.
    pub fn from_root<P>(root: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            root: root.as_ref().canonicalize()?,
            settings: PathBuf::from(SERVER_SETTINGS_FILENAME),
            executable: root.as_ref().join(executable::DEFAULT_PATH),
            game_store_id: None,
            prog_tx: None,
        })
    }

    /// Returns a new `Importer` with information from a stored `Game`.
    pub fn from_store(stored_game: &models::Game) -> Self {
        let root = PathBuf::from(&stored_game.path);
        let executable = root.join(executable::DEFAULT_PATH);

        Self {
            root,
            settings: PathBuf::from(SERVER_SETTINGS_FILENAME),
            executable,
            game_store_id: Some(stored_game.id),
            prog_tx: None,
        }
    }

    /// Specify a custom file to read the server's settings from. The path is relative to the server's root path.
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

    /// Specify a custom server executable path. The path is relative to the server's root path.
    #[allow(dead_code)]
    pub fn with_executable_path<P>(self, executable: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            executable: executable.as_ref().to_path_buf(),
            ..self
        }
    }

    /// Specifies an `AsyncProgressChannel` to use for status updates when importing the game.
    pub fn with_status_updates(self, prog_tx: status::AsyncProgressChannel) -> Self {
        Self {
            prog_tx: Some(prog_tx),
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

        let mut mods_builder = ModsBuilder::root(mods_path);

        if let Some(game_store_id) = self.game_store_id {
            mods_builder = mods_builder.with_game_store_id(game_store_id);
        }

        if let Some(prog_tx) = &self.prog_tx {
            mods_builder = mods_builder.with_status_updates(Arc::clone(prog_tx));
        }

        self.prog_tx
            .send_status(status::indefinite("Reading server settings..."))
            .await?;
        let settings = ServerSettings::from_game_json(&fs::read_to_string(settings_path)?)?;

        self.prog_tx
            .send_status(status::indefinite("Verifying executable..."))
            .await?;
        let executable = Executable::new(self.executable).await?;

        self.prog_tx.send_status(status::indefinite("Loading mods...")).await?;
        let mods = mods_builder.build(config, portal, Arc::clone(&store)).await?;

        Ok(Factorio {
            settings,
            mods,
            executable,
            root: self.root,
            store_id: Mutex::new(self.game_store_id),
            store,
        })
    }
}
