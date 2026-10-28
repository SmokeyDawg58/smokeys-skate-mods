use super::*;
use crate::{camera_attributes::CameraAttributes, reckoning_numeric::HostReckoningNumeric};
use std::sync::{Arc, Mutex};

fn compass() -> CameraCompass {
    CameraCompass::constructed(Arc::new(Mutex::new(CameraAttributes::begin_init())))
}
struct Numeric {
    fail_second_turn: bool,
    turn_calls: usize,
    rsqrt_calls: usize,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(HostReckoningNumeric.dot3(a, b).unwrap())
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.rsqrt_calls += 1;
        Ok(HostReckoningNumeric.reciprocal_sqrt_estimate(x).unwrap())
    }
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        if x.to_bits() == TWO_PI.to_bits() {
            self.turn_calls += 1;
            if self.fail_second_turn && self.turn_calls == 2 {
                return Err("second angle normalization");
            }
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
        fail_second_turn: false,
        turn_calls: 0,
        rsqrt_calls: 0,
    }
}

#[test]
fn zero_dt_velocity_does_not_read_unpublished_parameters_or_change_last_position() {
    let mut owner = compass();
    owner.last_position = SkeletonRootVector::known([7.0; 4]);
    owner.subject_velocity = SkeletonRootVector::known([2.0; 4]);
    owner.update_velocity(-0.0, &mut numeric()).unwrap();
    assert_eq!(owner.subject_velocity, ZERO);
    assert_eq!(owner.deadzone_velocity, ZERO);
    assert_eq!(owner.last_position, SkeletonRootVector::known([7.0; 4]));
    assert!(matches!(
        owner.parameters.lock().unwrap().parameters,
        CameraParameterPublication::Unwritten
    ));
}

#[test]
fn missing_deadzone_parameter_retains_velocity_and_wipeout_prefix() {
    let mut owner = compass();
    owner.input.transform = SkeletonRootMatrix::from_columns([
        ZERO,
        ZERO,
        ZERO,
        SkeletonRootVector::known([2.0, 4.0, 6.0, 8.0]),
    ]);
    owner.wipeout_over = true;
    let result = owner.update_velocity(2.0, &mut numeric());
    assert!(matches!(
        result,
        Err(CameraCompassError::UnwrittenParameter(
            "compass_max_deadzone_speed"
        ))
    ));
    assert_eq!(
        owner.subject_velocity,
        SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0])
    );
    assert!(!owner.wipeout_over);
    assert_eq!(owner.last_position, ZERO);
}

#[test]
fn interpolation_failure_keeps_the_first_interpolated_store() {
    let mut owner = compass();
    owner.ground_velocity_heading = 1.0;
    owner.input.trajectory_fraction = 0.5;
    owner.input.trajectory_end = SkeletonRootVector::known([0.0, 0.0, 2.0, 0.0]);
    let mut numeric = numeric();
    numeric.fail_second_turn = true;
    assert!(matches!(
        owner.velocity_heading(1.0, &mut numeric),
        Err(CameraCompassError::Numeric("second angle normalization"))
    ));
    assert_eq!(owner.velocity_heading.to_bits(), 0.5_f32.to_bits());
    assert_eq!(numeric.turn_calls, 2);
}

#[test]
fn adaptive_deadzone_keeps_all_lanes_and_has_two_estimate_calls() {
    let mut owner = compass();
    owner.adaptive_size = 1.0;
    let mut numeric = numeric();
    owner
        .adaptive_update(
            2.0,
            SkeletonRootVector::known([3.0, 4.0, 6.0, 8.0]),
            ZERO,
            &mut numeric,
        )
        .unwrap();
    assert_eq!(numeric.rsqrt_calls, 2);
    let position = owner.adaptive_position.known_lanes().unwrap();
    for (actual, expected) in position.into_iter().zip([2.4, 3.2, 4.8, 6.4]) {
        assert!((actual - expected).abs() < 0.00001);
    }
    assert_eq!(
        owner.adaptive_velocity,
        owner.adaptive_position.map(|v| v * 0.5)
    );
    let old = owner.adaptive_position;
    owner.adaptive_size = f32::NAN;
    let zero = SkeletonRootVector::known([9.0; 4]);
    owner
        .adaptive_update(
            -0.0,
            SkeletonRootVector::known([20.0; 4]),
            zero,
            &mut numeric,
        )
        .unwrap();
    assert_eq!(owner.adaptive_position, old);
    assert_eq!(owner.adaptive_velocity, zero);
}

#[test]
fn teleport_update_completes_without_parameter_defaults() {
    let mut owner = compass();
    let mut subject = CameraSubject::try_construct().unwrap();
    subject.flags.teleported = true;
    subject.hips_look = SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]);
    let mut published = [99.0; 4];
    owner
        .try_update_subject(
            &mut subject,
            crate::camera_subject::identity(),
            &mut published,
            1.0,
            &mut numeric(),
        )
        .unwrap();
    assert!(owner.teleported_last_frame);
    assert_eq!(subject.compass_norths[2].to_bits(), 0);
    assert_eq!(subject.compass_norths[0].to_bits(), published[0].to_bits());
    assert!(matches!(
        owner.parameters.lock().unwrap().parameters,
        CameraParameterPublication::Unwritten
    ));
}
