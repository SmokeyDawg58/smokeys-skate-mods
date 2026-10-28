use super::*;

#[test]
fn zero_angles_offset_the_actual_anchor_and_preserve_unknown_w() {
    let anchor = SkeletonRootVector {
        xyz: [3.0, 4.0, 5.0],
        w: None,
    };
    let result = calculate_position(anchor, 0.0, 0.0, 2.0);
    assert_eq!(
        result.xyz.map(f32::to_bits),
        [3.0_f32.to_bits(), 4.0_f32.to_bits(), 7.0_f32.to_bits()]
    );
    assert_eq!(result.w, None);
}

#[test]
fn infinite_distance_preserves_zero_times_infinity_in_transverse_lanes() {
    let result = calculate_position(
        SkeletonRootVector::known([0.0, 0.0, 0.0, 1.0]),
        0.0,
        0.0,
        f32::INFINITY,
    );
    assert!(result.xyz[0].is_nan());
    assert!(result.xyz[1].is_nan());
    assert_eq!(result.xyz[2].to_bits(), f32::INFINITY.to_bits());
    assert!(result.w.unwrap().is_nan());
}
