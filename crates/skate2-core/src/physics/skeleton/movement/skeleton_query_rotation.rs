//! Affine `QueryRotate` (0x82b4b128), including its `ATan` and degenerate-axis
//! dependencies. Output stores retain their order even when numeric services
//! fail. This is source arithmetic, not a generic quaternion decomposition.
//! Numeric estimates/reductions, denormals, NaN payloads and negative fused-zero
//! signs remain host boundaries. Positive-length axis W is unwritten stack data.

use crate::{
    animation::AnimationMatrix,
    reckoning_numeric::{
        ReckoningNumeric, cross_full4, length_from_squared_and_estimate,
        normalize_from_squared_and_estimate,
    },
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector,
};

#[cfg(test)]
#[path = "skeleton_query_rotation_tests.rs"]
mod tests;

const ZERO: SkeletonRootVector = SkeletonRootVector {
    xyz: [0.0; 3],
    w: Some(0.0),
};
const PI: f32 = f32::from_bits(0x4049_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
// 0x82261edc (PI) +0x20, loaded with lvlx at 0x82b4b314.
const MINIMUM_RECIPROCAL: f32 = f32::from_bits(0x0020_0000);

/// `QueryRotate(Matrix44Affine const&,Vector3&,VecFloat&,Vector3&)`,
/// 0x82b4b128..0x82b4b414. Center is calculated even when its caller discards it.
pub(crate) fn try_query_rotate<N: ReckoningNumeric>(
    matrix: &AnimationMatrix,
    axis: &mut SkeletonRootVector,
    angle: &mut f32,
    center: &mut SkeletonRootVector,
    numeric: &mut N,
) -> Result<(), N::Error> {
    let raw_axis = [
        matrix[6] - matrix[9],
        matrix[8] - matrix[2],
        matrix[1] - matrix[4],
        0.0,
    ];
    // Only XYZ is a dot operand. The source fourth lane came from unwritten
    // stack memory and is separately represented below, not initialized here.
    let squared = numeric.dot3(raw_axis, raw_axis)?;
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    let length = length_from_squared_and_estimate(squared, seed);
    let trace_minus_one = ((matrix[0] + matrix[5]) + matrix[10]) - 1.0;
    if length > 0.0 {
        let seed = numeric.reciprocal_estimate(length)?;
        let inverse = refine_drive_reciprocal(length, seed) * 1.0;
        *axis = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| raw_axis[lane] * inverse),
            w: None,
        };
    } else {
        *axis = ZERO;
    }

    let seed = numeric.reciprocal_estimate(trace_minus_one)?;
    // This ratio uses ONE refinement, unlike the axis and center inverses.
    let residual = (-seed).mul_add(trace_minus_one, 1.0);
    let inverse = seed.mul_add(residual, seed);
    let ratio = length.mul_add(inverse, 0.0);
    let tangent_angle = try_atan(ratio, numeric)?;
    let length_sign = length.to_bits() & 0x8000_0000;
    let signed_pi = f32::from_bits(PI.to_bits() | length_sign);
    let signed_half_pi = f32::from_bits(HALF_PI.to_bits() | length_sign);
    let quadrant_angle = if 0.0 > trace_minus_one {
        signed_pi + tangent_angle
    } else {
        tangent_angle
    };
    *angle = if 0.0 == trace_minus_one {
        signed_half_pi
    } else {
        quadrant_angle
    };

    if MINIMUM_RECIPROCAL >= length && 0.0 >= trace_minus_one {
        // Only commit replacement axis after this helper's full normalization.
        *axis = try_degenerate_axis(matrix, numeric)?;
    }

    let translation = SkeletonRootVector {
        xyz: [matrix[12], matrix[13], matrix[14]],
        w: Some(matrix[15]),
    };
    let first_cross = cross(*axis, translation);
    let denominator = 2.0 - trace_minus_one;
    let seed = numeric.reciprocal_estimate(denominator)?;
    let second_cross = cross(first_cross, *axis);
    let inverse = refine_drive_reciprocal(denominator, seed);
    let scale = inverse * (0.5 * length);
    *center = SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            second_cross.xyz[lane].mul_add(0.5, first_cross.xyz[lane] * scale)
        }),
        w: first_cross
            .w
            .zip(second_cross.w)
            .map(|(first, second)| second.mul_add(0.5, first * scale)),
    };
    Ok(())
}

fn cross(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    let value = cross_full4(
        [a.xyz[0], a.xyz[1], a.xyz[2], a.w.unwrap_or(0.0)],
        [b.xyz[0], b.xyz[1], b.xyz[2], b.w.unwrap_or(0.0)],
    );
    SkeletonRootVector {
        xyz: [value[0], value[1], value[2]],
        w: a.w.zip(b.w).map(|_| value[3]),
    }
}

/// `QueryRotateDegenerateUnitAxis`, 0x82806b80..0x82806e24. All comparisons
/// are ordered strict greater. The selected principal term is actually SQUARED
/// (raw words at 6c08/6c9c/6d4c/6de0), not the usual textbook doubled term.
fn try_degenerate_axis<N: ReckoningNumeric>(
    matrix: &AnimationMatrix,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let candidate = if matrix[0] > matrix[5] {
        if matrix[0] > matrix[10] {
            let principal = 1.0 + matrix[0];
            [
                principal * principal,
                matrix[1] + matrix[4],
                matrix[2] + matrix[8],
            ]
        } else {
            z_candidate(matrix)
        }
    } else if matrix[5] > matrix[10] {
        let principal = 1.0 + matrix[5];
        [
            matrix[4] + matrix[1],
            principal * principal,
            matrix[6] + matrix[9],
        ]
    } else {
        z_candidate(matrix)
    };
    // VectorPermuteConstant<0,5,0,0> then insertion produces W=X.
    let full = [candidate[0], candidate[1], candidate[2], candidate[0]];
    let squared = numeric.dot3(full, full)?;
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    let result = normalize_from_squared_and_estimate(full, squared, seed);
    Ok(SkeletonRootVector {
        xyz: [result[0], result[1], result[2]],
        w: Some(result[3]),
    })
}
fn z_candidate(matrix: &AnimationMatrix) -> [f32; 3] {
    let principal = 1.0 + matrix[10];
    [
        matrix[8] + matrix[2],
        matrix[9] + matrix[6],
        principal * principal,
    ]
}

// Original tables ATanConstants0/1 at 0x8203e0c0/d0, 2 at f0, 3 at e0.
const ATAN_NUMERATOR: [f32; 4] = [
    f32::from_bits(0xc15b_0533),
    f32::from_bits(0xc1a4_0bfe),
    f32::from_bits(0xc107_e9fb),
    f32::from_bits(0xbf56_6bd7),
];
const ATAN_DENOMINATOR: [f32; 4] = [
    f32::from_bits(0x4224_43e6),
    f32::from_bits(0x42ac_5090),
    f32::from_bits(0x426e_5052),
    f32::from_bits(0x4170_624f),
];
const ATAN_REDUCTION: [f32; 4] = [
    f32::from_bits(0x3fdd_b3d7),
    f32::from_bits(0x3f3b_67af),
    f32::from_bits(0x3e89_30a3),
    f32::from_bits(0x3980_0000),
];
const ATAN_ANGLES: [f32; 4] = [
    HALF_PI,
    f32::from_bits(0x3f86_0a92),
    f32::from_bits(0x3f06_0a92),
    f32::from_bits(0x7e80_0000),
];

/// Scalar-splat lane of `XMVectorATan`, 0x82581658..0x82581808. All three
/// reciprocal estimates execute regardless of the subsequent vector selects.
pub(crate) fn try_atan<N: ReckoningNumeric>(input: f32, numeric: &mut N) -> Result<f32, N::Error> {
    let absolute = f32::from_bits(input.to_bits() & 0x7fff_ffff);
    let large = absolute > 1.0;
    let inverse = try_atan_reciprocal(absolute, numeric)?;
    let t = if large { inverse } else { absolute };
    let initial_offset = if large { ATAN_ANGLES[0] } else { 0.0 };
    let reduced_offset = if large {
        ATAN_ANGLES[1]
    } else {
        ATAN_ANGLES[2]
    };
    let denominator = t + ATAN_REDUCTION[0];
    let numerator_base = t.mul_add(ATAN_REDUCTION[1], t);
    let reduce = t > ATAN_REDUCTION[2];
    let inverse = try_atan_reciprocal(denominator, numeric)?;
    let numerator = numerator_base + -1.0;
    let offset = if reduce {
        reduced_offset
    } else {
        initial_offset
    };
    let reduced = if reduce { numerator * inverse } else { t };
    let square = reduced * reduced;
    let small = ATAN_REDUCTION[3] > f32::from_bits(reduced.to_bits() & 0x7fff_ffff);
    let denominator0 = square + ATAN_DENOMINATOR[3];
    let numerator0 = square.mul_add(ATAN_NUMERATOR[3], ATAN_NUMERATOR[2]);
    let denominator1 = square.mul_add(denominator0, ATAN_DENOMINATOR[2]);
    let numerator1 = square.mul_add(numerator0, ATAN_NUMERATOR[1]);
    let denominator2 = square.mul_add(denominator1, ATAN_DENOMINATOR[1]);
    let numerator2 = square.mul_add(numerator1, ATAN_NUMERATOR[0]);
    let denominator3 = square.mul_add(denominator2, ATAN_DENOMINATOR[0]);
    let numerator3 = numerator2 * square;
    let inverse = try_atan_reciprocal(denominator3, numeric)?;
    let quotient = numerator3 * inverse;
    let approximation = reduced.mul_add(quotient, reduced);
    let result = if small { reduced } else { approximation };
    let result = if large { flip_sign(result) } else { result };
    let result = result + offset;
    let result = if 0.0 > input {
        flip_sign(result)
    } else {
        result
    };
    let result = if input > ATAN_ANGLES[3] {
        ATAN_ANGLES[0]
    } else {
        result
    };
    Ok(if flip_sign(ATAN_ANGLES[3]) > input {
        flip_sign(ATAN_ANGLES[0])
    } else {
        result
    })
}

fn flip_sign(value: f32) -> f32 {
    f32::from_bits(value.to_bits() ^ 0x8000_0000)
}
fn try_atan_reciprocal<N: ReckoningNumeric>(input: f32, numeric: &mut N) -> Result<f32, N::Error> {
    let seed = numeric.reciprocal_estimate(input)?;
    let residual0 = (-input).mul_add(seed, 1.0);
    let first = seed.mul_add(residual0, seed);
    let residual1 = (-input).mul_add(first, 1.0);
    let second = first.mul_add(residual1, first);
    // Native checks FIRST refinement self-equality, not final result validity.
    Ok(if first.is_nan() { seed } else { second })
}
