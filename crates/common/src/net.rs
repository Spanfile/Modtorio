//! Common network utilties.

use serde::{de, de::Visitor, Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
};

/// The prefix used to denote Unix socket paths
const UNIX_SOCKET_PREFIX: &str = "unix:";

/// An combined address used with client-server connections.
#[derive(Debug, PartialEq, Clone)]
pub enum NetAddress {
    /// An address used with a TCP connection. Consists of a `SocketAddr`; an address-port pair.
    TCP(SocketAddr),
    /// An address used with a Unix socket connection. Consists of a path to the socket file.
    Unix(PathBuf),
}

impl Display for NetAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NetAddress::TCP(addr) => write!(f, "{}", addr),
            NetAddress::Unix(path) => write!(f, "{}{}", UNIX_SOCKET_PREFIX, path.display()),
        }
    }
}

impl FromStr for NetAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(path) = s.strip_prefix(UNIX_SOCKET_PREFIX) {
            Ok(NetAddress::Unix(PathBuf::from(path)))
        } else {
            Ok(NetAddress::TCP(s.parse()?))
        }
    }
}

impl Serialize for NetAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for NetAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[allow(clippy::missing_docs_in_private_items)]
        struct NetAddressVisitor;

        impl<'de> Visitor<'de> for NetAddressVisitor {
            type Value = NetAddress;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("net address string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse::<Self::Value>()
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(NetAddressVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn parse() {
        assert_eq!(
            "0.0.0.0:1337".parse::<NetAddress>().expect("failed to parse TCP"),
            NetAddress::TCP(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1337))
        );

        assert_eq!(
            "unix:/temp/path".parse::<NetAddress>().expect("failed to parse Unix"),
            NetAddress::Unix(PathBuf::from("/temp/path"))
        );
    }

    #[test]
    fn serialize() {
        assert_eq!(
            serde_json::to_string(&NetAddress::TCP(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                1337
            )))
            .expect("failed to serialize TCP"),
            r#""0.0.0.0:1337""#
        );

        assert_eq!(
            serde_json::to_string(&NetAddress::Unix(PathBuf::from("/temp/path"))).expect("failed to serialize Unix"),
            r#""unix:/temp/path""#
        );
    }

    #[test]
    fn deserialize() {
        assert_eq!(
            serde_json::from_str::<NetAddress>(r#""0.0.0.0:1337""#).expect("failed to deserialize TCP"),
            NetAddress::TCP(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1337))
        );

        assert_eq!(
            serde_json::from_str::<NetAddress>(r#""unix:/temp/path""#).expect("failed to deserialize Unix"),
            NetAddress::Unix(PathBuf::from("/temp/path"))
        );
    }
}
