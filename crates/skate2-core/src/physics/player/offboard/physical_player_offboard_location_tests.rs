#![expect(clippy::float_cmp, reason = "exact source publication assertions")]
use super::*;
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig},
    physical_player_remaining_constructor_owners::WorldBinding,
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig,
    skateboard_assembly::SkateboardSimulationBackend,
};
use std::sync::Weak;

#[derive(Default)]
struct Tuning {
    fail_dot: bool,
    fail_trajectory: bool,
    reads: Vec<&'static str>,
}
impl LocationTuning for Tuning {
    type Error = &'static str;
    fn line_length(&mut self) -> Result<f32, Self::Error> {
        Ok(0.55)
    }
    fn start_height(&mut self) -> Result<f32, Self::Error> {
        Ok(0.7)
    }
    fn line_count(&mut self) -> Result<i32, Self::Error> {
        self.reads.push("count");
        Ok(3)
    }
    fn test_radius(&mut self) -> Result<f32, Self::Error> {
        Ok(0.165)
    }
    fn trajectory_start_height(&mut self) -> Result<f32, Self::Error> {
        if self.fail_trajectory {
            Err("trajectory")
        } else {
            Ok(1.5)
        }
    }
    fn trajectory_end_length(&mut self) -> Result<f32, Self::Error> {
        Ok(0.8)
    }
    fn trajectory_end_height(&mut self) -> Result<f32, Self::Error> {
        Ok(0.5)
    }
    fn minimum_y_normal(&mut self) -> Result<f32, Self::Error> {
        self.reads.push("minimum_y");
        Ok(0.3)
    }
    fn minimum_normal_dot(&mut self) -> Result<f32, Self::Error> {
        self.reads.push("minimum_dot");
        if self.fail_dot { Err("dot") } else { Ok(-0.9) }
    }
    fn turn_speed(&mut self) -> Result<f32, Self::Error> {
        self.reads.push("speed");
        Ok(2.0)
    }
    fn step_normal_y_threshold(&mut self) -> Result<f32, Self::Error> {
        self.reads.push("step_y");
        Ok(0.5)
    }
}
fn sentinel() -> LocationResult {
    LocationResult {
        landing_position: V {
            xyz: [9.0; 3],
            w: None,
        },
        turn: 7.0,
        landing_position_valid: true,
        blocked: false,
    }
}

#[test]
fn wall_turn_preserves_unwritten_fields_and_later_hits_replace_turn() {
    let mut result = sentinel();
    let mut tuning = Tuning::default();
    for (z, expected) in [(0.8, -2.0), (-0.8, 2.0)] {
        process_line_hit(
            &mut result,
            V::known([-0.6, 0.0, z, 0.0]),
            V::known([1.0, 0.0, 0.0, 0.0]),
            &mut tuning,
            &mut HostCollisionLineNumeric,
        )
        .unwrap();
        assert!(result.blocked);
        assert_eq!(result.turn, expected);
        assert_eq!(result.landing_position, sentinel().landing_position);
        assert!(result.landing_position_valid);
    }
    assert_eq!(
        tuning.reads,
        [
            "minimum_y",
            "minimum_dot",
            "speed",
            "minimum_y",
            "minimum_dot",
            "speed"
        ]
    );
}
#[test]
fn failed_minimum_dot_read_precedes_blocked_publication() {
    let mut result = sentinel();
    let mut tuning = Tuning {
        fail_dot: true,
        ..Default::default()
    };
    let error = process_line_hit(
        &mut result,
        V::known([-0.6, 0.0, 0.8, 0.0]),
        V::known([1.0, 0.0, 0.0, 0.0]),
        &mut tuning,
        &mut HostCollisionLineNumeric,
    )
    .unwrap_err();
    assert!(matches!(error, LocationError::Tuning("dot")));
    assert_eq!(result, sentinel());
}
#[test]
fn unordered_height_is_blocked_without_normalizing_or_turning() {
    let mut result = sentinel();
    let mut tuning = Tuning::default();
    process_line_hit(
        &mut result,
        V::known([0.0, f32::NAN, 0.0, 0.0]),
        V::known([1.0, 0.0, 0.0, 0.0]),
        &mut tuning,
        &mut HostCollisionLineNumeric,
    )
    .unwrap();
    assert!(result.blocked);
    assert_eq!(result.turn, 7.0);
    assert_eq!(tuning.reads, ["minimum_y"]);
}
#[test]
fn real_empty_world_queries_reuse_released_batches_and_preserve_unknown_w() {
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let dmos = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let cap = CollisionQueryBatchCapacity {
        batches: 1,
        tests: 4,
    };
    let batches = CollisionQueryBatchConfig {
        line: cap,
        boxes: cap,
        trajectory: cap,
        step_selector: cap,
    };
    let cfg = SimulationWorldLineQueryConfig {
        vehicle_update_in_parallel: false,
        ground: batches,
        dmo: batches,
        vehicle: batches,
    };
    let mut queries =
        SimulationWorldLineQueries::try_new(&manager, &backend, &dmos, 3, cfg).unwrap();
    let ground = StaticGroundPool::try_new(1).unwrap();
    let mut location = OffboardLocation {
        world: WorldBinding {
            owner: Weak::new(),
            manager: manager.assembly_pool_authority(),
        },
        ground_position: V::known([0.0; 4]),
        look_direction: V::known([0.0; 4]),
        jump_velocity: V::known([0.0; 4]),
        jumping: false,
        collide_with_world_only: false,
        sync_allowed: false,
        line_batch: None,
        trajectory_batch: None,
    };
    let input = LocationInput {
        ground: V {
            xyz: [0.0; 3],
            w: None,
        },
        look: V {
            xyz: [1.0, 0.0, 0.0],
            w: None,
        },
        jump_velocity: V::known([0.0; 4]),
        jumping: false,
        world_only: false,
    };
    let mut tuning = Tuning::default();
    for clear in [false, true, false] {
        location
            .start_query(
                input,
                &mut LocationWorld {
                    visual_island_enabled: false,
                    queries: &mut queries,
                    ground: &ground,
                    dmos: &dmos,
                },
                &mut tuning,
                &mut HostCollisionLineNumeric,
            )
            .unwrap();
        assert!(!location.sync_allowed);
        assert_eq!(location.ground_position.w, None);
        let mut result = sentinel();
        assert!(matches!(
            location.get_results(
                &mut result,
                &mut queries,
                &mut tuning,
                &mut HostCollisionLineNumeric
            ),
            Err(LocationError::NotReady)
        ));
        assert_eq!(result, sentinel());
        // Same readiness publication as the existing physical Update1 Tick.
        location.sync_allowed = true;
        if clear {
            location.clear_data(&mut queries).unwrap();
        } else {
            location
                .get_results(
                    &mut result,
                    &mut queries,
                    &mut tuning,
                    &mut HostCollisionLineNumeric,
                )
                .unwrap();
            assert_eq!(result, sentinel());
        }
        assert!(location.line_batch.is_none());
        assert!(location.trajectory_batch.is_none());
        assert!(!location.sync_allowed);
    }
    tuning.fail_trajectory = true;
    assert!(matches!(
        location.start_query(
            input,
            &mut LocationWorld {
                visual_island_enabled: false,
                queries: &mut queries,
                ground: &ground,
                dmos: &dmos
            },
            &mut tuning,
            &mut HostCollisionLineNumeric
        ),
        Err(LocationError::Tuning("trajectory"))
    ));
    assert!(location.line_batch.is_some());
    assert!(location.trajectory_batch.is_none());
    location.clear_data(&mut queries).unwrap();
    assert!(location.line_batch.is_none());
}
