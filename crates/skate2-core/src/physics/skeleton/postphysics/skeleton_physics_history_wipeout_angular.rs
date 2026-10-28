//! `SkeletonState::CalculateAngularVelocity`0x82b48668, on its retained record.
use super::SkeletonPhysicsHistory;
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_typed},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_physics_history::lane_knowledge::RecordVectorField,
    skeleton_root_transforms::SkeletonRootVector,
};

impl SkeletonPhysicsHistory {
    pub(crate) fn try_calculate_wipeout_angular_velocity<N: ReckoningNumeric>(
        &self,
        numeric: &mut N,
    ) -> Result<SkeletonRootVector, N::Error> {
        let com = self.record.vector_typed(RecordVectorField::CentreOfMass);
        let velocity = self
            .record
            .vector_typed(RecordVectorField::CentreOfMassVelocity);
        let mut result = SkeletonRootVector::known([0.0; 4]);
        for part in 1..24 {
            let radius = self
                .record
                .vector_typed(RecordVectorField::Position(part))
                .zip(com, |a, b| a - b);
            let delta = self
                .record
                .vector_typed(RecordVectorField::Velocity(part))
                .zip(velocity, |a, b| a - b);
            let squared = numeric.dot_vector(radius, radius)?;
            if squared > f32::from_bits(0x38d1_b717) {
                let seed = numeric.reciprocal_estimate(squared)?;
                let cross = cross_typed(radius, delta);
                let fraction = self.fractional_part_masses[part];
                let reciprocal = refine_drive_reciprocal(squared, seed);
                let angular = cross.map(|value| reciprocal * value);
                result = angular.zip(result, |value, old| value.mul_add(fraction, old));
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[test]
    fn actual_history_offsets_velocities_and_mass_fractions_feed_angular_result() {
        let mut history = SkeletonPhysicsHistory::new([1.0; 24]);
        history.fractional_part_masses[1] = 0.25;
        history.record.positions[1] = [2.0, 0.0, 0.0, 0.0];
        history.record.velocities[1] = [0.0, 4.0, 0.0, 0.0];
        let result = history
            .try_calculate_wipeout_angular_velocity(&mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(result.xyz.map(f32::to_bits), [0, 0, 0x3f00_0000]);
        history.record.store_vector_typed(
            RecordVectorField::Velocity(1),
            SkeletonRootVector {
                xyz: [0.0, 4.0, 0.0],
                w: None,
            },
        );
        let result = history
            .try_calculate_wipeout_angular_velocity(&mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(result.xyz.map(f32::to_bits), [0, 0, 0x3f00_0000]);
        assert_eq!(result.w, None);
    }
}
