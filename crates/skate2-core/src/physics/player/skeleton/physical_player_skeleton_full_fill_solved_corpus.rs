//! Original solved Actor-corpus child; requires actual pre-solve Actor
//! `ProcessData` on this SAME terrain pair. It does not create the missing input.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner, SimulationSkaterAssemblyHandle};
use crate::{
    physical_player_phys_out::PhysOutSystemVector, reckoning_numeric::HostReckoningNumeric,
    simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame,
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
};
use std::sync::Arc;

/// Runs after complete live Skeleton post-physics in the dedicated Actor fixture.
/// Every error remains a failing named stage; no input or output is synthesized.
#[expect(
    clippy::too_many_lines,
    reason = "one source-ordered complete Fill publication proof"
)]
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    solved_frame: &Arc<SimulationContactSpyFrame>,
    ground_reckoning: &crate::physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    actor: &crate::actor_construction::ActorConstruction,
) {
    assert!(Arc::ptr_eq(
        world.physical_body_member(board).unwrap().owner(),
        owner
    ));
    assert!(Arc::ptr_eq(
        world.physical_body_member(skeleton).unwrap().owner(),
        owner
    ));
    let occurrence = Arc::clone(owner.phys_out().lock().unwrap().occurrence());
    let processed = {
        let r = owner.reckoning().lock().unwrap();
        Arc::clone(r.processed())
    };
    let normal = world.skeleton(skeleton).unwrap();
    assert!(Arc::ptr_eq(
        normal.collision_frame_state().frame().unwrap(),
        solved_frame
    ));
    assert!(
        normal
            .state()
            .collision
            .specific_parts
            .contact
            .iter()
            .all(Option::is_some)
    );
    let expected_com_velocity = normal
        .post_physics_records()
        .centre_of_mass_velocity_typed();
    // Attached Part transforms do not retain the source engine-address W lanes.
    // The real history/COM producer must preserve that uncertainty through Fill.
    assert_eq!(expected_com_velocity.w, None);

    // Tokens are exclusively returned by actual production stages. No tuple
    // constructors, direct unlock, copied input values or synthetic line results.
    let before_board = world
        .try_begin_player_update5(board, skeleton)
        .expect("actual Update5 entry");
    let before_pumping = world
        .try_continue_update5_board_fill(
            board,
            skeleton,
            before_board,
            &mut HostSkeletonCollisionNumeric,
        )
        .expect("actual Board.Fill prefix");
    let before_contacts = world
        .try_continue_update5_board_pumping(board, skeleton, before_pumping)
        .expect("actual Board.Fill Pumping");
    let before_deck = world
        .try_continue_update5_board_contacts(board, skeleton, before_contacts)
        .expect("actual Board.Fill contacts");
    let before_surface = world
        .try_continue_update5_board_deck(board, skeleton, before_deck)
        .expect("actual Board.Fill deck");
    let before_wobble = world
        .try_continue_update5_board_surface(
            board,
            skeleton,
            before_surface,
            &mut HostReckoningNumeric,
        )
        .expect("actual Board.Fill surface");
    let before_forward = world
        .try_continue_update5_board_wobble(board, skeleton, before_wobble)
        .expect("actual Board.Fill wobble");
    let before_wheels = world
        .try_continue_update5_board_forward(
            board,
            skeleton,
            before_forward,
            &mut HostSkeletonCollisionNumeric,
        )
        .expect("actual Board.Fill forward");
    let before_ground = world
        .try_continue_update5_board_wheel_positions(board, skeleton, before_wheels)
        .expect("actual Board.Fill wheel positions");
    let before_slide = world
        .try_continue_update5_board_ground(
            board,
            skeleton,
            before_ground,
            &mut HostSkeletonCollisionNumeric,
        )
        .expect("actual Board.Fill ground");
    let after_board = world
        .try_finish_update5_board_fill(
            board,
            skeleton,
            before_slide,
            &mut HostSkeletonCollisionNumeric,
        )
        .expect("actual Board.Fill final");
    assert!(Arc::ptr_eq(&after_board.owner, owner));
    let before_parts = world
        .try_continue_update5_skeleton_prefix(
            board,
            skeleton,
            after_board,
            &mut HostReckoningNumeric,
        )
        .expect("actual Skeleton.Fill prefix (requires same Actor ProcessData input)");
    let captured = before_parts.captured_com_velocity;
    let expected_xyz = expected_com_velocity.xyz.map(f32::to_bits);
    assert_eq!(captured.xyz.map(f32::to_bits), expected_xyz);
    assert_eq!(
        captured.w.map(f32::to_bits),
        expected_com_velocity.w.map(f32::to_bits)
    );
    let before_angular = world
        .try_continue_update5_skeleton_parts(board, skeleton, before_parts)
        .expect("actual Skeleton.Fill parts");
    assert_eq!(
        before_angular.captured_com_velocity.xyz.map(f32::to_bits),
        captured.xyz.map(f32::to_bits)
    );
    assert_eq!(
        before_angular.captured_com_velocity.w.map(f32::to_bits),
        captured.w.map(f32::to_bits)
    );
    let before_pumping = world
        .try_continue_update5_skeleton_suffix(
            board,
            skeleton,
            before_angular,
            &owner.inputs().cvars,
            &mut HostReckoningNumeric,
        )
        .expect("full Skeleton.Fill mandatory IK/audio");
    assert!(Arc::ptr_eq(&before_pumping.owner, owner));
    let output = owner.phys_out().lock().unwrap();
    assert!(Arc::ptr_eq(output.occurrence(), &occurrence));
    let published = output
        .system_vector(PhysOutSystemVector::CentreOfMassVelocity)
        .unwrap()
        .value;
    assert_eq!(
        published.xyz.map(f32::to_bits),
        captured.xyz.map(f32::to_bits)
    );
    assert_eq!(published.w.map(f32::to_bits), captured.w.map(f32::to_bits));
    drop(output);
    assert!(Arc::ptr_eq(
        owner.reckoning().lock().unwrap().processed(),
        &processed
    ));
    let normal = world.skeleton(skeleton).unwrap();
    assert!(Arc::ptr_eq(
        normal.physics_history_frame_state().frame().unwrap(),
        solved_frame
    ));
    assert!(Arc::ptr_eq(
        normal.collision_frame_state().frame().unwrap(),
        solved_frame
    ));
    let before_grind = world
        .try_continue_update5_pumping(board, skeleton, before_pumping)
        .expect("actual same-Actor Pumping.Fill");
    assert!(Arc::ptr_eq(&before_grind.owner, owner));
    let before_handplant = world.try_continue_update5_managers(
        board, skeleton, before_grind, ground_reckoning,
        crate::simulation_assembly_backend::constraint_session::update5_managers::GrindManagerFillPolicy::ExcludeForFirstSkatingTest,
    ).expect("actual TrajectorySelector/Reckoning/StateChanger output");
    let before_controller = world
        .try_continue_update5_plants(board, skeleton, before_handplant)
        .expect("actual retained HandPlant/FootPlant output");
    let before_wipeout = world
        .try_continue_update5_controller(
            board,
            skeleton,
            before_controller,
            &mut HostReckoningNumeric,
        )
        .expect("actual controller mandatory Spine3 yaw and output flags");
    let after_wipeout = world
        .try_continue_update5_wipeout(board, skeleton, before_wipeout)
        .expect("actual Wipeout.Fill after plant/controller outputs");
    let before_type = world
        .try_continue_update5_outer_locality(board, skeleton, after_wipeout, actor)
        .expect("actual local identity and outer state flags before current type");

    // Explicit selected dependency check: GrindManager is parked and intervening
    // manager/Update5/UpdateAll callbacks remain pending. No full-frame token.
    let overlay = world.debug_overlay.clone();
    let attributes = world
        .try_with_board_force_runtime(board, |runtime| runtime.push_attributes.clone())
        .unwrap();
    let conditioned = crate::physical_player_assembly::selected_ground_conditioning::try_condition_selected_ground(
        world, before_type, board, skeleton, Some(&attributes),
        &owner.inputs().cvars, &overlay, &mut HostReckoningNumeric,
    ).expect("actual Ground.Fill -> retained Physics conditioner prefix");
    let conditioned = world
        .try_finish_selected_physics(board, skeleton, conditioned, &mut HostReckoningNumeric)
        .expect("actual retained landing-quality and stairs callbacks complete Physics.Update");
    assert!(Arc::ptr_eq(&conditioned.owner, owner));
    assert_eq!(conditioned.grind_policy,
        crate::simulation_assembly_backend::constraint_session::update5_managers::GrindManagerFillPolicy::ExcludeForFirstSkatingTest);

    let output = owner.phys_out().lock().unwrap();
    assert!(conditioned.physics.matches_output(&output));
    assert_eq!(
        output.try_filtered_output().unwrap().filtered_physics_state,
        crate::physics::FilteredPhysicsState::Ground
    );
    drop(output);
    let animated = world
        .try_finish_selected_animation(board, skeleton, conditioned, None)
        .expect("actual retained five-call Animation conditioner after Physics");
    assert!(Arc::ptr_eq(&animated.owner, owner));
    assert_eq!(animated.grind_policy,
        crate::simulation_assembly_backend::constraint_session::update5_managers::GrindManagerFillPolicy::ExcludeForFirstSkatingTest);
    let output = owner.phys_out().lock().unwrap();
    assert!(animated.animation.matches_output(&output));
    drop(output);
    // Consume the real newly conditioned output in the next Motion evaluation.
    // This is a callback continuation proof, not a second complete physics tick.
    actor.try_clear_motion_attributes().unwrap();
    actor.try_update_motion(1.0 / 60.0,
        &mut crate::simulation_assembly_backend::constraint_session::motion_force_physics::MotionForcePhysicsWriter::new(world, skeleton))
        .expect("actual grounded Motion lifecycle after seven Physics/five Animation conditioners");
    actor.check_grounded_motion_selection();
    eprintln!(
        "same Actor completed all seven Physics/five Animation conditioners and consumed output through grounded Motion lifecycle"
    );
}
