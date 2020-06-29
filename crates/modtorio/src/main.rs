#![feature(drain_filter)]
#![feature(async_closure)]

mod cache;
mod config;
mod error;
mod ext;
mod factorio;
mod log;
mod mod_common;
mod mod_portal;
mod util;

use ::log::*;
use cache::Cache;
use config::Config;
use mod_portal::ModPortal;
use std::sync::Arc;

const SAMPLE_GAME_DIRECTORY: &str = "./sample";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    let config = Arc::new(Config::from_env()?);

    log::setup_logging(&config)?;
    config.debug_values();

    let cache = Arc::new(cache::CacheBuilder::new().build().await?);
    let portal = Arc::new(ModPortal::new(&config)?);

    // let factorio = Arc::new(
    //     factorio::Importer::from_root("./sample")
    //         .import(config, portal, cache)
    //         .await?,
    // );

    let cached_games = cache.get_games().await?;
    let mut games = Vec::new();
    debug!("Got cached games: {:?}", cached_games);

    for cached_game in &cached_games {
        info!(
            "Importing cached game ID {} from path {}...",
            cached_game.id, cached_game.path
        );

        let game = factorio::Importer::from_cache(cached_game)
            .import(Arc::clone(&config), Arc::clone(&portal), Arc::clone(&cache))
            .await?;

        games.push(game);
        info!(
            "Cached game ID {} imported from {}",
            cached_game.id, cached_game.path
        );
        debug!("Cached game: {:?}", cached_game);
    }

    if games.is_empty() {
        info!(
            "No cached games found, importing from {}...",
            SAMPLE_GAME_DIRECTORY
        );

        games.push(
            factorio::Importer::from_root(SAMPLE_GAME_DIRECTORY)
                .import(Arc::clone(&config), Arc::clone(&portal), Arc::clone(&cache))
                .await?,
        );
    }

    // factorio
    //     .mods
    //     .add_from_portal("angelsindustries", None)
    //     .await?;

    for factorio in games.iter_mut() {
        // factorio.mods.update().await?;
        factorio.mods.ensure_dependencies().await?;
        factorio.update_cache().await?;
    }

    Ok(())
}
