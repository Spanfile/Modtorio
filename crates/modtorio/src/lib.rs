//! A wrapper for a headless Linux Factorio server to allow higher control over the server's
//! functionality.

#![feature(drain_filter)]
#![feature(async_closure)]
#![feature(associated_type_bounds)]
#![feature(thread_id_value)]
#![warn(clippy::if_not_else)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::missing_docs_in_private_items)]
// #![warn(clippy::pedantic)]

pub mod config;
pub mod error;
pub mod factorio;
pub mod log;
pub mod mod_common;
pub mod mod_portal;
pub mod opts;
pub mod store;
mod unix;
pub mod util;

use ::log::*;
use chrono::{DateTime, Utc};
use common::net::NetAddress;
use config::Config;
use error::{ModPortalError, RpcError};
use factorio::{Factorio, GameStoreId, ServerStatus};
use futures::{future::try_join_all, TryStreamExt};
use lazy_static::lazy_static;
use mod_portal::ModPortal;
use rpc::{instance_status, mod_rpc_server, send_command_request};
use std::{path::Path, sync::Arc};
use store::Store;
use tokio::{
    fs,
    net::UnixListener,
    sync::{mpsc, Mutex},
    task,
};
use tonic::{transport::Server, Request, Response, Status};
use util::{
    async_status,
    async_status::{AsyncProgressChannel, AsyncProgressChannelExt, AsyncProgressResult},
    HumanVersion,
};

/// The prefix used with every environment value related to the program configuration.
pub const APP_PREFIX: &str = "MODTORIO_";
/// The program's version at build-time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    static ref HVER_VERSION: HumanVersion = {
        VERSION
            .parse()
            .expect("failed to parse VERSION constant as HumanVersion")
    };
}

#[derive(Clone)]
/// A wrapper for a headless Linux Factorio server.
pub struct Modtorio {
    /// The program config.
    config: Arc<Config>,
    /// The mod portal.
    portal: Arc<ModPortal>,
    /// The program store.
    store: Arc<Store>,
    /// Collection of Factorio instances this Modtorio instance is managing.
    games: Arc<Mutex<Vec<Factorio>>>,
    /// Timestamp when this Modtorio instance was started.
    started_at: Arc<DateTime<Utc>>,
    /// The instance's status.
    status: Arc<Mutex<instance_status::Status>>,
}

impl Modtorio {
    /// Creates a new Modtorio instance with a given configuration object and a program store
    /// object.
    pub async fn new(config: Config, store: Store) -> anyhow::Result<Self> {
        let config = Arc::new(config);
        let store = Arc::new(store);

        let portal = Arc::new(ModPortal::new(Arc::clone(&config))?);
        let instance = Modtorio {
            config,
            portal,
            store,
            games: Arc::new(Mutex::new(Vec::new())),
            started_at: Arc::new(Utc::now()),
            status: Arc::new(Mutex::new(instance_status::Status::Starting)),
        };

        let i = instance.clone();
        task::spawn(async move {
            info!("Loading previous games...");
            let stored_games = match i.store.get_games().await {
                Ok(games) => games,
                Err(e) => {
                    error!("Failed to get stored games: {}", e);
                    return;
                }
            };
            let mut games = Vec::new();
            debug!("Got stored games: {:?}", stored_games);

            for stored_game in &stored_games {
                info!(
                    "Importing stored game ID {} from path {}...",
                    stored_game.id, stored_game.path
                );

                let game = match factorio::Importer::from_store(stored_game)
                    .import(Arc::clone(&i.config), Arc::clone(&i.portal), Arc::clone(&i.store))
                    .await
                {
                    Ok(game) => game,
                    Err(e) => {
                        error!("Failed to import stored game ID {}: {}", stored_game.id, e);
                        continue;
                    }
                };

                info!(
                    "Stored game ID {} imported from {}. {} mods",
                    stored_game.id,
                    stored_game.path,
                    game.mods().count()
                );
                debug!("Stored game: {:?}", stored_game);
                games.push(game);
            }

            info!("{} previous games loaded.", games.len());
            i.games.lock().await.extend(games);
            *i.status.lock().await = instance_status::Status::Running;
        });

        Ok(instance)
    }

    /// Runs a given Modtorio instance.
    pub async fn run(self) -> anyhow::Result<()> {
        let server_task = tokio::spawn(self.run_rpc());

        if let Err(e) = tokio::try_join!(server_task) {
            error!("Async task failed with: {}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }

    /// Runs the RPC server.
    async fn run_rpc(self) -> anyhow::Result<()> {
        let listen_addresses = self.config.listen();

        if listen_addresses.is_empty() {
            return Err(error::ConfigError::NoListenAddresses.into());
        }

        let mut rpc_listeners = Vec::new();

        for listen in listen_addresses {
            // TODO: TLS
            let server = Server::builder().add_service(mod_rpc_server::ModRpcServer::new(self.clone()));
            rpc_listeners.push(match listen {
                NetAddress::TCP(addr) => {
                    debug!("Starting RPC server on TCP {}", addr);

                    let addr = *addr;
                    task::spawn(async move {
                        server
                            .serve_with_shutdown(addr, term_signal())
                            .await
                            .expect("RPC TCP listener failed");
                        debug!("RPC TCP listener on {} shut down", addr);
                    })
                }
                NetAddress::Unix(path) => {
                    debug!("Starting RPC server on Unix {}", path.display());

                    let path = path.to_owned();
                    task::spawn(async move {
                        let mut unix = UnixListener::bind(&path).expect("failed to bind to unix socket");
                        server
                            .serve_with_incoming_shutdown(unix.incoming().map_ok(unix::UnixStream), term_signal())
                            .await
                            .expect("RPC Unix listener failed");

                        // since the socket we had was created with bind(), we have to remove it with unlink after
                        // we're done with it. right now Rust's remove_file corresponds to unlink, but it might not in
                        // the future
                        debug!("RPC Unix listener on {} shut down, removing socket", path.display());
                        fs::remove_file(&path).await.expect("failed to remove socket");
                    })
                }
            });
        }

        try_join_all(rpc_listeners).await?;
        Ok(())
    }

    /// Asserts that the instance's current status is `wanted`.
    async fn assert_instance_status(&self, wanted: instance_status::Status) -> anyhow::Result<()> {
        let status = self.get_instance_status().await;
        if status == instance_status::Status::Starting {
            error!(
                "RPC instance status assertion failed: wanted {:?}, actual {:?}",
                wanted, status
            );
            Err(RpcError::InvalidInstanceStatus { wanted, actual: status }.into())
        } else {
            Ok(())
        }
    }

    /// Returns a boolean on whether this instance manages a game identified by its root path.
    async fn game_exists_by_path<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        self.games
            .lock()
            .await
            .iter()
            .any(|game| util::file::are_same(game.root(), path.as_ref()).expect("failed to compare file paths"))
    }

    /// Returns this instance's uptime.
    async fn get_uptime(&self) -> chrono::Duration {
        Utc::now() - *self.started_at
    }

    /// Returns this instance's managed games in RPC format.
    async fn get_rpc_games(&self) -> Vec<instance_status::Game> {
        let mut rpc_games = Vec::new();

        for game in self.games.lock().await.iter() {
            let status = game.status().await.game_status() as i32;
            let game_id = game.store_id_option().await.unwrap_or(0);

            rpc_games.push(instance_status::Game {
                path: format!("{}", game.root().display()),
                status,
                game_id,
            });
        }

        rpc_games
    }

    /// Returns this instance's status.
    async fn get_instance_status(&self) -> instance_status::Status {
        *self.status.lock().await
    }

    /// Imports a new Factorio instance from a given path to its root directory.
    async fn import_game<P>(self, path: P, prog_tx: AsyncProgressChannel)
    where
        P: AsRef<Path>,
    {
        if let Err(e) = self.assert_instance_status(instance_status::Status::Running).await {
            send_error_status(&prog_tx, e).await;
            return;
        }

        if self.game_exists_by_path(&path).await {
            error!(
                "RPC tried to import already existing game from path {}",
                path.as_ref().display()
            );
            send_error_status(&prog_tx, RpcError::GameAlreadyExists(path.as_ref().to_path_buf())).await;
            return;
        }

        let path = path.as_ref().to_path_buf();
        task::spawn(async move {
            let importer = match factorio::Importer::from_root(&path) {
                Ok(i) => i,
                Err(e) => {
                    error!("Failed to create new Factorio importer: {}", e);
                    send_error_status(&prog_tx, e).await;
                    return;
                }
            };

            let game = match importer
                .with_status_updates(prog_tx.clone())
                .import(
                    Arc::clone(&self.config),
                    Arc::clone(&self.portal),
                    Arc::clone(&self.store),
                )
                .await
            {
                Ok(game) => {
                    info!("Imported new Factorio server instance from {}", path.display());
                    if !send_status(&prog_tx, async_status::indefinite("Game imported")).await {
                        return;
                    }
                    game
                }
                Err(e) => {
                    error!("Failed to import game: {}", e);
                    send_error_status(&prog_tx, e).await;
                    return;
                }
            };

            if let Err(e) = game.update_store(Some(prog_tx.clone())).await {
                error!("Failed to update game store: {}", e);
                send_error_status(&prog_tx, e).await;
                return;
            }

            self.games.lock().await.push(game);
            send_status(&prog_tx, async_status::done()).await;
        });
    }

    /// Updates a given game instance's store.
    async fn update_store(self, game_id: GameStoreId, prog_tx: AsyncProgressChannel) {
        if let Err(e) = self.assert_instance_status(instance_status::Status::Running).await {
            send_error_status(&prog_tx, e).await;
            return;
        }

        task::spawn(async move {
            let mut games = self.games.lock().await;
            match find_game(game_id, &mut games).await {
                Ok(game) => {
                    if let Err(e) = game.update_store(Some(prog_tx.clone())).await {
                        error!("Failed to update game store: {}", e);
                        send_error_status(&prog_tx, e).await;
                        return;
                    }

                    send_status(&prog_tx, async_status::done()).await
                }
                Err(e) => send_error_status(&prog_tx, e).await,
            };
        });
    }

    /// Installs a mod to a given game instance.
    async fn install_mod(
        self,
        game_id: GameStoreId,
        mod_name: String,
        version: Option<HumanVersion>,
        prog_tx: AsyncProgressChannel,
    ) {
        if let Err(e) = self.assert_instance_status(instance_status::Status::Running).await {
            send_error_status(&prog_tx, e).await;
            return;
        }

        task::spawn(async move {
            let mut games = self.games.lock().await;
            match find_game(game_id, &mut games).await {
                Ok(game) => {
                    if let Err(e) = game
                        .mods_mut()
                        .add_from_portal(&mod_name, version, Some(prog_tx.clone()))
                        .await
                    {
                        if let Some(ModPortalError::ClientError(reqwest::StatusCode::NOT_FOUND)) = e.downcast_ref() {
                            error!("Failed to install mod '{}': not found ({})", mod_name, e);
                            send_error_status(&prog_tx, RpcError::NoSuchMod(mod_name)).await;
                        } else {
                            error!("Failed to install mod '{}': {}", mod_name, e);
                            send_error_status(&prog_tx, e).await;
                        }
                        return;
                    }

                    send_status(&prog_tx, async_status::done()).await
                }
                Err(e) => send_error_status(&prog_tx, e).await,
            };
        });
    }

    /// Updates the installed mods of a given game instance.
    async fn update_mods(self, game_id: GameStoreId, prog_tx: AsyncProgressChannel) {
        if let Err(e) = self.assert_instance_status(instance_status::Status::Running).await {
            send_error_status(&prog_tx, e).await;
            return;
        }

        task::spawn(async move {
            let mut games = self.games.lock().await;
            match find_game(game_id, &mut games).await {
                Ok(game) => {
                    if let Err(e) = game.mods_mut().update(Some(prog_tx.clone())).await {
                        error!("Failed to update mods: {}", e);
                        send_error_status(&prog_tx, e).await;
                        return;
                    }

                    send_status(&prog_tx, async_status::done()).await
                }
                Err(e) => send_error_status(&prog_tx, e).await,
            };
        });
    }

    /// Updates the installed mods of a given game instance.
    async fn ensure_mod_dependencies(self, game_id: GameStoreId, prog_tx: AsyncProgressChannel) {
        if let Err(e) = self.assert_instance_status(instance_status::Status::Running).await {
            send_error_status(&prog_tx, e).await;
            return;
        }

        task::spawn(async move {
            let mut games = self.games.lock().await;
            match find_game(game_id, &mut games).await {
                Ok(game) => {
                    if let Err(e) = game.mods_mut().ensure_dependencies(Some(prog_tx.clone())).await {
                        error!("Failed to ensure mod dependencies: {}", e);
                        send_error_status(&prog_tx, e).await;
                        return;
                    }

                    send_status(&prog_tx, async_status::done()).await
                }
                Err(e) => send_error_status(&prog_tx, e).await,
            };
        });
    }

    /// Retrieves a given game instance's server settings.
    async fn get_server_settings(&self, game_id: GameStoreId) -> anyhow::Result<rpc::ServerSettings> {
        self.assert_instance_status(instance_status::Status::Running).await?;

        let mut games = self.games.lock().await;
        let game = find_game(game_id, &mut games).await?;
        let mut rpc_server_settings = rpc::ServerSettings::default();
        game.settings().to_rpc_format(&mut rpc_server_settings)?;

        Ok(rpc_server_settings)
    }

    /// Sets a given game instance's server settings.
    async fn set_server_settings(
        &self,
        game_id: GameStoreId,
        settings: Option<rpc::ServerSettings>,
    ) -> anyhow::Result<()> {
        self.assert_instance_status(instance_status::Status::Running).await?;

        let mut games = self.games.lock().await;
        let game = find_game(game_id, &mut games).await?;
        let server_settings = if let Some(settings) = settings {
            info!("Updating server ID {}'s settings", game_id);
            factorio::settings::ServerSettings::from_rpc_format(&settings)?
        } else {
            info!("Resetting server ID {}'s settings to default", game_id);
            factorio::settings::ServerSettings::default()
        };

        debug!("{:?}", server_settings);
        *game.settings_mut() = server_settings;

        Ok(())
    }

    /// Runs a given game instance.
    async fn run_server(&self, game_id: GameStoreId) -> anyhow::Result<()> {
        self.assert_instance_status(instance_status::Status::Running).await?;

        let mut games = self.games.lock().await;
        let game = find_game(game_id, &mut games).await?;

        if let Err(e) = game.run().await {
            error!("Server ID {} failed to run: {}", game_id, e);
            Err(e)
        } else {
            info!("Server ID {} starting", game_id);
            Ok(())
        }
    }

    /// Sends a command to a given game instance.
    async fn send_server_command(
        &self,
        game_id: GameStoreId,
        command: i32,
        arguments: Vec<String>,
    ) -> anyhow::Result<()> {
        self.assert_instance_status(instance_status::Status::Running).await?;

        let mut games = self.games.lock().await;
        let game = find_game(game_id, &mut games).await?;
        let command = match command {
            0 => send_command_request::Command::Raw,
            i => return Err(RpcError::NoSuchCommand(i).into()),
        };

        game.send_command(command, arguments).await?;

        Ok(())
    }

    /// Sends a command to a given game instance.
    async fn get_server_status(&self, game_id: GameStoreId) -> anyhow::Result<ServerStatus> {
        self.assert_instance_status(instance_status::Status::Running).await?;

        let mut games = self.games.lock().await;
        let game = find_game(game_id, &mut games).await?;

        Ok(game.status().await)
    }
}

#[tonic::async_trait]
impl mod_rpc_server::ModRpc for Modtorio {
    type ImportGameStream = mpsc::Receiver<Result<rpc::Progress, Status>>;
    type UpdateStoreStream = mpsc::Receiver<Result<rpc::Progress, Status>>;
    type InstallModStream = mpsc::Receiver<Result<rpc::Progress, Status>>;
    type UpdateModsStream = mpsc::Receiver<Result<rpc::Progress, Status>>;
    type EnsureModDependenciesStream = mpsc::Receiver<Result<rpc::Progress, Status>>;

    async fn get_version_information(
        &self,
        req: Request<rpc::Empty>,
    ) -> Result<Response<rpc::VersionInformation>, Status> {
        log_rpc_request(&req);

        respond(rpc::VersionInformation {
            version: Some((*HVER_VERSION).into()),
            protocol_version: Some(
                rpc::VERSION
                    .parse::<HumanVersion>()
                    .expect("failed to parse RPC protocol buffer specification version as HumanVersion")
                    .into(),
            ),
        })
    }

    async fn get_instance_status(&self, req: Request<rpc::Empty>) -> Result<Response<rpc::InstanceStatus>, Status> {
        log_rpc_request(&req);

        let uptime = self.get_uptime().await;
        let games = self.get_rpc_games().await;
        let instance_status = self.get_instance_status().await;

        respond(rpc::InstanceStatus {
            uptime: uptime.num_seconds(),
            games,
            instance_status: instance_status.into(),
        })
    }

    // I tried to macro these repetitive functions into DRYness but the tonic::async_trait macro messes with them in
    // some funky way that a macro_rules! didn't work as I'd hoped and I just couldn't bother to figure it out
    async fn import_game(&self, req: Request<rpc::ImportRequest>) -> Result<Response<Self::ImportGameStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        self.clone().import_game(msg.path, tx).await;

        respond(rx)
    }

    async fn update_store(
        &self,
        req: Request<rpc::UpdateStoreRequest>,
    ) -> Result<Response<Self::UpdateStoreStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        self.clone().update_store(msg.game_id, tx).await;

        respond(rx)
    }

    async fn install_mod(
        &self,
        req: Request<rpc::InstallModRequest>,
    ) -> Result<Response<Self::InstallModStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        let version = msg.mod_version.map(HumanVersion::from);
        self.clone().install_mod(msg.game_id, msg.mod_name, version, tx).await;

        respond(rx)
    }

    async fn update_mods(
        &self,
        req: Request<rpc::UpdateModsRequest>,
    ) -> Result<Response<Self::UpdateModsStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        self.clone().update_mods(msg.game_id, tx).await;

        respond(rx)
    }

    async fn ensure_mod_dependencies(
        &self,
        req: Request<rpc::EnsureModDependenciesRequest>,
    ) -> Result<Response<Self::EnsureModDependenciesStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        self.clone().ensure_mod_dependencies(msg.game_id, tx).await;

        respond(rx)
    }

    async fn get_server_settings(
        &self,
        req: Request<rpc::GetServerSettingsRequest>,
    ) -> Result<Response<rpc::ServerSettings>, Status> {
        log_rpc_request(&req);

        let msg = req.into_inner();
        map_to_response(self.get_server_settings(msg.game_id).await)
    }

    async fn set_server_settings(
        &self,
        req: Request<rpc::SetServerSettingsRequest>,
    ) -> Result<Response<rpc::Empty>, Status> {
        log_rpc_request(&req);

        let msg = req.into_inner();
        map_to_response(self.set_server_settings(msg.game_id, msg.settings).await)
    }

    async fn run_server(&self, req: Request<rpc::RunServerRequest>) -> Result<Response<rpc::Empty>, Status> {
        log_rpc_request(&req);

        let msg = req.into_inner();
        map_to_response(self.run_server(msg.game_id).await)
    }

    async fn send_server_command(&self, req: Request<rpc::SendCommandRequest>) -> Result<Response<rpc::Empty>, Status> {
        log_rpc_request(&req);

        let msg = req.into_inner();
        map_to_response(self.send_server_command(msg.game_id, msg.command, msg.arguments).await)
    }

    async fn get_server_status(
        &self,
        req: Request<rpc::ServerStatusRequest>,
    ) -> Result<Response<rpc::ServerStatus>, Status> {
        log_rpc_request(&req);

        let msg = req.into_inner();
        map_to_response(self.get_server_status(msg.game_id).await)
    }
}

/// Creates a new bounded channel and returns the receiver and sender, the sender wrapped in an
/// Arc<Mutex>.
fn channel<T>() -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
    let (tx, rx) = mpsc::channel(64);
    (tx, rx)
}

/// Logs a given RPC request.
fn log_rpc_request<T: std::fmt::Debug>(request: &Request<T>) {
    debug!(
        "RPC request from {}: {:?}",
        request
            .remote_addr()
            // TODO: this is a bit of stupid hack but; when using an Unix socket, the RPC server takes in a stream of
            // incoming connections which *don't* include the peer's socket address. in which case the socket address
            // here is None, so just call it "Unix"
            .map_or_else(|| String::from("Unix"), |addr| addr.to_string()),
        request.get_ref()
    );
    debug!("{:?}", request);
}

/// Sends a status update to a given channel, returning a boolean on whether the sending succeeded or not.
async fn send_status(prog_tx: &AsyncProgressChannel, status: AsyncProgressResult) -> bool {
    if let Err(e) = prog_tx.send_status(status).await {
        error!("Failed to send status update: {}", e);
        false
    } else {
        true
    }
}

/// Sends an error status update to a given channel, returning a boolean whether the sending succeeded or not.
async fn send_error_status<T>(prog_tx: &AsyncProgressChannel, error: T) -> bool
where
    T: Into<anyhow::Error>,
{
    let error = error.into();
    if let Some(rpc_error) = error.downcast_ref::<RpcError>() {
        send_status(&prog_tx, Err(rpc_error.into())).await
    } else {
        send_status(&prog_tx, Err(RpcError::from(error).into())).await
    }
}

/// Asynchronously returns the unit type after the current process receives a SIGINT signal (Ctrl-C).
async fn term_signal() {
    tokio::signal::ctrl_c().await.expect("failed to listen for SIGINT");
}

/// Creates a new RPC response with a given message, logs it and returns it wrapped in `Ok()`.
fn respond<T: std::fmt::Debug>(message: T) -> Result<Response<T>, Status> {
    let resp = Response::new(message);
    debug!("{:?}", resp);
    Ok(resp)
}

/// Creates a new RPC error respose, logs it and returns it.
fn respond_err<T>(error: anyhow::Error) -> Result<Response<T>, Status> {
    error!("RPC request failed: {}", error);

    if let Some(rpc_error) = error.downcast_ref::<RpcError>() {
        Err(rpc_error.into())
    } else {
        Err(RpcError::Internal(error).into())
    }
}

/// Maps a given `anyhow::Result` into an RPC response.
fn map_to_response<TResult, TResponse>(result: anyhow::Result<TResult>) -> Result<Response<TResponse>, Status>
where
    TResponse: std::fmt::Debug,
    TResult: Into<TResponse>,
{
    match result {
        Ok(result) => respond(result.into()),
        Err(e) => respond_err(e),
    }
}

/// Finds and returns a mutable reference to a game based on its store ID, or returns `RpcError::NoSuchGame` if the game
/// isn't found.
async fn find_game(game_id: GameStoreId, games: &mut Vec<Factorio>) -> anyhow::Result<&mut Factorio> {
    for g in games.iter_mut() {
        if let Some(id) = g.store_id_option().await {
            if id == game_id {
                return Ok(g);
            }
        }
    }

    Err(RpcError::NoSuchGame(game_id).into())
}
