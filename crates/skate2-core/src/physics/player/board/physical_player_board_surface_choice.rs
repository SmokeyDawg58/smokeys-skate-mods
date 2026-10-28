//! Selected actual `PostPhysics` surface producer; no solver completion claim.
use super::physical_board_forces::PhysicalPlayerBoardForceError;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    input_cvar_runtime::InputCvarError, physical_player_assembly::PhysicalAssemblyWorld,
    skateboard_contact_surface::choice::BoardSurfaceChoiceError,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum PhysicalBoardSurfaceChoiceError {
    Board(PhysicalPlayerBoardForceError),
    Poisoned,
    Choice(BoardSurfaceChoiceError<InputCvarError>),
}
impl std::fmt::Display for PhysicalBoardSurfaceChoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Board(e) => write!(f, "board surface owner: {e}"),
            Self::Poisoned => f.write_str("board contacts lock poisoned"),
            Self::Choice(e) => write!(f, "board surface choice: {e:?}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_choose_board_physics_surface(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalBoardSurfaceChoiceError> {
        use PhysicalBoardSurfaceChoiceError as E;
        let owner = Arc::clone(
            self.physical_body_owner_for_contacts(board)
                .map_err(|e| E::Board(PhysicalPlayerBoardForceError::Owner(e)))?,
        );
        let contacts = self
            .try_with_board_force_runtime(board, |runtime| Arc::clone(&runtime.contact_surface))
            .map_err(E::Board)?;
        contacts
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_choose_physics_surface(|| owner.inputs().cvars.try_forced_world_surface())
            .map_err(E::Choice)
    }
}
