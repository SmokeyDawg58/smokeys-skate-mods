//! Full four-column inverse, native `rw::math::vpu::Inverse82600f38`.
use crate::{animation::AnimationMatrix, reckoning_numeric::ReckoningNumeric};

pub(crate) trait PresentationMatrixNumeric: ReckoningNumeric {
    /// Optional observer of the actual computed operand; never changes arithmetic.
    fn observe_twist_length(&mut self, _source: usize, _destination: usize, _length: f32) {}
    fn dot4(&mut self, first: [f32; 4], second: [f32; 4]) -> Result<f32, Self::Error>;
}

fn minor(first: f32, second: f32, subtract_first: f32, subtract_second: f32) -> f32 {
    // Native initial VMADDFP uses explicit +0, then VNMSUB negates the rounded FMA.
    -subtract_first.mul_add(subtract_second, -first.mul_add(second, 0.0))
}

/// Cofactors of the first omitted column; cyclic row selection preserves the
/// native permute1230 graph and alternating XOR signs.
fn cofactor(b: [f32; 4], c: [f32; 4], d: [f32; 4], negative_even: bool) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let next = (lane + 1) & 3;
        let middle = (lane + 2) & 3;
        let last = (lane + 3) & 3;
        let first = minor(c[middle], d[last], c[last], d[middle]);
        let second = minor(c[last], d[next], c[next], d[last]);
        let third = minor(c[next], d[middle], c[middle], d[next]);
        let sum = b[next] * first;
        let sum = b[middle].mul_add(second, sum);
        let sum = b[last].mul_add(third, sum);
        let flip = u32::from((lane & 1 == 0) == negative_even) * 0x8000_0000;
        f32::from_bits(sum.to_bits() ^ flip)
    })
}

/// No singular-matrix check exists in the source. The actual estimate policy is
/// supplied by the retained numeric service. This full inverse must not be
/// replaced with a transpose merely because an ordinary pose looks orthogonal.
pub(crate) fn try_inverse<N: PresentationMatrixNumeric>(
    matrix: AnimationMatrix,
    numeric: &mut N,
) -> Result<AnimationMatrix, N::Error> {
    let a = [matrix[0], matrix[1], matrix[2], matrix[3]];
    let b = [matrix[4], matrix[5], matrix[6], matrix[7]];
    let c = [matrix[8], matrix[9], matrix[10], matrix[11]];
    let d = [matrix[12], matrix[13], matrix[14], matrix[15]];
    let first = cofactor(b, c, d, false);
    // The native second row uses C*(D*A minors), not a reordered -A*(C*D).
    let second = cofactor(c, d, a, true);
    let determinant = numeric.dot4(a, first)?; //106c
    let seed = numeric.reciprocal_estimate(determinant)?; //1070
    let third = cofactor(d, a, b, false);
    let fourth = cofactor(a, b, c, true);
    let residual = -seed.mul_add(determinant, -1.0);
    let once = seed.mul_add(residual, seed);
    let residual = -once.mul_add(determinant, -1.0);
    let inverse = once.mul_add(residual, once);
    // Native merge-high/low transposes the cofactor columns before multiplying.
    Ok(std::array::from_fn(|index| {
        inverse * [first, second, third, fourth][index & 3][index / 4]
    }))
}

#[cfg(test)]
#[path = "skater_presentation_matrix_inverse_tests.rs"]
mod tests;
