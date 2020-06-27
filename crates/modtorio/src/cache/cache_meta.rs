use rusqlite::{
    types::{self, FromSql},
    ToSql,
};
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use types::{FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};

#[derive(Debug, PartialEq, Copy, Clone, EnumString, Display)]
pub enum CacheMetaField {
    SchemaChecksum,
}

#[derive(Debug)]
pub struct CacheMetaValue {
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
