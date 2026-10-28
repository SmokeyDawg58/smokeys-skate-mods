use super::*;
use crate::{camera_attributes::CameraAttributes, camera_collision::CameraCollision};
use std::sync::{Arc, Mutex, Weak};

// Component-only constructor fixture. It never admits terrain or fabricates hits.
fn positioner() -> CameraPositioner {
    CameraPositioner::constructed(
        Arc::new(Mutex::new(CameraCollision::constructed(Weak::new()))),
        Arc::new(Mutex::new(CameraAttributes::begin_init())),
        true,
    )
}
#[test]
fn parameters_replace_only_nan_distance_and_instant_preserves_rates() {
    let mut p = positioner();
    p.distance.velocity = 4.0;
    p.distance.acceleration = 5.0;
    p.set_parameters(
        Vector::known([2.0; 4]),
        1.0,
        2.0,
        f32::NAN,
        3.0,
        4.0,
        0.5,
        true,
        false,
        true,
        true,
    );
    assert_eq!(p.requested_distance.to_bits(), 1.0_f32.to_bits());
    assert_eq!(p.distance.velocity.to_bits(), 4.0_f32.to_bits());
    assert_eq!(p.distance.acceleration.to_bits(), 5.0_f32.to_bits());
    p.set_parameters(
        Vector::known([2.0; 4]),
        1.0,
        2.0,
        f32::INFINITY,
        3.0,
        4.0,
        0.5,
        true,
        false,
        true,
        true,
    );
    assert_eq!(
        p.distance.output_position.to_bits(),
        f32::INFINITY.to_bits()
    );
}
#[test]
fn elevation_nan_selects_upper_and_epsilon_ignores_w_and_nan() {
    assert_eq!(clamp_elevation(f32::NAN).to_bits(), 0x3fbc_430e);
    assert!(!exceeds_epsilon(Vector::known([
        0.0,
        f32::NAN,
        0.0,
        1000.0
    ])));
    assert!(!exceeds_epsilon(Vector::known([
        f32::from_bits(0x3780_0000),
        0.0,
        0.0,
        0.0
    ])));
    assert!(exceeds_epsilon(Vector::known([
        f32::from_bits(0x3780_0001),
        0.0,
        0.0,
        0.0
    ])));
}
#[test]
fn distance_unordered_is_too_close_and_equality_is_not_far() {
    let mut p = positioner();
    p.minimum_distance = 1.0;
    p.collision_radius = 0.0;
    p.position = Vector::known([1.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        p.distance_result(PositionResult::Valid).unwrap(),
        PositionResult::TooClose
    );
    p.position.xyz[0] = f32::NAN;
    assert_eq!(
        p.distance_result(PositionResult::Colliding).unwrap(),
        PositionResult::TooClose
    );
}
#[test]
fn collision_start_limit_preserves_three_full_vectors() {
    let mut p = positioner();
    let point = Vector {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    for _ in 0..3 {
        p.add_collision_start(point).unwrap();
    }
    assert!(matches!(
        p.add_collision_start(Vector::known([9.0; 4])),
        Err(PositionerError::CollisionStartCount(3))
    ));
    assert_eq!(p.collision_start_count, 3);
    assert!(p.collision_starts.iter().all(|p| p.w.is_none()));
}
#[test]
fn distance_tracker_speed_publication_precedes_nan_threshold_parameter_failure() {
    let mut p = positioner();
    p.distance.output_position = f32::NAN;
    p.parameters
        .lock()
        .unwrap()
        .parameters
        .fields_mut()
        .distance_speed_clamp = Some(3.0);
    assert!(matches!(
        p.update_distance_tracker(0.1),
        Err(PositionerError::MissingParameter(
            "distance_acceleration_clamp_max"
        ))
    ));
    assert_eq!(
        p.distance.parameters.speed_clamp.unwrap().to_bits(),
        3.0_f32.to_bits()
    );
    assert!(p.distance.parameters.acceleration_max.is_none());
}

#[test]
fn hit_metric_keeps_the_extra_parameter_multiplier() {
    let mut p = positioner();
    p.position = Vector::known([2.0, 0.0, 0.0, 0.0]);
    let result = p
        .resolve_hit(
            CameraLineResult {
                position: Vector::known([1.0, 0.0, 0.0, 0.0]),
                normal: Vector::known([1.0, 0.0, 0.0, 0.0]),
                parameter: 0.01,
                collided: true,
            },
            Vector::known([0.0; 4]),
        )
        .unwrap();
    // Squared distance .0004 alone would take the distance branch. Native
    // multiplies by .01 again and therefore repairs the invalid anchor.
    assert_eq!(result, PositionResult::InvalidAnchor);
    assert_eq!(p.position.xyz[0].to_bits(), 0.02_f32.to_bits());
    assert!(p.current_collision_start.xyz[0] > 2.0);
}
