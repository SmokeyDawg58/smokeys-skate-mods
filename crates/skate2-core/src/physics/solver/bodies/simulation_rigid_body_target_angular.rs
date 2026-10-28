//! `SkateboardBody::TargetAngularDisplacement82b361e0..36510`.
//! Unregistered pool child; consumes the same actual deck occurrence.
use super::{
    SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_full4, try_normalize_safe_with_refinement},
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Debug)]
pub(crate) enum TargetAngularError<N> {
    Body(SimulationRigidBodyPoolError),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for TargetAngularError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Body(e) => e.fmt(f),
            Self::Numeric(e) => write!(f, "target angular numeric: {e:?}"),
        }
    }
}

fn sqrt_refinement(squared: f32, seed: f32) -> f32 {
    let square = seed * seed;
    let residual = -(squared.mul_add(square, -1.0));
    let refined = (seed * 0.5).mul_add(residual, seed);
    let square = refined * refined;
    let residual = -(squared.mul_add(square, -1.0));
    (refined * 0.5).mul_add(residual, refined)
}
fn reciprocal_first(value: f32, seed: f32) -> f32 {
    let residual = -(seed.mul_add(value, -1.0));
    seed.mul_add(residual, seed)
}
fn reciprocal_second(value: f32, first: f32) -> f32 {
    let residual = -(first.mul_add(value, -1.0));
    first.mul_add(residual, first)
}
fn cross_xyz(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    // Source YZXW permutations cannot route native adjacent tensor words to
    // XYZ. This unobserved scratch W is never published as a retained lane.
    let result = cross_full4([a[0], a[1], a[2], 0.0], [b[0], b[1], b[2], 0.0]);
    [result[0], result[1], result[2]]
}

impl SimulationRigidBodyPool {
    /// No torque or cooldown store occurs until all numerical operations pass.
    /// The two fixed-DT divisions and tensor/inverse-tensor products are kept
    /// separately; cancelling them changes the native numerical graph.
    pub(crate) fn try_target_skateboard_angular_displacement<N: ReckoningNumeric>(
        &mut self,
        observation: SimulationRigidBodyObservation,
        requested: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), TargetAngularError<N::Error>> {
        use TargetAngularError as E;
        let index = self.validate_observation(observation).map_err(E::Body)?;
        self.validate_integrity().map_err(E::Body)?;
        let slot = &mut self.slots[index];
        let dt = f32::from_bits(0x3c88_8889); //kUpdateTime@8225c488
        let angular_motion = SkeletonRootVector {
            xyz: slot.omega.map(|v| v * dt),
            w: None,
        };
        let zero = SkeletonRootVector::known([0.0; 4]);
        let normalized =
            try_normalize_safe_with_refinement(requested, zero, numeric, sqrt_refinement)
                .map_err(E::Numeric)?;
        let component = numeric
            .dot_vector(normalized, angular_motion)
            .map_err(E::Numeric)?;
        let projection = normalized.map(|v| v * component);
        let correction = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| requested.xyz[lane] - projection.xyz[lane]),
            w: requested.w.zip(projection.w).map(|(a, b)| a - b),
        };
        let correction_dot = numeric
            .dot_vector(correction, requested)
            .map_err(E::Numeric)?;
        let corrected = if 0.0 > correction_dot {
            zero
        } else {
            correction
        };
        // Source v62 now holds projection, NOT the former normalized vector.
        let projected_dot = numeric
            .dot_vector(projection, requested)
            .map_err(E::Numeric)?;
        let selected = if 0.0 > projected_dot {
            requested
        } else {
            corrected
        };

        let split = slot.inverse_inertia_split; //36384
        let old_x = slot.torque[0]; //36388
        let full = slot.inverse_inertia_full; //3638c
        let old_y = slot.torque[1]; //36398
        let column_y = [full[1], split[1], split[2]];
        let column_z = [full[2], split[2], split[0]];
        let final_full = slot.inverse_inertia_full; //363b0, source reload
        // Source cofactor lanes are formed in this order before transposition.
        let cofactor_y = cross_xyz(column_z, full);
        let dt_seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?; //363dc
        let cofactor_x = cross_xyz(column_y, column_z);
        let dt_residual = -(dt_seed.mul_add(dt, -1.0)); //363f4
        let cofactor_z = cross_xyz(full, column_y);
        let determinant = numeric
            .dot3(
                [full[0], full[1], full[2], 0.0],
                [cofactor_x[0], cofactor_x[1], cofactor_x[2], 0.0],
            )
            .map_err(E::Numeric)?; //36408
        // Source reloads the actual cached inverse tensor for the final multiply.
        let final_split = slot.inverse_inertia_split;
        let dt_first = dt_seed.mul_add(dt_residual, dt_seed); //36418
        let dt_residual = -(dt_first.mul_add(dt, -1.0)); //36434
        let determinant_seed = numeric
            .reciprocal_estimate(determinant)
            .map_err(E::Numeric)?; //3643c
        let inverse_dt = dt_first.mul_add(dt_residual, dt_first); //36448
        let determinant_first = reciprocal_first(determinant, determinant_seed);
        let inverse_determinant = reciprocal_second(determinant, determinant_first);
        let angular_rate = selected.xyz.map(|v| inverse_dt * v); //36460
        let inverse_x =
            [cofactor_x[0], cofactor_y[0], cofactor_z[0]].map(|v| v * inverse_determinant);
        let inverse_y =
            [cofactor_x[1], cofactor_y[1], cofactor_z[1]].map(|v| v * inverse_determinant);
        let inverse_z =
            [cofactor_x[2], cofactor_y[2], cofactor_z[2]].map(|v| v * inverse_determinant);
        let old_z = slot.torque[2]; //36478
        let second_dt_seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?; //36480, fresh
        let first = inverse_x.map(|v| v * angular_rate[0]);
        let second_dt_residual = -(second_dt_seed.mul_add(dt, -1.0));
        let second: [f32; 3] =
            std::array::from_fn(|lane| inverse_y[lane].mul_add(angular_rate[1], first[lane]));
        let second_dt_first = second_dt_seed.mul_add(second_dt_residual, second_dt_seed);
        let transformed: [f32; 3] =
            std::array::from_fn(|lane| inverse_z[lane].mul_add(angular_rate[2], second[lane]));
        let second_inverse_dt = reciprocal_second(dt, second_dt_first);
        let torque = transformed.map(|v| second_inverse_dt * v);
        let final_y = [final_full[1], final_split[1], final_split[2]];
        let final_z = [final_full[2], final_split[2], final_split[0]];
        let first = final_full.map(|v| v * torque[0]);
        let second: [f32; 3] =
            std::array::from_fn(|lane| final_y[lane].mul_add(torque[1], first[lane]));
        let acceleration: [f32; 3] =
            std::array::from_fn(|lane| final_z[lane].mul_add(torque[2], second[lane]));
        // Source operand order differs from ApplyLocalTorque for Y and Z.
        let new_z = acceleration[2] + old_z;
        let new_y = acceleration[1] + old_y;
        slot.torque[1] = new_y;
        let new_x = old_x + acceleration[0];
        slot.torque[0] = new_x;
        slot.torque[2] = new_z;
        slot.cooldown = 0;
        Ok(())
    }
}

#[cfg(test)]
#[path = "simulation_rigid_body_target_angular_tests.rs"]
mod tests;

#[path = "simulation_rigid_body_apply_angular.rs"]
pub(crate) mod apply_angular;
#[path = "simulation_rigid_body_target_angular_exact.rs"]
pub(crate) mod exact;
