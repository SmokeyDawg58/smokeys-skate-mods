//! Walking PhysicsUtility leaves82e491e0/82e49e90 and their exact children.
use crate::{
    physical_player_footplant::RetainedTrajectory,
    physical_player_trajectory_owners::offboard_runtime::{greatest_plane_time, position_at},
    reckoning_numeric::{
        ReckoningNumeric, cross_typed, length_from_squared_and_estimate, try_length,
        try_normalize_safe,
    },
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector as V,
};
const UP: V = V::known([0.0, 1.0, 0.0, 0.0]);
const ZERO: V = V::known([0.0; 4]);
const DT: f32 = f32::from_bits(0x3c88_8889);

///82e48fb8: normalize only above Math::VECFLOAT_SMALL, then clamp projected length.
fn closest_point<N: ReckoningNumeric>(
    position: V,
    first: V,
    second: V,
    numeric: &mut N,
) -> Result<V, N::Error> {
    let mut direction = second.zip(first, |b, a| b - a);
    let length = try_length(direction, numeric)?;
    if length > f32::from_bits(0x3780_0000) {
        let seed = numeric.reciprocal_estimate(length)?;
        let inverse = refine_drive_reciprocal(length, seed);
        direction = direction.map(|v| inverse * v);
    }
    let projection = numeric.dot_vector(direction, position.zip(first, |p, a| p - a))?;
    // Existing host VMX min/max boundary; no exceptional-value parity claim.
    let distance = projection.min(length).max(0.0);
    Ok(direction.zip(first, |d, a| d.mul_add(distance, a)))
}

///82e49080: stop at the first non-decreasing sample, not a global minimizer.
fn distance_to_point<N: ReckoningNumeric>(
    trajectory: &RetainedTrajectory,
    point: V,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let mut time = 0.0_f32;
    let mut best = f32::from_bits(0x4974_2400);
    while time < 2.0 {
        let delta = point.zip(position_at(trajectory, time), |p, t| p - t);
        let squared = numeric.dot_vector(delta, delta)?;
        if !(best > squared) {
            break;
        }
        time += DT;
        best = squared;
    }
    let seed = numeric.reciprocal_sqrt_estimate(best)?;
    Ok(length_from_squared_and_estimate(best, seed))
}

///82e491e0. Position commits before the distance child's potentially failing reads.
pub(crate) fn calc_dist_from_trajectory_to_line<N: ReckoningNumeric>(
    trajectory: &RetainedTrajectory,
    first: &V,
    second: &V,
    position_out: &mut V,
    distance_out: &mut f32,
    numeric: &mut N,
) -> Result<(), N::Error> {
    let difference = second.zip(*first, |b, a| b - a);
    let side = cross_typed(difference, UP);
    // Native computes this unused normalized plane normal before its next child.
    let _normal = try_normalize_safe(cross_typed(side, difference), UP, numeric)?;
    let normal = try_normalize_safe(cross_typed(trajectory.velocity, UP), ZERO, numeric)?;
    let a = numeric.dot_vector(first.zip(trajectory.position, |p, t| p - t), normal)?;
    let b = numeric.dot_vector(second.zip(trajectory.position, |p, t| p - t), normal)?;
    let abs_a = f32::from_bits(a.to_bits() & 0x7fff_ffff);
    let abs_b = f32::from_bits(b.to_bits() & 0x7fff_ffff);
    let chosen = if 0.0 > a * b {
        let sum = abs_a + abs_b;
        let weighted_second = second.map(|p| p * abs_a);
        let weighted_first = first.map(|p| p * abs_b);
        let first_seed = numeric.reciprocal_estimate(sum)?;
        let second_seed = numeric.reciprocal_estimate(sum)?;
        let first_inverse = refine_drive_reciprocal(sum, first_seed);
        let second_inverse = refine_drive_reciprocal(sum, second_seed);
        weighted_first.zip(weighted_second.map(|p| second_inverse * p), |p, q| {
            first_inverse.mul_add(p, q)
        })
    } else if abs_b > abs_a {
        *first
    } else {
        *second
    };
    *position_out = chosen;
    *distance_out = distance_to_point(trajectory, chosen, numeric)?;
    Ok(())
}

///82e49e90. Native never writes posOnPlane (the sixth argument).
pub(crate) fn calc_dist_from_trajectory_to_ground_plane<N: ReckoningNumeric>(
    trajectory: &RetainedTrajectory,
    ground: &V,
    first: &V,
    second: &V,
    position_out: &mut V,
    _plane_out: &mut V,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let mut time = [0.0; 4];
    if !greatest_plane_time(trajectory, *ground, UP, &mut time, numeric)? {
        return Ok(false);
    }
    let point = position_at(trajectory, time[0]);
    *position_out = closest_point(point, *first, *second, numeric)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[test]
    fn opposite_sides_interpolate_and_ties_select_second_endpoint() {
        let t = RetainedTrajectory {
            velocity: V::known([0.0, 0.0, 1.0, 0.0]),
            ..RetainedTrajectory::constructed()
        };
        let mut out = ZERO;
        let mut distance = -1.0;
        calc_dist_from_trajectory_to_line(
            &t,
            &V::known([-1.0, 0.0, 1.0, 2.0]),
            &V::known([1.0, 0.0, 1.0, 4.0]),
            &mut out,
            &mut distance,
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(out.known_lanes(), Some([0.0, 0.0, 1.0, 3.0]));
        assert!(distance < 0.02);
        let stationary = RetainedTrajectory::constructed();
        let second = V {
            xyz: [2.0, 0.0, 0.0],
            w: None,
        };
        calc_dist_from_trajectory_to_line(
            &stationary,
            &ZERO,
            &second,
            &mut out,
            &mut distance,
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(out, second);
        assert_eq!(distance, 2.0);
    }
    #[test]
    fn plane_miss_preserves_outputs_and_success_keeps_unused_output() {
        let mut t = RetainedTrajectory {
            position: V::known([0.0, 1.0, 0.0, 1.0]),
            gravity: V::known([0.0, -2.0, 0.0, 0.0]),
            ..RetainedTrajectory::constructed()
        };
        let mut out = UP;
        let mut untouched = V::known([7.0; 4]);
        assert!(
            calc_dist_from_trajectory_to_ground_plane(
                &t,
                &ZERO,
                &V::known([-1.0, 0.0, 0.0, 1.0]),
                &V::known([1.0, 0.0, 0.0, 1.0]),
                &mut out,
                &mut untouched,
                &mut HostReckoningNumeric
            )
            .unwrap()
        );
        assert_eq!(out.known_lanes(), Some([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(untouched, V::known([7.0; 4]));
        t.position = V::known([0.0, -1.0, 0.0, 1.0]);
        let previous = out;
        assert!(
            !calc_dist_from_trajectory_to_ground_plane(
                &t,
                &ZERO,
                &ZERO,
                &UP,
                &mut out,
                &mut untouched,
                &mut HostReckoningNumeric
            )
            .unwrap()
        );
        assert_eq!(out, previous);
        assert_eq!(untouched, V::known([7.0; 4]));
    }
}

#[cfg(test)]
mod publication_test {
    use super::*;
    use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric};
    struct FailDistance {
        dots: usize,
    }
    impl ReckoningNumeric for FailDistance {
        type Error = ();
        fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, ()> {
            self.dots += 1;
            if self.dots == 5 {
                Err(())
            } else {
                Ok(HostReckoningNumeric.dot3(a, b).unwrap())
            }
        }
        fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, ()> {
            Ok(HostReckoningNumeric.reciprocal_sqrt_estimate(v).unwrap())
        }
        fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, ()> {
            Ok(HostReckoningNumeric.reciprocal_estimate(v).unwrap())
        }
        fn truncate_i32(&mut self, v: f32) -> Result<i32, ()> {
            HostReckoningNumeric.truncate_i32(v).map_err(|_| ())
        }
    }
    #[test]
    fn selected_line_position_precedes_fallible_distance_publication() {
        let mut position = UP;
        let mut distance = 123.0;
        let endpoint = V::known([2.0, 0.0, 0.0, 1.0]);
        assert!(
            calc_dist_from_trajectory_to_line(
                &RetainedTrajectory::constructed(),
                &ZERO,
                &endpoint,
                &mut position,
                &mut distance,
                &mut FailDistance { dots: 0 }
            )
            .is_err()
        );
        assert_eq!(position, endpoint);
        assert_eq!(distance, 123.0);
    }
}
