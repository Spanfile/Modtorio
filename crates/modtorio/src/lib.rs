//! A wrapper for a headless Linux Factorio server to allow higher control over the server's
//! functionality.

#![feature(drain_filter)]
#![feature(async_closure)]
#![feature(associated_type_bounds)]
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
    Empty, ServerStatus,
};
use std::sync::Arc;
use store::Store;
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};

/// Location of the sample server used during development.
const SAMPLE_GAME_DIRECTORY: &str = "./sample";
/// The prefix used with every environment value related to the program configuration.
pub const APP_PREFIX: &str = "MODTORIO_";

/// A wrapper for a headless Linux Factorio server.
pub struct Modtorio {
    /// The program config.
    config: Arc<Config>,
    /// The program store.
    store: Arc<Store>,
    games: Arc<Mutex<Vec<Factorio>>>,
    started_at: DateTime<Utc>,
}

/// The RPC server implementation.
struct ModtorioRpc {
    modtorio: Arc<Modtorio>,
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
            store,
            games: Arc::new(Mutex::new(games)),
            started_at: Utc::now(),
        })
    }

    /// Runs a given Modtorio instance.
    pub async fn run(modtorio: Arc<Modtorio>) -> anyhow::Result<()> {
        let server_task = tokio::spawn(ModtorioRpc::run(Arc::clone(&modtorio)));

        if let Err(e) = tokio::try_join!(server_task) {
            error!("Async task failed with: {}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}

impl Modtorio {
    fn get_uptime(&self) -> chrono::Duration {
        Utc::now() - self.started_at
    }
}

#[tonic::async_trait]
impl ModRpc for ModtorioRpc {
    async fn get_server_status(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ServerStatus>, Status> {
        debug!("Got status request");

        let reply = ServerStatus {
            uptime: self.modtorio.get_uptime().num_seconds(),
        };

        Ok(Response::new(reply))
    }
}

impl ModtorioRpc {
    /// Runs the RPC server.
    async fn run(modtorio: Arc<Modtorio>) -> anyhow::Result<()> {
        let listen_addresses = modtorio.config.listen();

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
        let server = ModtorioRpc { modtorio };
        debug!("Starting RPC server. Listening on {}", addr);
        Server::builder()
            .add_service(ModRpcServer::new(server))
            .serve(addr)
            .await?;
        Ok(())
    }
}
