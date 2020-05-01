#![feature(drain_filter)]

mod config;
mod ext;
mod factorio;
mod log;
mod mod_portal;

use ::log::*;
use config::Config;
use mod_portal::ModPortal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::setup_logging()?;
    dotenv::dotenv()?;

    let config = Config::from_env()?;
    let portal = ModPortal::new(&config)?;

    let mut factorio = factorio::Importer::from("./sample")
        .import(&config, &portal)
        .await?;

    info!("Factorio imported. {} mods", factorio.mods.count());

    let updates = factorio.mods.check_updates().await?;
    factorio.mods.apply_updates(&updates).await?;

    Ok(())
}
