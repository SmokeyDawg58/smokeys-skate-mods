//! Unregistered backend child preserving the actual retained rigid-body pool.
use super::SimulationAssemblyBackend;
use crate::{
    reckoning_numeric::ReckoningNumeric,
    simulation_rigid_body_pool::{
        SimulationRigidBodyObservation, target_angular::apply_angular::ApplyAngularError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
impl<M, JF: Copy, JL: Copy> SimulationAssemblyBackend<M, JF, JL> {
    pub(crate) fn try_apply_skateboard_angular_displacement<N: ReckoningNumeric>(
        &mut self,
        body: SimulationRigidBodyObservation,
        requested: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), ApplyAngularError<N::Error>> {
        self.rigid_bodies
            .try_apply_skateboard_angular_displacement(body, requested, numeric)
    }
}
