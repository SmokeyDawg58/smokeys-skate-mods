//! Same-owner continuation 0x82b30050..0x82b300e0, before fresh deck transform.
use super::{
    board_fill_surface::Update5BeforeBoardWobble,
    physical_board_forces::PhysicalPlayerBoardForceError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill_wobble::{
        BoardFillWobbleOutputError, BoardFillWobbleSource,
    },
    skateboard_force_runtime::{SkateboardForceRuntime, speed_wobble::SkateboardSpeedWobble},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum BoardWobbleError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Board(PhysicalPlayerBoardForceError),
    Output(BoardFillWobbleOutputError<&'static str>),
}
impl std::fmt::Display for BoardWobbleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board wobble requires actual pool"),
            Self::Pair(e) => write!(f, "board wobble pair: {e:?}"),
            Self::ForeignOwner => f.write_str("board wobble continuation has foreign owner"),
            Self::PoisonedOutput => f.write_str("board wobble output lock poisoned"),
            Self::Board(e) => write!(f, "board wobble runtime: {e}"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeBoardForwardVelocity {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) assembly: super::board_fill_wheels::BoardFillAssemblyDescriptor,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_wobble(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardWobble,
    ) -> Result<Update5BeforeBoardForwardVelocity, BoardWobbleError> {
        use BoardWobbleError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardWobble {
            owner: previous_owner,
            assembly,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        {
            let mut output = owner.phys_out().lock().map_err(|_| E::PoisonedOutput)?;
            self.try_with_board_force_runtime(board, |runtime| {
                output.try_fill_board_wobble(&mut Source { runtime })
            })
            .map_err(E::Board)?
            .map_err(E::Output)?;
        }
        Ok(Update5BeforeBoardForwardVelocity { owner, assembly })
    }
}
struct Source<'a> {
    runtime: &'a SkateboardForceRuntime,
}
impl<'a> BoardFillWobbleSource for Source<'a> {
    type Error = &'static str;
    type Wobble = Option<&'a SkateboardSpeedWobble>;
    fn capture_wobble(&mut self) -> Result<Self::Wobble, Self::Error> {
        // Native null pointer capture itself does not dereference the object.
        Ok(self.runtime.speed_wobble.value())
    }
    fn effect(&mut self, wobble: &Self::Wobble) -> Result<f32, Self::Error> {
        Ok(wobble
            .ok_or("constructed SpeedWobble for effect+24")?
            .effect())
    }
    fn tilt(&mut self, wobble: &Self::Wobble) -> Result<f32, Self::Error> {
        Ok(wobble
            .ok_or("constructed SpeedWobble for tilt+28")?
            .wobble_tilt())
    }
    fn deck_tilt(&mut self) -> Result<f32, Self::Error> {
        Ok(self.runtime.steering.deck_tilt())
    }
}
