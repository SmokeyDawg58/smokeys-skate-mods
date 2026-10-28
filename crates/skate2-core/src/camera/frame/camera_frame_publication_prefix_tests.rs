use super::*;
use crate::skeleton_root_transforms::SkeletonRootMatrix;

#[test]
fn inverse_dt_uses_zero_gate_before_reciprocal() {
    assert_eq!(inverse_frame_dt(0.0).to_bits(), 1.0_f32.to_bits());
    assert_eq!(inverse_frame_dt(-0.0).to_bits(), 1.0_f32.to_bits());
    assert_eq!(inverse_frame_dt(0.25).to_bits(), 4.0_f32.to_bits());
    assert!(inverse_frame_dt(f32::NAN).is_nan());
}

#[test]
fn prefix_uses_old_translation_and_keeps_both_matrices_unchanged() {
    let mut frame = CameraFrame::constructed();
    frame
        .matrix
        .set_column(3, Vector::known([2.0, 4.0, 6.0, 8.0]));
    let old_matrix = frame.matrix;
    let old_previous = frame.previous_matrix;
    let old_frame_velocity = frame.velocity;
    let mut camera_velocity = Vector::known([99.0; 4]);
    let reference = Vector::known([5.0, 6.0, 7.0, 11.0]);
    publish_prefix_parts(
        &mut frame,
        &mut camera_velocity,
        reference,
        0.25,
        -0.5,
        Vector::known([3.0, 6.0, 9.0, 12.0]),
        2.0,
    );
    assert_eq!(frame.reference, reference);
    assert_eq!((frame.framing_x, frame.framing_y), (0.25, -0.5));
    assert_eq!(camera_velocity.known_lanes(), Some([2.0, 4.0, 6.0, 8.0]));
    assert_eq!(frame.matrix, old_matrix);
    assert_eq!(frame.previous_matrix, old_previous);
    assert_eq!(frame.velocity, old_frame_velocity);
}

#[test]
fn unknown_w_is_retained_through_velocity_and_reference_copies() {
    let mut frame = CameraFrame::constructed();
    frame.matrix = SkeletonRootMatrix::translation(Vector {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    });
    let mut camera_velocity = Vector::known([0.0; 4]);
    let reference = Vector {
        xyz: [4.0, 5.0, 6.0],
        w: None,
    };
    publish_prefix_parts(
        &mut frame,
        &mut camera_velocity,
        reference,
        0.0,
        0.0,
        Vector::known([4.0, 6.0, 8.0, 9.0]),
        1.0,
    );
    assert_eq!(frame.reference.w, None);
    assert_eq!(camera_velocity.xyz, [3.0, 4.0, 5.0]);
    assert_eq!(camera_velocity.w, None);
}
