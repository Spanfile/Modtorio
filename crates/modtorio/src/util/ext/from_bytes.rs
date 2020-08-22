//! Provides the `FromBytes` extension trait.

use std::net::{Ipv4Addr, Ipv6Addr};

/// Provides the `from_bytes` function which is used to create a new object from its representation as a byte array.
pub trait FromBytes {
    /// Returns a new object from a given slice of bytes.
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl FromBytes for Ipv6Addr {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut v6_addr = [0u8; 16];
        for (i, byte) in bytes.iter().take(16).enumerate() {
            v6_addr[i] = *byte;
        }

        Ipv6Addr::from(v6_addr)
    }
}

impl FromBytes for Ipv4Addr {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut v4_addr = [0u8; 4];
        for (i, byte) in bytes.iter().take(16).enumerate() {
            v4_addr[i] = *byte;
        }

        Ipv4Addr::from(v4_addr)
    }
}
