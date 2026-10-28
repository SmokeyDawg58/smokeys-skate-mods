//! Landing's affine interpolation, `PhysicsUtility::InterpolateMatrix44Affine`
//! 0x82e4acf8..0x82e4ad7c and affine `SLerp`0x82b4b418..0x82b4b86c.
//! This follows matrix-axis interpolation, not quaternion slerp. Source dot/
//! estimate ordering and full W arithmetic survive; existing numeric/FP host
//! boundaries apply. Private output temporaries publish only completed matrices.

use crate::{
    animation::AnimationMatrix,
    reckoning_numeric::{ReckoningNumeric, normalize_from_squared_and_estimate},
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_sin_cos},
    skeleton_query_rotation::try_query_rotate,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[cfg(test)]
#[path = "skeleton_affine_interpolation_tests.rs"]
mod tests;

const MINIMUM_SLERP_ANGLE: f32 = f32::from_bits(0x3d0e_fa35); // 0x82261ef4

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonAffineInterpolationError<N, R> {
    Numeric(N),
    Trigonometry(R),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for SkeletonAffineInterpolationError<N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "affine interpolation numeric operation: {error:?}"),
            Self::Trigonometry(error) => write!(f, "affine interpolation trigonometry: {error:?}"),
        }
    }
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::error::Error
    for SkeletonAffineInterpolationError<N, R>
{
}

/// 0x82e4acf8: translation is computed BEFORE `SLerp`, then overwrites its P.
/// Rotational endpoint gates do not clamp this full-four-lane extrapolation.
pub(crate) fn try_interpolate_affine<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    from: &AnimationMatrix,
    to: &AnimationMatrix,
    blend: f32,
    numeric: &mut N,
    trigonometry: &mut R,
) -> Result<AnimationMatrix, SkeletonAffineInterpolationError<N::Error, R::Error>> {
    let complement = 1.0 - blend;
    let position: [f32; 4] = std::array::from_fn(|lane| {
        let destination = to[12 + lane] * blend;
        from[12 + lane].mul_add(complement, destination)
    });
    let mut result = try_slerp(from, to, blend, numeric, trigonometry)?.matrix;
    result[12..16].copy_from_slice(&position);
    Ok(result)
}

#[derive(Clone, Copy)]
enum RotationPath {
    From,
    To,
    Linear,
    Rodrigues,
}
struct SlerpResult {
    matrix: AnimationMatrix,
    remaining_angle: f32,
    path: RotationPath,
}

/// Typed affine wrapper82e4acf8: weighted position still executes at endpoints.
pub(crate) fn try_interpolate_affine_typed<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    from: &SkeletonRootMatrix,
    to: &SkeletonRootMatrix,
    blend: f32,
    numeric: &mut N,
    trigonometry: &mut R,
) -> Result<SkeletonRootMatrix, SkeletonAffineInterpolationError<N::Error, R::Error>> {
    let complement = 1.0 - blend;
    let old = from.columns()[3];
    let new = to.columns()[3];
    let position = SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            let destination = new.xyz[lane] * blend;
            old.xyz[lane].mul_add(complement, destination)
        }),
        w: old.w.zip(new.w).map(|(old, new)| {
            let destination = new * blend;
            old.mul_add(complement, destination)
        }),
    };
    let (result, _) = try_slerp_typed(from, to, blend, numeric, trigonometry)?;
    let mut columns = *result.columns();
    columns[3] = position;
    Ok(SkeletonRootMatrix::from_columns(columns))
}

/// Affine interpolation82b4b418, including the output residual angle. Only XYZ drives
/// its control flow and query services. Scratch W never becomes retained data:
/// endpoint copies, linear dependencies and derived Rodrigues W are distinct.
pub(crate) fn try_slerp_typed<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    from: &SkeletonRootMatrix,
    to: &SkeletonRootMatrix,
    blend: f32,
    numeric: &mut N,
    trigonometry: &mut R,
) -> Result<(SkeletonRootMatrix, f32), SkeletonAffineInterpolationError<N::Error, R::Error>> {
    let result = try_slerp(
        &from.xyz_scratch(),
        &to.xyz_scratch(),
        blend,
        numeric,
        trigonometry,
    )?;
    let mut columns = *SkeletonRootMatrix::known(result.matrix).columns();
    for (index, column) in columns.iter_mut().enumerate() {
        let known = match result.path {
            RotationPath::From => from.columns()[index].w.is_some(),
            RotationPath::To => to.columns()[index].w.is_some(),
            RotationPath::Linear => {
                from.columns()[index].w.is_some() && to.columns()[index].w.is_some()
            }
            RotationPath::Rodrigues => {
                index < 3 || (from.columns()[index].w.is_some() && to.columns()[index].w.is_some())
            }
        };
        if !known {
            column.w = None;
        }
    }
    Ok((
        SkeletonRootMatrix::from_columns(columns),
        result.remaining_angle,
    ))
}

/// Fallible typed classical rotation normalization82b22c78. The input P is
/// preserved; IK base blending explicitly supplies P0 at its call site.
pub(crate) fn try_normalize_rotation<N: ReckoningNumeric>(
    input: &SkeletonRootMatrix,
    numeric: &mut N,
) -> Result<SkeletonRootMatrix, N::Error> {
    try_normalize_rotation_with_refinement(
        input,
        numeric,
        crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    )
}

pub(crate) fn try_normalize_rotation_with_refinement<N: ReckoningNumeric>(
    input: &SkeletonRootMatrix,
    numeric: &mut N,
    refinement: impl Fn(f32, f32) -> f32,
) -> Result<SkeletonRootMatrix, N::Error> {
    let mut output = *input.columns();
    for axis in (0..3).rev() {
        let original = input.columns()[axis];
        let mut candidate = original;
        for previous in ((axis + 1)..3).rev() {
            let basis = output[previous];
            let projection = numeric.dot3(vector_scratch(basis), vector_scratch(original))?;
            for lane in 0..3 {
                let product = basis.xyz[lane] * projection;
                candidate.xyz[lane] -= product;
            }
            candidate.w = candidate.w.zip(basis.w).map(|(old, basis)| {
                let product = basis * projection;
                old - product
            });
        }
        let squared = numeric.dot3(vector_scratch(candidate), vector_scratch(candidate))?;
        let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
        let inverse = refinement(squared, estimate);
        output[axis] = SkeletonRootVector {
            xyz: candidate.xyz.map(|v| v * inverse),
            w: candidate.w.map(|v| v * inverse),
        };
    }
    Ok(SkeletonRootMatrix::from_columns(output))
}
fn vector_scratch(value: SkeletonRootVector) -> [f32; 4] {
    [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ]
}

fn try_slerp<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    from: &AnimationMatrix,
    to: &AnimationMatrix,
    blend: f32,
    numeric: &mut N,
    trigonometry: &mut R,
) -> Result<SlerpResult, SkeletonAffineInterpolationError<N::Error, R::Error>> {
    use SkeletonAffineInterpolationError as Error;
    // Native vector ordered GE: NaN takes neither endpoint gate.
    if blend >= 1.0 {
        return Ok(SlerpResult {
            matrix: *to,
            remaining_angle: 0.0,
            path: RotationPath::To,
        });
    }
    let relative = relative_rotation(from, to);
    let mut axis = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    let mut angle = 0.0;
    let mut center = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    // Center is unused by SLerp, but the full query, including its final
    // reciprocal service, executes even when blend is zero or negative.
    try_query_rotate(&relative, &mut axis, &mut angle, &mut center, numeric)
        .map_err(Error::Numeric)?;
    if 0.0 >= blend {
        return Ok(SlerpResult {
            matrix: *from,
            remaining_angle: angle,
            path: RotationPath::From,
        });
    }
    let (matrix, path) = if MINIMUM_SLERP_ANGLE > angle {
        (
            try_linear_normalized(from, to, blend, numeric).map_err(Error::Numeric)?,
            RotationPath::Linear,
        )
    } else {
        let scaled_angle = angle * blend;
        let (sine, cosine) =
            try_sin_cos(scaled_angle, trigonometry).map_err(Error::Trigonometry)?;
        (
            rotate_from_axes(from, to, blend, axis.xyz, sine, cosine),
            RotationPath::Rodrigues,
        )
    };
    let used_angle = angle * blend;
    Ok(SlerpResult {
        matrix,
        remaining_angle: angle - used_angle,
        path,
    })
}

/// To is the outer matrix; the inner basis is the transposed From rotation.
/// Its position is ZERO, not the position of From's rigid inverse. The source
/// still executes all three zero-coefficient translation FMAs for `QueryRotate`.
fn relative_rotation(from: &AnimationMatrix, to: &AnimationMatrix) -> AnimationMatrix {
    std::array::from_fn(|index| {
        let column = index / 4;
        let lane = index % 4;
        if column == 3 {
            let x = 0.0_f32.mul_add(to[lane], to[12 + lane]);
            let xy = 0.0_f32.mul_add(to[4 + lane], x);
            0.0_f32.mul_add(to[8 + lane], xy)
        } else {
            let x = from[column] * to[lane];
            let xy = from[4 + column].mul_add(to[4 + lane], x);
            from[8 + column].mul_add(to[8 + lane], xy)
        }
    })
}

/// 0x82b4b628: independent axis normalization, with ALL three dots preceding
/// ALL three estimates. This is not the existing classical Gram-Schmidt graph.
fn try_linear_normalized<N: ReckoningNumeric>(
    from: &AnimationMatrix,
    to: &AnimationMatrix,
    blend: f32,
    numeric: &mut N,
) -> Result<AnimationMatrix, N::Error> {
    let mut result: AnimationMatrix = std::array::from_fn(|index| {
        let difference = to[index] - from[index];
        difference.mul_add(blend, from[index])
    });
    let axes: [[f32; 4]; 3] =
        std::array::from_fn(|axis| std::array::from_fn(|lane| result[axis * 4 + lane]));
    let mut squared = [0.0; 3];
    for axis in 0..3 {
        squared[axis] = numeric.dot3(axes[axis], axes[axis])?;
    }
    let mut estimates = [0.0; 3];
    for axis in 0..3 {
        estimates[axis] = numeric.reciprocal_sqrt_estimate(squared[axis])?;
    }
    for axis in 0..3 {
        let normalized =
            normalize_from_squared_and_estimate(axes[axis], squared[axis], estimates[axis]);
        result[axis * 4..axis * 4 + 4].copy_from_slice(&normalized);
    }
    Ok(result)
}

/// 0x82b4b728: Rodrigues columns assembled by [0,5,0,0] permutes and Z-only
/// inserts, hence temporary W=X. Only axis XYZ and From basis XYZ are read;
/// `QueryRotate`'s unresolved positive-length axis W is never homogenized.
fn rotate_from_axes(
    from: &AnimationMatrix,
    to: &AnimationMatrix,
    blend: f32,
    axis: [f32; 3],
    sine: f32,
    cosine: f32,
) -> AnimationMatrix {
    let [x, y, z] = axis;
    let complement = 1.0 - cosine;
    let weighted_x = complement * x;
    let weighted_y = complement * y;
    let weighted_z = complement * z;
    let sin_z = sine * z;
    let sin_y = sine * y;
    let sin_x = sine * x;
    let xx = weighted_x.mul_add(x, cosine);
    let xy = weighted_x.mul_add(y, sin_z);
    let xz = weighted_x * z - sin_y;
    let yx = weighted_y * x - sin_z;
    let yy = weighted_y.mul_add(y, cosine);
    let yz = weighted_y.mul_add(z, sin_x);
    let zx = weighted_z.mul_add(x, sin_y);
    let zy = weighted_z * y - sin_x;
    let zz = weighted_z.mul_add(z, cosine);
    let rotation = [[xx, xy, xz, xx], [yx, yy, yz, yx], [zx, zy, zz, zx]];
    std::array::from_fn(|index| {
        let column = index / 4;
        let lane = index % 4;
        if column == 3 {
            let difference = to[index] - from[index];
            blend.mul_add(difference, from[index])
        } else {
            let coefficient = column * 4;
            let x = from[coefficient] * rotation[0][lane];
            let xy = from[coefficient + 1].mul_add(rotation[1][lane], x);
            from[coefficient + 2].mul_add(rotation[2][lane], xy)
        }
    })
}
