//! Program cache meta information. Used to store metadata about the program and cache itself with a
//! [CacheMetaValue].
//!
//! [CacheMetaValue]: CacheMetaValue

use derive::Model;
use rusqlite::{
    types::{self, FromSql},
    ToSql,
};
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use types::{FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};

/// A meta value's field identifier. Derives [EnumString] and
/// [Display] to allow converting the enum to/from a string. Implements
/// [ToSql] and [FromSql] which use the to/from string
/// functions.
///
/// [EnumString]: strum_macros::EnumString
/// [Display]: strum_macros::Display
/// [ToSql]: rusqlite::ToSql
/// [FromSql]: rusqlite::types::FromSql
#[derive(Debug, PartialEq, Copy, Clone, EnumString, Display)]
pub enum CacheMetaField {
    /// The current cache database schema's checksum.
    SchemaChecksum,
}

/// A meta value for the program cache. Consists of a `CacheMetaField` field identifier and an
/// optional `String` value. Derives `Model` with the table name `_meta`.
#[derive(Debug, Model)]
#[table_name = "_meta"]
pub struct CacheMetaValue {
    #[index]
    pub field: CacheMetaField,
    pub value: Option<String>,
}

impl ToSql for CacheMetaField {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(types::Value::Text(self.to_string())))
    }
}

impl FromSql for CacheMetaField {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match CacheMetaField::from_str(value.as_str()?) {
            Ok(v) => Ok(v),
            Err(e) => Err(FromSqlError::Other(Box::new(e))),
        }
    }
}
