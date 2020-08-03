//! Provides the [Traffic](Traffic) object which corresponds to a server's settings about its
//! network traffic.

use super::{GameFormatConversion, ServerSettingsGameFormat};
use crate::util::{Limit, Range};
use serde::{Deserialize, Serialize};

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
#[derive(Deserialize, Serialize, Debug, Default, PartialEq)]
pub struct Traffic {
    /// Corresponds to the various upload settings.
    pub upload: Upload,
    /// Corresponds to the `minimum_latency_in_ticks` field. Defaults to 0.
    pub minimum_latency: u64,
    /// Corresponds to the various network message segment size settings.
    pub segment_size: SegmentSize,
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

impl GameFormatConversion for Traffic {
    fn from_game_format(game_format: &ServerSettingsGameFormat) -> anyhow::Result<Self> {
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
        })
    }

    fn to_game_format(&self, game_format: &mut ServerSettingsGameFormat) -> anyhow::Result<()> {
        game_format.max_upload_in_kilobytes_per_second = self.upload.max.into();
        game_format.max_upload_slots = self.upload.slots.into();
        game_format.minimum_latency_in_ticks = self.minimum_latency;
        game_format.minimum_segment_size = self.segment_size.size.min;
        game_format.maximum_segment_size = self.segment_size.size.max;
        game_format.minimum_segment_size_peer_count = self.segment_size.peer_count.min;
        game_format.maximum_segment_size_peer_count = self.segment_size.peer_count.max;

        Ok(())
    }
}
