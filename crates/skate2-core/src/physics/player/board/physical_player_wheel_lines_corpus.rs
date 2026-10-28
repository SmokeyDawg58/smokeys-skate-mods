//! Actual board Start -> queued `ApplyForces` -> ground solve -> End. This is
//! controlled scheduling, not the missing complete `BodyPostPhysics`/Actor path.
use super::PhysicalAssemblyWorld;
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        contact_feedback::ControlledNoFreezing, skateboard_ground::SkateboardGroundFrameInput,
    },
    skateboard_body_construction::SkateboardRetainedWheelLines,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::{convert::Infallible, sync::Arc};

/// Explicit diagnostic cvar provider, original default words from the source
/// packet. This does not claim the missing live console/registration owner.
pub(crate) struct OriginalPostCvars;
impl
    crate::simulation_assembly_backend::constraint_session::board_post_services::SkateboardPostCvars
    for OriginalPostCvars
{
    type Error = Infallible;
    type DeckTarget = ();
    type LineTarget = ();
    type FreeWheelTarget = ();
    fn capture_deck_end(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
    fn deck_end(&mut self, (): &()) -> Result<f32, Infallible> {
        Ok(f32::from_bits(0x3f6b_851f))
    }
    fn capture_line_distance(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
    fn line_distance(&mut self, (): &()) -> Result<f32, Infallible> {
        Ok(f32::from_bits(0x3d8f_5c29))
    }
    fn capture_free_wheel_drag(&mut self) -> Result<(), Infallible> {
        panic!("constructor clears freewheel bit")
    }
    fn free_wheel_drag(&mut self, (): &()) -> Result<f32, Infallible> {
        panic!("inactive freewheel")
    }
    fn forced_surface(&mut self) -> Result<u32, Infallible> {
        Ok(0)
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one actual source-ordered batch/solve/contact/output corpus schedule"
)]
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    reckoning: &Reckoning<ReckoningVaultAttributes>,
    ground: &StaticGroundPool,
    processed: &mut crate::skeleton_processed_contact::SkeletonProcessedContact,
) {
    let positions: [_; 5] = [0, 1, 2, 3, 6].map(|part| {
        let matrix = world
            .board(board)
            .unwrap()
            .try_live_part_transform_host(world.backend(), part)
            .unwrap();
        SkeletonRootVector {
            xyz: [matrix[12], matrix[13], matrix[14]],
            w: None,
        }
    });
    let up = reckoning.up();
    let retained = world
        .try_board_wheel_line_state::<Infallible>(board)
        .unwrap();
    assert!(!retained.lock().unwrap().has_batch());
    world
        .try_start_board_wheel_lines(board, ground, up, &mut HostCollisionLineNumeric)
        .unwrap();
    let batch_address = {
        let state = retained.lock().unwrap();
        assert_eq!(state.deck.start, Some(positions[4]));
        assert_eq!(state.deck.start.unwrap().w, None);
        assert_eq!(state.deck.valid, Some(false));
        std::ptr::from_ref(state.batch_for_test().unwrap())
    };
    let inputs = world.wheel_line_inputs_for_test(board);
    assert_eq!(inputs.len(), 5);
    for (index, (line, position)) in inputs.iter().zip(positions).enumerate() {
        assert_eq!(line.start, position);
        let expected = if index < 4 {
            std::array::from_fn(|lane| position.xyz[lane] - up.xyz[lane] * 0.2)
        } else {
            [position.xyz[0], position.xyz[1] - 100.0, position.xyz[2]]
        };
        assert_eq!(line.end.xyz.map(f32::to_bits), expected.map(f32::to_bits));
        assert_eq!(line.end.w, None);
        assert_eq!(
            line.fatness.to_bits(),
            if index < 4 { 0 } else { 0.15_f32.to_bits() }
        );
    }
    // Diagnostic result reads observe the actual retained World batch; they
    // neither replace its inputs nor supply End with independent fake hits.
    let before_solve = world.wheel_line_results_for_test(board);
    assert_eq!(before_solve.len(), 5);
    assert!(
        before_solve[4].is_some(),
        "actual fat deck query must hit original ground: start={:?}, end={:?}, results={before_solve:?}",
        inputs[4].start,
        inputs[4].end
    );
    let queued = world
        .try_with_board_force_runtime(board, |runtime| runtime.queue.len())
        .unwrap();
    world.try_apply_board_forces(board).unwrap();
    assert_eq!(
        world
            .try_with_board_force_runtime(board, |runtime| runtime.queue.len())
            .unwrap(),
        queued
    );
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
            f32::from_bits(0x3d08_8889),
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    assert!(counts.contacts > 0);
    assert_eq!(report.processed_assemblies, 2);
    let after_owner = world
        .try_board_wheel_line_state::<Infallible>(board)
        .unwrap();
    assert!(Arc::ptr_eq(&retained, &after_owner));
    assert_eq!(
        std::ptr::from_ref(after_owner.lock().unwrap().batch_for_test().unwrap()),
        batch_address
    );
    assert_eq!(world.wheel_line_results_for_test(board), before_solve);
    world
        .try_end_board_wheel_lines(board, ground, &mut HostCollisionLineNumeric)
        .unwrap();
    let state = retained.lock().unwrap();
    check_end(&state, &before_solve, positions[4]);
    drop(state);
    check_post_prefix(world, board, up);
    let before_timer = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .post_fields()
        .unwrap()
        .time_since_wheels_touching;
    world
        .try_finish_board_postphysics(
            board,
            &mut crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
        )
        .unwrap();
    let fields = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .post_fields()
        .unwrap();
    let expected = if fields.num_wheels == 0 {
        before_timer + f32::from_bits(0x3d08_8889)
    } else {
        0.0
    };
    assert_eq!(
        fields.time_since_wheels_touching.to_bits(),
        expected.to_bits()
    );
    assert!(matches!(world.try_finish_board_postphysics(board,&mut crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric),Err(crate::simulation_assembly_backend::constraint_session::board_post_end::PhysicalPlayerBoardPostEndError::Refused("Body post suffix already attempted"))));
    check_up_post(world, board, skeleton, processed);
    world
        .try_publish_pair_board_ground_up(board, skeleton)
        .unwrap();
    let last = world
        .try_with_board_force_runtime(board, |runtime| {
            runtime
                .contact_surface
                .lock()
                .unwrap()
                .collision_state
                .last_valid_normal
                .unwrap()
        })
        .unwrap();
    world
        .try_with_pair_phys_out(board, skeleton, |output| {
            use crate::physical_player_phys_out::{PhysOutGroundVector, PhysOutOrigin};
            for field in [
                PhysOutGroundVector::GroundUp,
                PhysOutGroundVector::LastValidGroundUp,
            ] {
                let copied = output.ground_vector(field).unwrap();
                assert_eq!(copied.value, last);
                assert_eq!(copied.origin, PhysOutOrigin::Publication);
            }
        })
        .unwrap();
    println!(
        "actual five-test wheel batch: Part0..3/Reckoning up thin rays and Part6 positive fat ground hit retained across ApplyForces and real solve; End publishes identical retained results and releases batch; controlled schedule, no full BodyPostPhysics/Actor claim"
    );
    super::motion_corpus::check(world, board, skeleton, processed);
    check_stale_frame(world, board, skeleton, ground, up);
}

fn check_stale_frame(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    ground: &StaticGroundPool,
    up: SkeletonRootVector,
) {
    // A genuine subsequent solve, without a new wheel batch, must not certify
    // the previous results or normal as belonging to this new frame.
    let motion_sample = world.try_sample_pair_board_motion(board, skeleton).unwrap();
    world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            f32::from_bits(0x3d08_8889),
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    let before = world.backend().rigid_body_snapshot();
    assert!(matches!(
        world.try_publish_pair_board_motion(board, skeleton, motion_sample,
            &mut crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric),
        Err(crate::simulation_assembly_backend::constraint_session::board_motion::PhysicalPlayerBoardMotionError::WrongFrame)
    ));
    assert!(matches!(world.try_update_board_post_ground_normals(board,f32::from_bits(0x3d08_8889),up,&mut OriginalPostCvars,&mut HostReckoningNumeric,&mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric),Err(crate::simulation_assembly_backend::constraint_session::board_postphysics::PhysicalPlayerBoardPostError::Refused("wheel Start/End does not belong to current solved frame"))));
    assert_eq!(world.backend().rigid_body_snapshot(), before);
    assert!(matches!(world.try_publish_pair_board_ground_up(board,skeleton),Err(crate::simulation_assembly_backend::constraint_session::physical_phys_out::PhysicalPlayerSurfaceOutputError::Publication(crate::physical_player_phys_out::PhysOutPublicationError::Source(crate::simulation_assembly_backend::constraint_session::physical_phys_out::BoardSurfacePublicationReadError::WrongFrame)))));
}

fn check_up_post(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    processed: &crate::skeleton_processed_contact::SkeletonProcessedContact,
) {
    use crate::simulation_assembly_backend::constraint_session::{
        board_up_input::PhysicalPlayerBoardUpInputError,
        board_up_post::PhysicalPlayerBoardUpPostError,
    };
    let before = world
        .try_with_board_force_runtime(board, |runtime| {
            let state = runtime.up_state.lock().unwrap();
            (
                state.dynamic_up,
                state.dynamic_speed,
                state.average_normal,
                runtime.queue.len(),
            )
        })
        .unwrap();
    let unbound = crate::skeleton_processed_contact::SkeletonProcessedContact::new();
    assert!(matches!(
        world.try_finish_board_ordinary_up_post(
            board,
            skeleton,
            &unbound,
            &mut HostReckoningNumeric
        ),
        Err(PhysicalPlayerBoardUpPostError::Input(
            PhysicalPlayerBoardUpInputError::ForeignProcessedOutput
        ))
    ));
    assert_eq!(
        world
            .try_with_board_force_runtime(board, |runtime| runtime.queue.len())
            .unwrap(),
        before.3
    );
    world
        .try_finish_board_ordinary_up_post(board, skeleton, processed, &mut HostReckoningNumeric)
        .unwrap();
    let wheels = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .post_fields()
        .unwrap()
        .num_wheels;
    world
        .try_with_board_force_runtime(board, |runtime| {
            assert_eq!(runtime.queue.len(), 0);
            if wheels == 0 {
                let state = runtime.up_state.lock().unwrap();
                assert_eq!(state.dynamic_up, before.0);
                assert_eq!(state.dynamic_speed, before.1);
                assert_eq!(state.average_normal, before.2);
                assert_eq!(
                    state.contact_acceleration,
                    Some(SkeletonRootVector::known([0.0; 4]))
                );
            }
        })
        .unwrap();
    assert!(matches!(
        world.try_finish_board_ordinary_up_post(
            board,
            skeleton,
            processed,
            &mut HostReckoningNumeric
        ),
        Err(PhysicalPlayerBoardUpPostError::Refused(
            "ordinary up post already attempted"
        ))
    ));
    println!(
        "actual Body suffix -> same outer UpdateUpVector (wheel count {wheels}) -> same force queue clear; wrong processed occurrence/replay refuse; ordinary selected continuation, exceptional branches and Actor remain pending"
    );
}

fn check_post_prefix(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    up: SkeletonRootVector,
) {
    let frames: [_; 5] = [6, 0, 1, 2, 3].map(|part| {
        world
            .board(board)
            .unwrap()
            .try_attached_part_body_transform(world.backend(), part)
            .unwrap()
    });
    let distance = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .wheel_distance()
        .unwrap();
    let normal_result = world
        .try_update_board_post_ground_normals(
            board,
            f32::from_bits(0x3d08_8889),
            up,
            &mut OriginalPostCvars,
            &mut HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
        .unwrap();
    assert!(normal_result.num_parts > 0);
    println!("actual normal endpoint outcome: {normal_result:?}");
    let contacted = world
        .try_with_board_force_runtime(board, |runtime| {
            runtime
                .contact_surface
                .lock()
                .unwrap()
                .collision_state
                .in_contact
        })
        .unwrap();
    for (wheel, contact) in contacted[..4].iter().enumerate() {
        let expected = if contact.unwrap() {
            0.0
        } else {
            f32::from_bits(0x3bc4_9ba6) * f32::from_bits(0x426f_ffff)
        };
        let inertia = world.board(board).unwrap().template().part_payloads()[wheel]
            .inertia
            .try_read()
            .unwrap();
        assert_eq!(inertia.angular_drag.to_bits(), expected.to_bits());
    }
    let before_replay = world.backend().rigid_body_snapshot();
    assert!(matches!(world.try_update_board_post_ground_normals(board,f32::from_bits(0x3d08_8889),up,&mut OriginalPostCvars,&mut HostReckoningNumeric,&mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric),Err(crate::simulation_assembly_backend::constraint_session::board_postphysics::PhysicalPlayerBoardPostError::Refused("contact frame already attempted"))));
    assert_eq!(world.backend().rigid_body_snapshot(), before_replay);
    let actual = world.board_wheel_compressions(board).unwrap().unwrap();
    // Independent f64 Cramer's-rule projection; production uses the recovered
    // f32 cofactor/refinement and translation/FMA graph instead.
    let deck = frames[0].columns();
    let x = deck[0].xyz.map(f64::from);
    let y = deck[1].xyz.map(f64::from);
    let z = deck[2].xyz.map(f64::from);
    let determinant = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| {
        a[0] * (b[1] * c[2] - b[2] * c[1]) - b[0] * (a[1] * c[2] - a[2] * c[1])
            + c[0] * (a[1] * b[2] - a[2] * b[1])
    };
    let det = determinant(x, y, z);
    assert!(det.abs() > 0.5);
    let local_y: [f64; 4] = std::array::from_fn(|i| {
        let displacement = std::array::from_fn(|lane| {
            f64::from(frames[i + 1].columns()[3].xyz[lane]) - f64::from(deck[3].xyz[lane])
        });
        determinant(x, displacement, z) / det
    });
    for (index, value) in actual.into_iter().enumerate() {
        let expected = local_y[index * 2].midpoint(local_y[index * 2 + 1]) - f64::from(distance);
        assert!(
            (f64::from(value) - expected).abs() < 0.0005,
            "compression {index}: {value} vs {expected}"
        );
    }
    println!(
        "actual board contact/normal endpoint: retained Vault wheel distance and current body frames agree with independent f64 projection; same solved contacts, current wheel results and real inertia stores consumed; separate truck/up continuation follows; Actor remains pending"
    );
}

fn check_end(
    state: &SkateboardRetainedWheelLines,
    before_solve: &[Option<crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineHit>],
    deck_start: SkeletonRootVector,
) {
    assert!(!state.has_batch());
    let mut height = 0.2_f32;
    for (index, result) in before_solve[..4].iter().enumerate() {
        let (normal, distance) = result.as_ref().map_or(
            (SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]), 0.0),
            |hit| (hit.normal, hit.line_parameter * 0.2),
        );
        assert_eq!(state.normals[index], Some(normal));
        assert_eq!(
            state.distances[index].unwrap().to_bits(),
            distance.to_bits()
        );
        if result.is_some()
            && matches!(
                (distance - height).partial_cmp(&0.0),
                Some(std::cmp::Ordering::Less) | None
            )
        {
            height = distance;
        }
    }
    assert_eq!(state.height.unwrap().to_bits(), height.to_bits());
    let deck = before_solve[4].as_ref().unwrap();
    assert_eq!(state.deck.valid, Some(true));
    assert_eq!(state.deck.position, Some(deck.position));
    assert_eq!(state.deck.normal, Some(deck.normal));
    assert_eq!(state.deck.surface, Some(u32::from(deck.surface_id)));
    assert_eq!(state.deck.start, Some(deck_start));
}
