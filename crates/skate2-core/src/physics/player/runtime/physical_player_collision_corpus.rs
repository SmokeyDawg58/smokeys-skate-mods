//! Actual original paired owners -> resident terrain -> solved Skeleton spies
//! -> SAME retained history and normal collision state. The original RAW pose
//! is uniformly translated in memory as a controlled terrain-contact fixture;
//! no geometry, contact, native address or live Actor pose is manufactured.
use super::*;
#[path = "../skeleton/physical_player_skeleton_lines_corpus.rs"]
mod skeleton_lines;
use crate::simulation_drive_pool::{
    SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
};
use crate::skeleton_drives::{SkeletonDriveType, update::SkeletonDrivesUpdateCvars};
use crate::static_ground_pool::StaticGroundPool;
use crate::{
    simulation_assembly_backend::constraint_session::{
        contact_feedback::ControlledNoFreezing, skateboard_ground::SkateboardGroundFrameInput,
    },
    simulation_contact_jacobian::ground_preparation::corpus::mounted_robs_park_ground_for_board_test,
    skateboard_attributes::SkateboardAttributes,
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_collision_data::update::{SkeletonCollisionCvars, SkeletonCollisionUpdateInput},
    skeleton_collision_numeric::{
        self as numeric_graph, HostSkeletonCollisionNumeric, SkeletonCollisionNumeric,
    },
};
use std::sync::RwLock;

const TERRAIN: [f32; 3] = [588.8617, -54.431, 996.5048];

fn check_com_publication(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    use crate::physical_player_phys_out::{PhysOutOrigin, PhysOutSystemVector};
    // Native Skeleton.Fill reads outer3d00/3d20, not the physics history record.
    let records = world.skeleton(skeleton).unwrap().post_physics_records();
    let expected = [
        records.centre_of_mass_typed(),
        records.centre_of_mass_velocity_typed(),
    ];
    world
        .try_publish_pair_skeleton_com(board, skeleton)
        .unwrap();
    world
        .try_with_pair_phys_out(board, skeleton, |output| {
            for (field, expected) in [
                PhysOutSystemVector::CentreOfMass,
                PhysOutSystemVector::CentreOfMassVelocity,
            ]
            .into_iter()
            .zip(expected)
            {
                let actual = output.system_vector(field).unwrap();
                assert_eq!(actual.origin, PhysOutOrigin::Publication);
                assert_eq!(
                    actual.value.xyz.map(f32::to_bits),
                    expected.xyz.map(f32::to_bits)
                );
                assert_eq!(
                    actual.value.w.map(f32::to_bits),
                    expected.w.map(f32::to_bits)
                );
            }
        })
        .unwrap();
}
const TIMESTEP: f32 = f32::from_bits(0x3c88_8889);

struct Services;
impl SkateboardSpawnServices for Services {
    fn current_orthogonal_tolerance(&mut self) -> f32 {
        0.01
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("controlled original pose placement");
    }
}
impl PhysicalPlayerAssemblyServices for Services {
    fn current_entity_flags(&mut self) -> u32 {
        1
    }
}
struct DriveCvars;
impl SkeletonDriveCvars for DriveCvars {
    type Error = Infallible;
    fn hard_drive_displacement(&mut self) -> Result<f32, Self::Error> {
        Ok(1.0)
    }
    fn hard_drive_strength(&mut self) -> Result<f32, Self::Error> {
        Ok(10.0)
    }
}
#[derive(Default)]
struct HistoryInput {
    events: Vec<&'static str>,
}
impl SkeletonHistoryInput for HistoryInput {
    type Error = Infallible;
    type Up = [f32; 4];
    fn acquire_up_vector(&mut self) -> Result<Self::Up, Self::Error> {
        self.events.push("acquire");
        Ok([0.0, 1.0, 0.0, -0.0])
    }
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.events.push("mode");
        Ok(0)
    }
    fn read_up_vector(&mut self, up: &Self::Up) -> Result<[f32; 4], Self::Error> {
        self.events.push("up");
        Ok(*up)
    }
}
#[derive(Default)]
struct CollisionCvars {
    events: Vec<&'static str>,
}
impl SkeletonCollisionCvars for CollisionCvars {
    type Error = Infallible;
    type FenceTarget = ();
    fn capture_fence_threshold(
        &mut self,
        _: crate::skeleton_collision_data::update::FenceThreshold,
    ) -> Result<(), Infallible> {
        panic!("normal collision corpus must not read fence cvars")
    }
    fn read_fence_threshold(&mut self, (): &()) -> Result<f32, Infallible> {
        panic!("normal collision corpus must not read fence cvars")
    }

    fn draw_body_part_collision(&mut self) -> Result<u8, Self::Error> {
        self.events.push("draw-body");
        Ok(0)
    }
    fn draw_collision_update(&mut self) -> Result<u8, Self::Error> {
        self.events.push("draw-update");
        Ok(0)
    }
    fn show_ground_plane(&mut self) -> Result<u8, Self::Error> {
        self.events.push("show-ground");
        Ok(0)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        panic!("all controlled drawing switches are off");
    }
    fn allow_any_contact(&mut self) -> Result<u8, Self::Error> {
        self.events.push("any-contact");
        Ok(0)
    }
    fn maximum_ground_distance_offboard(&mut self) -> Result<f32, Self::Error> {
        panic!("danger-zone normal contacts bypass ground-plane filtering");
    }
    fn small_object_force_scalar(&mut self) -> Result<f32, Self::Error> {
        panic!("actual static terrain is not a small dynamic object");
    }
    fn debug_always_collide(&mut self) -> Result<u8, Self::Error> {
        self.events.push("always-collide");
        Ok(0)
    }
}

/// Source inputs remain the original verified records. Only the supplied pose
/// translation is controlled. Anchor the original Part15 volume centre using
/// its real physics offset and the source first packed-name hierarchy match.
fn terrain_inputs(source: &PhysicalPlayerAssemblyInputs) -> PhysicalPlayerAssemblyInputs {
    let bone = &source.parameters.bones[15];
    let index = source
        .hierarchy
        .packed_bone_names
        .iter()
        .position(|name| *name == bone.packed_name)
        .unwrap();
    let original = source.pose_matrices[index];
    let centre: [f32; 3] = std::array::from_fn(|lane| {
        let x = original[lane].mul_add(bone.position[0], original[12 + lane]);
        let y = original[4 + lane].mul_add(bone.position[1], x);
        original[8 + lane].mul_add(bone.position[2], y)
    });
    let target = [TERRAIN[0], TERRAIN[1] + 0.005, TERRAIN[2]];
    let translation: [f32; 3] = std::array::from_fn(|axis| target[axis] - centre[axis]);
    let pose: Vec<_> = source
        .pose_matrices
        .iter()
        .map(|matrix| {
            let mut translated = *matrix;
            for axis in 0..3 {
                translated[12 + axis] += translation[axis];
            }
            assert_eq!(
                translated[..12]
                    .iter()
                    .map(|v| v.to_bits())
                    .collect::<Vec<_>>(),
                matrix[..12].iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
            assert_eq!(translated[15].to_bits(), matrix[15].to_bits());
            translated
        })
        .collect();
    assert_ne!(pose[index][13].to_bits(), original[13].to_bits());
    let mut spawn = FRESH_VOLUME_TRANSFORM;
    spawn[12..15].copy_from_slice(&TERRAIN);
    PhysicalPlayerAssemblyInputs {
        cvars: source.cvars.clone(),
        vault: Arc::clone(&source.vault),
        hierarchy: Arc::clone(&source.hierarchy),
        parameters: Arc::clone(&source.parameters),
        pose_hierarchy: Arc::clone(&source.pose_hierarchy),
        pose_matrices: pose.into(),
        spawn,
        hat_type: source.hat_type,
        bouncy_ragdoll: source.bouncy_ragdoll,
        is_human: source.is_human,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one actual original paired construction/terrain/response/cleanup proof"
)]
fn check_mode(original: &Arc<PhysicalPlayerAssemblyOwner>, live_postphysics: bool) {
    let source = original.inputs();
    let layout = source
        .vault
        .physics(SkateboardAttributes::DEFAULT_COLLECTION)
        .try_physics_layout()
        .unwrap();
    let gravity = std::array::from_fn(|axis| {
        f32::from_be_bytes(layout.raw_bytes[axis * 4..axis * 4 + 4].try_into().unwrap())
    });
    assert_eq!(gravity.map(f32::to_bits), [0, 0xc11c_cccd, 0]);
    // Independently fetch the actual original scalar, not the new provider's
    // output and not a hardcoded replacement for runtime Vault tuning.
    let timer_word = source
        .vault
        .instance(0x9f86_2c2e_aa01_6ce5, 0xd7ed_bd36_2d7d_2152)
        .try_scalar_word(0x12ea_5351_a38f_d32e, 0)
        .unwrap()
        .unwrap();
    assert!(f32::from_bits(timer_word) > 0.0);
    let ground = mounted_robs_park_ground_for_board_test();
    let mut selected = PhysicalAssemblyWorld::try_new(35, 28, 51, 256).unwrap();
    selected.try_initialize_grab_query_manager(true).unwrap();
    selected.backend_mut().set_gravity(gravity);
    selected.backend_mut().set_cooldown(2);
    selected
        .backend_mut()
        .set_minimum_energy(f32::from_bits(0x3727_c5ac));
    selected.try_initialize_skater_pool(3, false).unwrap();
    let mut factory =
        PhysicalPlayerAssemblyConstruction::new(&mut selected, terrain_inputs(source));
    factory.try_construct_board(&mut Services).unwrap();
    factory
        .try_construct_skeleton(&mut Services, &mut || panic!("original tutorial hat0"))
        .unwrap();
    let mut history = HistoryInput::default();
    factory
        .try_finish_skeleton(&mut DriveCvars, &mut history)
        .unwrap();
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Complete);
    let board = factory.board().unwrap().clone();
    let skeleton = factory.skeleton().unwrap().clone();
    let hook = factory.state.hook.as_ref().unwrap().clone();
    let owner = Arc::clone(factory.owner());
    let mut post_records = super::post_records_corpus::PostRecordsCorpus::new(&owner);
    let world = factory.completed_world_mut().unwrap();
    assert_eq!(world.physical_body_registry().target_count(), 31);
    assert_eq!(world.backend().rigid_body_snapshot().counts.active, 31);
    assert_eq!(
        world.backend().rigid_body_snapshot().counts.static_bodies,
        4
    );
    assert_eq!(world.hook_count(), 2);
    for member in [&board, &skeleton] {
        assert!(Arc::ptr_eq(
            world.physical_body_member(member).unwrap().owner(),
            &owner
        ));
    }
    // Read-only diagnostic of the actual constructed fixture, before any solve.
    let part15 = world.skeleton(&skeleton).unwrap();
    let part15_transform = part15
        .session()
        .try_live_part_transform_host(world.backend(), 15)
        .unwrap();
    let part15_shape = part15.state().binding.body().parts[15].debug.clone();
    let part15_center_clearance = part15_transform[13] - TERRAIN[1];
    // An independent plane-support observation for the unambiguous shape cases.
    // Capsule axis conventions are deliberately left in the printed shape/matrix.
    let part15_support_clearance = match &part15_shape {
        crate::skeleton_body_definition::SkeletonPartDebugShape::Box { half_size } => Some(
            part15_center_clearance
                - (part15_transform[1].abs() * half_size[0]
                    + part15_transform[5].abs() * half_size[1]
                    + part15_transform[9].abs() * half_size[2]),
        ),
        crate::skeleton_body_definition::SkeletonPartDebugShape::Sphere { radius }
        | crate::skeleton_body_definition::SkeletonPartDebugShape::HatSphere { radius } => {
            Some(part15_center_clearance - radius)
        }
        crate::skeleton_body_definition::SkeletonPartDebugShape::Capsule { .. } => None,
    };
    // Skeleton.Set intentionally leaves constructor hooks root-relative. This
    // component fixture supplied WORLD-space pose rows, so admit those same
    // rows through actual UpdateHooks/reset before its first isolated solve.
    let skeleton_hook = world
        .skeleton(&skeleton)
        .unwrap()
        .drive_owner()
        .unwrap()
        .hook()
        .unwrap()
        .clone();
    let before_hook_positions: [_; 2] = std::array::from_fn(|part| {
        world
            .try_skeleton_hook_part_transform(&skeleton_hook, part)
            .unwrap()
            .columns()[3]
            .xyz
    });
    let before_body_positions = [23, 0].map(|part| {
        world
            .skeleton(&skeleton)
            .unwrap()
            .session()
            .try_live_part_transform_typed(world.backend(), part)
            .unwrap()
            .columns()[3]
            .xyz
    });
    assert!(
        before_hook_positions[0]
            .iter()
            .zip(before_body_positions[0])
            .any(|(target, body)| (target - body).abs() > 1.0)
    );
    let reset = world.admit_world_pose_hooks_for_test(&skeleton, &owner.inputs().pose_matrices);
    let after_hook_positions: [_; 2] = std::array::from_fn(|part| {
        world
            .try_skeleton_hook_part_transform(&skeleton_hook, part)
            .unwrap()
            .columns()[3]
            .xyz
    });
    let after_body_positions = [23, 0].map(|part| {
        world
            .skeleton(&skeleton)
            .unwrap()
            .session()
            .try_live_part_transform_typed(world.backend(), part)
            .unwrap()
            .columns()[3]
            .xyz
    });
    eprintln!(
        "collision fixture hook admission: before hooks={before_hook_positions:?} bodies={before_body_positions:?}; after hooks={after_hook_positions:?} bodies={after_body_positions:?}; reset={reset}"
    );
    assert!(reset, "source >1 hook relocation requests ResetSkeleton");
    for part in 0..2 {
        for lane in 0..3 {
            assert!(
                (after_hook_positions[part][lane] - after_body_positions[part][lane]).abs()
                    < 1.0e-4
            );
            assert!(
                (before_body_positions[part][lane] - after_body_positions[part][lane]).abs()
                    < 1.0e-4
            );
        }
    }

    let before = world.skeleton(&skeleton).unwrap().state().collision.clone();
    super::board_surface_corpus::before_frame(world, &board);
    let (counts, report) = world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: &ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            TIMESTEP,
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    assert_eq!((counts.joints, counts.drives), (28, 49));
    assert_eq!(report.processed_assemblies, 2);
    assert!(
        counts.contacts > 0,
        "actual original body penetration: {report:?}"
    );
    super::board_surface_corpus::completed(world, &board, &skeleton);
    if !world
        .try_get_physical_collisions(&skeleton)
        .unwrap()
        .iter()
        .any(|contact| contact.part() > 0)
    {
        eprintln!(
            "collision diagnostic: live_postphysics={live_postphysics} counts={counts:?} part15_transform={part15_transform:?} shape={part15_shape:?} center_clearance={part15_center_clearance:?} support_clearance={part15_support_clearance:?}"
        );
        for (index, record) in world.contact_materials_for_test().iter().enumerate() {
            let endpoints = record.bodies.map(|body| {
                world.physical_body_target(body).unwrap().map(|target| {
                    (
                        target.member().same_as(&skeleton),
                        target.member().same_as(&board),
                        target.part(),
                    )
                })
            });
            eprintln!(
                "collision diagnostic: solved[{index}] endpoints(skeleton,board,part)={endpoints:?} spy_word={:08x} normal_impulse={:?} bits={:08x}",
                record.spy_word,
                f32::from_bits(record.normal_impulse_bits),
                record.normal_impulse_bits
            );
        }
        if let Some(spies) = world.contact_spies().unwrap() {
            for (index, spy) in spies.records().iter().enumerate() {
                let endpoints = [spy.body_a(), spy.body_b()].map(|body| {
                    world.physical_body_target(body).unwrap().map(|target| {
                        (
                            target.member().same_as(&skeleton),
                            target.member().same_as(&board),
                            target.part(),
                        )
                    })
                });
                eprintln!(
                    "collision diagnostic: exported[{index}] endpoints={endpoints:?} force={:?}",
                    spy.normal_force()
                );
            }
        }
    }
    let contacts = world.try_get_physical_collisions(&skeleton).unwrap();
    assert!(
        contacts.iter().any(|contact| contact.part() > 0),
        "must reach a positive solved nonroot Skeleton contact, not only board/root: {report:?}"
    );
    let frame = Arc::clone(
        contacts
            .first()
            .expect("positive actual contacts")
            .spy_frame(),
    );
    for contact in contacts {
        assert!(contact.this_body().same_as(&skeleton));
        assert!(!contact.self_colliding());
        assert!(
            contact.other_body().is_none(),
            "real terrain is unregistered StaticWorld"
        );
        assert!(Arc::ptr_eq(contact.spy_frame(), &frame));
        assert!(std::ptr::eq(
            contact.spy(),
            &raw const frame.records()[contact.spy_index()]
        ));
    }
    // Test-only expected observations, not cloned/injectable production Info.
    let contacts: Vec<_> = contacts
        .iter()
        .map(|contact| (contact.part(), *contact.normal()))
        .collect();
    history.events.clear();
    let input = SkeletonCollisionUpdateInput {
        ignore_rigid_body: None,
        ground_position: TERRAIN,
        ground_normal: [0.0, 1.0, 0.0],
        system_velocity: [0.0, -5.0, 0.0],
        centre_of_mass_velocity: [0.0; 3],
        timestep: TIMESTEP,
        danger_zone: 1,
        ai_skater: 0,
        offboard: 0,
        offboard_air: 0,
        ground_trick: 0,
        request_partial_ragdoll: 0,
    };
    let mut cvars = CollisionCvars::default();
    post_records.prepare(world, &skeleton, &board, &owner.inputs().pose_matrices);
    if live_postphysics {
        // Dedicated occurrence: actual solved contacts and retained source inputs,
        // with the existing controlled pose/Reckoning preparation. The explicit
        // false is this component fixture's argument, not a recovered HiLOD gate.
        postphysics_live::check_live_postphysics(
            world,
            &board,
            &skeleton,
            &owner.inputs().cvars,
            &frame,
            false,
        );
        factory.try_teardown().unwrap();
        assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Removed);
        assert_eq!(factory.world().physical_body_registry().target_count(), 0);
        eprintln!(
            "actual Skeleton postphysics preparation/history/collision/records complete on original solved frame; controlled pose/Reckoning and caller argument; same-frame replay refused"
        );
        return;
    }
    let mut records_services = post_records.services();
    world
        .try_update_skeleton_normal_post_physics(
            &skeleton,
            &board,
            &mut history,
            &input,
            &mut cvars,
            0,
            &mut records_services,
        )
        .unwrap();
    records_services.assert_normal_calls();
    drop(records_services);
    post_records.check(world, &skeleton, 0, TIMESTEP);
    skeleton_lines::initialize_query_world(world);
    skeleton_lines::check(world, &board, &skeleton, &owner, &ground);
    super::phys_out_corpus::construct(world, &board, &skeleton);
    check_com_publication(world, &board, &skeleton);
    assert_eq!(history.events, ["acquire", "mode", "up"]);
    let actual_owner = world.skeleton(&skeleton).unwrap();
    assert!(Arc::ptr_eq(
        actual_owner.physics_history_frame_state().frame().unwrap(),
        &frame
    ));
    assert!(Arc::ptr_eq(
        actual_owner.collision_frame_state().frame().unwrap(),
        &frame
    ));
    assert_eq!(actual_owner.collision_frame_state().progress(),
        crate::dynamic_assembly_session::skeleton_normal::collision_owner::SkeletonCollisionFrameProgress::Complete);
    let actual = &actual_owner.state().collision;
    assert_eq!(
        actual.priority.map(f32::to_bits),
        before.priority.map(f32::to_bits)
    );
    assert_eq!(actual.is_compliant, before.is_compliant);
    assert_eq!(actual.collision_timer.to_bits(), timer_word);
    assert_eq!(actual.drive_strength.to_bits(), 0);
    assert_eq!(actual.compliant_active, 1);
    assert_eq!((actual.hit_world, actual.hit_vehicle), (1, 0));
    assert_eq!(actual.is_wiping_out, 0);
    assert_eq!(actual.time_in_wipeout.to_bits(), 0);
    assert!(
        actual.contact_force.iter().any(|force| *force > 0.0),
        "actual ongoing history must drive nonzero response, not only bookkeeping"
    );
    let maximum_priority = contacts.iter().fold(0.0_f32, |old, (part, _)| {
        let candidate = before.priority[*part as usize];
        if candidate > old { candidate } else { old }
    });
    let expected_normals: Vec<_> = contacts
        .iter()
        .filter(|(part, _)| before.priority[*part as usize].to_bits() == maximum_priority.to_bits())
        .collect();
    assert_eq!(
        usize::try_from(actual.num_normals).unwrap(),
        expected_normals.len()
    );
    for (normal, (part, expected)) in actual.normal_info.iter().zip(expected_normals) {
        assert_eq!(normal.part.index(), *part as usize);
        assert_eq!(
            normal.normal.xyz().map(f32::to_bits),
            expected.map(f32::to_bits)
        );
        assert!(
            normal.normal.known_lanes().is_none(),
            "packed native W was not fabricated"
        );
    }
    for (part, _) in &contacts {
        let part = *part as usize;
        assert_eq!(actual.just_collided[part], 1);
        assert_eq!(actual.time_since_collision[part].to_bits(), 0);
    }
    for (force, normal) in actual.contact_force.iter().zip(actual.contact_normal) {
        if *force > 0.0 {
            assert!(normal.known_lanes().is_none());
            assert!(
                contacts
                    .iter()
                    .any(|(_, expected)| expected.map(f32::to_bits)
                        == normal.xyz().map(f32::to_bits))
            );
        }
    }
    assert!(cvars.events.contains(&"always-collide"));
    assert!(
        !cvars.events.contains(&"show-ground"),
        "controlled danger-zone bypass is real"
    );
    check_group_winners(world, &skeleton, &input, &before);
    check_feedback_reaches_next_solve(world, &skeleton, &ground);
    assert!(
        world
            .try_publish_pair_skeleton_com(&board, &skeleton)
            .is_err(),
        "a later actual solve must invalidate the previous history publication"
    );
    // The controlled drive entry replaced mapped rows without adjustments.
    // Post-record consumers must not reuse its old completed-animation token.
    assert!(
        world
            .skeleton(&skeleton)
            .unwrap()
            .current_animation()
            .root_input(TIMESTEP)
            .is_none()
    );
    post_records.prepare(world, &skeleton, &board, &owner.inputs().pose_matrices);
    // The first frame's downward retained COM velocity is still current here.
    // Complete active landing before the second records update replaces it.
    super::landing_corpus::check(world, &skeleton, &board, &owner);
    super::landing_spring_corpus::check(world, &skeleton, &board, &owner);
    let mut records_services = post_records.services();
    world
        .try_update_skeleton_normal_post_physics(
            &skeleton,
            &board,
            &mut history,
            &input,
            &mut cvars,
            0x1000,
            &mut records_services,
        )
        .unwrap();
    records_services.assert_normal_calls();
    drop(records_services);
    post_records.check(world, &skeleton, 0x1000, TIMESTEP);
    super::grounded_corpus::check(world, &skeleton, &board, &owner, &post_records.reckoning());
    super::ik_complete_corpus::check(
        world,
        &skeleton,
        &board,
        &owner,
        &post_records.reckoning(),
        &ground,
    );
    super::board_forces_corpus::check(world, &board, &owner, &post_records.reckoning(), &ground);
    super::ground_force_tail_corpus::check(world, &board, &owner, &post_records.reckoning());
    super::board_steering_corpus::check(world, &board, &owner, &ground);
    let processed_owner = Arc::clone(owner.reckoning().lock().unwrap().processed());
    let mut processed = processed_owner.lock().unwrap();
    super::phys_out_corpus::check(world, &board, &skeleton, &mut processed);
    super::wheel_material_corpus::check(world, &board, &skeleton, &hook, &ground, &mut processed);
    super::wheel_lines_corpus::check(
        world,
        &board,
        &skeleton,
        &post_records.reckoning(),
        &ground,
        &mut processed,
    );
    super::surface_motion_corpus::prepare(world, &board, &mut processed);
    super::wheel_support_corpus::check(
        world,
        &board,
        &skeleton,
        &hook,
        &processed,
        &post_records.reckoning(),
        &ground,
    );
    drop(processed);
    super::surface_motion_corpus::check(world, &board, &skeleton, post_records.reckoning_phase());
    super::set_velocity_corpus::check(world, &board, &skeleton, &ground);
    super::controller_corpus::check(world, &board, &skeleton);
    super::standard_corpus::check(world, &board);
    super::skeleton_controller_corpus::check(world, &board, &skeleton);
    assert!(
        owner
            .reckoning()
            .lock()
            .unwrap()
            .try_completed(post_records.reckoning_phase())
            .is_ok(),
        "same retained Reckoning phase remains valid across the controlled engine solves"
    );
    factory.try_teardown().unwrap();
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Removed);
    assert_eq!(factory.world().physical_body_registry().target_count(), 0);
    assert_eq!(factory.world().physical_contact_list_entry_count(), 0);
    assert!(
        factory
            .world()
            .backend()
            .drive_snapshot()
            .active_order
            .is_empty()
    );
    println!(
        "mounted translated original pose + real 35-body/28-joint/49-drive pair -> RobPark ground -> positive nonroot Skeleton spies/list -> same ongoing history and normal response; original Vault timer, contact/priority/normal provenance and explicit unknown W; controlled current inputs, not live Actor/ragdoll/full Tick"
    );
}

#[derive(Default)]
struct UpdateCvars {
    damping_reads: usize,
    refuse_reads: bool,
}
impl SkeletonDrivesUpdateCvars for UpdateCvars {
    type Error = Infallible;
    fn offboard_drive_scalar(&mut self) -> Result<f32, Self::Error> {
        panic!("mask0 must not sample offboard scalar, including replay");
    }
    fn soft_damping(&mut self) -> Result<f32, Self::Error> {
        assert!(
            !self.refuse_reads,
            "host replay/current-collision guards must precede cvar reads"
        );
        self.damping_reads += 1;
        Ok(0.75) // Explicit current-cvar fixture, not a native default.
    }
}
struct RetainedBoneDrive {
    part: usize,
    kind: SkeletonDriveType,
    frames: Arc<RwLock<SimulationDriveFrames>>,
    dynamics: Arc<RwLock<SimulationDriveDynamics>>,
    previous_frames: [u32; 16],
    previous_dynamics: [u32; 8],
}
fn frames_words(value: SimulationDriveFrames) -> [u32; 16] {
    let vectors = [
        value.quaternion_a,
        value.position_a,
        value.quaternion_b,
        value.position_b,
    ];
    std::array::from_fn(|index| vectors[index / 4][index % 4].to_bits())
}
fn params_words(value: SimulationDriveDynamicsParams) -> [u32; 4] {
    [
        value.spring.to_bits(),
        value.damping.to_bits(),
        value.strength.to_bits(),
        value.raw_type,
    ]
}
fn dynamics_words(value: SimulationDriveDynamics) -> [u32; 8] {
    let linear = params_words(value.linear);
    let angular = params_words(value.angular);
    std::array::from_fn(|index| {
        if index < 4 {
            linear[index]
        } else {
            angular[index - 4]
        }
    })
}
fn pool_payload_words(world: &PhysicalAssemblyWorld) -> Vec<([u32; 16], [u32; 8])> {
    world
        .backend()
        .active_drive_payloads_for_test()
        .unwrap()
        .into_iter()
        .map(|(frames, dynamics)| (frames_words(frames), dynamics_words(dynamics)))
        .collect()
}

/// Real next Build/solve, not a separately recomposed numeric batch. The current
/// external original pose receives an explicit small bone offset BEFORE source
/// mapping. Neither live physics history nor constructor `physics_pose` is used
/// as the current target. IK, adjustments and hook movement remain excluded.
#[expect(
    clippy::too_many_lines,
    reason = "one actual response/current-target/retained-drive/next-solve mounted proof"
)]
fn check_feedback_reaches_next_solve(
    world: &mut PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    ground: &StaticGroundPool,
) {
    // Original packed-name map independently frozen by skeleton_data_corpus.
    const MAPPED_BONES: [usize; 24] = [
        25, 7, 6, 16, 15, 14, 13, 12, 11, 10, 9, 5, 4, 3, 2, 24, 23, 22, 21, 20, 19, 18, 17, 1,
    ];
    let owner = world.skeleton(skeleton).unwrap();
    assert_eq!(owner.state().collision.drive_strength.to_bits(), 0);
    let data = Arc::clone(&owner.state().binding.body().data);
    let original = world
        .physical_body_member(skeleton)
        .unwrap()
        .owner()
        .inputs();
    let mut current_pose = original.pose_matrices.to_vec();
    assert_eq!(*data.bone_ids(), MAPPED_BONES);
    let changed_bone = MAPPED_BONES[1];
    current_pose[changed_bone][12] += 0.015_625;
    current_pose[changed_bone][13] += 0.031_25;
    assert_ne!(
        current_pose[changed_bone].map(f32::to_bits),
        original.pose_matrices[changed_bone].map(f32::to_bits)
    );
    assert_eq!(
        current_pose[changed_bone][15].to_bits(),
        original.pose_matrices[changed_bone][15].to_bits()
    );
    let animation = original
        .vault
        .instance(0xbabb_3ec1_1084_9e08, 0xd7ed_bd36_2d7d_2152);
    let scales = [0x0a58_9569_f5fa_0e6c, 0x13d1_a5a3_e27d_830e]
        .map(|key| f32::from_bits(animation.try_scalar_word(key, 0).unwrap().unwrap()));
    let soft = original
        .vault
        .instance(0xfdf6_1860_0d39_6c3b, 0xd7ed_bd36_2d7d_2152);
    let displacement = f32::from_bits(
        soft.try_scalar_word(0xe1a7_f2dc_fab0_b8d0, 0)
            .unwrap()
            .unwrap(),
    );
    let strength = f32::from_bits(
        soft.try_scalar_word(0xf300_ffd5_99e1_709e, 0)
            .unwrap()
            .unwrap(),
    );
    let runtime = owner.drive_owner().unwrap();
    let retained: Vec<_> = runtime
        .prepared()
        .eligible_bone_drives()
        .map(|emission| RetainedBoneDrive {
            part: emission.part.index(),
            kind: emission.kind,
            frames: Arc::clone(emission.frames),
            dynamics: Arc::clone(emission.dynamics),
            previous_frames: frames_words(*emission.frames.read().unwrap()),
            previous_dynamics: dynamics_words(*emission.dynamics.read().unwrap()),
        })
        .collect();
    assert_eq!(retained.len(), 44);
    let hook0_angular = params_words(runtime.prepared().hook_dynamics(0).read().unwrap().angular);
    let hook1 = dynamics_words(*runtime.prepared().hook_dynamics(1).read().unwrap());
    let before_slots = world.backend().drive_snapshot();
    let before_bodies = world.backend().rigid_body_snapshot();
    let previous_build = world.drive_words_for_test().to_vec();
    assert_eq!(
        previous_build.len(),
        49,
        "actual first ground Build capture"
    );
    let mut cvars = UpdateCvars::default();
    world
        .try_update_skeleton_drives_from_controlled_animation_pose(
            skeleton,
            &current_pose,
            0,
            &mut cvars,
        )
        .unwrap();
    assert_eq!(cvars.damping_reads, 44);
    assert_eq!(world.backend().drive_snapshot(), before_slots);
    assert_eq!(
        world.backend().rigid_body_snapshot(),
        before_bodies,
        "targets must not teleport actual bodies"
    );
    let owner = world.skeleton(skeleton).unwrap();
    let mapped = owner.animation_targets();
    let volumes = mapped.completed_volumes().unwrap();
    for part in 0..24 {
        let matrix = current_pose[MAPPED_BONES[part]];
        assert_eq!(
            mapped.animation_rows()[part].unwrap().map(f32::to_bits),
            matrix.map(f32::to_bits)
        );
        // Independent full-lane source compose: axes multiply then Y/Z FMA;
        // position starts X FMA into outer position. No canonical-W repair.
        let inner = data.bone_to_volume()[part];
        let expected: [f32; 16] = std::array::from_fn(|index| {
            let column = index / 4;
            let lane = index % 4;
            let x = if column == 3 {
                inner[column * 4].mul_add(matrix[lane], matrix[12 + lane])
            } else {
                inner[column * 4] * matrix[lane]
            };
            let y = inner[column * 4 + 1].mul_add(matrix[4 + lane], x);
            inner[column * 4 + 2].mul_add(matrix[8 + lane], y)
        });
        assert_eq!(volumes[part].map(f32::to_bits), expected.map(f32::to_bits));
    }
    assert_ne!(
        mapped.animation_rows()[1].unwrap().map(f32::to_bits),
        data.physics_pose()[1].map(f32::to_bits)
    );
    let runtime = owner.drive_owner().unwrap();
    assert_eq!(runtime.update_progress(), crate::dynamic_assembly_session::skeleton_normal::drive_owner::update_owner::SkeletonDriveUpdateProgress::Complete);
    let mut changed_frames = 0;
    let mut changed_dynamics = 0;
    for retained in &retained {
        let part = crate::ragdoll::SkeletonPartIndex::from_index(retained.part).unwrap();
        let bone = runtime.prepared().bone(part);
        assert!(Arc::ptr_eq(&retained.frames, bone.frames(retained.kind)));
        assert!(Arc::ptr_eq(
            &retained.dynamics,
            bone.dynamics(retained.kind)
        ));
        assert_eq!(bone.flags(), 3);
        let coefficients = owner.state().binding.body().attributes.drives[retained.part];
        // Actual collision strength0 collapses the source FMA interpolation
        // to the original body coefficient, then current global multiplier.
        let argument = match retained.kind {
            SkeletonDriveType::Local => coefficients.collision_strength * scales[0],
            SkeletonDriveType::LocalRoot => coefficients.collision_strength_root * scales[1],
        };
        assert_eq!(
            bone.strengths()[retained.kind.index()].to_bits(),
            argument.to_bits()
        );
        let expected_angular = [
            (displacement * argument).to_bits(),
            0.75_f32.to_bits(),
            ((strength * argument) * f32::from_bits(0x4560_fffe)).to_bits(),
            1,
        ];
        let actual_dynamics = *retained.dynamics.read().unwrap();
        assert_eq!(params_words(actual_dynamics.linear), [0, 0, 0, 1]);
        assert_eq!(params_words(actual_dynamics.angular), expected_angular);
        changed_frames +=
            usize::from(frames_words(*retained.frames.read().unwrap()) != retained.previous_frames);
        changed_dynamics +=
            usize::from(dynamics_words(actual_dynamics) != retained.previous_dynamics);
    }
    assert!(
        changed_frames > 0,
        "current pose must affect actual retained frames"
    );
    assert_eq!(changed_dynamics, 44);
    assert_eq!(
        params_words(runtime.prepared().hook_dynamics(0).read().unwrap().angular),
        hook0_angular
    );
    assert_eq!(
        dynamics_words(*runtime.prepared().hook_dynamics(1).read().unwrap()),
        hook1
    );
    let updated_payloads = pool_payload_words(world);
    assert_eq!(updated_payloads.len(), 49);
    // Actual source insertion: board3, Skeleton hooks2, then bones1..22
    // Local/Root. Dense ordinals here are not native addresses or issued IDs.
    for (index, retained) in retained.iter().enumerate() {
        assert_eq!(
            updated_payloads[index + 5],
            (
                frames_words(*retained.frames.read().unwrap()),
                dynamics_words(*retained.dynamics.read().unwrap())
            )
        );
    }
    cvars.damping_reads = 0;
    cvars.refuse_reads = true;
    assert!(
        world
            .try_update_skeleton_drives_from_controlled_animation_pose(
                skeleton,
                &current_pose,
                0,
                &mut cvars
            )
            .is_err()
    );
    assert_eq!(cvars.damping_reads, 0);
    assert_eq!(pool_payload_words(world), updated_payloads);
    let (counts, report) = world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            TIMESTEP,
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    assert_eq!((counts.joints, counts.drives), (28, 49));
    assert_eq!(report.processed_assemblies, 2);
    let built = world.drive_words_for_test();
    assert_eq!(built.len(), 49);
    for (index, retained) in retained.iter().enumerate() {
        let ordinal = index + 5;
        // Row2.W/Row3.W are actual Build dynamics coefficients, unchanged by
        // Horus. A hard1->soft0 change proves actual payload consumption, not
        // merely differing contact impulses or integrated body snapshots.
        assert_eq!(previous_build[ordinal][2][3], 1.0_f32.to_bits());
        assert_eq!(built[ordinal][2][3], 0);
        let angular = retained.dynamics.read().unwrap().angular;
        let spring_h2 = (TIMESTEP * angular.spring) * TIMESTEP;
        let damping_h = TIMESTEP * angular.damping;
        let denominator = (1.0 + spring_h2) + damping_h;
        let reciprocal = crate::simulation_drive_jacobian::refine_drive_reciprocal(
            denominator,
            denominator.recip(),
        );
        let expected = reciprocal * (spring_h2 + damping_h);
        assert_eq!(built[ordinal][3][3], expected.to_bits());
        assert_eq!(
            built[ordinal][12][3], previous_build[ordinal][12][3],
            "same actual Drive binding"
        );
    }
    assert_eq!(world.backend().drive_snapshot(), before_slots);
    // A new solve alone is insufficient: no current collision response yet.
    let before_refusal = pool_payload_words(world);
    assert!(
        world
            .try_update_skeleton_drives_from_controlled_animation_pose(
                skeleton,
                &current_pose,
                0,
                &mut cvars
            )
            .is_err()
    );
    assert_eq!(cvars.damping_reads, 0);
    assert_eq!(pool_payload_words(world), before_refusal);
    println!(
        "actual44 bone drives: current original mapped pose + positive collision -> source soft payloads -> next49 real Build/Horus records, exact linear/angular dynamics words; replay and missing-current-collision refused"
    );
}

#[derive(Clone, Copy)]
struct ExpectedGroup {
    force: f32,
    friction: f32,
    slide_speed: f32,
    normal: [f32; 3],
    part: Option<usize>,
    surface: u32,
    ordinal: Option<usize>,
}

/// Source-derived oracle for this concrete on-board, danger-zone, world-terrain
/// case (0x82b4b8f0..c6fc). The original group table is independently retained
/// below from 0x82262140. This does NOT call the response function or use any
/// computed response force as input. Separately audited math helpers express
/// the host dot/estimate boundary, not a native Xenon numeric trace.
#[expect(
    clippy::too_many_lines,
    reason = "ordered original contact oracle and every grouped output, with actual pool/history inputs"
)]
fn check_group_winners(
    world: &PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    input: &SkeletonCollisionUpdateInput,
    before: &SkeletonCollisionData,
) {
    const GROUP: [Option<usize>; 24] = [
        None,
        Some(0),
        Some(1),
        Some(2),
        Some(2),
        Some(2),
        Some(1),
        Some(3),
        Some(3),
        Some(3),
        Some(1),
        Some(1),
        Some(1),
        Some(1),
        Some(1),
        Some(6),
        Some(6),
        Some(4),
        Some(4),
        Some(7),
        Some(7),
        Some(5),
        Some(5),
        Some(1),
    ];
    let owner = world.skeleton(skeleton).unwrap();
    let history = owner.physics_history();
    let actual = &owner.state().collision;
    let pool = world.backend().collision_body_pool();
    // The caller has just validated successful current-frame response. This
    // read-only borrow is not treating a stale diagnostic prefix as authority.
    let contacts = world.physical_contact_list_prefix(skeleton).unwrap();
    assert!(contacts.len() <= 16, "actual Island per-Body list cap");
    let frame = owner.collision_frame_state().frame().unwrap();
    assert!(
        contacts
            .iter()
            .all(|contact| Arc::ptr_eq(contact.spy_frame(), frame))
    );
    let vault = &world
        .physical_body_member(skeleton)
        .unwrap()
        .owner()
        .inputs()
        .vault;
    let small_threshold = f32::from_bits(
        vault
            .instance(0x9f86_2c2e_aa01_6ce5, 0xd7ed_bd36_2d7d_2152)
            .try_scalar_word(0x9ab5_7b38_d5bb_9449, 0)
            .unwrap()
            .unwrap(),
    );
    let mut numeric = HostSkeletonCollisionNumeric;
    let direction =
        numeric_graph::try_normalize_safe_xyz(input.system_velocity, [0.0; 3], &mut numeric)
            .unwrap();
    let speed = numeric_graph::try_length_xyz(input.system_velocity, &mut numeric).unwrap();
    let mut groups: [ExpectedGroup; 8] = std::array::from_fn(|index| ExpectedGroup {
        force: 0.0,
        friction: 0.0,
        slide_speed: 0.0,
        normal: [0.0; 3],
        part: None,
        surface: before.audio_surface_type[index],
        ordinal: None,
    });
    let mut priority = 0.0_f32;
    let mut normals: Vec<(usize, [f32; 3])> = Vec::new();
    let mut accepted = [false; 24];
    let mut candidate_forces = Vec::new();
    for (ordinal, contact) in contacts.iter().enumerate() {
        assert!(!contact.self_colliding() && contact.other_body().is_none());
        // Source tests the count BEFORE greater-priority reset. Do not sort or
        // deduplicate; duplicate contacts and equal force retain their order.
        if normals.len() >= 20 {
            continue;
        }
        let part = usize::try_from(contact.part()).unwrap();
        accepted[part] = true;
        match before.priority[part].partial_cmp(&priority) {
            Some(std::cmp::Ordering::Greater) => {
                priority = before.priority[part];
                normals.clear();
                normals.push((part, *contact.normal()));
            }
            Some(std::cmp::Ordering::Equal) => normals.push((part, *contact.normal())),
            _ => {}
        }
        let spy = contact.spy();
        let mass = |body| {
            let inverse = if pool.try_raw_state(body).unwrap() == 1 {
                f32::from_bits(0x3a83_126f)
            } else {
                pool.try_contact_response_cached_inverse_mass(body).unwrap()
            };
            1.0 / inverse
        };
        let mass_a = mass(spy.body_a());
        let mass_b = mass(spy.body_b());
        let (opposite, other_mass) = if contact.is_body_a() {
            (spy.body_b(), mass_b)
        } else {
            (spy.body_a(), mass_a)
        };
        assert_eq!(pool.try_raw_state(opposite).unwrap(), 1);
        assert!(
            !(other_mass > f32::from_bits(0x38d1_b717) && other_mass < small_threshold),
            "this source oracle excludes the dynamic small-object cvar branch"
        );
        let friction_scale = ((mass_b + mass_a) / (mass_b * mass_a)) * f32::from_bits(0x3991_a2b5);
        let friction_velocity = spy.friction_force().map(|value| value * friction_scale);
        let friction_speed =
            numeric_graph::try_length_xyz(friction_velocity, &mut numeric).unwrap();
        let velocity = history.record().velocities()[part];
        let along = numeric
            .dot3([velocity[0], velocity[1], velocity[2]], direction)
            .unwrap();
        let modeled =
            numeric_graph::try_clamp_xyz(direction.map(|value| value * along), speed, &mut numeric)
                .unwrap();
        let terrain_velocity = pool.try_contact_spy_linear_velocity(opposite).unwrap();
        let relative = std::array::from_fn(|lane| terrain_velocity[lane] - modeled[lane]);
        let force = f32::from_bits(
            numeric.dot3(relative, *contact.normal()).unwrap().to_bits() & 0x7fff_ffff,
        );
        let Some(group) = GROUP[part] else {
            continue;
        };
        candidate_forces.push((group, ordinal, force));
        if force > groups[group].force {
            let slide = numeric_graph::cross_xyz(relative, *contact.normal());
            groups[group] = ExpectedGroup {
                force,
                friction: friction_speed * history.part_masses()[part],
                slide_speed: numeric_graph::try_length_xyz(slide, &mut numeric).unwrap(),
                normal: *contact.normal(),
                part: Some(part),
                surface: contact.surface() & 0x7f,
                ordinal: Some(ordinal),
            };
        }
    }
    assert!(groups.iter().any(|winner| winner.force > 0.0));
    assert_eq!(actual.collision_priority.to_bits(), priority.to_bits());
    assert_eq!(usize::try_from(actual.num_normals).unwrap(), normals.len());
    for (normal, (part, xyz)) in actual.normal_info.iter().zip(normals) {
        assert_eq!(normal.part.index(), part);
        assert_eq!(normal.normal.xyz().map(f32::to_bits), xyz.map(f32::to_bits));
        assert!(normal.normal.known_lanes().is_none());
    }
    for (part, accepted) in accepted.into_iter().enumerate() {
        assert_eq!(actual.just_collided[part], u8::from(accepted));
        let expected_time = if accepted {
            0.0
        } else {
            before.time_since_collision[part] - input.timestep
        };
        assert_eq!(
            actual.time_since_collision[part].to_bits(),
            expected_time.to_bits()
        );
    }
    for (group, expected) in groups.into_iter().enumerate() {
        assert_eq!(
            actual.contact_force[group].to_bits(),
            expected.force.to_bits(),
            "group{group} force"
        );
        assert_eq!(
            actual.contact_friction_force[group].to_bits(),
            expected.friction.to_bits(),
            "group{group} friction"
        );
        assert_eq!(
            actual.slide_speed[group].to_bits(),
            expected.slide_speed.to_bits(),
            "group{group} slide"
        );
        assert_eq!(
            actual.contact_bone[group].map(crate::ragdoll::SkeletonPartIndex::index),
            expected.part
        );
        assert_eq!(actual.audio_surface_type[group], expected.surface);
        assert_eq!(
            actual.contact_normal[group].xyz().map(f32::to_bits),
            expected.normal.map(f32::to_bits)
        );
        if let Some(ordinal) = expected.ordinal {
            assert!(actual.contact_normal[group].known_lanes().is_none());
            // All earlier equal maxima must lose to the earliest selected
            // occurrence. This checks exact list/tie order without fabricating
            // a duplicate production ContactInfo to force artificial coverage.
            let earliest = candidate_forces
                .iter()
                .find(|(candidate_group, _, force)| {
                    *candidate_group == group && force.to_bits() == expected.force.to_bits()
                })
                .unwrap();
            assert_eq!(ordinal, earliest.1);
        } else {
            assert_eq!(
                actual.contact_normal[group]
                    .known_lanes()
                    .unwrap()
                    .map(f32::to_bits),
                [0; 4]
            );
        }
    }
}

#[path = "../skeleton/physical_player_skeleton_postphysics_live_corpus.rs"]
pub(crate) mod postphysics_live;

pub(crate) fn check(original: &Arc<PhysicalPlayerAssemblyOwner>) {
    check_mode(original, false);
    check_mode(original, true);
}
