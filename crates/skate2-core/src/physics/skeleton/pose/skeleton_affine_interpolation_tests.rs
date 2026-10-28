#![expect(
    clippy::float_cmp,
    reason = "literal source copies and exactly representable fixtures"
)]
use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric, reckoning_sin_cos::HostReckoningSinCosNumeric,
};

const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
const QUARTER_TURN: AnimationMatrix = [
    0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

#[path = "skeleton_affine_typed_tests.rs"]
mod typed;

#[derive(Default)]
struct Numeric {
    calls: Vec<(&'static str, f32)>,
    fail_at: Option<usize>,
}
impl Numeric {
    fn event(&mut self, name: &'static str, value: f32, result: f32) -> Result<f32, &'static str> {
        let index = self.calls.len();
        self.calls.push((name, value));
        if self.fail_at == Some(index) {
            Err("numeric refusal")
        } else {
            Ok(result)
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        let value = HostReckoningNumeric.dot3(a, b).unwrap();
        self.event("dot", value, value)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("rsqrt", value, 1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("reciprocal", value, 1.0 / value)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("interpolation never converts to integer")
    }
}
#[derive(Default)]
struct Trigonometry {
    calls: Vec<f32>,
    fail: bool,
}
impl ReckoningSinCosNumeric for Trigonometry {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push(value);
        if self.fail {
            Err("trig refusal")
        } else {
            Ok(value.round_ties_even())
        }
    }
}

#[test]
fn upper_endpoint_skips_query_but_translation_extrapolates_and_keeps_zero_terms() {
    let mut from = IDENTITY;
    from[12..].copy_from_slice(&[2.0, 4.0, 6.0, 8.0]);
    let mut to = QUARTER_TURN;
    to[12..].copy_from_slice(&[10.0, 12.0, 14.0, 16.0]);
    let mut numeric = Numeric::default();
    let mut trig = Trigonometry::default();
    let result = try_interpolate_affine(&from, &to, 2.0, &mut numeric, &mut trig).unwrap();
    assert_eq!(&result[..12], &to[..12]);
    assert_eq!(&result[12..], &[18.0, 20.0, 22.0, 24.0]);
    assert!(numeric.calls.is_empty() && trig.calls.is_empty());
    from[12] = f32::INFINITY;
    let result = try_interpolate_affine(&from, &to, 1.0, &mut numeric, &mut trig).unwrap();
    assert!(
        result[12].is_nan(),
        "from.P*0 still executes even at blend1"
    );
}

#[test]
fn lower_endpoint_runs_complete_query_including_center_before_copying_rotation() {
    let mut from = IDENTITY;
    from[12..].copy_from_slice(&[2.0, 4.0, 6.0, 8.0]);
    let mut to = QUARTER_TURN;
    to[12..].copy_from_slice(&[10.0, 12.0, 14.0, 16.0]);
    for blend in [0.0, -1.0] {
        let mut numeric = Numeric::default();
        let mut trig = Trigonometry::default();
        let result = try_interpolate_affine(&from, &to, blend, &mut numeric, &mut trig).unwrap();
        assert_eq!(&result[..12], &from[..12]);
        assert_eq!(
            numeric.calls.iter().map(|call| call.0).collect::<Vec<_>>(),
            [
                "dot",
                "rsqrt",
                "reciprocal",
                "reciprocal",
                "reciprocal",
                "reciprocal",
                "reciprocal",
                "reciprocal"
            ]
        );
        assert_eq!(
            numeric.calls.last().unwrap().1,
            2.0,
            "center denominator service cannot be skipped"
        );
        assert!(trig.calls.is_empty());
        assert_eq!(result[12], if blend == 0.0 { 2.0 } else { -6.0 });
    }
    for fail_at in 0..8 {
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        assert_eq!(
            try_interpolate_affine(&from, &to, 0.0, &mut numeric, &mut Trigonometry::default()),
            Err(SkeletonAffineInterpolationError::Numeric("numeric refusal"))
        );
        assert_eq!(numeric.calls.len(), fail_at + 1);
    }
}

#[test]
fn relative_query_position_ignores_from_position_but_keeps_to_zero_coefficient_fmas() {
    let mut from = IDENTITY;
    from[12..].copy_from_slice(&[7.0, 11.0, 13.0, 17.0]);
    let mut to = QUARTER_TURN;
    to[12..].copy_from_slice(&[19.0, 23.0, 29.0, 31.0]);
    assert_eq!(relative_rotation(&from, &to), to);
    to[0] = f32::INFINITY;
    assert!(relative_rotation(&from, &to)[12].is_nan());
}

#[test]
fn small_angle_normalizes_axes_independently_and_batches_dots_before_estimates() {
    let matrix = [
        2.0, 0.0, 0.0, 8.0, 1.0, 3.0, 0.0, 12.0, 1.0, 1.0, 4.0, 16.0, 2.0, 3.0, 5.0, 7.0,
    ];
    let mut numeric = Numeric::default();
    let result = try_linear_normalized(&matrix, &matrix, 0.5, &mut numeric).unwrap();
    assert_eq!(
        numeric.calls.iter().map(|call| call.0).collect::<Vec<_>>(),
        ["dot", "dot", "dot", "rsqrt", "rsqrt", "rsqrt"]
    );
    assert_eq!(
        numeric.calls.iter().map(|call| call.1).collect::<Vec<_>>(),
        [4.0, 10.0, 18.0, 4.0, 10.0, 18.0]
    );
    assert_eq!(result[3], 4.0);
    assert!(result[4] > 0.3, "no Gram-Schmidt removal of X projection");
    assert_eq!(&result[12..], &matrix[12..]);
    for fail_at in 0..6 {
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        assert_eq!(
            try_linear_normalized(&matrix, &matrix, 0.5, &mut numeric),
            Err("numeric refusal")
        );
        assert_eq!(numeric.calls.len(), fail_at + 1);
        assert!(
            numeric.calls[..numeric.calls.len().min(3)]
                .iter()
                .all(|call| call.0 == "dot")
        );
    }
}

#[test]
fn small_angle_zero_axis_has_no_normalization_fallback() {
    let mut numeric = Numeric::default();
    let result = try_linear_normalized(&[0.0; 16], &[0.0; 16], 0.5, &mut numeric).unwrap();
    assert!(result[..12].iter().all(|value| value.is_nan()));
    assert_eq!(numeric.calls.len(), 6);
    assert_eq!(&result[12..], &[0.0; 4]);
}

#[test]
fn genuine_small_rotation_uses_normalized_linear_path_and_retains_axis_w() {
    let mut from = IDENTITY;
    from[3] = 4.0;
    let angle = 1.0_f32.to_radians();
    let mut to = IDENTITY;
    to[0] = angle.cos();
    to[2] = -angle.sin();
    to[8] = angle.sin();
    to[10] = angle.cos();
    to[3] = 10.0;
    let mut numeric = Numeric::default();
    let mut trig = Trigonometry::default();
    let result = try_interpolate_affine(&from, &to, 0.5, &mut numeric, &mut trig).unwrap();
    assert!(trig.calls.is_empty());
    assert_eq!(
        numeric.calls[numeric.calls.len() - 6..]
            .iter()
            .map(|call| call.0)
            .collect::<Vec<_>>(),
        ["dot", "dot", "dot", "rsqrt", "rsqrt", "rsqrt"]
    );
    assert!((result[2] + 0.5_f32.to_radians().sin()).abs() < 0.000_001);
    assert!(result[3] > 7.0);
}

#[test]
fn quarter_turn_half_blend_matches_source_fma_bits_and_recovers_rodrigues_w() {
    let mut from = IDENTITY;
    from[12..].copy_from_slice(&[2.0, 4.0, 6.0, 8.0]);
    let mut to = QUARTER_TURN;
    to[12..].copy_from_slice(&[10.0, 12.0, 14.0, 16.0]);
    let mut trig = Trigonometry::default();
    let result =
        try_interpolate_affine(&from, &to, 0.5, &mut Numeric::default(), &mut trig).unwrap();
    // Exact quarter-turn query overrides angle to HALF_PI; existing literal
    // SinCos polynomial at HALF_PI/2 and source Rodrigues FMAs yield these bits.
    assert_eq!(
        result.map(f32::to_bits),
        [
            0x3f35_04f3,
            0,
            0xbf35_04f3,
            0x3f35_04f3,
            0,
            0x3f80_0000,
            0,
            0,
            0x3f35_04f3,
            0,
            0x3f35_04f3,
            0x3f35_04f3,
            0x40c0_0000,
            0x4100_0000,
            0x4120_0000,
            0x4140_0000
        ]
    );
    assert_eq!(trig.calls.len(), 1);
}

#[test]
fn wrapper_translation_uses_separate_destination_product_then_source_fma() {
    let mut from = IDENTITY;
    let mut to = QUARTER_TURN;
    from[15] = f32::from_bits(0x40c8_cbc3);
    to[15] = f32::from_bits(0xc0d3_7cc1);
    let result = try_interpolate_affine(
        &from,
        &to,
        f32::from_bits(0x3ed8_0623),
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
    .unwrap();
    // Independent ctypes/ucrtbase fmaf snapshot: difference-form3f56c1d2,
    // defused weighted sum3f56c1cc, original weighted-FMA result3f56c1ce.
    assert_eq!(result[15].to_bits(), 0x3f56_c1ce);
}

#[test]
fn nan_blend_and_nan_query_angle_reach_trigonometry_instead_of_endpoint_or_small_branch() {
    for (to, blend) in [(QUARTER_TURN, f32::NAN), ([f32::NAN; 16], 0.5)] {
        let mut trig = Trigonometry {
            fail: true,
            ..Trigonometry::default()
        };
        assert_eq!(
            try_interpolate_affine(&IDENTITY, &to, blend, &mut Numeric::default(), &mut trig),
            Err(SkeletonAffineInterpolationError::Trigonometry(
                "trig refusal"
            ))
        );
        assert_eq!(trig.calls.len(), 1);
        assert!(trig.calls[0].is_nan());
    }
}
