//! Actual `CalculateAngularVelocity`82b4d348 and torque application82b598a0.
use super::SkeletonNormalAssembly;
use crate::{
    dynamic_assembly_session::part_transform::DynamicAssemblyPartDisplacementError,
    reckoning_numeric::{
        ReckoningNumeric, cross_typed, length_from_squared_and_estimate, try_normalize_safe,
    },
    skateboard_assembly::SkateboardSimulationBackend,
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Debug)]
pub(crate) enum WipeoutTorqueError<N> {
    Numeric(N),
    Part(super::part_transform_query::SkeletonPartTransformQueryError),
    Apply(DynamicAssemblyPartDisplacementError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for WipeoutTorqueError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(e) => write!(f, "wipeout torque numeric: {e:?}"),
            Self::Part(e) => write!(f, "wipeout torque part: {e}"),
            Self::Apply(e) => write!(f, "wipeout torque apply: {e:?}"),
        }
    }
}

impl SkeletonNormalAssembly {
    pub(crate) fn try_calculate_wipeout_angular_velocity<N: ReckoningNumeric>(
        &self,
        numeric: &mut N,
    ) -> Result<SkeletonRootVector, N::Error> {
        self.physics_history
            .try_calculate_wipeout_angular_velocity(numeric)
    }
    pub(crate) fn try_apply_wipeout_torque<N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        torque: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), WipeoutTorqueError<N::Error>> {
        use WipeoutTorqueError as E;
        let squared = numeric.dot_vector(torque, torque).map_err(E::Numeric)?;
        // Skeleton+3d00: post-physics COM, distinct from the history used above.
        let com = self.post_physics_records.centre_of_mass_typed();
        let seed = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let magnitude = length_from_squared_and_estimate(squared, seed);
        for part in 1..24 {
            let transform = self.try_query_part_transform(part).map_err(E::Part)?;
            let radius = transform.columns()[3].zip(com, |a, b| a - b);
            let radius_squared = numeric.dot_vector(radius, radius).map_err(E::Numeric)?;
            if radius_squared > f32::from_bits(0x3a83_126f) {
                let cross = cross_typed(torque, radius);
                let direction = try_normalize_safe(cross, [0.0; 4], numeric).map_err(E::Numeric)?;
                let displacement = direction.map(|value| value * magnitude);
                self.session
                    .try_apply_part_displacement(
                        backend,
                        part,
                        [
                            displacement.xyz[0],
                            displacement.xyz[1],
                            displacement.xyz[2],
                            displacement.w.unwrap_or(0.0),
                        ],
                        numeric,
                    )
                    .map_err(E::Apply)?;
            }
        }
        Ok(())
    }
}
