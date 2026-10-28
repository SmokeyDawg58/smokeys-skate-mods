//! Matrix33.OrthoNormalize82b24f88 and Math.RotationVectorFromMatrix33829b0a50.
//! The former uses squared-length admission, unlike the existing Matrix44
//! OrthoNormalize3x3. Shared estimates, cross product, acos and sine stay shared.
use super::{ReckoningNumeric, cross_full4, try_acos, try_normalize_unsafe};
use crate::{
    reckoning_sin_cos::{ReckoningSinCosNumeric, sine::try_sine},
    simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt},
};
pub(crate) type Matrix33 = [[f32; 4]; 3];

pub(crate) fn try_orthonormalize<N: ReckoningNumeric>(
    source: Matrix33,
    numeric: &mut N,
) -> Result<Matrix33, N::Error> {
    let squared = [
        numeric.dot3(source[0], source[0])?,
        numeric.dot3(source[1], source[1])?,
        numeric.dot3(source[2], source[2])?,
    ];
    // The source repeats Y/Z dots for its admission vector before estimates.
    let admitted = [
        squared[0],
        numeric.dot3(source[1], source[1])?,
        numeric.dot3(source[2], source[2])?,
    ];
    let seeds = [
        numeric.reciprocal_sqrt_estimate(squared[0])?,
        numeric.reciprocal_sqrt_estimate(squared[1])?,
        numeric.reciprocal_sqrt_estimate(squared[2])?,
    ];
    let mut axes: Matrix33 = std::array::from_fn(|i| {
        let inverse = refine_drive_reciprocal_sqrt(squared[i], seeds[i]);
        source[i].map(|lane| lane * inverse)
    });
    let (a, b, missing) = if !(admitted[0] > 0.) {
        (1, 2, 0)
    } else if !(admitted[1] > 0.) {
        (2, 0, 1)
    } else if admitted[2] > 0. {
        // Raw dot order Z.X, Y.Z, X.Y; strict ties prefer X/Y then Z/X.
        let zx = numeric.dot3(axes[2], axes[0])?.abs();
        let yz = numeric.dot3(axes[1], axes[2])?.abs();
        let xy = numeric.dot3(axes[0], axes[1])?.abs();
        if zx > yz {
            if xy > yz { (1, 2, 0) } else { (0, 1, 2) }
        } else if xy > zx {
            (2, 0, 1)
        } else {
            (0, 1, 2)
        }
    } else {
        (0, 1, 2)
    };
    axes[missing] = try_normalize_unsafe(cross_full4(axes[a], axes[b]), numeric)?;
    axes[b] = try_normalize_unsafe(cross_full4(axes[missing], axes[a]), numeric)?;
    Ok(axes)
}
#[derive(Debug)]
pub(crate) enum RotationVectorError<N, T> {
    Numeric(N),
    Trigonometry(T),
}
pub(crate) fn try_rotation_vector<N: ReckoningNumeric, T: ReckoningSinCosNumeric>(
    matrix: Matrix33,
    numeric: &mut N,
    trig: &mut T,
) -> Result<[f32; 4], RotationVectorError<N::Error, T::Error>> {
    let trace = (matrix[0][0] + matrix[1][1]) + matrix[2][2];
    let angle = try_acos(0.5 * (trace - 1.), numeric).map_err(RotationVectorError::Numeric)?;
    // Source does not clamp acos input or add a special pi-axis branch.
    if !(angle > 0.) {
        return Ok([0.; 4]);
    }
    let sine = try_sine(angle, trig).map_err(RotationVectorError::Trigonometry)?;
    let seed = numeric
        .reciprocal_estimate(sine)
        .map_err(RotationVectorError::Numeric)?;
    let inverse = refine_drive_reciprocal(sine, seed);
    let half_angle = 0.5 * angle;
    let differences = [
        matrix[1][2] - matrix[2][1],
        matrix[2][0] - matrix[0][2],
        matrix[0][1] - matrix[1][0],
    ];
    let result = differences.map(|v| 0. + ((v * half_angle) * inverse));
    // Final permute<0,5,0,0> + insertZ preserves replicated X in W.
    Ok([result[0], result[1], result[2], result[0]])
}
