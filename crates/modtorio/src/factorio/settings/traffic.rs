use super::{GameFormatConversion, Limit, Range, ServerSettingsGameFormat};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Upload {
    pub max: Limit,
    pub slots: Limit,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct SegmentSize {
    pub size: Range,
    pub peer_count: Range,
}

#[derive(Deserialize, Serialize, Debug, Default, PartialEq)]
pub struct Traffic {
    pub upload: Upload,
    pub minimum_latency: u64,
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
