#![expect(clippy::float_cmp, reason = "source copy and exact fixture assertions")]
use super::*;
use crate::{
    collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig},
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    input_cvar_runtime::InputCvarManager,
    physical_player_remaining_constructor_owners::{SkeletonBinding, WorldBinding},
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig,
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
    skateboard_assembly::SkateboardSimulationBackend,
    skeleton_processed_contact::{SkeletonProcessedContact, SkeletonProcessedContactBoard},
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::{Arc, Mutex, Weak};

struct FailEstimate;
impl ReckoningNumeric for FailEstimate {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(crate::reckoning_numeric::host_dot3(a, b))
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        Err("seed")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        unreachable!()
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        unreachable!()
    }
}

struct Board;
impl SkeletonProcessedContactBoard for Board {
    type Error = std::convert::Infallible;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        Ok(2.0)
    }
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        Ok(SkeletonRootMatrix::from_columns([
            V::known([1.0, 0.0, 0.0, 0.0]),
            V::known([0.0, 1.0, 0.0, 0.0]),
            V::known([0.0, 0.0, 1.0, 0.0]),
            V::known([0.0; 4]),
        ]))
    }
    fn flags_8c4(&self) -> u32 {
        0
    }
}
fn owner(manager: SimulationManagerContactAuthority) -> LandingOnDeck {
    let vault = Arc::new(crate::attribute_runtime::tests::graph(
        0x2377_cef0_ff59_c4d2,
        vec![],
    ));
    let mut processed = SkeletonProcessedContact::new();
    processed
        .try_initialize_constructor_fields_in_sim_allocation()
        .unwrap();
    processed.try_sample_toolkit_prefix(&mut Board).unwrap();
    LandingOnDeck {
        skeleton: SkeletonBinding {
            owner: Weak::new(),
            processed: Arc::new(Mutex::new(processed)),
            skeleton: SimulationSkaterAssemblyHandle::landing_manager_test_handle(),
        },
        world: WorldBinding {
            owner: Weak::new(),
            manager,
        },
        attributes: vault.instance(0x2377_cef0_ff59_c4d2, 0xd7ed_bd36_2d7d_2152),
        trajectory_batch: None,
        trajectory: RetainedTrajectory::constructed(),
        ideal_trajectory: RetainedTrajectory::constructed(),
        trajectory_time: 0.0,
        trajectory_valid: false,
        landing_error: V::known([0.0; 4]),
        landing_velocity: V::known([0.0; 4]),
        test_trajectory_end_y: 0.0,
        time_to_land: 0.0,
        ideal_time_to_land: 0.0,
        can_land: false,
        landing_hippy_jump: false,
        something_in_the_way: false,
        tested_trajectory: false,
    }
}
fn cvars() -> (InputCvarReferences, InputCvarManager) {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    (refs, manager)
}
fn config() -> SimulationWorldLineQueryConfig {
    let cap = CollisionQueryBatchCapacity {
        batches: 1,
        tests: 1,
    };
    let batches = CollisionQueryBatchConfig {
        line: cap,
        boxes: cap,
        trajectory: cap,
        step_selector: cap,
    };
    SimulationWorldLineQueryConfig {
        vehicle_update_in_parallel: false,
        ground: batches,
        dmo: batches,
        vehicle: batches,
    }
}

#[test]
fn assisted_copy_preserves_unknown_w_and_existing_elapsed_time() {
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    landing.ideal_trajectory.position = V {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    landing.ideal_trajectory.velocity = V {
        xyz: [3.0, 4.0, 0.0],
        w: None,
    };
    landing.ideal_trajectory.gravity = GRAVITY;
    landing.ideal_time_to_land = 0.75;
    landing.trajectory_time = 9.0;
    let (refs, _) = cvars();
    let error = landing
        .init_assisted_trajectory(
            V::known([0.0; 4]),
            &refs,
            &mut HostReckoningNumeric,
            &DebugOverlaySlot::default(),
        )
        .unwrap();
    assert_eq!(error, 5.0);
    assert_eq!(landing.trajectory, landing.ideal_trajectory);
    assert_eq!(landing.trajectory_time, 9.0);
    assert_eq!(landing.time_to_land, 0.75);
    assert!(landing.trajectory_valid);
}

#[test]
fn assisted_seed_failure_preserves_exact_copy_prefix_before_valid_and_time_stores() {
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    landing.ideal_trajectory.position = V::known([3.0; 4]);
    landing.ideal_trajectory.velocity = V::known([4.0; 4]);
    landing.ideal_trajectory.gravity = GRAVITY;
    landing.ideal_trajectory.time = [22.0; 4];
    landing.ideal_time_to_land = 5.0;
    let error = landing
        .init_assisted_trajectory(
            V::known([0.0; 4]),
            &InputCvarReferences::new_static_initialized(),
            &mut FailEstimate,
            &DebugOverlaySlot::default(),
        )
        .unwrap_err();
    assert!(matches!(error, LandingError::Numeric("seed")));
    assert_eq!(
        landing.trajectory.position,
        landing.ideal_trajectory.position
    );
    assert_eq!(
        landing.trajectory.velocity,
        landing.ideal_trajectory.velocity
    );
    assert_eq!(landing.trajectory.gravity, GRAVITY);
    assert_eq!(landing.trajectory.time, [-1.0; 4]);
    assert!(!landing.trajectory_valid);
    assert_eq!(landing.time_to_land, 0.0);
}

#[test]
fn real_output_obstruction_gate_and_write_lock_preserve_existing_output() {
    use crate::physical_player_phys_out::construct::PhysOutAllocation;
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    let (refs, mut cvar_manager) = cvars();
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let overlay = DebugOverlaySlot::default();
    landing.can_land = true;
    landing.something_in_the_way = true;
    landing
        .fill_phys_out::<()>(&mut output, &refs, &overlay)
        .unwrap();
    assert!(!output.try_offboard_output().unwrap().can_land_on_skateboard);
    landing.something_in_the_way = false;
    landing
        .fill_phys_out::<()>(&mut output, &refs, &overlay)
        .unwrap();
    assert!(output.try_offboard_output().unwrap().can_land_on_skateboard);
    output.lock_after_update1().unwrap();
    landing.can_land = false;
    assert!(matches!(
        landing.fill_phys_out::<()>(&mut output, &refs, &overlay),
        Err(LandingError::Output(_))
    ));
    assert!(output.try_offboard_output().unwrap().can_land_on_skateboard);
    cvar_manager
        .try_deserialize_startup(Some(b"[Config]\nkbLandingOnDeck_DebugDraw=1\n"))
        .unwrap();
    assert!(matches!(
        landing.fill_phys_out::<()>(&mut output, &refs, &overlay),
        Err(LandingError::Overlay(DebugOverlayError::Missing))
    ));
}

#[test]
fn consider_uses_half_relative_gravity_only_when_board_has_no_wheels() {
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    let (refs, _) = cvars();
    let overlay = DebugOverlaySlot::default();
    let position = V {
        xyz: [0.0, 1.0, 0.0],
        w: None,
    };
    landing
        .consider_skateboard(
            position,
            V::known([0.0; 4]),
            &refs,
            &mut HostReckoningNumeric,
            &overlay,
        )
        .unwrap();
    let airborne_time = landing.time_to_land;
    assert_eq!(landing.ideal_trajectory.position.w, None);
    assert_eq!(landing.ideal_trajectory.velocity.w, None);
    assert_eq!(landing.ideal_trajectory.gravity, GRAVITY);
    assert_eq!(landing.landing_error.xyz[1], 0.0);
    assert_eq!(landing.landing_error.w, None);
    landing
        .skeleton
        .processed
        .lock()
        .unwrap()
        .physics_state_fields_mut()
        .num_wheels_in_contact = Some(1);
    landing
        .consider_skateboard(
            position,
            V::known([0.0; 4]),
            &refs,
            &mut HostReckoningNumeric,
            &overlay,
        )
        .unwrap();
    assert!(airborne_time > landing.time_to_land);
    assert!((airborne_time / landing.time_to_land - 2.0_f32.sqrt()).abs() < 0.00001);
}

#[test]
fn real_empty_world_query_retains_lease_until_postphysics_and_then_allows_reuse() {
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let dmos = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let mut queries =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    let (refs, _) = cvars();
    let overlay = DebugOverlaySlot::default();
    overlay.try_construct().unwrap();
    for _ in 0..2 {
        landing
            .start_trajectory_test(
                V {
                    xyz: [0.0, 1.0, 0.0],
                    w: None,
                },
                0.5,
                &refs,
                &mut LandingWorld {
                    queries: &mut queries,
                    ground: &ground,
                    dmos: &dmos,
                },
                &mut HostReckoningNumeric,
                &overlay,
            )
            .unwrap();
        assert!(landing.trajectory_batch.is_some());
        assert!(landing.something_in_the_way);
        assert_eq!(landing.test_trajectory_end_y, 0.5);
        landing
            .update_post_physics::<std::convert::Infallible>(&mut queries, &refs, &overlay)
            .unwrap();
        assert!(landing.trajectory_batch.is_none());
        assert!(!landing.something_in_the_way);
    }
}

#[test]
fn missing_overlay_stops_before_acquisition_but_after_obstruction_store() {
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let dmos = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let mut queries =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    let (refs, _) = cvars();
    let error = landing
        .start_trajectory_test(
            V::known([0.0, 1.0, 0.0, 0.0]),
            0.5,
            &refs,
            &mut LandingWorld {
                queries: &mut queries,
                ground: &ground,
                dmos: &dmos,
            },
            &mut HostReckoningNumeric,
            &DebugOverlaySlot::default(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        LandingError::Overlay(DebugOverlayError::Missing)
    ));
    assert!(landing.something_in_the_way);
    assert!(landing.trajectory_batch.is_none());
    assert_eq!(landing.test_trajectory_end_y, 0.0);
}

#[test]
fn skipped_start_gate_preserves_can_land_but_resets_tested() {
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let dmos = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let mut queries =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, config()).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    landing
        .skeleton
        .processed
        .lock()
        .unwrap()
        .physics_state_fields_mut()
        .flags_8d0 = Some(0x40000);
    landing.can_land = true;
    landing.tested_trajectory = true;
    // Unlinked cvars are deliberately untouched by the short-circuited branch.
    landing
        .start_land_on_board_query(
            V::known([0.0; 4]),
            V::known([0.0; 4]),
            1.0,
            &InputCvarReferences::new_static_initialized(),
            &mut LandingWorld {
                queries: &mut queries,
                ground: &ground,
                dmos: &dmos,
            },
            &mut HostReckoningNumeric,
            &DebugOverlaySlot::default(),
        )
        .unwrap();
    assert!(landing.can_land);
    assert!(!landing.tested_trajectory);
    assert!(landing.trajectory_batch.is_none());
}

#[test]
fn trajectory_initialization_and_velocity_preserve_unknown_w() {
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    landing.trajectory_time = 8.0;
    let position = V {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    let velocity = V {
        xyz: [4.0, 5.0, 6.0],
        w: None,
    };
    landing
        .init_trajectory::<std::convert::Infallible>(position, velocity)
        .unwrap();
    assert_eq!(landing.trajectory.position, position);
    assert_eq!(landing.trajectory.time, [-1.0; 4]);
    assert_eq!(landing.trajectory_time, 0.0);
    assert!(landing.trajectory_valid);
    landing.trajectory_time = 0.5;
    assert_eq!(landing.current_velocity().w, None);
    assert_eq!(
        landing.current_velocity().xyz[1],
        GRAVITY.xyz[1].mul_add(0.5, 5.0)
    );
    landing.can_land = true;
    landing.something_in_the_way = true;
    assert!(!landing.can_land_on_deck());
    landing.something_in_the_way = false;
    assert!(landing.can_land_on_deck());
}

#[test]
fn correction_commits_position_before_failing_adjustment_seed() {
    struct FailReciprocal;
    impl ReckoningNumeric for FailReciprocal {
        type Error = &'static str;
        fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
            unreachable!()
        }
        fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            unreachable!()
        }
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            Err("reciprocal")
        }
        fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
            assert!(value > 59.0);
            Ok(59)
        }
    }
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let mut landing = owner(manager.assembly_pool_authority());
    landing.trajectory.position = V::known([2.0, 3.0, 4.0, 5.0]);
    landing.trajectory.velocity = V::known([1.0; 4]);
    landing.time_to_land = 1.0;
    let before_velocity = landing.trajectory.velocity;
    let (refs, _manager) = cvars();
    assert!(matches!(
        landing.correct_trajectory(
            V::known([1.0; 4]),
            &refs,
            &mut FailReciprocal,
            &DebugOverlaySlot::default()
        ),
        Err(LandingError::Adjustment(_))
    ));
    assert_eq!(landing.trajectory.position, V::known([3.0, 5.0, 7.0, 9.0]));
    assert_eq!(landing.trajectory.velocity, before_velocity);
}
