//! Numerical graphs consumed by `SkeletonCollision::Update` (0x82b4b8f0).
//! XYZ only: the response owner must retain unknown native packed W explicitly.
//! Dot reductions and estimate seeds are service boundaries; the authored scalar
//! products/FMA refinements do not claim Xenon denormal, NaN-payload, VNMSUB
//! exact-zero-sign or FP-status parity. The negative-multiplicand host FMA graph
//! is explicit; algebra alone does not prove native exceptional-result bits.
//! This is not generic normalized-vector or algebraic clamp replacement.
use crate::simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt};
use std::{convert::Infallible, fmt};

#[cfg(test)]
#[path = "skeleton_collision_numeric_tests.rs"]
mod tests;

/// Initialized by 0x82f763a0..0x82f763c8 from the literal at 0x8200fde0.
const NORMALIZE_EPSILON: f32 = f32::from_bits(0x3586_37bd);
/// `SMALL_FLOAT` loaded at 0x82e4948c from 0x82388ab0.
const CLAMP_SMALL_LENGTH: f32 = f32::from_bits(0x3780_0000);

pub(crate) trait SkeletonCollisionNumeric {
    type Error;
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, Self::Error>;
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
}

/// Explicit host policy: the shared `host_dot3` reduction, which flushes
/// denormals like VMX non-Java mode; estimates use ordinary host 1/sqrt(x) and
/// 1/x, with no finite-value repair. The subsequent source refinements still
/// run, even for exact host estimates.
pub(crate) struct HostSkeletonCollisionNumeric;
impl SkeletonCollisionNumeric for HostSkeletonCollisionNumeric {
    type Error = Infallible;
    fn dot3(&mut self, left: [f32; 3], right: [f32; 3]) -> Result<f32, Infallible> {
        let [lx, ly, lz] = left;
        let [rx, ry, rz] = right;
        Ok(crate::reckoning_numeric::host_dot3(
            [lx, ly, lz, 0.0],
            [rx, ry, rz, 0.0],
        ))
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Infallible> {
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Infallible> {
        Ok(1.0 / value)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SkeletonCollisionClampError<E> {
    /// Native FailAssert/DebugBreak can resume; this checked host helper stops
    /// before numeric callbacks instead of inventing a resumed assertion service.
    NegativeMaximum {
        bits: u32,
    },
    Numeric(E),
}
impl<E: fmt::Display> fmt::Display for SkeletonCollisionClampError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeMaximum { bits } => {
                write!(f, "collision clamp maximum is negative ({bits:08x})")
            }
            Self::Numeric(error) => write!(f, "collision clamp numeric service: {error}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for SkeletonCollisionClampError<E> {}

/// `Math::NormalizeSafe` 0x824b18e0..0x824b19b8. The comparison uses the
/// SUBTRACTION result, strictly >0; unordered chooses the unchanged fallback.
pub(crate) fn try_normalize_safe_xyz<N: SkeletonCollisionNumeric>(
    vector: [f32; 3],
    fallback: [f32; 3],
    numeric: &mut N,
) -> Result<[f32; 3], N::Error> {
    let squared = numeric.dot3(vector, vector)?;
    let epsilon_squared = NORMALIZE_EPSILON * NORMALIZE_EPSILON;
    let difference = squared - epsilon_squared;
    if difference > 0.0 {
        let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
        let inverse = refine_drive_reciprocal_sqrt(squared, estimate);
        Ok(vector.map(|component| component * inverse))
    } else {
        Ok(fallback)
    }
}

/// Source two-refinement finish followed by exact-zero selection. System speed
/// (0x82b4b940..0x82b4b9f0) and grouped slide (0x82b4c51c..0x82b4c598) place
/// state writes BETWEEN dot/estimate calls, so their caller invokes this finish
/// only after sampling those services at the original points.
pub(crate) fn length_from_squared_and_estimate(squared: f32, estimate: f32) -> f32 {
    let inverse = refine_drive_reciprocal_sqrt(squared, estimate);
    let product = squared * inverse;
    if squared == 0.0 { 0.0 } else { product }
}

/// Contiguous length graph for call sites with no intervening state stores.
/// Even zero squared length requests an estimate BEFORE selecting positive zero.
pub(crate) fn try_length_xyz<N: SkeletonCollisionNumeric>(
    vector: [f32; 3],
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let squared = numeric.dot3(vector, vector)?;
    let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
    Ok(length_from_squared_and_estimate(squared, estimate))
}

/// `ClampVectorWithinMaxLength` 0x82e49430..0x82e4957c. Source fsel selects
/// length when maximum-length >=0 (including -0), otherwise maximum; NaN takes
/// the latter arm. The two multiplications are NOT input*(selected/length).
pub(crate) fn try_clamp_xyz<N: SkeletonCollisionNumeric>(
    vector: [f32; 3],
    maximum: f32,
    numeric: &mut N,
) -> Result<[f32; 3], SkeletonCollisionClampError<N::Error>> {
    if maximum < 0.0 {
        return Err(SkeletonCollisionClampError::NegativeMaximum {
            bits: maximum.to_bits(),
        });
    }
    let length = try_length_xyz(vector, numeric).map_err(SkeletonCollisionClampError::Numeric)?;
    if length < CLAMP_SMALL_LENGTH {
        return Ok(vector);
    }
    let difference = maximum - length;
    let selected = if difference >= 0.0 { length } else { maximum };
    let estimate = numeric
        .reciprocal_estimate(length)
        .map_err(SkeletonCollisionClampError::Numeric)?;
    let scaled = vector.map(|component| component * selected);
    let inverse = refine_drive_reciprocal(length, estimate);
    Ok(scaled.map(|component| inverse * component))
}

/// Slide cross product 0x82b4c404..0x82b4c41c. Rounded a*perm(b), then
/// negative fused perm(a)*b subtraction, THEN YZX permutation. This fuses the
/// NEGATIVE product; fusing the positive product instead changes rounding.
pub(crate) fn cross_xyz(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|lane| {
        let j = (lane + 1) % 3;
        let k = (lane + 2) % 3;
        let positive = a[j] * b[k];
        (-a[k]).mul_add(b[j], positive)
    })
}
