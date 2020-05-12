#![feature(drain_filter)]
#![feature(async_closure)]

mod config;
mod ext;
mod factorio;
mod log;
mod mod_common;
mod mod_portal;
mod util;

use ::log::*;
use config::Config;
use mod_portal::ModPortal;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    let config = Arc::new(Config::from_env()?);

    log::setup_logging(&config)?;
    config.debug_values();

    let portal = Arc::new(ModPortal::new(&config)?);

    let mut factorio = factorio::Importer::from("./sample")
        .import(Arc::clone(&config), Arc::clone(&portal))
        .await?;

    info!("Factorio imported. {} mods", factorio.mods.count());

    // factorio
    //     .mods
    //     .add_from_portal("angelsindustries", None)
    //     .await?;

    factorio.mods.update().await?;
    factorio.mods.ensure_dependencies().await?;

    Ok(())
}
