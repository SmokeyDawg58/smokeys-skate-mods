//! Same-pair bridge for `SkeletonController::UpdateAirCollision`0x82dd9fc8.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    skeleton_controller::air_update::{AirCollisionError, AirCollisionInput},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_runtime_state::SkeletonRuntimeStateError,
};
use std::sync::Arc;

impl AirCollisionInput for SkeletonProcessedContact {
    type Error = &'static str;
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.known_flags_8c8().ok_or("Processed flags+8c8")
    }
    fn velocity_y(&mut self) -> Result<f32, Self::Error> {
        self.prior_output()
            .velocity()
            .map(|value| value.value.xyz[1])
            .ok_or("Processed velocity+190")
    }
}

#[derive(Debug)]
pub(crate) enum PairAirCollisionError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Poisoned,
    Update(AirCollisionError<&'static str, SkeletonRuntimeStateError>),
}
impl std::fmt::Display for PairAirCollisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Air collision requires skater pool"),
            Self::Pair(e) => e.fmt(f),
            Self::Poisoned => f.write_str("Air collision owner lock poisoned"),
            Self::Update(e) => e.fmt(f),
        }
    }
}
impl PhysicalAssemblyWorld {
    /// Uses the existing collision-pair lease and concrete normal-Skeleton adapter.
    /// No synthetic contact or alternate processed/controller occurrence is created.
    pub(crate) fn try_update_pair_air_collision(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PairAirCollisionError> {
        use PairAirCollisionError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_collision_pair(&self.backend, board, skeleton, |normal, owner| {
                let processed = Arc::clone(
                    owner
                        .reckoning()
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .processed(),
                );
                let mut input = processed.lock().map_err(|_| E::Poisoned)?;
                owner
                    .skeleton_controller()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .try_update_air_collision(&mut *input, normal)
                    .map_err(E::Update)
            })
            .map_err(E::Pair)?
    }
}
