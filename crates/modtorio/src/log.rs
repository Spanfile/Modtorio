use crate::config::Config;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
pub use log::{debug, error, info, trace, warn};
use std::time::Instant;

pub fn setup_logging(config: &Config) -> anyhow::Result<()> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Magenta)
        .warn(Color::Yellow)
        .error(Color::Red);
    let start = Instant::now();
    // let time_format = "%Y-%m-%d %H:%M:%S";

    Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "[{: >11.3}] [{: >5}] {{{}}} {}",
                // "[{} UTC] [{}] {}",
                // chrono::Utc::now().format(time_format),
                start.elapsed().as_secs_f32(),
                colors.color(record.level()),
                record.target(),
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
