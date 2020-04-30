mod config;
mod factorio;
mod log;
mod mod_portal;

use ::log::*;
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::setup_logging()?;
    dotenv::dotenv()?;

    let config = Config::from_env()?;

    let mod_portal = mod_portal::ModPortal::new(&config)?;
    let mut factorio = factorio::Importer::from("./sample").import()?;

    info!("Factorio imported. {} mods", factorio.mods.count());

    factorio
        .mods
        .add(factorio::ModSource::Portal {
            mod_portal: &mod_portal,
            name: String::from("Aircraft"),
            version: None,
        })
        .await?;

    Ok(())
}
