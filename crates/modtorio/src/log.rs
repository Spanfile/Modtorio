use crate::config::Config;
use chrono::Utc;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
pub use log::{debug, error, info, trace, warn};

pub fn setup_logging(config: &Config) -> anyhow::Result<()> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Magenta)
        .warn(Color::Yellow)
        .error(Color::Red);
    let time_format = "%y/%m/%d %H:%M:%S";

    Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "[{}] [{: ^6}] {}",
                // "[{} UTC] [{}] {}",
                Utc::now().format(time_format),
                colors.color(record.level()),
                msg
            ))
        })
        .level(config.log_level.to_level_filter())
        .level_for("hyper", log::LevelFilter::Info)
        .level_for("reqwest", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
