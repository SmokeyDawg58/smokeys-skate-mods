//! Unregistered backend child: direct same-pool deck torque services.
use super::SimulationAssemblyBackend;
use crate::{
    reckoning_numeric::ReckoningNumeric,
    simulation_rigid_body_pool::{
        SimulationRigidBodyObservation, SimulationRigidBodyPoolError,
        target_angular::TargetAngularError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
impl<M, JF: Copy, JL: Copy> SimulationAssemblyBackend<M, JF, JL> {
    pub(crate) fn try_target_skateboard_angular_displacement_exact<N: ReckoningNumeric>(
        &mut self,
        body: SimulationRigidBodyObservation,
        displacement: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), TargetAngularError<N::Error>> {
        self.rigid_bodies
            .try_target_skateboard_angular_displacement_exact(body, displacement, numeric)
    }
    pub(crate) fn try_apply_skateboard_local_torque(
        &mut self,
        body: SimulationRigidBodyObservation,
        torque: SkeletonRootVector,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.rigid_bodies
            .try_apply_skateboard_local_torque(body, torque)
    }
    pub(crate) fn try_target_skateboard_angular_displacement<N: ReckoningNumeric>(
        &mut self,
        body: SimulationRigidBodyObservation,
        displacement: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), TargetAngularError<N::Error>> {
        self.rigid_bodies
            .try_target_skateboard_angular_displacement(body, displacement, numeric)
    }
}
