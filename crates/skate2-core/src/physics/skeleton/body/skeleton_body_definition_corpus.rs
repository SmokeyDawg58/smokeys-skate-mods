//! Real retained Vault + authored24Parts -> actual owned shape/mass definitions.
//! Pose is the existing controlled original RAW fixture, not live caller data.
use crate::{
    attribute_runtime_corpus::mounted_physics_attributes_with_classes,
    dynamic_assembly_session::skeleton_normal::{NORMAL_COLLISION_ENABLED, SkeletonNormalAssembly},
    simulation_joint_pool::{
        SimulationJointFrames, SimulationJointLimits, SimulationJointPool,
        SimulationMutableJointLimits,
    },
    simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyPool},
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_assembly::SkeletonPreNormalSimulationBackend,
    skeleton_attributes::{
        SkeletonAttributeClass, SkeletonAttributes, SkeletonBodyAttributeData,
        SkeletonDriveAttributes,
    },
    skeleton_body_attributes::{
        self as keys, SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute as Route,
        SkeletonBodyAttributes, SkeletonBodyDebugRead,
    },
    skeleton_body_definition::{SkeletonBodyDefinition, SkeletonPartGeometry},
    skeleton_body_joints::{HostSkeletonJointNumeric, SkeletonBodyJoints},
    skeleton_body_mass::HostSkeletonBodyMassNumeric,
    skeleton_collision_attributes::{
        FACE_RADIUS, GROIN_RADIUS, SkeletonCollisionAttributeRead, SkeletonCollisionAttributes,
        SkeletonCollisionOffset,
    },
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_culling::{SkeletonCullingInventory, SkeletonCullingKey, SkeletonCullingResource},
    skeleton_data_corpus::with_mounted_skeleton_data,
    skeleton_hat::{HatAttributes, HostHatNumeric},
    skeleton_joint_attributes::{
        SkeletonJointAttributeProvider, SkeletonJointAttributeRead, SkeletonJointByteRoute,
    },
    skeleton_normal_joint_limits::{SkeletonJointLimitBinding, try_set_normal_joint_limits},
    skeleton_runtime_state::SkeletonPartMaterialSelection,
};
use skate2_formats::collision::VolumeType;
use std::sync::Arc;

/// Reuses the original-data producer chain for mixed pool tests. The selected
/// tutorial's actual `HatType0` is checked, not a synthetic geometry replacement.
/// `SkeletonPoseView` remains the explicitly controlled original RAW pose fixture
/// documented by `with_mounted_skeleton_data`, not a recovered live spawn pose.
pub(crate) fn with_mounted_original_normal_skeleton(
    inspect: impl FnOnce(SkeletonNormalAssembly, &mut SkeletonBodyAttributes),
) {
    let vault = mounted_physics_attributes_with_classes(&[
        SkeletonAttributeClass::Skeleton.key(),
        SkeletonAttributeClass::Bones.key(),
        SkeletonAttributeClass::Drives.key(),
        SkeletonAttributeClass::Joints.key(),
        keys::ANIMATION_CLASS,
        keys::WIPEOUT_CLASS,
        keys::DEBUG_PHYSICS_CLASS,
        SkeletonCollisionAttributes::CLASS,
        0x91bc_a669_3efc_9aa7,
    ]);
    let tutorial = vault.instance(0x91bc_a669_3efc_9aa7, 0x2275_b900_5016_6969);
    let hat = tutorial
        .try_scalar_word(0x3441_8cad_1c23_bc94, 0)
        .unwrap()
        .unwrap();
    assert_eq!(hat, 0);
    let attributes = Arc::new(
        SkeletonBodyAttributeData::try_read(
            &mut SkeletonAttributes::bind(&vault, keys::DEFAULT_COLLECTION),
            SkeletonDriveAttributes::constructor_value(),
        )
        .unwrap(),
    );
    let mut reader = SkeletonBodyAttributes::bind(&vault, keys::DEFAULT_COLLECTION);
    let mut joint_reader = SkeletonJointAttributeProvider::bind(&vault, keys::DEFAULT_COLLECTION);
    let collision =
        SkeletonCollisionData::try_new(&mut SkeletonCollisionAttributes::bind(&vault)).unwrap();
    let normal = SkeletonCullingInventory::default()
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    with_mounted_skeleton_data(|data| {
        let body = Arc::new(
            SkeletonBodyDefinition::try_create(
                Arc::clone(data),
                attributes,
                &mut reader,
                hat,
                None,
                &mut || panic!("actual tutorial hat0 has no hat-enable sampling"),
                &mut HostSkeletonBodyMassNumeric,
                &mut HostSimulationVolumeIntegralNumeric,
                &mut HostHatNumeric,
            )
            .unwrap(),
        );
        let joints = SkeletonBodyJoints::try_create(
            &body.data,
            &body.attributes,
            body.allocated_joint_count,
            &mut joint_reader,
            &mut HostSkeletonJointNumeric,
        )
        .unwrap();
        assert_eq!(body.parts.len(), 24);
        assert_eq!(joints.joints.len(), 22);
        inspect(
            SkeletonNormalAssembly::try_create(body, joints, normal, collision).unwrap(),
            &mut reader,
        );
    });
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
#[expect(
    clippy::too_many_lines,
    reason = "one original data producer chain including both hat layout variants"
)]
fn mounted_original_body_shapes_mass_caches_and_hat_variants() {
    crate::physical_player_assembly::corpus::mounted_paired_physical_construction();
    let vault = mounted_physics_attributes_with_classes(&[
        SkeletonAttributeClass::Skeleton.key(),
        SkeletonAttributeClass::Bones.key(),
        SkeletonAttributeClass::Drives.key(),
        SkeletonAttributeClass::Joints.key(),
        keys::ANIMATION_CLASS,
        keys::WIPEOUT_CLASS,
        keys::DEBUG_PHYSICS_CLASS,
        HatAttributes::CLASS,
        SkeletonCollisionAttributes::CLASS,
        0x91bc_a669_3efc_9aa7,
    ]);
    let tutorial = vault.instance(0x91bc_a669_3efc_9aa7, 0x2275_b900_5016_6969);
    let mut collision_reader = SkeletonCollisionAttributes::bind(&vault);
    // Independent original db.big PtrN record b3798 binds VLTslot5c068
    // (target dependency0) to dependency1 BIN9b420,112-byte cached layout.
    let expected_offsets = [[0xbe19_999a, 0, 0, 0], [0, 0x3d8f_5c29, 0, 0]];
    for (slot, expected) in [
        SkeletonCollisionOffset::Groin,
        SkeletonCollisionOffset::Face,
    ]
    .into_iter()
    .zip(expected_offsets)
    {
        assert_eq!(
            collision_reader
                .cached_offset(slot)
                .unwrap()
                .map(f32::to_bits),
            expected
        );
    }
    // Inline nodes5c140/+8 and5c0e0/+8, flags40: actual radius words,
    // not inferred from the constructor's squared-radius output.
    for (key, expected) in [(GROIN_RADIUS, 0x3dcc_cccd), (FACE_RADIUS, 0x3e19_999a)] {
        let pointer = collision_reader.lookup(key).unwrap();
        assert_eq!(
            collision_reader.float(&pointer).unwrap().to_bits(),
            expected
        );
    }
    let collision = SkeletonCollisionData::try_new(&mut collision_reader).unwrap();
    assert_eq!(
        collision
            .specific_parts
            .offsets
            .map(|vector| vector.map(f32::to_bits)),
        expected_offsets
    );
    assert_eq!(
        collision.specific_parts.radius_squared.map(f32::to_bits),
        [0x3c23_d70b, 0x3cb8_51ec]
    );
    assert_eq!(
        collision.specific_parts.parts,
        [
            crate::ragdoll::SkeletonPartIndex::Hips,
            crate::ragdoll::SkeletonPartIndex::Neck1
        ]
    );
    assert_eq!(
        collision
            .specific_parts
            .world_positions
            .map(|vector| vector.map(f32::to_bits)),
        [[0; 4]; 2]
    );
    assert_eq!(collision.specific_parts.contact, [None; 2]);
    assert_eq!(collision.specific_parts.recent_contact, [None; 2]);
    assert_eq!(collision.is_compliant, [0; 24]);
    assert_eq!(collision.priority.map(f32::to_bits), [0; 24]);
    assert_eq!(
        collision
            .most_vertical_contact_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0, 0x3f80_0000, 0, 0]
    );
    assert_eq!(
        collision
            .most_vertical_toe_contact_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0, 0x3f80_0000, 0, 0]
    );
    let selected_hat = tutorial
        .try_scalar_word(0x3441_8cad_1c23_bc94, 0)
        .unwrap()
        .unwrap();
    assert_eq!(selected_hat, 0); // Actual inherited GetSkaterHatType, not presentation-recipe inference.
    let attributes = Arc::new(
        SkeletonBodyAttributeData::try_read(
            &mut SkeletonAttributes::bind(&vault, keys::DEFAULT_COLLECTION),
            SkeletonDriveAttributes::constructor_value(),
        )
        .unwrap(),
    );
    let mut reader = SkeletonBodyAttributes::bind(&vault, keys::DEFAULT_COLLECTION);
    // Independent raw db.big node+8 words: 5e050,5dd20,5ddc0,5de40,5de50,5f520.
    for (route, key, bits) in [
        (Route::LocalBones, keys::CAPSULE_RADIUS_SCALE, 0x3f99_999a),
        (
            Route::GlobalSkeleton,
            keys::CAPSULE_LENGTH_SCALE,
            0x3f99_999a,
        ),
        (Route::LocalSkeleton, keys::PART_MASS_SCALE, 0x43a0_0000),
        (Route::LocalSkeleton, keys::ROOT_HALF_LENGTH, 0x3ecc_cccd),
        (Route::LocalSkeleton, keys::ROOT_RADIUS, 0x3eae_147b),
        (
            Route::GlobalWipeout,
            keys::RAGDOLL_INVERSE_MASS_NUMERATOR,
            0x3f80_0000,
        ),
    ] {
        assert_eq!(reader.float(route, key).unwrap().to_bits(), bits);
    }
    assert_eq!(
        reader
            .byte(Route::GlobalAnimation, keys::ENABLE_JOINTS)
            .unwrap(),
        1
    );
    let mut debug = reader.debug_instance().unwrap();
    assert_eq!(debug.mode().unwrap(), 1);
    assert_eq!(debug.factor().unwrap().to_bits(), 0x40a0_0000);
    drop(debug);
    let mut joint_reader = SkeletonJointAttributeProvider::bind(&vault, keys::DEFAULT_COLLECTION);
    // Independent original PtrN record b2518 targets VLT slot5dcf8 in dependency0
    // and resolves dependency1 BIN9eed0. Cached Skeleton layout+30 is this full
    // vector; it is not a generic attribute-node lookup or producer output.
    assert_eq!(
        joint_reader
            .cached_prism_velocity()
            .unwrap()
            .map(f32::to_bits),
        [0x3c23_d70a, 0x3c23_d70a, 0x3c23_d70a, 0]
    );
    // Original local Joints nodes5e220/5e1e0/5e1f0 and global Animation5bdc0:
    // each source boolean consumes exactly the first inline byte at node+8.
    for (route, key, byte) in [
        (
            SkeletonJointByteRoute::LocalJoints,
            0xdff9_7208_0e90_f82f,
            1,
        ),
        (
            SkeletonJointByteRoute::LocalJoints,
            0x9cf0_c7c3_8917_9e4c,
            0,
        ),
        (
            SkeletonJointByteRoute::LocalJoints,
            0x2654_99fa_ea27_836a,
            0,
        ),
        (
            SkeletonJointByteRoute::GlobalAnimation,
            0xdff9_7208_0e90_f82f,
            1,
        ),
    ] {
        assert_eq!(joint_reader.byte(route, key).unwrap(), byte);
    }
    // Raw Skeleton nodes5de90 and5def0 both contain41200000 (10), before
    // source multiplication by426fffff. All22 original bone-joint records have
    // BallJoint byte0 (BIN9fd68 layout, source-permuted rows).
    for key in [0x71cc_8102_bc85_277b, 0x1b84_fd92_5541_2c0f] {
        assert_eq!(
            joint_reader.skeleton_float(key).unwrap().to_bits(),
            0x4120_0000
        );
    }
    assert!(attributes.joints.iter().all(|joint| joint.ball_joint == 0));
    let mut culling_inventory = SkeletonCullingInventory::default();
    let normal = culling_inventory
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    let ragdoll = culling_inventory
        .try_get_or_create(SkeletonCullingKey::Ragdoll)
        .unwrap();
    assert!(!Arc::ptr_eq(&normal, &ragdoll));
    with_mounted_skeleton_data(|data| {
        for hat_type in [selected_hat, 4, 1] {
            let hat_layout = HatAttributes::bind(&vault, hat_type).try_layout().unwrap();
            let expected_hex = if hat_type == 4 {
                "3CF5C28F3DE147AE00000000000000003CF5C28F3FCCCCCD3E800000000000003CA3D70A3DB851EC0FCFC1285FF9B2EB"
            } else {
                "3CF5C28F00000000BCA3D70A000000003E99999A3FCCCCCD3E800000000000003CA3D70A3E3851EC0000000000000000"
            };
            let expected: [u8; 48] = std::array::from_fn(|i| {
                u8::from_str_radix(&expected_hex[i * 2..i * 2 + 2], 16).unwrap()
            });
            assert_eq!(hat_layout.bytes(), &expected);
            let mut samples = 0;
            // Deliberately changing test CVar, not a substituted native startup default.
            let mut enable = || {
                samples += 1;
                samples == 1
            };
            let body = SkeletonBodyDefinition::try_create(
                Arc::clone(data),
                Arc::clone(&attributes),
                &mut reader,
                hat_type,
                Some(&hat_layout),
                &mut enable,
                &mut HostSkeletonBodyMassNumeric,
                &mut HostSimulationVolumeIntegralNumeric,
                &mut HostHatNumeric,
            )
            .unwrap();
            assert_eq!(samples, if hat_type == 0 { 0 } else { 2 });
            assert_eq!(body.parts.len(), 24);
            assert_eq!(body.allocated_joint_count, 22);
            assert!(Arc::ptr_eq(&body.data, data));
            assert!(Arc::ptr_eq(&body.attributes, &attributes));
            let joints = SkeletonBodyJoints::try_create(
                &body.data,
                &body.attributes,
                body.allocated_joint_count,
                &mut joint_reader,
                &mut HostSkeletonJointNumeric,
            )
            .unwrap();
            assert_eq!(joints.allocated_count, 22);
            // Source nearest represented ancestor and actual original packed
            // name reverse map. No endpoint is taken from the output under test.
            let expected_pairs = [
                (2, 1),
                (11, 2),
                (4, 3),
                (5, 4),
                (6, 5),
                (11, 6),
                (8, 7),
                (9, 8),
                (10, 9),
                (11, 10),
                (12, 11),
                (13, 12),
                (14, 13),
                (23, 14),
                (16, 15),
                (17, 16),
                (18, 17),
                (23, 18),
                (20, 19),
                (21, 20),
                (22, 21),
                (23, 22),
            ];
            assert_eq!(
                joints
                    .joints
                    .iter()
                    .map(|joint| (joint.parent_part, joint.child_part))
                    .collect::<Vec<_>>(),
                expected_pairs
            );
            assert_eq!(joints.emitted_for_part[0], None);
            assert_eq!(joints.emitted_for_part[23], None);
            for (ordinal, joint) in joints.joints.iter().enumerate() {
                assert_eq!(joints.emitted_for_part[ordinal + 1], Some(ordinal));
                assert_eq!(joint.limits.p_prism.map(f32::to_bits), [0; 4]);
                // Independently f32-evaluated original .01/timestep refinement
                // and10*426fffff; explicit host oracle, not Xenon trace words.
                assert_eq!(
                    joint.limits.v_prism.map(f32::to_bits),
                    [0x3f19_9999, 0x3f19_9999, 0x3f19_9999, 0]
                );
                assert_eq!(joint.limits.v_swing.to_bits(), 0x4415_ffff);
                assert_eq!(joint.limits.v_twist.to_bits(), 0x4415_ffff);
                assert_eq!(
                    (joint.limits.raw_swing_type, joint.limits.raw_twist_type),
                    (1, 1)
                );
            }
            check_normal_joint_storage(&body, &joints);
            assert!(Arc::ptr_eq(
                &normal,
                &culling_inventory
                    .try_get_or_create(SkeletonCullingKey::Normal)
                    .unwrap()
            ));
            assert!(Arc::ptr_eq(
                &ragdoll,
                &culling_inventory
                    .try_get_or_create(SkeletonCullingKey::Ragdoll)
                    .unwrap()
            ));
            let SkeletonPartGeometry::Primitive(root) = &body.parts[0].geometry else {
                panic!("root capsule")
            };
            assert_eq!(root.volume_type(), Some(VolumeType::Capsule));
            assert_eq!(root.radius.to_bits(), 0x3eae_147b);
            assert_eq!(root.shape_words[0], 0x3ecc_cccd);
            assert_eq!(root.flags, 0);
            assert_eq!(body.parts[0].effective_mass.to_bits(), 0x3a83_126f);
            // Independent raw PhysicsParam/Vault arithmetic oracle, calculated
            // separately from this Rust producer. Not Xenon trace observations.
            for (index, raw, effective, ragdoll_inverse, animated_inverse, radius) in [
                (0, 0, 0x3a83_126f, 0x4479_ffff, 0x4120_0000, 0x3eae_147b),
                (
                    1,
                    0x3bab_f89e,
                    0x3fd6_f6c6,
                    0x3f18_6f53,
                    0x3e18_6f53,
                    0x3e0c_63f1,
                ),
                (
                    2,
                    0x3a4d_9528,
                    0x3e80_7d39,
                    0x407f_0682,
                    0x3f7f_0682,
                    0x3d89_2253,
                ),
                (
                    3,
                    0x39b0_563b,
                    0x3ddc_6bca,
                    0x4014_a933,
                    0x4014_a933,
                    0x3d70_d842,
                ),
                (
                    23,
                    0x3c1f_c9f9,
                    0x4047_bc77,
                    0x3ea4_0e70,
                    0x3c24_0e70,
                    0x3e2f_b00a,
                ),
            ] {
                let part = &body.parts[index];
                assert_eq!(part.unscaled_part_mass.to_bits(), raw);
                assert_eq!(part.effective_mass.to_bits(), effective);
                assert_eq!(part.ragdoll.inverse_mass.to_bits(), ragdoll_inverse);
                assert_eq!(part.animated.inverse_mass.to_bits(), animated_inverse);
                if let SkeletonPartGeometry::Primitive(volume) = &part.geometry {
                    assert_eq!(volume.radius.to_bits(), radius);
                }
            }
            if hat_type != 0 {
                let SkeletonPartGeometry::Hat(hat) = &body.parts[1].geometry else {
                    panic!("real hat aggregate")
                };
                assert_eq!(hat.children()[0].volume_type(), Some(VolumeType::Cylinder));
                assert_eq!(hat.children()[0].shape_words[0], 0x3c23_d70a);
                assert_eq!(
                    hat.children()[0].shape_words[1],
                    if hat_type == 4 {
                        0x3db8_51ec
                    } else {
                        0x3e38_51ec
                    }
                );
                assert_eq!(hat.children()[0].flags, 0);
                assert_eq!(hat.children()[1].flags, 1);
                assert!(body.parts[1].mass.has_inverse_body_transform);
                assert_eq!(
                    body.parts[1]
                        .mass
                        .inverse_body_transform_storage
                        .unwrap()
                        .map(f32::to_bits),
                    FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
                );
            }
            for part in &body.parts {
                assert!(
                    part.mass
                        .packed_inverse_tensor()
                        .iter()
                        .all(|lane| lane.is_finite() && *lane > 0.0)
                );
                assert_eq!(
                    part.ragdoll.inverse_tensor[3].to_bits(),
                    part.ragdoll.inverse_tensor[0].to_bits()
                );
                assert_eq!(
                    part.animated.inverse_tensor[3].to_bits(),
                    part.animated.inverse_tensor[0].to_bits()
                );
                assert_eq!(
                    part.material.static_friction.to_bits(),
                    attributes.normal.dynamic_friction.to_bits()
                );
                assert_eq!(
                    part.material.dynamic_friction.to_bits(),
                    attributes.normal.dynamic_friction.to_bits()
                );
            }
            for index in [0, 1, 2, 3, 23] {
                let part = &body.parts[index];
                println!(
                    "hat{hat_type} part{index}: {:?}, raw/effective {:08x}/{:08x}, ragdoll {:08x?}, animated {:08x?}",
                    part.debug,
                    part.unscaled_part_mass.to_bits(),
                    part.effective_mass.to_bits(),
                    part.ragdoll.inverse_tensor.map(f32::to_bits),
                    part.animated.inverse_tensor.map(f32::to_bits)
                );
            }
            // Each hat is an independent original-data fixture. Cloning this
            // constructor snapshot is not cloning a live collision owner.
            check_pre_normal_source_admission(
                body,
                joints,
                &normal,
                collision.clone(),
                &mut SkeletonBodyAttributes::bind(&vault, keys::DEFAULT_COLLECTION),
            );
        }
    });
}

/// Actual mounted geometry/mass/joints enter the existing Assembly transaction.
/// These definitions were made by the SAME retained SkeletonData/attribute
/// callers above. This exercises real admission, the explicit spy/material
/// property slice, represented normal setup and diagnostic collision publication,
/// but not missing island/body registration or a physics tick.
#[expect(
    clippy::too_many_lines,
    reason = "one mounted source-owner admission and explicit teardown proof"
)]
fn check_pre_normal_source_admission(
    body: SkeletonBodyDefinition,
    joints: SkeletonBodyJoints,
    normal: &Arc<SkeletonCullingResource>,
    collision: SkeletonCollisionData,
    reader: &mut SkeletonBodyAttributes,
) {
    let expected_joints: Vec<_> = joints
        .joints
        .iter()
        .map(|joint| {
            (
                joint.parent_part,
                joint.child_part,
                joint.frames,
                joint.limits,
            )
        })
        .collect();
    let expected_emitted = joints.emitted_for_part;
    assert_eq!(expected_joints.len(), body.data.joints().len());
    let body = Arc::new(body);
    let body_weak = Arc::downgrade(&body);
    let mut assembly = SkeletonNormalAssembly::try_create(
        Arc::clone(&body),
        joints,
        Arc::clone(normal),
        collision,
    )
    .unwrap();
    let session = assembly.session();
    let binding = Arc::clone(session.template().collision());
    assert!(Arc::ptr_eq(binding.initial_culling(), normal));
    assert_eq!(
        binding.initial_culling().table().words,
        crate::ragdoll::DEBUG_REFERENCE_NORMAL_CULLING_WORDS
    );
    assert!(Arc::ptr_eq(binding.body(), &body));
    assert!(Arc::ptr_eq(&binding.body().data, &body.data));
    assert!(Arc::ptr_eq(&binding.body().attributes, &body.attributes));
    assert_eq!(*binding.emitted_for_part(), expected_emitted);
    assert_eq!(binding.joint_limits().len(), 22);
    assert!(!session.is_attached());
    assert_eq!(
        session
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .len(),
        24
    );
    for matrix in session
        .part_transforms()
        .map(|matrix| matrix.expect("known fixture Part cache"))
    {
        assert_eq!(
            matrix.map(f32::to_bits),
            FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
        );
    }
    // Part::Initialize supplies I/J/K/zero even though supplied original pose
    // has nonidentity bone transforms. Do not silently place these at that pose.
    assert!(
        body.data
            .physics_pose()
            .iter()
            .any(|matrix| matrix.map(f32::to_bits) != FRESH_VOLUME_TRANSFORM.map(f32::to_bits))
    );
    for (ordinal, payload) in session.template().part_payloads().iter().enumerate() {
        assert!(payload.inertia.as_immutable().is_none());
        assert_eq!(
            payload.inertia.try_read().unwrap(),
            body.parts[ordinal].mass.inertia
        );
        assert_eq!(
            binding.inertias()[ordinal].try_read().unwrap(),
            body.parts[ordinal].mass.inertia
        );
        assert_eq!(payload.material, body.parts[ordinal].material);
        match (
            &binding.body().parts[ordinal].geometry,
            &body.parts[ordinal].geometry,
        ) {
            (
                SkeletonPartGeometry::Primitive(retained),
                SkeletonPartGeometry::Primitive(original),
            ) => assert!(Arc::ptr_eq(retained, original)),
            (SkeletonPartGeometry::Hat(retained), SkeletonPartGeometry::Hat(original)) => {
                assert!(Arc::ptr_eq(retained, original));
            }
            _ => panic!("source geometry owner changed during binding"),
        }
    }
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    assembly.attach(&mut backend).unwrap();
    assert_eq!(assembly.state().assembly_group, 1);
    assert_eq!(assembly.state().part_user_data, [1; 24]);
    let session = assembly.session();
    assert!(session.is_attached());
    assert!(backend.last_add_failure().is_none());
    let live_bodies = backend.rigid_body_snapshot();
    assert_eq!(live_bodies.counts.active, 24);
    assert_eq!(live_bodies.counts.free, 0);
    assert_eq!(live_bodies.counts.static_bodies, 0);
    assert_eq!(live_bodies.counts.frozen, 0);
    for (ordinal, slot) in live_bodies.slots.iter().enumerate() {
        let mass = &body.parts[ordinal].mass;
        let inverse = mass
            .has_inverse_body_transform
            .then(|| mass.inverse_body_transform_storage.as_ref().unwrap());
        // Independently route producer metadata through the already separately
        // source-tested Part attachment kernel; this is a routing/ownership
        // oracle, not a new independent Xenon numerical oracle.
        let expected_matrix = crate::world::orthonormalize_collision_affine(
            crate::world::part_attachment_transform(FRESH_VOLUME_TRANSFORM, inverse),
        );
        assert_eq!(slot.id.index(), u32::try_from(ordinal).unwrap());
        assert_eq!(slot.state, 4);
        assert!(slot.occurrence.is_some());
        assert_eq!(slot.inertia, Some(mass.inertia));
        assert_eq!(
            slot.inverse_mass.to_bits(),
            mass.inertia.inverse_mass.to_bits()
        );
        for (actual, start) in [
            (slot.right, 0),
            (slot.up, 4),
            (slot.at, 8),
            (slot.center_of_mass, 12),
        ] {
            assert_eq!(
                actual.map(f32::to_bits),
                std::array::from_fn(|lane| expected_matrix[start + lane].to_bits())
            );
        }
        let expected_tensor = crate::world::simulation_rotated_inverse_inertia_host(
            expected_matrix,
            mass.inertia.inverse_tensor,
            mass.inertia.inverse_mass,
        );
        assert_eq!(
            slot.inverse_inertia_full.map(f32::to_bits),
            expected_tensor.full.map(f32::to_bits)
        );
        assert_eq!(
            slot.inverse_inertia_split.map(f32::to_bits),
            expected_tensor.split.map(f32::to_bits)
        );
    }
    let live_joints = backend.joint_snapshot();
    assert_eq!(live_joints.counts.active, 22);
    assert_eq!(live_joints.counts.free, 0);
    let live_payloads = backend
        .active_joint_payloads_for_test()
        .unwrap_or_else(|_| panic!("mounted original Joint payloads must remain readable"));
    assert_eq!(live_payloads.len(), 22);
    for (ordinal, ((parent, child, frames, limits), (live_frames, live_limits))) in
        expected_joints.iter().zip(live_payloads).enumerate()
    {
        let slot = &live_joints.slots[ordinal];
        assert_eq!(
            slot.body_b.unwrap().id().index(),
            u32::try_from(*parent).unwrap()
        );
        assert_eq!(
            slot.body_a.unwrap().id().index(),
            u32::try_from(*child).unwrap()
        );
        assert!(slot.frames_present && slot.limits_present);
        assert_eq!(joint_frame_words(live_frames), joint_frame_words(*frames));
        assert_eq!(joint_limit_words(live_limits), joint_limit_words(*limits));
        assert_eq!(
            joint_limit_words(binding.joint_limits()[ordinal].try_read().unwrap()),
            joint_limit_words(*limits)
        );
    }
    assert_eq!(backend.drive_snapshot().counts.active, 0);
    assert!(session.template().drives().is_empty());
    check_actual_normal_setup(&mut assembly, &mut backend, reader, &expected_joints);
    assembly.detach(&mut backend).unwrap();
    assert!(!assembly.session().is_attached());
    assert_eq!(backend.rigid_body_snapshot().counts.free, 24);
    assert_eq!(backend.joint_snapshot().counts.free, 22);
    // Mutable pool slots retain only their payload owners while Free. The
    // actual source geometry lifetime remains the Assembly/body owner chain.
    drop(assembly);
    drop(binding);
    drop(body);
    assert!(body_weak.upgrade().is_none());
}

#[expect(
    clippy::too_many_lines,
    reason = "one original live normal setup and selective reset proof"
)]
fn check_actual_normal_setup(
    assembly: &mut SkeletonNormalAssembly,
    backend: &mut SkeletonPreNormalSimulationBackend,
    reader: &mut SkeletonBodyAttributes,
    original: &[(i32, i32, SimulationJointFrames, SimulationJointLimits)],
) {
    let binding = Arc::clone(assembly.session().template().collision());
    let body = binding.body();
    let before_bodies = backend.rigid_body_snapshot();
    let before_drives = backend.drive_snapshot();
    let original_specific = assembly.state().collision.specific_parts.clone();
    let initial_headers = assembly.state().volume_headers;
    // Initial source Create is after admission and BEFORE spy/root-material/
    // normal mutation. This local diagnostic publication is never supplied to
    // a runtime job between mutation and the explicit refresh below.
    let mut publication = crate::dynamic_assembly_session::skeleton_collision_publication::SkeletonCollisionPublication::try_create(
        assembly, backend, f32::from_bits(0x3f7f_be77),
    ).unwrap();
    let has_hat = matches!(&body.parts[1].geometry, SkeletonPartGeometry::Hat(_));
    assert_eq!(publication.records().len(), if has_hat { 25 } else { 24 });
    assert_eq!(publication.rigid_bodies().len(), 24);
    assert_eq!(publication.group(), 1);
    assert_eq!(publication.num_static(), 0);
    assert!(!publication.all_parts_frozen());
    assert_eq!(
        publication.culling_table().words,
        binding.initial_culling().table().words
    );
    let mut expected_mapping: Vec<u16> = (0..24).collect();
    if has_hat {
        expected_mapping.insert(2, 1);
    }
    assert_eq!(
        publication
            .records()
            .iter()
            .map(|record| record.rigid_body_index)
            .collect::<Vec<_>>(),
        expected_mapping
    );
    assert!(publication.records().iter().all(|record| record.group == 1));
    let original_records = publication.records().to_vec();
    let original_observations = publication.rigid_bodies().to_vec();
    if has_hat {
        assert_eq!(publication.records()[1].volume_type, VolumeType::Cylinder);
        assert_eq!(publication.records()[2].volume_type, VolumeType::Sphere);
        assert!(!publication.records()[1].enabled);
        assert!(publication.records()[2].enabled);
    }
    // Capture actual child records, not just count/type. Live normal header
    // mutation addresses the top-level aggregate, never these retained children.
    let hat_children = match &body.parts[1].geometry {
        SkeletonPartGeometry::Hat(hat) => Some(hat.children().to_vec()),
        SkeletonPartGeometry::Primitive(_) => None,
    };
    for (part, owner) in binding.inertias().iter().enumerate() {
        assert_eq!(
            owner.try_read_packed().unwrap().unwrap().map(f32::to_bits),
            body.parts[part]
                .mass
                .packed_inverse_tensor()
                .map(f32::to_bits)
        );
    }
    assert_eq!(assembly.state().disable_counts, [None; 24]);
    assert_eq!(assembly.state().volume_disable_count_active, None);
    let before_joints = backend.joint_snapshot();
    assembly.try_set_pre_normal_properties(backend).unwrap();
    let mut expected_spies = before_joints;
    for slot in &mut expected_spies.slots {
        slot.spy = 1;
    }
    assert_eq!(backend.joint_snapshot(), expected_spies);
    assert_eq!(
        assembly.state().material_selection[0],
        SkeletonPartMaterialSelection::Zero
    );
    assert_eq!(backend.rigid_body_snapshot(), before_bodies);

    // Source inputs independently determine the 44 ordered host Cos arguments.
    let floor = f32::from_bits(0x3c23_d70a);
    let angles: Vec<_> = original
        .iter()
        .enumerate()
        .map(|(row, (_, child, _, _))| {
            let bone = &body.data.parameters().bones[usize::try_from(*child).unwrap()];
            let attributes = body.attributes.joints[row];
            let swing = attributes.swing_angle * bone.swing;
            let twist = bone.twist * attributes.twist_angle;
            [swing, twist].map(|product| {
                if product - floor >= 0.0 {
                    product
                } else {
                    floor
                }
            })
        })
        .collect();
    // Independent actual Vault read; production below performs its own fresh
    // LocalSkeleton byte fetch AFTER the mass stores, not a substituted bool.
    assert_eq!(
        reader
            .byte(Route::LocalSkeleton, NORMAL_COLLISION_ENABLED)
            .unwrap(),
        1
    );
    let mut calls = 0;
    assembly
        .try_set_up_normal(backend, reader, &mut |angle| {
            assert_eq!(angle.to_bits(), angles[calls / 2][calls % 2].to_bits());
            calls += 1;
            Ok::<_, std::convert::Infallible>(angle.cos())
        })
        .unwrap();
    assert_eq!(calls, 44);
    assert_eq!(
        backend.joint_snapshot(),
        expected_spies,
        "spy1 and tag/topology survive limits writes"
    );
    assert_eq!(backend.drive_snapshot(), before_drives);
    let mut expected_bodies = before_bodies;
    for (part, owner) in binding.inertias().iter().enumerate() {
        let mut expected = body.parts[part].mass.inertia;
        expected.angular_drag = 0.0;
        expected.linear_drag = 0.0;
        let packed = if part == 0 {
            body.parts[0].mass.packed_inverse_tensor()
        } else {
            let cache = body.parts[part].animated;
            let [x, y, z, _] = cache.inverse_tensor;
            expected.inverse_mass = cache.inverse_mass;
            expected.inverse_tensor = [x, y, z];
            let xy = if x < y { x } else { y };
            expected.spherical = 1.0 / if xy < z { xy } else { z };
            cache.inverse_tensor
        };
        assert_eq!(owner.try_read().unwrap(), expected);
        assert_eq!(
            owner.try_read_packed().unwrap().unwrap().map(f32::to_bits),
            packed.map(f32::to_bits)
        );
        expected_bodies.slots[part].inertia = Some(expected);
    }
    // Only referenced inertia changed: cached mInvm/rotated tensors, pose,
    // motion, force, occurrence, rings and every other snapshot field remain.
    assert_eq!(backend.rigid_body_snapshot(), expected_bodies);
    let live_payloads = backend
        .active_joint_payloads_for_test()
        .unwrap_or_else(|_| panic!("mounted normal Joint payloads remain readable"));
    for (row, ((_, _, frames, limits), (live_frames, live_limits))) in
        original.iter().zip(live_payloads).enumerate()
    {
        let [swing, twist] = angles[row];
        let mut expected = *limits;
        expected.swing_a = swing;
        expected.twist_a = twist;
        expected.swing_c = swing.cos();
        expected.twist_c = twist.cos();
        assert_eq!(joint_frame_words(live_frames), joint_frame_words(*frames));
        assert_eq!(joint_limit_words(live_limits), joint_limit_words(expected));
        assert_eq!(
            joint_limit_words(binding.joint_limits()[row].try_read().unwrap()),
            joint_limit_words(expected)
        );
    }
    let state = assembly.state();
    publication.try_refresh(assembly, backend).unwrap();
    assert_eq!(publication.group(), 6);
    assert_eq!(publication.rigid_bodies(), original_observations);
    assert!(
        publication.has_bounds_for_frame(backend.collision_body_pool().frame_identity().unwrap())
    );
    for (record, original) in publication.records().iter().zip(&original_records) {
        assert_eq!(record.rigid_body_index, original.rigid_body_index);
        assert_eq!(record.volume_type, original.volume_type);
        assert_eq!(
            record.volume, original.volume,
            "Create-only inline geometry is not rewritten by Update"
        );
        assert_eq!(record.padding.to_bits(), original.padding.to_bits());
        assert_eq!(record.surface_id, original.surface_id);
        assert_eq!(record.group, 6);
        let part = usize::from(record.rigid_body_index);
        assert_eq!(record.material, *state.material(part));
        let enabled = if has_hat && part == 1 {
            original.enabled
        } else {
            part != 0
        };
        assert_eq!(record.enabled, enabled);
    }
    assert_eq!(state.assembly_group, 6);
    assert_eq!(state.part_user_data, [6; 24]);
    assert!(Arc::ptr_eq(
        &state.current_culling,
        binding.initial_culling()
    ));
    assert!(!state.is_ragdoll);
    assert_eq!(state.disable_counts[0], Some(0));
    assert_eq!(&state.disable_counts[1..], &[None; 23]);
    assert_eq!(state.volume_disable_count_active, None);
    assert_eq!(state.volume_headers[0].group_id, 4);
    assert_eq!(state.volume_headers[0].flags, initial_headers[0].flags & !1);
    let mut expected_compliance = [0; 24];
    expected_compliance[0] = 1;
    for part in 0..24 {
        let material = state.material(part);
        let words = [
            material.static_friction,
            material.dynamic_friction,
            material.restitution,
        ]
        .map(f32::to_bits);
        if part == 0 {
            assert_eq!(words, [0; 3]);
        } else {
            assert_eq!(
                state.material_selection[part],
                SkeletonPartMaterialSelection::Normal
            );
            let normal = body.attributes.normal;
            assert_eq!(
                words,
                [
                    normal.static_friction,
                    normal.dynamic_friction,
                    normal.restitution
                ]
                .map(f32::to_bits)
            );
            assert_eq!(state.volume_headers[part].group_id, 0);
            assert_eq!(
                state.volume_headers[part].flags,
                initial_headers[part].flags | 1
            );
            let drive = body.attributes.drives[part];
            expected_compliance[part] = drive.collision_compliant;
            assert_eq!(
                state.collision.priority[part].to_bits(),
                drive.collision_priority.to_bits()
            );
        }
    }
    assert_eq!(state.collision.priority[0].to_bits(), 0);
    assert_eq!(state.collision.is_compliant, expected_compliance);
    assert_eq!(state.collision.specific_parts, original_specific);
    let normal_headers = state.volume_headers;
    let normal_materials = state.material_selection;
    assembly.teleport_reset();
    let state = assembly.state();
    assert_eq!(state.collision.priority.map(f32::to_bits), [0; 24]);
    assert_eq!(state.collision.is_compliant, expected_compliance);
    assert_eq!(state.collision.specific_parts, original_specific);
    assert_eq!(state.disable_counts, [Some(0); 24]);
    assert_eq!(state.volume_disable_count_active, Some(false));
    assert!(!state.is_ragdoll);
    assert_eq!(state.volume_headers, normal_headers);
    assert_eq!(state.material_selection, normal_materials);
    assert_eq!(state.assembly_group, 6);
    assert_eq!(state.part_user_data, [6; 24]);
    assert_eq!(backend.rigid_body_snapshot(), expected_bodies);
    assert_eq!(backend.joint_snapshot(), expected_spies);
    if let Some(children) = hat_children {
        let SkeletonPartGeometry::Hat(hat) = &body.parts[1].geometry else {
            panic!("retained hat owner");
        };
        assert_eq!(hat.children().as_slice(), children.as_slice());
    }
    // Execute only the recovered 24-Part Skeleton::Set prefix. Its retained
    // original RAW pose is still a controlled fixture; the later two hook and
    // proxy-target updates of full Set are not silently supplied here.
    crate::dynamic_assembly_session::skeleton_normal::placement_tests::check_retained_pose_placement(
        assembly, backend,
    );
    publication.try_refresh(assembly, backend).unwrap();
    assert_eq!(publication.rigid_bodies(), original_observations);
    assert_eq!(publication.records().len(), original_records.len());
    for (record, original) in publication.records().iter().zip(&original_records) {
        let part = usize::from(record.rigid_body_index);
        assert_eq!(record.volume, original.volume);
        assert_eq!(record.rigid_body_index, original.rigid_body_index);
        assert_eq!(
            record.transform.map(f32::to_bits),
            assembly
                .session()
                .try_live_part_transform_host(backend, part)
                .unwrap()
                .map(f32::to_bits)
        );
        assert_eq!(record.material, *assembly.state().material(part));
    }
}

fn joint_frame_words(value: SimulationJointFrames) -> [u32; 20] {
    let vectors = [
        value.quaternion_a,
        value.position_a,
        value.quaternion_b,
        value.position_b,
        value.quaternion_l,
    ];
    std::array::from_fn(|lane| vectors[lane / 4][lane % 4].to_bits())
}

/// Original22 frame/limit records bound to actual pool occurrences. Identity
/// poses and default inertia below are explicitly controlled diagnostic inputs
/// solely to acquire real body handles: this is NOT `SkeletonBody` assembly
/// admission, original placement, collision publication, or a physics tick.
#[expect(
    clippy::too_many_lines,
    reason = "one original-data binding, source update, unchanged-field check and explicit cleanup lifecycle"
)]
fn check_normal_joint_storage(body: &SkeletonBodyDefinition, original: &SkeletonBodyJoints) {
    let mut bodies = SimulationRigidBodyPool::try_new(24).unwrap();
    let fixture_inertia = Arc::new(SimulationInertia::native_default());
    let handles: Vec<_> = (0..24)
        .map(|_| {
            bodies
                .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&fixture_inertia), 4)
                .unwrap()
                .unwrap()
        })
        .collect();
    let mut pool =
        SimulationJointPool::<SimulationJointFrames, SimulationJointLimits>::try_new(22).unwrap();
    let bindings: Vec<_> = original
        .joints
        .iter()
        .map(|joint| {
            let limits = SimulationMutableJointLimits::new(joint.limits);
            let handle = pool
                .try_add_joint_mutable_limits(
                    handles[usize::try_from(joint.parent_part).unwrap()],
                    handles[usize::try_from(joint.child_part).unwrap()],
                    &Arc::new(joint.frames),
                    &limits,
                )
                .unwrap()
                .unwrap();
            SkeletonJointLimitBinding {
                joint: handle,
                limits,
            }
        })
        .collect();
    assert_eq!(bindings.len(), 22);
    let before_bodies = bodies.snapshot();
    let before_joints = pool.snapshot();
    // Independent arithmetic from retained source inputs, not producer values.
    // Normal differs from InitJoints in multiplication operand order. The
    // source subtraction chooses a branch only; the selected value is the
    // ORIGINAL product, with .01 as floor, including unordered fallback.
    let epsilon = f32::from_bits(0x3c23_d70a);
    let expected_angles: Vec<_> = original
        .joints
        .iter()
        .enumerate()
        .map(|(row, joint)| {
            let bone = &body.data.parameters().bones[usize::try_from(joint.child_part).unwrap()];
            let attributes = body.attributes.joints[row];
            let swing = attributes.swing_angle * bone.swing;
            let twist = bone.twist * attributes.twist_angle;
            [
                if swing - epsilon >= 0.0 {
                    swing
                } else {
                    epsilon
                },
                if twist - epsilon >= 0.0 {
                    twist
                } else {
                    epsilon
                },
            ]
        })
        .collect();
    let mut calls = 0;
    try_set_normal_joint_limits(
        &body.data,
        &body.attributes,
        &mut pool,
        &bindings,
        &mut |angle| {
            assert_eq!(
                angle.to_bits(),
                expected_angles[calls / 2][calls % 2].to_bits()
            );
            calls += 1;
            // Explicit existing host scalar-cosine service, not captured Xenon bits.
            Ok::<_, std::convert::Infallible>(angle.cos())
        },
    )
    .unwrap();
    assert_eq!(calls, 44);
    assert_eq!(bodies.snapshot(), before_bodies);
    assert_eq!(pool.snapshot(), before_joints);
    for ((binding, joint), [swing, twist]) in
        bindings.iter().zip(&original.joints).zip(&expected_angles)
    {
        let mut expected = joint.limits;
        expected.swing_a = *swing;
        expected.twist_a = *twist;
        expected.swing_c = swing.cos();
        expected.twist_c = twist.cos();
        assert_eq!(
            joint_limit_words(pool.try_limits(binding.joint).unwrap()),
            joint_limit_words(expected)
        );
        assert_eq!(
            joint_limit_words(binding.limits.try_read().unwrap()),
            joint_limit_words(expected)
        );
    }
    // Read actual Active pool frames and endpoint observations too, rather than
    // asserting only against retained constructor copies.
    pool.try_with_active_build_occurrences_and_reservation(
        |occurrences| {
            for (ordinal, occurrence) in occurrences {
                let joint = &original.joints[ordinal];
                assert_eq!(
                    occurrence.body_b(),
                    handles[usize::try_from(joint.parent_part).unwrap()].observation()
                );
                assert_eq!(
                    occurrence.body_a(),
                    handles[usize::try_from(joint.child_part).unwrap()].observation()
                );
                let actual_frames = occurrence.frames();
                for (actual, expected) in [
                    actual_frames.quaternion_a,
                    actual_frames.position_a,
                    actual_frames.quaternion_b,
                    actual_frames.position_b,
                    actual_frames.quaternion_l,
                ]
                .into_iter()
                .zip([
                    joint.frames.quaternion_a,
                    joint.frames.position_a,
                    joint.frames.quaternion_b,
                    joint.frames.position_b,
                    joint.frames.quaternion_l,
                ]) {
                    assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
                }
                assert_eq!(
                    joint_limit_words(occurrence.try_limits()?),
                    joint_limit_words(bindings[ordinal].limits.try_read()?)
                );
            }
            Ok::<_, crate::simulation_joint_pool::SimulationJointPoolError>(())
        },
        |seen, capacity| seen.try_reserve_exact(capacity).map_err(|_| ()),
    )
    .unwrap_or_else(|_| panic!("original current Joint payloads must remain readable"));
    for handle in handles {
        bodies.try_remove_rigid_body(handle).unwrap();
    }
    for binding in bindings {
        pool.try_remove_joint(binding.joint).unwrap();
    }
    assert_eq!(bodies.counts().free, 24);
    assert_eq!(pool.counts().free, 22);
}

fn joint_limit_words(value: SimulationJointLimits) -> [u32; 16] {
    let mut words = [0; 16];
    words[..4].copy_from_slice(&value.p_prism.map(f32::to_bits));
    words[4..8].copy_from_slice(&value.v_prism.map(f32::to_bits));
    words[8..14].copy_from_slice(
        &[
            value.v_twist,
            value.v_swing,
            value.swing_a,
            value.twist_a,
            value.swing_c,
            value.twist_c,
        ]
        .map(f32::to_bits),
    );
    words[14] = value.raw_swing_type;
    words[15] = value.raw_twist_type;
    words
}
