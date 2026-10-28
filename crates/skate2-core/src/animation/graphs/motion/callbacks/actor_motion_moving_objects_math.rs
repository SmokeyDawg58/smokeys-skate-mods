//! Scalar/VMX graphs from MovingObject.Update82ca3a30 and ClampAngle82c8dd98.
use crate::{
    reckoning_numeric::{ReckoningNumeric, wrap_turns_to_radians},
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
};

pub(super) const PI: f32 = f32::from_bits(0x4049_0fdb);
pub(super) const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
pub(super) const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);

pub(super) fn angle_wrap(angle: f32) -> f32 {
    wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983))
}

fn sqrt_with_zero_selection<N: ReckoningNumeric>(
    squared: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
    let inverse = refine_drive_reciprocal_sqrt(squared, estimate);
    Ok(if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    })
}

/// Update82ca3c80..3e10: rounded XY products/addition, not a fused dot product.
/// Force opposite to movement contributes zero; perpendicular force is removed.
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native BLE/BLT do not take unordered branches"
)]
pub(super) fn speed_and_aligned_force<N: ReckoningNumeric>(
    movement: [f32; 2],
    force: [f32; 2],
    numeric: &mut N,
) -> Result<(f32, f32), N::Error> {
    let [x, z] = movement;
    let squared = (x * x) + (z * z);
    if squared <= f32::from_bits(0x38d1_b717) {
        return Ok((0.0, 0.0));
    }
    let speed = sqrt_with_zero_selection(squared, numeric)?;
    let dot = (force[0] * x) + (force[1] * z);
    let projection = dot / squared;
    let projected_x = x * projection;
    let projected_z = z * projection;
    let strength = if !(dot < 0.0) {
        sqrt_with_zero_selection(
            (projected_x * projected_x) + (projected_z * projected_z),
            numeric,
        )?
    } else {
        0.0
    };
    Ok((speed, strength))
}

/// Two source FSELs. Unlike f32::clamp, accepts inverted bounds and preserves
/// the source's unordered selections; no replacement tuning or sanitization.
pub(super) fn bound(value: f32, lower: f32, upper: f32) -> f32 {
    let selected = if lower - value >= 0.0 { lower } else { value };
    if upper - selected >= 0.0 {
        selected
    } else {
        upper
    }
}

pub(super) fn is_push(direction: u32) -> bool {
    matches!(direction, 1..=6)
}
pub(super) fn is_pull(direction: u32) -> bool {
    matches!(direction, 7..=12)
}
pub(super) fn is_left(direction: u32) -> bool {
    matches!(direction, 1 | 3 | 5 | 7 | 9 | 11)
}
pub(super) fn is_transition(direction: u32) -> bool {
    matches!(direction, 3..=6 | 9..=12)
}

/// ClampAngle82c8dd98: choose the closest CIRCULAR endpoint, ties choose end.
/// Raw bgelr preserves unordered->start, which HexRays renders as a wrong '<'.
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "raw ordered PPC branch layout"
)]
pub(super) fn clamp_angle(angle: f32, start: f32, end: f32) -> f32 {
    let inside = if !(start >= end) {
        !(angle < start) && angle <= end
    } else {
        angle >= start || angle <= end
    };
    if inside {
        return angle;
    }
    let to_start = f32::from_bits(angle_wrap(start - angle).to_bits() & 0x7fff_ffff);
    let to_end = f32::from_bits(angle_wrap(angle - end).to_bits() & 0x7fff_ffff);
    if to_start >= to_end { end } else { start }
}

/// Source direction families82ca441c..4680 and normalized angle82ca4688..4964.
/// Still retains its previous angle coefficient; it does not select a sector.
pub(super) fn directional_angle(direction: u32, full_angle: f32, previous: f32) -> (f32, f32) {
    if is_push(direction) {
        if is_left(direction) {
            let angle = clamp_angle(full_angle, -HALF_PI, 0.0);
            (angle, -angle / HALF_PI)
        } else {
            let angle = clamp_angle(full_angle, 0.0, HALF_PI);
            (angle, angle / HALF_PI)
        }
    } else if is_pull(direction) {
        if is_left(direction) {
            let angle = clamp_angle(full_angle, PI, -HALF_PI);
            (angle, angle_wrap(angle + PI) / HALF_PI)
        } else {
            let angle = clamp_angle(full_angle, HALF_PI, PI);
            (angle, angle_wrap(PI - angle) / HALF_PI)
        }
    } else {
        (full_angle, previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;

    #[test]
    fn strength_uses_force_projection_and_movement_gate() {
        for (movement, force, speed, strength) in [
            ([0.0, 2.0], [30.0, 4.0], 2.0, 4.0),
            ([0.0, 2.0], [30.0, -4.0], 2.0, 0.0),
            ([0.0, 0.0], [30.0, 4.0], 0.0, 0.0),
            ([3.0, 4.0], [6.0, 8.0], 5.0, 10.0),
        ] {
            let (actual_speed, actual_strength) =
                speed_and_aligned_force(movement, force, &mut HostReckoningNumeric).unwrap();
            assert!((actual_speed - speed).abs() < 0.00001);
            assert!((actual_strength - strength).abs() < 0.00001);
        }
    }

    #[test]
    fn sector_clamp_wraps_distances_and_retains_unordered_branch() {
        assert_eq!(clamp_angle(3.0, -HALF_PI, 0.0), -HALF_PI);
        assert_eq!(clamp_angle(-3.0, 0.0, HALF_PI), HALF_PI);
        assert_eq!(clamp_angle(0.0, PI, -HALF_PI), -HALF_PI);
        assert_eq!(clamp_angle(f32::NAN, -HALF_PI, 0.0), -HALF_PI);
        assert_eq!(clamp_angle(0.0, 0.0, 0.0), 0.0);
        assert_eq!(bound(f32::NAN, -1.0, 1.0), 1.0);
        assert_eq!(bound(0.0, 1.0, -1.0), -1.0);
    }

    #[test]
    fn all_authored_direction_families_use_original_sectors() {
        for direction in 1..=12 {
            let input = if is_push(direction) {
                if is_left(direction) { -0.5 } else { 0.5 }
            } else if is_left(direction) {
                -2.5
            } else {
                2.5
            };
            let (angle, coefficient) = directional_angle(direction, input, 0.9);
            assert_eq!(angle, input);
            assert!(coefficient > 0.0 && coefficient < 1.0);
        }
        assert_eq!(directional_angle(0, 0.7, 0.9), (0.7, 0.9));
    }
}
