use super::*;
use crate::{camera_subject::ZERO, reckoning_numeric::HostReckoningNumeric};
struct Numeric {
    events: Vec<&'static str>,
    reciprocal_calls: usize,
    fail_second: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.events.push("dot");
        Ok(HostReckoningNumeric.dot3(a, b).unwrap())
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.events.push("rsqrt");
        Ok(HostReckoningNumeric.reciprocal_sqrt_estimate(x).unwrap())
    }
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.events.push("reciprocal");
        self.reciprocal_calls += 1;
        if self.fail_second && self.reciprocal_calls == 2 {
            return Err("second reciprocal");
        }
        Ok(HostReckoningNumeric.reciprocal_estimate(x).unwrap())
    }
    fn truncate_i32(&mut self, x: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric
            .truncate_i32(x)
            .map_err(|_| "conversion")
    }
}
fn numeric() -> Numeric {
    Numeric {
        events: Vec::new(),
        reciprocal_calls: 0,
        fail_second: false,
    }
}
fn tracker() -> CameraTracker<SkeletonRootVector> {
    let mut result = CameraTracker::constructed(ZERO, ZERO);
    result.parameters.overshoot_zeroes_velocity = false;
    result.parameters.acceleration_min = Some(1000.0);
    result.parameters.acceleration_max = Some(1000.0);
    result.parameters.speed_clamp = Some(1000.0);
    result.parameters.smoothing_min = Some(0.0);
    result.parameters.smoothing_max = Some(0.0);
    result
}
#[test]
fn two_reciprocals_precede_acceleration_and_unknown_w_stays_unknown() {
    let mut tracker = tracker();
    let mut numeric = numeric();
    let target = SkeletonRootVector {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    tracker
        .try_update_vector(0.5, target, &mut numeric)
        .unwrap();
    assert_eq!(
        &numeric.events[..4],
        &["dot", "rsqrt", "reciprocal", "reciprocal"]
    );
    assert_eq!(numeric.reciprocal_calls, 2);
    assert_eq!(tracker.output_position, target);
    assert!(tracker.acceleration.w.is_none());
    assert!(tracker.velocity.w.is_none());
}
#[test]
fn failure_of_second_reciprocal_keeps_input_without_acceleration_publication() {
    let mut tracker = tracker();
    let target = SkeletonRootVector::known([1.0; 4]);
    let mut numeric = numeric();
    numeric.fail_second = true;
    assert!(matches!(
        tracker.try_update_vector(0.5, target, &mut numeric),
        Err(VectorTrackerError::Numeric("second reciprocal"))
    ));
    assert_eq!(tracker.input_position, target);
    assert_eq!(tracker.acceleration, ZERO);
    assert_eq!(tracker.output_position, ZERO);
}
#[test]
fn magnitude_clamp_has_no_negative_assertion_or_nan_sanitization() {
    let value = SkeletonRootVector {
        xyz: [3.0, 0.0, 0.0],
        w: None,
    };
    let negative = clamp_magnitude(value, -1.0, &mut HostReckoningNumeric).unwrap();
    assert_eq!(negative.xyz[0].to_bits(), (-1.0_f32).to_bits());
    assert!(negative.w.is_none());
    assert_eq!(
        clamp_magnitude(value, f32::NAN, &mut HostReckoningNumeric).unwrap(),
        value
    );
    let zero = clamp_magnitude(ZERO, -1.0, &mut HostReckoningNumeric).unwrap();
    assert!(zero.xyz.iter().all(|v| v.is_nan()));
}
#[test]
fn speed_parameter_failure_keeps_unclamped_velocity() {
    let mut tracker = tracker();
    tracker.parameters.speed_clamp = None;
    let target = SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]);
    assert!(matches!(
        tracker.try_update_vector(0.5, target, &mut HostReckoningNumeric),
        Err(VectorTrackerError::Parameter(
            ScalarTrackerError::MissingParameter("speed_clamp")
        ))
    ));
    assert_eq!(tracker.velocity.xyz[0].to_bits(), 2.0_f32.to_bits());
    assert_eq!(tracker.output_position, ZERO);
}
