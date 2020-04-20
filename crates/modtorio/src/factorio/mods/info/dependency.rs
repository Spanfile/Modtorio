use anyhow::anyhow;
use semver::VersionReq;
use serde::{de, de::Visitor, Deserialize};
use std::{fmt, str::FromStr};

#[derive(Debug)]
pub enum Requirement {
    Mandatory,
    Optional,
    OptionalHidden,
    Incompatible,
}

#[derive(Debug)]
pub struct Dependency {
    pub requirement: Requirement,
    pub name: String,
    pub version: VersionReq,
}

impl FromStr for Dependency {
    type Err = anyhow::Error;

    fn from_str(s: &std::primitive::str) -> Result<Self, Self::Err> {
        let args: Vec<&str> = s.split(' ').collect();

        let mut requirement = Requirement::Mandatory;
        let name;
        let version;

        match args[0] {
            "?" => {
                requirement = Requirement::Optional;
                name = Some(args[1]);
                version = Some(VersionReq::parse(&args[2..].join(" "))?);
            }
            "!" => {
                requirement = Requirement::Incompatible;
                name = Some(args[1]);
                version = Some(VersionReq::parse(&args[2..].join(" "))?);
            }
            "(?)" => {
                requirement = Requirement::OptionalHidden;
                name = Some(args[1]);
                version = Some(VersionReq::parse(&args[2..].join(" "))?);
            }
            n => {
                name = Some(n);
                version = Some(VersionReq::parse(&args[1..].join(" "))?);
            }
        }

        let name = name
            .ok_or_else(|| anyhow!("failed to parse dependency name"))?
            .to_string();
        let version = version.ok_or_else(|| anyhow!("failed to parse dependency version"))?;

        Ok(Dependency {
            requirement,
            name,
            version,
        })
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.requirement {
            Requirement::Optional => f.write_str("? ")?,
            Requirement::Incompatible => f.write_str("! ")?,
            Requirement::OptionalHidden => f.write_str("(?)")?,
            _ => {}
        }

        f.write_fmt(format_args!("{} ", &self.name))?;
        f.write_str(&self.version.to_string())
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
