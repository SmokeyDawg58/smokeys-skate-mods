//! Same-owner forward-speed continuation before four wheel-body positions.
use super::board_fill_wobble::Update5BeforeBoardForwardVelocity;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonJumpPairError,
        board_entry::{SkateboardEntrySessionError, try_current_board_effective_transform},
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill_forward::BoardForwardOutputError,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum BoardForwardSourceError {
    Pool(SimulationSkaterPoolError),
    Transform(SkateboardEntrySessionError),
}
#[derive(Debug)]
pub(crate) enum BoardForwardError<N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Output(BoardForwardOutputError<BoardForwardSourceError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for BoardForwardError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board forward requires actual pool"),
            Self::Pair(e) => write!(f, "board forward pair: {e:?}"),
            Self::ForeignOwner => f.write_str("board forward continuation has foreign owner"),
            Self::PoisonedOutput => f.write_str("board forward output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
impl std::fmt::Display for BoardForwardSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::Transform(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeBoardWheelPositions {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) assembly: super::board_fill_wheels::BoardFillAssemblyDescriptor,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_forward<N: SkeletonCollisionNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardForwardVelocity,
        numeric: &mut N,
    ) -> Result<Update5BeforeBoardWheelPositions, BoardForwardError<N::Error>> {
        use BoardForwardError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardForwardVelocity {
            owner: previous_owner,
            assembly,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_fill_board_forward(
                || {
                    let session = self.board(board).map_err(BoardForwardSourceError::Pool)?;
                    try_current_board_effective_transform(session, &self.backend, &owner)
                        .map_err(BoardForwardSourceError::Transform)
                },
                numeric,
            )
            .map_err(E::Output)?;
        Ok(Update5BeforeBoardWheelPositions { owner, assembly })
    }
}
