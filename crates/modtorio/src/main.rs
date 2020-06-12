#![feature(drain_filter)]
#![feature(async_closure)]

mod cache;
mod config;
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    let config = Arc::new(Config::from_env()?);

    log::setup_logging(&config)?;
    config.debug_values();

    let cache = Arc::new(cache::CacheBuilder::new().build()?);
    let portal = Arc::new(ModPortal::new(&config)?);

    // let factorio = Arc::new(
    //     factorio::Importer::from_root("./sample")
    //         .import(config, portal, cache)
    //         .await?,
    // );

    let cached_games = cache.get_game_ids().await?;
    let mut games = Vec::new();

    for id in cached_games {
        let game = factorio::Importer::from_cache(id)
            .import(Arc::clone(&config), Arc::clone(&portal), Arc::clone(&cache))
            .await?;

        games.push(game);
        debug!("Cached game id {} imported", id);
    }

    if games.is_empty() {
        info!("No cached games found, importing from /sample...");

        games.push(
            factorio::Importer::from_root("./sample")
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
