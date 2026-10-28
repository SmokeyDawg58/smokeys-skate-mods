//! Same-owner board output82b2f7fc..f94c, before the wheel-contact loop.
use super::{
    board_fill_prefix::Update5BeforeBoardPumping,
    physical_board_forces::PhysicalPlayerBoardForceError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill_pumping::{
        BoardFillPumpingOutputError, BoardFillPumpingSource,
    },
    physical_player_pumping::{PhysicalPlayerPumpingOwner, PumpingError},
    simulation_rigid_body_pool::{SimulationRigidBodyObservation, SimulationRigidBodyPoolError},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum BoardPumpingSourceError {
    Pumping(PumpingError),
    Board(PhysicalPlayerBoardForceError),
    Body(SimulationRigidBodyPoolError),
    Unknown(&'static str),
    Poisoned(&'static str),
}
impl std::fmt::Display for BoardPumpingSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pumping(e) => write!(f, "Pumping output: {e:?}"),
            Self::Board(e) => write!(f, "board output: {e}"),
            Self::Body(e) => write!(f, "board position: {e}"),
            Self::Unknown(field) => write!(f, "board output requires known {field}"),
            Self::Poisoned(owner) => write!(f, "board output {owner} lock poisoned"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum BoardPumpingError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Output(BoardFillPumpingOutputError<BoardPumpingSourceError>),
}
impl std::fmt::Display for BoardPumpingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board output requires actual skater pool"),
            Self::Pair(e) => write!(f, "board output pair: {e:?}"),
            Self::ForeignOwner => f.write_str("board Pumping continuation has a foreign owner"),
            Self::PoisonedOutput => f.write_str("board output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeBoardWheelContacts {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) assembly: super::board_fill_wheels::BoardFillAssemblyDescriptor,
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_pumping(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardPumping,
    ) -> Result<Update5BeforeBoardWheelContacts, BoardPumpingError> {
        use BoardPumpingError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardPumping {
            owner: previous_owner,
            captured_body,
            assembly,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_fill_board_pumping(&mut Source {
                world: self,
                board,
                owner: &owner,
                captured_body,
            })
            .map_err(E::Output)?;
        Ok(Update5BeforeBoardWheelContacts { owner, assembly })
    }
}
struct Source<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    owner: &'a PhysicalPlayerAssemblyOwner,
    captured_body: SimulationRigidBodyObservation,
}
impl Source<'_> {
    fn processed_flag(&self, mask: u32) -> Result<bool, BoardPumpingSourceError> {
        use BoardPumpingSourceError as E;
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned("Reckoning"))?
                .processed(),
        );
        let processed = processed.lock().map_err(|_| E::Poisoned("Processed"))?;
        let (value, known) = processed.known_flags_8c4();
        if known & mask != mask {
            return Err(E::Unknown("Processed+8c4 selected bit"));
        }
        Ok(value & mask != 0)
    }
}
impl BoardFillPumpingSource for Source<'_> {
    type Error = BoardPumpingSourceError;
    type Pumping = Arc<Mutex<PhysicalPlayerPumpingOwner>>;
    fn capture_pumping(&mut self) -> Result<Self::Pumping, Self::Error> {
        Ok(Arc::clone(self.owner.pumping()))
    }
    fn pump_amount(&mut self, pumping: &Self::Pumping) -> Result<f32, Self::Error> {
        pumping
            .lock()
            .map_err(|_| Self::Error::Poisoned("Pumping"))?
            .try_pump_amount()
            .map_err(Self::Error::Pumping)
    }
    fn intentional_pump(&mut self) -> Result<bool, Self::Error> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned("Reckoning"))?
                .processed(),
        );
        let value = processed
            .lock()
            .map_err(|_| Self::Error::Poisoned("Processed"))?
            .physics_state_fields()
            .flags_8cc
            .ok_or(Self::Error::Unknown("Processed+8cc"))?;
        Ok(value & 4 != 0)
    }
    fn abort_push(&mut self) -> Result<bool, Self::Error> {
        self.world
            .try_with_board_force_runtime(self.board, |runtime| runtime.entry.abort_push())
            .map_err(Self::Error::Board)?
            .ok_or(Self::Error::Unknown("board+530 abortPush"))
    }
    fn captured_body_position(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        let xyz = self
            .world
            .backend
            .collision_body_pool()
            .try_contact_spy_center_of_mass(self.captured_body)
            .map_err(Self::Error::Body)?;
        Ok(SkeletonRootVector { xyz, w: None })
    }
    fn board_flipped(&mut self) -> Result<bool, Self::Error> {
        self.processed_flag(0x0010_0000)
    }
    fn has_jump_delta(&mut self) -> Result<bool, Self::Error> {
        self.processed_flag(0x0040_0000)
    }
    fn jump_delta(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.world
            .try_with_board_force_runtime(self.board, |runtime| runtime.jump_delta)
            .map_err(Self::Error::Board)
    }
}
