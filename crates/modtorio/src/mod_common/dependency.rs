use crate::{cache, util::HumanVersionReq};
use anyhow::anyhow;
use lazy_static::lazy_static;
use regex::Regex;
use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    ToSql,
};
use serde::{de, de::Visitor, Deserialize};
use std::{fmt, str::FromStr};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Requirement {
    Mandatory = 0,
    Optional,
    OptionalHidden,
    Incompatible,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Dependency {
    requirement: Requirement,
    name: String,
    version: Option<HumanVersionReq>,
}

impl Dependency {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn requirement(&self) -> Requirement {
        self.requirement
    }

    pub fn version(&self) -> Option<HumanVersionReq> {
        self.version
    }
}

impl FromStr for Dependency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"(\?|!|\(\?\))? ?([^>=<]+)( ?[>=<]{1,2} ?[\d\.]*)?$").unwrap();
        }

        let captures = RE
            .captures(s)
            .ok_or_else(|| anyhow!("dependency regex returned no captures"))?;

        let requirement = captures
            .get(1)
            .map(|c| c.as_str())
            .map_or(Ok(Requirement::Mandatory), |s| s.parse::<Requirement>())?;

        let name = captures
            .get(2)
            .ok_or_else(|| anyhow!("dependency regex didn't capture name"))?
            .as_str()
            .trim()
            .to_string();

        let version = captures.get(3).map_or(Ok(None), |c| {
            c.as_str().parse::<HumanVersionReq>().map(Some)
        })?;

        Ok(Dependency {
            requirement,
            name,
            version,
        })
    }
}

impl FromStr for Requirement {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "?" => Ok(Requirement::Optional),
            "!" => Ok(Requirement::Incompatible),
            "(?)" => Ok(Requirement::OptionalHidden),
            "" => Ok(Requirement::Mandatory),
            _ => Err(anyhow!("invalid string for requirement: {}", s)),
        }
    }
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Requirement::Optional => f.write_str("?"),
            Requirement::Incompatible => f.write_str("!"),
            Requirement::OptionalHidden => f.write_str("(?)"),
            Requirement::Mandatory => f.write_str(""),
        }
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{} {} ", self.requirement, &self.name))?;
        if let Some(version) = self.version {
            f.write_fmt(format_args!("{}", version))?;
        }

        Ok(())
    }
}

impl ToSql for Requirement {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}

impl FromSql for Requirement {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match Requirement::from_str(value.as_str()?) {
            Ok(v) => Ok(v),
            Err(_) => Err(FromSqlError::InvalidType), // TODO: bad error type?
        }
    }
}

impl From<cache::models::ReleaseDependency> for Dependency {
    fn from(dep: cache::models::ReleaseDependency) -> Self {
        Self {
            requirement: dep.requirement,
            name: dep.name,
            version: dep.version_req,
        }
    }
}

impl<'de> Deserialize<'de> for Dependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DependencyVisitor;

        impl<'de> Visitor<'de> for DependencyVisitor {
            type Value = Dependency;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("dependency string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse::<Self::Value>()
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(DependencyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_str() -> anyhow::Result<()> {
        assert_eq!(
            "base".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Mandatory,
                name: String::from("base"),
                version: None,
            }
        );

        assert_eq!(
            "base >= 0.18.0".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Mandatory,
                name: String::from("base"),
                version: Some(">= 0.18.0".parse().unwrap()),
            }
        );

        assert_eq!(
            "base >= 0.18.00".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Mandatory,
                name: String::from("base"),
                version: Some(">= 0.18.00".parse().unwrap()),
            }
        );

        assert_eq!(
            "!base".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Incompatible,
                name: String::from("base"),
                version: None,
            }
        );

        assert_eq!(
            "?base".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::Optional,
                name: String::from("base"),
                version: None,
            }
        );

        assert_eq!(
            "(?)base".parse::<Dependency>()?,
            Dependency {
                requirement: Requirement::OptionalHidden,
                name: String::from("base"),
                version: None,
            }
        );

        Ok(())
    }
}
