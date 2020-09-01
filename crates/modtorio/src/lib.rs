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
mod rpc_handler;
pub mod store;
mod unix;
pub mod util;

use ::log::*;
use anyhow::Context;
use chrono::{DateTime, Utc};
use common::net::NetAddress;
use config::Config;
use error::RpcError;
use factorio::{Factorio, GameStoreId, ServerStatus};
use futures::{
    future::{join_all, try_join_all},
    TryStreamExt,
};
use lazy_static::lazy_static;
use mod_portal::ModPortal;
use rpc::{instance_status, mod_rpc_server, server_control_action_request};
use rpc_handler::RpcHandler;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use store::Store;
use tokio::{
    fs,
    net::UnixListener,
    sync::{mpsc, watch, Mutex},
    task,
};
use tonic::{transport::Server, Request, Response, Status};
use util::{
    async_status,
    async_status::{AsyncProgressChannel, AsyncProgressChannelExt, AsyncProgressResult},
    ext::StrExt,
    HumanVersion,
};

/// The prefix used with every environment value related to the program configuration.
pub const APP_PREFIX: &str = "MODTORIO_";
/// The program's version at build-time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The result type used in RPC processor functions.
type RpcResult<T> = Result<T, RpcError>;

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
    // TODO: rename to 'servers'
    /// Collection of Factorio instances this Modtorio instance is managing.
    servers: Arc<Mutex<Vec<Factorio>>>,
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
            servers: Arc::new(Mutex::new(Vec::new())),
            started_at: Arc::new(Utc::now()),
            status: Arc::new(Mutex::new(instance_status::Status::Starting)),
        };

        // run the stored server importing and server autostarting in the background to allow running the RPC listeners
        // as soon as possible
        let i = instance.clone();
        task::spawn(async move {
            i.import_stored_games().await;
            i.autostart_servers().await;
        });

        Ok(instance)
    }

    /// Runs a given Modtorio instance.
    pub async fn run(self) -> anyhow::Result<()> {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(());
        shutdown_rx.recv().await;

        task::spawn(async move {
            term_signal().await;
            debug!("SIGINT caught, sending shutdown signal");
            info!("Shutting down");
            shutdown_tx.broadcast(()).expect("failed to broadcast shutdown signal");
        });

        let result = if let Err(e) = self.run_rpc(shutdown_rx).await {
            error!("RPC server failed with: {}", e);
            Err(e)
        } else {
            Ok(())
        };

        self.wait_for_servers_to_shutdown().await?;
        self.update_all_server_stores().await?;
        result
    }

    /// Imports all stored servers.
    async fn import_stored_games(&self) {
        info!("Loading previous servers...");
        let stored_games = match self.store.get_games().await {
            Ok(servers) => servers,
            Err(e) => {
                error!("Failed to get stored servers: {}", e);
                return;
            }
        };
        let mut servers = Vec::new();
        debug!("Got stored servers: {:?}", stored_games);

        for stored_game in &stored_games {
            info!(
                "Importing stored server ID {} from path {}...",
                stored_game.id, stored_game.path
            );

            let server = match factorio::Importer::from_store(stored_game)
                .import(
                    Arc::clone(&self.config),
                    Arc::clone(&self.portal),
                    Arc::clone(&self.store),
                )
                .await
            {
                Ok(server) => server,
                Err(e) => {
                    error!("Failed to import stored server ID {}: {}", stored_game.id, e);
                    continue;
                }
            };

            info!(
                "Stored server ID {} imported from {}. {} mods",
                stored_game.id,
                stored_game.path,
                server.mods().count()
            );
            debug!("Stored server: {:?}", stored_game);
            servers.push(server);
        }

        info!("{} previous servers loaded.", servers.len());
        self.servers.lock().await.extend(servers);
        *self.status.lock().await = instance_status::Status::Running;
    }

    /// Start all servers that are set to be automatically started.
    async fn autostart_servers(&self) {
        trace!("Autostarting servers...");

        let servers = self.servers.lock().await;
        for server in servers.iter() {
            if server.settings().running.auto {
                let store_id = server
                    .store_id()
                    .await
                    .expect("imported server doesn't have store ID set");
                info!("Autostarting server ID {}...", store_id);

                match server.run().await {
                    Ok(_) => {
                        debug!("Game ID {} autostarted", store_id);
                    }
                    Err(e) => {
                        error!("Game ID {} failed to autostart: {}", store_id, e);
                    }
                }
            }
        }
    }

    /// Updates the program store for all the managed servers.
    async fn update_all_server_stores(&self) -> anyhow::Result<()> {
        let servers = self.servers.lock().await;
        for server in servers.iter() {
            server.update_store(true, None).await?;
        }
        Ok(())
    }

    /// Runs the RPC server.
    async fn run_rpc(&self, shutdown_rx: watch::Receiver<()>) -> anyhow::Result<()> {
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
                    let shutdown_signal = wait_for_signal(shutdown_rx.clone());
                    task::spawn(async move {
                        server
                            .serve_with_shutdown(addr, shutdown_signal)
                            .await
                            .expect("RPC TCP listener failed");
                        debug!("RPC TCP listener on {} shut down", addr);
                    })
                }
                NetAddress::Unix(path) => {
                    debug!("Starting RPC server on Unix {}", path.display());

                    let path = path.to_owned();
                    let shutdown_signal = wait_for_signal(shutdown_rx.clone());
                    task::spawn(async move {
                        let mut unix = UnixListener::bind(&path).expect("failed to bind to unix socket");
                        server
                            .serve_with_incoming_shutdown(unix.incoming().map_ok(unix::UnixStream), shutdown_signal)
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

    /// Waits for all the currently managed servers to be shut down.
    async fn wait_for_servers_to_shutdown(&self) -> anyhow::Result<()> {
        let servers = self.servers.lock().await;
        let mut waiters = Vec::new();

        for server in servers.iter() {
            debug!("Waiting for server ID {} to shut down...", server.store_id().await?);
            waiters.push(server.wait_for_shutdown());
        }

        join_all(waiters).await;
        Ok(())
    }

    /// Returns a boolean on whether this instance manages a server identified by its root path.
    async fn server_exists_by_path<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        self.servers
            .lock()
            .await
            .iter()
            .any(|server| util::file::are_same(server.root(), path.as_ref()).expect("failed to compare file paths"))
    }

    /// Returns this instance's uptime.
    async fn get_uptime(&self) -> chrono::Duration {
        Utc::now() - *self.started_at
    }

    /// Returns this instance's managed servers in RPC format.
    async fn get_rpc_servers(&self) -> Vec<instance_status::Server> {
        let mut rpc_games = Vec::new();

        for server in self.servers.lock().await.iter() {
            let status = server.status().await.game_status() as i32;
            let server_id = server.store_id_option().await.unwrap_or(0);

            rpc_games.push(instance_status::Server {
                path: format!("{}", server.root().display()),
                status,
                server_id,
            });
        }

        rpc_games
    }

    /// Returns this instance's status.
    async fn get_instance_status(&self) -> instance_status::Status {
        *self.status.lock().await
    }

    /// Returns this instance's status as an RPC response.
    async fn get_rpc_instance_status(&self, _req: rpc::Empty) -> RpcResult<rpc::InstanceStatus> {
        let uptime = self.get_uptime().await.num_seconds();
        let servers = self.get_rpc_servers().await;
        let instance_status = self.get_instance_status().await;

        Ok(rpc::InstanceStatus {
            uptime,
            servers,
            instance_status: instance_status as i32,
        })
    }

    /// Imports a new Factorio instance from a given path to its root directory.
    async fn import_game(self, msg: rpc::ImportRequest, prog_tx: AsyncProgressChannel) {
        task::spawn(async move {
            let result: anyhow::Result<()> = async {
                if self.server_exists_by_path(&msg.path).await {
                    return Err(RpcError::GameAlreadyExists(PathBuf::from(&msg.path)).into());
                }

                let mut importer = factorio::Importer::from_root(&msg.path)?;

                if let Some(path) = msg.settings.map_to_option() {
                    importer = importer.with_server_settings(path);
                }

                if let Some(path) = msg.whitelist.map_to_option() {
                    importer = importer.with_whitelist(path);
                }

                if let Some(path) = msg.adminlist.map_to_option() {
                    importer = importer.with_adminlist(path);
                }

                if let Some(path) = msg.banlist.map_to_option() {
                    importer = importer.with_banlist(path);
                }

                let server = importer
                    .with_status_updates(prog_tx.clone())
                    .import(
                        Arc::clone(&self.config),
                        Arc::clone(&self.portal),
                        Arc::clone(&self.store),
                    )
                    .await?;

                server.update_store(false, Some(prog_tx.clone())).await?;
                self.servers.lock().await.push(server);
                Ok(())
            }
            .await;
            rpc_handle_result(result.context("Failed to import server"), &prog_tx).await;
        });
    }

    /// Updates a given server instance's store.
    async fn update_store(self, msg: rpc::UpdateStoreRequest, prog_tx: AsyncProgressChannel) {
        task::spawn(async move {
            let result: anyhow::Result<()> = async {
                let mut servers = self.servers.lock().await;
                let server = find_server_mut(msg.server_id, &mut servers).await?;
                server.update_store(msg.skip_info_update, Some(prog_tx.clone())).await?;
                Ok(())
            }
            .await;
            rpc_handle_result(result, &prog_tx).await;
        });
    }

    /// Installs a mod to a given server instance.
    async fn install_mod(self, msg: rpc::InstallModRequest, prog_tx: AsyncProgressChannel) {
        task::spawn(async move {
            let result: anyhow::Result<()> = async {
                let mut servers = self.servers.lock().await;
                let server = find_server_mut(msg.server_id, &mut servers).await?;
                server
                    .mods_mut()
                    .add_from_portal(
                        &msg.mod_name,
                        msg.mod_version.map(HumanVersion::from),
                        Some(prog_tx.clone()),
                    )
                    .await?;

                Ok(())
            }
            .await;
            rpc_handle_result(result.context("Failed to install mod"), &prog_tx).await;
        });
    }

    /// Removes a mod from a given server instance.
    async fn remove_mod(&self, msg: rpc::RemoveModRequest) -> RpcResult<()> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(msg.server_id, &mut servers).await?;
        server.mods_mut().remove(&msg.mod_name).await?;
        Ok(())
    }

    /// Enables a mod for a given server instance.
    async fn enable_mod(&self, msg: rpc::EnableModRequest) -> RpcResult<()> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(msg.server_id, &mut servers).await?;
        server.mods().set_mod_enabled(&msg.mod_name, true)?;
        Ok(())
    }

    /// Disables a mod for a given server instance.
    async fn disable_mod(&self, msg: rpc::DisableModRequest) -> RpcResult<()> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(msg.server_id, &mut servers).await?;
        server.mods().set_mod_enabled(&msg.mod_name, false)?;
        Ok(())
    }

    /// Returns whether a mod is enabled or not.
    async fn get_mod_enabled(&self, msg: rpc::GetModEnabledRequest) -> RpcResult<rpc::ModEnabled> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(msg.server_id, &mut servers).await?;
        let enabled = server.mods().get_mod_enabled(&msg.mod_name)?;
        Ok(rpc::ModEnabled { enabled })
    }

    /// Returns the enabled status for all mods.
    async fn get_mods_enabled_status(
        &self,
        msg: rpc::GetModsEnabledStatusRequest,
    ) -> RpcResult<rpc::ModsEnabledStatus> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(msg.server_id, &mut servers).await?;
        let mods = server.mods().get_mods_enabled_status()?;
        Ok(rpc::ModsEnabledStatus {
            mods: mods
                .into_iter()
                .map(|(name, enabled)| rpc::mods_enabled_status::ModEnabled { name, enabled })
                .collect(),
        })
    }

    /// Updates the installed mods of a given server instance.
    async fn update_mods(self, msg: rpc::UpdateModsRequest, prog_tx: AsyncProgressChannel) {
        task::spawn(async move {
            let result: anyhow::Result<()> = async {
                let mut servers = self.servers.lock().await;
                let server = find_server_mut(msg.server_id, &mut servers).await?;
                server.mods_mut().update(Some(prog_tx.clone())).await?;
                Ok(())
            }
            .await;
            rpc_handle_result(result.context("Failed to update mods"), &prog_tx).await;
        });
    }

    /// Updates the installed mods of a given server instance.
    async fn ensure_mod_dependencies(self, msg: rpc::EnsureModDependenciesRequest, prog_tx: AsyncProgressChannel) {
        task::spawn(async move {
            let result: anyhow::Result<()> = async {
                let mut servers = self.servers.lock().await;
                let server = find_server_mut(msg.server_id, &mut servers).await?;
                server.mods_mut().ensure_dependencies(Some(prog_tx.clone())).await?;
                Ok(())
            }
            .await;
            rpc_handle_result(result.context("Failed to ensure mod dependencies"), &prog_tx).await;
        });
    }

    /// Retrieves a given server instance's server settings.
    async fn get_server_settings(&self, msg: rpc::GetServerSettingsRequest) -> RpcResult<rpc::ServerSettings> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(msg.server_id, &mut servers).await?;

        let mut rpc_server_settings = rpc::ServerSettings::default();
        server.settings().to_rpc_format(&mut rpc_server_settings);

        Ok(rpc_server_settings)
    }

    /// Sets a given server instance's server settings.
    async fn set_server_settings(&self, msg: rpc::SetServerSettingsRequest) -> RpcResult<()> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(msg.server_id, &mut servers).await?;

        if let Some(settings) = msg.settings {
            info!("Updating server ID {}'s settings", msg.server_id);
            server.settings_mut().modify_self_with_rpc(&settings)?;
        } else {
            info!("Resetting server ID {}'s settings to default", msg.server_id);
            *server.settings_mut() = Default::default();
        };

        Ok(())
    }

    /// Executes a server control action on a server.
    async fn execute_server_control_action(self, msg: rpc::ServerControlActionRequest, prog_tx: AsyncProgressChannel) {
        use server_control_action_request::{Action, Restart, Stop};

        task::spawn(async move {
            let result: anyhow::Result<()> = async {
                let mut servers = self.servers.lock().await;
                let server = find_server_mut(msg.server_id, &mut servers).await?;

                match msg.action {
                    Some(Action::Start { .. }) => {
                        info!("Starting server ID {}", msg.server_id);
                        server.run().await?;
                    }
                    Some(Action::Kill { .. }) => {
                        info!("Killing server ID {}", msg.server_id);
                        server.kill().await?;
                    }
                    Some(Action::Stop(Stop { timeout_override })) => {
                        info!("Gracefully shutting down server ID {}", msg.server_id);
                        send_status(
                            &prog_tx,
                            async_status::indefinite("Waiting for the server to be empty..."),
                        )
                        .await;

                        let timeout_override = if timeout_override == 0 {
                            None
                        } else {
                            debug!("Using timeout override: {}", timeout_override);
                            Some(timeout_override)
                        };

                        server.graceful_shutdown(timeout_override).await?;
                    }
                    Some(Action::Restart(Restart { timeout_override })) => {
                        info!("Gracefully restarting server ID {}", msg.server_id);
                        send_status(
                            &prog_tx,
                            async_status::indefinite("Waiting for the server to be empty..."),
                        )
                        .await;

                        let timeout_override = if timeout_override == 0 {
                            None
                        } else {
                            debug!("Using timeout override: {}", timeout_override);
                            Some(timeout_override)
                        };

                        server.graceful_restart(timeout_override).await?;
                    }
                    Some(Action::ForceRestart { .. }) => {
                        info!("Forcefully restarting server ID {}", msg.server_id);
                        server.force_restart().await?;
                    }
                    None => return Err(RpcError::MissingArgument.into()),
                }

                Ok(())
            }
            .await;
            rpc_handle_result(result.context("Failed to execute server control action"), &prog_tx).await;
        });
    }

    /// Sends a command to a given server instance.
    async fn send_server_command(&self, msg: rpc::SendCommandRequest) -> RpcResult<()> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(msg.server_id, &mut servers).await?;
        let command = msg.command.ok_or(RpcError::MissingArgument)?;
        server.send_command(command.into()).await?;

        Ok(())
    }

    /// Returns a given server's status.
    async fn get_server_status(&self, server_id: GameStoreId) -> RpcResult<ServerStatus> {
        let mut servers = self.servers.lock().await;
        let server = find_server_mut(server_id, &mut servers).await?;

        Ok(server.status().await)
    }

    /// Returns a given server's status.
    async fn get_rpc_server_status(&self, msg: rpc::ServerStatusRequest) -> RpcResult<rpc::ServerStatus> {
        let status = self.get_server_status(msg.server_id).await?;
        Ok(status.to_rpc_server_status().await)
    }

    /// Returns a new `RpcHandler` with `self` and a given RPC request.
    fn rpc_handler<'a, T>(&'a self, req: Request<T>) -> RpcHandler<'a, T>
    where
        T: std::fmt::Debug,
    {
        RpcHandler::new(self, req)
    }
}

#[tonic::async_trait]
impl mod_rpc_server::ModRpc for Modtorio {
    type ImportGameStream = mpsc::Receiver<Result<rpc::Progress, Status>>;
    type UpdateStoreStream = mpsc::Receiver<Result<rpc::Progress, Status>>;
    type InstallModStream = mpsc::Receiver<Result<rpc::Progress, Status>>;
    type UpdateModsStream = mpsc::Receiver<Result<rpc::Progress, Status>>;
    type EnsureModDependenciesStream = mpsc::Receiver<Result<rpc::Progress, Status>>;

    type ServerControlActionStream = mpsc::Receiver<Result<rpc::Progress, Status>>;

    async fn get_version_information(
        &self,
        req: Request<rpc::Empty>,
    ) -> Result<Response<rpc::VersionInformation>, Status> {
        self.rpc_handler(req)
            .respond(rpc::VersionInformation {
                version: Some((*HVER_VERSION).into()),
                protocol_version: Some(
                    rpc::VERSION
                        .parse::<HumanVersion>()
                        .expect("failed to parse RPC protocol buffer specification version as HumanVersion")
                        .into(),
                ),
            })
            .await
    }

    async fn get_instance_status(&self, req: Request<rpc::Empty>) -> Result<Response<rpc::InstanceStatus>, Status> {
        self.rpc_handler(req).result(Self::get_rpc_instance_status).await
    }

    async fn import_game(&self, req: Request<rpc::ImportRequest>) -> Result<Response<Self::ImportGameStream>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .stream(Self::import_game)
            .await
    }

    async fn update_store(
        &self,
        req: Request<rpc::UpdateStoreRequest>,
    ) -> Result<Response<Self::UpdateStoreStream>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .stream(Self::update_store)
            .await
    }

    async fn install_mod(
        &self,
        req: Request<rpc::InstallModRequest>,
    ) -> Result<Response<Self::InstallModStream>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .stream(Self::install_mod)
            .await
    }

    async fn remove_mod(&self, req: Request<rpc::RemoveModRequest>) -> Result<Response<rpc::Empty>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::remove_mod)
            .await
    }

    async fn enable_mod(&self, req: Request<rpc::EnableModRequest>) -> Result<Response<rpc::Empty>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::enable_mod)
            .await
    }

    async fn disable_mod(&self, req: Request<rpc::DisableModRequest>) -> Result<Response<rpc::Empty>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::disable_mod)
            .await
    }

    async fn get_mod_enabled(
        &self,
        req: Request<rpc::GetModEnabledRequest>,
    ) -> Result<Response<rpc::ModEnabled>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::get_mod_enabled)
            .await
    }

    async fn get_mods_enabled_status(
        &self,
        req: Request<rpc::GetModsEnabledStatusRequest>,
    ) -> Result<Response<rpc::ModsEnabledStatus>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::get_mods_enabled_status)
            .await
    }

    async fn update_mods(
        &self,
        req: Request<rpc::UpdateModsRequest>,
    ) -> Result<Response<Self::UpdateModsStream>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .stream(Self::update_mods)
            .await
    }

    async fn ensure_mod_dependencies(
        &self,
        req: Request<rpc::EnsureModDependenciesRequest>,
    ) -> Result<Response<Self::EnsureModDependenciesStream>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .stream(Self::ensure_mod_dependencies)
            .await
    }

    async fn get_server_settings(
        &self,
        req: Request<rpc::GetServerSettingsRequest>,
    ) -> Result<Response<rpc::ServerSettings>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::get_server_settings)
            .await
    }

    async fn set_server_settings(
        &self,
        req: Request<rpc::SetServerSettingsRequest>,
    ) -> Result<Response<rpc::Empty>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::set_server_settings)
            .await
    }

    async fn server_control_action(
        &self,
        req: Request<rpc::ServerControlActionRequest>,
    ) -> Result<Response<Self::ServerControlActionStream>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .stream(Self::execute_server_control_action)
            .await
    }

    async fn send_server_command(&self, req: Request<rpc::SendCommandRequest>) -> Result<Response<rpc::Empty>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::send_server_command)
            .await
    }

    async fn get_server_status(
        &self,
        req: Request<rpc::ServerStatusRequest>,
    ) -> Result<Response<rpc::ServerStatus>, Status> {
        self.rpc_handler(req)
            .require_status(instance_status::Status::Running)
            .result(Self::get_rpc_server_status)
            .await
    }
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

/// Asynchronously returns the unit type after a given `watch::Receiver` receives a value.
async fn wait_for_signal<T: Clone>(mut signal: watch::Receiver<T>) {
    signal.recv().await;
}

/// Finds and returns a mutable reference to a server based on its store ID, or returns `RpcError::NoSuchGame` if the
/// server isn't found.
async fn find_server_mut(server_id: GameStoreId, servers: &mut Vec<Factorio>) -> RpcResult<&mut Factorio> {
    for g in servers.iter_mut() {
        if let Some(id) = g.store_id_option().await {
            if id == server_id {
                return Ok(g);
            }
        }
    }

    Err(RpcError::NoSuchServer(server_id))
}

/// Given a `Result` and an `AsyncProgressChannel`, logs and sends the result to the channel.
async fn rpc_handle_result(result: anyhow::Result<()>, prog_tx: &AsyncProgressChannel) {
    if let Err(e) = result {
        // TODO: this doesn't seem to print the whole error message, only the first context
        error!("RPC: {}", e);
        send_error_status(prog_tx, e).await;
    } else {
        trace!("RPC task completed");
        send_status(prog_tx, async_status::done()).await;
    }
}
