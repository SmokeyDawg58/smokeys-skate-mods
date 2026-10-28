//! Deck-line output from the actual board record and retained collision owner.
use super::{
    board_fill_contacts::Update5BeforeBoardDeckLine,
    physical_wheel_lines::PhysicalPlayerWheelLineServiceError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill_deck::{
        BoardFillDeckError, BoardFillDeckSource,
    },
    skateboard_body_construction::SkateboardRetainedWheelLines,
    skateboard_contact_surface::SkateboardContactSurface,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
#[derive(Debug)]
pub(crate) enum BoardDeckSourceError {
    LineOwner(PhysicalPlayerWheelLineServiceError<Infallible>),
    Poisoned(&'static str),
    Unknown(&'static str),
}
impl std::fmt::Display for BoardDeckSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LineOwner(e) => write!(f, "board deck line owner: {e}"),
            Self::Poisoned(owner) => write!(f, "board deck {owner} lock poisoned"),
            Self::Unknown(field) => write!(f, "board deck output requires known {field}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum BoardDeckOutputError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Output(BoardFillDeckError<BoardDeckSourceError>),
}
impl std::fmt::Display for BoardDeckOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board deck output requires actual skater pool"),
            Self::Pair(e) => write!(f, "board deck pair: {e:?}"),
            Self::ForeignOwner => f.write_str("board deck continuation has foreign owner"),
            Self::PoisonedOutput => f.write_str("board deck output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeBoardSurfaceMotion {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) assembly: super::board_fill_wheels::BoardFillAssemblyDescriptor,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_deck(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardDeckLine,
    ) -> Result<Update5BeforeBoardSurfaceMotion, BoardDeckOutputError> {
        use BoardDeckOutputError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardDeckLine {
            owner: previous_owner,
            contacts,
            assembly,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_fill_board_deck(&mut Source {
                world: self,
                board,
                contacts,
            })
            .map_err(E::Output)?;
        Ok(Update5BeforeBoardSurfaceMotion { owner, assembly })
    }
}
struct Source<'a> {
    world: &'a PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    contacts: Arc<Mutex<SkateboardContactSurface>>,
}
impl BoardFillDeckSource for Source<'_> {
    type Error = BoardDeckSourceError;
    type Line = Arc<Mutex<SkateboardRetainedWheelLines>>;
    fn capture_line(&mut self) -> Result<Self::Line, Self::Error> {
        self.world
            .try_board_wheel_line_state(self.board)
            .map_err(Self::Error::LineOwner)
    }
    fn valid(&mut self, line: &Self::Line) -> Result<bool, Self::Error> {
        line.lock()
            .map_err(|_| Self::Error::Poisoned("line"))?
            .deck
            .valid
            .ok_or(Self::Error::Unknown("line.valid"))
    }
    fn start(&mut self, line: &Self::Line) -> Result<SkeletonRootVector, Self::Error> {
        line.lock()
            .map_err(|_| Self::Error::Poisoned("line"))?
            .deck
            .start
            .ok_or(Self::Error::Unknown("line.start"))
    }
    fn position(&mut self, line: &Self::Line) -> Result<SkeletonRootVector, Self::Error> {
        line.lock()
            .map_err(|_| Self::Error::Poisoned("line"))?
            .deck
            .position
            .ok_or(Self::Error::Unknown("line.position"))
    }
    fn normal(&mut self, line: &Self::Line) -> Result<SkeletonRootVector, Self::Error> {
        line.lock()
            .map_err(|_| Self::Error::Poisoned("line"))?
            .deck
            .normal
            .ok_or(Self::Error::Unknown("line.normal"))
    }
    fn surface(&mut self, line: &Self::Line) -> Result<u32, Self::Error> {
        line.lock()
            .map_err(|_| Self::Error::Poisoned("line"))?
            .deck
            .surface
            .ok_or(Self::Error::Unknown("line.surface"))
    }
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.contacts
            .lock()
            .map_err(|_| Self::Error::Poisoned("contacts"))?
            .collision_state
            .acceleration[6]
            .ok_or(Self::Error::Unknown("collision acceleration[6]"))
    }
}
