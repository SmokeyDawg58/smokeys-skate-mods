use super::{Vector, correct_planar, requires_planar_correction};
use crate::reckoning_numeric::HostReckoningNumeric;
#[test]
fn correction_matches_current_direction_bounded_by_authored_planar_speed_and_preserves_y_w() {
    let result = correct_planar(
        Vector::known([3.0, 9.0, 4.0, 7.0]),
        Vector::known([12.0, -20.0, 16.0, 99.0]),
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(
        result.known_lanes().unwrap().map(f32::to_bits),
        [3.0f32, 9.0, 4.0, 7.0].map(f32::to_bits)
    );
    let result = correct_planar(
        Vector::known([3.0, 9.0, 4.0, 7.0]),
        Vector::known([0.0, -20.0, -2.0, 99.0]),
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(
        result.known_lanes().unwrap().map(f32::to_bits),
        [0.0f32, 9.0, -2.0, 7.0].map(f32::to_bits)
    );
}
#[test]
fn zero_planar_authority_clears_motion_but_retains_vertical_and_unknown_w() {
    let result = correct_planar(
        Vector {
            xyz: [0.0, 6.0, 0.0],
            w: None,
        },
        Vector::known([2.0, 20.0, 0.0, 0.0]),
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(
        result.xyz.map(f32::to_bits),
        [0.0f32, 6.0, 0.0].map(f32::to_bits)
    );
    assert!(result.w.is_none());
}
#[test]
fn raw_bge_lt_clear_enters_correction_on_equality_and_unordered() {
    assert!(!requires_planar_correction(8.0, 3.0));
    assert!(requires_planar_correction(9.0, 3.0));
    assert!(requires_planar_correction(f32::NAN, 3.0));
    assert!(requires_planar_correction(0.0, f32::NAN));
    assert!(requires_planar_correction(f32::INFINITY, 3.0));
}
