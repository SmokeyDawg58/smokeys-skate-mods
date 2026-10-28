//! Native camera angle graphs; these differ from scalar `Math::Angle::Normalize`.
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_asin, try_length, try_normalize_angle},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_query_rotation::try_atan,
    skeleton_root_transforms::SkeletonRootVector,
};

pub(super) const PI: f32 = f32::from_bits(0x4049_0fdb);
pub(super) const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

/// Native FSEL lower/upper clamps map an unordered input to one.
pub(crate) fn unit_fraction(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

/// `Iris::CameraAngle::Normalize82e31eb8`: floating VRFIZ, not FCTIWZ.
pub(crate) fn camera_angle<N: ReckoningNumeric>(
    angle: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let seed = numeric.reciprocal_estimate(TWO_PI)?;
    let inverse = refine_drive_reciprocal(TWO_PI, seed);
    Ok(camera_angle_with_inverse(angle, inverse))
}

/// Shared remainder/FSEL graph; the caller preserves reciprocal issue timing.
pub(crate) fn camera_angle_with_inverse(angle: f32, inverse: f32) -> f32 {
    let turns = (inverse * angle).trunc();
    let remainder = (-turns).mul_add(TWO_PI, angle);
    let lower = if remainder + PI >= 0.0 {
        remainder
    } else {
        remainder + TWO_PI
    };
    if remainder - PI >= 0.0 {
        remainder - TWO_PI
    } else {
        lower
    }
}

/// `HeadingFromDirection82e34368`; even a zero Z reaches the `ATan` producer.
pub(super) fn heading<N: ReckoningNumeric>(
    direction: SkeletonRootVector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    if direction.xyz[1] > f32::from_bits(0x3f7f_f2e5) {
        return Ok(0.0);
    }
    camera_atan2_pending(-direction.xyz[0], direction.xyz[2], numeric)
        .map(CameraAtan2Pending::finish)
}

/// Camera-owned ratio/ATan result; `EulerXYZFromMatrix33` completes all native
/// atan producers before it issues the quadrant additions.
pub(crate) struct CameraAtan2Pending {
    numerator: f32,
    denominator: f32,
    angle: f32,
}
impl CameraAtan2Pending {
    pub(crate) fn finish(self) -> f32 {
        let sign = self.numerator.to_bits() & 0x8000_0000;
        let adjusted = f32::from_bits(PI.to_bits() | sign) + self.angle;
        let quadrant = if 0.0 > self.denominator {
            adjusted
        } else {
            self.angle
        };
        if self.denominator == 0.0 {
            f32::from_bits(0x3fc9_0fdb | sign)
        } else {
            quadrant
        }
    }
}

/// One-refinement ratio followed by the existing native `XMVectorATan` owner.
pub(crate) fn camera_atan2_pending<N: ReckoningNumeric>(
    numerator: f32,
    denominator: f32,
    numeric: &mut N,
) -> Result<CameraAtan2Pending, N::Error> {
    let seed = numeric.reciprocal_estimate(denominator)?;
    let residual = (-seed).mul_add(denominator, 1.0);
    let inverse = seed.mul_add(residual, seed);
    let angle = try_atan(numerator.mul_add(inverse, 0.0), numeric)?;
    Ok(CameraAtan2Pending {
        numerator,
        denominator,
        angle,
    })
}

/// Complete scalar calls of `EulerXYZFromNormalizedVector825f3e20`, with roll0.
/// Callers consume only Y, but the later pitch `ASin` still executes before return.
pub(super) fn euler_heading<N: ReckoningNumeric>(
    direction: SkeletonRootVector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    euler_angles(direction, numeric).map(|(_, heading)| heading)
}

/// Same complete native result, now consumed by the actual Rig orientation.
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native unordered comparison follows the same early-return branch"
)]
pub(crate) fn euler_angles<N: ReckoningNumeric>(
    direction: SkeletonRootVector,
    numeric: &mut N,
) -> Result<(f32, f32), N::Error> {
    let horizontal = [direction.xyz[0], 0.0, direction.xyz[2], 0.0];
    let length = try_length(horizontal, numeric)?;
    if !(length > f32::from_bits(0x38d1_b717)) {
        return Ok((0.0, 0.0));
    }
    let seed = numeric.reciprocal_estimate(length)?;
    let inverse = refine_drive_reciprocal(length, seed);
    let x = inverse * horizontal[0];
    let z = inverse * horizontal[2];
    let (argument, offset) = if x >= 0.0 {
        if z >= 0.0 {
            (x, 0.0)
        } else {
            (-z, f32::from_bits(0x3fc9_0fdb))
        }
    } else if 0.0 >= z {
        (-x, PI)
    } else {
        (z, f32::from_bits(0x4096_cbe4))
    };
    // Same exceptional host policy as physical_player_heading_runtime: preserve
    // unordered input, rather than host max()/min() eliminating NaN. Native
    // NaN payload selection and signed-zero exceptional bits remain a boundary.
    let angle = try_asin(signed_unit(argument), numeric)?;
    let result = try_normalize_angle(angle + offset, numeric)?;
    let pitch = try_asin(signed_unit(direction.xyz[1]), numeric)?;
    Ok((pitch, result))
}

fn signed_unit(value: f32) -> f32 {
    if value.is_nan() {
        value
    } else {
        value.clamp(-1.0, 1.0)
    }
}

/// `CameraAngle.InterpolateTo82e341a0`; the first store survives a later
/// normalization failure. All callers retain the same fresh numeric calls.
pub(crate) fn interpolate_angle<N: ReckoningNumeric>(
    destination: &mut f32,
    target: f32,
    amount: f32,
    numeric: &mut N,
) -> Result<(), N::Error> {
    let delta = camera_angle(target - *destination, numeric)?;
    *destination = delta.mul_add(amount, *destination);
    *destination = camera_angle(*destination, numeric)?;
    Ok(())
}
