//! The whole point. Provides the [`Factorio`](Factorio) struct used to interact with a single
//! instance of a Factorio server.

pub mod executable;
pub mod mods;
pub mod settings;
mod status;

use crate::{
    error::ServerError,
    store::{models, Store},
    util::{
        async_status::{self, AsyncProgressChannel, AsyncProgressChannelExt},
        ext::PathExt,
    },
    Config, ModPortal,
};
use executable::{Executable, ExecutableState, GameState};
use log::*;
use models::GameSettings;
use mods::{Mods, ModsBuilder};
use rpc::{instance_status::game::GameStatus, send_command_request::Command};
use settings::ServerSettings;
use status::ServerStatus;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    sync::{mpsc, Mutex, RwLock},
    task,
};

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
    store_id: Arc<Mutex<Option<GameStoreId>>>,
    /// Reference to the program store.
    store: Arc<Store>,
    /// The server's status.
    status: Arc<RwLock<ServerStatus>>,
    /// The running executable's stdin transmit channel.
    exec_stdin_tx: Mutex<Option<mpsc::Sender<String>>>,
    /// The running executable's stdout receiver channel.
    exec_stdout_rx: Mutex<Option<mpsc::Receiver<String>>>,
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
    prog_tx: Option<AsyncProgressChannel>,
}

impl Factorio {
    /// Updates all information about the instance in the program store.
    pub async fn update_store(&self, prog_tx: Option<AsyncProgressChannel>) -> anyhow::Result<()> {
        self.store.begin_transaction()?;

        let mut store_id = self.store_id.lock().await;
        let id = if let Some(c) = *store_id {
            info!("Updating existing game ID {} store", c);
            prog_tx
                .send_status(async_status::indefinite("Updating existing stored game..."))
                .await?;

            self.store
                .update_game(models::Game {
                    id: c,
                    path: self.root.get_str()?.to_string(),
                })
                .await?;

            c
        } else {
            info!("Creating new stored game...");
            prog_tx
                .send_status(async_status::indefinite("Creating new stored game..."))
                .await?;

            let new_id = self
                .store
                .insert_game(models::Game {
                    id: 0, /* this ID is irrelevant as the actual ID will be dictated by the
                            * database when inserting a new row */
                    path: self.root.get_str()?.to_string(),
                })
                .await?;
            *store_id = Some(new_id);
            debug!("New game store ID: {}", new_id);

            new_id
        };

        let mut new_settings = GameSettings::default();
        new_settings.game = id;
        self.settings.to_store_format(&mut new_settings)?;

        debug!("Created new settings to store: {:?}", new_settings);
        self.store.set_settings(new_settings).await?;

        self.mods.update_store(id, prog_tx).await?;
        self.store.commit_transaction()?;

        info!("Game ID {} store updated", id);
        Ok(())
    }

    /// Runs the server.
    pub async fn run(&self) -> anyhow::Result<()> {
        self.assert_status(GameStatus::Shutdown).await?;
        let store_id = self.store_id().await?;
        debug!("Running game ID {} executable", store_id);

        let (stdin_tx, stdin_rx) = mpsc::channel(64);
        let (stdout_tx, stdout_rx) = mpsc::channel(64);

        *self.exec_stdin_tx.lock().await = Some(stdin_tx);
        *self.exec_stdout_rx.lock().await = Some(stdout_rx);

        self.status.write().await.set_game_status(GameStatus::Starting);
        let mut state_rx = self.executable.run(stdout_tx, stdin_rx).await?;

        let status = Arc::clone(&self.status);
        task::spawn(async move {
            debug!(
                "Game ID {} executable running, beginning listening for state changes",
                store_id
            );
            while let Some(state) = state_rx.recv().await {
                match state {
                    ExecutableState::GameState(game_state) => {
                        debug!("Game ID {} executable got new game state: {:?}", store_id, game_state);

                        match game_state {
                            GameState::InGame => {
                                if status.read().await.game_status() == GameStatus::Starting {
                                    info!("Game ID {} started and is now running", store_id);
                                    status.write().await.set_game_status(GameStatus::Running);
                                }
                            }
                            GameState::DisconnectingScheduled => {
                                if status.read().await.game_status() == GameStatus::Running {
                                    info!("Game ID {} shutting down", store_id);
                                    status.write().await.set_game_status(GameStatus::ShuttingDown);
                                }
                            }
                            _ => {}
                        }
                    }
                    ExecutableState::Exited(exit_result) => {
                        debug!("Game ID {} executable exited with {:?}", store_id, exit_result);
                        if let Err(e) = exit_result {
                            error!("Game ID {} executable exited with error: {:?}", store_id, e);
                            status.write().await.set_game_status(GameStatus::Crashed);
                        } else {
                            info!("Game ID {} exited succesfully", store_id);
                            status.write().await.set_game_status(GameStatus::Shutdown);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Sends a command to the running executable.
    pub async fn send_command(&self, command: Command, arguments: Vec<String>) -> anyhow::Result<()> {
        self.assert_status(GameStatus::Running).await?;

        debug!("Building command from {:?}, arguments: {:?}", command, arguments);
        let mut command_components = Vec::new();
        match command {
            Command::Raw => {
                command_components.extend(arguments);
            }
            Command::Say => {
                // TODO
                todo!()
            }
            Command::Save => {
                command_components.push(String::from("save"));
                command_components.extend(arguments);
            }
            Command::Quit => {
                command_components.push(String::from("quit"));
            }
        }

        let command_string = format!("/{}\n", command_components.join(" "));
        debug!("Final command string: {}", command_string);
        self.write_to_exec_stdin(command_string).await?;

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

    /// Returns the server's store ID. The value is `None` if the server has been newly created and hasn't yet been
    /// added to the program store.
    pub async fn store_id_option(&self) -> Option<GameStoreId> {
        *self.store_id.lock().await
    }

    /// Returns the server's store ID. The value is `None` if the server has been newly created and hasn't yet been
    /// added to the program store.
    pub async fn store_id(&self) -> anyhow::Result<GameStoreId> {
        self.store_id
            .lock()
            .await
            .ok_or_else(|| ServerError::GameNotInStore.into())
    }

    /// Returns the server's status.
    pub async fn game_status(&self) -> GameStatus {
        self.status.read().await.game_status()
    }

    /// Asserts that the server's status is `expected`, otherwise returns `ServerError::InvalidStatus`.
    async fn assert_status(&self, expected: GameStatus) -> anyhow::Result<()> {
        let status = self.game_status().await;
        if status == expected {
            Ok(())
        } else {
            Err(ServerError::InvalidGameStatus(status).into())
        }
    }

    /// Writes a given `String` to the running executable's stdin tx channel.
    async fn write_to_exec_stdin(&self, msg: String) -> anyhow::Result<()> {
        if let Some(stdin_tx) = self.exec_stdin_tx.lock().await.as_mut() {
            stdin_tx.send(msg).await?;
        }

        Ok(())
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
    pub fn with_status_updates(self, prog_tx: AsyncProgressChannel) -> Self {
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
        let mut mods_builder = ModsBuilder::root(self.root.join(MODS_PATH));

        self.prog_tx
            .send_status(async_status::indefinite("Reading server settings..."))
            .await?;
        let settings = if let Some(game_store_id) = self.game_store_id {
            // TODO: ugly side effect
            mods_builder = mods_builder.with_game_store_id(game_store_id);

            let settings = ServerSettings::from_store_format(&store.get_settings(game_store_id).await?)?;
            debug!("Read settings from store: {:?}", settings);
            settings
        } else {
            let settings = ServerSettings::from_game_json(&fs::read_to_string(self.root.join(self.settings))?)?;
            debug!("Read settings from file: {:?}", settings);
            settings
        };

        if let Some(prog_tx) = &self.prog_tx {
            mods_builder = mods_builder.with_status_updates(prog_tx.clone());
        }

        self.prog_tx
            .send_status(async_status::indefinite("Verifying executable..."))
            .await?;
        let executable = Executable::new(self.executable).await?;

        self.prog_tx
            .send_status(async_status::indefinite("Loading mods..."))
            .await?;
        let mods = mods_builder.build(config, portal, Arc::clone(&store)).await?;

        Ok(Factorio {
            settings,
            mods,
            executable,
            root: self.root,
            store_id: Arc::new(Mutex::new(self.game_store_id)),
            store,
            status: Arc::new(RwLock::new(ServerStatus::default())),
            exec_stdin_tx: Mutex::new(None),
            exec_stdout_rx: Mutex::new(None),
        })
    }
}
