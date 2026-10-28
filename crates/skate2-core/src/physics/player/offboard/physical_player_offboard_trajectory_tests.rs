#![expect(clippy::float_cmp, reason = "source stores and exact fixture values")]
use super::*;
use crate::{
    physical_player_trajectory_owners::TrajectoryOwnerBindings,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Mutex, Weak};

fn owner() -> OffboardTrajectoryOwner {
    let vault = Arc::new(crate::attribute_runtime::tests::graph(
        0xcdd9_26b3_5ef0_f6c4,
        vec![],
    ));
    OffboardTrajectoryOwner::new(
        TrajectoryOwnerBindings {
            owner: Weak::new(),
            processed: Arc::new(Mutex::new(SkeletonProcessedContact::new())),
        },
        &vault,
    )
}
#[derive(Default)]
struct Source {
    events: Vec<&'static str>,
    fail: bool,
    draw: bool,
}
impl Services<HostReckoningNumeric> for Source {
    type Error = &'static str;
    fn maximum_time(&mut self) -> Result<f32, Self::Error> {
        self.events.push("duration");
        Ok(2.5)
    }
    fn maximum_error_end(&mut self) -> Result<f32, Self::Error> {
        Ok(0.1)
    }
    fn maximum_error_start(&mut self) -> Result<f32, Self::Error> {
        Ok(0.2)
    }
    fn force_failure(&mut self) -> Result<bool, Self::Error> {
        self.events.push("failure");
        Ok(self.fail)
    }
    fn debug_draw_enabled(&mut self) -> Result<bool, Self::Error> {
        self.events.push("draw?");
        Ok(self.draw)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.events.push("overlay");
        Ok(())
    }
    fn processed_up_vector(&mut self) -> Result<Vector, Self::Error> {
        self.events.push("up");
        Ok(Vector::known([1.0, 0.0, 0.0, 7.0]))
    }
    fn processed_dt(&mut self) -> Result<f32, Self::Error> {
        self.events.push("dt");
        Ok(0.125)
    }
    fn world_gravity(&mut self) -> Vector {
        Vector::known([0.0, -10.0, 0.0, 0.0])
    }
}

#[test]
fn fan_uses_requested_sixteen_ignores_cones_and_preserves_unknown_w() {
    let mut s = owner();
    s.launch.num_trajectories = 16;
    s.launch.cone_angle_x = f32::NAN;
    s.launch.cone_angle_z = f32::INFINITY;
    s.launch.start_velocity = Vector {
        xyz: [2.0, 3.0, 4.0],
        w: None,
    };
    s.launch.start_com_position = Vector::known([5.0, 6.0, 7.0, 8.0]);
    let mut source = Source::default();
    s.initialize::<HostReckoningNumeric, _>(&mut source)
        .unwrap();
    assert_eq!(s.num_trajectories, 16);
    assert_eq!(source.events, vec!["duration"; 16]);
    for data in &s.data {
        assert_eq!(data.trajectory.velocity.xyz, [2.0, 3.0, 4.0]);
        assert_eq!(data.trajectory.velocity.w, None);
        // Shift by zero still evaluates arithmetic, including unknown W.
        assert_eq!(data.trajectory.position.w, None);
        assert_eq!(data.trajectory.time, [2.5; 4]);
    }
    s.launch.num_trajectories = 17;
    assert!(matches!(
        s.initialize::<HostReckoningNumeric, _>(&mut source),
        Err(Error::CandidateCapacity)
    ));
    assert_eq!(s.num_trajectories, 17);
}

#[test]
fn pending_frame_uses_raw_trajectory_and_does_not_publish_readiness() {
    let mut s = owner();
    s.just_started_query = true;
    s.raw_trajectory = RetainedTrajectory {
        position: Vector::known([1.0, 2.0, 3.0, 4.0]),
        velocity: Vector {
            xyz: [60.0, 0.0, 0.0],
            w: None,
        },
        gravity: ZERO,
        time: [2.0; 4],
    };
    let mut source = Source::default();
    let frame = s
        .frame(3, 0.0, &mut source, &mut HostReckoningNumeric)
        .unwrap();
    // Three rounded 1/60 steps are not exact decimal 0.05 in binary32.
    assert_eq!(
        frame.com_position.xyz,
        [60.0_f32.mul_add(3.0 * DT, 1.0), 2.0, 3.0]
    );
    assert_eq!(frame.com_position.w, None);
    assert!(!frame.landing_valid && s.just_started_query && !s.ready);
    assert_eq!(frame.landing_normal, UP);
    assert_eq!(s.current_index, 3);
    assert!(source.events.is_empty());
}

#[test]
fn scoring_uses_fractional_position_but_integer_frame_velocity_and_keeps_slot_zero() {
    let mut s = owner();
    s.num_trajectories = 2;
    s.just_started_query = true;
    for i in 0..2 {
        s.queries[i].ready = true;
        s.queries[i].result.time = [0.125; 4];
        s.queries[i].result.frame = 30;
        s.queries[i].result.surface = 0x300;
        s.queries[i].result.ground_normal = UP;
        s.queries[i].result.position = Vector::known([999.0; 4]);
        s.queries[i].result.trajectory = RetainedTrajectory {
            position: Vector::known([0.0, 10.0, 0.0, 0.0]),
            velocity: ZERO,
            gravity: Vector::known([0.0, -8.0, 0.0, 0.0]),
            time: [2.0; 4],
        };
        s.data[i].trajectory = s.queries[i].result.trajectory;
        s.data[i].index_shift = 2;
    }
    let mut source = Source {
        draw: true,
        ..Source::default()
    };
    assert!(s.update::<HostReckoningNumeric, _>(&mut source).unwrap());
    assert_eq!(s.best_trajectory, 0);
    assert_eq!(s.winning_result, Some(0));
    assert_eq!(s.scores[..2], [1.0, 0.0]);
    assert_eq!(s.winning_data.landing_position.xyz[1], 9.9375);
    assert_eq!(s.winning_data.landing_velocity.xyz[1], -4.0);
    assert_eq!(s.winning_data.landing_normal, UP);
    assert_eq!(s.queries[0].result.normal.xyz, [1.0, 0.0, 0.0]);
    assert_eq!(s.land_index, 32);
    assert_eq!(
        source.events,
        [
            "failure", "up", "failure", "up", "draw?", "draw?", "overlay", "draw?", "overlay"
        ]
    );
}

#[test]
fn miss_preserves_landing_position_and_short_circuits_debug_failure_read() {
    let mut s = owner();
    s.num_trajectories = 1;
    s.just_started_query = true;
    s.queries[0].ready = true;
    s.queries[0].result.time = [1.0, 1.0, 1.0, -1.0];
    s.data[0].landing_position = Vector::known([91.0; 4]);
    let mut source = Source::default();
    s.update::<HostReckoningNumeric, _>(&mut source).unwrap();
    assert_eq!(s.winning_data.landing_position, Vector::known([91.0; 4]));
    assert_eq!(s.winning_data.collision_index, 120);
    assert!(!s.winning_data.collision_found);
    assert_eq!(source.events, ["draw?", "draw?"]);
}

#[test]
fn scoring_assert_after_first_ready_retains_source_prefix() {
    let mut s = owner();
    s.num_trajectories = 2;
    s.just_started_query = true;
    s.queries[0].ready = true;
    s.queries[0].result.time = [-1.0; 4];
    s.scores[1] = 100.0;
    let mut source = Source::default();
    let result: Result<bool, Error<&str, ReckoningHostNumericError>> =
        s.update::<HostReckoningNumeric, _>(&mut source);
    assert!(matches!(result, Err(Error::FutureNotReady)));
    assert!(!s.just_started_query && !s.ready);
    assert_eq!(s.data[0].collision_index, 120);
    assert_eq!(s.scores[1], 0.0);
    assert_eq!(s.winning_result, None);
}

#[test]
fn completed_frame_blends_only_position_and_calls_shared_plane_leaf() {
    let mut s = owner();
    s.best_trajectory = 0;
    s.raw_trajectory.position = Vector::known([1.0, 10.0, 3.0, 4.0]);
    s.winning_com.position = Vector::known([7.0, 8.0, 9.0, 10.0]);
    s.winning_com.velocity = Vector::known([0.0, -1.0, 0.0, 0.0]);
    s.winning_data.collision_found = true;
    let mut source = Source::default();
    // Zero DefaultDataArea graph gives blend0, so COM position remains raw;
    // COM velocity is already winning, not blended.
    let output = s
        .frame(0, -3.0, &mut source, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(output.com_position, s.raw_trajectory.position);
    assert_eq!(output.com_velocity, s.winning_com.velocity);
    assert!((output.time_to_land - (61.0_f32.sqrt() - 1.0) / 10.0).abs() < 0.000_001);
    assert!(output.landing_valid);
    assert_eq!(source.events, ["dt"]);
    assert_eq!(s.trajectory_blend_time, 0.125);
}

#[test]
fn plane_single_root_requires_positive_but_two_roots_can_both_be_negative() {
    let mut t = RetainedTrajectory {
        position: ZERO,
        velocity: Vector::known([0.0, 2.0, 0.0, 0.0]),
        gravity: Vector::known([0.0, -2.0, 0.0, 0.0]),
        time: [-1.0; 4],
    };
    let mut time = [17.0; 4];
    assert!(greatest_plane_time(&t, UP, UP, &mut time, &mut HostReckoningNumeric).unwrap());
    assert_eq!(time, [1.0; 4]);
    t.velocity.xyz[1] = -2.0;
    assert!(!greatest_plane_time(&t, UP, UP, &mut time, &mut HostReckoningNumeric).unwrap());
    assert_eq!(time, [-1.0; 4]);
    assert!(
        !greatest_plane_time(
            &t,
            Vector::known([0.0, 2.0, 0.0, 0.0]),
            UP,
            &mut time,
            &mut HostReckoningNumeric
        )
        .unwrap()
    );
    assert_eq!(time, [-1.0; 4]); // no-root leaves output untouched
    let point = Vector::known([0.0, 0.75, 0.0, 0.0]);
    assert!(greatest_plane_time(&t, point, UP, &mut time, &mut HostReckoningNumeric).unwrap());
    assert_eq!(time, [-0.5; 4]);
    t.gravity = ZERO;
    // No fabricated linear solution in native: exceptional quadratic path is retained.
    assert!(greatest_plane_time(&t, point, UP, &mut time, &mut HostReckoningNumeric).unwrap());
    assert!(time[0].is_nan());
}

#[test]
fn debug_child_reloads_flag_and_requires_overlay_before_time_assertion() {
    let s = owner();
    let mut result = s.queries[0].result;
    result.time = [100.0; 4];
    let mut source = Source {
        draw: true,
        ..Source::default()
    };
    assert!(matches!(
        debug_draw_trajectory::<HostReckoningNumeric, _>(&result, &s.winning_com, 17, &mut source),
        Err(Error::Selector(
            super::super::selector_update::SelectorError::CollisionTime(_)
        ))
    ));
    assert_eq!(source.events, ["draw?", "overlay"]);
    source.events.clear();
    result.time[2] = -1.0;
    debug_draw_trajectory::<HostReckoningNumeric, _>(&result, &s.winning_com, 17, &mut source)
        .unwrap();
    assert_eq!(source.events, ["draw?"]);
    source.events.clear();
    source.draw = false;
    result.time = [100.0; 4];
    debug_draw_trajectory::<HostReckoningNumeric, _>(&result, &s.winning_com, 17, &mut source)
        .unwrap();
    assert_eq!(source.events, ["draw?"]);
}
