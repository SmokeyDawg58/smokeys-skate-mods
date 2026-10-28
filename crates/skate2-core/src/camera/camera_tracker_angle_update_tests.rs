use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

fn tuned(position: f32) -> CameraTracker<f32> {
    let mut tracker = CameraTracker::constructed(position, 0.0);
    tracker.parameters.acceleration_min = Some(1000.0);
    tracker.parameters.acceleration_max = Some(1000.0);
    tracker.parameters.speed_clamp = Some(1000.0);
    tracker.parameters.smoothing_min = Some(0.0);
    tracker.parameters.smoothing_max = Some(0.0);
    tracker
}

#[test]
fn shortest_angle_crosses_boundary_without_long_scalar_path() {
    let mut angle = tuned(3.0);
    let mut scalar = tuned(3.0);
    angle
        .try_update_angle(0.1, -3.0, &mut HostReckoningNumeric)
        .unwrap();
    scalar.try_update_scalar(0.1, -3.0).unwrap();
    assert!(angle.velocity > 0.0);
    assert!(scalar.velocity < 0.0);
    assert!((angle.output_position + 3.0).abs() < 0.00001);
    assert!(angle.parameters.umbra.is_none());
}

#[test]
fn initial_numeric_failure_keeps_only_input_store() {
    let mut tracker = tuned(1.0);
    tracker.velocity = 4.0;
    assert!(matches!(
        tracker.try_update_angle(0.1, f32::NAN, &mut HostReckoningNumeric),
        Err(AngleTrackerError::Numeric(_))
    ));
    assert!(tracker.input_position.is_nan());
    assert_eq!(tracker.velocity.to_bits(), 4.0_f32.to_bits());
    assert_eq!(tracker.output_position.to_bits(), 1.0_f32.to_bits());
}

#[test]
fn late_angle_failure_preserves_velocity_and_smoothing_but_not_position() {
    let mut tracker = tuned(0.0);
    tracker.parameters.umbra = Some(1.0);
    tracker.parameters.smoothing_min = Some(f32::NAN);
    tracker.parameters.smoothing_max = Some(f32::NAN);
    assert!(matches!(
        tracker.try_update_angle(0.1, 0.5, &mut HostReckoningNumeric),
        Err(AngleTrackerError::Numeric(_))
    ));
    assert!(tracker.velocity > 0.0);
    assert!(tracker.smoothing.is_nan());
    assert_eq!(tracker.output_position.to_bits(), 0.0_f32.to_bits());
}
