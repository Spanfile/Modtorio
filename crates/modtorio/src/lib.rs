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
pub mod util;

use ::log::*;
use chrono::{DateTime, Utc};
use common::net::NetAddress;
use config::Config;
use error::ModPortalError;
use factorio::Factorio;
use lazy_static::lazy_static;
use mod_portal::ModPortal;
use rpc::{
    mod_rpc_server::{ModRpc, ModRpcServer},
    server_status::{game::GameStatus, Game, InstanceStatus},
    Empty, EnsureModDependenciesRequest, ImportRequest, InstallModRequest, Progress, ServerStatus, UpdateCacheRequest,
    UpdateModsRequest, VersionInformation,
};
use std::{path::Path, sync::Arc};
use store::Store;
use tokio::{
    sync::{mpsc, Mutex},
    task,
};
use tonic::{transport::Server, Request, Response, Status};
use util::{
    status,
    status::{AsyncProgressChannel, AsyncProgressChannelExt, AsyncProgressResult},
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
    status: Arc<Mutex<InstanceStatus>>,
}

impl Modtorio {
    /// Creates a new Modtorio instance with a given configuration object and a program store
    /// object.
    pub async fn new(config: Config, store: Store) -> anyhow::Result<Self> {
        let config = Arc::new(config);
        let store = Arc::new(store);

        let portal = Arc::new(ModPortal::new(&config)?);
        let instance = Modtorio {
            config,
            portal,
            store,
            games: Arc::new(Mutex::new(Vec::new())),
            started_at: Arc::new(Utc::now()),
            status: Arc::new(Mutex::new(InstanceStatus::Starting)),
        };

        let i = instance.clone();
        task::spawn(async move {
            info!("Loading previous games...");
            let cached_games = match i.store.cache.get_games().await {
                Ok(games) => games,
                Err(e) => {
                    error!("Failed to get cached games: {}", e);
                    return;
                }
            };
            let mut games = Vec::new();
            debug!("Got cached games: {:?}", cached_games);

            for cached_game in &cached_games {
                info!(
                    "Importing cached game ID {} from path {}...",
                    cached_game.id, cached_game.path
                );

                let game = match factorio::Importer::from_cache(cached_game)
                    .import(Arc::clone(&i.config), Arc::clone(&i.portal), Arc::clone(&i.store))
                    .await
                {
                    Ok(game) => game,
                    Err(e) => {
                        error!("Failed to import cached game: {}", e);
                        return;
                    }
                };

                info!(
                    "Cached game ID {} imported from {}. {} mods",
                    cached_game.id,
                    cached_game.path,
                    game.mods.count()
                );
                debug!("Cached game: {:?}", cached_game);
                games.push(game);
            }

            info!("{} previous games loaded.", games.len());
            i.games.lock().await.extend(games);
            *i.status.lock().await = InstanceStatus::Running;
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
        if listen_addresses.len() > 1 {
            unimplemented!("listening to multiple addresses not yet supported");
        }

        let listen = listen_addresses.first().unwrap().clone();
        let addr = match listen {
            NetAddress::TCP(addr) => addr,
            NetAddress::Unix(_) => unimplemented!(),
        };

        debug!("Starting RPC server. Listening on {}", addr);
        Server::builder()
            .add_service(ModRpcServer::new(self))
            .serve(addr)
            .await?;
        Ok(())
    }

    /// Asserts that the instance's current status is `wanted`.
    async fn assert_instance_status(&self, wanted: InstanceStatus, prog_tx: status::AsyncProgressChannel) -> bool {
        let status = self.get_instance_status().await;
        if status == InstanceStatus::Starting {
            error!(
                "RPC instance status assertion failed: wanted {:?}, actual {:?}",
                wanted, status
            );
            send_status(
                &prog_tx,
                status::failed_precondition("Modtorio instance is still starting up"),
            )
            .await;
            false
        } else {
            true
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
    async fn get_rpc_games(&self) -> Vec<Game> {
        let games = self.games.lock().await;
        games
            .iter()
            .map(|game| Game {
                path: format!("{}", game.root().display()),
                status: GameStatus::Shutdown.into(),
            })
            .collect()
    }

    /// Returns this instance's status.
    async fn get_instance_status(&self) -> InstanceStatus {
        *self.status.lock().await
    }

    /// Imports a new Factorio instance from a given path to its root directory.
    async fn import_game<P>(self, path: P, prog_tx: status::AsyncProgressChannel)
    where
        P: AsRef<Path>,
    {
        if self
            .assert_instance_status(InstanceStatus::Running, prog_tx.clone())
            .await
        {
            if self.game_exists_by_path(&path).await {
                error!(
                    "RPC tried to import already existing game from path {}",
                    path.as_ref().display()
                );
                send_status(
                    &prog_tx,
                    status::invalid_argument(&format!(
                        "A game from the directory {} already exists.",
                        path.as_ref().display()
                    )),
                )
                .await;
                return;
            }

            let path = path.as_ref().to_path_buf();
            task::spawn(async move {
                let game = match factorio::Importer::from_root(&path)
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
                        if !send_status(&prog_tx, status::indefinite("Game imported")).await {
                            return;
                        }
                        game
                    }
                    Err(e) => {
                        error!("Failed to import game: {}", e);
                        send_status(
                            &prog_tx,
                            status::internal_error(&format!("Failed to import game: {}", e)),
                        )
                        .await;
                        return;
                    }
                };

                if let Err(e) = game.update_cache(Some(prog_tx.clone())).await {
                    error!("Failed to update game cache: {}", e);
                    send_status(
                        &prog_tx,
                        status::internal_error(&format!("Failed to update game cache: {}", e)),
                    )
                    .await;
                    return;
                }

                self.games.lock().await.push(game);
                send_status(&prog_tx, status::indefinite("Done")).await;
            });
        }
    }

    /// Updates a given game instance's cache.
    async fn update_cache(self, server_index: usize, prog_tx: status::AsyncProgressChannel) {
        if self
            .assert_instance_status(InstanceStatus::Running, prog_tx.clone())
            .await
        {
            task::spawn(async move {
                let games = self.games.lock().await;
                let game = games.get(server_index);
                if let Some(game) = game {
                    if let Err(e) = game.update_cache(Some(prog_tx.clone())).await {
                        error!("Failed to update game cache: {}", e);
                        send_status(
                            &prog_tx,
                            status::internal_error(&format!("Failed to update game cache: {}", e)),
                        )
                        .await;
                        return;
                    }

                    send_status(&prog_tx, status::indefinite("Done")).await;
                } else {
                    send_status(
                        &prog_tx,
                        status::invalid_argument(&format!("No such game index: {}", server_index)),
                    )
                    .await;
                }
            });
        }
    }

    /// Installs a mod to a given game instance.
    async fn install_mod(
        self,
        game_index: usize,
        mod_name: String,
        version: Option<HumanVersion>,
        prog_tx: status::AsyncProgressChannel,
    ) {
        if self
            .assert_instance_status(InstanceStatus::Running, prog_tx.clone())
            .await
        {
            task::spawn(async move {
                let mut games = self.games.lock().await;
                let game = games.get_mut(game_index);
                if let Some(game) = game {
                    if let Err(e) = game
                        .mods
                        .add_from_portal(&mod_name, version, Some(prog_tx.clone()))
                        .await
                    {
                        if let Some(ModPortalError::ClientError(reqwest::StatusCode::NOT_FOUND)) = e.downcast_ref() {
                            error!("Failed to install mod '{}': not found ({})", mod_name, e);
                            send_status(
                                &prog_tx,
                                status::invalid_argument(&format!(
                                    "Failed to install mod '{}': not found ({})",
                                    mod_name, e
                                )),
                            )
                            .await;
                        } else {
                            error!("Failed to install mod '{}': {}", mod_name, e);
                            send_status(
                                &prog_tx,
                                status::internal_error(&format!("Failed to install mod: {}", e)),
                            )
                            .await;
                        }
                        return;
                    }

                    send_status(&prog_tx, status::indefinite("Done")).await;
                } else {
                    send_status(
                        &prog_tx,
                        status::invalid_argument(&format!("No such game index: {}", game_index)),
                    )
                    .await;
                }
            });
        }
    }

    /// Updates the installed mods of a given game instance.
    async fn update_mods(self, game_index: usize, prog_tx: status::AsyncProgressChannel) {
        if self
            .assert_instance_status(InstanceStatus::Running, prog_tx.clone())
            .await
        {
            task::spawn(async move {
                let mut games = self.games.lock().await;
                let game = games.get_mut(game_index);
                if let Some(game) = game {
                    if let Err(e) = game.mods.update(Some(prog_tx.clone())).await {
                        error!("Failed to update mods: {}", e);
                        send_status(
                            &prog_tx,
                            status::internal_error(&format!("Failed to update mods: {}", e)),
                        )
                        .await;
                        return;
                    }

                    send_status(&prog_tx, status::indefinite("Done")).await;
                } else {
                    send_status(
                        &prog_tx,
                        status::invalid_argument(&format!("No such game index: {}", game_index)),
                    )
                    .await;
                }
            });
        }
    }

    /// Updates the installed mods of a given game instance.
    async fn ensure_mod_dependencies(self, game_index: usize, prog_tx: status::AsyncProgressChannel) {
        if self
            .assert_instance_status(InstanceStatus::Running, prog_tx.clone())
            .await
        {
            task::spawn(async move {
                let mut games = self.games.lock().await;
                let game = games.get_mut(game_index);
                if let Some(game) = game {
                    if let Err(e) = game.mods.ensure_dependencies(Some(prog_tx.clone())).await {
                        error!("Failed to ensure mod dependencies: {}", e);
                        send_status(
                            &prog_tx,
                            status::internal_error(&format!("Failed to ensure mod dependencies: {}", e)),
                        )
                        .await;
                        return;
                    }

                    send_status(&prog_tx, status::indefinite("Done")).await;
                } else {
                    send_status(
                        &prog_tx,
                        status::invalid_argument(&format!("No such game index: {}", game_index)),
                    )
                    .await;
                }
            });
        }
    }
}

#[tonic::async_trait]
impl ModRpc for Modtorio {
    type ImportGameStream = mpsc::Receiver<Result<Progress, Status>>;
    type UpdateCacheStream = mpsc::Receiver<Result<Progress, Status>>;
    type InstallModStream = mpsc::Receiver<Result<Progress, Status>>;
    type UpdateModsStream = mpsc::Receiver<Result<Progress, Status>>;
    type EnsureModDependenciesStream = mpsc::Receiver<Result<Progress, Status>>;

    async fn get_version_information(&self, req: Request<Empty>) -> Result<Response<VersionInformation>, Status> {
        log_rpc_request(&req);

        let resp = Response::new(VersionInformation {
            version: Some((*HVER_VERSION).into()),
            protocol_version: Some(
                rpc::VERSION
                    .parse::<HumanVersion>()
                    .expect("failed to parse RPC protocol buffer specification version as HumanVersion")
                    .into(),
            ),
        });
        log_rpc_response(&resp);

        Ok(resp)
    }

    async fn get_server_status(&self, req: Request<Empty>) -> Result<Response<ServerStatus>, Status> {
        log_rpc_request(&req);

        let uptime = self.get_uptime().await;
        let games = self.get_rpc_games().await;
        let instance_status = self.get_instance_status().await;

        let resp = Response::new(ServerStatus {
            uptime: uptime.num_seconds(),
            games,
            instance_status: instance_status.into(),
        });
        log_rpc_response(&resp);

        Ok(resp)
    }

    // I tried to macro these repetitive functions into DRYness but the tonic::async_trait macro messes with them in
    // some funky way that a macro_rules! didn't work
    async fn import_game(&self, req: Request<ImportRequest>) -> Result<Response<Self::ImportGameStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        self.clone().import_game(req.into_inner().path, tx).await;
        let resp = Response::new(rx);
        log_rpc_response(&resp);

        Ok(resp)
    }

    async fn update_cache(
        &self,
        req: Request<UpdateCacheRequest>,
    ) -> Result<Response<Self::UpdateCacheStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        self.clone().update_cache(msg.game_index as usize, tx).await;
        let resp = Response::new(rx);
        log_rpc_response(&resp);

        Ok(resp)
    }

    async fn install_mod(&self, req: Request<InstallModRequest>) -> Result<Response<Self::InstallModStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        let version = msg.mod_version.map(HumanVersion::from);
        self.clone()
            .install_mod(msg.game_index as usize, msg.mod_name, version, tx)
            .await;
        let resp = Response::new(rx);
        log_rpc_response(&resp);

        Ok(resp)
    }

    async fn update_mods(&self, req: Request<UpdateModsRequest>) -> Result<Response<Self::UpdateModsStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        self.clone().update_mods(msg.game_index as usize, tx).await;
        let resp = Response::new(rx);
        log_rpc_response(&resp);

        Ok(resp)
    }

    async fn ensure_mod_dependencies(
        &self,
        req: Request<EnsureModDependenciesRequest>,
    ) -> Result<Response<Self::EnsureModDependenciesStream>, Status> {
        log_rpc_request(&req);
        let (tx, rx) = channel();

        let msg = req.into_inner();
        self.clone().ensure_mod_dependencies(msg.game_index as usize, tx).await;
        let resp = Response::new(rx);
        log_rpc_response(&resp);

        Ok(resp)
    }
}

/// Creates a new bounded channel and returns the receiver and sender, the sender wrapped in an
/// Arc<Mutex>.
fn channel<T>() -> (Arc<Mutex<mpsc::Sender<T>>>, mpsc::Receiver<T>) {
    let (tx, rx) = mpsc::channel(64);
    (Arc::new(Mutex::new(tx)), rx)
}

/// Logs a given RPC request.
fn log_rpc_request<T: std::fmt::Debug>(request: &Request<T>) {
    info!(
        "Got an RPC request from '{}'",
        request
            .remote_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| String::from("unknown"))
    );
    debug!("{:?}", request);
}

/// Logs a given RPC response.
fn log_rpc_response<T: std::fmt::Debug>(response: &Response<T>) {
    debug!("{:?}", response);
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
