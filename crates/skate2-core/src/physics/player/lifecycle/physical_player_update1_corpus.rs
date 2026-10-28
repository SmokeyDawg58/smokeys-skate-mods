//! Original Actor input through selected Update1/2/3 and Ground preparation.
//! Includes actual GroundSkateboard/Pumping/control and selected later Update5 board
//! output through its return. Remaining force/solver phases are unscheduled.
use super::*;
use crate::collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig};
use crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig;

pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &crate::dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    skeleton: &crate::dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    actor: &crate::actor_construction::ActorConstruction,
) {
    check_mode(world, board, skeleton, actor, false);
}

#[expect(
    clippy::too_many_lines,
    reason = "one consecutive original Actor input and physical preparation path"
)]
pub(super) fn check_mode(
    world: &mut PhysicalAssemblyWorld,
    board: &crate::dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    skeleton: &crate::dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    actor: &crate::actor_construction::ActorConstruction,
    solved_fill: bool,
) {
    let mut cvars = actor
        .cached_physical_allocation()
        .unwrap()
        .owner()
        .inputs()
        .cvars
        .clone();
    let grind_residency = super::grind_corpus::mount(&world.grind_services, &cvars);
    let ground = crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_robs_park_ground_for_board_test();
    let capacity = CollisionQueryBatchCapacity {
        batches: 4,
        tests: 5,
    };
    let batches = CollisionQueryBatchConfig {
        line: capacity,
        boxes: capacity,
        trajectory: capacity,
        step_selector: capacity,
    };
    world
        .try_initialize_world_line_queries(
            SimulationWorldLineQueryConfig {
                vehicle_update_in_parallel: false,
                ground: batches,
                dmo: batches,
                vehicle: batches,
            },
            false,
        )
        .unwrap();
    let physical = actor.cached_physical_allocation().unwrap();
    let expected = actor.published_animation_physics_input().unwrap();
    let old_category = physical
        .owner()
        .phys_out()
        .lock()
        .unwrap()
        .try_guarded_state_output()
        .unwrap()
        .physics_state_category;
    let continuation = world
        .try_begin_player_update1(
            board,
            skeleton,
            &actor,
            &expected,
            &ground,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    assert!(Arc::ptr_eq(&continuation.owner, physical.owner()));
    assert!(Arc::ptr_eq(&continuation.input, &expected));
    assert_eq!(continuation.prior_category, old_category);
    {
        let tail = continuation.owner.tail().lock().unwrap();
        assert!(
            tail.line_records
                .as_ref()
                .unwrap()
                .iter()
                .all(|line| line.trajectory_batch.is_some())
        );
    }
    assert_eq!(
        continuation
            .processed
            .lock()
            .unwrap()
            .physics_state_fields()
            .time_delta,
        Some(1.0 / 60.0)
    );
    let continuation = world
        .try_continue_player_update1_controls(board, skeleton, continuation)
        .unwrap()
        .continuation;
    // Actual previous PhysOut has not yet received the Ground state's output.
    assert_eq!(old_category, 0);
    {
        let mut processed = continuation.processed.lock().unwrap();
        assert!(processed.selected_surface().is_some());
        assert_eq!(
            processed.input_controls_mut().transition.unwrap().to_bits(),
            0
        );
        assert_eq!(
            processed
                .input_controls_mut()
                .time_on_ground
                .unwrap()
                .to_bits(),
            0
        );
        assert_eq!(
            processed.input_controls_mut().frames_since_teleport,
            Some(0)
        );
        assert_eq!(processed.known_flags_8c4().0 & 0x2000, 0x2000);
        assert_eq!(processed.known_flags_8c8().unwrap() & 0x8000, 0);
        let prefix = continuation.owner.prefix().lock().unwrap();
        let prefix = prefix.as_ref().unwrap();
        assert_eq!(prefix.ground_state_count, 0);
        assert_eq!(prefix.non_collision_flags, 0xc000_0000);
    }
    {
        let input = continuation.input.lock().unwrap();
        let output = continuation.owner.phys_out().lock().unwrap();
        continuation
            .processed
            .lock()
            .unwrap()
            .try_copy_prior_input_runtime(&output, &input)
            .unwrap();
    }
    continuation
        .processed
        .lock()
        .unwrap()
        .try_copy_prior_output_tail(&continuation.owner)
        .unwrap();
    continuation
        .processed
        .lock()
        .unwrap()
        .try_copy_prior_output_body(&continuation.owner)
        .unwrap();
    continuation
        .processed
        .lock()
        .unwrap()
        .try_copy_prior_output_wheel_timers(&continuation.owner)
        .unwrap();
    {
        let processed = continuation.processed.lock().unwrap();
        assert_eq!(processed.known_flags_8c8().unwrap() & 0x1e00_0000, 0);
        let timers = processed.prior_wheel_timer_fields();
        assert!(timers.complete);
        assert_eq!(timers.skitching_vehicle_id, Some(0));
        assert_eq!(timers.skitch_delay_until_reskitchable.unwrap().to_bits(), 0);
        assert_eq!(timers.pushing_delay_until_repushable.unwrap().to_bits(), 0);
    }
    world
        .try_continue_player_update1_ground(
            board,
            skeleton,
            &continuation,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    {
        let mut processed = continuation.processed.lock().unwrap();
        let ground = processed.ground_position_fields_mut();
        assert_eq!(ground.time_offboard.unwrap().to_bits(), 0);
        // First-frame teleport history suppresses damping even though the
        // constructor's frames-since-jump counter permits the position copy.
        assert_eq!(
            ground.damped_ground_velocity.unwrap(),
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4])
        );
    }
    world
        .try_prepare_player_toolkit(
            board,
            skeleton,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    world
        .try_fill_skeleton_line_surfaces(board, skeleton)
        .unwrap();
    {
        let processed = continuation.processed.lock().unwrap();
        assert!(processed.physical_mass().unwrap() > 0.0);
        assert!(processed.toolkit_state().directions_complete());
        // Fill consumes retained previous results while this frame's three
        // trajectory batches are still leased and have not been collected.
        let lines = processed.line_surfaces();
        assert!(!lines.left.unwrap().valid);
        assert!(!lines.right.unwrap().valid);
        assert!(!lines.hips.unwrap().valid);
    }
    world
        .try_process_player_skeleton_data_prefix(
            board,
            skeleton,
            &continuation,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    world
        .try_process_input_after_skeleton(
            board,
            skeleton,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    // Explicit corpus schedule selection, not a native query-miss substitute.
    // Grinds were parked for the first rendered test. Opt in to reproduce
    // the unfinished native prefix and its documented deck-W refusal.
    if std::env::var_os("SKATE2_TEST_GRIND_PREFIX").is_some() {
        world
            .try_update_player_grind_prefix(board, skeleton)
            .unwrap();
    } else {
        eprintln!(
            "selected first-test schedule: GrindManager excluded by first-test scope; no complete native Update1 claim"
        );
    }
    world
        .try_publish_pair_jump_timing(board, skeleton, &cvars)
        .unwrap();
    world
        .try_poll_pair_trajectory_selector(board, skeleton)
        .unwrap();
    world
        .try_update_pair_heading(
            board,
            skeleton,
            &cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    world
        .try_update_pair_permission_latches(board, skeleton, &cvars)
        .unwrap();
    world
        .try_finish_pair_process_input(board, skeleton, &cvars)
        .unwrap();
    world
        .try_update_pair_system_logic_with_actor(
            board,
            skeleton,
            actor,
            &mut cvars,
            Some(&ground),
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            &mut |angle| Ok(angle.cos()),
            &mut None,
        )
        .unwrap();
    world
        .try_update_pair_prediction_before_state(
            board,
            skeleton,
            &ground,
            &cvars,
            &mut crate::physical_player_grind_manager::math::GrindNum {
                n: &mut crate::reckoning_numeric::HostReckoningNumeric,
                s: &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            },
            &mut crate::collision_line_numeric::HostCollisionLineNumeric,
        )
        .unwrap();
    let ground_reckoning = world
        .try_update_pair_ground_reckoning(
            board,
            skeleton,
            &cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    world
        .try_update_pair_ground_handplants(board, skeleton)
        .unwrap();
    world
        .try_update_pair_ground_skateboard(
            board,
            skeleton,
            &ground,
            &cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            &mut crate::skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
        )
        .unwrap();
    world
        .try_update_pair_ground_pumping(
            board,
            skeleton,
            &cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    world
        .try_update_pair_ground_control(
            board,
            skeleton,
            &cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
        .unwrap();
    let before_fake_torque = world
        .try_update_pair_ground_wallride(
            board,
            skeleton,
            &ground,
            &cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &before_fake_torque.owner,
        &continuation.owner
    ));
    assert_eq!(
        before_fake_torque.wallride_force.local_space_position,
        crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4])
    );
    // The force remains retained for its later original queue stage.
    let before_jump = world
        .try_update_pair_ground_fake_torque(
            board,
            skeleton,
            before_fake_torque,
            &cvars,
            &mut crate::skateboard_force_runtime::fake_torque::HostFakeTorqueNumeric,
        )
        .unwrap();
    // Exercise the old JumpLaunchPending boundary on the same retained pair.
    // Restore the stationary fixture before continuing its ordinary-force checks.
    let before_jump = {
        use crate::simulation_assembly_backend::constraint_session::{
            ground_collision::GroundAfterCollision,
            ground_fake_torque::GroundBeforeJump,
        };
        let owner = std::sync::Arc::clone(&before_jump.0.owner);
        let velocity = crate::skeleton_root_transforms::SkeletonRootVector {
            xyz: [3.0, 4.0, -5.0], w: None,
        };
        let saved = {
            let mut state = owner.physics_state().lock().unwrap();
            let fields = &mut state.ground.as_mut().unwrap().fields;
            let saved = (fields.do_jump, fields.jump_direction, fields.exiting_wallride);
            fields.do_jump = true;
            fields.jump_direction = velocity;
            saved
        };
        let queued = world.ground_force_queue_for_test(board);
        let result = world.try_update_pair_ground_collision(
            board, skeleton, before_jump, &cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        ).unwrap();
        assert_eq!(world.ground_force_queue_for_test(board).len(), queued.len());
        let GroundAfterCollision::JumpLaunch { before, velocity: selected } = result else {
            panic!("doJump must select the launch continuation before collision forces");
        };
        assert_eq!(selected, velocity);
        assert!(std::sync::Arc::ptr_eq(&before.owner, &owner));
        {
            let mut state = owner.physics_state().lock().unwrap();
            let fields = &mut state.ground.as_mut().unwrap().fields;
            assert_eq!(fields.exiting_wallride, saved.2, "launch has not completed yet");
            fields.do_jump = saved.0;
            fields.jump_direction = saved.1;
        }
        GroundBeforeJump(before)
    };
    let after_collision = world
        .try_update_pair_ground_collision(
            board,
            skeleton,
            before_jump,
            &cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    let before_prephysics = match after_collision {
        crate::simulation_assembly_backend::constraint_session::ground_collision::GroundAfterCollision::JumpLaunch { .. } => {
            panic!("stationary Ground corpus unexpectedly requested a wallride launch");
        }
        crate::simulation_assembly_backend::constraint_session::ground_collision::GroundAfterCollision::BeforeBraking(before) => {
            assert!(std::sync::Arc::ptr_eq(&before.owner, &continuation.owner));
            let queue_before = world.ground_force_queue_for_test(board);
            let forces = world.try_update_pair_ground_ordinary_forces(
                board, skeleton,
                crate::simulation_assembly_backend::constraint_session::ground_collision::GroundAfterCollision::BeforeBraking(before),
                &cvars, &mut crate::reckoning_numeric::HostReckoningNumeric,
            ).unwrap();
            assert!(std::sync::Arc::ptr_eq(&forces.before.owner, &continuation.owner));
            let published = continuation.owner.physics_state().lock().unwrap()
                .ground.as_ref().unwrap().fields.anti_flip_torque.unwrap();
            assert_eq!(published, forces.antiflip);
            eprintln!("actual Ground torques through82d9a770: straighten {:?}, heading {:?}, antiFlip {:?}; same Ground+90 published", forces.straighten, forces.heading, forces.antiflip);
            let queue_after = world.ground_force_queue_for_test(board);
            assert_eq!(queue_before, queue_after);
            eprintln!("actual Ground ordinary forces through pump82d9a72c; threshold {:?}, retained brake/push/front/rear/speed/slide/pump {:?}; original queue unchanged, manual effect/application/solver suffix pending",
                forces.foot_threshold, [forces.brake, forces.push, forces.front, forces.rear, forces.speed, forces.slide, forces.pump]);
            let manual = world.try_update_pair_ground_manual_nudge(
                board, skeleton, forces, &cvars,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            ).unwrap();
            assert!(std::sync::Arc::ptr_eq(&manual.before.before.owner, &continuation.owner));
            assert_eq!(queue_before, world.ground_force_queue_for_test(board));
            eprintln!("actual Ground through Nudge82d9a7e4: manual {:?}, linear drag {:?}, push+brake {:?}; neutral nudge preserves queue", manual.manual, manual.linear_drag, manual.combined_push_brake);
            let queued = world.try_update_pair_ground_queue_torque_prefix(
                board, skeleton, manual, &mut crate::reckoning_numeric::HostReckoningNumeric,
            ).unwrap();
            assert!(std::sync::Arc::ptr_eq(&queued.0.before.before.owner, &continuation.owner));
            let records = world.ground_force_queue_for_test(board);
            assert_eq!(&records[..queue_before.len()], queue_before.as_slice());
            assert_eq!(records[queue_before.len()..].iter().map(|v| v.kind).collect::<Vec<_>>(), [4,5,2,3,6,7,1,8,16]);
            eprintln!("actual Ground queues/torque/drag through82d9a980: retained records {records:?}; ManageHangUps and remaining solver schedule pending");
            let completed = world.try_update_pair_ground_hang_pinning(
                board, skeleton, queued, &cvars, &mut crate::reckoning_numeric::HostReckoningNumeric,
            ).unwrap();
            assert!(std::sync::Arc::ptr_eq(&completed.owner, &continuation.owner));
            let state = continuation.owner.physics_state().lock().unwrap();
            let ground = &state.ground.as_ref().unwrap().fields;
            assert_eq!(ground.was_pinning, ground.is_pinning);
            eprintln!("actual Ground.UpdateSkateboard returned through82d9a998; retained hang state and pinning evaluated; outer Ground/controller/solver schedule remains selected");
            drop(state);
            let predicted = world.try_update_pair_ground_force_prediction(
                board, skeleton, completed, &mut crate::reckoning_numeric::HostReckoningNumeric,
            ).unwrap();
            assert!(std::sync::Arc::ptr_eq(&predicted.0.owner, &continuation.owner));
            assert_eq!(records, world.ground_force_queue_for_test(board));
            let completed = world.try_finish_pair_ground_update(board, skeleton, predicted).unwrap();
            assert!(std::sync::Arc::ptr_eq(&completed.0.0.owner, &continuation.owner));
            eprintln!("actual Ground suffix through82da109c: push exit, Skeleton flag, timer, actual grab invalidation");
            eprintln!("actual Ground force-based hook/board prediction through82da1024; retained queue preserved and SAME hook Part0/predicted position updated");
            Some(world.try_update3_controller_after_ground(board, skeleton, completed).unwrap())




        }
        crate::simulation_assembly_backend::constraint_session::ground_collision::GroundAfterCollision::CollisionExit(before) => {
            assert!(std::sync::Arc::ptr_eq(&before.owner, &continuation.owner));
            eprintln!("actual Ground collision response -> native UpdateSkateboard exit; later controller/force/solver schedule pending");
            None
        }
    };
    if solved_fill {
        let queued = world.ground_force_queue_for_test(board);
        let processed_timer = continuation
            .owner
            .reckoning()
            .lock()
            .unwrap()
            .processed()
            .lock()
            .unwrap()
            .physics_state_fields()
            .state_timer;
        let update3 = world
            .try_finish_update3_prephysics(
                board,
                skeleton,
                before_prephysics.expect("ordinary Ground controller returned"),
                &ground,
                &mut crate::collision_line_numeric::HostCollisionLineNumeric,
            )
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(&update3.owner, &continuation.owner));
        assert_eq!(queued, world.ground_force_queue_for_test(board));
        let retained = world
            .try_with_board_force_runtime(board, |runtime| runtime.expected_displacement)
            .unwrap();
        assert_eq!(retained, Some(update3.expected_displacement));
        assert_eq!(
            continuation
                .owner
                .physics_state()
                .lock()
                .unwrap()
                .state_timer,
            Some(update3.state_timer)
        );
        assert_eq!(
            continuation
                .owner
                .reckoning()
                .lock()
                .unwrap()
                .processed()
                .lock()
                .unwrap()
                .physics_state_fields()
                .state_timer,
            processed_timer
        );
        solved_fill::check(
            world,
            board,
            skeleton,
            &continuation.owner,
            &ground,
            &ground_reckoning,
            actor,
        );
        grind_residency.finish(&world.grind_services, &cvars);
        return;
    }
    // Selected later PrePhysics query prefix (82b37cfc). Remaining Ground,
    // controller, force application and solver phases are still omitted here.
    // Read the actual late Reckoning up+1b0; this is not groundNormal+1f0.
    let up = continuation
        .owner
        .reckoning()
        .lock()
        .unwrap()
        .reckoning()
        .unwrap()
        .up();
    world
        .try_start_board_wheel_lines(
            board,
            &ground,
            up,
            &mut crate::collision_line_numeric::HostCollisionLineNumeric,
        )
        .unwrap();
    // Selected Update4 collectors in source order, without a solver claim.
    world.try_end_skeleton_line_tests(board, skeleton).unwrap();
    world
        .try_end_board_wheel_lines(
            board,
            &ground,
            &mut crate::collision_line_numeric::HostCollisionLineNumeric,
        )
        .unwrap();
    // Selected later Update5 entry; intervening phases are still incomplete.
    // Actual ChoosePhysicsSurfaceType3709c predecessor, reading retained
    // collision state; the rest of Body.PostPhysics is still unscheduled here.
    world.try_choose_board_physics_surface(board).unwrap();
    let output_prefix = world.try_begin_player_update5(board, skeleton).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &output_prefix.owner,
        &continuation.owner
    ));
    let before_pumping = world
        .try_continue_update5_board_fill(
            board,
            skeleton,
            output_prefix,
            &mut crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
        )
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &before_pumping.owner,
        &continuation.owner
    ));
    let before_wheels = world
        .try_continue_update5_board_pumping(board, skeleton, before_pumping)
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &before_wheels.owner,
        &continuation.owner
    ));
    let before_deck = world
        .try_continue_update5_board_contacts(board, skeleton, before_wheels)
        .unwrap();
    let before_surface = world
        .try_continue_update5_board_deck(board, skeleton, before_deck)
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &before_surface.owner,
        &continuation.owner
    ));
    let before_wobble = world
        .try_continue_update5_board_surface(
            board,
            skeleton,
            before_surface,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &before_wobble.owner,
        &continuation.owner
    ));
    let before_forward = world
        .try_continue_update5_board_wobble(board, skeleton, before_wobble)
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &before_forward.owner,
        &continuation.owner
    ));
    assert!(before_forward.assembly.matches(board));
    assert!(!before_forward.assembly.matches(skeleton));
    let before_wheel_positions = world
        .try_continue_update5_board_forward(
            board,
            skeleton,
            before_forward,
            &mut crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
        )
        .unwrap();
    let before_ground = world
        .try_continue_update5_board_wheel_positions(board, skeleton, before_wheel_positions)
        .unwrap();
    let before_slide = world
        .try_continue_update5_board_ground(
            board,
            skeleton,
            before_ground,
            &mut crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
        )
        .unwrap();
    let after_board = world
        .try_finish_update5_board_fill(
            board,
            skeleton,
            before_slide,
            &mut crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
        )
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &after_board.owner,
        &continuation.owner
    ));
    let before_part19 = world
        .try_continue_update5_skeleton_prefix(
            board,
            skeleton,
            after_board,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &before_part19.owner,
        &continuation.owner,
    ));
    let before_angular = world
        .try_continue_update5_skeleton_parts(board, skeleton, before_part19)
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &before_angular.owner,
        &continuation.owner
    ));
    // Selected later Update5 spans. Skeleton suffix and intermediate manager
    // publications remain pending; this does not authorize output conditioning.
    crate::simulation_assembly_backend::constraint_session::update5_state_publication::try_publish_current_type(&continuation.owner, board, skeleton).unwrap();
    crate::simulation_assembly_backend::constraint_session::update5_state_publication::try_publish_category_and_dt(&continuation.owner, board, skeleton).unwrap();
    {
        let out = continuation.owner.phys_out().lock().unwrap();
        let state = out.try_state_output().unwrap();
        assert_eq!(
            (state.physics_state, state.physics_state_category),
            (100, 100)
        );
        assert!(!state.is_offboard && !state.is_offboard_ground);
        eprintln!(
            "actual Update5 selected publications: Ground type/category100, HiLOD count {}, processed DT; complete Ground.Fill and conditioner still pending",
            state.update_count
        );
    }
    grind_residency.finish(&world.grind_services, &cvars);
    crate::actor_construction::action_runtime_session::ground_corpus::check(actor);
    crate::actor_construction::action_runtime_session::released_trick_corpus::check(actor);
    eprintln!(
        "original SAME Actor bound input -> selected Update1/Update2/Update3 -> Ground Reckoning/Handplants/Skateboard/Pumping/control/WallRide and ordinary forces through82d9a72c; GrindManager explicitly opt-in; selected later line collection, actual surface selection and Update5 board Fill through82b3074c and Skeleton Fill through82b589b4; remaining control/force/solver phases and full physical frame pending"
    );
}

#[path = "../runtime/physical_player_actor_solved_fill_corpus.rs"]
mod solved_fill;
