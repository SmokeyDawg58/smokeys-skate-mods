//! Nested model ownership regressions; analytic host expectations, not traces.
use super::*;
use crate::{
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skeleton_body_definition::fresh_volume, world::CollisionSimplePrimitiveSet,
};

fn arena(children: Vec<Volume>) -> CollisionArena {
    CollisionArena {
        id: 1,
        grind_assets: vec![],
        aggregate_roots: vec![],
        primitive_sets: vec![],
        simple_primitive_sets: vec![CollisionSimplePrimitiveSet {
            dictionary_index: 1,
            bounding_box_min: [-100.; 3],
            bounding_box_max: [100.; 3],
            volumes: children,
        }],
        meshes: vec![],
        collision_models: vec![],
        instances: vec![],
    }
}

#[test]
fn nested_disabled_box_keeps_both_transforms_and_source_volume_not_set_bbox() {
    let mut box_volume = fresh_volume(VolumeType::Box, 0., [1.0_f32.to_bits(); 3]);
    box_volume.flags = 0;
    box_volume.transform[12] = 2.;
    let arena = arena(vec![box_volume]);
    let mut root = fresh_volume(VolumeType::Aggregate, 0., [1, 0, 0]);
    root.flags = 0;
    root.transform[12] = 3.;
    let result = try_arena_volume_integral(&arena, &root, &mut HostSimulationVolumeIntegralNumeric)
        .unwrap()
        .unwrap();
    // Cube volume8, COM.x5. Its enclosing +/-100 box must not supply mass.
    assert_eq!(result.columns[3], [40., 0., 0., 8.]);
    assert_eq!(result.columns[0][3], 40.);
}

#[test]
fn empty_mapped_array_succeeds_but_null_does_not_and_cycles_are_rejected() {
    let mut root = fresh_volume(VolumeType::Aggregate, 0., [1, 0, 0]);
    root.transform[12] = f32::NAN;
    let mut numeric = HostSimulationVolumeIntegralNumeric;
    let empty = try_arena_volume_integral(&arena(vec![]), &root, &mut numeric)
        .unwrap()
        .unwrap();
    assert_eq!(
        empty.columns, [[0.; 4]; 4],
        "no successful child means no outer transform"
    );
    let null = fresh_volume(VolumeType::Null, 0., [0; 3]);
    assert!(
        try_arena_volume_integral(&arena(vec![]), &null, &mut numeric)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        try_arena_volume_integral(&arena(vec![root.clone()]), &root, &mut numeric),
        Err(ArenaIntegralError::Cycle(_))
    ));
}
