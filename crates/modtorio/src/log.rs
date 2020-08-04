//! Provides functionality to set up a logging facade and print logging information for the program.

use crate::config::Config;
use chrono::Local;
use fern::Dispatch;
pub use log::{debug, error, info, trace, warn};

/// The time format used in log messages.
const TIME_FORMAT: &str = "%y/%m/%d %H:%M:%S%.6f";

/// Sets up the logging facade.
pub fn setup_logging(config: &Config) -> anyhow::Result<()> {
    Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                // "[{}] [{}] {{{}}} {}",
                // "[{} UTC] [{}] {}",
                Local::now().format(TIME_FORMAT),
                record.level(),
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
