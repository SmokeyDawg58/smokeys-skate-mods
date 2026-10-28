//! Actual retained contact arrays and live wheel bodies for Fill82b2f950..fc20.
use super::{
    board_fill_pumping::Update5BeforeBoardWheelContacts,
    physical_board_forces::PhysicalPlayerBoardForceError,
};
use crate::{
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError,
        },
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill_contacts::{
        BoardFillContactSource, BoardFillContactsError,
    },
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_contact_surface::SkateboardContactSurface,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

type Contacts = Arc<Mutex<SkateboardContactSurface>>;
#[derive(Debug)]
pub(crate) enum BoardContactSourceError {
    Board(PhysicalPlayerBoardForceError),
    Pool(SimulationSkaterPoolError),
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
    PoisonedContacts,
    Unknown(&'static str, usize),
}
impl std::fmt::Display for BoardContactSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Board(e) => write!(f, "contact board: {e}"),
            Self::Pool(e) => write!(f, "contact pool: {e}"),
            Self::Part(e) => write!(f, "contact wheel Part: {e}"),
            Self::Body(e) => write!(f, "contact wheel body: {e}"),
            Self::PoisonedContacts => f.write_str("board contacts lock poisoned"),
            Self::Unknown(field, part) => write!(f, "board Part{part} {field} is unknown"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum BoardContactOutputError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Output(BoardFillContactsError<BoardContactSourceError>),
}
impl std::fmt::Display for BoardContactOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board contact output requires skater pool"),
            Self::Pair(e) => write!(f, "board contact pair: {e:?}"),
            Self::ForeignOwner => f.write_str("board contact continuation has foreign owner"),
            Self::PoisonedOutput => f.write_str("board contact output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeBoardDeckLine {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) contacts: Contacts,
    pub(crate) assembly: super::board_fill_wheels::BoardFillAssemblyDescriptor,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_contacts(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardWheelContacts,
    ) -> Result<Update5BeforeBoardDeckLine, BoardContactOutputError> {
        use BoardContactOutputError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardWheelContacts {
            owner: previous_owner,
            assembly,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        let contacts = {
            let mut output = owner.phys_out().lock().map_err(|_| E::PoisonedOutput)?;
            let mut source = Source {
                world: self,
                board,
                captured: None,
            };
            output
                .try_fill_board_contacts(&mut source)
                .map_err(E::Output)?;
            source
                .captured
                .expect("successful source captured contact owner")
        };
        Ok(Update5BeforeBoardDeckLine {
            owner,
            contacts,
            assembly,
        })
    }
}
struct Source<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    captured: Option<Contacts>,
}
impl BoardFillContactSource for Source<'_> {
    type Error = BoardContactSourceError;
    type Contacts = Contacts;
    fn capture_contacts(&mut self) -> Result<Contacts, Self::Error> {
        let contacts = self
            .world
            .try_with_board_force_runtime(self.board, |runtime| {
                Arc::clone(&runtime.contact_surface)
            })
            .map_err(Self::Error::Board)?;
        self.captured = Some(Arc::clone(&contacts));
        Ok(contacts)
    }
    fn in_contact(&mut self, contacts: &Contacts, part: usize) -> Result<bool, Self::Error> {
        contacts
            .lock()
            .map_err(|_| Self::Error::PoisonedContacts)?
            .collision_state
            .in_contact[part]
            .ok_or(Self::Error::Unknown("inContact", part))
    }
    fn wheel_velocity(&mut self, wheel: usize) -> Result<SkeletonRootVector, Self::Error> {
        let body = self
            .world
            .board(self.board)
            .map_err(Self::Error::Pool)?
            .try_part_contact_spy_observation(&self.world.backend, wheel)
            .map_err(Self::Error::Part)?;
        let xyz = self
            .world
            .backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(body)
            .map_err(Self::Error::Body)?;
        Ok(SkeletonRootVector { xyz, w: None })
    }
    fn position(
        &mut self,
        contacts: &Contacts,
        part: usize,
    ) -> Result<SkeletonRootVector, Self::Error> {
        contacts
            .lock()
            .map_err(|_| Self::Error::PoisonedContacts)?
            .collision_state
            .position[part]
            .ok_or(Self::Error::Unknown("position", part))
    }
    fn normal(
        &mut self,
        contacts: &Contacts,
        part: usize,
    ) -> Result<SkeletonRootVector, Self::Error> {
        contacts
            .lock()
            .map_err(|_| Self::Error::PoisonedContacts)?
            .collision_state
            .normal[part]
            .ok_or(Self::Error::Unknown("normal", part))
    }
    fn audio(&mut self, contacts: &Contacts, part: usize) -> Result<u32, Self::Error> {
        contacts
            .lock()
            .map_err(|_| Self::Error::PoisonedContacts)?
            .collision_state
            .audio_type[part]
            .ok_or(Self::Error::Unknown("audio surface", part))
    }
    fn pattern(&mut self, contacts: &Contacts, part: usize) -> Result<u32, Self::Error> {
        contacts
            .lock()
            .map_err(|_| Self::Error::PoisonedContacts)?
            .collision_state
            .pattern_type[part]
            .ok_or(Self::Error::Unknown("surface pattern", part))
    }
    fn overall_surface(&mut self, contacts: &Contacts) -> Result<u32, Self::Error> {
        contacts
            .lock()
            .map_err(|_| Self::Error::PoisonedContacts)?
            .overall()
            .ok_or(Self::Error::Unknown("overall surface", 6))
    }
}
