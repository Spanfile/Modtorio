//! Provides the [Traffic](Traffic) object which corresponds to a server's settings about its
//! network traffic.

use super::ServerSettingsGameFormat;
use crate::{
    store::models::GameSettings,
    util::{Limit, Range},
};
use rpc::{server_settings, server_settings::socket_addr};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

/// Factorio's default server listen port.
pub const DEFAULT_LISTEN_PORT: u16 = 34197;

/// Contains a server's settings related to its upload capabilities.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Upload {
    /// Corresponds to the `max_upload_in_kilobytes_per_second` field. Defaults to
    /// `Limit::Unlimited` (value of 0 in `server-settings.json`).
    pub max: Limit,
    /// Corresponds to the `max_upload_slots` field. Defaults to `Limit::Limited(5)` (value of 5 in
    /// `server-settings.json`).
    pub slots: Limit,
}

/// Contains a server's settings related to network message segment sizes.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct SegmentSize {
    /// Corresponds to the `minimum_segment_size` and `maximum_segment_size` fields. Defaults to a
    /// minimum of 25 and a maximum of 100.
    pub size: Range,
    /// Corresponds to the `minimum_segment_size_peer_count` and `maximum_segment_size_peer_count`
    /// fields. Defaults to a minimum of 20 and a maximum of 10.
    pub peer_count: Range,
}

/// Contains a server's settings related to its network traffic.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Network {
    /// Corresponds to the various upload settings.
    pub upload: Upload,
    /// Corresponds to the `minimum_latency_in_ticks` field. Defaults to 0.
    pub minimum_latency: u64,
    /// Corresponds to the various network message segment size settings.
    pub segment_size: SegmentSize,
    /// Corresponds to the `--bind` command line option.
    pub bind_address: SocketAddr,
}

impl Default for Upload {
    fn default() -> Self {
        Self {
            max: Limit::Unlimited,
            slots: Limit::Limited(5),
        }
    }
}

impl Default for SegmentSize {
    fn default() -> Self {
        Self {
            size: Range { min: 25, max: 100 },
            peer_count: Range { min: 20, max: 10 },
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self {
            upload: Default::default(),
            minimum_latency: Default::default(),
            segment_size: Default::default(),
            bind_address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, DEFAULT_LISTEN_PORT)),
        }
    }
}

impl Network {
    pub fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
        Ok(Self {
            upload: Upload {
                max: Limit::from(game_format.max_upload_in_kilobytes_per_second),
                slots: Limit::from(game_format.max_upload_slots),
            },
            minimum_latency: game_format.minimum_latency_in_ticks,
            segment_size: SegmentSize {
                size: Range {
                    min: game_format.minimum_segment_size,
                    max: game_format.maximum_segment_size,
                },
                peer_count: Range {
                    min: game_format.minimum_segment_size_peer_count,
                    max: game_format.maximum_segment_size_peer_count,
                },
            },
            // the game format does not include the listen address
            ..Default::default()
        })
    }

    pub fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.max_upload_in_kilobytes_per_second = self.upload.max.into();
        game_format.max_upload_slots = self.upload.slots.into();
        game_format.minimum_latency_in_ticks = self.minimum_latency;
        game_format.minimum_segment_size = self.segment_size.size.min;
        game_format.maximum_segment_size = self.segment_size.size.max;
        game_format.minimum_segment_size_peer_count = self.segment_size.peer_count.min;
        game_format.maximum_segment_size_peer_count = self.segment_size.peer_count.max;

        Ok(())
    }

    pub fn from_store_format(store_format: &GameSettings) -> anyhow::Result<Self> {
        Ok(Self {
            upload: Upload {
                max: Limit::from(store_format.max_upload_in_kilobytes_per_second as u64),
                slots: Limit::from(store_format.max_upload_slots as u64),
            },
            minimum_latency: store_format.minimum_latency_in_ticks as u64,
            segment_size: SegmentSize {
                size: Range {
                    min: store_format.minimum_segment_size as u64,
                    max: store_format.maximum_segment_size as u64,
                },
                peer_count: Range {
                    min: store_format.minimum_segment_size_peer_count as u64,
                    max: store_format.maximum_segment_size_peer_count as u64,
                },
            },
            // the store format does not include the listen address
            ..Default::default()
        })
    }

    pub fn to_store_format(&self, store_format: &mut GameSettings) -> anyhow::Result<()> {
        store_format.max_upload_in_kilobytes_per_second = u64::from(self.upload.max) as i64;
        store_format.max_upload_slots = u64::from(self.upload.slots) as i64;
        store_format.minimum_latency_in_ticks = self.minimum_latency as i64;
        store_format.minimum_segment_size = self.segment_size.size.min as i64;
        store_format.maximum_segment_size = self.segment_size.size.max as i64;
        store_format.minimum_segment_size_peer_count = self.segment_size.peer_count.min as i64;
        store_format.maximum_segment_size_peer_count = self.segment_size.peer_count.max as i64;

        Ok(())
    }

    pub fn from_rpc_format(rpc_format: &rpc::ServerSettings) -> anyhow::Result<Self> {
        let bind_address = if let Some(bind_addr) = &rpc_format.bind {
            let port = bind_addr.port as u16;
            if let Some(addr) = &bind_addr.addr {
                match addr {
                    socket_addr::Addr::V4(v4_addr) => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(*v4_addr), port)),
                    socket_addr::Addr::V6(v6_bytes) => {
                        // the byte array from protobuf may contain any number of bytes. copy up to the first 16 bytes
                        // into a static array to build a v6 address
                        let mut v6_addr = [0u8; 16];
                        for (i, byte) in v6_bytes.iter().take(16).enumerate() {
                            v6_addr[i] = *byte;
                        }

                        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(v6_addr), port, 0, 0))
                    }
                }
            } else {
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
            }
        } else {
            Self::default().bind_address
        };

        Ok(Self {
            upload: Upload {
                max: Limit::from(rpc_format.max_upload_in_kilobytes_per_second),
                slots: Limit::from(rpc_format.max_upload_slots),
            },
            minimum_latency: rpc_format.minimum_latency_in_ticks,
            segment_size: SegmentSize {
                size: Range {
                    min: rpc_format.minimum_segment_size,
                    max: rpc_format.maximum_segment_size,
                },
                peer_count: Range {
                    min: rpc_format.minimum_segment_size_peer_count,
                    max: rpc_format.maximum_segment_size_peer_count,
                },
            },
            bind_address,
        })
    }

    pub fn to_rpc_format(&self, rpc_format: &mut rpc::ServerSettings) -> anyhow::Result<()> {
        rpc_format.max_upload_in_kilobytes_per_second = self.upload.max.into();
        rpc_format.max_upload_slots = self.upload.slots.into();
        rpc_format.minimum_latency_in_ticks = self.minimum_latency;
        rpc_format.minimum_segment_size = self.segment_size.size.min;
        rpc_format.maximum_segment_size = self.segment_size.size.max;
        rpc_format.minimum_segment_size_peer_count = self.segment_size.peer_count.min;
        rpc_format.maximum_segment_size_peer_count = self.segment_size.peer_count.max;
        rpc_format.bind = Some(server_settings::SocketAddr {
            port: self.bind_address.port() as u32,
            addr: Some(match self.bind_address.ip() {
                IpAddr::V4(v4_addr) => socket_addr::Addr::V4(u32::from(v4_addr)),
                IpAddr::V6(v6_addr) => socket_addr::Addr::V6(v6_addr.octets().to_vec()),
            }),
        });

        Ok(())
    }
}
