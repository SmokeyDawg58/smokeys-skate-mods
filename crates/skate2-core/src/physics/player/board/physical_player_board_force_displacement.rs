//! Same retained board's `StoreForceDisplacement`, 0x82b23830..82b2390c.
use crate::simulation_assembly_backend::constraint_session::physical_board_forces::PhysicalPlayerBoardForceError;
use crate::{
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
        },
    },
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skeleton_root_transforms::SkeletonRootVector,
};

const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889); // original8225c488

#[derive(Debug)]
pub(crate) enum StoreForceDisplacementError {
    Force(PhysicalPlayerBoardForceError),
    Pool(SimulationSkaterPoolError),
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
}
impl std::fmt::Display for StoreForceDisplacementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Force(error) => error.fmt(f),
            Self::Pool(error) => error.fmt(f),
            Self::Part(error) => error.fmt(f),
            Self::Body(error) => error.fmt(f),
        }
    }
}

/// Raw VMX graph retains zero multiplications (including nonfinite torque).
/// Two rounded dt products are distinct; the velocity addition is fused later.
fn force_displacement(force: [f32; 4], torque: [f32; 4]) -> [f32; 4] {
    let zxy = [torque[2], torque[0], torque[1], torque[3]];
    let yzx = [torque[1], torque[2], torque[0], torque[3]];
    std::array::from_fn(|lane| {
        let residual = -(0.0_f32.mul_add(zxy[lane], -force[lane]));
        let effective = 0.0_f32.mul_add(yzx[lane], residual);
        let first = effective * UPDATE_TIME;
        first * UPDATE_TIME
    })
}

/// Preserve the first original store on a later checked velocity-read failure.
fn try_store<E>(
    output: &mut Option<SkeletonRootVector>,
    force: [f32; 4],
    torque: [f32; 4],
    velocity: impl FnOnce() -> Result<SkeletonRootVector, E>,
) -> Result<SkeletonRootVector, E> {
    let partial = force_displacement(force, torque);
    *output = Some(SkeletonRootVector::known(partial)); //238f0
    let velocity = velocity()?; // fresh body238f4 then velocity238fc
    let value = SkeletonRootVector {
        xyz: std::array::from_fn(|lane| velocity.xyz[lane].mul_add(UPDATE_TIME, partial[lane])),
        w: velocity.w.map(|w| w.mul_add(UPDATE_TIME, partial[3])),
    };
    *output = Some(value); //23904; native velocity W is an intrusive pointer
    Ok(value)
}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_store_force_displacement(
        &mut self,
        backend: &crate::skateboard_assembly::SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<SkeletonRootVector, StoreForceDisplacementError> {
        use StoreForceDisplacementError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let runtime = candidate.force_runtime.as_mut().ok_or({
            E::Force(PhysicalPlayerBoardForceError::Forces(
                super::SimulationSkateboardForceError::MissingRuntime,
            ))
        })?;
        let session = &candidate.session;
        let observation = session
            .try_part_contact_spy_observation(backend, 6)
            .map_err(E::Part)?;
        let body = backend
            .collision_body_pool()
            .try_serialised_assembly_snapshot(observation)
            .map_err(E::Body)?;
        // Existing serialized-body mapping: +90 force/kine, +a0 torque/cool.
        let force = [
            body.force[0],
            body.force[1],
            body.force[2],
            body.kinematic_value,
        ];
        let torque = [
            body.torque[0],
            body.torque[1],
            body.torque[2],
            f32::from_bits(body.cooldown),
        ];
        // Split borrows preserve the SAME runtime and a genuinely fresh Part6 read.
        try_store(&mut runtime.expected_displacement, force, torque, || {
            let observation = session
                .try_part_contact_spy_observation(backend, 6)
                .map_err(E::Part)?;
            let velocity = backend
                .collision_body_pool()
                .try_contact_spy_linear_velocity(observation)
                .map_err(E::Body)?;
            Ok(SkeletonRootVector {
                xyz: velocity,
                w: None,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{UPDATE_TIME, force_displacement, try_store};
    use crate::skeleton_root_transforms::SkeletonRootVector;

    #[test]
    fn two_products_then_fused_velocity_preserve_all_known_lanes() {
        let mut output = None;
        let force = [3600.0, -7200.0, 0.125, -0.0];
        let velocity = [60.0, -120.0, 0.5, -0.0];
        let value = try_store::<()>(&mut output, force, [1.0; 4], || {
            Ok(SkeletonRootVector::known(velocity))
        })
        .unwrap();
        let mut expected: [f32; 4] = std::array::from_fn(|i| {
            velocity[i].mul_add(UPDATE_TIME, (force[i] * UPDATE_TIME) * UPDATE_TIME)
        });
        // The intervening +0 FMA changes -0 force to +0 before velocity FMA.
        expected[3] = 0.0;
        assert_eq!(
            value.known_lanes().unwrap().map(f32::to_bits),
            expected.map(f32::to_bits)
        );
        assert_eq!(output, Some(value));
    }

    #[test]
    fn separate_rounded_time_products_differ_from_precomputed_time_squared() {
        let force = f32::from_bits(0x3f00_0004);
        let result = force_displacement([force; 4], [0.0; 4]);
        assert_eq!(result.map(f32::to_bits), [0x3911_a2b9; 4]);
        assert_eq!((force * (UPDATE_TIME * UPDATE_TIME)).to_bits(), 0x3911_a2ba);
    }

    #[test]
    fn nonfinite_torque_is_not_simplified_away_and_late_failure_keeps_first_store() {
        let mut output = None;
        assert_eq!(
            try_store(
                &mut output,
                [1.0; 4],
                [f32::INFINITY, 0.0, 0.0, 0.0],
                || Err("velocity")
            ),
            Err("velocity")
        );
        let partial = output.unwrap();
        assert!(partial.xyz[1].is_nan());
        assert!(partial.xyz[2].is_nan());
        assert_eq!(
            partial.xyz[0].to_bits(),
            ((1.0_f32 * UPDATE_TIME) * UPDATE_TIME).to_bits()
        );
        let signs = force_displacement([-0.0; 4], [0.0; 4]);
        assert_eq!(signs.map(f32::to_bits), [0; 4]);
    }

    #[test]
    fn unknown_velocity_pointer_lane_remains_unknown_after_final_store() {
        let mut output = None;
        let value = try_store::<()>(&mut output, [1.0; 4], [0.0; 4], || {
            Ok(SkeletonRootVector {
                xyz: [2.0; 3],
                w: None,
            })
        })
        .unwrap();
        assert_eq!(value.w, None);
        assert_eq!(output, Some(value));
    }
}
