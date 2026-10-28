//! `pegasus::tAIPathData`, TOC type `0x00eb0014`: the recorded AI paths one arena carries.
//!
//! Pointers are self-relative offsets that `tAIPathData::Fixup` 0x829c8428 and
//! `tAIPath::Fixup` 0x829c7e40 turn into addresses: the path array is relative to the
//! data object; nodes, extended node data and branch groups to their `Path`; a node's
//! extended data to the node; a group's branches to the group.

use std::fmt;

pub const AI_PATH_DATA_TYPE_ID: u32 = 0x00eb_0014;
const PATH_SIZE: usize = 96;
const NODE_SIZE: usize = 44;
const EXTENDED_SIZE: usize = 40;
const BRANCH_GROUP_SIZE: usize = 12;
const BRANCH_SIZE: usize = 24;

/// `AIPath::AIPathID`: byte 0 is the progression branch, zero for ambient paths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AiPathId(pub [u8; 16]);

impl AiPathId {
    /// `IsAmbientID` 0x824b15f8.
    #[must_use]
    pub const fn is_ambient(&self) -> bool {
        self.0[0] == 0
    }

    /// The big-endian word at `offset`, as the filter and hash compare it.
    #[must_use]
    pub fn word(&self, offset: usize) -> u32 {
        u32::from_be_bytes(self.0[offset..offset + 4].try_into().expect("four ID bytes"))
    }
}

/// `AIPath::ExtendedNodeData` (40 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AiPathExtendedNodeData {
    pub trajectory_start_position: [f32; 3],
    pub trajectory_start_velocity: [f32; 3],
    pub trajectory_offset: [f32; 3],
    pub trick_scorable: i16,
    pub air_spin_180_count: i8,
    pub bit_flags: u8,
}

/// `AIPath::Node` (44 bytes). The orientations are `CompressedQuaternion` bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AiPathNode {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub board_orientation: [u8; 4],
    pub skater_orientation: [u8; 4],
    pub extended: Option<AiPathExtendedNodeData>,
    pub frame_count_since_last_node: u8,
    pub width_left: u8,
    pub width_right: u8,
    pub event_type: u8,
    pub bit_flags: u8,
    pub extra_data: [u8; 3],
}

/// `AIPath::Branch` (24 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AiPathBranch {
    pub destination: AiPathId,
    pub destination_node_index: i32,
    pub source_node_ratio: f32,
}

/// `AIPath::BranchGroup` (12 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct AiPathBranchGroup {
    pub branches: Vec<AiPathBranch>,
    pub source_node_index: i32,
}

/// `AIPath::Path` (96 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct AiPath {
    pub bbox_min: [f32; 4],
    pub bbox_max: [f32; 4],
    pub id: AiPathId,
    pub nodes: Vec<AiPathNode>,
    /// `m_pExtraNodeData` was set; each node reaches its own entry through its pointer.
    pub has_extra_node_data: bool,
    pub branch_groups: Vec<AiPathBranchGroup>,
    pub bit_flags: u32,
    pub allowed_skaters: u64,
    pub skill_level: u32,
    pub extra_data: [i32; 3],
}

/// `pegasus::tAIPathData`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AiPathData {
    pub paths: Vec<AiPath>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiPathError {
    Truncated { offset: usize, needed: usize, len: usize },
    PointerOutsideObject { offset: usize, pointer: u32 },
    NegativeCount { offset: usize, count: i32 },
}

impl fmt::Display for AiPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { offset, needed, len } => {
                write!(f, "AI path data truncated: {needed} bytes at {offset:#x} of {len:#x}")
            }
            Self::PointerOutsideObject { offset, pointer } => {
                write!(f, "AI path pointer {pointer:#x} at {offset:#x} leaves the object")
            }
            Self::NegativeCount { offset, count } => {
                write!(f, "AI path count {count} at {offset:#x} is negative")
            }
        }
    }
}

impl std::error::Error for AiPathError {}

fn span(bytes: &[u8], offset: usize, len: usize) -> Result<&[u8], AiPathError> {
    offset
        .checked_add(len)
        .and_then(|end| bytes.get(offset..end))
        .ok_or(AiPathError::Truncated { offset, needed: len, len: bytes.len() })
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32, AiPathError> {
    Ok(u32::from_be_bytes(span(bytes, offset, 4)?.try_into().expect("four bytes")))
}

fn be_i32(bytes: &[u8], offset: usize) -> Result<i32, AiPathError> {
    Ok(be_u32(bytes, offset)? as i32)
}

fn be_f32(bytes: &[u8], offset: usize) -> Result<f32, AiPathError> {
    Ok(f32::from_bits(be_u32(bytes, offset)?))
}

fn be_f32s<const N: usize>(bytes: &[u8], offset: usize) -> Result<[f32; N], AiPathError> {
    let mut out = [0.; N];
    for (index, lane) in out.iter_mut().enumerate() {
        *lane = be_f32(bytes, offset + index * 4)?;
    }
    Ok(out)
}

fn id(bytes: &[u8], offset: usize) -> Result<AiPathId, AiPathError> {
    Ok(AiPathId(span(bytes, offset, 16)?.try_into().expect("sixteen bytes")))
}

/// A Fixup pointer stored at `field`, relative to `base`; `None` for null.
fn pointer(bytes: &[u8], base: usize, field: usize) -> Result<Option<usize>, AiPathError> {
    let raw = be_u32(bytes, field)?;
    if raw == 0 {
        return Ok(None);
    }
    base.checked_add(raw as usize)
        .filter(|target| *target < bytes.len())
        .map(Some)
        .ok_or(AiPathError::PointerOutsideObject { offset: field, pointer: raw })
}

fn count(bytes: &[u8], offset: usize) -> Result<usize, AiPathError> {
    let raw = be_i32(bytes, offset)?;
    usize::try_from(raw).map_err(|_| AiPathError::NegativeCount { offset, count: raw })
}

fn extended(bytes: &[u8], at: usize) -> Result<AiPathExtendedNodeData, AiPathError> {
    let raw = span(bytes, at, EXTENDED_SIZE)?;
    Ok(AiPathExtendedNodeData {
        trajectory_start_position: be_f32s(raw, 0)?,
        trajectory_start_velocity: be_f32s(raw, 12)?,
        trajectory_offset: be_f32s(raw, 24)?,
        trick_scorable: i16::from_be_bytes([raw[36], raw[37]]),
        air_spin_180_count: raw[38] as i8,
        bit_flags: raw[39],
    })
}

fn node(bytes: &[u8], at: usize) -> Result<AiPathNode, AiPathError> {
    let raw = span(bytes, at, NODE_SIZE)?;
    let extended = pointer(bytes, at, at + 0x20)?.map(|target| extended(bytes, target)).transpose()?;
    Ok(AiPathNode {
        position: be_f32s(raw, 0)?,
        direction: be_f32s(raw, 12)?,
        board_orientation: raw[24..28].try_into().expect("four bytes"),
        skater_orientation: raw[28..32].try_into().expect("four bytes"),
        extended,
        frame_count_since_last_node: raw[36],
        width_left: raw[37],
        width_right: raw[38],
        event_type: raw[39],
        bit_flags: raw[40],
        extra_data: [raw[41], raw[42], raw[43]],
    })
}

fn branch_group(bytes: &[u8], at: usize) -> Result<AiPathBranchGroup, AiPathError> {
    let length = be_u32(bytes, at + 4)? as usize;
    let mut branches = Vec::with_capacity(length.min(64));
    if let Some(first) = pointer(bytes, at, at)? {
        for index in 0..length {
            let entry = first + index * BRANCH_SIZE;
            span(bytes, entry, BRANCH_SIZE)?;
            branches.push(AiPathBranch {
                destination: id(bytes, entry)?,
                destination_node_index: be_i32(bytes, entry + 16)?,
                source_node_ratio: be_f32(bytes, entry + 20)?,
            });
        }
    }
    Ok(AiPathBranchGroup { branches, source_node_index: be_i32(bytes, at + 8)? })
}

fn path(bytes: &[u8], at: usize) -> Result<AiPath, AiPathError> {
    span(bytes, at, PATH_SIZE)?;
    let node_count = count(bytes, at + 0x34)?;
    let mut nodes = Vec::with_capacity(node_count.min(1 << 16));
    if let Some(first) = pointer(bytes, at, at + 0x30)? {
        for index in 0..node_count {
            nodes.push(node(bytes, first + index * NODE_SIZE)?);
        }
    }
    let group_count = count(bytes, at + 0x40)?;
    let mut branch_groups = Vec::with_capacity(group_count.min(1 << 12));
    if let Some(first) = pointer(bytes, at, at + 0x3c)? {
        for index in 0..group_count {
            branch_groups.push(branch_group(bytes, first + index * BRANCH_GROUP_SIZE)?);
        }
    }
    let allowed = span(bytes, at + 0x48, 8)?;
    Ok(AiPath {
        bbox_min: be_f32s(bytes, at)?,
        bbox_max: be_f32s(bytes, at + 0x10)?,
        id: id(bytes, at + 0x20)?,
        nodes,
        has_extra_node_data: be_u32(bytes, at + 0x38)? != 0,
        branch_groups,
        bit_flags: be_u32(bytes, at + 0x44)?,
        allowed_skaters: u64::from_be_bytes(allowed.try_into().expect("eight bytes")),
        skill_level: be_u32(bytes, at + 0x50)?,
        extra_data: [be_i32(bytes, at + 0x54)?, be_i32(bytes, at + 0x58)?, be_i32(bytes, at + 0x5c)?],
    })
}

impl AiPathData {
    /// Parses one `tAIPathData` object as stored in its arena (before Fixup).
    ///
    /// # Errors
    ///
    /// Returns [`AiPathError`] for truncated records, negative counts or pointers that leave
    /// the object.
    pub fn parse(bytes: &[u8]) -> Result<Self, AiPathError> {
        let path_count = be_u32(bytes, 0)? as usize;
        let mut paths = Vec::with_capacity(path_count.min(1 << 12));
        if let Some(first) = pointer(bytes, 0, 4)? {
            for index in 0..path_count {
                paths.push(path(bytes, first + index * PATH_SIZE)?);
            }
        }
        Ok(Self { paths })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn self_relative_pointers_reach_nodes_extended_data_and_branches() {
        // Data header, one path at 0x10, two nodes at 0x70, extended data at 0xc8,
        // one branch group at 0xf0 and its branch at 0x100.
        let mut bytes = vec![0u8; 0x118];
        put_u32(&mut bytes, 0, 1);
        put_u32(&mut bytes, 4, 0x10);
        let path = 0x10;
        put_u32(&mut bytes, path, 1.0f32.to_bits());
        bytes[path + 0x20] = 0;
        bytes[path + 0x24] = 0x80;
        put_u32(&mut bytes, path + 0x30, 0x60);
        put_u32(&mut bytes, path + 0x34, 2);
        put_u32(&mut bytes, path + 0x38, 0xb8);
        put_u32(&mut bytes, path + 0x3c, 0xe0);
        put_u32(&mut bytes, path + 0x40, 1);
        put_u32(&mut bytes, path + 0x44, 7);
        let second = 0x70 + NODE_SIZE;
        put_u32(&mut bytes, second, 2.5f32.to_bits());
        put_u32(&mut bytes, second + 0x20, (0xc8 - second) as u32);
        bytes[second + 39] = 1;
        bytes[0xc8 + 36..0xc8 + 38].copy_from_slice(&(-3i16).to_be_bytes());
        bytes[0xc8 + 38] = 2;
        put_u32(&mut bytes, 0xf0, 0x10);
        put_u32(&mut bytes, 0xf4, 1);
        put_u32(&mut bytes, 0xf8, 1);
        bytes[0x100] = 4;
        put_u32(&mut bytes, 0x110, 5);
        put_u32(&mut bytes, 0x114, 0.5f32.to_bits());
        let data = AiPathData::parse(&bytes).unwrap();
        let path = &data.paths[0];
        assert!(path.id.is_ambient());
        assert_eq!(path.id.word(4), 0x8000_0000);
        assert_eq!(path.nodes.len(), 2);
        assert!(path.nodes[0].extended.is_none());
        assert_eq!(path.nodes[1].position[0], 2.5);
        assert_eq!(path.nodes[1].event_type, 1);
        let extended = path.nodes[1].extended.unwrap();
        assert_eq!((extended.trick_scorable, extended.air_spin_180_count), (-3, 2));
        assert_eq!(path.branch_groups[0].source_node_index, 1);
        let branch = path.branch_groups[0].branches[0];
        assert_eq!((branch.destination.0[0], branch.destination_node_index), (4, 5));
        assert_eq!(branch.source_node_ratio, 0.5);
        assert_eq!(path.bit_flags, 7);
    }

    #[test]
    fn a_pointer_past_the_object_is_refused() {
        let mut bytes = vec![0u8; 8];
        put_u32(&mut bytes, 0, 1);
        put_u32(&mut bytes, 4, 0x40);
        assert!(matches!(AiPathData::parse(&bytes), Err(AiPathError::PointerOutsideObject { .. })));
    }
}
