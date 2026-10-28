use super::*;

fn equal_ranges() -> CameraTracker<f32> {
    let mut tracker = CameraTracker::constructed(1.0, 0.0);
    tracker.parameters.acceleration_min = Some(500.0);
    tracker.parameters.acceleration_max = Some(500.0);
    tracker.parameters.speed_clamp = Some(100.0);
    tracker.parameters.smoothing_min = Some(f32::from_bits(0x3f59_999a));
    tracker.parameters.smoothing_max = tracker.parameters.smoothing_min;
    tracker
}

#[test]
fn equal_ranges_bypass_actual_constructor_unknown_thresholds() {
    let mut tracker = equal_ranges();
    tracker.try_update_scalar(0.25, -1.0).unwrap();
    assert_eq!(
        tracker.output_position.to_bits(),
        0.700_000_05_f32.to_bits()
    );
    assert_eq!(tracker.acceleration.to_bits(), (-32.0_f32).to_bits());
    assert!(tracker.parameters.umbra.is_none());
    assert!(tracker.parameters.penumbra.is_none());
}

#[test]
fn unknown_weight_only_stops_when_an_actual_range_uses_it() {
    let mut tracker = equal_ranges();
    tracker.parameters.acceleration_max = Some(600.0);
    assert_eq!(
        tracker.try_update_scalar(0.25, -1.0),
        Err(ScalarTrackerError::UnknownDistanceWeight)
    );
    assert_eq!(tracker.input_position.to_bits(), (-1.0_f32).to_bits());
    assert_eq!(tracker.acceleration.to_bits(), (-32.0_f32).to_bits());
    assert_eq!(tracker.output_position.to_bits(), 1.0_f32.to_bits());
}

#[test]
fn missing_speed_preserves_unclamped_velocity_publication() {
    let mut tracker = equal_ranges();
    tracker.parameters.speed_clamp = None;
    assert_eq!(
        tracker.try_update_scalar(0.25, -1.0),
        Err(ScalarTrackerError::MissingParameter("speed_clamp"))
    );
    assert_eq!(tracker.velocity.to_bits(), (-8.0_f32).to_bits());
    assert_eq!(tracker.output_position.to_bits(), 1.0_f32.to_bits());
}

#[test]
fn zero_dt_runs_native_division_instead_of_substituting_a_timestep() {
    let mut tracker = equal_ranges();
    tracker.try_update_scalar(0.0, 1.0).unwrap();
    assert!(tracker.acceleration.is_nan());
    assert!(tracker.velocity.is_nan());
    assert!(tracker.output_position.is_nan());
}

#[test]
fn source_ordered_thresholds_preserve_nan_zero_and_endpoint_selection() {
    assert_eq!(
        distance_weight(f32::NAN, None, None).map(f32::to_bits),
        Some(0.0_f32.to_bits())
    );
    assert_eq!(
        distance_weight(0.5, Some(1.0), None).map(f32::to_bits),
        Some(1.0_f32.to_bits())
    );
    assert_eq!(
        distance_weight(2.0, Some(1.0), Some(2.0)).map(f32::to_bits),
        Some(0.0_f32.to_bits())
    );
    assert_eq!(
        distance_weight(1.5, Some(1.0), Some(2.0)).map(f32::to_bits),
        Some(0.5_f32.to_bits())
    );
}
