//! Same actual board constructor/session material operations for bail entry.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
};
use crate::{
    input_cvar_runtime::InputCvarReferences, skateboard_assembly::SkateboardSimulationBackend,
    skateboard_body_construction::wipeout::WipeoutBoardError,
};

#[derive(Clone, Copy)]
pub(crate) enum BoardWipeoutMaterialOperation<'a> {
    SetFree(&'a InputCvarReferences),
    SetWipeout,
    RestoreWheels,
    /// `SetMaterialsForGrind` (true) / `RestoreMaterialsFromGrind` (false).
    Grind(bool),
}

#[derive(Debug)]
pub(crate) enum PoolBoardWipeoutError {
    Pool(SimulationSkaterPoolError),
    MissingConstruction,
    Body(WipeoutBoardError),
}
impl std::fmt::Display for PoolBoardWipeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "wipeout board pool: {e}"),
            Self::MissingConstruction => {
                f.write_str("wipeout board lacks actual constructor owner")
            }
            Self::Body(e) => write!(f, "{e}"),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_board_wipeout_material_operation(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        operation: BoardWipeoutMaterialOperation<'_>,
    ) -> Result<(), PoolBoardWipeoutError> {
        use PoolBoardWipeoutError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        let board = self.members[index].content.board_mut().map_err(E::Pool)?;
        let construction = board.construction.as_mut().ok_or(E::MissingConstruction)?;
        match operation {
            BoardWipeoutMaterialOperation::SetFree(refs) => {
                construction.try_set_free(&mut board.session, backend, refs)
            }
            BoardWipeoutMaterialOperation::SetWipeout => {
                construction.try_set_wipeout(&mut board.session, backend)
            }
            BoardWipeoutMaterialOperation::RestoreWheels => {
                construction.try_restore_wheel_material(&mut board.session, backend)
            }
            BoardWipeoutMaterialOperation::Grind(grind) => {
                construction.try_set_grind_materials(&mut board.session, backend, grind)
            }
        }
        .map_err(E::Body)
    }
}
