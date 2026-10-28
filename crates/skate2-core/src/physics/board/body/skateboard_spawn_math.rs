//! Spawn-specific Math helpers, not the different rw body normalization.
//!
//! Source: `docs/evidence/skateboard-spawn-transform-source.md`. The arithmetic
//! graph and raw affine lanes are retained. Host dot reductions and reciprocal
//! square-root estimates are explicit approximations, not Xenon bit parity.
//! These helpers neither sample the live tolerance cvar nor move any body.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "spawn awaits live player construction")
)]

/// Numeric boundaries for `Math::OrthoNormalizeRotation` (`0x82b22c78`) and
/// `Math::IsOrthogonalRotation` (`0x82b2f038`). Calls preserve source order.
pub(crate) trait SkateboardSpawnMathNumeric {
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> f32;
    fn dot4(&mut self, left: [f32; 4], right: [f32; 4]) -> f32;
    fn reciprocal_sqrt_estimate(&mut self, squared_length: f32) -> f32;
}

/// Pure host model: nested scalar FMAs for reductions, `sqrt().recip()` for
/// the estimate. Native VMX reduction/estimate, denormal and NaN bits are open.
pub(crate) struct HostSkateboardSpawnMathNumeric;

impl SkateboardSpawnMathNumeric for HostSkateboardSpawnMathNumeric {
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
        left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
    }

    fn dot4(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
        left[0].mul_add(
            right[0],
            left[1].mul_add(right[1], left[2].mul_add(right[2], left[3] * right[3])),
        )
    }

    fn reciprocal_sqrt_estimate(&mut self, squared_length: f32) -> f32 {
        squared_length.sqrt().recip()
    }
}

/// Full four-lane affine composition from `SkateboardBody::SetTransform`
/// (`0x82b34b84..0x82b34c18`). Axes: X multiply, Y FMA, Z FMA. Translation:
/// X FMA into outer P, Y FMA, Z FMA. Inner W coefficients are not consumed;
/// outer W lanes are computed rather than replaced with homogeneous constants.
pub(crate) fn skateboard_spawn_compose(outer: &[f32; 16], inner: &[f32; 16]) -> [f32; 16] {
    std::array::from_fn(|index| {
        let column = index / 4;
        let lane = index % 4;
        let coefficient = column * 4;
        let x = if column == 3 {
            inner[coefficient].mul_add(outer[lane], outer[12 + lane])
        } else {
            inner[coefficient] * outer[lane]
        };
        let xy = inner[coefficient + 1].mul_add(outer[4 + lane], x);
        inner[coefficient + 2].mul_add(outer[8 + lane], xy)
    })
}

/// Rigid transpose inverse at `0x82b34b34..0x82b34b78`. No normalization or
/// determinant inverse. New basis W lanes are +0; inverse translation uses
/// arithmetic `+0 - P`, then Z multiply, Y FMA, X FMA, including output W.
pub(crate) fn skateboard_spawn_rigid_inverse(matrix: &[f32; 16]) -> [f32; 16] {
    let mut inverse = rotation_transpose(matrix);
    let negative_position: [f32; 4] = std::array::from_fn(|lane| 0.0 - matrix[12 + lane]);
    for lane in 0..4 {
        let z = negative_position[2] * inverse[8 + lane];
        let zy = negative_position[1].mul_add(inverse[4 + lane], z);
        inverse[12 + lane] = negative_position[0].mul_add(inverse[lane], zy);
    }
    inverse
}

fn rotation_transpose(matrix: &[f32; 16]) -> [f32; 16] {
    std::array::from_fn(|index| {
        let column = index / 4;
        let lane = index % 4;
        if column < 3 && lane < 3 {
            matrix[lane * 4 + column]
        } else {
            0.0
        }
    })
}

fn column(matrix: &[f32; 16], index: usize) -> [f32; 4] {
    std::array::from_fn(|lane| matrix[index * 4 + lane])
}

/// Reverse-axis CLASSICAL Gram-Schmidt at `0x82b22c78..0x82b22d24`.
/// Z then Y then X; projections use the original current input, not its
/// progressively shortened candidate. Projection products/subtractions remain
/// separate. Both fused reciprocal-square-root refinements run for every axis.
/// No finite/zero guard, alternate axis, or handedness repair is performed.
pub(crate) fn skateboard_spawn_normalize(
    input: &[f32; 16],
    numeric: &mut impl SkateboardSpawnMathNumeric,
) -> [f32; 16] {
    let translation = column(input, 3);
    let mut output = [0.0; 16];
    for axis in (0..3).rev() {
        let original = column(input, axis);
        let mut candidate = original;
        for previous in ((axis + 1)..3).rev() {
            let basis = column(&output, previous);
            let projection = numeric.dot3(basis, original);
            for lane in 0..4 {
                let product = basis[lane] * projection;
                candidate[lane] -= product;
            }
        }
        let squared_length = numeric.dot3(candidate, candidate);
        let estimate = numeric.reciprocal_sqrt_estimate(squared_length);
        let reciprocal_length = refine_reciprocal_length(squared_length, estimate);
        for lane in 0..4 {
            output[axis * 4 + lane] = candidate[lane] * reciprocal_length;
        }
    }
    output[12..16].copy_from_slice(&translation);
    output
}

fn refine_reciprocal_length(squared_length: f32, mut estimate: f32) -> f32 {
    for _ in 0..2 {
        let square = estimate * estimate;
        let half = estimate * 0.5;
        let residual = (-squared_length).mul_add(square, 1.0);
        estimate = half.mul_add(residual, estimate);
    }
    estimate
}

/// Original quartic assertion predicate (`0x82b2f038..0x82b2f1b0`), NOT a
/// finite-input validator. The caller supplies the current tolerance sample.
/// Equality and unordered comparison pass; reflections are not rejected.
pub(crate) fn skateboard_spawn_is_orthogonal(
    input: &[f32; 16],
    tolerance: f32,
    numeric: &mut impl SkateboardSpawnMathNumeric,
) -> bool {
    let transpose = rotation_transpose(input);
    let mut rotation = *input;
    // Source constructs a zero translation; even that column's zero-term
    // arithmetic is evaluated, preserving exceptional basis propagation.
    rotation[12..16].fill(0.0);
    let mut error = skateboard_spawn_compose(&transpose, &rotation);
    for axis in 0..3 {
        error[axis * 4 + axis] -= 1.0;
    }
    // Subtract the other identity lanes too; do not replace source subtraction
    // with mere copying when signed-zero/exceptional inputs are supplied.
    for (index, value) in error.iter_mut().enumerate() {
        if !matches!(index, 0 | 5 | 10) {
            *value -= 0.0;
        }
    }
    let squared_errors: [f32; 4] = std::array::from_fn(|axis| {
        let vector = column(&error, axis);
        numeric.dot3(vector, vector)
    });
    let quartic_error = numeric.dot4(squared_errors, squared_errors);
    let absolute_error = f32::from_bits(quartic_error.to_bits() & 0x7fff_ffff);
    absolute_error.partial_cmp(&tolerance) != Some(std::cmp::Ordering::Greater)
}

#[cfg(test)]
// Exact representable discriminators below intentionally compare f32 values.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    #[derive(Debug)]
    enum Call {
        Dot3([f32; 4], [f32; 4]),
        Dot4([f32; 4], [f32; 4]),
        Estimate(f32),
    }

    #[derive(Default)]
    struct RecordingNumeric {
        calls: Vec<Call>,
    }

    impl SkateboardSpawnMathNumeric for RecordingNumeric {
        fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
            self.calls.push(Call::Dot3(left, right));
            HostSkateboardSpawnMathNumeric.dot3(left, right)
        }

        fn dot4(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
            self.calls.push(Call::Dot4(left, right));
            HostSkateboardSpawnMathNumeric.dot4(left, right)
        }

        fn reciprocal_sqrt_estimate(&mut self, value: f32) -> f32 {
            self.calls.push(Call::Estimate(value));
            HostSkateboardSpawnMathNumeric.reciprocal_sqrt_estimate(value)
        }
    }

    #[test]
    fn composition_retains_full_output_w_and_ignores_inner_w_coefficients() {
        let outer = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let inner = [
            2.0,
            0.0,
            0.0,
            f32::NAN,
            0.0,
            3.0,
            0.0,
            f32::INFINITY,
            0.0,
            0.0,
            4.0,
            -99.0,
            2.0,
            3.0,
            4.0,
            100.0,
        ];
        assert_eq!(
            skateboard_spawn_compose(&outer, &inner),
            [
                2.0, 4.0, 6.0, 8.0, 15.0, 18.0, 21.0, 24.0, 36.0, 40.0, 44.0, 48.0, 66.0, 76.0,
                86.0, 96.0,
            ]
        );
    }

    #[test]
    fn composition_keeps_axis_multiply_and_translation_fma_distinct() {
        let mut outer = IDENTITY;
        outer[0] = 4097.0;
        outer[4] = 4097.0;
        outer[12] = -16_785_408.0;
        let mut inner = IDENTITY;
        inner[0] = 4097.0;
        inner[1] = -4097.0;
        inner[12] = 4097.0;
        let result = skateboard_spawn_compose(&outer, &inner);
        assert_eq!(result[0], -1.0); // Defusing the Y FMA would give zero.
        assert_eq!(result[12], 1.0); // Defusing translation's first FMA gives zero.
    }

    #[test]
    fn rigid_inverse_transposes_xyz_and_keeps_arithmetic_translation_w() {
        let input = [
            0.0,
            1.0,
            0.0,
            7.0,
            -1.0,
            0.0,
            0.0,
            8.0,
            0.0,
            0.0,
            1.0,
            9.0,
            2.0,
            3.0,
            4.0,
            f32::NAN,
        ];
        let result = skateboard_spawn_rigid_inverse(&input);
        assert_eq!(
            &result[..12],
            &[0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,]
        );
        assert_eq!(&result[12..15], &[-3.0, 2.0, -4.0]);
        assert_eq!(result[15].to_bits(), (-0.0_f32).to_bits());
        for axis in 0..3 {
            assert_eq!(result[axis * 4 + 3].to_bits(), 0);
        }
    }

    #[test]
    fn rigid_inverse_translation_starts_with_z_multiply_then_y_and_x_fmas() {
        let mut input = IDENTITY;
        input[1] = 4097.0;
        input[2] = 4097.0;
        input[12] = 0.0;
        input[13] = 4097.0;
        input[14] = -4097.0;
        let result = skateboard_spawn_rigid_inverse(&input);
        // Z product rounds down, then Y FMA preserves the -1 residual.
        // Reversing accumulation or splitting the FMA changes this result.
        assert_eq!(result[12], -1.0);
    }

    #[test]
    fn normalization_visits_reverse_axes_and_projects_original_current_axis() {
        let input = [
            4.0, 5.0, 6.0, 7.0, 1.0, 3.0, 2.0, 8.0, 0.0, 0.0, 2.0, 9.0, 11.0, 12.0, 13.0, -0.0,
        ];
        let mut numeric = RecordingNumeric::default();
        let result = skateboard_spawn_normalize(&input, &mut numeric);
        assert_eq!(numeric.calls.len(), 9);
        assert!(matches!(numeric.calls[0], Call::Dot3(a, b)
            if a == column(&input, 2) && b == a));
        assert!(matches!(numeric.calls[1], Call::Estimate(4.0)));
        assert!(matches!(numeric.calls[2], Call::Dot3(a, b)
            if a == column(&result, 2) && b == column(&input, 1)));
        assert!(matches!(numeric.calls[3], Call::Dot3(a, b) if a == b));
        assert!(matches!(numeric.calls[4], Call::Estimate(_)));
        assert!(matches!(numeric.calls[5], Call::Dot3(a, b)
            if a == column(&result, 2) && b == column(&input, 0)));
        assert!(matches!(numeric.calls[6], Call::Dot3(a, b)
            if a == column(&result, 1) && b == column(&input, 0)));
        assert!(matches!(numeric.calls[7], Call::Dot3(a, b) if a == b));
        assert!(matches!(numeric.calls[8], Call::Estimate(_)));
        assert_eq!(result[11], 4.5); // W is scaled with Z, not reset.
        assert_eq!(
            column(&result, 3).map(f32::to_bits),
            column(&input, 3).map(f32::to_bits)
        );
    }

    #[test]
    fn projection_product_is_rounded_before_subtracting() {
        struct ScriptedDot(usize);
        impl SkateboardSpawnMathNumeric for ScriptedDot {
            fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> f32 {
                let result = [1.0, 4097.0, 1.0, 0.0, 0.0, 1.0][self.0];
                self.0 += 1;
                result
            }
            fn dot4(&mut self, _: [f32; 4], _: [f32; 4]) -> f32 {
                panic!("normalization does not use dot4")
            }
            fn reciprocal_sqrt_estimate(&mut self, value: f32) -> f32 {
                assert_eq!(value, 1.0);
                1.0
            }
        }
        let mut input = IDENTITY;
        input[8] = 4097.0;
        input[10] = 0.0;
        input[4] = 16_785_408.0;
        let output = skateboard_spawn_normalize(&input, &mut ScriptedDot(0));
        assert_eq!(output[4], 0.0);
        assert_eq!((-4097.0_f32).mul_add(4097.0, input[4]), -1.0);
    }

    #[test]
    fn reciprocal_length_keeps_two_fused_refinement_rounds() {
        assert_eq!(refine_reciprocal_length(1.0, 0.5), 0.868_774_4);
        let value = f32::from_bits(0x68d3_92ce);
        let estimate = value.sqrt().recip();
        assert_eq!(
            refine_reciprocal_length(value, estimate).to_bits(),
            0x2ac7_1ea9
        );
        let mut defused = estimate;
        for _ in 0..2 {
            let square = defused * defused;
            let half = defused * 0.5;
            let residual = 1.0 - value * square;
            defused += half * residual;
        }
        assert_eq!(defused.to_bits(), 0x2ac7_1ea8);
    }

    #[test]
    fn normalization_copies_translation_bits_and_has_no_degenerate_repair() {
        let mut input = IDENTITY;
        input[0] = -1.0;
        input[12] = f32::from_bits(0x7fc0_4321);
        input[15] = -0.0;
        let result = skateboard_spawn_normalize(&input, &mut HostSkateboardSpawnMathNumeric);
        assert_eq!(result[0], -1.0); // No handedness repair.
        assert_eq!(
            column(&result, 3).map(f32::to_bits),
            column(&input, 3).map(f32::to_bits)
        );
        let zero = skateboard_spawn_normalize(&[0.0; 16], &mut HostSkateboardSpawnMathNumeric);
        assert!(zero[..12].iter().all(|value| value.is_nan()));
    }

    #[test]
    fn orthogonality_uses_quartic_reduction_and_accepts_equality() {
        let mut input = IDENTITY;
        input[0] = 2.0;
        let mut numeric = RecordingNumeric::default();
        assert!(skateboard_spawn_is_orthogonal(&input, 81.0, &mut numeric));
        assert_eq!(numeric.calls.len(), 5);
        assert!(matches!(numeric.calls[4], Call::Dot4(a, b)
            if a == [9.0, 0.0, 0.0, 0.0] && b == a));
        assert!(!skateboard_spawn_is_orthogonal(&input, 10.0, &mut numeric));
    }

    #[test]
    fn orthogonality_is_not_a_finite_or_handedness_check() {
        let mut reflected = IDENTITY;
        reflected[0] = -1.0;
        reflected[3] = f32::NAN;
        reflected[12..16].fill(f32::NAN);
        assert!(skateboard_spawn_is_orthogonal(
            &reflected,
            0.0,
            &mut HostSkateboardSpawnMathNumeric
        ));
        assert!(!skateboard_spawn_is_orthogonal(
            &IDENTITY,
            -1.0,
            &mut HostSkateboardSpawnMathNumeric
        ));
        assert!(skateboard_spawn_is_orthogonal(
            &IDENTITY,
            f32::NAN,
            &mut HostSkateboardSpawnMathNumeric
        ));
        let mut exceptional = IDENTITY;
        exceptional[0] = f32::INFINITY;
        let mut numeric = RecordingNumeric::default();
        assert!(skateboard_spawn_is_orthogonal(
            &exceptional,
            0.0,
            &mut numeric
        ));
        assert!(matches!(numeric.calls[3], Call::Dot3(a, b)
            if a[0].is_nan() && b[0].is_nan())); // Fourth zero column was evaluated.
    }
}
