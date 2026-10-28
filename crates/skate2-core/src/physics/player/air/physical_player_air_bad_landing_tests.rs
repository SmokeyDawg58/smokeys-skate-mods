#![expect(clippy::float_cmp, reason = "exact unchanged tangent lane")]
use super::*;

#[test]
fn native_gt_clear_branch_skips_unordered_and_equality() {
    assert!(!ordered_above(f32::NAN, 1.0));
    assert!(!ordered_above(1.0, f32::NAN));
    assert!(!ordered_above(1.0, 1.0));
    assert!(!ordered_above(-0.0, 0.0));
    assert!(ordered_above(f32::INFINITY, 1.0));
}

#[test]
fn tangent_preserves_sloped_ground_cancellation_and_unknown_w() {
    let normal = SkeletonRootVector {
        xyz: [0.6, 0.8, 0.0],
        w: None,
    };
    let velocity = SkeletonRootVector::known([-3.0, -4.0, 7.0, 0.0]);
    let projected = tangent(normal, velocity, 5.0);
    assert!(projected.xyz[0].abs() < 0.000_001);
    assert!(projected.xyz[1].abs() < 0.000_001);
    assert_eq!(projected.xyz[2], 7.0);
    assert_eq!(projected.w, None);
}

#[test]
fn tangent_keeps_source_fused_cancellation() {
    let n = f32::from_bits(0x3f80_0001);
    let scale = f32::from_bits(0x3f7f_ffff);
    let result = tangent(
        SkeletonRootVector::known([n, 0.0, 0.0, 0.0]),
        SkeletonRootVector::known([-1.0, 0.0, 0.0, 0.0]),
        scale,
    );
    assert_eq!(result.xyz[0].to_bits(), 0x337f_fffe);
    assert_eq!((n * scale - 1.0).to_bits(), 0);
}
