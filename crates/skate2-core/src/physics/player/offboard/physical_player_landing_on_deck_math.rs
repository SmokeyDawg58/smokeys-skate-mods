//! Numeric children of LandingOnDeckManager, decoded from raw VMX operands.
//! These preserve the existing fallible numeric service and unknown vector W.
use crate::{
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector as V,
};

/// FindTrajTimeWithSmallestStartingSpeed82dc8de8..82dc8ed0.
/// Native uses literal9.8, not the current world's gravity magnitude.
pub(crate) fn smallest_starting_speed_time<N: ReckoningNumeric>(
    distance: V,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let squared = numeric.dot_vector(distance, distance)?;
    let length_seed = numeric.reciprocal_sqrt_estimate(squared)?;
    let gravity_seed = numeric.reciprocal_estimate(f32::from_bits(0x411c_cccd))?;
    let inverse_gravity = refine_drive_reciprocal(f32::from_bits(0x411c_cccd), gravity_seed);
    let length = length_from_squared_and_estimate(squared, length_seed);
    let doubled = 2.0 * length;
    let time_squared = inverse_gravity * doubled;
    let time_seed = numeric.reciprocal_sqrt_estimate(time_squared)?;
    Ok(length_from_squared_and_estimate(time_squared, time_seed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;

    #[test]
    fn optimum_uses_distance_magnitude_without_consuming_unknown_w() {
        let mut numeric = HostReckoningNumeric;
        let known = V::known([3.0, 4.0, 0.0, 999.0]);
        let unknown = V {
            xyz: known.xyz,
            w: None,
        };
        let a = smallest_starting_speed_time(known, &mut numeric).unwrap();
        let b = smallest_starting_speed_time(unknown, &mut numeric).unwrap();
        assert_eq!(a.to_bits(), b.to_bits());
        assert!((a - (10.0_f32 / 9.8).sqrt()).abs() < 0.000_001);
        assert_eq!(
            smallest_starting_speed_time(V::known([0.0; 4]), &mut numeric)
                .unwrap()
                .to_bits(),
            0.0_f32.to_bits()
        );
    }
}
