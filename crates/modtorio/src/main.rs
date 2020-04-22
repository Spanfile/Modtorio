mod config;
mod factorio;
mod log;
mod mod_portal;

use ::log::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::setup_logging()?;

    let config = config::Config {
        portal: config::PortalConfig {
            username: String::from("Spans"),
            token: String::from("e41a4beb65dd776d47ae1fc8cffb8d"),
        },
    };

    let mod_portal = mod_portal::ModPortal::new(&config);
    let factorio = factorio::Importer::from("./sample").import()?;

    info!("Factorio imported. {} mods", factorio.mods.count());

    Ok(())
}
