//! Synthetic owned definitions with real bodies and constraints. The fixture
//! proves the source Part get/set bridge, not an original board spawn trace.
use super::*;
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyDriveDefinition, DynamicAssemblyJointDefinition,
        DynamicAssemblyPartAttachmentMetadata, DynamicAssemblyPartAttachmentTemplate,
        DynamicAssemblyPartPayload, DynamicAssemblyRuntimeTemplate,
    },
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
    },
    simulation_rigid_body_pool::SimulationInertia,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    world::orthonormalize_collision_affine,
};
use std::sync::{Arc, RwLock};

type Backend = SimulationAssemblyBackend<(), (), ()>;
type Session = DynamicAssemblySession<Backend, ()>;

#[test]
fn typed_part_getter_preserves_detached_cache_and_attached_unknown_packed_w() {
    use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
    let mut session = fixture(Some(inverse()));
    let mut backend = Backend::try_new(2, 1, 1).unwrap();
    let original = cache(&session, 0);
    assert_eq!(
        session
            .try_live_part_transform_typed(&backend, 0)
            .unwrap()
            .known_lanes()
            .map(|lanes| lanes.map(f32::to_bits)),
        Some(original)
    );
    let partial = SkeletonRootMatrix::from_columns([
        SkeletonRootVector {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        },
        SkeletonRootVector::known([4.0, 5.0, 6.0, -0.0]),
        SkeletonRootVector::known([7.0, 8.0, 9.0, 11.0]),
        SkeletonRootVector {
            xyz: [10.0, 12.0, 13.0],
            w: None,
        },
    ]);
    session.part_attachment.parts[0].part_transform =
        DynamicAssemblyPartTransformCache::Partial(partial);
    assert_eq!(
        session.try_live_part_transform_typed(&backend, 0).unwrap(),
        partial
    );
    assert_eq!(
        session
            .try_live_part_transform_typed(&backend, 0)
            .unwrap()
            .columns()[1]
            .w
            .unwrap()
            .to_bits(),
        0x8000_0000
    );
    session.part_attachment.parts[0].part_transform =
        DynamicAssemblyPartTransformCache::Known(original.map(f32::from_bits));
    session.attach(&mut backend, 4).unwrap();
    let projected = session.try_live_part_transform_host(&backend, 0).unwrap();
    // Attached branch must ignore even a partially known cache.
    session.part_attachment.parts[0].part_transform =
        DynamicAssemblyPartTransformCache::Partial(partial);
    let typed = session.try_live_part_transform_typed(&backend, 0).unwrap();
    assert_eq!(typed.known_lanes(), None);
    for (index, column) in typed.columns().iter().enumerate() {
        assert_eq!(column.w, None);
        assert_eq!(
            column.xyz.map(f32::to_bits),
            [
                projected[index * 4],
                projected[index * 4 + 1],
                projected[index * 4 + 2]
            ]
            .map(f32::to_bits)
        );
    }
    let foreign = Backend::try_new(2, 1, 1).unwrap();
    assert!(matches!(
        session.try_live_part_transform_typed(&foreign, 0),
        Err(DynamicAssemblyPartTransformError::ForeignBackend)
    ));
}

#[test]
fn attached_body_frame_ignores_inverse_part_frame_and_keeps_native_w_unknown() {
    let mut session = fixture(Some(inverse()));
    let mut backend = Backend::try_new(2, 1, 1).unwrap();
    assert!(matches!(
        session.try_attached_part_body_transform(&backend, 0),
        Err(DynamicAssemblyPartTransformError::BodyTransformRequiresAttachedBody { index: 0 })
    ));
    session.attach(&mut backend, 4).unwrap();
    let body = session.part_attachment.parts[0].body.unwrap();
    let raw = backend.try_part_body_pose(body).unwrap();
    let actual = session
        .try_attached_part_body_transform(&backend, 0)
        .unwrap();
    let mapped = session.try_live_part_transform_host(&backend, 0).unwrap();
    assert_ne!(raw.map(f32::to_bits), mapped.map(f32::to_bits));
    for (index, column) in actual.columns().iter().enumerate() {
        assert_eq!(
            column.xyz.map(f32::to_bits),
            [raw[index * 4], raw[index * 4 + 1], raw[index * 4 + 2]].map(f32::to_bits)
        );
        assert_eq!(column.w, None);
    }
    let foreign = Backend::try_new(2, 1, 1).unwrap();
    assert!(matches!(
        session.try_attached_part_body_transform(&foreign, 0),
        Err(DynamicAssemblyPartTransformError::ForeignBackend)
    ));
}

#[test]
fn displacement_requires_an_actual_owned_part_body_before_any_numeric_call() {
    struct NoNumeric;
    impl crate::reckoning_numeric::ReckoningNumeric for NoNumeric {
        type Error = ();
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, ()> {
            panic!("invalid body must fail first");
        }
        fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, ()> {
            panic!("no rsqrt");
        }
        fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, ()> {
            panic!("no dot");
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, ()> {
            panic!("no conversion");
        }
    }
    let mut session = fixture(None);
    let mut backend = Backend::try_new(2, 1, 1).unwrap();
    let cached = [cache(&session, 0), cache(&session, 1)];
    assert!(matches!(
        session.try_apply_part_displacement(&mut backend, 0, [0.0; 4], &mut NoNumeric),
        Err(DynamicAssemblyPartDisplacementError::MissingBody { index: 0 })
    ));
    assert!(matches!(
        session.try_apply_part_displacement(&mut backend, 2, [0.0; 4], &mut NoNumeric),
        Err(DynamicAssemblyPartDisplacementError::Part(
            DynamicAssemblyPartTransformError::PartIndex { index: 2, count: 2 }
        ))
    ));
    session.attach(&mut backend, 4).unwrap();
    let before = backend.rigid_body_snapshot();
    let mut foreign = Backend::try_new(2, 1, 1).unwrap();
    assert!(matches!(
        session.try_apply_part_displacement(&mut foreign, 0, [0.0; 4], &mut NoNumeric),
        Err(DynamicAssemblyPartDisplacementError::Part(
            DynamicAssemblyPartTransformError::ForeignBackend
        ))
    ));
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!([cache(&session, 0), cache(&session, 1)], cached);
}

#[test]
fn detached_only_setter_needs_no_backend_and_preserves_raw_cache_words() {
    let mut session = fixture(Some(inverse()));
    let mut words = requested().map(f32::to_bits);
    words[0] = 0x7fc1_2345;
    words[3] = 0x8000_0000;
    words[15] = 0x0000_0001;
    let other = cache(&session, 1);
    session
        .try_set_detached_part_transform(0, words.map(f32::from_bits))
        .unwrap();
    assert_eq!(cache(&session, 0), words);
    assert_eq!(cache(&session, 1), other);
    assert_eq!(
        session.try_set_detached_part_transform(2, FRESH_VOLUME_TRANSFORM),
        Err(DynamicAssemblyPartTransformError::PartIndex { index: 2, count: 2 })
    );
    assert_eq!(cache(&session, 0), words);
}

#[test]
fn detached_only_setter_refuses_attached_session_before_cache_or_body_effects() {
    let mut session = fixture(None);
    let mut backend = Backend::try_new(2, 1, 1).unwrap();
    session.attach(&mut backend, 4).unwrap();
    let before = backend.rigid_body_snapshot();
    let cached = [cache(&session, 0), cache(&session, 1)];
    assert_eq!(
        session.try_set_detached_part_transform(0, requested()),
        Err(DynamicAssemblyPartTransformError::AttachedRequiresBackend)
    );
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!([cache(&session, 0), cache(&session, 1)], cached);
    session.detach(&mut backend).unwrap();
    session
        .try_set_detached_part_transform(0, requested())
        .unwrap();
    assert_eq!(cache(&session, 0), requested().map(f32::to_bits));
}

fn fixture(inverse: Option<[f32; 16]>) -> Session {
    let metadata = (0..2)
        .map(|_| DynamicAssemblyPartAttachmentMetadata {
            inverse_body_transform: inverse.unwrap_or(FRESH_VOLUME_TRANSFORM),
            has_inverse_body_transform: u32::from(inverse.is_some()),
            is_static_body: 0,
        })
        .collect();
    let payloads = (0..2)
        .map(|_| DynamicAssemblyPartPayload {
            inertia: Arc::new(SimulationInertia {
                inverse_tensor: [2.0, 5.0, 11.0],
                inverse_mass: 0.125,
                ..SimulationInertia::native_default()
            })
            .into(),
            material: (),
        })
        .collect();
    let attachment =
        DynamicAssemblyPartAttachmentTemplate::try_from_parts(metadata, payloads).unwrap();
    let params = SimulationDriveDynamicsParams {
        spring: 0.0,
        damping: 0.0,
        strength: 0.0,
        raw_type: 0,
    };
    let drive = DynamicAssemblyDriveDefinition {
        parent_part: 0,
        child_part: 1,
        frames: Arc::new(RwLock::new(SimulationDriveFrames {
            quaternion_a: [0.0, 0.0, 0.0, 1.0],
            position_a: [0.0; 4],
            quaternion_b: [0.0, 0.0, 0.0, 1.0],
            position_b: [0.0; 4],
        })),
        dynamics: Arc::new(RwLock::new(SimulationDriveDynamics {
            linear: params,
            angular: params,
        })),
    };
    let template = DynamicAssemblyRuntimeTemplate::try_from_owned_parts(
        Arc::new(()),
        Arc::new(attachment),
        vec![DynamicAssemblyJointDefinition {
            parent_part: 0,
            child_part: 1,
            frames: Arc::new(()),
            limits: Arc::new(()).into(),
        }],
        vec![drive],
    )
    .unwrap();
    Session::try_new(Arc::new(template), vec![FRESH_VOLUME_TRANSFORM; 2]).unwrap()
}
fn inverse() -> [f32; 16] {
    [
        0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 3.0, -5.0, 0.0,
    ]
}
fn requested() -> [f32; 16] {
    [
        0.0, 3.0, 0.0, -0.0, -4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 17.0, -19.0, 23.0, 29.0,
    ]
}
fn cache(session: &Session, index: usize) -> [u32; 16] {
    session
        .part_transforms()
        .map(|matrix| matrix.expect("known fixture Part cache"))
        .nth(index)
        .unwrap()
        .map(f32::to_bits)
}
fn canonical_pose(mut matrix: [f32; 16]) -> [f32; 16] {
    matrix[3] = 0.0;
    matrix[7] = 0.0;
    matrix[11] = 0.0;
    matrix[15] = 1.0;
    matrix
}

fn partial_request() -> crate::skeleton_root_transforms::SkeletonRootMatrix {
    use crate::skeleton_root_transforms::SkeletonRootMatrix;
    let mut columns = *SkeletonRootMatrix::known(requested()).columns();
    columns[3].w = None;
    SkeletonRootMatrix::from_columns(columns)
}

#[test]
fn attached_partial_cache_keeps_unknown_w_while_actual_body_and_constraints_remain_usable() {
    for inverse_frame in [None, Some(inverse())] {
        let mut session = fixture(inverse_frame);
        let mut backend = Backend::try_new(2, 1, 1).unwrap();
        session.attach(&mut backend, 4).unwrap();
        let joints = backend.joint_snapshot();
        let drives = backend.drive_snapshot();
        let partial = partial_request();
        session
            .try_set_part_transform_typed(&mut backend, 0, &partial)
            .unwrap();
        assert!(session.part_transforms().next().unwrap().is_none());
        assert_eq!(session.part_transforms().len(), 2);
        assert_eq!(session.try_cached_part_transform_typed(0).unwrap(), partial);
        let body = backend.rigid_body_snapshot();
        let live = session.try_live_part_transform_host(&backend, 0).unwrap();
        // Known and unknown W requests have identical XYZ inputs to the body.
        // Compare on the SAME body occurrence, avoiding allocation differences.
        session
            .try_set_part_transform_host(&mut backend, 0, requested())
            .unwrap();
        assert_eq!(backend.rigid_body_snapshot(), body);
        assert_eq!(
            session
                .try_live_part_transform_host(&backend, 0)
                .unwrap()
                .map(f32::to_bits),
            live.map(f32::to_bits)
        );
        session
            .try_set_part_transform_typed(&mut backend, 0, &partial)
            .unwrap();
        assert_eq!(backend.joint_snapshot(), joints);
        assert_eq!(backend.drive_snapshot(), drives);
        session.detach(&mut backend).unwrap();
        assert!(session.part_transforms().all(|matrix| matrix.is_some()));
        assert_eq!(
            session
                .try_live_part_transform_host(&backend, 0)
                .unwrap()
                .map(f32::to_bits),
            live.map(f32::to_bits)
        );
        // Detach's actual copyback completes the cache before generic reattach.
        session.attach(&mut backend, 4).unwrap();
        session.detach(&mut backend).unwrap();
    }
}

#[test]
fn partial_setter_refusals_preserve_the_actual_cache_and_body_prefix() {
    let mut session = fixture(None);
    let mut backend = Backend::try_new(2, 1, 1).unwrap();
    let partial = partial_request();
    let old = cache(&session, 0);
    assert_eq!(
        session.try_set_part_transform_typed(&mut backend, 0, &partial),
        Err(DynamicAssemblyPartTransformError::PartialTransformRequiresAttachedBody { index: 0 })
    );
    assert_eq!(cache(&session, 0), old);
    session.attach(&mut backend, 4).unwrap();
    let before = backend.rigid_body_snapshot();
    let mut foreign = Backend::try_new(2, 1, 1).unwrap();
    assert_eq!(
        session.try_set_part_transform_typed(&mut foreign, 0, &partial),
        Err(DynamicAssemblyPartTransformError::ForeignBackend)
    );
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(cache(&session, 0), old);
    session.detach(&mut backend).unwrap();
}

#[test]
fn detached_get_set_copies_all_sixteen_raw_words_without_body_or_inverse_math() {
    let words = [
        0x7fc1_2345,
        0xff80_0000,
        0x8000_0000,
        0x7f80_0000,
        0x0000_0001,
        0xffc5_4321,
        0x8000_0001,
        0x3f80_0000,
        0x4000_0000,
        0x4040_0000,
        0x4080_0000,
        0x40a0_0000,
        0x40c0_0000,
        0x40e0_0000,
        0x4100_0000,
        0x8000_0000,
    ];
    for mapped in [None, Some(inverse())] {
        let mut session = fixture(mapped);
        let mut backend = Backend::try_new(0, 0, 0).unwrap();
        let before = backend.rigid_body_snapshot();
        session
            .try_set_part_transform_host(&mut backend, 0, words.map(f32::from_bits))
            .unwrap();
        assert_eq!(cache(&session, 0), words);
        assert_eq!(
            session
                .try_live_part_transform_host(&backend, 0)
                .unwrap()
                .map(f32::to_bits),
            words
        );
        assert_eq!(cache(&session, 1), FRESH_VOLUME_TRANSFORM.map(f32::to_bits));
        assert_eq!(backend.rigid_body_snapshot(), before);
        // Detached cache access is not bound to an unassigned backend owner.
        let other = Backend::try_new(0, 0, 0).unwrap();
        assert_eq!(
            session
                .try_live_part_transform_host(&other, 0)
                .unwrap()
                .map(f32::to_bits),
            words
        );
    }
}

#[test]
fn attached_set_maps_inverse_updates_existing_body_and_keeps_original_raw_cache_last() {
    for mapped in [None, Some(inverse())] {
        let mut session = fixture(mapped);
        let mut backend = Backend::try_new(2, 1, 1).unwrap();
        session.attach(&mut backend, 4).unwrap();
        let body = session.part_attachment.parts[0].body.unwrap();
        let template = Arc::clone(session.template());
        let joints = backend.joint_snapshot();
        let drives = backend.drive_snapshot();
        let initial = backend.rigid_body_snapshot();
        let inertia = Arc::clone(
            backend
                .rigid_body_pool_for_test()
                .try_inertia(body)
                .unwrap()
                .unwrap(),
        );
        let definition_before = template.part_attachment.metadata_for_test().to_vec();
        session
            .try_set_part_transform_host(&mut backend, 0, requested())
            .unwrap();
        let mapped_body = part_attachment_transform(requested(), mapped.as_ref());
        let expected = canonical_pose(orthonormalize_collision_affine(mapped_body));
        let actual = backend.try_part_body_pose(body).unwrap();
        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
        let expected_position: [f32; 3] = if mapped.is_some() {
            [9.0, -28.0, 33.0]
        } else {
            [17.0, -19.0, 23.0]
        };
        assert_eq!(
            [actual[12], actual[13], actual[14]].map(f32::to_bits),
            expected_position.map(f32::to_bits)
        );
        assert_eq!(cache(&session, 0), requested().map(f32::to_bits));
        assert_ne!(cache(&session, 0), actual.map(f32::to_bits));
        let live = session.try_live_part_transform_host(&backend, 0).unwrap();
        let expected_live = part_get_transform(requested(), Some(&actual), mapped.as_ref());
        assert_eq!(live.map(f32::to_bits), expected_live.map(f32::to_bits));
        assert_eq!(backend.joint_snapshot(), joints);
        assert_eq!(backend.drive_snapshot(), drives);
        assert_eq!(
            template.part_attachment.metadata_for_test(),
            definition_before
        );
        assert!(Arc::ptr_eq(session.template(), &template));
        assert!(Arc::ptr_eq(
            backend
                .rigid_body_pool_for_test()
                .try_inertia(body)
                .unwrap()
                .unwrap(),
            &inertia
        ));
        let mut after = backend.rigid_body_snapshot();
        let index = body.id().index() as usize;
        let changed = &mut after.slots[index];
        let before = &initial.slots[index];
        changed.right = before.right;
        changed.up = before.up;
        changed.at = before.at;
        changed.center_of_mass = before.center_of_mass;
        changed.quaternion = before.quaternion;
        changed.inverse_inertia_full = before.inverse_inertia_full;
        changed.inverse_inertia_split = before.inverse_inertia_split;
        changed.inverse_mass = before.inverse_mass;
        assert_eq!(after, initial);
        assert_eq!(session.part_attachment.parts[0].body, Some(body));
        session.detach(&mut backend).unwrap();
    }
}

#[test]
fn live_getter_observes_later_body_movement_not_the_unchanged_part_cache() {
    for mapped in [None, Some(inverse())] {
        let mut session = fixture(mapped);
        let mut backend = Backend::try_new(2, 1, 1).unwrap();
        session.attach(&mut backend, 4).unwrap();
        let body = session.part_attachment.parts[0].body.unwrap();
        let old_cache = cache(&session, 0);
        // Real owner-qualified body update emulates a later live-pose producer;
        // this is not claimed to be an integration trace or a Part cache write.
        backend
            .try_set_part_body_transform_host(body, requested())
            .unwrap();
        let body_pose = backend.try_part_body_pose(body).unwrap();
        let expected =
            part_get_transform(FRESH_VOLUME_TRANSFORM, Some(&body_pose), mapped.as_ref());
        let actual = session.try_live_part_transform_host(&backend, 0).unwrap();
        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
        assert_ne!(actual.map(f32::to_bits), old_cache);
        assert_eq!(cache(&session, 0), old_cache);
        session.detach(&mut backend).unwrap();
    }
}

#[test]
fn authority_and_index_refusals_leave_raw_cache_and_all_pool_state_untouched() {
    let mut session = fixture(None);
    let mut backend = Backend::try_new(2, 1, 1).unwrap();
    let mut other = Backend::try_new(0, 0, 0).unwrap();
    let error = DynamicAssemblyPartTransformError::PartIndex { index: 2, count: 2 };
    assert_eq!(
        session.try_set_part_transform_host(&mut backend, 2, requested()),
        Err(error)
    );
    session.attach(&mut backend, 4).unwrap();
    let cache_before = cache(&session, 0);
    let bodies = backend.rigid_body_snapshot();
    let joints = backend.joint_snapshot();
    let drives = backend.drive_snapshot();
    for index in [0, 2] {
        assert_eq!(
            session.try_set_part_transform_host(&mut other, index, requested()),
            Err(DynamicAssemblyPartTransformError::ForeignBackend)
        );
        assert_eq!(
            session.try_live_part_transform_host(&other, index),
            Err(DynamicAssemblyPartTransformError::ForeignBackend)
        );
    }
    assert_eq!(
        session.try_set_part_transform_host(&mut backend, 2, requested()),
        Err(DynamicAssemblyPartTransformError::PartIndex { index: 2, count: 2 })
    );
    assert_eq!(
        session.try_live_part_transform_host(&backend, 2),
        Err(DynamicAssemblyPartTransformError::PartIndex { index: 2, count: 2 })
    );
    assert_eq!(cache(&session, 0), cache_before);
    assert_eq!(backend.rigid_body_snapshot(), bodies);
    assert_eq!(backend.joint_snapshot(), joints);
    assert_eq!(backend.drive_snapshot(), drives);
    session.detach(&mut backend).unwrap();
}

#[test]
fn stale_body_error_happens_before_raw_cache_publication_and_preserves_valid_constraints() {
    let mut session = fixture(Some(inverse()));
    let mut temporary = fixture(None);
    let mut backend = Backend::try_new(4, 2, 2).unwrap();
    session.attach(&mut backend, 4).unwrap();
    temporary.attach(&mut backend, 4).unwrap();
    let stale = temporary.part_attachment.parts[0].body.unwrap();
    temporary.detach(&mut backend).unwrap();
    temporary.attach(&mut backend, 4).unwrap();
    let actual = session.part_attachment.parts[0].body;
    // Private invalid token injection is a synthetic failure discriminator;
    // no public constructor creates or exports this corrupted Part binding.
    session.part_attachment.parts[0].body = Some(stale);
    let before = backend.rigid_body_snapshot();
    let joints = backend.joint_snapshot();
    let drives = backend.drive_snapshot();
    let old_cache = cache(&session, 0);
    let error = SimulationRigidBodyPoolError::StaleHandle {
        slot: stale.id().index(),
    };
    assert_eq!(
        session.try_set_part_transform_host(&mut backend, 0, requested()),
        Err(DynamicAssemblyPartTransformError::Body(error.clone()))
    );
    assert_eq!(
        session.try_live_part_transform_host(&backend, 0),
        Err(DynamicAssemblyPartTransformError::Body(error))
    );
    assert_eq!(cache(&session, 0), old_cache);
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(backend.joint_snapshot(), joints);
    assert_eq!(backend.drive_snapshot(), drives);
    session.part_attachment.parts[0].body = actual;
    temporary.detach(&mut backend).unwrap();
    session.detach(&mut backend).unwrap();
}
