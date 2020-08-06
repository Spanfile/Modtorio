use derive::Model;
use rusqlite::{
    types::{self, FromSql},
    ToSql,
};
use std::{str::FromStr, string::ToString};
use strum_macros::{Display, EnumString};
use types::{FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};

#[derive(Debug, PartialEq, Copy, Clone, EnumString, Display)]
pub enum Field {
    PortalUsername,
    PortalToken,
    SchemaChecksum,
}

#[derive(Debug, Model)]
#[table_name = "options"]
pub struct Value {
    #[index]
    field: Field,
    value: Option<String>,
}

impl Value {
    pub fn new(field: Field, value: Option<String>) -> Self {
        Self { field, value }
    }

    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

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
