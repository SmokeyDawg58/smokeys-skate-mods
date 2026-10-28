//! The vector math the presentation restores blend with: the XNA Math sine, cosine and
//! arc cosine the original links (`XMVectorSin` 0x824b0060, `XMVectorCos` 0x824b0130,
//! `XMVectorACos` 0x824b0278), lane by lane, RenderWare's quaternion slerp as
//! `CameraFrame::Lerp` 0x829d3978, `SkaterPresentation::UpdateSkater` 0x827d9e10 and
//! `cPresVehicle::UpdateFromReplayData` 0x82807e88 inline it, and its matrix slerp.

use crate::{
    animation::AnimationMatrix,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    skeleton_query_rotation::try_query_rotate,
    skeleton_root_transforms::SkeletonRootVector,
};

/// `g_XMPiConstants0`: pi, two pi, one over pi, one over two pi.
const PI_CONSTANTS: [f32; 4] = [
    f32::from_bits(0x4049_0fdb),
    f32::from_bits(0x40c9_0fdb),
    f32::from_bits(0x3ea2_f983),
    f32::from_bits(0x3e22_f983),
];
/// `g_XMSinCoefficients0..2`.
const SIN_COEFFICIENTS: [u32; 12] = [
    0x3f80_0000,
    0xbe2a_aaab,
    0x3c08_8889,
    0xb950_0d01,
    0x3638_ef1d,
    0xb2d7_322b,
    0x2f30_9231,
    0xab57_3f9f,
    0x274a_963c,
    0xa317_a4da,
    0x1eb8_dc78,
    0x9a3b_0da1,
];
/// `g_XMCosCoefficients0..2`.
const COS_COEFFICIENTS: [u32; 12] = [
    0x3f80_0000,
    0xbf00_0000,
    0x3d2a_aaab,
    0xbab6_0b61,
    0x37d0_0d01,
    0xb493_f27e,
    0x310f_76c8,
    0xad49_cba5,
    0x2957_3f9f,
    0xa534_13c3,
    0x20f2_a15d,
    0x9c86_71cb,
];
/// `g_XMASinCoefficients0..2`.
const ASIN_COEFFICIENTS: [u32; 12] = [
    0xbd6d_d42d,
    0xbed6_5553,
    0x3e66_3246,
    0x400b_1889,
    0x3f1d_d7b6,
    0x4089_80bd,
    0xbf98_3f2f,
    0xc0d1_360e,
    0xbfaf_4418,
    0xc08f_6ad9,
    0x3fb5_8485,
    0x40af_6ad8,
];
/// `XMVectorACos`'s `OnePlusEpsilon`.
const ONE_PLUS_EPSILON: f32 = f32::from_bits(0x3f80_0001);
/// `VECTOR_MIN_SLERP_ANGLE`: five degrees, below which the slerp normalizes a lerp.
const MIN_SLERP_ANGLE: f32 = f32::from_bits(0x3db2_b8c3);
/// `MATRIX_MIN_SLERP_ANGLE`: two degrees, below which the matrix slerp normalizes a lerp.
const MATRIX_MIN_SLERP_ANGLE: f32 = f32::from_bits(0x3d0e_fa35);

fn coefficient(table: &[u32; 12], index: usize) -> f32 {
    f32::from_bits(table[index])
}

/// `vnmsubfp`: `b - a * c`, rounded once.
fn negative_multiply_subtract(a: f32, c: f32, b: f32) -> f32 {
    (-a).mul_add(c, b)
}

/// `XMVectorModAngles`' reduction into one turn about zero.
fn mod_angle(value: f32) -> f32 {
    let turns = (value * PI_CONSTANTS[3]).round_ties_even();
    negative_multiply_subtract(PI_CONSTANTS[1], turns, value)
}

/// `XMVectorSin` 0x824b0060.
pub fn xm_sin(value: f32) -> f32 {
    let v1 = mod_angle(value);
    let v2 = v1 * v1;
    let mut power = v2 * v1;
    let mut result = coefficient(&SIN_COEFFICIENTS, 1).mul_add(power, v1);
    for index in 2..12 {
        power *= v2;
        result = coefficient(&SIN_COEFFICIENTS, index).mul_add(power, result);
    }
    result
}

/// `XMVectorCos` 0x824b0130.
pub fn xm_cos(value: f32) -> f32 {
    let v1 = mod_angle(value);
    let v2 = v1 * v1;
    let v4 = v2 * v2;
    let v6 = v4 * v2;
    let v8 = v4 * v4;
    let v10 = v6 * v4;
    let v12 = v6 * v6;
    let v14 = v8 * v6;
    let v16 = v8 * v8;
    let v18 = v10 * v8;
    let v20 = v10 * v10;
    let v22 = v12 * v10;
    let powers = [v2, v4, v6, v8, v10, v12, v14, v16, v18, v20, v22];
    let mut result = 1.0;
    for (index, power) in (1..12).zip(powers) {
        result = coefficient(&COS_COEFFICIENTS, index).mul_add(power, result);
    }
    result
}

/// `XMVectorACos` 0x824b0278.
pub fn xm_acos(value: f32) -> f32 {
    let absolute = f32::from_bits(value.to_bits() & 0x7fff_ffff);
    let square = value * value;
    let remainder = ONE_PLUS_EPSILON - absolute;
    let cube = square * absolute;
    let odd = negative_multiply_subtract(absolute, value, value);
    let first: [f32; 4] = std::array::from_fn(|lane| {
        coefficient(&ASIN_COEFFICIENTS, lane)
            .mul_add(absolute, coefficient(&ASIN_COEFFICIENTS, 4 + lane))
    });
    let second: [f32; 4] = std::array::from_fn(|lane| {
        first[lane].mul_add(absolute, coefficient(&ASIN_COEFFICIENTS, 8 + lane))
    });
    let estimate = 1.0 / remainder.sqrt();
    let half_remainder = remainder * 0.5;
    let half_pi = PI_CONSTANTS[0] * 0.5;
    let high = second[2].mul_add(cube, second[3]);
    let low = second[0].mul_add(cube, second[1]);
    let residual = negative_multiply_subtract(half_remainder, estimate * estimate, 0.5);
    let scaled_high = value * high;
    let scaled_low = odd * low;
    let root = estimate.mul_add(residual, estimate);
    half_pi - scaled_low.mul_add(root, scaled_high)
}

fn dot4(a: [f32; 4], b: [f32; 4]) -> f32 {
    crate::reckoning_numeric::host_dot4(a, b)
}

/// `vrsqrtefp` and the two Newton steps that refine it.
fn reciprocal_root(value: f32) -> f32 {
    let mut estimate = 1.0 / value.sqrt();
    for _ in 0..2 {
        let square = estimate * estimate;
        let half = estimate * 0.5;
        let residual = negative_multiply_subtract(value, square, 1.0);
        estimate = half.mul_add(residual, estimate);
    }
    estimate
}

/// `vrefp` and one Newton step.
fn reciprocal(value: f32) -> f32 {
    let estimate = 1.0 / value;
    estimate.mul_add(negative_multiply_subtract(estimate, value, 1.0), estimate)
}

/// RenderWare's quaternion slerp: the shorter arc, a normalized lerp within five
/// degrees, otherwise the sine-weighted blend.
pub fn slerp(from: [f32; 4], to: [f32; 4], amount: f32) -> [f32; 4] {
    let mut from = from;
    let mut dot = dot4(from, to);
    if 0.0 > dot {
        from = from.map(|lane| f32::from_bits(lane.to_bits() ^ 0x8000_0000));
        dot = f32::from_bits(dot.to_bits() ^ 0x8000_0000);
    }
    if dot > xm_cos(MIN_SLERP_ANGLE) {
        let lerp: [f32; 4] =
            std::array::from_fn(|lane| (to[lane] - from[lane]).mul_add(amount, from[lane]));
        let opposite: [f32; 4] = std::array::from_fn(|lane| {
            negative_multiply_subtract(to[lane] + from[lane], amount, from[lane])
        });
        let chosen = if dot4(from, to) > 0.0 { lerp } else { opposite };
        let scale = reciprocal_root(dot4(chosen, chosen));
        return chosen.map(|lane| lane * scale);
    }
    let angle = xm_acos(dot);
    let sine = xm_sin(angle);
    let first = reciprocal(sine);
    let second = reciprocal(sine);
    let to_sine = xm_sin(amount * angle);
    let second = second.mul_add(negative_multiply_subtract(second, sine, 1.0), second);
    let to_weight = second * to_sine;
    let from_sine = xm_sin((1.0 - amount) * angle);
    let first = first.mul_add(negative_multiply_subtract(first, sine, 1.0), first);
    let from_weight = first * from_sine;
    std::array::from_fn(|lane| from[lane].mul_add(from_weight, to[lane] * to_weight))
}

/// RenderWare's matrix slerp, `SLerp(Matrix44)` 0x82808ff8, as the living world's restore
/// blends its entities. `QueryRotate` reads the axis and angle of `from`'s transpose times
/// `to` over all four rows, so the translations' products enter the rotation it reads.
/// Below two degrees the rows lerp and each normalizes; otherwise `from`'s rows turn about
/// the axis by `amount` of the angle. The translation row lerps.
///
/// # Errors
/// `QueryRotate`'s numeric services.
pub(crate) fn matrix_slerp(
    from: &AnimationMatrix,
    to: &AnimationMatrix,
    amount: f32,
) -> Result<AnimationMatrix, ReckoningHostNumericError> {
    if amount >= 1.0 {
        return Ok(*to);
    }
    let relative: AnimationMatrix = std::array::from_fn(|index| {
        let (row, lane) = (index / 4, index % 4);
        let x = from[row] * to[lane];
        let xy = from[4 + row].mul_add(to[4 + lane], x);
        let xyz = from[8 + row].mul_add(to[8 + lane], xy);
        from[12 + row].mul_add(to[12 + lane], xyz)
    });
    let unset = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    let (mut axis, mut angle, mut center) = (unset, 0.0, unset);
    try_query_rotate(
        &relative,
        &mut axis,
        &mut angle,
        &mut center,
        &mut HostReckoningNumeric,
    )?;
    if 0.0 >= amount {
        return Ok(*from);
    }
    let mut result = *from;
    for index in 12..16 {
        result[index] = amount.mul_add(to[index] - from[index], from[index]);
    }
    if MATRIX_MIN_SLERP_ANGLE > angle {
        for row in 0..3 {
            let lerped: [f32; 4] = std::array::from_fn(|lane| {
                let index = row * 4 + lane;
                amount.mul_add(to[index] - from[index], from[index])
            });
            let scale = reciprocal_root(dot4(lerped, lerped));
            for (lane, value) in lerped.into_iter().enumerate() {
                result[row * 4 + lane] = value * scale;
            }
        }
        return Ok(result);
    }
    let (sine, cosine) = try_sin_cos(angle * amount, &mut HostReckoningSinCosNumeric)
        .unwrap_or_else(|never| match never {});
    let [x, y, z] = axis.xyz;
    let complement = 1.0 - cosine;
    let (weighted_x, weighted_y, weighted_z) = (complement * x, complement * y, complement * z);
    let (sin_x, sin_y, sin_z) = (sine * x, sine * y, sine * z);
    let rotation = [
        [
            weighted_x.mul_add(x, cosine),
            weighted_x.mul_add(y, sin_z),
            weighted_x * z - sin_y,
            0.0,
        ],
        [
            weighted_y * x - sin_z,
            weighted_y.mul_add(y, cosine),
            weighted_y.mul_add(z, sin_x),
            0.0,
        ],
        [
            weighted_z.mul_add(x, sin_y),
            weighted_z * y - sin_x,
            weighted_z.mul_add(z, cosine),
            0.0,
        ],
        [0.0, 0.0, 0.0, 1.0],
    ];
    for row in 0..3 {
        let basis: [f32; 4] = std::array::from_fn(|lane| from[row * 4 + lane]);
        for lane in 0..4 {
            let first = rotation[0][lane] * basis[0];
            let second = rotation[1][lane].mul_add(basis[1], first);
            let third = rotation[2][lane].mul_add(basis[2], second);
            result[row * 4 + lane] = rotation[3][lane].mul_add(basis[3], third);
        }
    }
    Ok(result)
}

#[cfg(test)]
#[path = "presentation_restore_math_tests.rs"]
mod tests;
