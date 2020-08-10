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
use factorio::Factorio;
use mod_portal::ModPortal;
use rpc::{
    mod_rpc_server::{ModRpc, ModRpcServer},
    server_status::{game::GameStatus, Game},
    Empty, ImportRequest, Progress, ServerStatus, UpdateCacheRequest,
};
use std::{path::Path, sync::Arc};
use store::Store;
use tokio::{
    sync::{mpsc, Mutex},
    task,
};
use tonic::{transport::Server, Request, Response, Status};
use util::status;

/// The prefix used with every environment value related to the program configuration.
pub const APP_PREFIX: &str = "MODTORIO_";

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
}

impl Modtorio {
    /// Creates a new Modtorio instance with a given configuration object and a program store
    /// object.
    pub async fn new(config: Config, store: Store) -> anyhow::Result<Self> {
        let config = Arc::new(config);
        let store = Arc::new(store);

        let portal = Arc::new(ModPortal::new(&config)?);

        info!("Loading previous games...");
        let cached_games = store.cache.get_games().await?;
        let mut games = Vec::new();
        debug!("Got cached games: {:?}", cached_games);

        for cached_game in &cached_games {
            info!(
                "Importing cached game ID {} from path {}...",
                cached_game.id, cached_game.path
            );

            let game = factorio::Importer::from_cache(cached_game)
                .import(Arc::clone(&config), Arc::clone(&portal), Arc::clone(&store))
                .await?;

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

        Ok(Modtorio {
            config,
            portal,
            store,
            games: Arc::new(Mutex::new(games)),
            started_at: Arc::new(Utc::now()),
        })
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

    /// Imports a new Factorio instance from a given path to its root directory.
    async fn import_game<P>(self, path: P, prog_tx: status::AsyncProgressChannel)
    where
        P: AsRef<Path>,
    {
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
                    info!(
                        "Imported new Factorio server instance from {}",
                        path.display()
                    );
                    if let Err(e) = status::send_status(
                        Some(prog_tx.clone()),
                        status::indefinite("Game imported"),
                    )
                    .await
                    {
                        error!("Failed to send status update: {}", e);
                        return;
                    }
                    game
                }
                Err(e) => {
                    error!("Failed to import game: {}", e);
                    if let Err(nested) =
                        status::send_status(Some(prog_tx.clone()), status::error(&e.to_string()))
                            .await
                    {
                        error!("Failed to send status update: {}", nested);
                        return;
                    }
                    return;
                }
            };

            if let Err(e) = game.update_cache(Some(prog_tx.clone())).await {
                error!("Failed to update game cache: {}", e);
                if let Err(e) = status::send_status(
                    Some(prog_tx.clone()),
                    status::error("Failed to update game cache"),
                )
                .await
                {
                    error!("Failed to send status update: {}", e);
                    return;
                }
            }

            if let Err(e) =
                status::send_status(Some(prog_tx.clone()), status::indefinite("Done")).await
            {
                error!("Failed to send status update: {}", e);
                return;
            }
        });
    }

    /// Updates a given game instance's cache.
    async fn update_cache(self, server_index: usize, prog_tx: status::AsyncProgressChannel) {
        task::spawn(async move {
            let games = self.games.lock().await;
            let game = games.get(server_index);
            if let Some(game) = game {
                if let Err(e) = game.update_cache(Some(prog_tx.clone())).await {
                    error!("Failed to update game cache: {}", e);
                    if let Err(e) = status::send_status(
                        Some(prog_tx.clone()),
                        status::error("Failed to update game cache"),
                    )
                    .await
                    {
                        error!("Failed to send status update: {}", e);
                        return;
                    }
                }

                if let Err(e) =
                    status::send_status(Some(prog_tx.clone()), status::indefinite("Done")).await
                {
                    error!("Failed to send status update: {}", e);
                    return;
                }
            } else if let Err(e) = status::send_status(
                Some(prog_tx.clone()),
                status::error(&format!("No such game index: {}", server_index)),
            )
            .await
            {
                error!("Failed to send status update: {}", e);
                return;
            }
        });
    }
}

#[tonic::async_trait]
impl ModRpc for Modtorio {
    type ImportGameStream = mpsc::Receiver<Result<Progress, Status>>;
    type UpdateCacheStream = mpsc::Receiver<Result<Progress, Status>>;

    async fn get_server_status(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<ServerStatus>, Status> {
        log_rpc_request(&request);

        let uptime = self.get_uptime().await;
        let games = self.get_rpc_games().await;

        Ok(Response::new(ServerStatus {
            uptime: uptime.num_seconds(),
            games,
        }))
    }

    async fn import_game(
        &self,
        request: Request<ImportRequest>,
    ) -> Result<Response<Self::ImportGameStream>, Status> {
        log_rpc_request(&request);
        let (tx, rx) = channel();

        self.clone()
            .import_game(request.into_inner().path, tx)
            .await;
        Ok(Response::new(rx))
    }

    async fn update_cache(
        &self,
        request: Request<UpdateCacheRequest>,
    ) -> Result<Response<Self::UpdateCacheStream>, Status> {
        log_rpc_request(&request);
        let (tx, rx) = channel();

        self.clone()
            .update_cache(request.into_inner().server_index as usize, tx)
            .await;
        Ok(Response::new(rx))
    }
}

/// Creates a new bounded channel and returns the receiver and sender, the sender wrapped in an
/// Arc<Mutex>.
fn channel<T>() -> (Arc<Mutex<mpsc::Sender<T>>>, mpsc::Receiver<T>) {
    let (tx, rx) = mpsc::channel(8);
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
