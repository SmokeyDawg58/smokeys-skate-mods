//! ComputeVolumeIntegral82ae9878 for the existing decoded collision arena.
//! Uses the same primitive, mapped-array and Jacobi math as constructed bodies.
//! Null returns false; recognized mapped arrays return true even if empty.
use super::{
    SimulationMappedArrayIntegral, SimulationVolumeIntegral, SimulationVolumeIntegralError,
    SimulationVolumeIntegralNumeric, integral_from_bbox, try_child_integral, try_sum_integrals,
};
use crate::{
    dynamic_clustered_mesh_template::{
        DynamicClusteredMeshTemplateError, resolve_aggregate_target,
    },
    world::{
        CollisionAggregateRef, CollisionArena, compose_collision_affine, transform_collision_aabb,
    },
};
use skate2_formats::collision::{Volume, VolumeType};

#[cfg(test)]
#[path = "simulation_volume_integral_arena_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum ArenaIntegralError<E> {
    Primitive(SimulationVolumeIntegralError<E>),
    Aggregate(DynamicClusteredMeshTemplateError),
    MissingAggregate(CollisionAggregateRef),
    Cycle(CollisionAggregateRef),
}

pub(crate) fn try_arena_volume_integral<N: SimulationVolumeIntegralNumeric>(
    arena: &CollisionArena,
    volume: &Volume,
    numeric: &mut N,
) -> Result<Option<SimulationVolumeIntegral>, ArenaIntegralError<N::Error>> {
    try_volume(arena, volume, &mut Vec::new(), numeric)
}

fn try_volume<N: SimulationVolumeIntegralNumeric>(
    arena: &CollisionArena,
    volume: &Volume,
    path: &mut Vec<CollisionAggregateRef>,
    numeric: &mut N,
) -> Result<Option<SimulationVolumeIntegral>, ArenaIntegralError<N::Error>> {
    if volume.volume_type() != Some(VolumeType::Aggregate) {
        return try_child_integral(volume, 0, numeric).map_err(ArenaIntegralError::Primitive);
    }
    let aggregate = resolve_aggregate_target(arena, volume.shape_words[0])
        .map_err(ArenaIntegralError::Aggregate)?;
    if path.contains(&aggregate) {
        // Malformed host asset, not a replacement for native recursive behavior.
        return Err(ArenaIntegralError::Cycle(aggregate));
    }
    path.push(aggregate);
    let result = match aggregate {
        CollisionAggregateRef::SimplePrimitiveSet(index) => {
            let set = arena
                .simple_primitive_sets
                .get(index)
                .ok_or(ArenaIntegralError::MissingAggregate(aggregate))?;
            try_children(arena, &set.volumes, &volume.transform, path, numeric)
        }
        CollisionAggregateRef::PrimitiveSet(index) => {
            let set = arena
                .primitive_sets
                .get(index)
                .ok_or(ArenaIntegralError::MissingAggregate(aggregate))?;
            try_children(arena, &set.volumes, &volume.transform, path, numeric)
        }
        CollisionAggregateRef::Mesh(index) => {
            let mesh = &arena
                .meshes
                .get(index)
                .ok_or(ArenaIntegralError::MissingAggregate(aggregate))?
                .clustered_mesh;
            // ComputeVolumeIntegral82ae995c passes vol itself as parent to
            // AggregateVolume.GetBBox82adc9c8: stored transform is applied twice.
            // Preserve that call, even though normally these model matrices are
            // identity. Aggregate header bounds, NOT a recomputed triangle box.
            let transform = compose_collision_affine(volume.transform, volume.transform);
            let (low, high) = transform_collision_aabb(
                std::array::from_fn(|i| mesh.bounding_box_min[i]),
                std::array::from_fn(|i| mesh.bounding_box_max[i]),
                transform,
            );
            Ok(integral_from_bbox(low, high))
        }
    };
    path.pop();
    result.map(Some)
}

fn try_children<N: SimulationVolumeIntegralNumeric>(
    arena: &CollisionArena,
    children: &[Volume],
    transform: &[f32; 16],
    path: &mut Vec<CollisionAggregateRef>,
    numeric: &mut N,
) -> Result<SimulationVolumeIntegral, ArenaIntegralError<N::Error>> {
    Ok(try_sum_integrals(
        children
            .iter()
            .map(|child| try_volume(arena, child, path, numeric)),
        Some(transform),
    )?
    .integral)
}

/// The two actual children of LWVehicleModel.CreateFromMesh82d1efd0's
/// constructed SimpleMappedArray: original source volume, then the extra box.
/// No synthetic arena IDs or flattened/unioned replacement geometry.
pub(crate) fn try_vehicle_mapped_integral<N: SimulationVolumeIntegralNumeric>(
    arena: &CollisionArena,
    source: &Volume,
    extra_box: &Volume,
    numeric: &mut N,
) -> Result<SimulationMappedArrayIntegral, ArenaIntegralError<N::Error>> {
    let mut path = Vec::new();
    try_sum_integrals(
        [source, extra_box]
            .into_iter()
            .map(|volume| try_volume(arena, volume, &mut path, numeric)),
        Some(&crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM),
    )
}
