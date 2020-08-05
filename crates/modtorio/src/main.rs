#![feature(drain_filter)]
#![feature(async_closure)]
#![warn(clippy::if_not_else)]
#![warn(clippy::needless_pass_by_value)]
// #![warn(clippy::pedantic)]

mod cache;
mod config;
mod error;
mod ext;
mod factorio;
mod log;
mod mod_common;
mod mod_portal;
mod opts;
mod util;

use ::log::*;
use cache::Cache;
use config::Config;
use mod_portal::ModPortal;
use opts::Opts;
use std::sync::Arc;

/// Location of the sample server used during development.
const SAMPLE_GAME_DIRECTORY: &str = "./sample";

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::get();
    let config = Arc::new(Config::build(&opts)?);

    log::setup_logging(&config)?;
    // config.debug_values();

    debug!("{:?}", opts);
    debug!("{:?}", util::dump_env_lines(config::APP_PREFIX));
    debug!("{:?}", config);

    log_program_information();

    return Ok(());

    let cache = Arc::new(cache::Builder::new().build().await?);
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

        info!(
            "Cached game ID {} imported from {}. {} mods",
            cached_game.id,
            cached_game.path,
            game.mods.count()
        );
        debug!("Cached game: {:?}", cached_game);
        games.push(game);
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

    for factorio in &mut games {
        // factorio.mods.add_from_portal("FARL", None).await?;

        // factorio.mods.update().await?;
        // factorio.mods.ensure_dependencies().await?;
        factorio.update_cache().await?;
    }

    Ok(())
}

fn log_program_information() {
    info!("Program version: {}", VERSION);
}
