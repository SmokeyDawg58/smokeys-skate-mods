#![expect(
    clippy::float_cmp,
    reason = "exact controlled geometry and source sentinels"
)]
use super::*;
use crate::{
    collision_query_batches::{
        CollisionQueryBatchCapacity, CollisionQueryBatchConfig, CollisionQueryBatchState,
        CollisionQueryBatches,
    },
    reckoning_numeric::HostReckoningNumeric,
};

fn input() -> BallisticTrajectoryInput {
    BallisticTrajectoryInput {
        position: SkeletonRootVector {
            xyz: [0.0, 10.0, 0.0],
            w: None,
        },
        velocity: SkeletonRootVector::known([2.0, 4.0, 0.0, 17.0]),
        gravity: SkeletonRootVector {
            xyz: [0.0, -8.0, 0.0],
            w: None,
        },
        duration: 2.0,
        radius: 1.0,
        maximum_error_start: 1.0,
        maximum_error_end: 1.0,
    }
}
fn config(tests: u32) -> CollisionQueryBatchConfig {
    let capacity = CollisionQueryBatchCapacity { batches: 1, tests };
    CollisionQueryBatchConfig {
        line: capacity,
        boxes: capacity,
        trajectory: capacity,
        step_selector: capacity,
    }
}
#[derive(Default)]
struct Geometry {
    segments: Vec<([f32; 3], [f32; 3], f32)>,
    hit: Option<TrajectorySweepHit>,
    triangles: usize,
}
impl TrajectoryCollisionWorld for Geometry {
    type Error = CollisionError;
    fn sweep_sphere_segment(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Option<TrajectorySweepHit>, Self::Error> {
        self.segments.push((start, end, radius));
        Ok(self.hit.take())
    }
    fn triangles_overlapping_aabb(
        &mut self,
        _: [f32; 3],
        _: [f32; 3],
        capacity: usize,
    ) -> Result<Vec<TrajectoryGroundTriangle>, Self::Error> {
        assert_eq!(capacity, 64);
        self.triangles += 1;
        Ok(Vec::new())
    }
}

#[test]
fn real_dispatch_consumes_gravity_and_duration_while_skeleton_keeps_zero_g_one_second() {
    let request = input();
    let mut ballistic = Geometry::default();
    let result = run_request(
        TrajectoryInput::Ballistic(request),
        &mut ballistic,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(result, TrajectoryResult::initialized());
    assert_eq!(
        ballistic.segments,
        vec![
            ([0.0, 10.0, 0.0], [2.0, 10.0, 0.0], 1.0),
            ([2.0, 10.0, 0.0], [4.0, 2.0, 0.0], 1.0),
        ]
    );
    let mut skeleton = Geometry::default();
    run_request(
        TrajectoryInput::Skeleton(SkeletonTrajectoryInput {
            position: request.position,
            velocity: request.velocity,
            radius: request.radius,
        }),
        &mut skeleton,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(
        skeleton.segments,
        vec![([0.0, 10.0, 0.0], [2.0, 14.0, 0.0], 1.0)]
    );
}

#[test]
fn zero_gravity_ballistic_keeps_authored_duration_and_start_error_threshold() {
    let mut request = input();
    request.gravity.xyz = [0.0; 3];
    request.velocity.xyz = [0.25, 0.0, 0.0];
    request.duration = 3.0;
    request.maximum_error_start = 0.5;
    let mut geometry = Geometry::default();
    run_ballistic(request, &mut geometry, &mut HostReckoningNumeric).unwrap();
    // Chord .75 exceeds error-radius .5, but not the old incorrect radius 1.
    assert_eq!(
        geometry.segments,
        vec![([0.0, 10.0, 0.0], [0.75, 10.0, 0.0], 1.0)]
    );
}

#[test]
fn zero_error_linear_query_still_sweeps_a_short_chord() {
    let mut request = input();
    request.gravity.xyz = [0.0; 3];
    request.velocity.xyz = [0.1, 0.0, 0.0];
    request.duration = 1.0;
    request.radius = 1000.0;
    request.maximum_error_start = 0.0;
    let mut geometry = Geometry::default();
    run_ballistic(request, &mut geometry, &mut HostReckoningNumeric).unwrap();
    assert_eq!(geometry.segments.len(), 1);
}

#[test]
fn hit_conversion_preserves_fractional_time_and_unknown_geometry_w() {
    let mut request = input();
    request.gravity.xyz = [0.0; 3];
    request.velocity.xyz = [0.0, -2.0, 0.0];
    let mut geometry = Geometry {
        hit: Some(TrajectorySweepHit {
            position: [0.0, 9.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            surface_id: 19,
        }),
        ..Geometry::default()
    };
    let result = run_ballistic(request, &mut geometry, &mut HostReckoningNumeric).unwrap();
    assert_eq!(result.frame, 0);
    assert_eq!(result.time, [0.5 * f32::from_bits(0x3c88_8889); 4]);
    assert_eq!(result.surface, 19);
    assert_eq!(
        (result.position.w, result.normal.w, result.ground_normal.w),
        (None, None, None)
    );
    assert_eq!(geometry.triangles, 1);
}

#[test]
fn same_batch_retains_mixed_requests_and_requires_later_sync() {
    let mut batches = CollisionQueryBatches::<()>::new(config(2));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Trajectory, "mixed")
        .unwrap();
    batches.begin(&lease).unwrap();
    let ballistic = input();
    let skeleton = SkeletonTrajectoryInput {
        position: ballistic.position,
        velocity: ballistic.velocity,
        radius: ballistic.radius,
    };
    assert!(batches.add_skeleton_trajectory(&lease, skeleton).unwrap());
    assert!(batches.add_ballistic_trajectory(&lease, ballistic).unwrap());
    let mut observed = Vec::new();
    batches
        .end_trajectory(&lease, |request| {
            observed.push(request);
            Ok::<_, ()>(TrajectoryResult::initialized())
        })
        .unwrap();
    assert_eq!(
        observed,
        vec![
            TrajectoryInput::Skeleton(skeleton),
            TrajectoryInput::Ballistic(ballistic)
        ]
    );
    assert_eq!(batches.state(&lease), Ok(CollisionQueryBatchState::Running));
    assert!(batches.get_trajectory_result(&lease, 1).is_err());
    batches.sync(&lease).unwrap();
    assert_eq!(batches.get_trajectory_result(&lease, 1).unwrap(), None);
    batches.release(&lease).unwrap();
}

#[test]
fn gravity_nan_is_rejected_before_full_capacity_but_unknown_w_is_accepted() {
    let mut batches = CollisionQueryBatches::<()>::new(config(1));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Trajectory, "validation")
        .unwrap();
    batches.begin(&lease).unwrap();
    assert!(batches.add_ballistic_trajectory(&lease, input()).unwrap());
    let mut bad = input();
    bad.gravity.xyz[0] = f32::NAN;
    assert!(matches!(
        batches.add_ballistic_trajectory(&lease, bad),
        Err(TrajectoryBatchError::InputNaN)
    ));
    assert!(!batches.add_ballistic_trajectory(&lease, input()).unwrap());
}

#[test]
fn failing_ballistic_job_cannot_be_synced_into_a_successful_miss() {
    let mut batches = CollisionQueryBatches::<()>::new(config(1));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Trajectory, "failure")
        .unwrap();
    batches.begin(&lease).unwrap();
    batches.add_ballistic_trajectory(&lease, input()).unwrap();
    assert!(matches!(
        batches.end_trajectory(&lease, |_| Err::<TrajectoryResult, _>("geometry")),
        Err(TrajectoryBatchError::Execute {
            test: 0,
            source: "geometry"
        })
    ));
    assert!(batches.sync(&lease).is_err());
    assert!(batches.get_trajectory_result(&lease, 0).is_err());
}

#[derive(Default)]
struct Numeric {
    calls: Vec<(&'static str, f32)>,
    fail_at: Option<usize>,
}
impl Numeric {
    fn admit(&mut self, name: &'static str, value: f32) -> Result<(), &'static str> {
        self.calls.push((name, value));
        if self.fail_at == Some(self.calls.len()) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.admit("dot", a[0])?;
        HostReckoningNumeric.dot3(a, b).map_err(|_| "host")
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.admit("reciprocal", value)?;
        HostReckoningNumeric
            .reciprocal_estimate(value)
            .map_err(|_| "host")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.admit("rsqrt", value)?;
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(value)
            .map_err(|_| "host")
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        self.admit("truncate", value)?;
        HostReckoningNumeric.truncate_i32(value).map_err(|_| "host")
    }
}

#[test]
fn ballistic_numeric_failure_stops_before_geometry_and_interpolation_remains_lazy() {
    let mut numeric = Numeric {
        fail_at: Some(6),
        ..Numeric::default()
    };
    let mut geometry = Geometry::default();
    assert!(matches!(
        run_ballistic(input(), &mut geometry, &mut numeric),
        Err(TrajectoryJobError::Numeric("reciprocal"))
    ));
    assert_eq!(
        &numeric.calls[..4],
        &[
            ("reciprocal", -8.0),
            ("rsqrt", 1.0),
            ("reciprocal", -8.0),
            ("rsqrt", 1.0),
        ]
    );
    assert_eq!(numeric.calls[4].0, "dot");
    assert_eq!(numeric.calls[5], ("reciprocal", 2.0));
    assert_eq!(geometry.segments.len(), 1);
    let mut first = Numeric {
        fail_at: Some(1),
        ..Numeric::default()
    };
    let mut untouched = Geometry::default();
    assert!(run_ballistic(input(), &mut untouched, &mut first).is_err());
    assert!(untouched.segments.is_empty());
}

#[test]
fn linear_hit_numeric_conversion_failure_precedes_ground_normal_query() {
    let mut request = input();
    request.gravity.xyz = [0.0; 3];
    request.velocity.xyz = [0.0, -2.0, 0.0];
    let mut geometry = Geometry {
        hit: Some(TrajectorySweepHit {
            position: [0.0, 9.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            surface_id: 19,
        }),
        ..Geometry::default()
    };
    // chord dot; half-gravity reciprocal; distance dot/rsqrt; speed dot/rsqrt;
    // speed reciprocal; truncate. Source frame/time cannot be published on failure.
    let mut numeric = Numeric {
        fail_at: Some(8),
        ..Numeric::default()
    };
    assert!(matches!(
        run_ballistic(request, &mut geometry, &mut numeric),
        Err(TrajectoryJobError::Numeric("truncate"))
    ));
    assert_eq!(geometry.triangles, 0);
}

#[test]
fn generalized_world_job_does_not_add_legacy_radius_gravity_or_time_assertions() {
    let mut request = input();
    request.gravity.xyz = [1.0, 8.0, 2.0];
    request.duration = 101.0;
    request.radius = 0.001;
    let mut geometry = Geometry::default();
    run_ballistic(request, &mut geometry, &mut HostReckoningNumeric).unwrap();
    assert_eq!(geometry.segments.len(), 1);
    assert_eq!(geometry.segments[0].1, [5302.5, 41218.0, 10201.0]);
}

#[test]
fn coincident_linear_hit_returns_before_speed_numeric_operations() {
    let mut request = input();
    request.gravity.xyz = [0.0; 3];
    let mut geometry = Geometry {
        hit: Some(TrajectorySweepHit {
            position: request.position.xyz,
            normal: [0.0, 1.0, 0.0],
            surface_id: 19,
        }),
        ..Geometry::default()
    };
    let mut numeric = Numeric {
        fail_at: Some(5),
        ..Numeric::default()
    };
    let result = run_ballistic(request, &mut geometry, &mut numeric).unwrap();
    assert_eq!(result.time, [0.0; 4]);
    assert_eq!(result.frame, 0);
    assert_eq!(numeric.calls.len(), 4);
    assert_eq!(numeric.calls[1], ("reciprocal", 2.0));
    assert_eq!(numeric.calls[3], ("rsqrt", 0.0));
    assert_eq!(geometry.triangles, 1);
}

#[test]
fn negative_collision_time_does_not_erase_the_native_hit_payload() {
    let mut geometry = Geometry {
        hit: Some(TrajectorySweepHit {
            position: [f32::MAX, 9.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            surface_id: 71,
        }),
        ..Geometry::default()
    };
    let result = run_ballistic(input(), &mut geometry, &mut HostReckoningNumeric).unwrap();
    // First distance overflows to infinity > initial MAX; source backs up one frame.
    assert!(result.time[0] < 0.0);
    assert_eq!(result.position.xyz, [f32::MAX, 9.0, 0.0]);
    assert_eq!(result.normal.xyz, [1.0, 0.0, 0.0]);
    assert_eq!(result.surface, 71);
    assert_eq!(result.position.w, None);
    assert!(!result.valid());
}

#[test]
fn trajectory_acquisition_retains_requested_children_and_default_order() {
    use crate::{
        simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig,
        skateboard_assembly::SkateboardSimulationBackend,
    };
    let mut backend = SkateboardSimulationBackend::try_new(4, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let dmos = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    for island in [1, 3] {
        let mut queries = SimulationWorldLineQueries::try_new(
            &manager,
            &backend,
            &dmos,
            island,
            SimulationWorldLineQueryConfig {
                vehicle_update_in_parallel: false,
                ground: config(1),
                dmo: config(1),
                vehicle: config(1),
            },
        )
        .unwrap();
        assert!(matches!(
            queries.try_acquire_trajectory_with_flags(&dmos, "zero", 0),
            Err(SimulationWorldLineLeaseError::ZeroFlags)
        ));
        for flags in [1, 2, 4, 7, 8] {
            let batch = queries
                .try_acquire_trajectory_with_flags(&dmos, "flags", flags)
                .unwrap();
            let actual: Vec<_> = batch.0.children.iter().map(|entry| entry.child).collect();
            use SimulationWorldLineChild::{Dmo, Ground, Vehicle};
            let expected = match (island, flags) {
                (_, 1) => vec![Ground],
                (_, 2) => vec![Dmo],
                (1, 4) | (_, 8) => vec![],
                (3, 4) => vec![Vehicle],
                (1, 7) => vec![Ground, Dmo],
                (3, 7) => vec![Ground, Dmo, Vehicle],
                _ => unreachable!("explicit test case"),
            };
            assert_eq!(actual, expected);
            queries.try_release_trajectory(&batch).unwrap();
        }
        let batch = queries.try_acquire_trajectory(&dmos, "default").unwrap();
        assert_eq!(batch.0.children.len(), if island == 3 { 3 } else { 2 });
        queries.try_release_trajectory(&batch).unwrap();
    }
}
