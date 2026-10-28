//! TargetAngularDisplacementExact82b36518: distinct from clamped targeting.
use super::*;
impl SimulationRigidBodyPool {
    pub(crate) fn try_target_skateboard_angular_displacement_exact<N: ReckoningNumeric>(
        &mut self,
        observation: SimulationRigidBodyObservation,
        requested: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), TargetAngularError<N::Error>> {
        use TargetAngularError as E;
        let index = self.validate_observation(observation).map_err(E::Body)?;
        self.validate_integrity().map_err(E::Body)?;
        let slot = &mut self.slots[index];
        let dt = f32::from_bits(0x3c888889);
        let motion = SkeletonRootVector {
            xyz: slot.omega.map(|v| v * dt),
            w: None,
        };
        let normalized = try_normalize_safe_with_refinement(
            requested,
            SkeletonRootVector::known([0.; 4]),
            numeric,
            sqrt_refinement,
        )
        .map_err(E::Numeric)?;
        let component = numeric.dot_vector(normalized, motion).map_err(E::Numeric)?;
        let projection = normalized.map(|v| v * component);
        let full = slot.inverse_inertia_full;
        let split = slot.inverse_inertia_split;
        let y = [full[1], split[1], split[2]];
        let z = [full[2], split[2], split[0]];
        let cofactor_x = cross_xyz(y, z);
        let cofactor_y = cross_xyz(z, full);
        let cofactor_z = cross_xyz(full, y);
        let determinant = numeric
            .dot3(
                [full[0], full[1], full[2], 0.],
                [cofactor_x[0], cofactor_x[1], cofactor_x[2], 0.],
            )
            .map_err(E::Numeric)?;
        // Exact issues determinant reciprocal before either DT reciprocal.
        let determinant_seed = numeric
            .reciprocal_estimate(determinant)
            .map_err(E::Numeric)?;
        let determinant_residual = -(determinant_seed.mul_add(determinant, -1.));
        let correction = requested.zip(projection, |a, b| a - b);
        let old = slot.torque;
        let final_full = slot.inverse_inertia_full;
        let final_split = slot.inverse_inertia_split;
        let determinant_first = determinant_seed.mul_add(determinant_residual, determinant_seed);
        let dt_seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
        let determinant_residual = -(determinant_first.mul_add(determinant, -1.));
        let dt_residual = -(dt_seed.mul_add(dt, -1.));
        let inverse_determinant =
            determinant_first.mul_add(determinant_residual, determinant_first);
        let second_dt_seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
        let dt_first = dt_seed.mul_add(dt_residual, dt_seed);
        let inverse_x =
            [cofactor_x[0], cofactor_y[0], cofactor_z[0]].map(|v| v * inverse_determinant);
        let second_dt_residual = -(second_dt_seed.mul_add(dt, -1.));
        let inverse_z =
            [cofactor_x[2], cofactor_y[2], cofactor_z[2]].map(|v| v * inverse_determinant);
        let dt_residual = -(dt_first.mul_add(dt, -1.));
        let second_dt_first = second_dt_seed.mul_add(second_dt_residual, second_dt_seed);
        let inverse_y =
            [cofactor_x[1], cofactor_y[1], cofactor_z[1]].map(|v| v * inverse_determinant);
        let inverse_dt = dt_first.mul_add(dt_residual, dt_first);
        let second_dt_residual = -(second_dt_first.mul_add(dt, -1.));
        let rate = correction.xyz.map(|v| inverse_dt * v);
        let second_inverse_dt = second_dt_first.mul_add(second_dt_residual, second_dt_first);
        let first = inverse_x.map(|v| v * rate[0]);
        let second: [f32; 3] = std::array::from_fn(|i| inverse_y[i].mul_add(rate[1], first[i]));
        let transformed: [f32; 3] =
            std::array::from_fn(|i| inverse_z[i].mul_add(rate[2], second[i]));
        let torque = transformed.map(|v| second_inverse_dt * v);
        let final_y = [final_full[1], final_split[1], final_split[2]];
        let final_z = [final_full[2], final_split[2], final_split[0]];
        let first = final_full.map(|v| v * torque[0]);
        let second: [f32; 3] = std::array::from_fn(|i| final_y[i].mul_add(torque[1], first[i]));
        let acceleration: [f32; 3] =
            std::array::from_fn(|i| final_z[i].mul_add(torque[2], second[i]));
        // 36768/6c/74: old+new for Z/Y; new+old for X, Y/X/Z store order.
        let z = old[2] + acceleration[2];
        let y = old[1] + acceleration[1];
        slot.torque[1] = y;
        slot.torque[0] = acceleration[0] + old[0];
        slot.torque[2] = z;
        slot.cooldown = 0;
        Ok(())
    }
}
