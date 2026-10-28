use super::*;
use crate::{
    camera_attributes::CameraAttributes, camera_collision::CameraCollision, physics::PointGraph,
};
use std::sync::{Arc, Mutex, Weak};

/// Constructor-only fixture; it never admits a world or issues a query.
fn predictor() -> CameraDropPredictor {
    CameraDropPredictor::constructed(
        Arc::new(Mutex::new(CameraAttributes::begin_init())),
        Arc::new(Mutex::new(CameraCollision::constructed(Weak::new()))),
    )
}

#[test]
fn largest_drop_keeps_first_tie_and_stops_at_invalid_prefix() {
    let mut predictor = predictor();
    for test in &mut predictor.tests {
        test.valid = true;
    }
    predictor.tests[0].distance = 3.0;
    predictor.tests[1].distance = 3.0;
    predictor.tests[2].distance = f32::NAN;
    predictor.tests[3].valid = false;
    predictor.tests[4].distance = 100.0;
    assert_eq!(predictor.find_largest(), 0);
    predictor.tests[1].distance = 4.0;
    assert_eq!(predictor.find_largest(), 1);
}

#[test]
fn inactive_values_still_evaluate_curves_and_keep_late_failure_prefix() {
    let mut predictor = predictor();
    {
        let mut attributes = predictor.parameters.lock().unwrap();
        let p = attributes.parameters.fields_mut();
        p.drop_elevation_curve = Some(PointGraph {
            x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            y: [2.0; 8],
        });
        p.drop_smoothing_curve = Some(PointGraph {
            x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            y: [0.75; 8],
        });
        p.drop_elevation_offset = Some(10.0);
    }
    predictor.was_using_drop_curve = false;
    assert!(matches!(
        predictor.update_camera_values(1.0 / 60.0, false),
        Err(DropUpdateError::MissingParameter(
            "drop_tracker_speed_clamp"
        ))
    ));
    assert_eq!(
        predictor.target_elevation_offset.to_bits(),
        ((10.0 * f32::from_bits(0x3c8e_fa35)) * 2.0).to_bits()
    );
    assert!(predictor.was_using_drop_curve);
    assert_eq!(predictor.elevation_offset.to_bits(), 0.0_f32.to_bits());
    assert!(predictor.batch.is_none());
}

#[test]
fn unordered_target_can_select_incline_curve_before_late_failure() {
    let mut predictor = predictor();
    {
        let mut attributes = predictor.parameters.lock().unwrap();
        let p = attributes.parameters.fields_mut();
        p.drop_min_drop_distance = Some(1.0);
        p.drop_elevation_curve = Some(PointGraph {
            x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            y: [2.0; 8],
        });
        p.incline_smoothing_curve = Some(PointGraph {
            x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            y: [0.25; 8],
        });
        p.drop_elevation_offset = Some(f32::NAN);
    }
    assert!(matches!(
        predictor.update_camera_values(1.0 / 60.0, true),
        Err(DropUpdateError::MissingParameter(
            "drop_tracker_speed_clamp"
        ))
    ));
    assert!(predictor.target_elevation_offset.is_nan());
    assert!(!predictor.was_using_drop_curve);
}
