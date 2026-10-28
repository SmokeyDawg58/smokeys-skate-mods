use super::*;
struct Source {
    flags: [bool; 4],
    values: [f32; 7],
    tuning: [f32; 6],
    graph: PointGraph<8>,
    cvar: f32,
    log: Vec<String>,
    fail: Option<String>,
    lose_trajectory: bool,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            flags: [false; 4],
            values: [1.0, 0.0, -1.0, 0.2, 0.2, 0.2, 0.5],
            tuning: [0.5, 0.5, -10.0, 1.0, 2.0, 0.2],
            graph: PointGraph {
                x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
                y: [2.0; 8],
            },
            cvar: 1.2,
            log: Vec::new(),
            fail: None,
            lose_trajectory: false,
        }
    }
}
impl Source {
    fn event(&mut self, name: String) -> Result<(), String> {
        self.log.push(name.clone());
        if self.fail.as_ref() == Some(&name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl PrepareLandSource for Source {
    type Error = String;
    fn resolve_physical(&mut self) -> Result<bool, String> {
        self.event("resolve".into())?;
        Ok(true)
    }
    fn flag(&mut self, field: Flag) -> Result<bool, String> {
        self.event(format!("flag {field:?}"))?;
        let result = self.flags[field as usize];
        if field == Flag::AirTrajectoryValid && self.lose_trajectory {
            self.flags[field as usize] = false;
        }
        Ok(result)
    }
    fn value(&mut self, field: Value) -> Result<f32, String> {
        self.event(format!("value {field:?}"))?;
        Ok(self.values[field as usize])
    }
    fn tuning(&mut self, field: Tuning) -> Result<f32, String> {
        self.event(format!("tuning {field:?}"))?;
        Ok(self.tuning[field as usize])
    }
    fn landing_graph(&mut self) -> Result<PointGraph<8>, String> {
        self.event("graph".into())?;
        Ok(PointGraph {
            x: self.graph.x,
            y: self.graph.y,
        })
    }
    fn offboard_preland_time(&mut self) -> Result<f32, String> {
        self.event("cvar".into())?;
        Ok(self.cvar)
    }
}
#[test]
fn wallride_override_stops_before_globals_or_second_interface_resolution() {
    let mut s = Source::default();
    s.flags[Flag::TargetingWallRide as usize] = true;
    assert!(!time_to_prepare_land(&mut s).unwrap());
    assert_eq!(s.log, ["resolve", "flag TargetingWallRide"]);
}
#[test]
fn tilt_override_uses_ordered_gates_and_bitwise_absolute_value() {
    let mut s = Source::default();
    s.values[Value::LandingNormalY as usize] = f32::NAN;
    s.values[Value::LastTiltX as usize] = -2.0;
    assert!(has_tilt_too_large(&mut s).unwrap());
    s.values[Value::LastTiltX as usize] = f32::NAN;
    s.log.clear();
    assert!(!has_tilt_too_large(&mut s).unwrap());
    assert_eq!(s.log.last().unwrap(), "tuning TiltXLimit");
}
#[test]
fn non_descending_and_unordered_velocity_stop_before_inair_scalars() {
    for velocity in [0.0, -0.0, 1.0, f32::NAN] {
        let mut s = Source::default();
        s.values[Value::VelocityY as usize] = velocity;
        assert!(!time_to_prepare_land(&mut s).unwrap());
        assert_eq!(s.log.last().unwrap(), "value VelocityY");
        assert!(!s.log.iter().any(|v| v == "tuning FinalDistance"));
    }
}
#[test]
fn skateboard_landing_takes_priority_and_preserves_threshold_nan() {
    let mut s = Source::default();
    s.flags[Flag::CanLandOnBoard as usize] = true;
    s.flags[Flag::AirTrajectoryValid as usize] = true;
    assert!(time_to_prepare_land(&mut s).unwrap());
    assert_eq!(
        &s.log[s.log.len() - 3..],
        [
            "flag CanLandOnBoard",
            "tuning LandingOnDeckDistance",
            "value OffboardTime"
        ]
    );
    s.tuning[Tuning::DistanceVelocity as usize] = f32::NAN;
    assert!(!time_to_prepare_land(&mut s).unwrap());
}
#[test]
fn trajectory_flag_is_resampled_and_normal_nan_selects_upper_graph_input() {
    let mut s = Source::default();
    s.flags[Flag::AirTrajectoryValid as usize] = true;
    s.values[Value::TrajectoryNormalY as usize] = f32::NAN;
    s.graph.y = [0.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
    assert!(time_to_prepare_land(&mut s).unwrap()); // NaN clamp input=1 -> threshold .8
    s.flags[Flag::AirTrajectoryValid as usize] = true;
    s.lose_trajectory = true;
    s.log.clear();
    assert!(time_to_prepare_land(&mut s).unwrap());
    assert!(!s.log.iter().any(|v| v == "graph"));
    assert_eq!(
        s.log
            .iter()
            .filter(|v| *v == "flag AirTrajectoryValid")
            .count(),
        2
    );
}
#[test]
fn offboard_cvar_is_last_read_and_late_failure_is_not_hidden() {
    let mut s = Source::default();
    assert!(!time_to_prepare_land(&mut s).unwrap());
    assert_eq!(s.log.last().unwrap(), "flag OffboardTrajectoryReady");
    s.flags[Flag::OffboardTrajectoryReady as usize] = true;
    s.fail = Some("cvar".into());
    assert_eq!(time_to_prepare_land(&mut s), Err("cvar".into()));
    assert_eq!(&s.log[s.log.len() - 2..], ["value OffboardTime", "cvar"]);
}
