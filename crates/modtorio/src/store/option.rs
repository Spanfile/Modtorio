//! Provides the [`Value`](Value) and [`Field`](Field) objects, used to store various options about
//! the program in the program store.

use derive::Model;
use rusqlite::{
    types::{self, FromSql},
    ToSql,
};
use std::{str::FromStr, string::ToString};
use strum_macros::{Display, EnumString};
use types::{FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};

/// Different field identifiers possibly used with the program store options.
#[derive(Debug, PartialEq, Copy, Clone, EnumString, Display)]
pub enum Field {
    /// The username to the mod portal.
    PortalUsername,
    /// The token to the mod portal.
    PortalToken,
}

/// A store option value.
#[derive(Debug, Model)]
#[table_name = "options"]
pub struct Value {
    /// The field of this value.
    #[index]
    field: Field,
    /// The optional string value of this value.
    value: Option<String>,
}

impl Value {
    /// Returns a new `Value` with a given field and optional string value.
    pub fn new(field: Field, value: Option<String>) -> Self {
        Self { field, value }
    }

    /// Returns a reference to the value string.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Consumes the `Value` and returns its inner string value.
    pub fn take_value(self) -> Option<String> {
        self.value
    }
}

impl ToSql for Field {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(types::Value::Text(self.to_string())))
    }
}

impl FromSql for Field {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match Self::from_str(value.as_str()?) {
            Ok(v) => Ok(v),
            Err(e) => Err(FromSqlError::Other(Box::new(e))),
        }
    }
}
