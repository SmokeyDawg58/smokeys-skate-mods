//! EA Pathfinder 5.3 interactive-music data (`path\5.03.00-sk8`): the `.mpf` map files
//! (`PATHFINDHEADER`, magic `PFDx`) that `PATH_addmapfile` 0x829bb2b0 points its
//! `PATHFINDERSTATE` into, and the `*_Stream.mus` stream files (`PATHMUSHEADER`, then
//! `PATHNODEOFFSETS` from byte 40) that `PathTrackRWAudioCoreStream::StreamCache` caches.
//!
//! The Xbox 360 compiler allocates bit-fields from the least significant bit of a big-endian
//! word, so every bit-field below is `(word >> lsb) & mask` of the word read big-endian.

use std::sync::Arc;

/// `'PFDx'`.
pub const PATHFINDER_ID: u32 = 0x5046_4478;
/// `sizeof(PATHFINDNODE)`; the node's `PATHFINDBRANCH`es follow it.
pub const NODE_SIZE: usize = 16;
/// `sizeof(PATHEVENT)`; the event's `PATHACTION`s follow it.
pub const EVENT_HEADER_SIZE: usize = 20;
pub const ACTION_SIZE: usize = 12;
/// `sizeof(PATHMUSHEADER)`.
pub const MUS_HEADER_SIZE: usize = 40;
/// `sizeof(PATHNODEOFFSETS)`.
pub const NODE_OFFSETS_SIZE: usize = 28;

fn be32(bytes: &[u8], offset: usize) -> u32 {
    bytes.get(offset..offset + 4).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn be16(bytes: &[u8], offset: usize) -> u16 {
    bytes.get(offset..offset + 2).map_or(0, |b| u16::from_be_bytes([b[0], b[1]]))
}

/// A signed bit-field.
#[must_use]
pub fn sbits(word: u32, lsb: u32, width: u32) -> i32 {
    ((word << (32 - lsb - width)) as i32) >> (32 - width)
}

/// An unsigned bit-field.
#[must_use]
pub fn ubits(word: u32, lsb: u32, width: u32) -> u32 {
    (word >> lsb) & ((1u32 << width) - 1)
}

/// A loaded `.mpf` map file (`PATHFINDHEADER` and the tables its offsets point at).
#[derive(Debug, Clone)]
pub struct MapFile {
    bytes: Arc<[u8]>,
}

impl MapFile {
    /// The header checks of `PATH_addmapfile`: the `PFDx` id and version 5.3.
    ///
    /// # Errors
    /// When the file is shorter than its header, has another id or another version.
    pub fn parse(bytes: Arc<[u8]>) -> Result<Self, String> {
        if bytes.len() < 0x48 {
            return Err("pathfinder map file shorter than its header".into());
        }
        if be32(&bytes, 0) != PATHFINDER_ID {
            return Err("PATH_addmapfile - PATHFINDER MAP FILE ID IS INCORRECT.".into());
        }
        if bytes[4] != 5 || bytes[5] != 3 {
            return Err(format!("LIBRARY IS VERSION 5.3, GENERATED DATA IS VERSION {}.{}.", bytes[4], bytes[5]));
        }
        Ok(Self { bytes })
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn generate_id(&self) -> u16 {
        be16(&self.bytes, 0xA)
    }

    #[must_use]
    pub fn project_id(&self) -> u8 {
        self.bytes[0xC]
    }

    #[must_use]
    pub fn num_tracks(&self) -> u8 {
        self.bytes[0xD]
    }

    #[must_use]
    pub fn num_events(&self) -> u8 {
        self.bytes[0xF]
    }

    #[must_use]
    pub fn num_named_vars(&self) -> u8 {
        self.bytes[0x11]
    }

    #[must_use]
    pub fn num_nodes(&self) -> u16 {
        be16(&self.bytes, 0x12)
    }

    fn table(&self, field: usize) -> usize {
        be32(&self.bytes, field) as usize
    }

    /// `PATHI_getnode` 0x829ba560: the node at word offset `pnodeoffsets[node]`; `None` for a
    /// negative index or one past `numnodes` (the native check is `node > numnodes`).
    #[must_use]
    pub fn node(&self, node: i32) -> Option<Node<'_>> {
        if node < 0 || node > i32::from(self.num_nodes()) {
            return None;
        }
        let offset = 4 * usize::from(be16(&self.bytes, self.table(0x14) + 2 * node as usize));
        self.bytes.get(offset..offset + NODE_SIZE).map(|_| Node { bytes: &self.bytes, offset })
    }

    /// The event template at `peventoffsets[index]`.
    #[must_use]
    pub fn event_template(&self, index: usize) -> Option<&[u8]> {
        let offset = 4 * usize::from(be16(&self.bytes, self.table(0x1C) + 2 * index));
        let numactions = usize::from(*self.bytes.get(offset + 15)?);
        self.bytes.get(offset..offset + EVENT_HEADER_SIZE + ACTION_SIZE * numactions)
    }

    /// The initial `PATHNAMEDVAR`s (name, value).
    #[must_use]
    pub fn named_vars(&self) -> Vec<NamedVar> {
        let base = self.table(0x24);
        (0..usize::from(self.num_named_vars()))
            .map(|i| {
                let offset = base + 20 * i;
                let mut name = [0u8; 16];
                if let Some(bytes) = self.bytes.get(offset..offset + 16) {
                    name.copy_from_slice(bytes);
                }
                NamedVar { name, value: be32(&self.bytes, offset + 16) as i32 }
            })
            .collect()
    }

    /// `prouters` (word `index`).
    #[must_use]
    pub fn router(&self, index: usize) -> i32 {
        be32(&self.bytes, self.table(0x28) + 4 * index) as i32
    }

    /// A word of the map file (the routers address `pmap` by word).
    #[must_use]
    pub fn word(&self, index: usize) -> u32 {
        be32(&self.bytes, 4 * index)
    }

    /// `ptrackoffsets[track]` resolved to its `PATHTRACKINFO`, for `track < numtracks`.
    #[must_use]
    pub fn track_info(&self, track: u8) -> Option<TrackInfo> {
        if track >= self.num_tracks() {
            return None;
        }
        let offset = 4 * be32(&self.bytes, self.table(0x2C) + 4 * usize::from(track)) as usize;
        let b = self.bytes.get(offset..offset + 20)?;
        Some(TrackInfo {
            starting_sample: be32(b, 0),
            num_subbanks: be16(b, 4),
            purge_mode: be16(b, 6),
            mus_checksum: be32(b, 8),
            max_aram: be32(b, 12),
            max_mram: be32(b, 16),
        })
    }

    /// `psampleoffsets[index]` (`PATHFINDSAMPLE`).
    #[must_use]
    pub fn sample(&self, index: usize) -> Sample {
        let offset = self.table(0x34) + 8 * index;
        Sample { offset: be32(&self.bytes, offset), duration: be32(&self.bytes, offset + 4) }
    }

    /// `PATHI_getevent` 0x829b6fd0: the last event whose `eventID` matches under `mask`.
    #[must_use]
    pub fn find_event(&self, event_id: u32, mask: u32) -> Option<&[u8]> {
        (0..usize::from(self.num_events())).rev().find_map(|index| {
            let event = self.event_template(index)?;
            ((be32(event, 12) >> 8) & mask == event_id & mask).then_some(event)
        })
    }
}

/// `PATHNAMEDVAR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedVar {
    pub name: [u8; 16],
    pub value: i32,
}

/// `PATHTRACKINFO`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrackInfo {
    pub starting_sample: u32,
    pub num_subbanks: u16,
    pub purge_mode: u16,
    pub mus_checksum: u32,
    pub max_aram: u32,
    pub max_mram: u32,
}

/// `PATHFINDSAMPLE`: the sample's 128-byte stream offset and its duration in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sample {
    pub offset: u32,
    pub duration: u32,
}

/// A `PATHFINDNODE` and its branches.
#[derive(Debug, Clone, Copy)]
pub struct Node<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Node<'_> {
    #[must_use]
    pub fn word(&self, index: usize) -> u32 {
        be32(self.bytes, self.offset + 4 * index)
    }

    /// `repeat:5`.
    #[must_use]
    pub fn repeat(&self) -> i32 {
        sbits(self.word(0), 0, 5)
    }

    /// `sectionID:6`.
    #[must_use]
    pub fn section(&self) -> u32 {
        ubits(self.word(0), 5, 6)
    }

    /// `trackID:5`.
    #[must_use]
    pub fn track(&self) -> u32 {
        ubits(self.word(0), 11, 5)
    }

    /// `index:16`: the 1-based sample, or a logic node (0 entry, -1 repeat, -2 random control,
    /// -3 event).
    #[must_use]
    pub fn index(&self) -> i32 {
        sbits(self.word(0), 16, 16)
    }

    /// `bars:8`.
    #[must_use]
    pub fn bars(&self) -> u32 {
        ubits(self.word(1), 0, 8)
    }

    /// `beats:4`.
    #[must_use]
    pub fn beats(&self) -> u32 {
        ubits(self.word(1), 8, 4)
    }

    /// `controller:3`.
    #[must_use]
    pub fn controller(&self) -> u32 {
        ubits(self.word(1), 12, 3)
    }

    /// `numbranches:5`.
    #[must_use]
    pub fn num_branches(&self) -> u32 {
        ubits(self.word(1), 15, 5)
    }

    /// `routerID:12`.
    #[must_use]
    pub fn router(&self) -> u32 {
        self.word(1) >> 20
    }

    /// `channelbranching:1`.
    #[must_use]
    pub fn channel_branching(&self) -> bool {
        self.word(2) & 2 != 0
    }

    /// `synch:2`.
    #[must_use]
    pub fn synch(&self) -> u32 {
        ubits(self.word(2), 2, 2)
    }

    /// `partID:16`.
    #[must_use]
    pub fn part(&self) -> u16 {
        (self.word(2) >> 16) as u16
    }

    /// `extra` (`PATHNODEEXTRA`).
    #[must_use]
    pub fn extra(&self) -> u32 {
        self.word(3)
    }

    /// Branch `index`: (`controlmin`, `controlmax`, `dstnode`).
    #[must_use]
    pub fn branch(&self, index: usize) -> (i8, i8, i16) {
        let offset = self.offset + NODE_SIZE + 4 * index;
        let b = &self.bytes[offset..offset + 4];
        (b[0] as i8, b[1] as i8, i16::from_be_bytes([b[2], b[3]]))
    }
}

/// `PATHNODEOFFSETS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeOffsets {
    pub node_checksum: u32,
    pub index: u16,
    pub sub_index: u16,
    /// In 16-byte units: the sample's EAAC header in the stream cache.
    pub snr_offset: u32,
    /// In 128-byte units: the sample's chunks in the stream file.
    pub sns_offset: u32,
    pub snr_bytes: u32,
    pub sns_bytes: u32,
}

/// `PathTrackRWAudioCoreStream::FindSampleOffsets` 0x829ba4d0 over the stream cache (the first
/// `maxaram` bytes of the `.mus` file): the first entry from byte 40 whose `snsoffset` is at
/// least `offset` (its `snroffset` when `snsoffset` is 0). The native loop has no end check; a
/// cache without a match yields `None` here.
#[must_use]
pub fn find_sample_offsets(cache: &[u8], offset: i32) -> Option<NodeOffsets> {
    let mut at = MUS_HEADER_SIZE;
    while at + NODE_OFFSETS_SIZE <= cache.len() {
        let sns = be32(cache, at + 12);
        let found = if sns == 0 { offset <= be32(cache, at + 8) as i32 } else { offset <= sns as i32 };
        if found {
            return Some(NodeOffsets {
                node_checksum: be32(cache, at),
                index: be16(cache, at + 4),
                sub_index: be16(cache, at + 6),
                snr_offset: be32(cache, at + 8),
                sns_offset: sns,
                snr_bytes: be32(cache, at + 16),
                sns_bytes: be32(cache, at + 20),
            });
        }
        at += NODE_OFFSETS_SIZE;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_fields_are_lsb_first() {
        // index 0x0001, trackID 0, sectionID 1, repeat 0 (game.mpf node 1 word 0 shape).
        let word = 0x0001_0020;
        assert_eq!(sbits(word, 16, 16), 1);
        assert_eq!(ubits(word, 5, 6), 1);
        assert_eq!(sbits(0xFFFF_0000, 16, 16), -1);
        assert_eq!(sbits(0x1F, 0, 5), -1);
    }

    #[test]
    fn sample_offsets_follow_the_sns_offset() {
        let mut cache = vec![0u8; MUS_HEADER_SIZE + 2 * NODE_OFFSETS_SIZE];
        let entry = |c: &mut Vec<u8>, i: usize, snr: u32, sns: u32| {
            let at = MUS_HEADER_SIZE + i * NODE_OFFSETS_SIZE;
            c[at + 8..at + 12].copy_from_slice(&snr.to_be_bytes());
            c[at + 12..at + 16].copy_from_slice(&sns.to_be_bytes());
        };
        entry(&mut cache, 0, 0xD80, 0x2A6);
        entry(&mut cache, 1, 0xD81, 0x4EA);
        assert_eq!(find_sample_offsets(&cache, 0x2A6).map(|o| o.snr_offset), Some(0xD80));
        assert_eq!(find_sample_offsets(&cache, 0x4EA).map(|o| o.snr_offset), Some(0xD81));
        assert_eq!(find_sample_offsets(&cache, 0x4EB), None);
    }
}
