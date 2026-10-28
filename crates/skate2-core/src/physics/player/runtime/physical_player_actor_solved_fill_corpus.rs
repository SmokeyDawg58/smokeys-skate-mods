//! Dedicated original Actor component schedule: solve, Skeleton post-physics,
//! and complete Skeleton output on the SAME pair initialized by `ProcessData`.
//! The selected chain includes board force application and ordinary post-physics.
//! Omitted controller phases still prevent a complete native-frame claim.
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        contact_feedback::ControlledNoFreezing, skateboard_ground::SkateboardGroundFrameInput,
    },
    skateboard_attributes::SkateboardAttributes,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

// Reuse the existing new-only full Fill proof; its imports resolve to the SAME
// owner types above. This does not recreate tokens or duplicate the Fill chain.
#[path = "../skeleton/physical_player_skeleton_full_fill_solved_corpus.rs"]
mod full_fill;
#[path = "../wipeout/physical_player_post_wipeout_pose_corpus.rs"]
mod pose_publication;

/// Insert immediately after actual `StartBoardWheelLines` in a fresh dedicated
/// invocation of the real Actor Update1 corpus, before either End collector.
/// The enclosing world must have contact capacity and must not already be solved.
#[expect(
    clippy::too_many_lines,
    reason = "one source-ordered actual Actor solve and post-physics proof"
)]
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    ground: &StaticGroundPool,
    ground_reckoning: &crate::physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    actor: &crate::actor_construction::ActorConstruction,
) {
    for member in [board, skeleton] {
        assert!(Arc::ptr_eq(
            world.physical_body_member(member).unwrap().owner(),
            owner
        ));
    }
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    let timestep = processed
        .lock()
        .unwrap()
        .physics_state_fields()
        .time_delta
        .expect("actual Processed dt+93c");
    let layout = owner
        .inputs()
        .vault
        .physics(SkateboardAttributes::DEFAULT_COLLECTION)
        .try_physics_layout()
        .unwrap();
    let gravity = std::array::from_fn(|axis| {
        f32::from_be_bytes(layout.raw_bytes[axis * 4..axis * 4 + 4].try_into().unwrap())
    });
    world.backend_mut().set_gravity(gravity);

    // The solver and collision feedback are real. Iteration budget and freezing
    // collaborator are explicit existing corpus controls, not native scheduler parity.
    let (_, report) = world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            timestep,
            50,
            &mut ControlledNoFreezing,
        )
        .expect("actual same-Actor pair solver/contact feedback");
    assert_eq!(report.processed_assemblies, 2);
    world.try_end_skeleton_line_tests(board, skeleton).unwrap();
    let result = world.try_update_board_postphysics_live(
        board, skeleton, ground,
        &mut crate::simulation_assembly_backend::constraint_session::board_post_outer_live::BoardPostNumerics {
            line: &mut HostCollisionLineNumeric,
            vector: &mut HostReckoningNumeric,
            rounding: &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            truck: &mut crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
        },
    ).expect("same Actor board post-physics with actual branch predicates");
    assert_eq!(result, 0);
    assert!(world.ground_force_queue_for_test(board).is_empty());

    // IsDeckInHand82dc8358 reads controller+1bc and returns state==1. HiLOD
    // 82e01d00 skips SkateboardBody+360 entirely when false. This older corpus
    // covers only that non-held branch; production post-solve reads the board's
    // retained contact count when held.
    let deck_in_hand = owner
        .skateboard_controller()
        .lock()
        .unwrap()
        .state()
        .expect("actual constructed controller state+1bc")
        == 1;
    assert!(
        !deck_in_hand,
        "this corpus covers only the non-held post-solve branch"
    );
    world
        .try_update_skeleton_postphysics_live(
            board,
            skeleton,
            &owner.inputs().cvars,
            deck_in_hand,
            &mut HostReckoningNumeric,
        )
        .expect("complete same-Actor Skeleton.UpdatePostPhysics");
    let overlay = world.debug_overlay.clone();
    let prior_timer = owner
        .wipeout()
        .lock()
        .unwrap()
        .try_ground_invincibility()
        .unwrap();
    world
        .try_update_ground_postphysics(board, skeleton, owner, &overlay, &mut HostReckoningNumeric)
        .expect("actual Ground.Post wipeout detector after Skeleton.Post");
    assert_eq!(
        owner
            .wipeout()
            .lock()
            .unwrap()
            .try_ground_invincibility()
            .unwrap()
            .to_bits(),
        (prior_timer - timestep).to_bits()
    );
    let old_global = owner
        .animation_physics_input
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .global_pose
        .clone()
        .unwrap();
    let before_ik = world
        .try_begin_post_wipeout(
            board,
            skeleton,
            owner,
            &mut HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
        .expect("actual post-wipeout error/wobble, root, IK, head and dispatched pose publication");
    assert!(!before_ik.deck.has_nan_xyz());
    eprintln!(
        "post-wipeout caller completed: air={} offboard={} contact={} teleport={} starting={}",
        before_ik.airborne,
        before_ik.offboard,
        before_ik.board_contact,
        before_ik.teleporting,
        before_ik.wipeout_starting
    );
    let normal = world.skeleton(skeleton).unwrap();
    let ik = normal.current_animation().ik_runtime().unwrap().state();
    assert_eq!(ik.external_target_normal_set(), &[false; 4]);
    assert_eq!(ik.external_target_set(), &[false; 4]);
    assert_eq!(ik.local_target_set(), &[false; 4]);
    let frame = Arc::clone(normal.physics_history_frame_state().frame().unwrap());
    assert!(Arc::ptr_eq(
        normal.collision_frame_state().frame().unwrap(),
        &frame
    ));
    // Zero contacts are allowed only as the actual terrain solver result; no
    // terrain translation, pose overlay or artificial contact is introduced.
    full_fill::check(
        world,
        board,
        skeleton,
        owner,
        &frame,
        ground_reckoning,
        actor,
    );
    pose_publication::check(world, skeleton, owner, &old_global);
    eprintln!(
        "same Actor solved component schedule: actual ProcessData -> board ApplyForces -> solver -> ordinary Board.PostPhysics -> complete Skeleton post-physics -> full Skeleton.Fill; remaining controller schedule excluded"
    );
}
