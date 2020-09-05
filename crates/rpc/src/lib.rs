tonic::include_proto!("mod_rpc");

use std::net::IpAddr;

/// The version of the RPC protocol buffer specification.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

impl From<()> for Empty {
    fn from(_: ()) -> Empty {
        Empty {}
    }
}

impl From<String> for PossibleWarning {
    fn from(warning: String) -> PossibleWarning {
        PossibleWarning { warning }
    }
}

impl From<Option<String>> for PossibleWarning {
    fn from(warning: Option<String>) -> PossibleWarning {
        PossibleWarning {
            warning: warning.unwrap_or_default(),
        }
    }
}

impl From<std::net::SocketAddr> for SocketAddr {
    fn from(addr: std::net::SocketAddr) -> SocketAddr {
        SocketAddr {
            port: addr.port() as u32,
            addr: Some(match addr.ip() {
                IpAddr::V4(v4_addr) => socket_addr::Addr::V4(u32::from(v4_addr)),
                IpAddr::V6(v6_addr) => socket_addr::Addr::V6(v6_addr.octets().to_vec()),
            }),
        }
    }
}
