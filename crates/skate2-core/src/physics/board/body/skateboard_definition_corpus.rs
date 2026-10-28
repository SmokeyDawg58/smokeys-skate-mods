//! Raw mounted Vault -> original authored wheel/truck/deck definition sequence.
//! Extends through local seven-body/six-joint/two-truck-drive admission and
//! teardown plus retained SkaterPool/HookPool and original-terrain frames.
//! This is not complete Island/player spawn/Tick or a Xenon trace.
//! Source words below are assertions only, never construction inputs.

use crate::{
    attribute_runtime_corpus::mounted_physics_attributes_with_classes,
    dynamic_assembly_session::{
        DynamicAssemblyAttachError, DynamicAssemblyDetachError, DynamicAssemblySession,
        DynamicAssemblyTemplateError,
        hook_pool::SimulationHookAssemblyCandidate,
        skater_pool::{
            SimulationSkaterAssemblyCandidate, SimulationSkaterAssemblyHandle,
            SimulationSkaterPoolError,
        },
    },
    simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession,
    skateboard_assembly::{
        SkateboardAssemblyDriveAdmission, SkateboardAssemblySession, SkateboardCollisionDefinition,
        SkateboardSimulationBackend, try_bind_skateboard_assembly,
    },
    skateboard_attributes::{
        SkateboardAttributeClass as Class, SkateboardAttributeRead, SkateboardAttributes,
    },
    skateboard_body_construction::{SkateboardBodyConstruction, SkateboardBodyConstructionPhase},
    skateboard_deck::{HostSkateboardDeckNumeric, SkateboardDeckDefinition},
    skateboard_hook::{SkateboardHookConstruction, SkateboardHookDefinition},
    skateboard_initial_transforms::{
        HostSkateboardInitialTransformNumeric, initialize_skateboard_culling_table,
        try_initialize_skateboard_transforms,
    },
    skateboard_joints::{
        HostSkateboardJointNumeric, try_calc_truck_transforms, try_create_skateboard_joints,
    },
    skateboard_truck_drives::{HostSkateboardTruckDriveNumeric, SkateboardTruckDrives},
    skateboard_trucks::SkateboardTruckDefinitions,
    skateboard_wheels::{
        FRESH_VOLUME_TRANSFORM, HostSkateboardWheelMassNumeric, SkateboardWheelAttributes,
        SkateboardWheelDefinitions,
    },
};
use skate2_formats::collision::VolumeType;
use std::sync::Arc;

#[path = "../runtime/skateboard_ground_corpus_checks.rs"]
mod ground_checks;

#[path = "skateboard_pool_corpus_checks.rs"]
mod pool_checks;

fn check_debug_color_projection(constructed: &mut SkateboardBodyConstruction) {
    let mut colors = constructed.debug_colors().unwrap().lock().unwrap();
    assert!(
        colors
            .colors()
            .iter()
            .all(|color| color.map(f32::to_bits) == [0x3f80_0000; 4])
    );
    // Original deck has thirteen objects, after four wheels and two trucks.
    colors.try_recolor(6, true).unwrap();
    for (index, color) in colors.colors().iter().enumerate() {
        let expected = if (6..19).contains(&index) {
            [0x3f19_999a, 0x3f19_999a, 0x3f19_999a, 0x3f80_0000]
        } else {
            [0x3f80_0000; 4]
        };
        assert_eq!(color.map(f32::to_bits), expected);
    }
    for part in 0..7 {
        colors.try_recolor(part, false).unwrap();
    }
    assert!(colors.colors()[..19].iter().all(
        |color| color.map(f32::to_bits) == [0x3dcc_cccd, 0x3dcc_cccd, 0x3e4c_cccd, 0x3f80_0000]
    ));
    assert!(
        colors.colors()[19..]
            .iter()
            .all(|color| color.map(f32::to_bits) == [0x3f80_0000; 4])
    );
    let previous = colors.colors().map(|color| color.map(f32::to_bits));
    assert_eq!(colors.try_recolor(7, true), Err(7));
    assert_eq!(
        colors.colors().map(|color| color.map(f32::to_bits)),
        previous
    );
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
#[expect(
    clippy::float_cmp,
    clippy::too_many_lines,
    reason = "single source-ordered mounted construction proof with exact copied-value assertions"
)]
fn mounted_vault_creates_wheels_trucks_and_thirteen_child_deck() {
    let vault = mounted_physics_attributes_with_classes(&[
        Class::Deck.key(),
        Class::TrucksDrives.key(),
        Class::Trucks.key(),
        Class::Wipeout.key(),
    ]);
    // Independent extracted-data source words, rechecked against raw mounted
    // storage. Enables inspect the first byte, not a big-endian integer bool.
    for (class, key, expected) in [
        (Class::TrucksDrives, 0xa9f9_72c5_5a52_8cb7, 0x3f80_0000),
        (Class::TrucksDrives, 0x71be_814c_d51a_993f, 0x4220_0000),
        (Class::TrucksDrives, 0xaa32_fea0_0e7d_0921, 0x3f80_0000),
        (Class::TrucksDrives, 0x22f5_4336_9220_c8c7, 0x450a_e000),
        (Class::Trucks, 0xb803_dfbe_2e4b_56fb, 0x3f09_999a),
        (Class::Trucks, 0x03ed_f4ad_8e17_41f6, 0x3fac_cccd),
        (Class::Wheels, 0x1264_6f5e_842c_4db3, 0x3cfd_f3b6),
        (Class::Wheels, 0x5a84_c435_dc34_0af8, 0x3dc2_8f5c),
        (Class::Trucks, 0x7e13_5b94_4834_707e, 0),
        (Class::Trucks, 0xb34a_6487_0f3f_5944, 0x3f66_6666),
        (Class::Trucks, 0x3298_05a8_2950_ab54, 0),
        (Class::Trucks, 0x8e3a_799a_0726_0fd8, 0x3ec7_ae14),
        (Class::World, 0x4650_7e40_c853_72fc, 0x40a0_0000),
        (Class::Deck, 0xd76d_9db5_ee1f_d008, 0x3e75_c28f),
        (Class::Deck, 0xc94d_6509_1dcb_828b, 0x3f17_0a3d),
        (Class::Deck, 0xceda_721a_4f02_55b6, 0x3c75_c28f),
        (Class::Deck, 0x49f4_29ae_2e66_f063, 0x3e28_f5c3),
        (Class::Deck, 0x4bcc_b0af_113e_6953, 5),
        (Class::Deck, 0x8020_3de9_f9b7_32ca, 0x4150_0000),
        (Class::Deck, 0x00e9_49f6_d567_bb10, 0x4148_0000),
        (Class::Deck, 0xac32_e9a7_f005_6c6d, 0x3dcc_cccd),
        (Class::Deck, 0xaae1_43f5_64f2_63aa, 0x3dcc_cccd),
        (Class::Deck, 0x254d_958a_72d5_9a2d, 0),
        (Class::Deck, 0x8d30_65ce_d858_a7df, 0x3ee6_6666),
        (Class::Deck, 0x01d8_0275_8fa1_c38a, 0x3f99_999a),
        (Class::Trucks, 0x7ca2_0106_5c78_f1cb, 0x41f8_0000),
        (Class::Trucks, 0xe8b3_4c12_d753_b164, 0x4160_0000),
        (Class::Trucks, 0x870b_a43e_d433_77d6, 0x42ac_3333),
        (Class::Trucks, 0x2be0_8dd7_a920_7622, 0xbd67_6c8b),
        (Class::Trucks, 0xe02d_cbd0_7830_d3a5, 0xbd54_fdf4),
        (Class::Trucks, 0xa69c_ca4e_15db_a369, 0xbd54_fdf4),
        (Class::Wheels, 0x1a90_ba4d_1195_7560, 0x4974_2400),
        (Class::Wipeout, 0x9904_48fc_2357_51f6, 0x3eb8_51ec),
        (Class::Wipeout, 0xd0fb_2f7d_77b0_bea4, 0),
    ] {
        let instance = vault.instance(class.key(), SkateboardAttributes::DEFAULT_COLLECTION);
        assert_eq!(
            instance.try_scalar_word(key, 0).unwrap(),
            Some(expected),
            "{class:?} key {key:016x}"
        );
    }
    for (class, key) in [
        (Class::Trucks, 0xddca_edd8_fa8f_5529),
        (Class::Trucks, 0xb1cb_e3e9_29af_c8a0),
        (Class::Deck, 0x3273_d82d_b269_c8ca),
        (Class::Deck, 0x3322_84c4_e878_a7c5),
    ] {
        let instance = vault.instance(class.key(), SkateboardAttributes::DEFAULT_COLLECTION);
        assert_eq!(
            instance.try_get_data(key, 0).unwrap().unwrap().bytes(),
            &[1]
        );
    }
    for (class, key, expected) in [
        (Class::Deck, 0x103f_dc19_514d_4246, 1),
        (Class::Trucks, 0x7210_da40_dfb1_8272, 0),
    ] {
        let instance = vault.instance(class.key(), SkateboardAttributes::DEFAULT_COLLECTION);
        assert_eq!(
            instance.try_get_data(key, 0).unwrap().unwrap().bytes(),
            &[expected]
        );
    }
    let wheel_attributes = SkateboardWheelAttributes::bind(&vault);
    let gravity_layout = vault
        .physics(SkateboardAttributes::DEFAULT_COLLECTION)
        .try_physics_layout()
        .unwrap();
    assert_eq!(
        gravity_layout.origin,
        crate::attribute_runtime::AttributePhysicsLayoutOrigin::CachedLayout
    );
    assert_eq!(&gravity_layout.raw_bytes[12..16], &[0; 4]);
    let gravity = std::array::from_fn(|axis| {
        f32::from_be_bytes(
            gravity_layout.raw_bytes[axis * 4..axis * 4 + 4]
                .try_into()
                .unwrap(),
        )
    });
    assert_eq!(gravity.map(f32::to_bits), [0, 0xc11c_cccd, 0]);
    let mut attributes = SkateboardAttributes::bind(&vault);
    let mut constructed = Box::new(SkateboardBodyConstruction::new(&vault));
    drop(vault);
    constructed.try_construct().unwrap();
    check_constructed_inventory(&constructed, SkateboardBodyConstructionPhase::Complete);
    check_debug_color_projection(&mut constructed);
    let wheels = SkateboardWheelDefinitions::try_create(
        wheel_attributes.try_snapshot().unwrap(),
        0,
        &mut HostSkateboardWheelMassNumeric,
    )
    .unwrap();
    let trucks = SkateboardTruckDefinitions::try_create(
        &mut attributes,
        wheels.debug_end(),
        &mut HostSkateboardWheelMassNumeric,
    )
    .unwrap();
    let deck = SkateboardDeckDefinition::try_create(
        &mut attributes,
        &mut HostSkateboardDeckNumeric,
        trucks.debug_end(),
    )
    .unwrap();
    assert_eq!(wheels.parts().len() + trucks.parts().len() + 1, 7);
    assert_eq!(wheels.debug_end(), 4);
    assert_eq!(trucks.debug_end(), 6);
    assert_eq!(deck.debug_range(), 6..19);
    for (index, truck) in trucks.parts().iter().enumerate() {
        assert_eq!(usize::from(truck.ordinal()), index + 4);
        assert!(Arc::ptr_eq(truck.volume(), trucks.parts()[0].volume()));
        assert_eq!(truck.volume().volume_type(), Some(VolumeType::Capsule));
        assert_eq!(truck.volume().group_id, 0);
        assert_eq!(truck.volume().flags, 1);
        assert_eq!(truck.material().static_friction.to_bits(), 0x3f66_6666);
        assert_eq!(truck.material().dynamic_friction.to_bits(), 0);
        assert_eq!(truck.inertia().max_omega, f32::MAX);
        assert!(truck.inverse_body_transform().is_none());
        assert_eq!(truck.debug().index, u32::try_from(index).unwrap() + 4);
        assert!(
            truck
                .inertia()
                .inverse_tensor
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
        );
    }
    let aggregate = deck.aggregate();
    assert_eq!(aggregate.children().len(), 13);
    assert_eq!(aggregate.tag_bits(), 4);
    assert_eq!(deck.category_counts(), [5, 5, 3]);
    assert_eq!(deck.category_first_indices(), [4, 3, 0]);
    assert_eq!(aggregate.children()[0].volume_type(), Some(VolumeType::Box));
    for capsule in &aggregate.children()[1..3] {
        assert_eq!(capsule.volume_type(), Some(VolumeType::Capsule));
        assert_eq!(capsule.transform[..12], FRESH_VOLUME_TRANSFORM[..12]);
    }
    for (index, triangle) in aggregate.children()[3..].iter().enumerate() {
        assert_eq!(triangle.volume_type(), Some(VolumeType::Triangle));
        assert_eq!(triangle.flags, 0x3e3);
        assert_eq!(
            triangle.shape_words,
            [0x3f80_0000, 0xbf80_0000, 0x3f80_0000]
        );
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|lane| triangle.transform[12 + lane].to_bits()),
            [0; 4]
        );
        assert_eq!(triangle.transform[2].is_sign_negative(), index % 2 == 0);
    }
    assert_eq!(deck.material().static_friction.to_bits(), 0x3dcc_cccd);
    assert_eq!(deck.material().dynamic_friction.to_bits(), 0x3dcc_cccd);
    assert_eq!(
        deck.inertia().inverse_mass.to_bits(),
        (1.0_f32 / 6.0).to_bits()
    );
    assert_eq!(
        deck.inertia().angular_drag.to_bits(),
        (f32::from_bits(0x3ee6_6666) * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(deck.inverse_body_transform(), Some(&FRESH_VOLUME_TRANSFORM));
    assert_eq!(deck.inverse_tensor_w(), deck.inertia().inverse_tensor[0]);
    assert!(!deck.is_static_body());
    assert!(deck.principal_mass().volume.is_finite() && deck.principal_mass().volume > 0.0);
    assert!(
        deck.inertia()
            .inverse_tensor
            .iter()
            .all(|value| value.is_finite() && *value > 0.0)
    );
    assert_eq!(deck.integral().columns[3][3], deck.principal_mass().volume);
    println!(
        "raw mounted Vault -> 4 wheels + 2 trucks + 13-child deck; debug 0..19; host deck volume={} diagonal={:?} inverse_tensor={:?} bounds={:?}; not registered board or native numeric trace",
        deck.principal_mass().volume,
        deck.principal_mass().diagonal,
        deck.inertia().inverse_tensor,
        aggregate.bounds()
    );
    // The Init enable byte above establishes the shipped six-joint branch.
    // This harness composes its recovered constructor calls, not the missing
    // complete Init/Island/Actor lifecycle.
    let joints =
        try_create_skateboard_joints(&mut attributes, &mut HostSkateboardJointNumeric).unwrap();
    for (ordinal, (joint, (parent, child))) in joints
        .iter()
        .zip([(6, 4), (6, 5), (4, 0), (4, 1), (5, 2), (5, 3)])
        .enumerate()
    {
        assert_eq!((joint.parent_part, joint.child_part), (parent, child));
        assert_eq!(joint.frames.position_a.map(f32::to_bits), [0; 4]);
        assert_eq!(
            (joint.limits.raw_swing_type, joint.limits.raw_twist_type),
            if ordinal < 2 { (0, 1) } else { (4, 0) }
        );
        assert!(
            joint
                .frames
                .quaternion_a
                .into_iter()
                .chain(joint.frames.quaternion_b)
                .chain(joint.frames.quaternion_l)
                .all(f32::is_finite)
        );
        for prior in &joints[..ordinal] {
            assert!(!Arc::ptr_eq(&joint.frames, &prior.frames));
            assert!(!Arc::ptr_eq(&joint.limits, &prior.limits));
        }
    }
    let culling = initialize_skateboard_culling_table();
    let initial = try_initialize_skateboard_transforms(
        &mut attributes,
        &mut HostSkateboardInitialTransformNumeric,
    )
    .unwrap();
    assert_eq!(
        initial.ordered_updates().map(|(part, _)| part),
        [6, 0, 1, 2, 3, 4, 5]
    );
    let collision = Arc::new(SkateboardCollisionDefinition::from_components(
        wheels, trucks, deck, culling,
    ));
    check_local_admission(
        collision,
        &joints,
        &initial.by_part(),
        &mut attributes,
        gravity,
        constructed,
    );
}

fn check_constructed_inventory(
    owner: &SkateboardBodyConstruction,
    phase: SkateboardBodyConstructionPhase,
) {
    // Independent raw db.big inline nodes, inspected BEFORE this producer ran.
    // They are assertions, never constructor inputs or runtime tuning.
    assert_eq!(owner.phase(), phase);
    assert_eq!(owner.joint_count(), Some(6));
    assert_eq!(owner.wheel_distance().map(f32::to_bits), Some(0xbd67_6c8b));
    assert!(
        !owner.remain_on_reset(),
        "HiLOD completion/registration is not implemented here"
    );
    assert_eq!(owner.group(), Some(4));
    assert_eq!(owner.part_user_data(), Some([4; 7]));
    let materials = owner.materials();
    let standard = owner
        .standard_material_owners()
        .expect("completed actual material suffix");
    assert!(std::ptr::eq(standard[0].as_ref(), materials.deck()));
    assert!(std::ptr::eq(standard[1].as_ref(), materials.wheel()));
    assert!(std::ptr::eq(standard[2].as_ref(), materials.truck()));
    for (material, expected) in [
        (materials.wheel(), [0x3f4c_cccd, 0x3f33_3333, 0]),
        (materials.deck(), [0x3dcc_cccd, 0x3dcc_cccd, 0]),
        (materials.truck(), [0x3f66_6666, 0, 0]),
        (materials.wipeout(), [0x3eb8_51ec, 0x3eb8_51ec, 0]),
        (materials.wheel_grind(), [0; 3]),
        (materials.deck_grind(), [0; 3]),
        (materials.truck_grind(), [0; 3]),
    ] {
        assert_eq!(
            [
                material.static_friction,
                material.dynamic_friction,
                material.restitution
            ]
            .map(f32::to_bits),
            expected
        );
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one retained owner through preflight, rollback, attachment and teardown is the lifecycle discriminator"
)]
fn check_local_admission(
    collision: Arc<SkateboardCollisionDefinition>,
    joints: &[crate::skateboard_joints::SkateboardJointDefinition; 6],
    initial: &[[f32; 16]; 7],
    attributes: &mut SkateboardAttributes,
    gravity: [f32; 3],
    constructed: Box<SkateboardBodyConstruction>,
) {
    assert_eq!(
        (
            collision.culling.rows,
            collision.culling.columns,
            collision.culling.array_size
        ),
        (7, 7, 2)
    );
    assert_eq!(collision.culling.words, [0xffff_ffff, 0x0001_ffff]);
    let mut invalid_joints = joints.to_vec();
    invalid_joints[5].parent_part = 7;
    assert!(matches!(
        try_bind_skateboard_assembly(Arc::clone(&collision), invalid_joints),
        Err(DynamicAssemblyTemplateError::PartIndex {
            ordinal: 5,
            part_index: 7,
            ..
        })
    ));
    let template = try_bind_skateboard_assembly(Arc::clone(&collision), joints.to_vec()).unwrap();
    assert!(Arc::ptr_eq(template.collision(), &collision));
    assert_eq!(template.part_payloads().len(), 7);
    assert_eq!(template.joints().len(), 6);
    for (bound, authored) in template.joints().iter().zip(joints) {
        assert!(Arc::ptr_eq(&bound.frames, &authored.frames));
        assert!(Arc::ptr_eq(
            bound.limits.as_immutable().unwrap(),
            &authored.limits
        ));
    }
    assert!(
        template.drives().is_empty(),
        "native definition begins with zero drives, before later constructor-created truck/hook drives"
    );
    for index in 0..7 {
        let original = match index {
            0..=3 => collision.wheels.parts()[index].inertia(),
            4..=5 => collision.trucks.parts()[index - 4].inertia(),
            _ => collision.deck.inertia(),
        };
        assert_eq!(
            **template.part_payloads()[index]
                .inertia
                .as_immutable()
                .unwrap(),
            *original
        );
        let original_material = match index {
            0..=3 => collision.wheels.parts()[index].material(),
            4..=5 => collision.trucks.parts()[index - 4].material(),
            _ => *collision.deck.material(),
        };
        assert_eq!(template.part_payloads()[index].material, original_material);
        for prior in &template.part_payloads()[..index] {
            assert!(!Arc::ptr_eq(
                template.part_payloads()[index]
                    .inertia
                    .as_immutable()
                    .unwrap(),
                prior.inertia.as_immutable().unwrap()
            ));
        }
    }
    // Real source-owned definitions enter the same transaction even when a
    // pool runs out. No alternate fake collision template is manufactured.
    for (body_capacity, joint_capacity, expected) in [
        (
            6,
            6,
            DynamicAssemblyAttachError::PartUnavailable { ordinal: 6 },
        ),
        (
            7,
            5,
            DynamicAssemblyAttachError::JointUnavailable { ordinal: 5 },
        ),
    ] {
        let mut backend =
            SkateboardSimulationBackend::try_new(body_capacity, joint_capacity, 0).unwrap();
        let mut session =
            SkateboardAssemblySession::try_new(Arc::clone(&template), initial.to_vec()).unwrap();
        assert_eq!(session.attach(&mut backend, 4), Err(expected));
        assert!(!session.is_attached());
        assert_eq!(
            backend.rigid_body_snapshot().counts.free,
            u32::try_from(body_capacity).unwrap()
        );
        assert_eq!(backend.rigid_body_snapshot().counts.active, 0);
        assert_eq!(
            backend.joint_snapshot().counts.free,
            u32::try_from(joint_capacity).unwrap()
        );
        assert_eq!(backend.joint_snapshot().counts.active, 0);
    }
    let retained = Arc::downgrade(&collision);
    let mut session =
        SkateboardAssemblySession::try_new(Arc::clone(&template), initial.to_vec()).unwrap();
    session.board_collision_groups.try_initialize(7).unwrap();
    assert_eq!(
        session
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .copied()
            .collect::<Vec<_>>(),
        *initial
    );
    drop(collision);
    drop(template);
    assert!(retained.upgrade().is_some());
    let mut backend = SkateboardSimulationBackend::try_new(7, 6, 2).unwrap();
    session.attach(&mut backend, 4).unwrap();
    assert!(session.is_attached());
    let bodies = backend.rigid_body_snapshot();
    assert_eq!(bodies.counts.active, 7);
    assert_eq!(bodies.counts.free, 0);
    for (ordinal, body) in bodies.slots.iter().enumerate() {
        assert_eq!(body.id.index(), u32::try_from(ordinal).unwrap());
        assert_eq!(body.state, 4);
        assert_eq!(
            body.inertia.as_ref(),
            Some(
                session.template().part_payloads()[ordinal]
                    .inertia
                    .as_immutable()
                    .unwrap()
                    .as_ref()
            )
        );
        assert_eq!(
            body.center_of_mass.map(f32::to_bits),
            std::array::from_fn(|axis| initial[ordinal][12 + axis].to_bits())
        );
    }
    let live_joints = backend.joint_snapshot();
    assert_eq!(live_joints.counts.active, 6);
    for (slot, expected) in live_joints.slots.iter().zip(joints) {
        assert!(slot.frames_present && slot.limits_present);
        assert_eq!(
            slot.body_b.unwrap().id().index(),
            u32::try_from(expected.parent_part).unwrap()
        );
        assert_eq!(
            slot.body_a.unwrap().id().index(),
            u32::try_from(expected.child_part).unwrap()
        );
    }
    // Original constructor calculates these AFTER admission. They are not
    // mistaken for initial Part transforms or substituted for the actual drives.
    let transforms =
        try_calc_truck_transforms(attributes, &mut HostSkateboardJointNumeric).unwrap();
    assert!(transforms.back[14] > 0.0 && transforms.front[14] < 0.0);
    assert_eq!(transforms.back[12..], joints[0].frames.position_b);
    assert_eq!(transforms.front[12..], joints[1].frames.position_b);
    let mut truck_drives = SkateboardTruckDrives::new(transforms);
    truck_drives
        .try_create_fresh(
            attributes,
            &mut HostSkateboardTruckDriveNumeric,
            &mut SkateboardAssemblyDriveAdmission {
                session: &mut session,
                backend: &mut backend,
            },
        )
        .unwrap();
    assert_eq!(truck_drives.count(), 2);
    assert_eq!(truck_drives.add_results(), [Some(true), Some(true)]);
    let drive_slots = backend.drive_snapshot();
    assert_eq!(drive_slots.counts.active, 2);
    for (ordinal, slot) in drive_slots.slots.iter().enumerate() {
        assert_eq!(slot.body_b.unwrap().id().index(), 6);
        assert_eq!(
            slot.body_a.unwrap().id().index(),
            4 + u32::try_from(ordinal).unwrap()
        );
    }
    for (ordinal, (frames, dynamics)) in backend
        .active_drive_payloads_for_test()
        .unwrap()
        .into_iter()
        .enumerate()
    {
        assert_eq!(frames, *truck_drives.frames()[ordinal].read().unwrap());
        assert_eq!(dynamics, *truck_drives.dynamics()[ordinal].read().unwrap());
        assert_eq!(dynamics.angular.spring.to_bits(), 0x450a_e000);
        assert_eq!(dynamics.angular.damping.to_bits(), 0x4220_0000);
        assert_eq!(dynamics.angular.strength.to_bits(), 0x4560_fffe);
        assert_eq!(dynamics.angular.raw_type, 1);
        assert_eq!(
            [
                dynamics.linear.spring,
                dynamics.linear.damping,
                dynamics.linear.strength
            ]
            .map(f32::to_bits),
            [0; 3]
        );
        assert_eq!(dynamics.linear.raw_type, 0);
    }
    assert_eq!(session.detach(&mut backend), Err(crate::dynamic_assembly_session::DynamicAssemblyDetachError::PostAttachmentDrivesPresent));
    println!(
        "raw mounted authored board -> 7 source-placed Active bodies + 6 joints + 2 retained mutable truck drives; ordered endpoints and pool payloads verified; culling retained49bits; no SkaterPool/hook/constraint-tick/spawn claim"
    );
    session.remove_post_attachment_drives(&mut backend).unwrap();
    assert_eq!(backend.drive_snapshot().counts.free, 2);
    session.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.free, 7);
    assert_eq!(backend.joint_snapshot().counts.free, 6);
    assert!(
        retained.upgrade().is_some(),
        "detached session still owns its original collision definitions"
    );
    pool_checks::check(Arc::clone(session.template()), initial);
    crate::dynamic_assembly_session::skater_pool::mixed_tests::check_mounted_board_and_skeleton(
        session.template(),
        initial,
    );
    let zero_iteration = run_contactless_frames(
        Arc::clone(session.template()),
        initial,
        attributes,
        gravity,
        0,
        false,
        None,
    );
    let constrained = run_contactless_frames(
        Arc::clone(session.template()),
        initial,
        attributes,
        gravity,
        50,
        false,
        None,
    );
    assert!(
        constrained
            .iter()
            .zip(&zero_iteration)
            .any(|(solved, baseline)| {
                solved.slots[1..]
                    .iter()
                    .zip(&baseline.slots[1..])
                    .any(|(a, b)| {
                        a.quaternion.map(f32::to_bits) != b.quaternion.map(f32::to_bits)
                            || a.center_of_mass.map(f32::to_bits)
                                != b.center_of_mass.map(f32::to_bits)
                    })
            }),
        "real compiled constraints must affect the live pose versus integration-only control"
    );
    let with_hook = run_contactless_frames(
        Arc::clone(session.template()),
        initial,
        attributes,
        gravity,
        50,
        true,
        Some(constructed),
    );
    // Source HookDisableAngularDrive/LinearDrive (82b24c98/82b24cb8) use
    // exactly the fresh HARD2/zero params. Build strength*h*h caps are zero,
    // so the real third record is intentionally inert in this finite fixture.
    // Gameplay activation belongs to later original state transitions.
    for (hooked, unhooked) in with_hook.iter().zip(&constrained) {
        for (with, without) in hooked.slots[1..8].iter().zip(&unhooked.slots[1..8]) {
            assert_eq!(
                with.center_of_mass.map(f32::to_bits),
                without.center_of_mass.map(f32::to_bits)
            );
            assert_eq!(
                with.quaternion.map(f32::to_bits),
                without.quaternion.map(f32::to_bits)
            );
            assert_eq!(
                with.velocity.map(f32::to_bits),
                without.velocity.map(f32::to_bits)
            );
            assert_eq!(
                with.omega.map(f32::to_bits),
                without.omega.map(f32::to_bits)
            );
        }
    }
    drop(session);
    assert!(retained.upgrade().is_none());
}

#[expect(
    clippy::too_many_lines,
    reason = "one retained real-data construction/frame/ordered-teardown proof"
)]
fn run_contactless_frames(
    template: Arc<crate::skateboard_assembly::SkateboardRuntimeTemplate>,
    initial: &[[f32; 16]; 7],
    attributes: &mut SkateboardAttributes,
    gravity: [f32; 3],
    iterations: u32,
    with_hook: bool,
    construction: Option<Box<SkateboardBodyConstruction>>,
) -> Vec<crate::simulation_rigid_body_pool::SimulationRigidBodyPoolSnapshot> {
    // These are independent controlled host worlds. Zero iterations is an
    // integration-only discriminator, not original gameplay configuration.
    // Fifty is the original single-player cvar default; no live cvar service
    // or mutable World watcher is claimed by this harness.
    let hook_slots = usize::from(with_hook);
    let expected_drives = 2 + hook_slots;
    let mut world = SimulationAssemblyConstraintSession::<_, SkateboardHookDefinition>::try_new(
        8 + hook_slots,
        6,
        expected_drives,
        if with_hook { 256 } else { 0 }, // Controlled corpus Contact capacity.
    )
    .unwrap();
    let backend = world.backend_mut();
    backend.set_gravity(gravity);
    backend.set_cooldown(2);
    backend.set_minimum_energy(f32::from_bits(0x3727_c5ac));
    assert_eq!(backend.rigid_body_snapshot().counts.active, 0);
    assert_eq!(backend.rigid_body_snapshot().static_order.len(), 1);
    // Explicit current settings select the real nonparallel pool component;
    // they do not claim the missing flags3 adjacent humanoid/board scheduler.
    world.try_initialize_skater_pool(3, false).unwrap();
    let candidate = if let Some(construction) = construction {
        let candidate =
            SimulationSkaterAssemblyCandidate::try_from_construction(construction).unwrap();
        assert_eq!(
            candidate
                .session()
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .copied()
                .collect::<Vec<_>>(),
            initial.to_vec()
        );
        // A real preflight refusal must return constructor/session/reader intact.
        let mut uninitialized =
            SimulationAssemblyConstraintSession::<_, SkateboardHookDefinition>::try_new(1, 0, 0, 0)
                .unwrap();
        let original_owner = std::ptr::from_ref(candidate.construction().unwrap());
        let failed = uninitialized
            .try_insert_board(candidate, 4, -1.0)
            .unwrap_err();
        assert!(matches!(
            failed.error,
            SimulationSkaterPoolError::Uninitialized
        ));
        assert!(std::ptr::eq(
            failed.candidate.construction().unwrap(),
            original_owner
        ));
        assert!(!failed.candidate.session().is_attached());
        check_constructed_inventory(
            failed.candidate.construction().unwrap(),
            SkateboardBodyConstructionPhase::HandedOff,
        );
        failed.candidate
    } else {
        let session = SkateboardAssemblySession::try_new(template, initial.to_vec()).unwrap();
        session.board_collision_groups.try_initialize(7).unwrap();
        SimulationSkaterAssemblyCandidate::try_new(session).unwrap()
    };
    let constructed = candidate.construction().is_some();
    let retained_constructor = candidate.construction().map(std::ptr::from_ref);
    assert_eq!(candidate.world_index(), 0xffff);
    let board = world
        .try_insert_board(candidate, 4, f32::from_bits(0x3f7f_be77))
        .unwrap();
    assert_eq!(world.board_count(), 1);
    assert_eq!(world.board_world_index(&board).unwrap(), 0);
    assert!(world.board(&board).unwrap().is_attached());
    assert_eq!(world.board_publication(&board).unwrap().records().len(), 19);
    if let Some(owner) = world.board_construction(&board).unwrap() {
        check_constructed_inventory(owner, SkateboardBodyConstructionPhase::HandedOff);
        assert!(std::ptr::eq(owner, retained_constructor.unwrap()));
    }
    let level = if constructed {
        world
            .try_calc_constructed_board_truck_transforms(&board)
            .unwrap()
    } else {
        try_calc_truck_transforms(attributes, &mut HostSkateboardJointNumeric).unwrap()
    };
    let mut drives = SkateboardTruckDrives::new(level);
    if constructed {
        world
            .try_initialize_constructed_board_truck_drives(&board, &mut drives)
            .unwrap();
    } else {
        world
            .try_initialize_board_truck_drives(
                &board,
                &mut drives,
                attributes,
                &mut HostSkateboardTruckDriveNumeric,
            )
            .unwrap();
    }
    let hook = if with_hook {
        // Original mounted switch controls the preceding optional-wheel branch.
        // Nonzero remains outside this fixture; no four drives are silently omitted.
        let wheel_switch = if constructed {
            world
                .try_constructed_board_optional_wheel_drive_byte(&board)
                .unwrap()
        } else {
            let pointer = attributes
                .lookup(Class::Wheels, 0x69ce_f3ca_3f14_80b0)
                .unwrap();
            attributes.byte(&pointer).unwrap()
        };
        assert_eq!(wheel_switch, 0);
        let mut construction = SkateboardHookConstruction::new();
        construction
            .try_prepare(
                &mut HostSkateboardTruckDriveNumeric,
                &mut HostSkateboardWheelMassNumeric,
            )
            .unwrap();
        let hook_session = DynamicAssemblySession::try_new(
            Arc::clone(construction.template().unwrap()),
            vec![FRESH_VOLUME_TRANSFORM],
        )
        .unwrap();
        let candidate = SimulationHookAssemblyCandidate::try_new(hook_session).unwrap();
        assert_eq!(candidate.world_index(), 0xffff);
        let handle = world.try_insert_hook(candidate).unwrap();
        let published = world.try_hook_snapshot(&handle).unwrap();
        // Both IDs come from actual publication in the same manager.
        assert_eq!(published.world_index, 1);
        assert_eq!(published.group_id, 1);
        assert_eq!(published.assembly_user_data, None);
        assert_eq!(
            (published.parts[0].world_index, published.parts[0].user_data),
            (0, 1)
        );
        assert!(published.bounds.iter().flatten().all(|v| v.to_bits() == 0));
        assert!(
            world
                .try_connect_registered_hook(
                    &board,
                    &handle,
                    construction.frames(),
                    construction.dynamics()
                )
                .unwrap()
        );
        let records = world.backend().drive_snapshot();
        let drive = &records.slots[records.active_order[2].index() as usize];
        assert_eq!(drive.body_b.unwrap().id().index(), 7);
        assert_eq!(drive.body_a.unwrap().id().index(), 8);
        assert!(matches!(world.try_remove_hook(&handle), Err(crate::dynamic_assembly_session::hook_pool::SimulationHookPoolError::Detach(
            crate::dynamic_assembly_session::DynamicAssemblyDetachError::IncomingCrossDrivesPresent))));
        Some((construction, handle))
    } else {
        None
    };
    assert_eq!(world.hook_count(), hook_slots);
    let before = world.backend().rigid_body_snapshot();
    assert_eq!(before.counts.active, 7);
    assert_eq!(before.counts.free, 0);
    let occurrences: Vec<_> = before.slots.iter().map(|slot| slot.occurrence).collect();
    let mut frames = Vec::new();
    for frame in 1..=4 {
        let counts = world
            .try_step_contactless(f32::from_bits(0x3c88_8889), iterations)
            .unwrap();
        assert_eq!(
            (counts.contacts, counts.joints, counts.drives),
            (0, 6, expected_drives)
        );
        assert!(
            world.active_rf_is_zero_for_test(),
            "integration clears the actual live RF rows"
        );
        let bodies = world.backend().rigid_body_snapshot();
        assert_eq!(
            bodies
                .slots
                .iter()
                .map(|slot| slot.occurrence)
                .collect::<Vec<_>>(),
            occurrences
        );
        assert_eq!(bodies.counts.active, 7);
        assert_eq!(bodies.static_order.len(), 1 + hook_slots);
        for body in &bodies.slots[1..8] {
            assert!(
                body.center_of_mass
                    .into_iter()
                    .chain(body.quaternion)
                    .chain(body.velocity)
                    .chain(body.omega)
                    .all(f32::is_finite)
            );
            assert_eq!(body.force.map(f32::to_bits), gravity.map(f32::to_bits));
            assert_eq!(body.torque.map(f32::to_bits), [0; 3]);
        }
        if with_hook {
            let drive_records = world.drive_words_for_test();
            assert_eq!(drive_records.len(), 3);
            let hook_record = &drive_records[2];
            // Actual compiled/solved third record: six limit W fields, then
            // the two accumulated impulse XYZ fields. Readback is not input.
            for row in [6, 7, 8, 9, 10, 11] {
                assert_eq!(hook_record[row][3], 0);
            }
            assert_eq!(&hook_record[2][..3], &[0; 3]);
            assert_eq!(&hook_record[3][..3], &[0; 3]);
            assert_eq!(
                bodies.slots[8], before.slots[8],
                "Static hook state is not integrated or reset"
            );
            assert!(bodies.slots[8].inertia.is_none());
            let labels = world.island_labels_for_test();
            assert_eq!(labels.iter().find(|(slot, _)| *slot == 8).unwrap().1, -1);
            for slot in 1..8 {
                assert_eq!(
                    labels.iter().find(|(index, _)| *index == slot).unwrap().1,
                    0
                );
            }
        }
        println!(
            "authored constraint frame={frame} iterations={iterations} C/J/D=0/6/{expected_drives} HookPool={with_hook} deckCOM={:?} deckQ={:?}; actual retained pools, source drive normalization, real RF cleared; deliberately contactless/pair-free, not complete spawn or original whole Tick",
            bodies.slots[7].center_of_mass, bodies.slots[7].quaternion
        );
        frames.push(bodies);
    }
    let mut relocated_hook = hook
        .as_ref()
        .map(|(_, handle)| check_live_spawn_and_solve(&mut world, &board, handle));
    if let Some((_, handle)) = hook.as_ref() {
        relocated_hook = Some(ground_checks::check(&mut world, &board, handle));
    }
    // Pool membership cannot bypass the retained post-admission drive owners.
    let before_refused_remove = world.backend().rigid_body_snapshot();
    assert!(matches!(
        world.try_remove_board(&board),
        Err(SimulationSkaterPoolError::Detach(
            DynamicAssemblyDetachError::PostAttachmentDrivesPresent
        ))
    ));
    assert_eq!(world.backend().rigid_body_snapshot(), before_refused_remove);
    assert_eq!(world.board_count(), 1);
    world.try_remove_board_drives(&board).unwrap();
    let retained_hook_frames = hook.map(|(construction, handle)| {
        let geometry = Arc::downgrade(construction.definition().unwrap());
        let frames = Arc::downgrade(construction.frames());
        let removed = world.try_remove_hook(&handle).unwrap();
        assert!(!removed.session().is_attached());
        assert_eq!(removed.world_index(), 1);
        // Detach copies live body pose into cache. The original requested cache
        // is separately checked in the owner-level tests before detach.
        assert_eq!(
            removed
                .session()
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .next()
                .unwrap()[12..15],
            relocated_hook.unwrap()[12..15]
        );
        drop(removed);
        drop(construction);
        assert!(geometry.upgrade().is_none());
        assert!(
            frames.upgrade().is_some(),
            "removed Drive pool slots retain payload owners until reuse/drop"
        );
        frames
    });
    assert_eq!(world.hook_count(), 0);
    let removed_board = world.try_remove_board(&board).unwrap();
    assert_eq!(removed_board.world_index(), 0);
    assert!(!removed_board.session().is_attached());
    assert_eq!(world.board_count(), 0);
    assert!(matches!(
        world.board(&board),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    assert!(matches!(
        world.board_publication(&board),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    let after_remove = world.backend().rigid_body_snapshot();
    assert!(matches!(
        world.try_remove_board(&board),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    assert_eq!(world.backend().rigid_body_snapshot(), after_remove);
    if constructed {
        let retained = removed_board
            .into_session()
            .err()
            .expect("cannot discard real constructor state");
        let owner = retained.construction().unwrap();
        assert!(std::ptr::eq(owner, retained_constructor.unwrap()));
        check_constructed_inventory(owner, SkateboardBodyConstructionPhase::HandedOff);
        assert!(!retained.session().is_attached());
    } else {
        let detached_board = removed_board
            .into_session()
            .ok()
            .expect("legacy session-only fixture");
        assert!(!detached_board.is_attached());
    }
    assert_eq!(
        world.backend().rigid_body_snapshot().counts.free,
        7 + u32::try_from(hook_slots).unwrap()
    );
    assert_eq!(world.backend().rigid_body_snapshot().static_order.len(), 1);
    assert_eq!(world.backend().joint_snapshot().counts.free, 6);
    assert_eq!(
        world.backend().drive_snapshot().counts.free,
        u32::try_from(expected_drives).unwrap()
    );
    drop(world);
    if let Some(retained) = retained_hook_frames {
        assert!(retained.upgrade().is_none());
    }
    frames
}

fn check_live_spawn_and_solve(
    world: &mut SimulationAssemblyConstraintSession<
        skate2_formats::collision::AssemblyPartMaterial,
        SkateboardHookDefinition,
    >,
    board: &SimulationSkaterAssemblyHandle,
    hook: &crate::dynamic_assembly_session::hook_pool::SimulationHookAssemblyHandle,
) -> [f32; 16] {
    struct CurrentTolerance {
        reads: usize,
    }
    impl crate::skateboard_spawn::SkateboardSpawnServices for CurrentTolerance {
        fn current_orthogonal_tolerance(&mut self) -> f32 {
            self.reads += 1;
            // Explicit controlled cvar service, not a native live cvar owner.
            if self.reads == 1 { 0.01 } else { 0.02 }
        }
        fn orthogonality_assertion_failed(&mut self) {
            panic!("controlled spawn input should pass source assertion");
        }
    }
    let mut services = CurrentTolerance { reads: 0 };
    let mut requested = [
        0.0, 1.01, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 12.0, 8.0, -5.0, 0.0,
    ];
    for relocation in 0_u8..2 {
        requested[12] += f32::from(relocation);
        let before = world.backend().rigid_body_snapshot();
        let joints = world.backend().joint_snapshot();
        let drives = world.backend().drive_snapshot();
        let cached = *world
            .board(board)
            .unwrap()
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .nth(6)
            .unwrap();
        let live = world
            .board(board)
            .unwrap()
            .try_live_part_transform_host(world.backend(), 6)
            .unwrap();
        assert_ne!(
            cached.map(f32::to_bits),
            live.map(f32::to_bits),
            "integrated live deck must not be replaced by the stored cache"
        );
        world
            .try_set_registered_board_transform(board, hook, requested, &mut services)
            .unwrap();
        let after = world.backend().rigid_body_snapshot();
        let mut expected = before.clone();
        for (expected, actual) in expected.slots[1..].iter_mut().zip(&after.slots[1..]) {
            expected.right = actual.right;
            expected.up = actual.up;
            expected.at = actual.at;
            expected.center_of_mass = actual.center_of_mass;
            expected.quaternion = actual.quaternion;
            if expected.inertia.is_some() {
                expected.inverse_inertia_full = actual.inverse_inertia_full;
                expected.inverse_inertia_split = actual.inverse_inertia_split;
                expected.inverse_mass = actual.inverse_mass;
            }
        }
        assert_eq!(
            after, expected,
            "all non-pose/tensor state must survive relocation"
        );
        assert_eq!(world.backend().joint_snapshot(), joints);
        assert_eq!(world.backend().drive_snapshot(), drives);
        assert_eq!(
            after.slots[7].center_of_mass.map(f32::to_bits),
            [requested[12], requested[13], requested[14]].map(f32::to_bits)
        );
        assert_eq!(
            after.slots[8].center_of_mass.map(f32::to_bits),
            after.slots[7].center_of_mass.map(f32::to_bits)
        );
        assert!(world.active_rf_is_zero_for_test());
        let counts = world
            .try_step_contactless(f32::from_bits(0x3c88_8889), 50)
            .unwrap();
        assert_eq!((counts.contacts, counts.joints, counts.drives), (0, 6, 3));
        let solved = world.backend().rigid_body_snapshot();
        assert_eq!(solved.slots[8], after.slots[8]);
        for body in &solved.slots[1..8] {
            assert!(
                body.center_of_mass
                    .into_iter()
                    .chain(body.quaternion)
                    .chain(body.velocity)
                    .chain(body.omega)
                    .all(f32::is_finite)
            );
        }
        assert!(world.active_rf_is_zero_for_test());
    }
    assert_eq!(services.reads, 2);
    println!(
        "original-data pool-owned board/hook -> two controlled in-place relocations after live solver frames -> unchanged occurrences/motion/constraints -> C/J/D 0/6/3 solve; not native spawn location, whole Island/Tick or trace parity"
    );
    requested
}
