#![feature(drain_filter)]

mod config;
mod ext;
mod factorio;
mod log;
mod mod_common;
mod mod_portal;

use ::log::*;
use config::Config;
use mod_portal::ModPortal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    let config = Config::from_env()?;

    log::setup_logging(&config)?;
    config.debug_values();

    let portal = ModPortal::new(&config)?;

    let mut factorio = factorio::Importer::from("./sample")
        .import(&config, &portal)
        .await?;

    info!("Factorio imported. {} mods", factorio.mods.count());

    // factorio
    //     .mods
    //     .add_from_portal("angelsindustries", None)
    //     .await?;

    factorio.mods.update().await?;

    Ok(())
}
