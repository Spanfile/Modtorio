use crate::config::Config;
use chrono::Local;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
pub use log::{debug, error, info, trace, warn};

pub fn setup_logging(config: &Config) -> anyhow::Result<()> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Magenta);
    let time_format = "%y/%m/%d %H:%M:%S%.6f";

    Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "[{}] [{: ^6}] {}",
                // "[{} UTC] [{}] {}",
                Local::now().format(time_format),
                colors.color(record.level()),
                msg
            ))
        })
        .level(config.log.level.to_level_filter())
        .level_for("hyper", log::LevelFilter::Info)
        .level_for("reqwest", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
