//! `SkeletonCollision::CalculateNormalDisplacement`, 0x82b3e960..0x82b3ec78.
//! Actual retained normals feed the projected displacement and angular envelope.
//! Source scalar/vector operation dependencies and output-store prefixes are
//! retained. Numeric estimates, dot reductions, denormals, NaN payloads, float
//! rounding and negative fused-subtraction zero signs remain host boundaries.

use std::cmp::Ordering;

use crate::{
    reckoning_numeric::{ReckoningNumeric, try_normalize_safe, try_signed_angle},
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_cosine},
    skeleton_collision_data::{SkeletonCollisionData, SkeletonCollisionVector},
    skeleton_root_transforms::SkeletonRootVector,
};

#[cfg(test)]
#[path = "skeleton_normal_displacement_tests.rs"]
mod tests;

const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const INVERSE_TWO_PI: f32 = f32::from_bits(0x3e22_f983);
const MINIMUM_NORMALIZED_SQUARED: f32 = f32::from_bits(0x3dcc_cccd);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonNormalDisplacementError<N, R> {
    Numeric(N),
    Rounding(R),
    /// A corrupt native count reaches a row outside the retained allocation.
    /// Checked only when that row would be loaded, after the projection store.
    NormalUnavailable {
        index: i32,
    },
}

impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for SkeletonNormalDisplacementError<N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "normal displacement numeric operation: {error:?}"),
            Self::Rounding(error) => {
                write!(f, "normal displacement trigonometric rounding: {error:?}")
            }
            Self::NormalUnavailable { index } => {
                write!(f, "collision normal row {index} unavailable")
            }
        }
    }
}

impl<N: std::fmt::Debug, R: std::fmt::Debug> std::error::Error
    for SkeletonNormalDisplacementError<N, R>
{
}

impl SkeletonCollisionData {
    /// 0x82b3e960: output receives the original plane projection before the
    /// first normalization. Later refusal preserves that prefix. Collision
    /// data is borrowed from its real owner, never a caller-provided row list.
    pub(crate) fn try_calculate_normal_displacement<
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &self,
        displacement: SkeletonRootVector,
        ground_normal: SkeletonRootVector,
        output: &mut SkeletonRootVector,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), SkeletonNormalDisplacementError<N::Error, R::Error>> {
        *output = try_project(displacement, ground_normal, numeric)
            .map_err(SkeletonNormalDisplacementError::Numeric)?;
        let direction = try_normalize_safe(scratch(*output), [0.0; 4], numeric)
            .map_err(SkeletonNormalDisplacementError::Numeric)?;
        let direction_squared = numeric
            .dot3(direction, direction)
            .map_err(SkeletonNormalDisplacementError::Numeric)?;
        if MINIMUM_NORMALIZED_SQUARED > direction_squared {
            return Ok(());
        }

        let mut minimum = HALF_PI;
        let mut maximum = -HALF_PI;
        let mut found = false;
        let mut index = 0_i32;
        while index < self.num_normals {
            let row = usize::try_from(index)
                .ok()
                .and_then(|index| self.normal_info.get(index))
                .ok_or(SkeletonNormalDisplacementError::NormalUnavailable { index })?;
            let projected = try_project(from_collision(row.normal), ground_normal, numeric)
                .map_err(SkeletonNormalDisplacementError::Numeric)?;
            let normal = try_normalize_safe(scratch(projected), [0.0; 4], numeric)
                .map_err(SkeletonNormalDisplacementError::Numeric)?;
            let normal_squared = numeric
                .dot3(normal, normal)
                .map_err(SkeletonNormalDisplacementError::Numeric)?;
            if MINIMUM_NORMALIZED_SQUARED > normal_squared {
                index += 1;
                continue;
            }
            let angle = try_signed_angle(direction, normal, scratch(ground_normal), numeric)
                .map_err(SkeletonNormalDisplacementError::Numeric)?;
            let wrapped = wrap_angle(angle);
            if retain_angle(wrapped, &mut minimum, &mut maximum) {
                found = true;
            }
            index += 1;
        }

        let sum = maximum + minimum;
        let middle = sum * 0.5;
        let factor = if found {
            try_cosine(middle, rounding).map_err(SkeletonNormalDisplacementError::Rounding)?
        } else {
            0.0
        };
        // Native still calls the rotation helper when no normal was retained.
        // Both rounding operations, and all zero multiplications, must execute.
        let rotation = try_rotated_identity(ground_normal.xyz, middle, rounding)
            .map_err(SkeletonNormalDisplacementError::Rounding)?;
        let rotated = transform_vector(rotation, output.xyz);
        let result = rotated.map(|lane| lane * factor);
        *output = SkeletonRootVector {
            xyz: [result[0], result[1], result[2]],
            // The source rotation constructs all lanes from axis XYZ alone.
            // This recovers W from actual arithmetic, not homogeneous zero.
            w: Some(result[3]),
        };
        Ok(())
    }
}

fn from_collision(vector: SkeletonCollisionVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: *vector.xyz(),
        w: vector.known_lanes().map(|lanes| lanes[3]),
    }
}

/// Only dot3, scalar angle calculation and discarded normalized scratch lanes
/// consume this W. Unknown W never becomes a published, allegedly known lane.
fn scratch(vector: SkeletonRootVector) -> [f32; 4] {
    [
        vector.xyz[0],
        vector.xyz[1],
        vector.xyz[2],
        vector.w.unwrap_or(0.0),
    ]
}

fn try_project<N: ReckoningNumeric>(
    value: SkeletonRootVector,
    normal: SkeletonRootVector,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let dot = numeric.dot3(scratch(value), scratch(normal))?;
    Ok(SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            let product = normal.xyz[lane] * dot;
            value.xyz[lane] - product
        }),
        w: value.w.zip(normal.w).map(|(value, normal)| {
            let product = normal * dot;
            value - product
        }),
    })
}

/// SIMD fractional-turn wrap at 0x82b3eae0..0x82b3eb08. Floor returns a float;
/// this is neither integer conversion nor the scalar `Math::Angle` normalizer.
fn wrap_angle(angle: f32) -> f32 {
    let turns = angle * INVERSE_TWO_PI;
    let fractional = turns - turns.floor();
    let subtractor = if fractional > 0.5 { 1.0 } else { 0.0 };
    let signed_fractional = fractional - subtractor;
    signed_fractional * TWO_PI
}

fn retain_angle(angle: f32, minimum: &mut f32, maximum: &mut f32) -> bool {
    // Raw 0x4099002c/0x40980020 branch when CR6.GT/LT is CLEAR. PPC BLE/BGE
    // therefore include unordered: both strict interval tests must be ordered.
    if angle.partial_cmp(&-HALF_PI) != Some(Ordering::Greater)
        || angle.partial_cmp(&HALF_PI) != Some(Ordering::Less)
    {
        return false;
    }
    if angle < *minimum {
        *minimum = angle;
    }
    if angle > *maximum {
        *maximum = angle;
    }
    true
}

// Standalone XMVectorSin/Cos use the same original tables as XMVectorSinCos,
// but DIFFERENT rounded power dependencies. Keep these narrowly scoped graphs.
// g_XMSinCoefficients0/1/2: 0x823cfe10/20/30, excluding the leading 1.
const SINE: [u32; 11] = [
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
fn try_reduced_angle<R: ReckoningSinCosNumeric>(
    angle: f32,
    rounding: &mut R,
) -> Result<f32, R::Error> {
    let turns = angle * INVERSE_TWO_PI;
    let nearest = rounding.round_nearest_even(turns)?;
    Ok((-TWO_PI).mul_add(nearest, angle))
}

/// Scalar-splat `XMVectorSin`, 0x824b0060..0x824b0130.
pub(crate) fn try_sine<R: ReckoningSinCosNumeric>(
    angle: f32,
    rounding: &mut R,
) -> Result<f32, R::Error> {
    let x = try_reduced_angle(angle, rounding)?;
    let x2 = x * x;
    let mut power = x2 * x;
    let mut result = x;
    for (index, coefficient) in SINE.into_iter().enumerate() {
        result = f32::from_bits(coefficient).mul_add(power, result);
        if index + 1 < SINE.len() {
            power *= x2;
        }
    }
    Ok(result)
}

/// `Math::RotateAroundArbitraryAxis`, 0x82647348..0x826474bc, specialized to
/// this caller's identity matrix without dropping its zero-product/FMA graph.
/// Only axis XYZ are splatted. The permute at 0x82fd6e90 selects [0,5,0,0],
/// followed by Z-only inserts: each temporary column W duplicates its X.
pub(crate) fn try_rotated_identity<R: ReckoningSinCosNumeric>(
    axis: [f32; 3],
    angle: f32,
    rounding: &mut R,
) -> Result<[[f32; 4]; 3], R::Error> {
    let cosine = try_cosine(angle, rounding)?;
    let sine = try_sine(angle, rounding)?;
    let [x, y, z] = axis;
    let sin_z = sine * z;
    let sin_y = sine * y;
    let sin_x = sine * x;
    let complement = 1.0 - cosine;
    let weighted_x = complement * x;
    let weighted_y = complement * y;
    let weighted_z = complement * z;
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
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    Ok(identity.map(|column| transform_vector(rotation, column)))
}

fn transform_vector(columns: [[f32; 4]; 3], value: [f32; 3]) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let x = columns[0][lane] * value[0];
        let xy = columns[1][lane].mul_add(value[1], x);
        columns[2][lane].mul_add(value[2], xy)
    })
}
