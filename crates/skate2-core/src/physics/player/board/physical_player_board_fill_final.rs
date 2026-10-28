//! Same-owner final board output stores; subsequent skeleton output remains separate.
use super::{
    board_fill_ground::Update5BeforeBoardSlideSpeed,
    physical_board_forces::PhysicalPlayerBoardForceError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonJumpPairError,
        board_entry::SkateboardEntrySessionError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill_final::{
        BoardFillFinalSource, BoardFinalOutputError,
    },
    skateboard_body_construction::output_getters::BoardSlideSpeedError,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum BoardFinalSourceError<N> {
    Pool(SimulationSkaterPoolError),
    Board(PhysicalPlayerBoardForceError),
    Attributes(SkateboardEntrySessionError),
    Slide(BoardSlideSpeedError<N>),
    Missing(&'static str),
    Poisoned(&'static str),
}
impl<N: std::fmt::Debug> std::fmt::Display for BoardFinalSourceError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Attributes(e) => e.fmt(f),
            Self::Slide(e) => e.fmt(f),
            Self::Missing(field) => write!(f, "final board output requires {field}"),
            Self::Poisoned(owner) => write!(f, "final board {owner} lock poisoned"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum BoardFinalError<N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Output(BoardFinalOutputError<BoardFinalSourceError<N>>),
}
impl<N: std::fmt::Debug> std::fmt::Display for BoardFinalError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("final board output requires actual pool"),
            Self::Pair(e) => write!(f, "final board pair: {e:?}"),
            Self::ForeignOwner => f.write_str("final board continuation has foreign owner"),
            Self::PoisonedOutput => f.write_str("final board output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5AfterBoardFill {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_finish_update5_board_fill<N: SkeletonCollisionNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardSlideSpeed,
        numeric: &mut N,
    ) -> Result<Update5AfterBoardFill, BoardFinalError<N::Error>> {
        use BoardFinalError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardSlideSpeed {
            owner: previous_owner,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_fill_board_final(&mut Source {
                world: self,
                board,
                owner: &owner,
                numeric,
            })
            .map_err(E::Output)?;
        Ok(Update5AfterBoardFill { owner })
    }
}
struct Source<'a, N> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    owner: &'a PhysicalPlayerAssemblyOwner,
    numeric: &'a mut N,
}
impl<N: SkeletonCollisionNumeric> BoardFillFinalSource for Source<'_, N> {
    type Error = BoardFinalSourceError<N::Error>;
    type Body = SimulationSkaterAssemblyHandle;
    fn capture_body(&mut self) -> Result<Self::Body, Self::Error> {
        self.world.board(self.board).map_err(Self::Error::Pool)?;
        Ok(self.board.clone())
    }
    fn deck_slide_speed(&mut self, body: &Self::Body) -> Result<f32, Self::Error> {
        let contacts = self
            .world
            .try_with_board_force_runtime(body, |runtime| Arc::clone(&runtime.contact_surface))
            .map_err(Self::Error::Board)?;
        contacts
            .lock()
            .map_err(|_| Self::Error::Poisoned("contacts"))?
            .try_part_slide_speed(6, self.numeric)
            .map_err(Self::Error::Slide)
    }
    fn deck_to_wheel_base(&mut self, body: &Self::Body) -> Result<f32, Self::Error> {
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Missing("actual skater pool"))?
            .try_board_output_deck_to_wheel_base(&self.world.backend, body)
            .map_err(Self::Error::Attributes)
    }
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned("Reckoning"))?
                .processed(),
        );
        processed
            .lock()
            .map_err(|_| Self::Error::Poisoned("Processed"))?
            .animation_wheel_hardness()
            .ok_or(Self::Error::Missing("Processed+9c8 wheel hardness"))
    }
    fn wants_respawn(&mut self) -> Result<bool, Self::Error> {
        self.world
            .try_with_board_force_runtime(self.board, |runtime| runtime.entry.wants_respawn())
            .map_err(Self::Error::Board)
    }
    fn clear_wants_respawn(&mut self) -> Result<(), Self::Error> {
        self.world
            .try_with_board_force_runtime(self.board, |runtime| runtime.entry.clear_wants_respawn())
            .map_err(Self::Error::Board)
    }
}
