//! The whole point. Provides the [`Factorio`](Factorio) struct used to interact with a single
//! instance of a Factorio server.

mod command;
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
        file,
    },
    Config, ModPortal,
};
use executable::{Executable, ExecutableEvent, GameEvent};
use log::*;
use models::GameSettings;
use mods::{Mods, ModsBuilder};
use settings::{ServerSettings, StartBehaviour};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    sync::{mpsc, watch, Mutex, RwLock},
    task,
};

pub use command::Command;
pub use status::{ExecutionStatus, InGameStatus, ServerStatus};

/// The file name of the JSON file used to store a Factorio server's settings.
const SERVER_SETTINGS_FILENAME: &str = "server-settings.json";
/// The file name of the JSON file used to store a Factorio server's whitelisted players.
const WHITELIST_FILENAME: &str = "server-whitelist.json";
/// The file name of the JSON file used to store a Factorio server's banned players.
const BANLIST_FILENAME: &str = "server-banlist.json";
/// The file name of the JSON file used to store a Factorio server's admins.
const ADMINLIST_FILENAME: &str = "server-adminlist.json";
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
    /// The server's settings file's path relative to the root directory.
    settings_file: Option<PathBuf>,
    /// The server's player whitelist file's path relative to the root directory.
    whitelist_file: Option<PathBuf>,
    /// The server's banlist file's path relative to the root directory.
    banlist_file: Option<PathBuf>,
    /// The server's adminlist file's path relative to the root directory.
    adminlist_file: Option<PathBuf>,
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
    /// The signal receiver for the executable's exit.
    exec_shutdown_rx: Mutex<Option<watch::Receiver<()>>>,
}

/// Builds a new instance of a [`Factorio`](Factorio) server by importing its information from the
/// filesystem or from the program store.
pub struct Importer {
    /// The server's root directory.
    root: PathBuf,
    /// The server's settings file's location relative to the root directory.
    settings: PathBuf,
    /// The server's whitelist file's location relative to the root directory.
    whitelist: PathBuf,
    /// The server's banlist file's location relative to the root directory.
    banlist: PathBuf,
    /// The server's adminlist file's location relative to the root directory.
    adminlist: PathBuf,
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
        let mut new_game_model = models::Game {
            id: 0, /* this ID is irrelevant as the actual ID will be dictated by the
                    * database when inserting a new row, or by the cache ID later */
            path: self.root.get_string()?,
            settings_file: self.settings_file.as_ref().map(|p| p.get_string()).transpose()?,
            whitelist_file: self.whitelist_file.as_ref().map(|p| p.get_string()).transpose()?,
            banlist_file: self.banlist_file.as_ref().map(|p| p.get_string()).transpose()?,
            adminlist_file: self.adminlist_file.as_ref().map(|p| p.get_string()).transpose()?,
        };

        let id = if let Some(c) = *store_id {
            info!("Updating existing game ID {} store", c);
            prog_tx
                .send_status(async_status::indefinite("Updating existing stored game..."))
                .await?;

            new_game_model.id = c;
            self.store.update_game(new_game_model).await?;

            c
        } else {
            info!("Creating new stored game...");
            prog_tx
                .send_status(async_status::indefinite("Creating new stored game..."))
                .await?;

            let new_id = self.store.insert_game(new_game_model).await?;
            *store_id = Some(new_id);
            debug!("New game store ID: {}", new_id);

            new_id
        };

        let mut new_settings = GameSettings::default();
        new_settings.game = id;
        self.settings.to_store_format(&mut new_settings);

        debug!("Created new settings to store: {:?}", new_settings);
        self.store.set_settings(new_settings).await?;

        self.mods.update_store(id, prog_tx).await?;
        self.store.commit_transaction()?;

        info!("Game ID {} store updated", id);
        Ok(())
    }

    /// Runs the server.
    pub async fn run(&self) -> anyhow::Result<()> {
        self.assert_status(ExecutionStatus::Shutdown).await?;
        let store_id = self.store_id().await?;
        debug!("Running game ID {} executable", store_id);

        let (stdin_tx, stdin_rx) = mpsc::channel(64);
        let (stdout_tx, stdout_rx) = mpsc::channel(64);
        *self.exec_stdin_tx.lock().await = Some(stdin_tx);
        *self.exec_stdout_rx.lock().await = Some(stdout_rx);

        let exec_args = self.get_executable_args();
        let mut state_rx = self.executable.run(stdout_tx, stdin_rx, &exec_args).await?;

        let (shutdown_tx, mut shutdown_rx) = watch::channel(());
        shutdown_rx.recv().await;
        *self.exec_shutdown_rx.lock().await = Some(shutdown_rx);

        let status = Arc::clone(&self.status);
        {
            let mut status_w = status.write().await;
            status_w.reset_started_at();
            status_w.set_game_status(ExecutionStatus::Starting);
        }

        task::spawn(async move {
            debug!(
                "Game ID {} executable running, beginning listening for state changes",
                store_id
            );

            while let Some(event) = state_rx.recv().await {
                match event {
                    ExecutableEvent::GameEvent(game_event) => process_game_event(store_id, game_event, &status).await,
                    ExecutableEvent::Exited(exit_result) => {
                        process_exited_event(store_id, exit_result, &status).await;
                        break;
                    }
                }
            }

            shutdown_tx.broadcast(()).expect("failed to send shutdown signal");
        });

        Ok(())
    }

    /// Asynchronously waits for the game executable to shut down. Returns immediately if the executable isn't running.
    pub async fn wait_for_shutdown(&self) {
        if let Some(mut rx) = self.exec_shutdown_rx.lock().await.clone() {
            rx.recv().await;
        }
    }

    /// Sends a command to the running executable.
    pub async fn send_command(&self, command: Command) -> anyhow::Result<()> {
        self.assert_status(ExecutionStatus::Running).await?;

        let command_string = command.get_command_string();
        debug!("Built command string '{}' from command {:?}", command_string, command);
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
    pub async fn status(&self) -> ServerStatus {
        // TODO: docs
        self.status.read().await.clone()
    }

    /// Asserts that the server's status is `expected`, otherwise returns `ServerError::InvalidStatus`.
    async fn assert_status(&self, expected: ExecutionStatus) -> anyhow::Result<()> {
        let status = self.status().await;
        if status.game_status() == expected {
            Ok(())
        } else {
            Err(ServerError::InvalidGameStatus(status.game_status()).into())
        }
    }

    /// Writes a given `String` to the running executable's stdin tx channel.
    async fn write_to_exec_stdin(&self, msg: String) -> anyhow::Result<()> {
        if let Some(stdin_tx) = self.exec_stdin_tx.lock().await.as_mut() {
            trace!("Writing to executable stdin channel: {}", msg);
            stdin_tx.send(msg).await?;
        } else {
            trace!("No executable stdin when trying to write message");
        }

        Ok(())
    }

    /// Returns the proper server executable arguments to match the server's settings.
    fn get_executable_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        match self.settings.start.behaviour {
            StartBehaviour::LoadLatest => args.push(String::from("--start-server-load-latest")),
            StartBehaviour::LoadFile => args.extend(vec![
                String::from("--start-server"),
                self.settings.start.save_name.clone(),
            ]),
            _ => unimplemented!(), // TODO
        }

        args.extend(vec![
            String::from("--bind"),
            self.settings.network.bind_address.to_string(),
        ]);

        args
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
            whitelist: PathBuf::from(WHITELIST_FILENAME),
            banlist: PathBuf::from(BANLIST_FILENAME),
            adminlist: PathBuf::from(ADMINLIST_FILENAME),
            executable: root.as_ref().join(executable::DEFAULT_PATH),
            game_store_id: None,
            prog_tx: None,
        })
    }

    /// Returns a new `Importer` with information from a stored `Game`.
    pub fn from_store(stored_game: &models::Game) -> Self {
        let root = PathBuf::from(&stored_game.path);

        let settings = root.join(if let Some(path) = &stored_game.settings_file {
            path
        } else {
            SERVER_SETTINGS_FILENAME
        });
        let whitelist = root.join(if let Some(path) = &stored_game.whitelist_file {
            path
        } else {
            WHITELIST_FILENAME
        });
        let banlist = root.join(if let Some(path) = &stored_game.banlist_file {
            path
        } else {
            BANLIST_FILENAME
        });
        let adminlist = root.join(if let Some(path) = &stored_game.adminlist_file {
            path
        } else {
            ADMINLIST_FILENAME
        });

        let executable = root.join(executable::DEFAULT_PATH);

        Self {
            root,
            settings,
            whitelist,
            banlist,
            adminlist,
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

    /// Specify a custom file to read the server's whitelisted users from. The path is relative to the server's root
    /// path.
    #[allow(dead_code)]
    pub fn with_whitelist<P>(self, whitelist: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            whitelist: whitelist.as_ref().to_path_buf(),
            ..self
        }
    }

    /// Specify a custom file to read the server's banned players from. The path is relative to the server's root path.
    #[allow(dead_code)]
    pub fn with_banlist<P>(self, banlist: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            banlist: banlist.as_ref().to_path_buf(),
            ..self
        }
    }

    /// Specify a custom file to read the server's admins from. The path is relative to the server's root path.
    #[allow(dead_code)]
    pub fn with_adminlist<P>(self, adminlist: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            adminlist: adminlist.as_ref().to_path_buf(),
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

        let settings_path = self.root.join(&self.settings);
        let _whitelist_path = self.root.join(&self.whitelist);
        let _banlist_path = self.root.join(&self.banlist);
        let _adminlist_path = self.root.join(&self.adminlist);

        debug!("Settings file: {}", settings_path.display());
        debug!("Whitelist file: {}", _whitelist_path.display());
        debug!("Banlist file: {}", _banlist_path.display());
        debug!("Adminlist file: {}", _adminlist_path.display());

        self.prog_tx
            .send_status(async_status::indefinite("Reading server settings..."))
            .await?;
        let settings = if let Some(game_store_id) = self.game_store_id {
            // TODO: ugly side effect
            mods_builder = mods_builder.with_game_store_id(game_store_id);

            let settings = ServerSettings::from_store_format(&store.get_settings(game_store_id).await?)?;
            trace!("Read settings from store: {:?}", settings);

            let file_last_mtime = file::get_last_mtime(&settings_path)?;
            debug!(
                "Settings file last mtime: {}. Stored last mtime: {:?}",
                file_last_mtime, settings.file_last_mtime
            );

            if let Some(stored_last_mtime) = settings.file_last_mtime {
                if file_last_mtime > stored_last_mtime {
                    warn!("Settings file modified after storing. Reloading settings from file");
                    ServerSettings::from_game_json(&settings_path)?
                } else {
                    settings
                }
            } else {
                warn!("Stored settings did not have last mtime field. Reloading settings from file");
                ServerSettings::from_game_json(&settings_path)?
            }
        } else {
            let settings = ServerSettings::from_game_json(&settings_path)?;
            trace!("Read settings from file");
            settings
        };
        debug!("Server settings: {:?}", settings);

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
            settings_file: if self.settings.to_string_lossy() == SERVER_SETTINGS_FILENAME {
                None
            } else {
                Some(self.settings)
            },
            whitelist_file: if self.whitelist.to_string_lossy() == WHITELIST_FILENAME {
                None
            } else {
                Some(self.whitelist)
            },
            banlist_file: if self.banlist.to_string_lossy() == BANLIST_FILENAME {
                None
            } else {
                Some(self.banlist)
            },
            adminlist_file: if self.adminlist.to_string_lossy() == ADMINLIST_FILENAME {
                None
            } else {
                Some(self.adminlist)
            },
            store_id: Arc::new(Mutex::new(self.game_store_id)),
            store,
            status: Arc::new(RwLock::new(ServerStatus::default())),
            exec_stdin_tx: Mutex::new(None),
            exec_stdout_rx: Mutex::new(None),
            exec_shutdown_rx: Mutex::new(None),
        })
    }
}

/// Processes a given `GameEvent` for a certain game (identified by `store_id`) and modifies a given `ServerStatus`
/// accordingly.
async fn process_game_event(store_id: GameStoreId, event: GameEvent, status: &RwLock<ServerStatus>) {
    debug!("Game ID {} got new game event: {:?}", store_id, event);

    match event {
        GameEvent::GameStateChanged { from: _, to } => {
            let mut status_w = status.write().await;
            status_w.set_in_game_status(to);

            match to {
                InGameStatus::InGame => {
                    if status_w.game_status() == ExecutionStatus::Starting {
                        info!("Game ID {} started and is now running", store_id);
                        status_w.set_game_status(ExecutionStatus::Running);
                    }
                }
                InGameStatus::DisconnectingScheduled => {
                    if status_w.game_status() == ExecutionStatus::Running {
                        info!("Game ID {} shutting down", store_id);
                        status_w.set_game_status(ExecutionStatus::ShuttingDown);
                    }
                }
                in_game_status => {
                    trace!("Unhandled in-game status: {:?}", in_game_status);
                }
            }
        }
        GameEvent::RefusingConnection { peer, username, reason } => {
            info!(
                "Game ID {} refusing connection for '{}' (addr {}): {}",
                store_id, username, peer, reason
            );
        }
        GameEvent::PeerJoined { username } => match status.write().await.add_player(&username).await {
            Ok(_) => info!("Game ID {}: {} joined the game", store_id, username),
            Err(e) => error!("Failed to add new player in game ID {}: {}", store_id, e),
        },
        GameEvent::PeerLeft { username } => match status.write().await.remove_player(&username).await {
            Ok(_) => info!("Game ID {}: {} left the game", store_id, username),
            Err(e) => error!("Failed to remove player from game ID {}: {}", store_id, e),
        },
        _ => {}
    }
}

/// Processes a given executable exit event for a certain game (identified by `store_id`) and modifies a given
/// `ServerStatus` accordingly.
async fn process_exited_event(store_id: GameStoreId, exit_result: anyhow::Result<()>, status: &RwLock<ServerStatus>) {
    debug!("Game ID {} executable exited with {:?}", store_id, exit_result);

    if let Err(e) = exit_result {
        error!("Game ID {} executable exited with error: {:?}", store_id, e);
        status.write().await.set_game_status(ExecutionStatus::Crashed);
    } else {
        info!("Game ID {} exited succesfully", store_id);
        status.write().await.set_game_status(ExecutionStatus::Shutdown);
    }
}
