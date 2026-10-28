//! Scalar-splat graph of `XMVectorTan` (0x82581520..0x82581654), consumed
//! by the IK constructor at 0x82b55374. Host estimate bits, denormal mode,
//! NaN payloads and floating-point status remain explicit numeric boundaries.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "IK constructor integration is owned by its aggregate"
    )
)]

use crate::{reckoning_numeric::ReckoningNumeric, reckoning_sin_cos::ReckoningSinCosNumeric};

#[cfg(test)]
#[path = "skeleton_ik_tangent_tests.rs"]
mod tests;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkTangentError<N, R> {
    Numeric(N),
    Rounding(R),
}

const A: [f32; 4] = [
    f32::from_bits(0x3f80_0000),
    f32::from_bits(0xbeee_f582),
    f32::from_bits(0x3cd2_3cf5),
    f32::from_bits(0xb9a3_7b25),
];
const B: [f32; 4] = [
    f32::from_bits(0x3505_bba8),
    f32::from_bits(0xbe08_95af),
    f32::from_bits(0x3b60_7415),
    f32::from_bits(0xb795_d5b9),
];
const C: [f32; 4] = [
    f32::from_bits(0x3fc9_0fdb),
    f32::from_bits(0x2e85_a309),
    f32::from_bits(0x3980_0000),
    f32::from_bits(0x3f22_f983),
];

/// Preserves both unconditional reciprocal calls before either refinement.
/// The caller commits its retained slope only after this function succeeds.
pub(crate) fn try_tangent<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    value: f32,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<f32, SkeletonIkTangentError<N::Error, R::Error>> {
    let input_zero = value == 0.0;
    let turns = rounding
        .round_nearest_even(value * C[3])
        .map_err(SkeletonIkTangentError::Rounding)?;
    let absolute_turns = f32::from_bits(turns.to_bits() & 0x7fff_ffff);
    let reduced_high = (-turns).mul_add(C[0], value);
    let integer_turns = saturating_signed_word(absolute_turns);
    let reduced = (-turns).mul_add(C[1], reduced_high);
    let even = integer_turns & 1 == 0;
    let squared = reduced * reduced;
    let mask = bounds_bits(reduced, C[2]);

    let denominator_inner = squared.mul_add(B[0], A[3]);
    let numerator_inner = squared.mul_add(B[3], B[2]);
    let denominator_middle = squared.mul_add(denominator_inner, A[2]);
    let numerator_middle = squared.mul_add(numerator_inner, B[1]);
    let denominator_outer = squared.mul_add(denominator_middle, A[1]);
    let numerator_outer = squared * numerator_middle;
    let denominator = squared.mul_add(denominator_outer, A[0]);
    let numerator = reduced.mul_add(numerator_outer, reduced);
    // 0x825815bc/15e0/15e4: bounds supplies two high bits, never a bool mask.
    let denominator = bitselect(denominator, 1.0, mask);
    let numerator = bitselect(numerator, reduced, mask);

    let denominator_seed = numeric
        .reciprocal_estimate(denominator)
        .map_err(SkeletonIkTangentError::Numeric)?;
    let negative_numerator = f32::from_bits(numerator.to_bits() ^ 0x8000_0000);
    let negative_seed = numeric
        .reciprocal_estimate(negative_numerator)
        .map_err(SkeletonIkTangentError::Numeric)?;
    let denominator_residual = (-denominator).mul_add(denominator_seed, 1.0);
    let negative_residual = (-negative_numerator).mul_add(negative_seed, 1.0);
    let denominator_first = denominator_seed.mul_add(denominator_residual, denominator_seed);
    let negative_first = negative_seed.mul_add(negative_residual, negative_seed);
    let denominator_residual = (-denominator).mul_add(denominator_first, 1.0);
    // Source self comparisons test these FIRST refinements, not the seconds.
    let denominator_valid = !denominator_first.is_nan();
    let negative_valid = !negative_first.is_nan();
    let denominator_second = denominator_first.mul_add(denominator_residual, denominator_first);
    let negative_residual = (-negative_numerator).mul_add(negative_first, 1.0);
    let denominator_inverse = if denominator_valid {
        denominator_second
    } else {
        denominator_seed
    };
    let negative_second = negative_first.mul_add(negative_residual, negative_first);
    let even_result = numerator * denominator_inverse;
    let negative_inverse = if negative_valid {
        negative_second
    } else {
        negative_seed
    };
    let odd_result = denominator * negative_inverse;
    let result = if even { even_result } else { odd_result };
    Ok(if input_zero { 0.0 } else { result })
}

/// VMX vector signed conversion saturates and maps NaN to zero. The scalar
/// `fctiwz` service intentionally has different checked-host behavior.
#[expect(
    clippy::cast_possible_truncation,
    reason = "native vcfpsxws truncates and saturates"
)]
fn saturating_signed_word(value: f32) -> i32 {
    value as i32
}

/// Native compare-bounds result has only its two most significant bits set.
fn bounds_bits(value: f32, bound: f32) -> u32 {
    let high = if value <= bound { 0 } else { 0x8000_0000 };
    let low = if value >= -bound { 0 } else { 0x4000_0000 };
    high | low
}

fn bitselect(left: f32, right: f32, mask: u32) -> f32 {
    f32::from_bits((left.to_bits() & !mask) | (right.to_bits() & mask))
}
