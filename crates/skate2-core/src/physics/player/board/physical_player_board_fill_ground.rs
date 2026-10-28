//! Same-owner ground/contact output, stopping before actual slide-speed query.
use super::{
    board_fill_wheels::Update5BeforeBoardGroundState,
    physical_board_forces::PhysicalPlayerBoardForceError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill_ground::{
        BoardFillGroundSource, BoardGroundOutputError,
    },
    skateboard_contact_surface::SkateboardContactSurface,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum BoardGroundSourceError {
    Pool(SimulationSkaterPoolError),
    Board(PhysicalPlayerBoardForceError),
    Missing(&'static str),
    Poisoned(&'static str),
}
impl std::fmt::Display for BoardGroundSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Missing(field) => write!(f, "board ground requires {field}"),
            Self::Poisoned(owner) => write!(f, "board ground {owner} lock poisoned"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum BoardGroundError<N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Output(BoardGroundOutputError<BoardGroundSourceError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for BoardGroundError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board ground requires actual pool"),
            Self::Pair(e) => write!(f, "board ground pair: {e:?}"),
            Self::ForeignOwner => f.write_str("board ground continuation has foreign owner"),
            Self::PoisonedOutput => f.write_str("board ground output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeBoardSlideSpeed {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_ground<N: SkeletonCollisionNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardGroundState,
        numeric: &mut N,
    ) -> Result<Update5BeforeBoardSlideSpeed, BoardGroundError<N::Error>> {
        use BoardGroundError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardGroundState {
            owner: previous_owner,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_fill_board_ground(&mut Source { world: self, board }, numeric)
            .map_err(E::Output)?;
        Ok(Update5BeforeBoardSlideSpeed { owner })
    }
}
struct Source<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
}
impl Source<'_> {
    fn contacts(
        &mut self,
        body: &SimulationSkaterAssemblyHandle,
    ) -> Result<Arc<Mutex<SkateboardContactSurface>>, BoardGroundSourceError> {
        self.world
            .try_with_board_force_runtime(body, |runtime| Arc::clone(&runtime.contact_surface))
            .map_err(BoardGroundSourceError::Board)
    }
    fn post_fields(
        &self,
        body: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        &crate::skateboard_body_construction::SkateboardBodyPostFields,
        BoardGroundSourceError,
    > {
        self.world
            .board_construction(body)
            .map_err(BoardGroundSourceError::Pool)?
            .and_then(crate::skateboard_body_construction::SkateboardBodyConstruction::post_fields)
            .ok_or(BoardGroundSourceError::Missing(
                "body constructor post fields",
            ))
    }
}
impl BoardFillGroundSource for Source<'_> {
    type Error = BoardGroundSourceError;
    type Body = SimulationSkaterAssemblyHandle;
    fn capture_body(&mut self) -> Result<Self::Body, Self::Error> {
        self.world.board(self.board).map_err(Self::Error::Pool)?;
        Ok(self.board.clone())
    }
    fn time_since_wheels(&mut self, body: &Self::Body) -> Result<f32, Self::Error> {
        Ok(self.post_fields(body)?.time_since_wheels_touching)
    }
    fn dynamic_up(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let state = self
            .world
            .try_with_board_force_runtime(self.board, |runtime| Arc::clone(&runtime.up_state))
            .map_err(Self::Error::Board)?;
        state
            .lock()
            .map_err(|_| Self::Error::Poisoned("outer up"))?
            .dynamic_up
            .ok_or(Self::Error::Missing("outer+70 dynamic up"))
    }
    fn last_valid_normal(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error> {
        self.contacts(body)?
            .lock()
            .map_err(|_| Self::Error::Poisoned("contacts"))?
            .collision_state
            .last_valid_normal
            .ok_or(Self::Error::Missing("body+50 last valid normal"))
    }
    fn contact_acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let state = self
            .world
            .try_with_board_force_runtime(self.board, |runtime| Arc::clone(&runtime.up_state))
            .map_err(Self::Error::Board)?;
        state
            .lock()
            .map_err(|_| Self::Error::Poisoned("outer up"))?
            .contact_acceleration
            .ok_or(Self::Error::Missing("outer+90 contact acceleration"))
    }
    fn average_normal(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error> {
        self.contacts(body)?
            .lock()
            .map_err(|_| Self::Error::Poisoned("contacts"))?
            .collision_state
            .average_normal
            .ok_or(Self::Error::Missing("body+40 average normal"))
    }
    fn num_wheels(&mut self, body: &Self::Body) -> Result<u8, Self::Error> {
        Ok(self.post_fields(body)?.num_wheels)
    }
    fn in_contact(&mut self, body: &Self::Body, part: usize) -> Result<bool, Self::Error> {
        self.contacts(body)?
            .lock()
            .map_err(|_| Self::Error::Poisoned("contacts"))?
            .collision_state
            .in_contact[part]
            .ok_or(Self::Error::Missing("selected inContact byte"))
    }
    fn deck_normal(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error> {
        self.contacts(body)?
            .lock()
            .map_err(|_| Self::Error::Poisoned("contacts"))?
            .collision_state
            .normal[6]
            .ok_or(Self::Error::Missing("deck normal"))
    }
    fn deck_acceleration(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error> {
        self.contacts(body)?
            .lock()
            .map_err(|_| Self::Error::Poisoned("contacts"))?
            .collision_state
            .acceleration[6]
            .ok_or(Self::Error::Missing("deck acceleration"))
    }
}
