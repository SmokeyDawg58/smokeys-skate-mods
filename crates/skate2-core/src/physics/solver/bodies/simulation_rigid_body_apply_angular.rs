//! New child of `target_angular`: distinct `ApplyAngularDisplacement82b36798`.
use super::{
    SimulationRigidBodyObservation, SimulationRigidBodyPool, TargetAngularError, cross_xyz,
    reciprocal_second,
};
use crate::{reckoning_numeric::ReckoningNumeric, skeleton_root_transforms::SkeletonRootVector};
pub(crate) type ApplyAngularError<N> = TargetAngularError<N>;
impl SimulationRigidBodyPool {
    /// Direct source displacement; no angular-velocity projection/normalization.
    pub(crate) fn try_apply_skateboard_angular_displacement<N: ReckoningNumeric>(
        &mut self,
        observation: SimulationRigidBodyObservation,
        requested: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), ApplyAngularError<N::Error>> {
        use TargetAngularError as E;
        let index = self.validate_observation(observation).map_err(E::Body)?;
        self.validate_integrity().map_err(E::Body)?;
        let slot = &mut self.slots[index];
        let dt = f32::from_bits(0x3c88_8889);
        let split = slot.inverse_inertia_split; //36800
        let old_x = slot.torque[0]; //3680c
        let full = slot.inverse_inertia_full; //36810
        let old_y = slot.torque[1]; //3681c
        let column_z = [full[2], split[2], split[0]];
        let column_y = [full[1], split[1], split[2]];
        let cofactor_x = cross_xyz(column_y, column_z); //36860
        let dt_seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?; //36868
        let cofactor_y = cross_xyz(column_z, full);
        let cofactor_z = cross_xyz(full, column_y);
        let dt_residual = -(dt_seed.mul_add(dt, -1.0)); //3687c
        let determinant = numeric
            .dot3(
                [full[0], full[1], full[2], 0.0],
                [cofactor_x[0], cofactor_x[1], cofactor_x[2], 0.0],
            )
            .map_err(E::Numeric)?;
        let final_split = split; //3688c retains original cached split vector
        let dt_first = dt_seed.mul_add(dt_residual, dt_seed); //368a8
        let determinant_seed = numeric
            .reciprocal_estimate(determinant)
            .map_err(E::Numeric)?; //368b4
        let dt_residual = -(dt_first.mul_add(dt, -1.0));
        let determinant_residual = -(determinant_seed.mul_add(determinant, -1.0));
        let inverse_dt = dt_first.mul_add(dt_residual, dt_first);
        let determinant_first = determinant_seed.mul_add(determinant_residual, determinant_seed);
        let inverse_determinant = reciprocal_second(determinant, determinant_first);
        let angular_rate = requested.xyz.map(|v| inverse_dt * v); //368e0
        let final_full = slot.inverse_inertia_full; //368e4 source reload
        let final_z = [final_full[2], final_split[2], final_split[0]];
        let old_z = slot.torque[2]; //368f0
        let final_y = [final_full[1], final_split[1], final_split[2]];
        let second_dt_seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?; //368f8
        let inverse_x =
            [cofactor_x[0], cofactor_y[0], cofactor_z[0]].map(|v| v * inverse_determinant);
        let inverse_y =
            [cofactor_x[1], cofactor_y[1], cofactor_z[1]].map(|v| v * inverse_determinant);
        let inverse_z =
            [cofactor_x[2], cofactor_y[2], cofactor_z[2]].map(|v| v * inverse_determinant);
        let second_dt_residual = -(second_dt_seed.mul_add(dt, -1.0)); //3691c
        let first = inverse_x.map(|v| v * angular_rate[0]);
        let second_dt_first = second_dt_seed.mul_add(second_dt_residual, second_dt_seed);
        let second: [f32; 3] =
            std::array::from_fn(|lane| inverse_y[lane].mul_add(angular_rate[1], first[lane]));
        let second_dt_residual = -(second_dt_first.mul_add(dt, -1.0));
        let transformed: [f32; 3] =
            std::array::from_fn(|lane| inverse_z[lane].mul_add(angular_rate[2], second[lane]));
        let second_inverse_dt = second_dt_first.mul_add(second_dt_residual, second_dt_first);
        let torque = transformed.map(|v| second_inverse_dt * v);
        let first = final_full.map(|v| v * torque[0]);
        let second: [f32; 3] =
            std::array::from_fn(|lane| final_y[lane].mul_add(torque[1], first[lane]));
        let acceleration: [f32; 3] =
            std::array::from_fn(|lane| final_z[lane].mul_add(torque[2], second[lane]));
        let new_z = old_z + acceleration[2]; //36964
        let new_y = old_y + acceleration[1];
        slot.torque[1] = new_y;
        let new_x = acceleration[0] + old_x;
        slot.torque[0] = new_x;
        slot.torque[2] = new_z;
        slot.cooldown = 0;
        Ok(())
    }
}
#[cfg(test)]
#[path = "simulation_rigid_body_apply_angular_tests.rs"]
mod tests;
