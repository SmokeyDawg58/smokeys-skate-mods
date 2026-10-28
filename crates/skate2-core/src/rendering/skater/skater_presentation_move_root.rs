//! Actual replay SQT correction in `MoveRootToHips_Individual827d35d8`.
use super::corrections::CorrectionChannels;
use crate::{animation::AnimationMatrix, world::try_simulation_quaternion_candidate};
#[cfg(test)]
use skate2_formats::andale::AndaleSqt;

#[path = "skater_presentation_matrix_inverse.rs"]
pub(crate) mod matrix_inverse;
use matrix_inverse::{PresentationMatrixNumeric, try_inverse};

#[derive(Debug)]
pub(crate) enum MoveRootError<N> {
    MissingBone(usize),
    Numeric(N),
    Owner(super::ActorFaceError),
}
impl<N: std::fmt::Debug> std::fmt::Display for MoveRootError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBone(index) => write!(f, "MoveRootToHips missing replay bone {index}"),
            Self::Owner(error) => write!(f, "MoveRootToHips pose access: {error:?}"),
            Self::Numeric(error) => write!(f, "MoveRootToHips numeric service: {error:?}"),
        }
    }
}

/// Source constructs a rigid quaternion matrix and never loads SQT scale.
/// This differs numerically from the ACS products-before-doubling converter.
pub(crate) fn rigid_matrix(rotation: [f32; 4], translation: [f32; 4]) -> AnimationMatrix {
    let q = rotation.map(|value| f32::from_bits(0x3fb5_04f3) * value);
    let diagonal = q.map(|value| -value.mul_add(value, -0.5));
    let products = [q[0] * q[1], q[1] * q[2], q[2] * q[0]];
    let w_products = [q[3] * q[2], q[3] * q[0], q[3] * q[1]];
    let sums: [f32; 3] = std::array::from_fn(|lane| products[lane] + w_products[lane]);
    let differences: [f32; 3] = std::array::from_fn(|lane| products[lane] - w_products[lane]);
    [
        diagonal[1] + diagonal[2],
        sums[0],
        differences[2],
        0.0,
        differences[0],
        diagonal[2] + diagonal[0],
        sums[1],
        0.0,
        sums[2],
        differences[1],
        diagonal[0] + diagonal[1],
        0.0,
        translation[0],
        translation[1],
        translation[2],
        1.0,
    ]
}

pub(crate) fn multiply(left: AnimationMatrix, right: AnimationMatrix) -> AnimationMatrix {
    std::array::from_fn(|index| {
        let row = index & 3;
        let column = (index / 4) * 4;
        let first = left[row] * right[column];
        let second = left[4 + row].mul_add(right[column + 1], first);
        let third = left[8 + row].mul_add(right[column + 2], second);
        left[12 + row].mul_add(right[column + 3], third)
    })
}

/// Borrow the same synchronized replay[2*i] allocation for this mutation.
/// A later checked failure retains the source's earlier hips/reset writes.
/// Publication generations belong to the enclosing allocation mutation guard.
pub(crate) fn try_move_root_channels<N: PresentationMatrixNumeric>(
    replay: &mut impl CorrectionChannels,
    hips: usize,
    skateboard: usize,
    hips_transform: &mut Option<AnimationMatrix>,
    numeric: &mut N,
) -> Result<(), MoveRootError<N::Error>> {
    let hips_sqt = replay
        .replay(hips)
        .ok_or(MoveRootError::MissingBone(hips))?;
    *hips_transform = Some(rigid_matrix(hips_sqt.rotation, hips_sqt.translation)); //3690..3708
    replay
        .rotation(hips, [0.0, 0.0, 0.0, 1.0])
        .map_err(MoveRootError::Owner)?; //370c
    replay.scale(hips, [1.0; 4]).map_err(MoveRootError::Owner)?; //3710
    replay
        .translation(hips, [0.0, 0.0, 0.0, 1.0])
        .map_err(MoveRootError::Owner)?; //3714
    // Read only after the hips reset, including a checked fixture's alias case.
    let board = replay
        .replay(skateboard)
        .ok_or(MoveRootError::MissingBone(skateboard))?;
    let board_matrix = rigid_matrix(board.rotation, board.translation);
    let inverse = try_inverse(
        hips_transform.expect("hips transform already published"),
        numeric,
    )
    .map_err(MoveRootError::Numeric)?;
    replay
        .scale(skateboard, [1.0; 4])
        .map_err(MoveRootError::Owner)?; //3820
    let relative = multiply(inverse, board_matrix);
    let quaternion = try_simulation_quaternion_candidate(relative, |magnitudes| {
        let mut seeds = [0.0; 4];
        for lane in 0..4 {
            seeds[lane] = numeric.reciprocal_sqrt_estimate(magnitudes[lane])?;
        }
        Ok(seeds)
    })
    .map_err(MoveRootError::Numeric)?;
    replay
        .rotation(skateboard, quaternion)
        .map_err(MoveRootError::Owner)?; //39f4
    replay
        .translation(
            skateboard,
            [relative[12], relative[13], relative[14], relative[15]],
        )
        .map_err(MoveRootError::Owner)?; //39f8
    Ok(())
}

#[cfg(test)]
fn try_move_root_to_hips<N: PresentationMatrixNumeric>(
    replay: &mut [AndaleSqt],
    hips: usize,
    board: usize,
    transform: &mut AnimationMatrix,
    numeric: &mut N,
) -> Result<(), MoveRootError<N::Error>> {
    let mut output = Some(*transform);
    let result = try_move_root_channels(
        &mut super::corrections::SliceChannels { bind: &[], replay },
        hips,
        board,
        &mut output,
        numeric,
    );
    *transform = output.expect("existing fixture transform");
    result
}
#[cfg(test)]
#[path = "skater_presentation_move_root_tests.rs"]
mod tests;
