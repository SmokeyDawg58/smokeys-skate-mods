//! Private kernel fixtures only: no constructed publication, arena identity,
//! native address image, or claimed original-board geometry. Mounted owner
//! coverage remains in the separate original-data corpus.

use super::*;
use crate::{
    narrow_phase::GpPrimitive,
    simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyPool},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};

/// Original-data diagnostic on the admitted board and its existing publication.
/// This deliberately installs one diagnostic immutable material on four Parts.
/// The caller MUST run the real selected-token setter again before its next
/// grounded-contact test; copying old triples would not restore pointer identity.
pub(crate) fn check_material_replacement(
    session: &mut SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
    publication: &mut SkateboardCollisionPublication,
) {
    let frame = backend.collision_body_pool().frame_identity().unwrap();
    publication.try_refresh(session, backend).unwrap();
    let bits = |value: AssemblyPartMaterial| {
        [
            value.static_friction,
            value.dynamic_friction,
            value.restitution,
        ]
        .map(f32::to_bits)
    };
    let old_records: Vec<_> = publication
        .records()
        .iter()
        .map(|record| bits(record.material))
        .collect();
    let authored: Vec<_> = session
        .template
        .part_payloads()
        .iter()
        .map(|part| bits(part.material))
        .collect();
    let nonwheels: Vec<_> = (4..7)
        .map(|part| bits(*session.current_part_material(part).unwrap()))
        .collect();
    let selected = Arc::new(AssemblyPartMaterial {
        static_friction: 0.75,
        dynamic_friction: 0.25,
        restitution: 0.125,
    });
    for ordinal in 0..4 {
        session
            .try_set_part_material(backend, ordinal, selected.clone())
            .unwrap();
    }
    for ordinal in 0..4 {
        assert!(Arc::ptr_eq(
            session.part_attachment.parts[ordinal]
                .material_override
                .as_ref()
                .unwrap(),
            &selected
        ));
    }
    assert!(!publication.has_bounds_for_frame(frame));
    assert_eq!(
        publication.try_stage_bounds(session, backend),
        Err(SkateboardCollisionPublicationError::StaleVolumes)
    );
    assert_eq!(
        publication
            .records()
            .iter()
            .map(|record| bits(record.material))
            .collect::<Vec<_>>(),
        old_records
    );
    publication.try_refresh(session, backend).unwrap();
    assert!(publication.has_bounds_for_frame(frame));
    for record in publication.records() {
        let ordinal = usize::from(record.rigid_body_index);
        assert_eq!(
            bits(record.material),
            bits(*session.current_part_material(ordinal).unwrap())
        );
        if ordinal < 4 {
            assert_eq!(bits(record.material), bits(*selected));
        }
    }
    assert_eq!(
        session
            .template
            .part_payloads()
            .iter()
            .map(|part| bits(part.material))
            .collect::<Vec<_>>(),
        authored
    );
    assert_eq!(
        (4..7)
            .map(|part| bits(*session.current_part_material(part).unwrap()))
            .collect::<Vec<_>>(),
        nonwheels
    );
    check_volume_flag_publication(session, backend, publication);
    check_group_publication(session, backend, publication);
}

fn check_group_publication(
    session: &SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
    publication: &mut SkateboardCollisionPublication,
) {
    let frame = backend.collision_body_pool().frame_identity().unwrap();
    let original = session
        .board_collision_groups
        .try_snapshot()
        .unwrap()
        .assembly;
    let untouched = Arc::new(BoardCollisionGroups::default());
    untouched.try_initialize(7).unwrap();
    for world_only in [true, false] {
        let old_group = publication.group();
        session
            .board_collision_groups
            .try_set_mode(world_only)
            .unwrap();
        assert!(!publication.has_bounds_for_frame(frame));
        assert_eq!(publication.group(), old_group);
        assert_eq!(
            publication.try_stage_bounds(session, backend),
            Err(SkateboardCollisionPublicationError::StaleVolumes)
        );
        publication.try_refresh(session, backend).unwrap();
        let expected = if world_only { 2 } else { 4 };
        assert_eq!(publication.group(), expected);
        assert!(
            publication
                .records()
                .iter()
                .all(|record| record.group == expected)
        );
        assert!(publication.has_bounds_for_frame(frame));
        assert_eq!(untouched.try_snapshot().unwrap().assembly, 4);
    }
    session
        .board_collision_groups
        .try_set_mode(original == 2)
        .unwrap();
    publication.try_refresh(session, backend).unwrap();
}

fn check_volume_flag_publication(
    session: &SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
    publication: &mut SkateboardCollisionPublication,
) {
    let frame = backend.collision_body_pool().frame_identity().unwrap();
    let definition = session.template.collision();
    assert!(Arc::ptr_eq(definition, &publication.definition));
    let original_shape_flags = definition.wheels.parts()[0].volume().flags;
    assert_ne!(original_shape_flags & 1, 0);
    definition.volume_flags.disable_wheels_for_test();
    assert!(!publication.has_bounds_for_frame(frame));
    assert_eq!(
        publication.try_stage_bounds(session, backend),
        Err(SkateboardCollisionPublicationError::StaleVolumes)
    );
    publication.try_refresh(session, backend).unwrap();
    assert!(publication.has_bounds_for_frame(frame));
    assert!(
        publication.records()[..4]
            .iter()
            .all(|record| !record.enabled)
    );
    definition.volume_flags.enable_top(2).unwrap();
    assert!(!publication.has_bounds_for_frame(frame));
    publication.try_refresh(session, backend).unwrap();
    assert!(
        publication.records()[..4]
            .iter()
            .all(|record| record.enabled)
    );
    assert_eq!(
        definition.wheels.parts()[0].volume().flags,
        original_shape_flags
    );
    assert!(Arc::ptr_eq(definition, &publication.definition));
}

fn live_parts_fixture(
    states: [u32; BOARD_PART_COUNT],
) -> (SimulationRigidBodyPool, [LivePart; BOARD_PART_COUNT]) {
    let mut pool = SimulationRigidBodyPool::try_new(BOARD_PART_COUNT).unwrap();
    let inertia = Arc::new(SimulationInertia::native_default());
    let parts = std::array::from_fn(|ordinal| {
        let mut pose = FRESH_VOLUME_TRANSFORM;
        pose[12] = match ordinal {
            0 => -40.0,
            1 => 80.0,
            _ => 0.0,
        };
        let handle = pool
            .try_add_rigid_body(pose, Some(&inertia), states[ordinal])
            .unwrap()
            .unwrap();
        let body = pool.try_observe(handle).unwrap();
        let snapshot = pool.try_serialised_assembly_snapshot(body).unwrap();
        LivePart {
            body,
            transform: pool.try_rigid_body_pose(handle).unwrap(),
            material: AssemblyPartMaterial {
                static_friction: 0.5,
                dynamic_friction: 0.3,
                restitution: 0.4,
            },
            state: snapshot.state,
            motion: semantic_motion(
                snapshot.velocity,
                snapshot.omega,
                snapshot.force,
                snapshot.kinematic_value,
                snapshot.torque,
                snapshot.cooldown,
            ),
        }
    });
    (pool, parts)
}

fn volume_fixture(kind: VolumeType) -> Volume {
    let mut volume = Volume {
        transform: FRESH_VOLUME_TRANSFORM,
        vtable_offset: kind as u32,
        shape_words: [0; 3],
        radius: 1.0,
        group_id: 0,
        surface_id: 0,
        flags: 1,
    };
    match kind {
        VolumeType::Sphere => {}
        VolumeType::Capsule => volume.shape_words[0] = 2.0_f32.to_bits(),
        VolumeType::Box => volume.shape_words = [1.0_f32, 2.0, 3.0].map(f32::to_bits),
        VolumeType::Cylinder => volume.shape_words = [2.0_f32, 0.75, 0.0].map(f32::to_bits),
        VolumeType::Triangle => {
            // Same actual Volume field layout/dirty-normal flags as authored
            // deck triangles, but deliberately simple test coordinates.
            volume.transform = [
                0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ];
            volume.shape_words = [1.0_f32, -1.0, 1.0].map(f32::to_bits);
            volume.flags = 0x3e3;
        }
        _ => panic!("fixture supports concrete primitives only"),
    }
    volume
}

fn sphere_record(
    ordinal: usize,
    part: &LivePart,
    enabled: bool,
) -> SerialisedAssemblyVolumeDataState {
    let mut volume = volume_fixture(VolumeType::Sphere);
    volume.flags = u32::from(enabled);
    SerialisedAssemblyVolumeDataState {
        rigid_body_index: u16::try_from(ordinal).unwrap(),
        volume_type: VolumeType::Sphere,
        volume,
        padding: SERIALISED_ASSEMBLY_VOLUME_PADDING,
        material: part.material,
        surface_id: 0,
        transform: part.transform,
        enabled,
        group: 4,
        frozen: part.state & 2 != 0,
    }
}

fn xyz_bits(bounds: AabbWords) -> [[u32; 3]; 2] {
    [bounds.min_f32(), bounds.max_f32()]
        .map(|vector| std::array::from_fn(|axis| vector[axis].to_bits()))
}

#[test]
fn create_and_refresh_bounds_include_disabled_static_and_frozen_parts() {
    let (pool, parts) = live_parts_fixture([1, 2, 4, 4, 4, 4, 4]);
    for part in parts {
        pool.try_validate_observation(part.body).unwrap();
    }
    let records = [
        sphere_record(0, &parts[0], false), // static AND disabled, at x=-40
        sphere_record(1, &parts[1], true),  // frozen, at x=80
        sphere_record(2, &parts[2], true),  // ordinary Active, at x=0
    ];
    assert!(!records[0].enabled);
    assert_eq!(parts[0].state, 1);
    assert!(records[1].frozen);
    let created = calculate_create_bounds(&records, &parts).unwrap();
    let refreshed = calculate_bounds(&records, &parts).unwrap();
    let expected = [
        [-41.05_f32, -1.05, -1.05].map(f32::to_bits),
        [81.05_f32, 1.05, 1.05].map(f32::to_bits),
    ];
    assert_eq!(xyz_bits(created), expected);
    assert_eq!(xyz_bits(refreshed), expected);
    let active_only = calculate_bounds(&records[2..], &parts).unwrap();
    assert_ne!(xyz_bits(active_only), expected);
    // These pure infallible kernels share a result here; this is an inclusion
    // test, not a claim to observe the distinct native callback chronology.
}

#[test]
fn header_counts_real_static_and_frozen_bits_without_conflating_them() {
    let (_mixed_pool, mixed) = live_parts_fixture([1, 2, 4, 1, 2, 4, 4]);
    assert_eq!(header_state(&mixed), (2, false));
    let (_static_pool, all_static) = live_parts_fixture([1; BOARD_PART_COUNT]);
    assert_eq!(header_state(&all_static), (7, false));
    let (_frozen_pool, all_frozen) = live_parts_fixture([2; BOARD_PART_COUNT]);
    assert_eq!(header_state(&all_frozen), (0, true));
}

const PRIMITIVES: [VolumeType; 5] = [
    VolumeType::Sphere,
    VolumeType::Capsule,
    VolumeType::Box,
    VolumeType::Triangle,
    VolumeType::Cylinder,
];
const W_PROBES: [u32; 5] = [
    0x7fc0_5678,
    0x7f80_0000,
    0xff80_0000,
    0x8000_0000,
    0x4120_0000,
];

fn parent_frame() -> [f32; 16] {
    [
        0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 9.0, 8.0, 7.0, 1.0,
    ]
}

fn moving_fixture() -> SerialisedAssemblyBodyMotion {
    semantic_motion(
        [2.0, -3.0, 5.0],
        [0.1, 0.2, -0.3],
        [1.0, 2.0, 3.0],
        0.25,
        [-0.3, 0.4, 0.5],
        30,
    )
}

fn primitive_swept_bounds(
    volume: &Volume,
    parent: &[f32; 16],
    motion: SerialisedAssemblyBodyMotion,
) -> AabbWords {
    infallible(try_calculate_primitive_part_bbox(
        volume,
        parent,
        motion,
        &mut HostSerialisedAssemblyBBoxNumeric,
    ))
    .unwrap()
}

#[test]
fn raw_volume_and_parent_w_poisoning_preserves_supported_bbox_and_gp_xyz() {
    for kind in PRIMITIVES {
        let original = volume_fixture(kind);
        let parent = parent_frame();
        let baseline_bounds = primitive_swept_bounds(&original, &parent, moving_fixture());
        let baseline_gp = GpPrimitive::from_volume(&original, Some(parent)).unwrap();
        assert!(
            baseline_bounds.min_f32()[..3]
                .iter()
                .all(|value| value.is_finite())
        );
        for probe in 0..W_PROBES.len() {
            let mut poisoned = original.clone();
            let mut poisoned_parent = parent;
            for column in 0..4 {
                poisoned.transform[column * 4 + 3] =
                    f32::from_bits(W_PROBES[(probe + column) % W_PROBES.len()]);
                poisoned_parent[column * 4 + 3] =
                    f32::from_bits(W_PROBES[(probe + column + 1) % W_PROBES.len()]);
            }
            let bounds = primitive_swept_bounds(&poisoned, &poisoned_parent, moving_fixture());
            let gp = GpPrimitive::from_volume(&poisoned, Some(poisoned_parent)).unwrap();
            assert_eq!(
                xyz_bits(bounds),
                xyz_bits(baseline_bounds),
                "{kind:?}, W probe {probe}"
            );
            // GP stores semantic XYZ vectors/scalars, not packed fourth lanes.
            assert_eq!(gp, baseline_gp, "{kind:?}, W probe {probe}");
        }
    }
}

#[test]
fn swept_motion_fourth_lanes_do_not_feed_xyz_but_real_xyz_motion_does() {
    for kind in PRIMITIVES {
        let volume = volume_fixture(kind);
        let parent = parent_frame();
        let motion = moving_fixture();
        let baseline = primitive_swept_bounds(&volume, &parent, motion);
        for probe in 0..W_PROBES.len() {
            let mut changed = motion;
            changed.velocity_and_right[3] = f32::from_bits(W_PROBES[probe]);
            changed.omega_and_left[3] = f32::from_bits(W_PROBES[(probe + 1) % W_PROBES.len()]);
            changed.force_and_kine[3] = f32::from_bits(W_PROBES[(probe + 2) % W_PROBES.len()]);
            changed.torque_and_cool[3] = f32::from_bits(W_PROBES[(probe + 3) % W_PROBES.len()]);
            assert_eq!(
                xyz_bits(primitive_swept_bounds(&volume, &parent, changed)),
                xyz_bits(baseline),
                "{kind:?}, W probe {probe}"
            );
        }
        let mut displaced = motion;
        displaced.velocity_and_right[0] = 1000.0;
        assert_ne!(
            xyz_bits(primitive_swept_bounds(&volume, &parent, displaced)),
            xyz_bits(baseline),
            "{kind:?} must consume real velocity XYZ"
        );
        // The above deliberate test mutation proves lane independence only.
        // Production semantic_motion still retains actual kine/cool exactly;
        // it does not authorize zeroing or fabricating those body fields.
        assert_eq!(motion.force_and_kine[3].to_bits(), 0.25_f32.to_bits());
        assert_eq!(motion.torque_and_cool[3].to_bits(), 30);
    }
}
