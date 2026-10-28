//! Gameplay mesh unit traversal, `LineTest` 0x827c01a0..0x827c0428.
//! The caller retains the actual mesh bytes and existing KD query. This
//! cursor publishes only the native range/chunk bookkeeping.

use super::{
    ClusteredMesh, CollisionError, KdTreeLineRangeQuery, MeshUnit, VertexCompression, be_i32,
    be_u16, be_u32, offset_slice, parse_unit, parse_vertex, require_size,
};
use std::fmt;

/// Iterator fields published by initial range selection or a completed chunk.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MeshLineCursorProgress {
    pub cluster_index: u32,
    pub unit_byte_offset: u32,
    pub processed: u32,
    pub total: u32,
}

/// A single decoded unit, borrowed only for its visitor invocation.
#[derive(Debug)]
pub struct MeshLineUnit<'a> {
    pub cluster_index: usize,
    pub unit_byte_offset: usize,
    pub encoded_size: usize,
    pub cluster: &'a MeshLineClusterView<'a>,
    pub unit: &'a MeshUnit,
}

/// Borrowed gameplay cluster. Unused normal, vertex and unit tails are never
/// decoded or validated merely because a KD range selects this cluster.
#[derive(Debug)]
pub struct MeshLineClusterView<'a> {
    bytes: &'a [u8],
    cluster_index: usize,
    unit_data_start: u16,
    pub unit_data_size: u16,
    vertex_count: u8,
    compression: u8,
}

impl MeshLineClusterView<'_> {
    /// Decodes one actually requested vertex using the existing native
    /// compression graph. No other vertex or serialized normal is read.
    ///
    /// # Errors
    /// Refuses an out-of-range index, unknown selected compression mode or
    /// a truncated selected record/origin. These are checked host boundaries.
    pub fn vertex_position(
        &self,
        mesh: &ClusteredMesh,
        index: u8,
    ) -> Result<[f32; 3], CollisionError> {
        if index >= self.vertex_count {
            return Err(CollisionError::VertexIndexOutsideCluster {
                cluster: self.cluster_index,
            });
        }
        let (compression, start, stride) = match self.compression {
            0 => (VertexCompression::Uncompressed, 16, 16),
            1 => (VertexCompression::Unsigned16, 28, 6),
            2 => (VertexCompression::Signed32, 16, 12),
            value => return Err(CollisionError::UnknownVertexCompression { value }),
        };
        let origin = if compression == VertexCompression::Unsigned16 {
            [
                be_i32(self.bytes, 16)?,
                be_i32(self.bytes, 20)?,
                be_i32(self.bytes, 24)?,
            ]
        } else {
            [0; 3]
        };
        let start = start + usize::from(index) * stride;
        let selected = self
            .bytes
            .get(start..)
            .ok_or(CollisionError::RangeOutsideObject {
                start,
                end: start,
                object_size: self.bytes.len(),
            })?;
        Ok(parse_vertex(selected, compression, origin)?.position(mesh))
    }

    fn unit_remaining(&self, offset: usize) -> Result<&[u8], CollisionError> {
        if offset > usize::from(self.unit_data_size) {
            return Err(CollisionError::EncodedUnitOffsetNotFound {
                cluster: self.cluster_index,
                offset,
            });
        }
        let base = 16 + usize::from(self.unit_data_start) * 16;
        let start = base + offset;
        // A missing later tail must not reject a complete current record.
        // Conversely, declared unit bounds cannot bleed into another table.
        let end = (base + usize::from(self.unit_data_size)).min(self.bytes.len());
        self.bytes
            .get(start..end)
            .ok_or(CollisionError::RangeOutsideObject {
                start,
                end: start.max(end),
                object_size: self.bytes.len(),
            })
    }
}

/// A malformed visited input or a refusal from actual GP/line construction.
#[derive(Debug)]
pub enum MeshLineCursorError<E> {
    Decode(CollisionError),
    Visitor(E),
}
impl<E: fmt::Display> fmt::Display for MeshLineCursorError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "mesh line cursor: {error}"),
            Self::Visitor(error) => write!(f, "mesh line unit visitor: {error}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for MeshLineCursorError<E> {}

/// Retained gameplay range cursor; retains no mesh allocation or KD traversal.
#[derive(Debug, Default)]
pub struct MeshLineRangeCursor {
    progress: MeshLineCursorProgress,
}
impl MeshLineRangeCursor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn progress(&self) -> MeshLineCursorProgress {
        self.progress
    }

    /// Visits one native chunk and publishes its cursor, returning true.
    /// False means the existing KD query has no further range. Within a range
    /// of N remaining units, the updated-count comparison processes ceil(N/2)
    /// units, then resumes that range on the next call without advancing KD.
    ///
    /// The visitor must complete the unit's GP construction and primitive
    /// tests before returning success, including for a culled/zero-GP unit.
    /// Units and subsequent clusters are decoded only when reached. No
    /// nearest-hit clipping, sorting, deduplication or unit-count truncation
    /// occurs here.
    ///
    /// # Errors
    /// Returns a visited decode error or the visitor's error. Initial new
    /// range publication remains visible on error; local per-unit advancement
    /// does not publish until the chunk completes (0x827c03f4..0x827c03fc).
    /// Earlier visitor effects remain its owner's responsibility. Discard this
    /// cursor and query after an error; native malformed-input retry semantics
    /// are not established.
    pub fn visit_next_chunk<E, F>(
        &mut self,
        mesh: &ClusteredMesh,
        mesh_bytes: &[u8],
        query: &mut KdTreeLineRangeQuery<'_>,
        mut visitor: F,
    ) -> Result<bool, MeshLineCursorError<E>>
    where
        F: FnMut(MeshLineUnit<'_>) -> Result<(), E>,
    {
        if self.progress.processed == self.progress.total {
            let Some(range) = query.next_range().map_err(MeshLineCursorError::Decode)? else {
                return Ok(false);
            };
            let shift = if mesh.maximum_cluster_count <= 0x1_0000 {
                16
            } else {
                20
            };
            // QueryRange's new-range fields precede any selected-cluster read.
            self.progress = MeshLineCursorProgress {
                cluster_index: range.encoded_start >> shift,
                unit_byte_offset: range.encoded_start & ((1_u32 << shift) - 1),
                processed: 0,
                total: range.unit_count,
            };
        }

        let mut current = self.progress;
        let mut chunk_count = 0_u32;
        let mut parsed_index = current.cluster_index;
        let mut cluster = parse_selected_cluster(mesh, mesh_bytes, parsed_index)
            .map_err(MeshLineCursorError::Decode)?;
        loop {
            if parsed_index != current.cluster_index {
                cluster = parse_selected_cluster(mesh, mesh_bytes, current.cluster_index)
                    .map_err(MeshLineCursorError::Decode)?;
                parsed_index = current.cluster_index;
            }
            let cluster_index = usize::try_from(current.cluster_index)
                .map_err(|_| MeshLineCursorError::Decode(CollisionError::IntegerOverflow))?;
            let unit_byte_offset = usize::try_from(current.unit_byte_offset)
                .map_err(|_| MeshLineCursorError::Decode(CollisionError::IntegerOverflow))?;
            let remaining = cluster
                .unit_remaining(unit_byte_offset)
                .map_err(MeshLineCursorError::Decode)?;
            let (unit, encoded_size) = parse_unit(remaining, unit_byte_offset, mesh)
                .map_err(MeshLineCursorError::Decode)?;
            visitor(MeshLineUnit {
                cluster_index,
                unit_byte_offset,
                encoded_size,
                cluster: &cluster,
                unit: &unit,
            })
            .map_err(MeshLineCursorError::Visitor)?;

            current.processed = current.processed.wrapping_add(1);
            chunk_count = chunk_count.wrapping_add(1);
            let encoded_size = u32::try_from(encoded_size)
                .map_err(|_| MeshLineCursorError::Decode(CollisionError::IntegerOverflow))?;
            let next_byte = current.unit_byte_offset.wrapping_add(encoded_size);
            // Native signed32(size_u16 - next_byte - 1) < 0. No dependency on
            // GP count or the cluster's declared number of units.
            let remainder = u32::from(cluster.unit_data_size)
                .wrapping_sub(next_byte)
                .wrapping_sub(1);
            if remainder & 0x8000_0000 != 0 {
                current.unit_byte_offset = 0;
                current.cluster_index = current.cluster_index.wrapping_add(1);
            } else {
                current.unit_byte_offset = next_byte;
            }
            if chunk_count >= current.total.wrapping_sub(current.processed) {
                break;
            }
        }

        self.progress.cluster_index = current.cluster_index;
        self.progress.unit_byte_offset = current.unit_byte_offset;
        self.progress.processed = current.processed;
        Ok(true)
    }
}

fn parse_selected_cluster<'a>(
    mesh: &ClusteredMesh,
    mesh_bytes: &'a [u8],
    index: u32,
) -> Result<MeshLineClusterView<'a>, CollisionError> {
    let index = usize::try_from(index).map_err(|_| CollisionError::IntegerOverflow)?;
    let cluster_count =
        usize::try_from(mesh.cluster_count).map_err(|_| CollisionError::IntegerOverflow)?;
    if index >= cluster_count {
        return Err(CollisionError::EncodedClusterOutsideMesh {
            index,
            cluster_count,
        });
    }
    let table =
        usize::try_from(mesh.cluster_data_offset).map_err(|_| CollisionError::IntegerOverflow)?;
    let entry = index
        .checked_mul(4)
        .and_then(|offset| table.checked_add(offset))
        .ok_or(CollisionError::IntegerOverflow)?;
    let offset = be_u32(mesh_bytes, entry)?;
    let bytes = offset_slice(mesh_bytes, offset)?;
    require_size(bytes, 16)?;
    let total_size = usize::from(be_u16(bytes, 8)?);
    Ok(MeshLineClusterView {
        bytes: &bytes[..total_size.min(bytes.len())],
        cluster_index: index,
        unit_data_start: be_u16(bytes, 4)?,
        unit_data_size: be_u16(bytes, 2)?,
        vertex_count: bytes[10],
        compression: bytes[12],
    })
}

#[cfg(test)]
#[path = "collision_line_cursor_tests.rs"]
mod tests;
