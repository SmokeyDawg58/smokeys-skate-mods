//! Mesh `LineTest` specialization, 0x827c01a0..0x827c0428.
//! Native KD/range/unit order feeds complete modern GP construction and the
//! triangle wrapper. Every hit increments the count; strict nearest ties keep
//! the first occurrence. Collection/job publication belongs to the caller.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "ground query awaits the live World query owner")
)]

use std::fmt;

use skate2_formats::collision::{
    ClusteredMesh, CollisionError,
    line_cursor::{MeshLineCursorError, MeshLineRangeCursor},
};

use crate::{
    animation::AnimationMatrix,
    collision_triangle_line::{
        CollisionTriangleLineError, CollisionTriangleLineInput, CollisionTriangleLineOutput,
        try_triangle_line,
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_ground_triangle_topology::{
        AssemblyGroundNumeric, AssemblyGroundQueryBox, UnitTopologyError,
        build_new_unit_gp_with_numeric,
    },
    skeleton_root_transforms::SkeletonRootVector,
};

#[cfg(test)]
#[path = "collision_mesh_line_tests.rs"]
pub(crate) mod tests;

/// Borrowed actual collection occurrence. Both transforms require proven full
/// lanes; an upstream unknown W must not be filled to satisfy this contract.
pub(crate) struct CollisionMeshOccurrence<'a, O: Clone> {
    pub(crate) mesh: &'a ClusteredMesh,
    pub(crate) bytes: &'a [u8],
    pub(crate) forward: &'a AnimationMatrix,
    pub(crate) inverse: &'a AnimationMatrix,
    pub(crate) owner: Option<O>,
    pub(crate) slot: usize,
}

pub(crate) struct CollisionMeshLineInput {
    pub(crate) start: SkeletonRootVector,
    pub(crate) end: SkeletonRootVector,
    /// Original world delta, distinct from the transformed endpoint pair.
    pub(crate) delta: SkeletonRootVector,
    pub(crate) model_box: AssemblyGroundQueryBox,
    pub(crate) line_fatness: f32,
}

/// Diagnostic provenance follows the actual encounter; it never sorts or
/// breaks ties. Owner identity remains the borrowed occurrence's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CollisionMeshLineProvenance {
    pub(crate) occurrence_slot: usize,
    pub(crate) cluster_index: usize,
    pub(crate) unit_byte_offset: usize,
    pub(crate) primitive_ordinal: usize,
}

#[derive(Debug, PartialEq)]
pub(crate) struct CollisionMeshLineNearest<O> {
    pub(crate) position: SkeletonRootVector,
    pub(crate) normal: SkeletonRootVector,
    pub(crate) line_parameter: f32,
    pub(crate) surface_id: u16,
    pub(crate) owner: Option<O>,
    pub(crate) provenance: Option<CollisionMeshLineProvenance>,
    pub(crate) hit_count: u32,
}

impl<O> CollisionMeshLineNearest<O> {
    /// Local functor defaults at 0x827c1a68..0x827c1ab8. This is caller-owned
    /// scratch, not automatic publication into a retained batch result.
    pub(crate) fn new() -> Self {
        Self {
            position: SkeletonRootVector::known([0.0; 4]),
            normal: SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            line_parameter: f32::MAX,
            surface_id: 0,
            owner: None,
            provenance: None,
            hit_count: 0,
        }
    }
}

#[derive(Debug)]
pub(crate) enum CollisionMeshLineUnitError<E> {
    Topology(UnitTopologyError),
    Triangle(CollisionTriangleLineError<E>),
}

impl<E: fmt::Debug> fmt::Display for CollisionMeshLineUnitError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Topology(error) => write!(f, "line GP construction: {error:?}"),
            Self::Triangle(error) => write!(f, "{error}"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum CollisionMeshLineError<E> {
    Decode(CollisionError),
    Cursor(MeshLineCursorError<CollisionMeshLineUnitError<E>>),
}

impl<E: fmt::Debug> fmt::Display for CollisionMeshLineError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "mesh line KD decode: {error:?}"),
            Self::Cursor(error) => write!(f, "mesh line unit traversal: {error}"),
        }
    }
}

impl<E: fmt::Debug> std::error::Error for CollisionMeshLineError<E> {}

/// 0x827c01a0. The same numeric object serves modern GP and thin/wrapper work;
/// its two trait implementations must describe one chosen reduction/estimate
/// policy. The existing host implementations have different reduction orders
/// and must not be silently mixed. KD retains its documented formats backend.
pub(crate) fn try_mesh_line<O: Clone, N: AssemblyGroundNumeric + ReckoningNumeric>(
    occurrence: &CollisionMeshOccurrence<'_, O>,
    input: &CollisionMeshLineInput,
    nearest: &mut CollisionMeshLineNearest<O>,
    numeric: &mut N,
) -> Result<(), CollisionMeshLineError<N::Error>> {
    let profiling = crate::collision_line_collection::profile::enabled();
    let mesh_start = profiling.then(std::time::Instant::now);
    let mut units = 0;
    let mut triangles = 0;
    let mut gp_time = std::time::Duration::ZERO;
    let mut intersection_time = std::time::Duration::ZERO;
    let local_start = transform_point(occurrence.inverse, input.start);
    let local_end = transform_point(occurrence.inverse, input.end);
    let tree = occurrence
        .mesh
        .kd_tree(occurrence.bytes)
        .map_err(CollisionMeshLineError::Decode)?;
    let mut query = tree
        .line_range_query(
            occurrence.bytes,
            local_start.xyz,
            local_end.xyz,
            input.line_fatness,
        )
        .map_err(CollisionMeshLineError::Decode)?;
    let mut cursor = MeshLineRangeCursor::new();
    let mut triangle_output = CollisionTriangleLineOutput::default();
    loop {
        let visited = cursor
            .visit_next_chunk(occurrence.mesh, occurrence.bytes, &mut query, |selected| {
                if profiling {
                    units += 1;
                }
                let gp_start = profiling.then(std::time::Instant::now);
                let batch = build_new_unit_gp_with_numeric(
                    occurrence.mesh,
                    selected.cluster_index,
                    selected.cluster,
                    selected.unit,
                    input.model_box,
                    *occurrence.forward,
                    numeric,
                )
                .map_err(CollisionMeshLineUnitError::Topology)?;
                if let Some(start) = gp_start {
                    gp_time += start.elapsed();
                }
                for (primitive_ordinal, triangle) in batch.iter().enumerate() {
                    if profiling {
                        triangles += 1;
                    }
                    let intersection_start = profiling.then(std::time::Instant::now);
                    let triangle_input = CollisionTriangleLineInput {
                        start: input.start,
                        delta: input.delta,
                        vertices: triangle.points_full4().map(SkeletonRootVector::known),
                    };
                    let hit = try_triangle_line(
                        &triangle_input,
                        input.line_fatness,
                        0.0,
                        &mut triangle_output,
                        numeric,
                    )
                    .map_err(CollisionMeshLineUnitError::Triangle)?;
                    if let Some(start) = intersection_start {
                        intersection_time += start.elapsed();
                    }
                    if !hit {
                        continue;
                    }

                    let parameter = triangle_output
                        .line_parameter
                        .expect("a successful source triangle wrote its parameter");
                    let lower = if -parameter >= 0.0 { 0.0 } else { parameter };
                    let clamped = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
                    let [surface_high, surface_low, _, _] = triangle.user_tag().to_be_bytes();
                    let surface = u16::from_be_bytes([surface_high, surface_low]);
                    let owner = occurrence.owner.clone();
                    let normal = SkeletonRootVector::known(triangle.normal_full4());
                    if clamped < nearest.line_parameter {
                        nearest.position = triangle_output
                            .position
                            .expect("a successful source triangle wrote its position");
                        nearest.normal = normal;
                        nearest.surface_id = surface;
                        nearest.line_parameter = clamped;
                        nearest.owner = owner;
                        nearest.provenance = Some(CollisionMeshLineProvenance {
                            occurrence_slot: occurrence.slot,
                            cluster_index: selected.cluster_index,
                            unit_byte_offset: selected.unit_byte_offset,
                            primitive_ordinal,
                        });
                    }
                    nearest.hit_count = nearest.hit_count.wrapping_add(1);
                }
                Ok(())
            })
            .map_err(CollisionMeshLineError::Cursor)?;
        if !visited {
            if let Some(start) = mesh_start {
                crate::collision_line_collection::profile::mesh(
                    units,
                    triangles,
                    gp_time,
                    intersection_time,
                    start.elapsed(),
                );
            }
            return Ok(());
        }
    }
}

fn transform_point(matrix: &AnimationMatrix, point: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector::known(std::array::from_fn(|lane| {
        let x = matrix[lane].mul_add(point.xyz[0], matrix[12 + lane]);
        let y = matrix[4 + lane].mul_add(point.xyz[1], x);
        matrix[8 + lane].mul_add(point.xyz[2], y)
    }))
}
