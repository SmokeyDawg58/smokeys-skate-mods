//! Late native body+0x30 read for selected `FillPhysOut` motion publication.
use super::{
    SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};
impl SimulationRigidBodyPool {
    /// PhysicsGround.Enter 82d99d00/04 replaces XYZ while retaining old packed
    /// W. The host represents that adjacent owner/link separately; no W is made up.
    pub(crate) fn try_set_part_angular_velocity(
        &mut self,
        observation: SimulationRigidBodyObservation,
        omega: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        self.slots[index].omega = omega;
        // Same freshness rule as the actual linear-velocity replacement.
        self.frame_identity = None;
        Ok(())
    }
    /// `Skateboard::UpdatePostPhysics`82b37e10..18 changes deck omega after
    /// solving, then HiLOD reads that same solve's Skeleton contacts. Keep
    /// the frame certificate and native packed W/links; replace only XYZ.
    pub(crate) fn try_store_post_solve_angular_velocity(
        &mut self,
        observation: SimulationRigidBodyObservation,
        omega: [f32; 3],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        self.slots[index].omega = omega;
        Ok(())
    }
    /// 0x82b2f718. XYZ only: adjacent native W is not a recovered vector lane.
    pub(crate) fn try_phys_out_angular_velocity(
        &self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<[f32; 3], SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        Ok(self.slots[index].omega)
    }
}
#[cfg(test)]
#[path = "simulation_rigid_body_angular_velocity_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "simulation_rigid_body_ground_entry_tests.rs"]
mod ground_entry_tests;
