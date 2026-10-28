use super::*;
use std::cell::RefCell;

#[derive(Default)]
struct RecordingCvars(RefCell<Vec<C>>);

impl FramingCvarSource for RecordingCvars {
    fn read_framing_boolean(&self, kind: C) -> Result<bool, InputCvarError> {
        self.0.borrow_mut().push(kind);
        Ok(!matches!(kind, C::PitchSpeed | C::YawSpeed))
    }
}

#[test]
fn ordered_umbra_gates_return_zero_for_unordered_value() {
    let mut numeric = HostReckoningNumeric;
    assert_eq!(
        evaluate_umbra(f32::NAN, 0.1, 0.3, &mut numeric).unwrap(),
        0.0
    );
    assert_eq!(evaluate_umbra(0.0, 0.1, 0.3, &mut numeric).unwrap(), 1.0);
    assert_eq!(evaluate_umbra(0.3, 0.1, 0.3, &mut numeric).unwrap(), 0.0);
    let between = evaluate_umbra(0.2, 0.1, 0.3, &mut numeric).unwrap();
    assert!(between > 0.0 && between < 1.0);
}

#[test]
fn shot_editor_uses_target_angles_independently_of_retry_state() {
    let target = (0.2, 0.4);
    let tracked = (0.7, 0.9);
    assert_eq!(
        select_matrix_angles(true, target.0, target.1, tracked.0, tracked.1),
        target
    );
    assert_eq!(
        select_matrix_angles(false, target.0, target.1, tracked.0, tracked.1),
        tracked
    );
}

#[test]
fn roll_rotates_humanized_columns_and_preserves_computed_w() {
    let basis = framing_basis(0.0, 0.0, &mut HostReckoningSinCosNumeric).unwrap();
    let columns = rotate_basis_about_z(basis, 0.5);
    for column in &columns {
        assert_eq!(column.w.unwrap().to_bits(), column.xyz[0].to_bits());
    }
    let (sine, cosine) = 0.5_f32.sin_cos();
    assert!((columns[0].xyz[0] - cosine).abs() < 0.0001);
    assert!((columns[0].xyz[1] - sine).abs() < 0.0001);
    assert!((columns[1].xyz[0] + sine).abs() < 0.0001);
    assert!((columns[1].xyz[1] - cosine).abs() < 0.0001);
    assert!((columns[2].xyz[2] - 1.0).abs() < 0.0001);
}

#[test]
fn live_tracker_cvar_reads_follow_source_order() {
    let mut pitch_parameters = CameraParameters::default();
    pitch_parameters.tilt_acceleration_clamp_min = Some(2.0);
    pitch_parameters.tilt_acceleration_clamp_max = Some(3.0);
    pitch_parameters.tilt_smoothing_min = Some(0.1);
    pitch_parameters.tilt_delta_umbra = Some(5.0);
    let pitch_reads = RecordingCvars::default();
    let pitch_error = update_pitch_tracker(
        &mut CameraTracker::constructed(0.0, 0.0),
        0.016,
        0.2,
        0.3,
        &pitch_parameters,
        &pitch_reads,
        &mut HostReckoningNumeric,
    )
    .unwrap_err();
    assert!(matches!(
        pitch_error,
        RigFramingError::MissingParameter("tilt_delta_penumbra")
    ));
    assert_eq!(
        *pitch_reads.0.borrow(),
        [
            C::PitchSpeed,
            C::PitchAcceleration,
            C::PitchAcceleration,
            C::PitchSmoothing,
            C::PitchSmoothing,
        ]
    );

    let mut yaw_parameters = CameraParameters::default();
    yaw_parameters.pan_acceleration_clamp_min = Some(2.0);
    yaw_parameters.pan_acceleration_clamp_max = Some(3.0);
    yaw_parameters.pan_smoothing_min = Some(0.1);
    yaw_parameters.pan_delta_umbra = Some(5.0);
    let yaw_reads = RecordingCvars::default();
    let yaw_error = update_yaw_tracker(
        &mut CameraTracker::constructed(0.0, 0.0),
        0.016,
        0.2,
        0.3,
        &yaw_parameters,
        &yaw_reads,
        &mut HostReckoningNumeric,
    )
    .unwrap_err();
    assert!(matches!(
        yaw_error,
        RigFramingError::MissingParameter("pan_delta_penumbra")
    ));
    assert_eq!(
        *yaw_reads.0.borrow(),
        [
            C::YawSpeed,
            C::YawAcceleration,
            C::YawAcceleration,
            C::YawSmoothing,
            C::YawSmoothing,
        ]
    );
}

#[test]
fn late_missing_pitch_parameter_preserves_prior_tracker_writes() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = crate::input_cvar_runtime::InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut p = CameraParameters::default();
    p.tilt_acceleration_clamp_min = Some(2.0);
    p.tilt_acceleration_clamp_max = Some(3.0);
    p.tilt_smoothing_min = Some(0.1);
    p.tilt_delta_umbra = Some(5.0);
    // `tilt_delta_penumbra` remains unproduced after preceding live writes.
    let mut tracker = CameraTracker::constructed(0.0, 0.0);
    let error = update_pitch_tracker(
        &mut tracker,
        0.016,
        0.2,
        0.3,
        &p,
        &refs,
        &mut HostReckoningNumeric,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        RigFramingError::MissingParameter("tilt_delta_penumbra")
    ));
    assert_eq!(tracker.parameters.speed_clamp, Some(f32::MAX));
    assert_eq!(tracker.parameters.acceleration_min, Some(2.0 * RADIANS));
    assert_eq!(tracker.parameters.acceleration_max, Some(3.0 * RADIANS));
    assert_eq!(tracker.parameters.smoothing_min, Some(0.1));
    assert_eq!(tracker.parameters.smoothing_max, Some(0.3));
    assert_eq!(tracker.parameters.umbra, Some(5.0 * RADIANS));
    assert_eq!(tracker.parameters.penumbra, None);
    assert_eq!(tracker.output_position, 0.0);
}
