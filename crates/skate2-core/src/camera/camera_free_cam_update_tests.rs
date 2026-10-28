use super::*;
use crate::camera_attributes::CameraAttributes;
use std::sync::{Arc, Mutex};

#[test]
fn late_elevation_parameter_failure_retains_heading_store() {
    let mut attributes = CameraAttributes::begin_init();
    let fields = attributes.parameters.fields_mut();
    fields.free_cam_heading_offset = Some(60.0);
    fields.free_cam_heading_speed = Some(60.0);
    fields.free_cam_elevation_speed = Some(60.0);
    let mut camera = CameraFreeCam {
        parameters: Arc::new(Mutex::new(attributes)),
        heading_offset: 0.0,
        elevation_offset: 0.25,
    };
    assert!(matches!(
        camera.try_update(Vector::known([1.0, 1.0, 0.0, 0.0])),
        Err(FreeCamUpdateError::MissingParameter("elevation_offset"))
    ));
    assert_eq!(
        camera.heading_offset.to_bits(),
        (60.0 * f32::from_bits(0x3c8e_fa35)).to_bits()
    );
    assert_eq!(camera.elevation_offset.to_bits(), 0.25_f32.to_bits());
}

#[test]
fn source_fsel_nan_target_selects_upper_limit() {
    assert_eq!(clamp_target(f32::NAN, 2.0).to_bits(), 2.0_f32.to_bits());
    assert_eq!(clamp_target(-3.0, 2.0).to_bits(), (-2.0_f32).to_bits());
    assert_eq!(clamp_target(-0.0, 2.0).to_bits(), (-0.0_f32).to_bits());
}

use crate::skeleton_root_transforms::SkeletonRootVector as Vector;
