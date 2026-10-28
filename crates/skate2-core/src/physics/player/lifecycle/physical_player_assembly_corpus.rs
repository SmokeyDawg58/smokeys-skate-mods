//! The real paired factory consumes verified mounted inputs, not a prebuilt
//! SkeletonData/body. Original RAW rig pose, spawn and collaborator services
//! remain controlled fixtures; this is not a live Actor or whole `HiLOD` claim.
use super::*;
#[path = "../ground/physical_player_ground_startup_corpus.rs"]
mod ground_startup;
#[path = "../runtime/physical_player_queued_input_corpus.rs"]
mod queued_input;
use crate::{
    attribute_runtime_corpus::mounted_physics_attributes_with_classes,
    dynamic_assembly_session::{
        skater_pool::physical::SimulationPhysicalBodyKind,
        skeleton_normal::history_owner::SkeletonHistoryInitialization,
    },
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
    skateboard_attributes::SkateboardAttributeClass,
    skeleton_body_attributes,
    skeleton_data_corpus::with_mounted_skeleton_inputs,
};

struct Services {
    flags: u32,
    next_flags: Option<u32>,
    events: Vec<&'static str>,
}
impl Services {
    fn simulation() -> Self {
        Self {
            flags: 1,
            next_flags: None,
            events: Vec::new(),
        }
    }
}
impl SkateboardSpawnServices for Services {
    fn current_orthogonal_tolerance(&mut self) -> f32 {
        self.events.push("tolerance");
        0.01
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("controlled fresh spawn must pass the actual guard");
    }
}
impl PhysicalPlayerAssemblyServices for Services {
    fn current_entity_flags(&mut self) -> u32 {
        self.events.push("flags");
        let flags = self.flags;
        if let Some(next) = self.next_flags.take() {
            self.flags = next;
        }
        flags
    }
}

#[derive(Default)]
struct DriveCvars {
    events: Vec<&'static str>,
}
impl SkeletonDriveCvars for DriveCvars {
    type Error = Infallible;
    fn hard_drive_displacement(&mut self) -> Result<f32, Self::Error> {
        self.events.push("displacement");
        Ok(1.0)
    }
    fn hard_drive_strength(&mut self) -> Result<f32, Self::Error> {
        self.events.push("strength");
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

fn world() -> PhysicalAssemblyWorld {
    // StaticWorld1 + board7 + board hook1 + skeleton24 + skeleton hook2.
    let mut world = PhysicalAssemblyWorld::try_new(35, 28, 49, 0).unwrap();
    world.try_initialize_grab_query_manager(true).unwrap();
    world.try_initialize_skater_pool(3, false).unwrap();
    world
}

fn copy_inputs(inputs: &PhysicalPlayerAssemblyInputs) -> PhysicalPlayerAssemblyInputs {
    PhysicalPlayerAssemblyInputs {
        cvars: inputs.cvars.clone(),
        vault: Arc::clone(&inputs.vault),
        hierarchy: Arc::clone(&inputs.hierarchy),
        parameters: Arc::clone(&inputs.parameters),
        pose_hierarchy: Arc::clone(&inputs.pose_hierarchy),
        pose_matrices: Arc::clone(&inputs.pose_matrices),
        spawn: inputs.spawn,
        hat_type: inputs.hat_type,
        bouncy_ragdoll: inputs.bouncy_ragdoll,
        is_human: inputs.is_human,
    }
}

/// Shared original mounted inputs; each consumer retains a fresh actual owner chain.
#[expect(
    clippy::too_many_lines,
    reason = "Retain the original mounted factory ownership and construction sequence"
)]
fn with_original_inputs(operation: impl FnOnce(PhysicalPlayerAssemblyInputs)) {
    let vault = mounted_physics_attributes_with_classes(
        &crate::selected_original_asset_classes::selected_additional_classes(),
    );
    let hat = vault
        .instance(0x91bc_a669_3efc_9aa7, 0x2275_b900_5016_6969)
        .try_scalar_word(0x3441_8cad_1c23_bc94, 0)
        .unwrap()
        .unwrap();
    assert_eq!(hat, 0, "actual tutorial recipe, not a skipped unknown hat");
    let face_name = vault
        .instance(0x91bc_a669_3efc_9aa7, 0x2275_b900_5016_6969)
        .try_cached_layout_text(0)
        .unwrap()
        .expect("selected original marquee has a nonnull face name");
    assert_eq!(face_name.bytes(), b"tutorial_male");
    with_mounted_skeleton_inputs(|hierarchy, parameters, pose_hierarchy, pose_matrices| {
        assert!(!Arc::ptr_eq(&hierarchy, &pose_hierarchy));
        let (_cvar_manager, cvars) = crate::game_input_recognizer_corpus::original_input_cvars();
        let inputs = PhysicalPlayerAssemblyInputs {
            cvars,
            vault,
            hierarchy,
            parameters,
            pose_hierarchy,
            pose_matrices,
            spawn: FRESH_VOLUME_TRANSFORM,
            hat_type: hat,
            // Explicit controlled human/non-bouncy caller selections, not an
            // inferred live Character or reconstructed Actor input record.
            bouncy_ragdoll: 0,
            is_human: 1,
        };
        operation(inputs);
    });
}

#[path = "../wipeout/physical_player_wipeout_live_corpus.rs"]
mod wipeout_live;

pub(crate) fn mounted_paired_physical_construction() {
    with_original_inputs(|inputs| {
        let shared_world = Arc::new(std::sync::Mutex::new(world()));
        shared_world
            .lock()
            .unwrap()
            .try_initialize_debug_overlay()
            .unwrap();
        ground_startup::check(copy_inputs(&inputs));
        let (actor, _physical_animation, _input_session) =
            crate::game_input_recognizer_corpus::check(
                &inputs.vault,
                &shared_world,
                inputs.spawn,
                inputs.cvars.clone(),
            );
        let mut selected_world = shared_world.lock().unwrap();
        let mut factory = PhysicalPlayerAssemblyConstruction::new(&mut selected_world, inputs);
        let weak_owner = Arc::downgrade(factory.owner());
        assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Fresh);
        assert!(factory.state.skeleton_construction.is_none());
        assert!(factory.completed_world_mut().is_err());
        let mut services = Services::simulation();
        factory.try_construct_board(&mut services).unwrap();
        assert_board_prefix(&factory);
        factory
            .try_construct_skeleton(&mut services, &mut || {
                panic!("actual hat0 does not sample hat cvar")
            })
            .unwrap();
        assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::SkeletonReady);
        // Skeleton constructor owns a separate GetPhysicsIsland call before
        // Body's later routing query.
        assert_eq!(services.events, ["flags", "tolerance", "flags", "flags"]);
        let mut cvars = DriveCvars::default();
        let mut history = HistoryInput::default();
        factory
            .try_finish_skeleton(&mut cvars, &mut history)
            .unwrap();
        assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Complete);
        assert_eq!(
            cvars.events,
            ["displacement", "strength", "displacement", "strength"]
        );
        assert_eq!(
            history.events,
            ["acquire", "mode", "up", "acquire", "mode", "up"]
        );
        assert_complete_pair(&factory);
        super::jump_corpus::check(&mut factory);
        check_ongoing_pair_history(&mut factory);
        super::contact_list_corpus::check(factory.owner());
        super::collision_corpus::check(factory.owner());
        assert_changed_selector(factory.owner());
        assert_visual_precedes_malformed_board(factory.owner());
        assert_uninitialized_pool_retains_actual_definition(factory.owner());
        check_board_entry_effects(&mut factory);
        cleanup_complete_pair(&mut factory);
        assert!(weak_owner.upgrade().is_some());
        drop(factory);
        assert!(weak_owner.upgrade().is_none());
        assert_eq!(selected_world.physical_body_registry().next_body_id(), 3);
        drop(selected_world);
        drop(actor);
    });
    println!(
        "original mounted inputs -> source-ordered real board then skeleton construction -> SAME selected manager/owner, 31 registered Parts, two hook assemblies/three hook Parts, 28 joints/49 drives, full Set/two history records, guarded ordered cleanup; controlled pose/spawn/cvars, not full Actor/HiLOD/flags3 Tick"
    );
}

fn check_board_entry_effects(factory: &mut PhysicalPlayerAssemblyConstruction<'_>) {
    let board = factory.board().unwrap().clone();
    let actual_hook = Arc::clone(factory.state.hook_construction.as_ref().unwrap().dynamics());
    let (animated, speed, retained) = factory.world().board_entry_snapshot(&board);
    assert!(!animated);
    assert_eq!(speed.to_bits(), 0);
    assert!(Arc::ptr_eq(&actual_hook, &retained));
    // Controlled pre-entry dynamics distinguish the live write from the hook
    // constructor's already-HARD/zero defaults; not a claimed animation tick.
    let seeded = crate::simulation_drive_pool::SimulationDriveDynamicsParams {
        spring: 3.0,
        damping: 4.0,
        strength: 5.0,
        raw_type: 1,
    };
    *actual_hook.write().unwrap() = crate::simulation_drive_pool::SimulationDriveDynamics {
        linear: seeded,
        angular: seeded,
    };
    let owner = Arc::clone(factory.owner());
    let world = factory.completed_world_mut().unwrap();
    world
        .try_set_board_velocity(
            &board,
            &crate::skeleton_root_transforms::SkeletonRootVector {
                xyz: [2.0, -3.0, 4.0],
                w: None,
            },
        )
        .unwrap();
    let transform = world
        .board(&board)
        .unwrap()
        .try_live_part_transform_host(world.backend(), 6)
        .unwrap();
    // Independent f64 projection of a controlled velocity onto the actual
    // mounted deck axis; source Reset established the same owner's flip bit.
    let expected_speed = (2.0 * f64::from(transform[8]) - 3.0 * f64::from(transform[9])
        + 4.0 * f64::from(transform[10]))
    .abs();
    world
        .try_reset_board_speed_model(&board, &mut crate::reckoning_numeric::HostReckoningNumeric)
        .unwrap();
    let (_, target_speed, _) = world.board_entry_snapshot(&board);
    assert!((f64::from(target_speed) - expected_speed).abs() < 0.000_01);
    assert!(target_speed > 0.0);
    let before_bodies = world.backend().rigid_body_snapshot();
    let before_drives = world.backend().drive_snapshot();
    let wobble_attributes = world.with_board_speed_wobble_for_test(&board, |owner| {
        let wobble = owner.value_mut_for_test().unwrap();
        assert_eq!(wobble.state_bits(), (false, [0; 6], None));
        let attributes = Arc::clone(wobble.attributes());
        let layout = attributes.try_capture_layout().unwrap();
        assert!(!layout.uses_default());
        assert_eq!(layout.bytes().len(), 0x100);
        wobble.seed_reset_fields_for_test(3.0, Some(7.0));
        attributes
    });
    world.try_reset_board_speed_wobble(&board).unwrap();
    world.with_board_speed_wobble_for_test(&board, |owner| {
        let wobble = owner.value_mut_for_test().unwrap();
        assert_eq!(
            wobble.state_bits(),
            (false, [0; 6], Some(7.0_f32.to_bits()))
        );
        assert!(Arc::ptr_eq(wobble.attributes(), &wobble_attributes));
    });
    world.try_disable_board_animation(&board).unwrap();
    assert_eq!(world.backend().rigid_body_snapshot(), before_bodies);
    assert_eq!(world.backend().drive_snapshot(), before_drives);
    for params in [
        actual_hook.read().unwrap().linear,
        actual_hook.read().unwrap().angular,
    ] {
        assert_eq!(params.raw_type, 2);
        assert_eq!(
            [params.spring, params.damping, params.strength].map(f32::to_bits),
            [0; 3]
        );
    }
    owner
        .reckoning()
        .lock()
        .unwrap()
        .try_reset_body_spin()
        .unwrap();

    check_capacity_null_board_hook(&owner, seeded);
}

fn check_capacity_null_board_hook(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    seeded: crate::simulation_drive_pool::SimulationDriveDynamicsParams,
) {
    // Two real truck drives fill this separate manager; AddDrive for the hook
    // returns its source-supported null result. The same Body payload survives.
    let mut limited = PhysicalAssemblyWorld::try_new(35, 28, 2, 0).unwrap();
    limited.try_initialize_skater_pool(3, false).unwrap();
    let mut limited_factory =
        PhysicalPlayerAssemblyConstruction::new(&mut limited, copy_inputs(owner.inputs()));
    limited_factory
        .try_construct_board(&mut Services::simulation())
        .unwrap();
    let limited_board = limited_factory.board().unwrap().clone();
    assert_eq!(
        limited_factory
            .world()
            .backend()
            .drive_snapshot()
            .counts
            .free,
        0
    );
    assert_eq!(
        limited_factory
            .world()
            .backend()
            .drive_snapshot()
            .active_order
            .len(),
        2
    );
    let limited_hook = Arc::clone(
        limited_factory
            .state
            .hook_construction
            .as_ref()
            .unwrap()
            .dynamics(),
    );
    let (_, _, retained) = limited_factory.world().board_entry_snapshot(&limited_board);
    assert!(Arc::ptr_eq(&limited_hook, &retained));
    *limited_hook.write().unwrap() = crate::simulation_drive_pool::SimulationDriveDynamics {
        linear: seeded,
        angular: seeded,
    };
    let before = limited_factory.world().backend().rigid_body_snapshot();
    let before_drives = limited_factory.world().backend().drive_snapshot();
    limited_factory
        .world
        .try_disable_board_animation(&limited_board)
        .unwrap();
    assert_eq!(
        limited_factory.world().backend().rigid_body_snapshot(),
        before
    );
    assert_eq!(
        limited_factory.world().backend().drive_snapshot(),
        before_drives
    );
    for params in [
        limited_hook.read().unwrap().linear,
        limited_hook.read().unwrap().angular,
    ] {
        assert_eq!(params.raw_type, 2);
        assert_eq!(
            [params.spring, params.damping, params.strength].map(f32::to_bits),
            [0; 3]
        );
    }
    limited_factory.try_teardown().unwrap();
    eprintln!(
        "original board entry: SAME hook retained with admitted/capacity-null drive; DisableBoardAnimation exact parameter stores; body/pool membership unchanged; selected ResetBodySpin PASS (not full PhysicsGround.Enter)"
    );
}

fn check_ongoing_pair_history(factory: &mut PhysicalPlayerAssemblyConstruction<'_>) {
    use crate::{
        dynamic_assembly_session::skeleton_normal::history_owner::tests::{
            check_mounted_ongoing_history, check_mounted_ongoing_history_late_failure,
        },
        simulation_assembly_backend::constraint_session::{
            contact_feedback::ControlledNoFreezing, physical_history::PhysicalPlayerHistoryError,
            skateboard_ground::SkateboardGroundFrameInput,
        },
        simulation_contact_jacobian::ground_preparation::corpus::mounted_robs_park_ground_for_board_test,
    };
    let layout = factory
        .owner()
        .inputs()
        .vault
        .physics(crate::skateboard_attributes::SkateboardAttributes::DEFAULT_COLLECTION)
        .try_physics_layout()
        .unwrap();
    let gravity: [f32; 3] = std::array::from_fn(|axis| {
        f32::from_be_bytes(layout.raw_bytes[axis * 4..axis * 4 + 4].try_into().unwrap())
    });
    let skeleton = factory.skeleton().unwrap().clone();
    let board = factory.board().unwrap().clone();
    let mut roots = super::roots_corpus::RootCorpus::new(factory.owner());
    let current_pose = Arc::clone(&factory.owner().inputs().pose_matrices);
    let world = factory.completed_world_mut().unwrap();
    let mut input = HistoryInput::default();
    assert!(matches!(
        world.try_update_skeleton_physics_history(&skeleton, &board, &mut input),
        Err(PhysicalPlayerHistoryError::MissingFrame)
    ));
    assert!(
        input.events.is_empty(),
        "no current input is read without a real solved frame"
    );
    world.backend_mut().set_gravity(gravity);
    world.backend_mut().set_cooldown(2);
    world
        .backend_mut()
        .set_minimum_energy(f32::from_bits(0x3727_c5ac));
    let ground = mounted_robs_park_ground_for_board_test();
    // Original terrain stays mounted; the controlled identity spawn is outside
    // its bounds. This is actual paired constraint integration, not a positive
    // skater/terrain response or a whole gameplay Tick.
    for frame in 0..5 {
        let (counts, _) = world
            .try_step_authored_ground(
                SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                    ground_pool: &ground,
                    ground_group: 0,
                    pipeline_culling: None,
                    allow_multiple_writes: false,
                    spy_flags: 4,
                },
                f32::from_bits(0x3d08_8889),
                50,
                &mut ControlledNoFreezing,
            )
            .unwrap();
        assert_eq!((counts.contacts, counts.joints, counts.drives), (0, 28, 49));
        if frame == 0 {
            crate::simulation_assembly_backend::constraint_session::physical_history::tests::check_mounted_phase_guards(world, &skeleton, &board);
        }
        if frame < 4 {
            check_mounted_ongoing_history(world, &skeleton, &board);
            roots.check(world, &skeleton, &board, &current_pose);
        } else {
            check_mounted_ongoing_history_late_failure(world, &skeleton, &board);
        }
    }
    // Retained successful bookkeeping alone is not a freshness certificate.
    world
        .backend_mut()
        .begin_dynamic_update_frame(f32::from_bits(0x3c88_8889));
    input.events.clear();
    assert!(matches!(
        world.try_update_skeleton_physics_history(&skeleton, &board, &mut input),
        Err(PhysicalPlayerHistoryError::Spy(_))
    ));
    assert!(input.events.is_empty());
}

fn assert_board_prefix(factory: &PhysicalPlayerAssemblyConstruction<'_>) {
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::BoardReady);
    assert!(
        factory.state.skeleton_construction.is_none(),
        "no early SkeletonData or body construction"
    );
    let board = factory.board().unwrap();
    let world = factory.world();
    assert_eq!(world.board_world_index(board).unwrap(), 0);
    assert_eq!(
        world
            .try_hook_snapshot(factory.state.hook.as_ref().unwrap())
            .unwrap()
            .world_index,
        1
    );
    assert_eq!(world.hook_count(), 1);
    assert_eq!(world.physical_body_registry().member_count(), 1);
    assert_eq!(world.physical_body_registry().target_count(), 7);
    assert_eq!(world.physical_body_member(board).unwrap().body_id(), 1);
    assert_eq!(world.backend().rigid_body_snapshot().active_order.len(), 7);
    assert_eq!(world.backend().joint_snapshot().active_order.len(), 6);
    assert_eq!(world.backend().drive_snapshot().active_order.len(), 3);
}

fn observations(
    world: &PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Vec<SimulationRigidBodyObservation> {
    world
        .board_publication(board)
        .unwrap()
        .rigid_bodies()
        .iter()
        .chain(world.skeleton_publication(skeleton).unwrap().rigid_bodies())
        .copied()
        .collect()
}

#[expect(
    clippy::too_many_lines,
    reason = "one actual paired pool and retained-owner consistency check"
)]
fn assert_complete_pair(factory: &PhysicalPlayerAssemblyConstruction<'_>) {
    let history = factory.owner().prepare_velocity().lock().unwrap();
    assert_eq!(history.velocity().unwrap().xyz.map(f32::to_bits), [0; 3]);
    assert_eq!(history.velocity().unwrap().w.map(f32::to_bits), Some(0));
    assert_eq!(history.last_spin().unwrap().to_bits(), 0);
    assert_eq!(history.same_direction_frames(), Some(0));
    assert!(!history.complete());
    drop(history);
    let world = factory.world();
    let board = factory.board().unwrap();
    let skeleton = factory.skeleton().unwrap();
    assert_eq!(world.skater_world_index(skeleton).unwrap(), 2);
    let skeleton_owner = world.skeleton(skeleton).unwrap();
    assert_constructed_ik(skeleton_owner, &factory.owner().inputs().vault);
    assert_eq!(skeleton_owner.jump_adjustment_x().unwrap().to_bits(), 0);
    assert_eq!(skeleton_owner.jump_adjustment_z().unwrap().to_bits(), 0);
    let construction = world.skeleton_construction(skeleton).unwrap().unwrap();
    assert!(Arc::ptr_eq(construction.owner(), factory.owner()));
    assert!(Arc::ptr_eq(
        construction.data().unwrap().hierarchy(),
        &factory.owner().inputs().hierarchy
    ));
    assert!(Arc::ptr_eq(
        construction.data().unwrap(),
        &construction.body().unwrap().data
    ));
    assert_eq!(construction.island_body_id(), Some(2));
    let skeleton_hook = skeleton_owner.drive_owner().unwrap().hook().unwrap();
    let hook = world.try_hook_snapshot(skeleton_hook).unwrap();
    assert_eq!(hook.world_index, 3);
    assert_eq!(hook.parts.len(), 2);
    // Nonserialized HookPool assigns the Assembly index, but does not run
    // serialized Create's Part-world-index propagation: each retains default0.
    assert!(
        hook.parts
            .iter()
            .all(|part| part.world_index == 0 && part.user_data == 1)
    );
    assert_eq!(world.hook_count(), 2);
    let registry = world.physical_body_registry();
    assert_eq!(
        (
            registry.member_count(),
            registry.target_count(),
            registry.next_body_id()
        ),
        (2, 31, 3)
    );
    assert!(registry.static_order().is_empty());
    assert_eq!(registry.dynamic_order().len(), 2);
    for (handle, id, count, kind) in [
        (board, 1, 7, SimulationPhysicalBodyKind::Board),
        (skeleton, 2, 24, SimulationPhysicalBodyKind::Skeleton),
    ] {
        let member = world.physical_body_member(handle).unwrap();
        assert!(member.handle().same_as(handle));
        assert!(Arc::ptr_eq(member.owner(), factory.owner()));
        assert_eq!(
            (
                member.body_id(),
                member.requested_state(),
                member.registered_parts()
            ),
            (id, 4, count)
        );
        assert_eq!(member.kind(), kind);
        assert!(member.list_published() && member.registration_complete());
        assert!(registry.dynamic_order()[(id - 1) as usize].same_as(handle));
    }
    let bodies = world.backend().rigid_body_snapshot();
    assert_eq!(
        (
            bodies.active_order.len(),
            bodies.static_order.len(),
            bodies.free_order.len()
        ),
        (31, 4, 0)
    );
    assert_eq!(
        bodies.slots.iter().filter(|body| body.state == 12).count(),
        31
    );
    assert_eq!(
        bodies.slots.iter().filter(|body| body.state == 1).count(),
        4
    );
    for (ordinal, observation) in observations(world, board, skeleton).into_iter().enumerate() {
        let target = world.physical_body_target(observation).unwrap().unwrap();
        let (handle, part) = if ordinal < 7 {
            (board, ordinal)
        } else {
            (skeleton, ordinal - 7)
        };
        assert!(target.member().same_as(handle));
        assert_eq!(target.part() as usize, part);
        let body = &bodies.slots[observation.identity().id().index() as usize];
        // Island target tag is the original Part ordinal; it is not the
        // separate rigid body's solver tag field.
        assert_eq!(body.state, 12);
    }
    assert_eq!(world.backend().joint_snapshot().active_order.len(), 28);
    assert_eq!(world.backend().drive_snapshot().active_order.len(), 49);
    assert_eq!(
        skeleton_owner.physics_history_initialization(),
        SkeletonHistoryInitialization::Complete
    );
    assert_eq!(
        skeleton_owner.physics_history().time_delta().to_bits(),
        0x3c88_8889
    );
    for velocity in skeleton_owner.physics_history().record().velocities() {
        assert_eq!(
            velocity[..3]
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            [0; 3]
        );
    }
}

fn cleanup_complete_pair(factory: &mut PhysicalPlayerAssemblyConstruction<'_>) {
    let board = factory.board().unwrap().clone();
    let skeleton = factory.skeleton().unwrap().clone();
    let hook = factory.state.hook.as_ref().unwrap().clone();
    let world = factory.completed_world_mut().unwrap();
    let original = observations(world, &board, &skeleton);
    crate::simulation_assembly_backend::constraint_session::physical_island::tests::check_registration_defenses(world, &board);
    // Refused removal must leave the actual state/map/list prefix untouched.
    assert!(world.try_remove_skeleton(&skeleton).is_err());
    assert!(world.try_remove_board(&board).is_err());
    assert_eq!(world.physical_body_registry().target_count(), 31);
    assert!(
        world
            .physical_body_member(&skeleton)
            .unwrap()
            .registration_complete()
    );
    assert!(
        world
            .physical_body_member(&board)
            .unwrap()
            .registration_complete()
    );
    assert_eq!(
        world
            .backend()
            .rigid_body_snapshot()
            .slots
            .iter()
            .filter(|body| body.state == 12)
            .count(),
        31
    );
    // Re-registering uses the same occurrences and original tags, not new IDs.
    world.try_register_physical_body_targets(&board).unwrap();
    world.try_register_physical_body_targets(&skeleton).unwrap();
    assert_eq!(world.physical_body_registry().target_count(), 31);
    world.try_release_skeleton_drives(&skeleton).unwrap();
    assert_eq!(world.hook_count(), 1);
    assert_eq!(world.backend().drive_snapshot().active_order.len(), 3);
    let removed_skeleton = world.try_remove_skeleton(&skeleton).unwrap();
    assert_eq!(
        (
            world.physical_body_registry().member_count(),
            world.physical_body_registry().target_count()
        ),
        (1, 7)
    );
    world.try_remove_board_drives(&board).unwrap();
    crate::simulation_assembly_backend::constraint_session::physical_island::tests::check_cleanup_progress(world, &board);
    drop(world.try_remove_hook(&hook).unwrap());
    let removed_board = world.try_remove_board(&board).unwrap();
    assert_eq!(
        (
            world.physical_body_registry().member_count(),
            world.physical_body_registry().target_count()
        ),
        (0, 0)
    );
    assert!(world.physical_body_registry().dynamic_order().is_empty());
    assert!(
        original
            .into_iter()
            .all(|body| world.physical_body_target(body).is_err())
    );
    assert_eq!(world.backend().rigid_body_snapshot().static_order.len(), 1);
    assert_eq!(world.backend().rigid_body_snapshot().free_order.len(), 34);
    assert!(world.backend().joint_snapshot().active_order.is_empty());
    assert!(world.backend().drive_snapshot().active_order.is_empty());
    assert_candidate_manager_binding(world, removed_board, removed_skeleton);
}

#[expect(
    clippy::too_many_lines,
    reason = "real candidate failure ownership across two managers and explicit cleanup"
)]
fn assert_candidate_manager_binding(
    original: &mut PhysicalAssemblyWorld,
    board: SimulationSkaterAssemblyCandidate,
    skeleton: SimulationSkeletonAssemblyCandidate,
) {
    let mut foreign = world();
    let before_bodies = foreign.backend().rigid_body_snapshot();
    let board_index = board.world_index();
    let skeleton_index = skeleton.world_index();
    let board = match foreign.try_insert_board(board, 4, -1.0) {
        Err(failure) => {
            assert!(matches!(
                failure.error,
                SimulationSkaterPoolError::ForeignManager
            ));
            failure.candidate
        }
        Ok(_) => panic!("a retained physical owner cannot change selected manager"),
    };
    let skeleton = match foreign.try_insert_skeleton(skeleton, -1.0) {
        Err(failure) => {
            assert!(matches!(
                failure.error,
                SimulationSkaterPoolError::ForeignManager
            ));
            failure.candidate
        }
        Ok(_) => panic!("a retained physical owner cannot change selected manager"),
    };
    assert_eq!(foreign.backend().rigid_body_snapshot(), before_bodies);
    assert_eq!(foreign.physical_body_registry().next_body_id(), 1);
    assert_eq!(
        (board.world_index(), skeleton.world_index()),
        (board_index, skeleton_index)
    );
    assert_eq!(board.construction().unwrap().island_body_id(), Some(1));
    assert_eq!(skeleton.construction().unwrap().island_body_id(), Some(2));
    // Actual first insertion in the refused manager still receives0. This is
    // checked through a real producer, not by burning arbitrary manager IDs.
    let mut fresh = PhysicalPlayerAssemblyConstruction::new(
        &mut foreign,
        copy_inputs(
            board
                .construction()
                .unwrap()
                .physical_owner()
                .unwrap()
                .inputs(),
        ),
    );
    fresh
        .try_construct_board(&mut Services::simulation())
        .unwrap();
    assert_eq!(
        fresh
            .world()
            .board_world_index(fresh.board().unwrap())
            .unwrap(),
        0
    );
    fresh
        .try_construct_skeleton(&mut Services::simulation(), &mut || panic!("hat0"))
        .unwrap();
    fresh
        .try_finish_skeleton(&mut DriveCvars::default(), &mut HistoryInput::default())
        .unwrap();
    assert_eq!(fresh.phase(), PhysicalPlayerAssemblyPhase::Complete);
    assert_eq!(fresh.world().physical_body_registry().target_count(), 31);
    fresh.try_teardown().unwrap();
    assert_eq!(fresh.phase(), PhysicalPlayerAssemblyPhase::Removed);
    assert!(
        fresh
            .world()
            .backend()
            .drive_snapshot()
            .active_order
            .is_empty()
    );
    assert_eq!(fresh.world().physical_body_registry().target_count(), 0);
    assert_eq!(
        fresh
            .state
            .board_candidate
            .as_ref()
            .unwrap()
            .construction()
            .unwrap()
            .island_body_id(),
        Some(1)
    );
    assert_eq!(
        fresh
            .state
            .skeleton_candidate
            .as_ref()
            .unwrap()
            .construction()
            .unwrap()
            .island_body_id(),
        Some(2)
    );
    // Explicit host candidate-retention check, not replay of either native
    // constructor: same selected manager can reattach those actual sessions.
    // Earlier live velocity writes invalidate the frame. Begin a new diagnostic
    // frame before publishing reattached candidates; do not reuse stale bounds.
    original
        .backend_mut()
        .begin_dynamic_update_frame(f32::from_bits(0x3c88_8889));
    let board = original.try_insert_board(board, 4, -1.0).unwrap();
    let skeleton = original.try_insert_skeleton(skeleton, -1.0).unwrap();
    assert_eq!(original.physical_body_registry().next_body_id(), 3);
    assert!(matches!(
        original.try_add_physical_body_membership(&board, 1),
        Err(SimulationPhysicalBodyError::RequestedStateMismatch {
            expected: 4,
            actual: 1
        })
    ));
    assert!(matches!(
        original.try_add_physical_body_membership(&skeleton, 1),
        Err(SimulationPhysicalBodyError::RequestedStateMismatch {
            expected: 4,
            actual: 1
        })
    ));
    assert_eq!(original.physical_body_registry().next_body_id(), 3);
    assert_eq!(original.physical_body_registry().member_count(), 0);
    let skeleton = original.try_remove_skeleton(&skeleton).unwrap();
    let board = original.try_remove_board(&board).unwrap();
    assert_eq!(skeleton.construction().unwrap().island_body_id(), Some(2));
    assert_eq!(board.construction().unwrap().island_body_id(), Some(1));
}

fn assert_changed_selector(original: &Arc<PhysicalPlayerAssemblyOwner>) {
    let mut selected = world();
    let mut factory =
        PhysicalPlayerAssemblyConstruction::new(&mut selected, copy_inputs(original.inputs()));
    assert!(
        !Arc::ptr_eq(factory.owner(), original),
        "same assets never alias distinct physical owners"
    );
    let mut services = Services::simulation();
    factory.try_construct_board(&mut services).unwrap();
    // The constructor's separate island query succeeds; the later Body
    // routing query must freshly observe the changed Entity flags.
    services.next_flags = Some(0);
    assert!(matches!(
        factory.try_construct_skeleton(&mut services, &mut || panic!("hat0")),
        Err(PhysicalPlayerAssemblyError::VisualIslandUnsupported { flags: 0 })
    ));
    assert_eq!(
        factory.phase(),
        PhysicalPlayerAssemblyPhase::SkeletonRouting
    );
    assert!(
        factory.state.skeleton_candidate.is_some(),
        "native skeleton selector occurs after actual definition construction"
    );
    assert!(factory.skeleton().is_none());
    assert_constructed_ik(
        factory.state.skeleton_candidate.as_ref().unwrap().owner(),
        &factory.owner().inputs().vault,
    );
    assert_eq!(factory.world().physical_body_registry().target_count(), 7);
    assert_eq!(factory.world().physical_body_registry().next_body_id(), 2);
    assert!(factory.completed_world_mut().is_err());
    assert!(matches!(
        factory.try_construct_skeleton(&mut services, &mut || panic!("no replay")),
        Err(PhysicalPlayerAssemblyError::WrongPhase(
            PhysicalPlayerAssemblyPhase::SkeletonRouting
        ))
    ));
    factory.try_teardown().unwrap();
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Removed);
    assert!(factory.board().is_none());
    assert_eq!(factory.world().physical_body_registry().target_count(), 0);
    assert_eq!(factory.world().physical_body_registry().next_body_id(), 2);
    factory.try_teardown().unwrap();
}

fn assert_constructed_ik(
    skeleton: &crate::dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly,
    vault: &Arc<AttributeRuntimeVault>,
) {
    let runtime = skeleton
        .current_animation()
        .ik_runtime()
        .expect("actual factory constructed IK before routing");
    let geometry = runtime.geometry();
    assert_eq!(
        geometry.progress(),
        crate::skeleton_ik_geometry::SkeletonIkGeometryProgress::Complete
    );
    let deck = vault.instance(SkateboardAttributeClass::Deck.key(), 0xd7ed_bd36_2d7d_2152);
    let value = |key| {
        f32::from_bits(
            deck.try_scalar_word(key, 0)
                .unwrap()
                .expect("original deck scalar"),
        )
    };
    let half_width = value(0xd76d_9db5_ee1f_d008) * 0.5;
    let half_length = value(0xc94d_6509_1dcb_828b) * 0.5;
    assert_eq!(
        geometry.box_half_width().unwrap().to_bits(),
        half_width.to_bits()
    );
    assert_eq!(
        geometry.box_half_length().unwrap().to_bits(),
        half_length.to_bits()
    );
    assert_eq!(
        geometry.total_half_length().unwrap().to_bits(),
        (value(0x1d53_bc53_9321_90e7) + half_length).to_bits()
    );
    let radians = value(0x8020_3de9_f9b7_32ca) * f32::from_bits(0x3c8e_fa35);
    // This original bounded value admits an independent trigonometric check;
    // endpoint/VMX graph fidelity is covered by frozen tangent unit oracles.
    assert!(radians > 0.01 && radians < 1.5);
    assert!((f64::from(geometry.end_slope().unwrap()) - f64::from(radians).tan()).abs() < 0.00001);
    assert!(runtime.state().do_ik());
    assert_eq!(runtime.state().modes(), &[0; 4]);
    let data = runtime.data_in();
    for matrix in [
        data.animation_to_world,
        data.world_to_animation,
        data.physics_skateboard,
        data.inverse_skateboard,
        data.hips,
        data.skateboard,
    ] {
        assert_eq!(matrix.known_lanes().unwrap().map(f32::to_bits), [0; 16]);
    }
    assert_eq!(
        data.toe_contact_normal,
        crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4])
    );
    assert_eq!(
        data.hips_offset,
        crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4])
    );
}

fn assert_visual_precedes_malformed_board(original: &Arc<PhysicalPlayerAssemblyOwner>) {
    use crate::attribute_runtime::tests::{collection, definition, graph, node};
    let class = SkateboardAttributeClass::Trucks.key();
    let distance = 0xb1cb_e3e9_29af_c8a0; // board-only joint-enable byte
    let mut malformed = graph(class, vec![definition(distance, 4, 4)]);
    collection(
        &mut malformed,
        class,
        skeleton_body_attributes::DEFAULT_COLLECTION,
        0,
        None,
        vec![node(distance, 0x40, 0x7f00_0000)],
    );
    let mut inputs = copy_inputs(original.inputs());
    let grind_class = crate::physical_player_grind_air_adjust::GRIND_AIR_CLASS;
    let count_key = 0x9759_3d0d_4f34_2621;
    crate::attribute_runtime::tests::add_fixture_class(
        &mut malformed,
        grind_class,
        vec![definition(count_key, 4, 4)],
    );
    collection(
        &mut malformed,
        grind_class,
        skeleton_body_attributes::DEFAULT_COLLECTION,
        0,
        None,
        vec![node(count_key, 0x40, 4)],
    );
    inputs.vault = Arc::new(malformed);
    let mut selected = world();
    let mut factory = PhysicalPlayerAssemblyConstruction::new(&mut selected, inputs);
    let mut services = Services {
        flags: 0,
        next_flags: None,
        events: Vec::new(),
    };
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::VisualIslandUnsupported { flags: 0 })
    ));
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::BoardRouting);
    assert_eq!(services.events, ["flags"]);
    assert!(factory.board().is_none());
    assert!(factory.state.skeleton_construction.is_none());
    assert_eq!(factory.world().physical_body_registry().next_body_id(), 1);
    assert_eq!(factory.world().hook_count(), 0);
    assert_eq!(
        factory
            .world()
            .backend()
            .rigid_body_snapshot()
            .free_order
            .len(),
        34
    );
}

fn assert_uninitialized_pool_retains_actual_definition(
    original: &Arc<PhysicalPlayerAssemblyOwner>,
) {
    let mut selected = PhysicalAssemblyWorld::try_new(35, 28, 49, 0).unwrap();
    let mut factory =
        PhysicalPlayerAssemblyConstruction::new(&mut selected, copy_inputs(original.inputs()));
    assert!(matches!(
        factory.try_construct_board(&mut Services::simulation()),
        Err(PhysicalPlayerAssemblyError::Pool(
            SimulationSkaterPoolError::Uninitialized
        ))
    ));
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::BoardAdmission);
    let candidate = factory.state.board_candidate.as_ref().unwrap();
    assert_eq!(candidate.world_index(), 0xffff);
    assert!(!candidate.session().is_attached());
    assert_eq!(candidate.construction().unwrap().island_body_id(), None);
    assert!(Arc::ptr_eq(
        candidate.construction().unwrap().physical_owner().unwrap(),
        factory.owner()
    ));
    assert_eq!(factory.world().physical_body_registry().next_body_id(), 1);
    factory.try_teardown().unwrap();
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Removed);
    assert_eq!(
        factory
            .world()
            .backend()
            .rigid_body_snapshot()
            .free_order
            .len(),
        34
    );
    assert!(
        factory.state.board_candidate.is_some(),
        "explicit cleanup retains the actual successful definition prefix"
    );
}

#[path = "../runtime/physical_player_roster_startup_corpus.rs"]
mod roster_startup_corpus;
