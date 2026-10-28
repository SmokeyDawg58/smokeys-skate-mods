//! Ordered no-spatial-index Collection `LineTest` 0x827c1300 and local job 0x827c19c8.
//! Actual Ground/DMO constructors select this route. Pool ownership, batch
//! acquisition and aggregation belong to the enclosing World query service.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "ground query awaits the live World query owner")
)]

#[path = "collision_line_profile.rs"]
pub(crate) mod profile;
use crate::{
    collision_mesh_line::{
        CollisionMeshLineError, CollisionMeshLineInput, CollisionMeshLineNearest,
        CollisionMeshOccurrence, try_mesh_line,
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_ground_triangle_topology::{AssemblyGroundNumeric, AssemblyGroundQueryBox},
    skeleton_root_transforms::SkeletonRootVector,
    world::transform_collision_aabb,
};
use skate2_formats::collision::CollisionError;

#[cfg(test)]
#[path = "collision_line_collection_tests.rs"]
mod tests;

/// Borrows one actually published mesh slot and its retained local bounds.
/// A saved decoding error is observed only when that slot is visited.
pub(crate) struct CollisionLineCollectionMesh<'a, O: Clone> {
    pub(crate) occurrence: CollisionMeshOccurrence<'a, O>,
    pub(crate) local_bounds: &'a Result<([f32; 3], [f32; 3]), CollisionError>,
}

#[derive(Clone, Copy)]
pub(crate) struct CollisionLineInput {
    pub(crate) start: SkeletonRootVector,
    pub(crate) end: SkeletonRootVector,
    pub(crate) fatness: f32,
}

#[derive(Debug)]
pub(crate) enum CollisionLineCollectionError<N> {
    Bounds {
        slot: usize,
        source: CollisionError,
    },
    Mesh {
        slot: usize,
        source: CollisionMeshLineError<N>,
    },
}

impl<N: std::fmt::Debug> std::fmt::Display for CollisionLineCollectionError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bounds { slot, source } => write!(f, "line slot {slot} bounds: {source}"),
            Self::Mesh { slot, source } => write!(f, "line slot {slot}: {source}"),
        }
    }
}
impl<N: std::fmt::Debug> std::error::Error for CollisionLineCollectionError<N> {}

/// The mesh collection remains borrowed throughout traversal. No sorting,
/// identity-transform shortcut or global bounds/decode preflight occurs.
pub(crate) fn try_collection_line<'a, O: Clone, N: AssemblyGroundNumeric + ReckoningNumeric>(
    meshes: impl Iterator<Item = CollisionLineCollectionMesh<'a, O>>,
    input: CollisionLineInput,
    nearest: &mut CollisionMeshLineNearest<O>,
    numeric: &mut N,
) -> Result<(), CollisionLineCollectionError<N::Error>> {
    // Query min/max are invariant during this call. Consecutive occurrences
    // with bit-identical inverse transforms have the exact same pure transformed
    // box. Reuse that result, never replace the native transform with identity.
    let mut previous = None;
    try_collection_line_with_transform(
        meshes,
        input,
        nearest,
        numeric,
        |minimum, maximum, inverse| {
            let bits = inverse.map(f32::to_bits);
            if let Some((previous_bits, bounds)) = previous {
                if previous_bits == bits {
                    return bounds;
                }
            }
            let bounds = transform_collision_aabb(minimum, maximum, inverse);
            previous = Some((bits, bounds));
            bounds
        },
    )
}

fn try_collection_line_with_transform<'a, O: Clone, N: AssemblyGroundNumeric + ReckoningNumeric>(
    meshes: impl Iterator<Item = CollisionLineCollectionMesh<'a, O>>,
    input: CollisionLineInput,
    nearest: &mut CollisionMeshLineNearest<O>,
    numeric: &mut N,
    mut transform: impl FnMut([f32; 3], [f32; 3], [f32; 16]) -> ([f32; 3], [f32; 3]),
) -> Result<(), CollisionLineCollectionError<N::Error>> {
    let profiling = profile::enabled();
    if profiling {
        profile::ray();
    }
    let mut mesh_count = 0;
    let mut survivor_count = 0;
    let delta = SkeletonRootVector {
        xyz: std::array::from_fn(|axis| input.end.xyz[axis] - input.start.xyz[axis]),
        w: input
            .end
            .w
            .zip(input.start.w)
            .map(|(end, start)| end - start),
    };
    if !delta
        .xyz
        .iter()
        .any(|value| value.abs() > f32::from_bits(0x3780_0000))
    {
        return Ok(());
    }
    // Only XYZ of this local query box reaches subsequent culls/transforms;
    // output hit vectors retain their independently known/unknown full W.
    // Exceptional VMX min/max bits retain the existing formats host boundary.
    let minimum =
        std::array::from_fn(|axis| input.start.xyz[axis].min(input.end.xyz[axis]) - input.fatness);
    let maximum =
        std::array::from_fn(|axis| input.start.xyz[axis].max(input.end.xyz[axis]) + input.fatness);
    for mesh in meshes {
        if profiling {
            mesh_count += 1;
        }
        let slot = mesh.occurrence.slot;
        // The same source midpoint/half-extent result, even for identity inverse.
        let (local_minimum, local_maximum) = transform(minimum, maximum, *mesh.occurrence.inverse);
        let &(bounds_minimum, bounds_maximum) =
            mesh.local_bounds
                .as_ref()
                .map_err(|source| CollisionLineCollectionError::Bounds {
                    slot,
                    source: source.clone(),
                })?;
        if (0..3).any(|axis| {
            bounds_minimum[axis] > local_maximum[axis] || local_minimum[axis] > bounds_maximum[axis]
        }) {
            continue;
        }
        if profiling {
            survivor_count += 1;
        }
        try_mesh_line(
            &mesh.occurrence,
            &CollisionMeshLineInput {
                start: input.start,
                end: input.end,
                delta,
                model_box: AssemblyGroundQueryBox {
                    minimum: local_minimum,
                    maximum: local_maximum,
                },
                line_fatness: input.fatness,
            },
            nearest,
            numeric,
        )
        .map_err(|source| CollisionLineCollectionError::Mesh { slot, source })?;
    }
    if profiling {
        profile::collection(mesh_count, survivor_count);
    }
    Ok(())
}

/// Local job results publish only after normal return, including a true miss.
/// Earlier leaf effects stay in local scratch when traversal/numerics refuse.
pub(crate) fn try_publish_collection_line<
    'a,
    O: Clone,
    N: AssemblyGroundNumeric + ReckoningNumeric,
>(
    meshes: impl Iterator<Item = CollisionLineCollectionMesh<'a, O>>,
    input: CollisionLineInput,
    result: &mut CollisionMeshLineNearest<O>,
    numeric: &mut N,
) -> Result<(), CollisionLineCollectionError<N::Error>> {
    let mut local = CollisionMeshLineNearest::new();
    try_collection_line(meshes, input, &mut local, numeric)?;
    // Position/normal/surface/parameter/owner/count commits have no intervening
    // collaborator callback or shared observation through this exclusive borrow.
    *result = local;
    Ok(())
}
