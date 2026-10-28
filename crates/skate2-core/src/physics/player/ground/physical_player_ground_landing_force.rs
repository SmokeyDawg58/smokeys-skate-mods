//! Ground.Enter82d99de4..9f94 after LandingOnDeck505, on the retained board.
use super::*;
use crate::{
    skateboard_foot_attributes::{SkateboardFootAttributeError, SkateboardFootAttributeRead},
    skateboard_foot_forces::SkateboardFootForce,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
};
use std::sync::Mutex;

#[derive(Debug)]
pub(crate) enum LandingForceError<N> {
    Poisoned,
    Missing(&'static str),
    Board(PhysicalPlayerBoardForceError),
    Attribute(SkateboardFootAttributeError),
    Numeric(N),
}
impl PhysicalAssemblyWorld {
    pub(super) fn try_ground_landing_force<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        numeric: &mut N,
    ) -> Result<(), LandingForceError<N::Error>> {
        use LandingForceError as E;
        let (velocity, com_velocity, up) = {
            let p = processed.lock().map_err(|_| E::Poisoned)?;
            let prior = p.prior_output();
            (
                prior.velocity().ok_or(E::Missing("velocity190"))?.value,
                prior
                    .centre_of_mass_velocity()
                    .ok_or(E::Missing("COM velocity260"))?
                    .value,
                prior.reckoning_up().ok_or(E::Missing("up220"))?.value,
            )
        };
        let relative = com_velocity.zip(velocity, |a, b| a - b);
        let vertical = numeric.dot_vector(relative, up).map_err(E::Numeric)?;
        let projected = up.map(|v| v * vertical);
        let tangent = relative.zip(projected, |a, b| a - b);
        let strength = self
            .try_with_board_force_runtime(board, |r| {
                let pointer = r.foot_attributes.lookup(0xd5e2_3929_1aa4_4155)?;
                r.foot_attributes.float(&pointer)
            })
            .map_err(E::Board)?
            .map_err(E::Attribute)?;
        let mass = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physical_mass()
            .ok_or(E::Missing("mass970"))?;
        let dt = f32::from_bits(0x3c88_8889);
        let seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
        let first_residual = -seed.mul_add(dt, -1.0);
        let scaled = tangent.map(|v| v * strength);
        let once = seed.mul_add(first_residual, seed);
        let scaled = scaled.map(|v| v * mass);
        let second_residual = -once.mul_add(dt, -1.0);
        let inverse = once.mul_add(second_residual, once);
        let force = scaled.map(|v| inverse * v);
        let offset = self
            .try_with_board_force_runtime(board, |r| {
                let pointer = r.foot_attributes.lookup(0x62a6_0bae_ae01_ed89)?;
                r.foot_attributes.float(&pointer)
            })
            .map_err(E::Board)?
            .map_err(E::Attribute)?;
        self.try_add_board_force(
            board,
            0x13,
            SkateboardFootForce {
                world_space_force: force,
                local_space_position: V::known([0.0, offset, 0.0, 0.0]),
            },
        )
        .map_err(E::Board)
    }
}
