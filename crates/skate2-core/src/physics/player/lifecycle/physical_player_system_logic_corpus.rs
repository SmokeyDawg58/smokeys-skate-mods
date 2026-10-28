//! Selected Update2 over SAME fresh Ground pair, genuine original-terrain line
//! results and unchanged supplied pose mapped through the existing source producer.
//! The mapping/batch schedule is controlled; complete Update1/Update3 and Actor
//! animation sampling are not claimed by this integration fixture. This supplied
//! placement produces genuine misses and selects Air. Separate fresh-pair
//! schedules exercise the selected refusal and the actual Actor transition.
use crate::physics_state_ids::PHYSICS_STATE_PHYSICS_GROUND;
use super::*;

#[path = "../ground/physical_player_grounded_system_logic_corpus.rs"]
pub(crate) mod grounded;
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig},
    input_cvar_runtime::InputCvarReferences,
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig,
};

#[expect(
    clippy::too_many_lines,
    reason = "one measured actual-pair mapping/query/state/failure-prefix integration fixture"
)]
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    refs: &mut InputCvarReferences,
    actor: Option<&crate::actor_construction::ActorConstruction>,
) {
    let owner = world
        .skaters
        .as_mut()
        .unwrap()
        .try_with_jump_pair(&world.backend, board, skeleton, |_, owner| {
            Arc::clone(owner)
        })
        .unwrap();
    assert!(!owner.wipeout().lock().unwrap().requests_initialized());

    let ground = crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_robs_park_ground_for_board_test();
    let capacity = CollisionQueryBatchCapacity {
        batches: 1,
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
    let up = owner.reckoning().lock().unwrap().reckoning().unwrap().up();
    let retained = world
        .try_board_wheel_line_state::<std::convert::Infallible>(board)
        .unwrap();
    assert_eq!(retained.lock().unwrap().height.unwrap().to_bits(), 0);
    world
        .try_start_board_wheel_lines(board, &ground, up, &mut HostCollisionLineNumeric)
        .unwrap();
    let hits = world.wheel_line_results_for_test(board);
    world
        .try_end_board_wheel_lines(board, &ground, &mut HostCollisionLineNumeric)
        .unwrap();
    let height = retained.lock().unwrap().height.unwrap();
    assert!(Arc::ptr_eq(
        &retained,
        &world
            .try_board_wheel_line_state::<std::convert::Infallible>(board)
            .unwrap()
    ));
    assert!(!retained.lock().unwrap().has_batch());

    world
        .skaters
        .as_mut()
        .unwrap()
        .try_with_jump_pair(&world.backend, board, skeleton, |normal, same_owner| {
            assert!(Arc::ptr_eq(same_owner, &owner));
            normal
                .map_original_pose_for_animation_query(&same_owner.inputs().pose_matrices)
                .unwrap();
        })
        .unwrap();
    let animated = world
        .skeleton(skeleton)
        .unwrap()
        .try_is_skateboard_animated()
        .unwrap();
    let (clear, recent, grace) = {
        use crate::physical_player_state_changer::attributes::{
            CLEAR_HEIGHT, CONTACT_GRACE, RECENT_HEIGHT,
        };
        let changer = owner.state_changer().lock().unwrap();
        let attributes = changer.attributes.as_ref().unwrap();
        (
            attributes.scalar(CLEAR_HEIGHT).unwrap(),
            attributes.scalar(RECENT_HEIGHT).unwrap(),
            attributes.scalar(CONTACT_GRACE).unwrap(),
        )
    };
    let timer = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .post_fields()
        .unwrap()
        .time_since_wheels_touching;
    let before = owner.state_changer().lock().unwrap().fields.unwrap();
    if let Some(actor) = actor {
        assert!(hits.iter().all(Option::is_none));
        assert_eq!(height.to_bits(), 0.2_f32.to_bits());
        world
            .try_update_pair_system_logic_with_actor(
                board,
                skeleton,
                actor,
                refs,
                Some(&ground),
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                &mut |_| panic!("Air transition does not request ragdoll cosine"),
                &mut None,
            )
            .unwrap();
        let state = owner.physics_state().lock().unwrap();
        assert_eq!(state.current, CurrentPhysicalState::Air);
        assert_eq!(
            state.stage,
            crate::physical_player_physics_state::PhysicalStateStage::EntryComplete
        );
        assert_eq!(state.frames_since_jump, Some(100));
        let air = &state.air.as_ref().unwrap().fields;
        assert!(!air.use_com_velocity);
        assert_eq!(air.run_mount, Some(false));
        assert_eq!(air.sprint_mount, Some(false));
        drop(state);
        let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
        let fields = *processed.lock().unwrap().physics_state_fields();
        assert_eq!(
            (
                fields.last,
                fields.last_category,
                fields.next,
                fields.current
            ),
            (Some(100), Some(100), Some(200), Some(200))
        );
        // SetPhysicsState does not advance these separately published fields.
        assert_eq!((fields.physics, fields.category), (Some(0), Some(0)));
        assert!(owner.wipeout().lock().unwrap().requests_initialized());
        let after = owner.state_changer().lock().unwrap().fields.unwrap();
        assert_eq!(
            after.post_grind_jump_counter,
            before.post_grind_jump_counter.wrapping_add(1)
        );
        println!(
            "original five terrain misses -> SAME registered Actor/physical pair -> Ground.Exit -> Air.Enter -> actual Update2 request clearing; controlled schedule, no complete frame loop"
        );
        return;
    }
    let result = world.try_update_pair_system_logic(board, skeleton, refs);
    println!(
        "selected original Update2: actual line hits={:?}, height={height:?}, timer={timer:?}, air thresholds={clear:?}/{recent:?}/{grace:?}, mapped animated={animated}, result={result:?}",
        hits.iter().map(Option::is_some).collect::<Vec<_>>()
    );
    assert!(hits.iter().all(Option::is_none));
    assert_eq!(height.to_bits(), 0.2_f32.to_bits());
    assert!(height > clear);
    assert!(matches!(
        result,
        Err(SystemLogicError::PendingTransition(200))
    ));
    // CheckForStateChange commits HiLOD+128 before calling the missing Air
    // transition. Update2 must not clear the preceding Wipeout request ledger.
    assert_eq!(
        owner.physics_state().lock().unwrap().frames_since_jump,
        Some(100)
    );
    assert!(!owner.wipeout().lock().unwrap().requests_initialized());
    let after = owner.state_changer().lock().unwrap().fields.unwrap();
    assert_eq!(after.current, Some(PHYSICS_STATE_PHYSICS_GROUND));
    assert_eq!(
        after.post_grind_jump_counter,
        before.post_grind_jump_counter.wrapping_add(1)
    );
    assert_eq!(
        owner.physics_state().lock().unwrap().current,
        CurrentPhysicalState::Ground
    );
}
