use super::*;

#[derive(Default)]
struct Calls {
    events: Vec<&'static str>,
    dot_input: Option<[f32; 4]>,
}
impl ReckoningNumeric for Calls {
    type Error = ();
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, ()> {
        self.events.push("dot3");
        self.dot_input = Some(left);
        assert_eq!(left, right);
        Ok(left[0].mul_add(left[0], left[1].mul_add(left[1], left[2] * left[2])))
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, ()> {
        self.events.push("rsqrt");
        Ok(value.sqrt().recip())
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, ()> {
        unreachable!("length has no reciprocal estimate")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, ()> {
        unreachable!("length has no integer conversion")
    }
}

#[test]
fn speed_is_horizontal_subject_velocity_and_issues_zero_estimate() {
    let mut numeric = Calls::default();
    let velocity = Vector::known([-3.0, 40.0, 4.0, f32::NAN]);
    let speed = subject_speed(velocity, &mut numeric).unwrap();
    assert!((speed - 5.0).abs() < 0.001);
    assert_eq!(numeric.dot_input, Some([-3.0, 0.0, 4.0, 0.0]));
    assert_eq!(numeric.events, ["dot3", "rsqrt"]);

    let mut zero_calls = Calls::default();
    let zero = subject_speed(Vector::known([0.0, 100.0, 0.0, 0.0]), &mut zero_calls).unwrap();
    assert_eq!(zero.to_bits(), 0);
    assert_eq!(zero_calls.events, ["dot3", "rsqrt"]);
}

#[test]
fn distance_uses_anchor_translation_minus_camera_and_ignores_w_in_dot() {
    let anchor = Matrix::from_columns([
        Vector::known([1.0, 0.0, 0.0, 0.0]),
        Vector::known([0.0, 1.0, 0.0, 0.0]),
        Vector::known([0.0, 0.0, 1.0, 0.0]),
        Vector::known([5.0, 9.0, 12.0, 20.0]),
    ]);
    let camera = Vector::known([2.0, 5.0, 0.0, -80.0]);
    let mut numeric = Calls::default();
    let distance = subject_distance(&anchor, camera, &mut numeric).unwrap();
    assert!((distance - 13.0).abs() < 0.001);
    assert_eq!(numeric.dot_input, Some([3.0, 4.0, 12.0, 100.0]));
    assert_eq!(numeric.events, ["dot3", "rsqrt"]);
}

#[test]
fn wipeout_suppresses_shake_only_when_not_colliding() {
    assert!(shake_active(false, false));
    assert!(shake_active(false, true));
    assert!(!shake_active(true, false));
    assert!(shake_active(true, true));
}
