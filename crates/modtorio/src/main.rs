mod factorio;
mod log;

use ::log::*;

fn main() -> anyhow::Result<()> {
    log::setup_logging()?;

    let factorio = factorio::Importer::from("./sample").import()?;
    info!("Factorio imported. {} mods", factorio.mods.mods.len());
    Ok(())
}
