//! Provides functionality to set up a logging facade and print logging information for the program.

use crate::config::Config;
use fern::Dispatch;
pub use log::{debug, error, info, trace, warn};
use std::{thread, time::Instant};

/// Sets up the logging facade.
pub fn setup_logging(config: &Config) -> anyhow::Result<()> {
    let start = Instant::now();
    Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "{: >11.3} {: >5} [{:?}] {}",
                // "[{} UTC] [{}] {}",
                // chrono::Utc::now().format(time_format),
                start.elapsed().as_secs_f32(),
                record.level(),
                thread::current().id().as_u64(),
                msg
            ))
        })
        .level(config.log_level().to_level_filter())
        .level_for("hyper", log::LevelFilter::Info)
        .level_for("reqwest", log::LevelFilter::Info)
        .level_for("mio", log::LevelFilter::Info)
        .level_for("module", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
