use super::*;

#[test]
fn euler_producer_returns_pitch_separately_without_reading_w() {
    let (pitch, heading) = euler_angles(
        SkeletonRootVector {
            xyz: [0.0, 0.6, 0.8],
            w: None,
        },
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert!(pitch > 0.6 && pitch < 0.7);
    assert_eq!(heading.to_bits(), 0.0_f32.to_bits());
    let known = euler_angles(
        SkeletonRootVector::known([0.0, 0.6, 0.8, f32::NAN]),
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(
        (pitch.to_bits(), heading.to_bits()),
        (known.0.to_bits(), known.1.to_bits())
    );
}

#[test]
fn smoothing_does_not_clamp_out_of_range_authored_values() {
    assert_eq!(smoothing_complement(3.0).to_bits(), (-3.0_f32).to_bits());
    assert!(smoothing_complement(f32::NAN).is_nan());
}
