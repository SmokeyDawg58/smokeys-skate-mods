//! Same-owner Update5 call82e04f24 after the complete Skeleton callback.
use super::update5_skeleton_suffix::Update5BeforePumping;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::pumping_fill::PumpingFillError,
    physical_player_pumping::fill::{PumpingFillRuntimeError, try_fill_pumping_output},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum Update5PumpingError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    Fill(PumpingFillError<PumpingFillRuntimeError>),
}
impl std::fmt::Display for Update5PumpingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Pumping.Fill requires the actual skater pool"),
            Self::Pair(error) => write!(f, "Pumping.Fill pair: {error:?}"),
            Self::ForeignOwner => f.write_str("Pumping.Fill continuation belongs to another pair"),
            Self::Fill(error) => error.fmt(f),
        }
    }
}
/// Pumping returned; native GrindManager.Fill82e04f30 remains explicitly parked.
pub(crate) struct Update5BeforeGrindManagerFill {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl PhysicalAssemblyWorld {
    ///82e04f24: consumes the actual Skeleton-return token and retains its owner.
    pub(crate) fn try_continue_update5_pumping(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforePumping,
    ) -> Result<Update5BeforeGrindManagerFill, Update5PumpingError> {
        use Update5PumpingError as E;
        let same_owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                Arc::ptr_eq(owner, &before.owner)
            })
            .map_err(E::Pair)?;
        if !same_owner {
            return Err(E::ForeignOwner);
        }
        try_fill_pumping_output(&before.owner).map_err(E::Fill)?;
        Ok(Update5BeforeGrindManagerFill {
            owner: before.owner,
        })
    }
}
