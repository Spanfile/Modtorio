use ::log::*;
use modtorio::*;
use std::{env, fs::File, path::Path, sync::Arc};

/// The program's version at build-time.
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The name of the environment variable used to store the mod portal username
const PORTAL_USERNAME_ENV_VARIABLE: &str = "MODTORIO_PORTAL_USERNAME";
/// The name of the environment variable used to store the mod portal token
const PORTAL_TOKEN_ENV_VARIABLE: &str = "MODTORIO_PORTAL_TOKEN";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = opts::Opts::get();
    let store = store::Builder::from_location((&opts.store).into())
        .build()
        .await?;
    let config = build_config(&opts, &store).await?;

    log::setup_logging(&config)?;

    debug!("{:?}", opts);
    debug!("Env {:?}", util::env::dump_lines(APP_PREFIX));
    debug!("{:?}", config);

    if !opts.no_env {
        update_store_from_env(&store).await?;
    }
    log_program_information();

    let modtorio = Arc::new(Modtorio::new(config, store).await?);

    debug!("Program initialisation complete, running Modtorio");
    Modtorio::run(modtorio).await?;
    Ok(())
}

/// Builds a complete configuration object with given command-line Opts and a program Store.
async fn build_config(opts: &opts::Opts, store: &store::Store) -> anyhow::Result<config::Config> {
    let mut builder = config::Builder::new();

    if !opts.no_conf {
        if !opts.config.exists() {
            create_default_config_file(&opts.config)?;
        }

        builder = builder.apply_config_file(&mut File::open(&opts.config)?)?;
    }

    builder = builder.apply_opts(opts).apply_store(&store).await?;

    if !opts.no_env {
        builder = builder.apply_env()?;
    }

    Ok(builder.build())
}

/// Creates a new config file with default values in a given path. This will overwrite any existing
/// file in the path.
fn create_default_config_file<P>(path: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    config::Config::write_default_config_to_writer(&mut File::create(path)?)
}

/// Updates a given program `Store` from the current environment variables.
///
/// The following values are updated:
/// * `Field::PortalUsername` from the variable whose name is in the constant
///   `PORTAL_USERNAME_ENV_VARIABLE`
/// * `Field::PortalToken` from the variable whose name is in the constant
///   `PORTAL_TOKEN_ENV_VARIABLE`
async fn update_store_from_env(store: &store::Store) -> anyhow::Result<()> {
    store.begin_transaction()?;

    for (key, value) in util::env::dump_map(APP_PREFIX) {
        match key.as_str() {
            PORTAL_USERNAME_ENV_VARIABLE => {
                debug!("Got portal username env variable, updating store");
                store
                    .set_option(store::option::Value::new(
                        store::option::Field::PortalUsername,
                        Some(value),
                    ))
                    .await?
            }
            PORTAL_TOKEN_ENV_VARIABLE => {
                debug!("Got portal token env variable, updating store");
                store
                    .set_option(store::option::Value::new(
                        store::option::Field::PortalToken,
                        Some(value),
                    ))
                    .await?
            }
            _ => {}
        }
    }

    store.commit_transaction()?;
    Ok(())
}

/// Logs the program's information.
fn log_program_information() {
    info!("Program version: {}", VERSION);
    info!(
        "Working directory: {}",
        env::current_dir()
            .expect("failed to get current working directory")
            .display()
    );
}
