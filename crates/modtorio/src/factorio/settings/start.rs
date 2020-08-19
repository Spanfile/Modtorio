//! Provides the [`Start`](Start) struct which corresponds to the various server start command line options.

use crate::{error::SettingsError, store::models::GameSettings};
use rpc::server_settings;
use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    ToSql,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::{Display, EnumString};

/// Represents the various start command line options.
///
/// Defaults to `AdminsOnly`.
#[derive(Deserialize, Serialize, Debug, PartialEq, Default)]
pub struct Start {
    /// The save or scenario name to use.
    pub save_name: String,
    /// The start behaviour.
    pub behaviour: StartBehaviour,
}

/// Represents the combination of the `--create`, `--start-server`, `--start-server-load-latest` and
/// `--start-server-load-scenario` command line parameters.
#[derive(Deserialize, Serialize, Debug, PartialEq, Copy, Clone, EnumString, Display)]
pub enum StartBehaviour {
    /// Corresponds to using the `--start-server-load-latest` command line option.
    LoadLatest,
    /// Corresponds to using the `--start-server` command line option.
    LoadFile,
    /// Corresponds to using the `--start-server-load-scenario` command line option.
    LoadScenario, // TODO: these two
    /// Corresponds to using the `--create` command line option.
    Create,
}

impl Default for StartBehaviour {
    fn default() -> Self {
        Self::LoadLatest
    }
}

impl ToSql for StartBehaviour {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}

impl FromSql for StartBehaviour {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match StartBehaviour::from_str(value.as_str()?) {
            Ok(v) => Ok(v),
            Err(_) => Err(FromSqlError::InvalidType), // TODO: bad error type?
        }
    }
}

impl Start {
    /// Returns a new `Start` from a given `GameSettings`.
    pub fn from_store_format(store_format: &GameSettings) -> Self {
        Self {
            save_name: store_format.save_name.to_owned(),
            behaviour: store_format.start_behaviour,
        }
    }

    /// Modifies a given `GameSettings` with this object's settings.
    pub fn to_store_format(&self, store_format: &mut GameSettings) {
        store_format.save_name = self.save_name.to_owned();
        store_format.start_behaviour = self.behaviour;
    }

    /// Returns a new `Start` from a given `ServerSettings`.
    pub fn from_rpc_format(rpc_format: &rpc::ServerSettings) -> anyhow::Result<Self> {
        Ok(Self {
            save_name: rpc_format.save_name.to_owned(),
            behaviour: match rpc_format.start_behaviour {
                0 => StartBehaviour::LoadLatest,
                1 => StartBehaviour::LoadFile,
                2 => StartBehaviour::LoadScenario,
                3 => StartBehaviour::Create,
                v => return Err(SettingsError::UnexpectedValue(v.to_string()).into()),
            },
        })
    }

    /// Modifies a given `ServerSettings` with this object's settings.
    pub fn to_rpc_format(&self, rpc_format: &mut rpc::ServerSettings) {
        rpc_format.save_name = self.save_name.to_owned();
        rpc_format.start_behaviour = match self.behaviour {
            StartBehaviour::LoadLatest => server_settings::StartBehaviour::LoadLatest.into(),
            StartBehaviour::LoadFile => server_settings::StartBehaviour::LoadFile.into(),
            StartBehaviour::LoadScenario => server_settings::StartBehaviour::LoadScenario.into(),
            StartBehaviour::Create => server_settings::StartBehaviour::Create.into(),
        };
    }
}
