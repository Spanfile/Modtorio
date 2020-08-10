//! Provides the [`Opts`](Opts) struct, used to read and access the program's command line
//! arguments.

use crate::{config, util::LogLevel};
use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;
use strum::VariantNames;

/// Stores command line parameters.
#[derive(Debug)]
pub struct Opts {
    /// Path to the config file.
    pub config: PathBuf,
    /// Path to the program store database file.
    pub store: PathBuf,
    /// Whether to skip applying configuration from the environment variables.
    pub no_env: bool,
    /// Whether to skip applying configuration from the configuration file.
    pub no_conf: bool,
    /// The log level to use.
    pub log_level: Option<LogLevel>,
    /// The program cache expiry in seconds.
    pub cache_expiry: Option<u64>,
}

impl Opts {
    /// Builds a new `clap::App` used to parse a given set of command line parameters.
    fn build_app() -> App<'static, 'static> {
        App::new(clap::crate_name!())
            .version(clap::crate_version!())
            .author(clap::crate_authors!())
            .about(clap::crate_description!())
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("FILE")
                    .default_value(config::DEFAULT_CONFIG_FILE_LOCATION)
                    .help("Sets a custom config file")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("store")
                    .long("store")
                    .value_name("FILE")
                    .default_value(config::DEFAULT_STORE_FILE_LOCATION)
                    .help(
                        "Sets a custom program persistent store file. The special value '_memory' specifies an \
                         ephemeral in-memory store, which is primarily used for debugging purposes.",
                    )
                    .takes_value(true),
            )
            .arg(Arg::with_name("no-env").long("no-env").help(
                "Skip loading configuration values from the environment variables. Primarily used for debugging \
                 purposes.",
            ))
            .arg(
                Arg::with_name("no-conf").long("no-conf").help(
                    "Skip loading configuration values from the config file. Primarily used for debugging purposes.",
                ),
            )
            .arg(
                Arg::with_name("log-level")
                    .long("log-level")
                    .value_name("LOG LEVEL")
                    .possible_values(LogLevel::VARIANTS)
                    .case_insensitive(true)
                    .takes_value(true)
                    .help("Specify the log level to use."),
            )
            .arg(
                Arg::with_name("cache-expiry")
                    .long("cache-expiry")
                    .value_name("SECONDS")
                    .takes_value(true)
                    .help("Specify the cache expiry time."),
            )
    }

    /// Returns a new `Opts` object from a given set of matched command line parameters.
    fn from_matches(matches: &ArgMatches) -> Self {
        Opts {
            config: matches
                .value_of_os("config")
                .expect("config option has no value")
                .into(),
            store: matches.value_of_os("store").expect("store option has no value").into(),
            no_env: matches.is_present("no-env"),
            no_conf: matches.is_present("no-conf"),
            log_level: matches
                .value_of("log-level")
                .map(|s| s.parse().expect("failed to parse value as log level")),
            cache_expiry: matches
                .value_of("cache-expiry")
                .map(|s| s.parse().expect("failed to parse value as u64")),
        }
    }

    /// Returns a new `Opts` object built from the program's command line parameters.
    pub fn get() -> Opts {
        Opts::from_matches(&Opts::build_app().get_matches())
    }

    #[allow(dead_code)]
    /// Returns a new `Opts` object built from custom command line parameters.
    pub fn custom_args(args: &[&str]) -> Opts {
        let mut full_args = vec!["modtorio"];
        full_args.extend_from_slice(args);
        Opts::from_matches(&Opts::build_app().get_matches_from(&full_args))
    }
}
