//! Provides the `VersionInformation` object used to represent a server executable's version.

use crate::{error::ExecutableError, util::HumanVersion};
use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;

/// Represents a Factorio server's version information
#[derive(Debug)]
pub struct VersionInformation {
    /// The server's version.
    version: HumanVersion,
    /// The server's meta information (e.g. build number, platform)
    meta: String,
    /// The server's binary version.
    binary: String,
    /// The server's map input version.
    map_input: String,
    /// The server's map output version.
    map_output: String,
}

impl FromStr for VersionInformation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // example of valid version information:
        // Version: 0.18.47 (build 54412, linux64, headless)
        // Binary version: 64
        // Map input version: 0.16.0-0
        // Map output version: 0.18.47-0

        lazy_static! {
            static ref RE: Regex = Regex::new(r"^Version: ([\w.-]+) \((.+)\)\nBinary version: ([\w.-]+)\nMap input version: ([\w.-]+)\nMap output version: ([\w.-]+)$")
                .expect("failed to create version information regex");
        }

        let captures = RE
            .captures(s.trim())
            .ok_or_else(|| ExecutableError::InvalidVersionInformation {
                ver_str: String::from(s),
                source: anyhow::anyhow!("Regex returned no captures"),
            })?;

        let version = captures
            .get(1)
            .ok_or_else(|| ExecutableError::InvalidVersionInformation {
                ver_str: String::from(s),
                source: anyhow::anyhow!("Executable version not captured"),
            })?
            .as_str()
            .parse::<HumanVersion>()
            .map_err(|e| ExecutableError::InvalidVersionInformation {
                ver_str: String::from(s),
                source: e.into(),
            })?;
        let meta = captures
            .get(2)
            .ok_or_else(|| ExecutableError::InvalidVersionInformation {
                ver_str: String::from(s),
                source: anyhow::anyhow!("Meta not captured"),
            })?
            .as_str()
            .to_owned();
        let binary = captures
            .get(3)
            .ok_or_else(|| ExecutableError::InvalidVersionInformation {
                ver_str: String::from(s),
                source: anyhow::anyhow!("Binary version not captured"),
            })?
            .as_str()
            .to_owned();
        let map_input = captures
            .get(4)
            .ok_or_else(|| ExecutableError::InvalidVersionInformation {
                ver_str: String::from(s),
                source: anyhow::anyhow!("Map input version not captured"),
            })?
            .as_str()
            .to_owned();
        let map_output = captures
            .get(5)
            .ok_or_else(|| ExecutableError::InvalidVersionInformation {
                ver_str: String::from(s),
                source: anyhow::anyhow!("Map output version not captured"),
            })?
            .as_str()
            .to_owned();

        Ok(Self {
            version,
            meta,
            binary,
            map_input,
            map_output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_version_string_trailing_newline() {
        let ver_str = r"Version: 0.18.47 (build 54412, linux64, headless)
Binary version: 64
Map input version: 0.16.0-0
Map output version: 0.18.47-0
";
        let parsed = ver_str
            .parse::<VersionInformation>()
            .expect("valid version string failed to parse");

        assert_eq!(parsed.version, HumanVersion::new(0, 18, 47));
        assert_eq!(parsed.binary, "64");
        assert_eq!(parsed.map_input, "0.16.0-0");
        assert_eq!(parsed.map_output, "0.18.47-0");
    }

    #[test]
    fn valid_version_string_no_trailing_newline() {
        let ver_str = r"Version: 0.18.47 (build 54412, linux64, headless)
Binary version: 64
Map input version: 0.16.0-0
Map output version: 0.18.47-0";
        let parsed = ver_str
            .parse::<VersionInformation>()
            .expect("valid version string failed to parse");

        assert_eq!(parsed.version, HumanVersion::new(0, 18, 47));
        assert_eq!(parsed.binary, "64");
        assert_eq!(parsed.map_input, "0.16.0-0");
        assert_eq!(parsed.map_output, "0.18.47-0");
    }
}
