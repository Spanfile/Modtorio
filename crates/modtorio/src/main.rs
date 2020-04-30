#![feature(drain_filter)]

mod config;
mod ext;
mod factorio;
mod log;
mod mod_portal;

use ::log::*;
use config::Config;

#[tokio::main(core_threads = 8)]
async fn main() -> anyhow::Result<()> {
    log::setup_logging()?;
    dotenv::dotenv()?;

    let config = Config::from_env()?;

    let mod_portal = mod_portal::ModPortal::new(&config)?;
    let mut factorio = factorio::Importer::from("./sample").import().await?;

    info!("Factorio imported. {} mods", factorio.mods.count());

    let updates = factorio.mods.check_updates(&mod_portal).await?;
    factorio.mods.apply_updates(&updates, &mod_portal).await?;

    Ok(())
}
