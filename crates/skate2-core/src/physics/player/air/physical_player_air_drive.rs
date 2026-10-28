//! Physical world forwarding to the same board's retained hook dynamics.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        board_entry::{
            SkateboardEntrySessionError, air_drive::AirDriveError,
            animation::BoardAnimationRuntimeError,
        },
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
};
impl PhysicalAssemblyWorld {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_enable_source_plant_drive(
        &mut self,board:&SimulationSkaterAssemblyHandle,boneless:bool,
    )->Result<(),BoardAnimationRuntimeError<std::convert::Infallible>> {
        self.skaters.as_mut().ok_or(BoardAnimationRuntimeError::Owner(
            SkateboardEntrySessionError::Pool(SimulationSkaterPoolError::Uninitialized)))?
            .try_enable_source_plant(&self.backend,board,boneless)
    }

    pub(crate) fn try_disable_board_linear_drive(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), AirDriveError<std::convert::Infallible>> {
        self.skaters
            .as_mut()
            .ok_or(AirDriveError::Entry(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            )))?
            .try_disable_board_linear_drive(&self.backend, board)
    }

    pub(crate) fn try_enable_board_animation_for_hold<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        soft: bool,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), BoardAnimationRuntimeError<N::Error>> {
        self.skaters
            .as_mut()
            .ok_or(BoardAnimationRuntimeError::Owner(
                SkateboardEntrySessionError::Pool(SimulationSkaterPoolError::Uninitialized),
            ))?
            .try_enable_board_animation_for_hold(&self.backend, board, soft, refs, numeric)
    }
    pub(crate) fn try_enable_board_angular_drive_only<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        strength: f32,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AirDriveError<N::Error>> {
        self.skaters
            .as_mut()
            .ok_or(AirDriveError::Entry(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            )))?
            .try_enable_board_angular_drive_only(&self.backend, board, strength, refs, numeric)
    }
}
