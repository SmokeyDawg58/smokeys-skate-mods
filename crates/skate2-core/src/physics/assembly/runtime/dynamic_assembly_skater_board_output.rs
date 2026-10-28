//! Current constructor-owned attribute reader for actual board output.
use super::board_entry::SkateboardEntrySessionError;
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SkateboardSimulationBackend,
};
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_board_output_deck_to_wheel_base(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<f32, SkateboardEntrySessionError> {
        use SkateboardEntrySessionError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(board).map_err(E::Pool)?;
        self.members[index]
            .content
            .board_mut()
            .map_err(E::Pool)?
            .construction
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .try_output_deck_to_wheel_base()
            .map_err(E::Attribute)
    }
}
