//! Actual same-pair board `CollideWithWorldOnly`/`CollideNormally` mutations.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkeletonJumpPairError,
};
use crate::{
    dynamic_assembly_session::board_collision_groups::BoardCollisionGroupError,
    skateboard_assembly::SkateboardSimulationBackend,
};
#[derive(Debug)]
pub(crate) enum BoardCollisionModeError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Groups(BoardCollisionGroupError),
}
impl std::fmt::Display for BoardCollisionModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board collision pool unavailable"),
            Self::Pair(e) => write!(f, "board collision mode: {e}"),
            Self::Groups(e) => write!(f, "{e}"),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_set_pair_board_collision_mode(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        world_only: bool,
    ) -> Result<(), BoardCollisionModeError> {
        use BoardCollisionModeError as E;
        self.try_with_jump_pair(backend, board, skeleton, |_, _| ())
            .map_err(E::Pair)?;
        self.invalidate_refresh();
        let session = self
            .session(board)
            .map_err(|e| E::Pair(SimulationSkeletonJumpPairError::Pool(e)))?;
        session
            .board_collision_groups
            .try_set_mode(world_only)
            .map_err(E::Groups)
    }
}
