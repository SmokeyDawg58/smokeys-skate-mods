#![expect(
    clippy::float_cmp,
    reason = "exact source stores, signed zero and branch fixtures"
)]

use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    skeleton_collision_attributes::{SkeletonCollisionAttributeRead, SkeletonCollisionOffset},
};

struct Attributes;
impl SkeletonCollisionAttributeRead for Attributes {
    type Pointer = ();
    type Error = std::convert::Infallible;
    fn cached_offset(&mut self, _: SkeletonCollisionOffset) -> Result<[f32; 4], Self::Error> {
        Ok([0.0; 4])
    }
    fn lookup(&mut self, _: u64) -> Result<(), Self::Error> {
        Ok(())
    }
    fn float(&mut self, (): &()) -> Result<f32, Self::Error> {
        Ok(0.0)
    }
}

fn collision() -> SkeletonCollisionData {
    SkeletonCollisionData::try_new(&mut Attributes).unwrap()
}

fn vector(lanes: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [lanes[0], lanes[1], lanes[2]],
        w: Some(lanes[3]),
    }
}

#[derive(Default)]
struct Numeric {
    calls: Vec<&'static str>,
    fail_at: Option<usize>,
    nan_at: Option<usize>,
}
impl Numeric {
    fn event(&mut self, name: &'static str, value: f32) -> Result<f32, &'static str> {
        let index = self.calls.len();
        self.calls.push(name);
        if self.fail_at == Some(index) {
            Err("numeric refusal")
        } else if self.nan_at == Some(index) {
            Ok(f32::NAN)
        } else {
            Ok(value)
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        let value = HostReckoningNumeric.dot3(a, b).unwrap();
        self.event("dot", value)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("rsqrt", 1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("normal displacement does not request reciprocal estimates")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("SIMD angle wrap must not use signed integer conversion")
    }
}

#[derive(Default)]
struct Rounding {
    values: Vec<f32>,
    fail_at: Option<usize>,
}
impl ReckoningSinCosNumeric for Rounding {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        let index = self.values.len();
        self.values.push(value);
        if self.fail_at == Some(index) {
            Err("rounding refusal")
        } else {
            Ok(value.round_ties_even())
        }
    }
}

#[test]
fn projection_is_published_after_first_dot_before_normalization_refusal() {
    let data = collision();
    let displacement = vector([2.0, 3.0, 4.0, 7.0]);
    let ground = vector([0.0, 1.0, 0.0, 2.0]);
    let sentinel = vector([9.0; 4]);
    for fail_at in 0..4 {
        let mut output = sentinel;
        let mut numeric = Numeric {
            fail_at: Some(fail_at),
            ..Numeric::default()
        };
        let mut rounding = Rounding::default();
        assert_eq!(
            data.try_calculate_normal_displacement(
                displacement,
                ground,
                &mut output,
                &mut numeric,
                &mut rounding
            ),
            Err(SkeletonNormalDisplacementError::Numeric("numeric refusal"))
        );
        assert_eq!(
            output,
            if fail_at == 0 {
                sentinel
            } else {
                vector([2.0, 0.0, 4.0, 1.0])
            }
        );
        assert_eq!(numeric.calls.len(), fail_at + 1);
        assert!(rounding.values.is_empty());
    }
}

#[test]
fn degenerate_projection_preserves_unknown_w_and_bypasses_invalid_normal_count() {
    let mut data = collision();
    data.num_normals = i32::MAX;
    let mut output = vector([9.0; 4]);
    let mut numeric = Numeric::default();
    let mut rounding = Rounding::default();
    data.try_calculate_normal_displacement(
        SkeletonRootVector {
            xyz: [0.0, 3.0, 0.0],
            w: None,
        },
        vector([0.0, 1.0, 0.0, 0.0]),
        &mut output,
        &mut numeric,
        &mut rounding,
    )
    .unwrap();
    assert_eq!(
        output,
        SkeletonRootVector {
            xyz: [0.0; 3],
            w: None
        }
    );
    assert_eq!(numeric.calls, ["dot", "dot", "dot"]);
    assert!(rounding.values.is_empty());
}

#[test]
fn out_of_bounds_count_refuses_only_after_loading_all_twenty_retained_rows() {
    let mut data = collision();
    data.num_normals = 21;
    let mut output = vector([9.0; 4]);
    let mut numeric = Numeric::default();
    let mut rounding = Rounding::default();
    assert_eq!(
        data.try_calculate_normal_displacement(
            vector([2.0, 3.0, 0.0, 7.0]),
            vector([0.0, 1.0, 0.0, 2.0]),
            &mut output,
            &mut numeric,
            &mut rounding,
        ),
        Err(SkeletonNormalDisplacementError::NormalUnavailable { index: 20 })
    );
    assert_eq!(output, vector([2.0, 0.0, 0.0, 1.0]));
    assert_eq!(&numeric.calls[..4], ["dot", "dot", "rsqrt", "dot"]);
    assert_eq!(numeric.calls.len(), 4 + 20 * 3);
    assert!(numeric.calls[4..].iter().all(|call| *call == "dot"));
    assert!(rounding.values.is_empty());
}

#[test]
fn empty_or_negative_count_still_calls_rotation_cosine_then_sine() {
    for count in [0, -1] {
        let mut data = collision();
        data.num_normals = count;
        for fail_at in [Some(0), Some(1), None] {
            let mut output = vector([9.0; 4]);
            let mut rounding = Rounding {
                fail_at,
                ..Rounding::default()
            };
            let result = data.try_calculate_normal_displacement(
                vector([2.0, 3.0, 4.0, 7.0]),
                vector([0.0, 1.0, 0.0, 2.0]),
                &mut output,
                &mut Numeric::default(),
                &mut rounding,
            );
            if let Some(index) = fail_at {
                assert_eq!(
                    result,
                    Err(SkeletonNormalDisplacementError::Rounding(
                        "rounding refusal"
                    ))
                );
                assert_eq!(output, vector([2.0, 0.0, 4.0, 1.0]));
                assert_eq!(rounding.values.len(), index + 1);
            } else {
                result.unwrap();
                assert_eq!(output.xyz, [0.0; 3]);
                assert_eq!(output.w, Some(0.0));
                assert_eq!(rounding.values.len(), 2);
            }
            assert!(rounding.values.iter().all(|value| value.to_bits() == 0));
        }
    }
}

#[test]
fn unordered_normalized_squared_does_not_take_ordered_early_return() {
    let data = collision();
    let mut output = vector([9.0; 4]);
    let mut numeric = Numeric {
        nan_at: Some(3),
        ..Numeric::default()
    };
    let mut rounding = Rounding::default();
    data.try_calculate_normal_displacement(
        vector([2.0, 3.0, 4.0, 7.0]),
        vector([0.0, 1.0, 0.0, 2.0]),
        &mut output,
        &mut numeric,
        &mut rounding,
    )
    .unwrap();
    assert_eq!(rounding.values.len(), 2);
    assert_eq!(output.xyz, [0.0; 3]);
}

#[test]
fn active_normal_has_three_independent_trig_reads_and_preserves_projection_on_each_refusal() {
    let mut data = collision();
    data.num_normals = 1;
    data.normal_info[0].normal = SkeletonCollisionVector::xyz_only([1.0, 0.0, -1.0]);
    for fail_at in 0..3 {
        let mut output = vector([9.0; 4]);
        let mut rounding = Rounding {
            fail_at: Some(fail_at),
            ..Rounding::default()
        };
        assert_eq!(
            data.try_calculate_normal_displacement(
                vector([2.0, 3.0, 0.0, 7.0]),
                vector([0.0, 1.0, 0.0, 2.0]),
                &mut output,
                &mut Numeric::default(),
                &mut rounding,
            ),
            Err(SkeletonNormalDisplacementError::Rounding(
                "rounding refusal"
            ))
        );
        assert_eq!(output, vector([2.0, 0.0, 0.0, 1.0]));
        assert_eq!(rounding.values.len(), fail_at + 1);
        assert!(
            rounding
                .values
                .iter()
                .all(|value| value.to_bits() == rounding.values[0].to_bits())
        );
    }
}

#[test]
fn angular_extrema_not_average_select_rotation_and_recover_source_w_from_xyz() {
    let mut data = collision();
    data.num_normals = 5;
    for (index, degrees) in [-20.0_f32, 60.0, 55.0].into_iter().enumerate() {
        let angle = degrees.to_radians();
        data.normal_info[index].normal =
            SkeletonCollisionVector::xyz_only([angle.cos(), 0.0, -angle.sin()]);
    }
    data.normal_info[3].normal = SkeletonCollisionVector::xyz_only([0.0, 1.0, 0.0]);
    data.normal_info[4].normal = SkeletonCollisionVector::xyz_only([-1.0, 0.0, 0.0]);
    let mut output = vector([9.0; 4]);
    let mut rounding = Rounding::default();
    data.try_calculate_normal_displacement(
        SkeletonRootVector {
            xyz: [2.0, 3.0, 0.0],
            w: None,
        },
        SkeletonRootVector {
            xyz: [0.0, 1.0, 0.0],
            w: None,
        },
        &mut output,
        &mut Numeric::default(),
        &mut rounding,
    )
    .unwrap();
    // Independent geometric oracle checks the source's extrema choice. Literal
    // power-DAG bits and the noncanonical rotation W are covered separately.
    let middle = 20.0_f32.to_radians();
    let expected = [
        2.0 * middle.cos() * middle.cos(),
        0.0,
        -2.0 * middle.sin() * middle.cos(),
    ];
    for (actual, expected) in output.xyz.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.000_01,
            "{actual} != {expected}"
        );
    }
    assert_eq!(output.w.unwrap().to_bits(), output.xyz[0].to_bits());
    assert!(output.w.unwrap() > 1.0);
    assert_eq!(rounding.values.len(), 3);
}

#[test]
fn unordered_signed_angle_is_rejected_by_raw_cr_bit_gates() {
    let mut data = collision();
    data.num_normals = 1;
    data.normal_info[0].normal = SkeletonCollisionVector::xyz_only([1.0, 0.0, -1.0]);
    // The ACos estimate is the fourteenth numeric operation. Returning NaN
    // leaves the preceding normal/length gates finite but poisons the angle.
    let mut numeric = Numeric {
        nan_at: Some(13),
        ..Numeric::default()
    };
    let mut rounding = Rounding::default();
    let mut output = vector([9.0; 4]);
    data.try_calculate_normal_displacement(
        vector([2.0, 3.0, 0.0, 7.0]),
        vector([0.0, 1.0, 0.0, 2.0]),
        &mut output,
        &mut numeric,
        &mut rounding,
    )
    .unwrap();
    assert_eq!(numeric.calls.len(), 15);
    assert_eq!(numeric.calls[13], "rsqrt");
    assert_eq!(rounding.values.len(), 2);
    assert!(rounding.values.iter().all(|value| value.to_bits() == 0));
    assert_eq!(output.xyz, [0.0; 3]);
}

#[test]
fn float_wrap_and_extrema_preserve_strict_endpoints_and_reject_unordered() {
    assert_eq!(wrap_angle(TWO_PI * 0.5).to_bits(), (TWO_PI * 0.5).to_bits());
    assert_eq!(wrap_angle(-TWO_PI * 0.25).to_bits(), (-HALF_PI).to_bits());
    assert_eq!(wrap_angle(TWO_PI * 0.75).to_bits(), (-HALF_PI).to_bits());
    assert!(wrap_angle(f32::INFINITY).is_nan());
    let mut minimum = HALF_PI;
    let mut maximum = -HALF_PI;
    for angle in [
        -HALF_PI,
        HALF_PI,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::NAN,
    ] {
        assert!(!retain_angle(angle, &mut minimum, &mut maximum));
    }
    assert_eq!((minimum, maximum), (HALF_PI, -HALF_PI));
    assert!(retain_angle(0.25, &mut minimum, &mut maximum));
    assert_eq!((minimum, maximum), (0.25, 0.25));
}

#[test]
fn standalone_trig_matches_independent_binary32_fma_snapshots_and_differs_from_combined_graph() {
    // Independent Python ctypes/ucrtbase fmaf oracle; rounded original operand
    // DAGs and table bits, not libc trig. These samples expose power reassociation.
    let cases = [
        (0x3e00_0000, 0x3dff_5577, 0x3f7e_00ab),
        (0x3f99_999a, 0x3f6e_9a1d, 0x3eb9_86f4),
        (0x402d_f854, 0x3ed2_51ee, 0xbf69_6763),
        (0x4049_0625, 0x3a1b_53c6, 0xbf7f_fff8),
        (0x40e8_0000, 0x3f52_b56c, 0x3f11_637e),
        (0x42f6_e979, 0xbf4d_cec1, 0xbf18_3f4b),
        (0xc039_999a, 0xbe74_fdb2, 0xbf78_90b9),
    ];
    let mut differs = false;
    for (input, sine, cosine) in cases {
        let angle = f32::from_bits(input);
        assert_eq!(
            try_sine(angle, &mut HostReckoningSinCosNumeric)
                .unwrap()
                .to_bits(),
            sine
        );
        assert_eq!(
            try_cosine(angle, &mut HostReckoningSinCosNumeric)
                .unwrap()
                .to_bits(),
            cosine
        );
        let combined = try_sin_cos(angle, &mut HostReckoningSinCosNumeric).unwrap();
        differs |= (combined.0.to_bits(), combined.1.to_bits()) != (sine, cosine);
    }
    assert!(differs);
}

#[test]
fn axis_rotation_constructs_noncanonical_w_and_does_not_drop_identity_zero_terms() {
    let rotation = try_rotated_identity([0.0, 1.0, 0.0], 0.0, &mut Rounding::default()).unwrap();
    assert_eq!(
        rotation,
        [
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0]
        ]
    );
    // The source never normalizes/sanitizes the axis. Infinite coefficients
    // contaminate otherwise zero-selected columns through the identity graph.
    let rotation =
        try_rotated_identity([f32::INFINITY, 1.0, 0.0], 0.0, &mut Rounding::default()).unwrap();
    assert!(rotation.iter().flatten().all(|lane| lane.is_nan()));
}
