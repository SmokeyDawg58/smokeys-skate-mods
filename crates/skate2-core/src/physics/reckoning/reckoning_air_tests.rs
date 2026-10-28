#![expect(clippy::float_cmp, reason = "exact source branch and lane fixtures")]
use super::{ReckoningSinCosNumeric, Vector, rotate_heading, try_clamp_air_angle};
use crate::reckoning_numeric::HostReckoningNumeric;
use crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
#[test]
fn heading_rotation_retains_source_permuted_w_and_ignores_input_w() {
    let h = Vector::known([1.0, 0.0, 0.0, f32::NAN]);
    let a = Vector::known([0.0, 1.0, 0.0, 0.0]);
    assert_eq!(
        rotate_heading(h, a, 0.0, 1.0).known_lanes(),
        Some([1.0, 0.0, 0.0, 1.0])
    );
    assert_eq!(rotate_heading(h, a, 1.0, 0.0).xyz, [0.0, 0.0, -1.0]);
}
#[test]
fn clamp_keeps_original_vector_magnitude_and_target_direction() {
    let result = try_clamp_air_angle(
        Vector::known([2.0, 0.0, 0.0, 0.0]),
        Vector::known([0.0, 1.0, 0.0, 0.0]),
        std::f32::consts::FRAC_PI_4,
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap();
    for x in &result.xyz[..2] {
        assert!((*x - std::f32::consts::SQRT_2).abs() < 0.00001);
    }
    assert_eq!(result.xyz[2], 0.0);
}
#[test]
fn antiparallel_zero_axis_returns_original_before_sincos() {
    struct NoSinCos;
    impl ReckoningSinCosNumeric for NoSinCos {
        type Error = ();
        fn round_nearest_even(&mut self, _: f32) -> Result<f32, ()> {
            panic!("zero axis must return before sincos")
        }
    }
    let v = Vector::known([2.0, 0.0, 0.0, 7.0]);
    assert_eq!(
        try_clamp_air_angle(
            v,
            Vector::known([-1.0, 0.0, 0.0, 0.0]),
            0.1,
            &mut HostReckoningNumeric,
            &mut NoSinCos
        )
        .unwrap(),
        v
    );
}
