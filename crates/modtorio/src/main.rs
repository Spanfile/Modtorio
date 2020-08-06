#![feature(drain_filter)]
#![feature(async_closure)]
#![warn(clippy::if_not_else)]
#![warn(clippy::needless_pass_by_value)]
// #![warn(clippy::pedantic)]

mod config;
mod error;
mod ext;
mod factorio;
mod log;
mod mod_common;
mod mod_portal;
mod opts;
mod store;
mod util;

use ::log::*;
use config::Config;
use mod_portal::ModPortal;
use opts::Opts;
use std::{fs::File, path::Path, sync::Arc};
use store::Store;

/// Location of the sample server used during development.
const SAMPLE_GAME_DIRECTORY: &str = "./sample";
/// The prefix used with every environment value related to the program configuration.
pub const APP_PREFIX: &str = "MODTORIO_";
const PORTAL_USERNAME_ENV_VARIABLE: &str = "MODTORIO_PORTAL_USERNAME";
const PORTAL_TOKEN_ENV_VARIABLE: &str = "MODTORIO_PORTAL_TOKEN";

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::get();
    let store = Arc::new(Store::build((&opts.store).into()).await?);
    let config = Arc::new(build_config(&opts, &store).await?);

    log::setup_logging(&config)?;
    // config.debug_values();

    debug!("{:?}", opts);
    debug!("Env {:?}", util::env::dump_lines(APP_PREFIX));
    debug!("{:?}", config);

    if !opts.no_env {
        update_store_from_env(&store).await?;
    }
    log_program_information();

    return Ok(());

    let portal = Arc::new(ModPortal::new(&config)?);

    // let factorio = Arc::new(
    //     factorio::Importer::from_root("./sample")
    //         .import(config, portal, cache)
    //         .await?,
    // );

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

    if games.is_empty() {
        info!(
            "No cached games found, importing from {}...",
            SAMPLE_GAME_DIRECTORY
        );

        games.push(
            factorio::Importer::from_root(SAMPLE_GAME_DIRECTORY)
                .import(Arc::clone(&config), Arc::clone(&portal), Arc::clone(&store))
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

async fn build_config(opts: &Opts, store: &Store) -> anyhow::Result<Config> {
    if !opts.config.exists() {
        create_default_config_file(&opts.config)?;
    }

    let mut builder = config::Builder::new()
        .apply_config_file(&mut File::open(&opts.config)?)?
        .apply_store(store)
        .await?;

    if !opts.no_env {
        builder = builder.apply_env()?;
    }

    Ok(builder.build())
}

fn create_default_config_file<P>(path: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    Config::write_default_config_to_writer(&mut File::create(path)?)
}

async fn update_store_from_env(store: &Store) -> anyhow::Result<()> {
    for (key, value) in util::env::dump_map(APP_PREFIX) {
        match key.as_ref() {
            PORTAL_USERNAME_ENV_VARIABLE => {
                debug!("Got portal username env variable, updating store");
                store
                    .set_option(store::option::Value {
                        field: store::option::Field::PortalUsername,
                        value: Some(value),
                    })
                    .await?
            }
            PORTAL_TOKEN_ENV_VARIABLE => {
                debug!("Got portal token env variable, updating store");
                store
                    .set_option(store::option::Value {
                        field: store::option::Field::PortalToken,
                        value: Some(value),
                    })
                    .await?
            }
            _ => {}
        }
    }

    Ok(())
}

fn log_program_information() {
    info!("Program version: {}", VERSION);
}
