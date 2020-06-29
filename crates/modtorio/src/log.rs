use crate::config::Config;
use chrono::Local;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
pub use log::{debug, error, info, trace, warn};

const TIME_FORMAT: &str = "%y/%m/%d %H:%M:%S%.6f";

pub fn setup_logging(config: &Config) -> anyhow::Result<()> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Magenta);

    Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                // "[{}] [{}] {{{}}} {}",
                // "[{} UTC] [{}] {}",
                Local::now().format(TIME_FORMAT),
                colors.color(record.level()),
                // record.target(),
                msg
            ))
        })
        .level(config.log.level.to_level_filter())
        .level_for("hyper", log::LevelFilter::Info)
        .level_for("reqwest", log::LevelFilter::Info)
        .level_for("mio", log::LevelFilter::Info)
        .level_for("module", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
