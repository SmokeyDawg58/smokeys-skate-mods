//! Scalar-splat `XMVectorSin824b0060`; child of the existing sine/cosine module.
use super::{INVERSE_TWO_PI, ReckoningSinCosNumeric, SINE_COEFFICIENTS, TWO_PI};

/// Native `vrfin` remains a collaborator. Unlike `XMVectorSinCos`, this source
/// builds every succeeding odd power by multiplying the previous one by x².
pub(crate) fn try_sine<N: ReckoningSinCosNumeric>(
    angle: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let turns = angle * INVERSE_TWO_PI;
    let nearest = numeric.round_nearest_even(turns)?;
    // Standard VNMSUB at824b00cc: negate AFTER the fused subtraction rounds.
    let reduced = -TWO_PI.mul_add(nearest, -angle);
    let squared = reduced * reduced;
    let mut odd_power = squared * reduced;
    let mut result = reduced;
    for (index, coefficient) in SINE_COEFFICIENTS.into_iter().enumerate() {
        result = coefficient.mul_add(odd_power, result);
        if index + 1 != SINE_COEFFICIENTS.len() {
            odd_power *= squared;
        }
    }
    Ok(result)
}
