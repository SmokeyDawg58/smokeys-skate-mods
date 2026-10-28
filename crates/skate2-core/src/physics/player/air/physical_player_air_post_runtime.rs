//! Actual retained board velocity and Air wipeout collaborators for Air.Post.
use crate::{
    debug_overlay::DebugOverlaySlot,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, board_entry::SkateboardEntrySessionError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::air_post_solve::{
        AirPostCollaborators, AirPostError, try_update_current_air_post,
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::ground_wipeout::GroundWipeoutRuntimeError,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum AirPostRuntimeError<N> {
    Board(SkateboardEntrySessionError),
    Wipeout(GroundWipeoutRuntimeError<N>),
}
struct Source<'a, N> {
    world: &'a mut PhysicalAssemblyWorld,
    overlay: DebugOverlaySlot,
    numeric: &'a mut N,
}
impl<N: ReckoningNumeric> AirPostCollaborators for Source<'_, N> {
    type Error = AirPostRuntimeError<N::Error>;
    fn board_velocity_y(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<f32, Self::Error> {
        use SkateboardEntrySessionError as B;
        let body = self
            .world
            .board(board)
            .map_err(|e| Self::Error::Board(B::Pool(e)))?;
        crate::dynamic_assembly_session::skater_pool::board_entry::try_current_board_deck_velocity(
            body,
            self.world.backend(),
        )
        .map(|value| value.xyz[1])
        .map_err(Self::Error::Board)
    }
    fn check_air_wipeout(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        use_com: bool,
    ) -> Result<(), Self::Error> {
        self.world
            .try_check_air_wipeout(board, skeleton, owner, &self.overlay, self.numeric, use_com)
            .map_err(Self::Error::Wipeout)
    }
}
impl PhysicalAssemblyWorld {
    /// `PhysicsAir::PostPhysics`82d8b238, after actual Skeleton.Post.
    /// Source apex store survives any later wipeout failure.
    pub(crate) fn try_update_air_postphysics<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<(), AirPostError<AirPostRuntimeError<N::Error>>> {
        let overlay = self.debug_overlay.clone();
        try_update_current_air_post(
            owner,
            board,
            skeleton,
            &mut Source {
                world: self,
                overlay,
                numeric,
            },
        )
    }
}
