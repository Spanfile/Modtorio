//! Program store meta information. Used to store metadata about the program and the program store
//! itself with a [`Value`].
//!
//! [Value]: Value

use derive::Model;
use rusqlite::{
    types::{self, FromSql},
    ToSql,
};
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use types::{FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};

/// A meta value's field identifier. Derives [`EnumString`] and
/// [`Display`] to allow converting the enum to/from a string. Implements
/// [`ToSql`] and [`FromSql`] which use the to/from string
/// functions.
///
/// [EnumString]: strum_macros::EnumString
/// [Display]: strum_macros::Display
/// [ToSql]: rusqlite::ToSql
/// [FromSql]: rusqlite::types::FromSql
#[derive(Debug, PartialEq, Copy, Clone, EnumString, Display)]
pub enum Field {
    /// The current cache database schema's checksum.
    SchemaChecksum,
}

/// A meta value for the program store. Consists of a field identifier and an
/// optional `String` value. Derives `Model` with the table name `_meta`.
#[derive(Debug, Model)]
#[table_name = "_meta"]
pub struct Value {
    #[index]
    pub field: Field,
    pub value: Option<String>,
}

impl ToSql for Field {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(types::Value::Text(self.to_string())))
    }
}

impl FromSql for Field {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match Field::from_str(value.as_str()?) {
            Ok(v) => Ok(v),
            Err(e) => Err(FromSqlError::Other(Box::new(e))),
        }
    }
}
