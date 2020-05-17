#![feature(drain_filter)]
#![feature(async_closure)]

// diesel still requires this even if Rust 2018 has moved away from it
#[macro_use]
extern crate diesel;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    let config = Config::from_env()?;

    log::setup_logging(&config)?;
    config.debug_values();

    let cache = cache::CacheBuilder::new().build()?;
    let portal = ModPortal::new(&config)?;

    let mut factorio = factorio::Importer::from_root("./sample")
        .import(&config, &portal, &cache)
        .await?;
    // let mut factorio = factorio::Importer::from_cache(0)
    //     .import(&config, &portal, &cache)
    //     .await?;

    info!("Factorio imported. {} mods", factorio.mods.count());

    // factorio
    //     .mods
    //     .add_from_portal("angelsindustries", None)
    //     .await?;

    factorio.mods.update().await?;
    factorio.mods.ensure_dependencies().await?;

    Ok(())
}
