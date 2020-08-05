//! Provides the [`Opts`](Opts) struct, used to read and access the program's command line
//! arguments.

use crate::config;
use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

/// Stores command line parameters.
#[derive(Debug)]
pub struct Opts {
    pub config: PathBuf,
    pub store: PathBuf,
    pub no_env: bool,
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
                        "Sets a custom program persistent store file. The special value '_memory' \
                         specifies an ephemeral in-memory store, which is primarily used for \
                         debugging purposes.",
                    )
                    .takes_value(true),
            )
            .arg(Arg::with_name("no-env").long("no-env").help(
                "Skip loading configuration values from the environment variables. Primarily used \
                 for debugging purposes.",
            ))
    }

    /// Returns a new `Opts` object from a given set of matched command line parameters.
    fn from_matches(matches: &ArgMatches) -> Self {
        Opts {
            config: matches
                .value_of_os("config")
                .expect("config option has no value")
                .into(),
            store: matches
                .value_of_os("store")
                .expect("store option has no value")
                .into(),
            no_env: matches.is_present("no-env"),
        }
    }

    /// Returns a new `Opts` object built from the program's command line parameters.
    pub fn get() -> Opts {
        Opts::from_matches(&Opts::build_app().get_matches())
    }

    /// Returns a new `Opts` object built from custom command line parameters.
    pub fn custom_args(args: &[&str]) -> Opts {
        Opts::from_matches(&Opts::build_app().get_matches_from(args))
    }
}
