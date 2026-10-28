#![expect(clippy::float_cmp, reason = "exact source stores and fixture values")]
use super::*;
use crate::{
    physical_player_trajectory_owners::TrajectoryOwnerBindings,
    reckoning_numeric::{HostReckoningNumeric, host_dot3},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::Weak;

fn owner() -> OnboardTrajectoryOwner {
    let mut owner = OnboardTrajectoryOwner::new(
        TrajectoryOwnerBindings {
            owner: Weak::new(),
            processed: Arc::new(Mutex::new(SkeletonProcessedContact::new())),
        },
        false,
    );
    owner.launch.start_velocity = Vector {
        xyz: [3.0, 0.0, 4.0],
        w: None,
    };
    owner.launch.velocity_before_jump = Vector::known([0.0, 12.0, 0.0, 29.0]);
    owner.launch.num_trajectories = 1;
    owner.launch.cone_angle_x = 20.0;
    owner.launch.cone_angle_z = 30.0;
    owner.launch.com_position = Vector::known([4.0, 10.0, 6.0, 3.0]);
    owner.launch.board_position = Vector::known([2.0, 1.0, 3.0, 2.0]);
    owner.launch.reckoning_orientation_inverse = SkeletonRootMatrix::from_columns([
        Vector::known([1.0, 0.0, 0.0, 0.0]),
        Vector::known([0.0, 1.0, 0.0, 0.0]),
        Vector::known([0.0, 0.0, 1.0, 0.0]),
        Vector {
            xyz: [7.0, 8.0, 9.0],
            w: None,
        },
    ]);
    owner.targeting_wallride = true;
    owner.start_board_position = Vector::known([71.0; 4]);
    owner.start_trajectory_position = Vector::known([72.0; 4]);
    owner.initial_offset = Vector::known([73.0; 4]);
    owner
}
#[derive(Default)]
struct Source {
    calls: Vec<(&'static str, f32)>,
    fail: Option<&'static str>,
}
impl Source {
    fn call(&mut self, name: &'static str, input: f32) -> Result<(), &'static str> {
        self.calls.push((name, input));
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl FanAttributes for Source {
    type Error = &'static str;
    fn cone_z_graph(&mut self, input: f32) -> Result<f32, Self::Error> {
        self.call("fan_graph", input)?;
        Ok(1.0)
    }
    fn speed_bounds(&mut self) -> Result<(f32, f32), Self::Error> {
        self.call("bounds", 0.0)?;
        Ok((1.0, 20.0))
    }
}
impl PrepareSource for Source {
    fn ground_normal(&mut self) -> Result<Vector, Self::Error> {
        self.call("normal", 0.0)?;
        Ok(Vector::known([0.0, -0.75, 0.0, 0.0]))
    }
    fn velocity_y(&mut self) -> Result<f32, Self::Error> {
        self.call("velocity", 0.0)?;
        Ok(4.0)
    }
    fn ground_position(&mut self) -> Result<Vector, Self::Error> {
        self.call("ground", 0.0)?;
        Ok(Vector {
            xyz: [0.0; 3],
            w: None,
        })
    }
    fn reckoning_graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error> {
        match offset {
            0x260 => {
                self.call("slope", input)?;
                Ok(0.25)
            }
            0x220 => {
                self.call("speed", input)?;
                Ok(0.5)
            }
            _ => panic!("unexpected graph"),
        }
    }
    fn reckoning_scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        match key {
            RADIUS => {
                self.call("radius", 0.0)?;
                Ok(1.0)
            }
            START_OFFSET => {
                self.call("offset", 0.0)?;
                Ok(2.0)
            }
            _ => panic!("unexpected FETCH"),
        }
    }
}
fn run(
    owner: &mut OnboardTrajectoryOwner,
    source: &mut Source,
) -> Result<
    LaunchPreparation,
    PrepareError<
        &'static str,
        crate::reckoning_numeric::ReckoningHostNumericError,
        std::convert::Infallible,
    >,
> {
    prepare(
        owner,
        source,
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
}

#[test]
fn actual_retained_preparation_stops_before_max_time_and_leaves_futures_unlaunched() {
    let mut owner = owner();
    let mut source = Source::default();
    assert_eq!(
        run(&mut owner, &mut source).unwrap(),
        LaunchPreparation::BeforeCalcMaxTime
    );
    assert!(!owner.targeting_wallride);
    assert_eq!(owner.num_trajectories, 1);
    assert_eq!(owner.highest_wallride_position, -1_000_000.0);
    assert_eq!(owner.data[0].velocity, owner.launch.start_velocity);
    assert_eq!(owner.start_board_position, owner.launch.board_position);
    assert_eq!(
        owner.start_trajectory_position,
        Vector::known([4.0, 8.0, 6.0, 3.0])
    );
    assert_eq!(owner.initial_offset, Vector::known([0.0, 2.0, 0.0, 0.0]));
    assert_eq!(owner.board_position_local.xyz, [5.0, 1.0, 6.0]);
    assert_eq!(owner.com_position_local.xyz, [7.0, 10.0, 9.0]);
    assert_eq!(owner.board_position_local.w, None);
    assert_eq!(owner.com_position_local.w, None);
    assert!(!owner.ready && !owner.just_started_query);
    assert!(owner.queries.iter().all(|future| !future.ready));
    assert_eq!(
        source.calls,
        vec![
            ("fan_graph", 0.25),
            ("bounds", 0.0),
            ("normal", 0.0),
            ("slope", 0.75),
            ("velocity", 0.0),
            ("speed", 0.4),
            ("ground", 0.0),
            ("radius", 0.0),
            ("offset", 0.0),
        ]
    );
}

#[test]
fn override_still_runs_attributes_and_local_board_uses_original_launch_board() {
    let mut owner = owner();
    owner.launch.use_start_override = true;
    owner.launch.trajectory_start_override = [20.0, 30.0, 40.0, 9.0].into();
    owner.launch.board_position_override = [999.0; 4].into();
    let mut source = Source::default();
    run(&mut owner, &mut source).unwrap();
    assert_eq!(owner.start_board_position, Vector::known([999.0; 4]));
    assert_eq!(
        owner.start_trajectory_position,
        Vector::known([20.0, 30.0, 40.0, 9.0])
    );
    assert_eq!(owner.board_position_local.xyz, [-11.0, -21.0, -28.0]);
    assert_eq!(owner.com_position_local.xyz, [-9.0, -12.0, -25.0]);
    assert_eq!(
        owner.initial_offset,
        Vector::known([-16.0, -20.0, -34.0, -6.0])
    );
    assert_eq!(source.calls.last().unwrap().0, "offset");
}

#[test]
fn failed_fan_graph_retains_source_prefix_stores_and_never_copies_start_board() {
    let mut owner = owner();
    owner.launch.num_trajectories = 20;
    let mut source = Source {
        fail: Some("fan_graph"),
        ..Source::default()
    };
    assert!(matches!(
        run(&mut owner, &mut source),
        Err(PrepareError::Source("fan_graph"))
    ));
    assert!(!owner.targeting_wallride);
    assert_eq!(owner.num_trajectories, 7);
    assert_eq!(owner.highest_wallride_position, -1_000_000.0);
    assert_eq!(owner.data[0].velocity, owner.launch.start_velocity);
    assert_eq!(owner.start_board_position, Vector::known([71.0; 4]));
}

#[test]
fn late_attribute_failure_retains_start_board_but_not_start_trajectory_or_offsets() {
    let mut owner = owner();
    let mut source = Source {
        fail: Some("offset"),
        ..Source::default()
    };
    assert!(matches!(
        run(&mut owner, &mut source),
        Err(PrepareError::Source("offset"))
    ));
    assert_eq!(owner.start_board_position, owner.launch.board_position);
    assert_eq!(owner.start_trajectory_position, Vector::known([72.0; 4]));
    assert_eq!(owner.initial_offset, Vector::known([73.0; 4]));
}

#[test]
fn zero_count_still_executes_fan_prefix_and_two_sine_calls() {
    struct Rounding(usize);
    impl ReckoningSinCosNumeric for Rounding {
        type Error = &'static str;
        fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
            self.0 += 1;
            if self.0 == 2 {
                Err("second sine")
            } else {
                Ok(value.round_ties_even())
            }
        }
    }
    let mut owner = owner();
    owner.launch.num_trajectories = 0;

    let mut source = Source::default();
    let mut rounding = Rounding(0);
    assert!(matches!(
        prepare(
            &mut owner,
            &mut source,
            &mut HostReckoningNumeric,
            &mut rounding
        ),
        Err(PrepareError::Rounding("second sine"))
    ));
    assert_eq!(owner.num_trajectories, 0);
    assert_eq!(owner.data[0].velocity, owner.launch.start_velocity);
    assert_eq!(source.calls.len(), 2);
    assert_eq!(owner.start_board_position, Vector::known([71.0; 4]));
}

#[test]
fn retained_fan_caps_count_and_preserves_unknown_velocity_w() {
    let mut owner = owner();
    owner.launch.num_trajectories = u16::MAX;
    run(&mut owner, &mut Source::default()).unwrap();
    assert_eq!(owner.num_trajectories, 7);
    assert!(
        owner
            .data
            .iter()
            .all(|candidate| candidate.velocity.w.is_none())
    );
    assert_ne!(owner.data[1].velocity.xyz, owner.data[2].velocity.xyz);
}

#[test]
fn both_velocity_dots_precede_both_estimates_even_when_not_jumped() {
    #[derive(Default)]
    struct Numeric(Vec<&'static str>);
    impl ReckoningNumeric for Numeric {
        type Error = &'static str;
        fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
            self.0.push("dot");
            Ok(host_dot3(a, b))
        }
        fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
            self.0.push("rsqrt");
            Ok(1.0 / value.sqrt())
        }
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("unused")
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
            panic!("unused")
        }
    }
    let mut numeric = Numeric::default();
    let mut source = Source {
        fail: Some("fan_graph"),
        ..Source::default()
    };
    assert!(
        prepare(
            &mut owner(),
            &mut source,
            &mut numeric,
            &mut HostReckoningSinCosNumeric
        )
        .is_err()
    );
    assert_eq!(
        numeric.0,
        [
            "dot", "rsqrt", "dot", "rsqrt", "dot", "dot", "rsqrt", "rsqrt"
        ]
    );
}
