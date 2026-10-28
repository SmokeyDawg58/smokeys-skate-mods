//! Same actual world owners for individual `WipeoutGround` entry operations.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle,
        board_entry::wipeout::WipeoutBoardControlError,
        wipeout::{WipeoutSkeletonError, WipeoutSkeletonOperation},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
};
#[derive(Debug)]
pub(crate) enum WipeoutHelperError<N> {
    MissingPool,
    Skeleton(WipeoutSkeletonError<N>),
    Board(WipeoutBoardControlError),
}
impl<N: std::fmt::Debug> std::fmt::Display for WipeoutHelperError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("wipeout skater pool unavailable"),
            Self::Skeleton(e) => write!(f, "{e}"),
            Self::Board(e) => write!(f, "{e}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_wipeout_skeleton_operation<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        operation: WipeoutSkeletonOperation,
        numeric: &mut N,
    ) -> Result<(), WipeoutHelperError<N::Error>> {
        self.skaters
            .as_mut()
            .ok_or(WipeoutHelperError::MissingPool)?
            .try_wipeout_skeleton_operation(&mut self.backend, board, skeleton, operation, numeric)
            .map_err(WipeoutHelperError::Skeleton)
    }
    pub(crate) fn try_wipeout_board_turn<N>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), WipeoutHelperError<N>> {
        self.skaters
            .as_mut()
            .ok_or(WipeoutHelperError::MissingPool)?
            .try_wipeout_board_turn(&self.backend, board)
            .map_err(WipeoutHelperError::Board)
    }
    pub(crate) fn try_wipeout_disable_board_hook<N>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        angular: bool,
    ) -> Result<(), WipeoutHelperError<N>> {
        self.skaters
            .as_mut()
            .ok_or(WipeoutHelperError::MissingPool)?
            .try_wipeout_disable_board_hook(&self.backend, board, angular)
            .map_err(WipeoutHelperError::Board)
    }
}
