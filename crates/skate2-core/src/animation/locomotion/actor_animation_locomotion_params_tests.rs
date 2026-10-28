use super::*;
use crate::{
    input_cvar_runtime::InputCvarManager,
    reckoning_numeric::{HostReckoningNumeric, host_dot3},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
};
#[derive(Default)]
struct Numeric {
    calls: Vec<&'static str>,
    fail: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls.push("dot");
        if self.fail {
            Err("dot")
        } else {
            Ok(host_dot3(a, b))
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.calls.push("sqrt");
        Ok(1.0 / v.sqrt())
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.calls.push("reciprocal");
        Ok(1.0 / v)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("unexpected scalar conversion")
    }
}
fn vector(x: f32, y: f32) -> V {
    V {
        xyz: [x, y, 0.0],
        w: None,
    }
}
#[test]
fn contact_and_pass_keep_distinct_numeric_issue_order_and_unknown_w() {
    let mut trajectory = AnimationLocomotionTrajectory::new();
    for s in &mut trajectory.input_feet[0].states {
        s.time = 1.0;
    }
    trajectory.input_feet[0].states[0].vectors[2] = vector(2.0, 0.0);
    trajectory.input_feet[0].states[1].vectors[0] = vector(2.0, 0.0);
    trajectory.input_feet[0].states[1].vectors[2] = vector(2.0, 0.0);
    trajectory.physical_feet[0].states[2].vectors[0] = vector(4.0, 0.0);
    trajectory.physical_feet[0].states[2].time = 1.0;
    let mut numeric = Numeric::default();
    trajectory
        .compute_contact_to_pass::<_, ()>(0, 0, &mut numeric)
        .unwrap();
    assert_eq!(numeric.calls, ["dot", "dot", "sqrt", "sqrt"]);
    assert_eq!(trajectory.physical_feet[0].states[0].time, 1.0);
    assert_eq!(trajectory.physical_feet[0].states[0].vectors[2].w, None);
    numeric.calls.clear();
    trajectory
        .compute_pass_to_air::<_, ()>(0, 0, &mut numeric)
        .unwrap();
    assert_eq!(numeric.calls, ["dot", "sqrt", "dot", "sqrt"]);
    assert_eq!(trajectory.physical_feet[0].states[1].time, 1.0);
    assert_eq!(trajectory.physical_feet[0].states[1].vectors[2].w, None);
}
#[test]
fn unordered_input_duration_fails_after_vector_prefix_without_time_write() {
    let mut trajectory = AnimationLocomotionTrajectory::new();
    trajectory.input_feet[0].states[0].time = f32::NAN;
    trajectory.input_feet[0].states[0].vectors[0] = vector(3.0, 4.0);
    trajectory.physical_feet[0].states[0].time = 9.0;
    let mut numeric = Numeric::default();
    assert!(matches!(
        trajectory.compute_contact_to_pass::<_, ()>(0, 0, &mut numeric),
        Err(LocomotionParamsError::Assertion("contact duration"))
    ));
    assert_eq!(
        trajectory.physical_feet[0].states[0].vectors[0],
        vector(3.0, 4.0)
    );
    assert_eq!(trajectory.physical_feet[0].states[0].time, 9.0);
    assert!(numeric.calls.is_empty());
}
#[test]
fn air_keeps_world_prefix_on_numeric_error_then_publishes_relative_ballistic_state() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut t = AnimationLocomotionTrajectory::new();
    t.input_feet[0].initialized = true;
    t.input_feet[1].initialized = true;
    t.input_feet[0].states[1].vectors[0] = vector(0.2, 0.0);
    t.input_feet[0].states[1].vectors[2] = vector(0.2, 0.0);
    t.input_feet[0].states[1].vectors[3] = vector(1.0, 0.0);
    t.input_feet[0].states[2].vectors[0] = vector(0.4, 0.0);
    t.input_feet[0].states[2].vectors[1] = vector(1.0, 0.0);
    t.input_feet[0].states[2].time = 0.5;
    t.input_feet[1].data.position = vector(1.0, 0.0);
    t.physical_feet[0].states[2].debug_flags = 0x87;
    let start = vector(10.0, 1.0);
    let end = vector(11.0, 1.0);
    let mut numeric = Numeric {
        fail: true,
        ..Numeric::default()
    };
    assert!(matches!(
        t.compute_air(
            0,
            0,
            start,
            end,
            &refs,
            &mut numeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(LocomotionParamsError::Numeric("dot"))
    ));
    assert_eq!(t.physical_feet[0].states[2].vectors[2], end);
    assert_eq!(
        t.physical_feet[0].states[2].vectors[0].xyz,
        [10.4, 1.0, 0.0]
    );
    assert_eq!(t.physical_feet[0].states[2].debug_flags, 0x87);
    assert!(!t.physical_feet[0].initialized);
    t.compute_air(
        0,
        0,
        start,
        end,
        &refs,
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap();
    let air = t.physical_feet[0].states[2];
    assert!(t.physical_feet[0].initialized);
    assert_eq!(air.vectors[1].w, None);
    assert_eq!(air.debug_flags, 0x80);
    assert!((air.time - 0.6).abs() < 1e-5);
    for axis in 0..3 {
        let g = if axis == 1 {
            f32::from_bits(0xc09cf5c3)
        } else {
            0.0
        };
        let displacement = (g * air.time).mul_add(air.time, air.vectors[1].xyz[axis] * air.time);
        assert!((displacement - air.vectors[2].xyz[axis]).abs() < 1e-5);
    }
}
