//! Scalar-splat projection of `XMVectorSinCos`, 0x82584f60..0x825850e8.
//!
//! `Skeleton::UpdateRootTransforms` loads the angle's X and splats it before the
//! call at 0x82b42b7c. This helper computes that one independent lane, not four
//! invented service calls. The native function stores cosine before sine into
//! distinct stack outputs there; this checked host API publishes a completed
//! (sine, cosine) pair after its sole fallible rounding service.
//!
//! The literal polynomial, rounded power dependencies and ascending fused sums
//! are retained. Nearest rounding, denormals, NaN payloads, VNMSUB zero signs and
//! FP status remain explicit host boundaries; this is not a Xenon bit-trace claim.

use std::convert::Infallible;

#[cfg(test)]
#[path = "reckoning_sin_cos_tests.rs"]
mod tests;

/// Native `vrfin128` returns an integer-valued float, with nearest-even rounding.
/// It is not fctiwz, an i32 range check, or halfway-away-from-zero rounding.
pub(crate) trait ReckoningSinCosNumeric {
    type Error;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error>;
}

/// Ordinary host nearest-even float semantics; no nonfinite repair or narrowing
/// conversion. The polynomial still runs for every successful returned value.
pub(crate) struct HostReckoningSinCosNumeric;

impl ReckoningSinCosNumeric for HostReckoningSinCosNumeric {
    type Error = Infallible;

    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Infallible> {
        Ok(value.round_ties_even())
    }
}

const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const INVERSE_TWO_PI: f32 = f32::from_bits(0x3e22_f983);

// Original g_XMSinCoefficients0/1/2 at 0x823cfe10/20/30. The first table's
// leading 1 is the initial reduced angle; remaining entries multiply x^3..x^23.
const SINE_COEFFICIENTS: [f32; 11] = [
    f32::from_bits(0xbe2a_aaab),
    f32::from_bits(0x3c08_8889),
    f32::from_bits(0xb950_0d01),
    f32::from_bits(0x3638_ef1d),
    f32::from_bits(0xb2d7_322b),
    f32::from_bits(0x2f30_9231),
    f32::from_bits(0xab57_3f9f),
    f32::from_bits(0x274a_963c),
    f32::from_bits(0xa317_a4da),
    f32::from_bits(0x1eb8_dc78),
    f32::from_bits(0x9a3b_0da1),
];

// Original g_XMCosCoefficients0/1/2 at 0x823cfe40/50/60. Native constructs
// the initial 1 separately; these entries multiply x^2..x^22 in ascending order.
const COSINE_COEFFICIENTS: [f32; 11] = [
    f32::from_bits(0xbf00_0000),
    f32::from_bits(0x3d2a_aaab),
    f32::from_bits(0xbab6_0b61),
    f32::from_bits(0x37d0_0d01),
    f32::from_bits(0xb493_f27e),
    f32::from_bits(0x310f_76c8),
    f32::from_bits(0xad49_cba5),
    f32::from_bits(0x2957_3f9f),
    f32::from_bits(0xa534_13c3),
    f32::from_bits(0x20f2_a15d),
    f32::from_bits(0x9c86_71cb),
];

/// `XMVectorSinCos` range reduction: round the f32 turns product to an integral
/// float, then negative fused subtraction of that many turns. No modulo,
/// domain/finite gate, normalization helper, or libc `sin_cos` substitution.
pub(crate) fn try_sin_cos<N: ReckoningSinCosNumeric>(
    angle: f32,
    numeric: &mut N,
) -> Result<(f32, f32), N::Error> {
    let turns = angle * INVERSE_TWO_PI;
    let nearest = numeric.round_nearest_even(turns)?;
    let reduced = (-TWO_PI).mul_add(nearest, angle);
    Ok(sin_cos_from_reduced(reduced))
}

/// Named polynomial powers retain the original multiply dependency graph.
/// Independent power construction is grouped before the sums; there are no
/// intervening callbacks or published stores, and native FP status is excluded.
fn sin_cos_from_reduced(angle: f32) -> (f32, f32) {
    let x2 = angle * angle;
    let x3 = x2 * angle;
    let x4 = x2 * x2;
    let x5 = x3 * x2;
    let x6 = x3 * x3;
    let x8 = x4 * x4;
    let x7 = x4 * x3;
    let x9 = x5 * x4;
    let x10 = x5 * x5;
    let x11 = x6 * x5;
    let x12 = x6 * x6;
    let x13 = x7 * x6;
    let x15 = x8 * x7;
    let x14 = x7 * x7;
    let x16 = x8 * x8;
    let x17 = x9 * x8;
    let x19 = x10 * x9;
    let x18 = x9 * x9;
    let x21 = x11 * x10;
    let x20 = x10 * x10;
    let x23 = x12 * x11;
    let x22 = x11 * x11;
    let odd = [x3, x5, x7, x9, x11, x13, x15, x17, x19, x21, x23];
    let even = [x2, x4, x6, x8, x10, x12, x14, x16, x18, x20, x22];
    let mut sine = angle;
    let mut cosine = 1.0;
    for index in 0..11 {
        cosine = COSINE_COEFFICIENTS[index].mul_add(even[index], cosine);
        sine = SINE_COEFFICIENTS[index].mul_add(odd[index], sine);
    }
    (sine, cosine)
}

/// Scalar-splat `XMVectorCos`, 0x824b0130..0x824b0208.
pub(crate) fn try_cosine<R: ReckoningSinCosNumeric>(
    angle: f32,
    rounding: &mut R,
) -> Result<f32, R::Error> {
    let turns = angle * INVERSE_TWO_PI;
    let nearest = rounding.round_nearest_even(turns)?;
    let x = (-TWO_PI).mul_add(nearest, angle);
    let x2 = x * x;
    let x4 = x2 * x2;
    let x6 = x4 * x2;
    let x8 = x4 * x4;
    let x10 = x6 * x4;
    let x12 = x6 * x6;
    let x14 = x8 * x6;
    let x16 = x8 * x8;
    let x18 = x10 * x8;
    let x20 = x10 * x10;
    let x22 = x12 * x10;
    let powers = [x2, x4, x6, x8, x10, x12, x14, x16, x18, x20, x22];
    let mut result = 1.0;
    for (coefficient, power) in COSINE_COEFFICIENTS.into_iter().zip(powers) {
        result = coefficient.mul_add(power, result);
    }
    Ok(result)
}

#[path = "reckoning_sine.rs"]
pub(crate) mod sine;
