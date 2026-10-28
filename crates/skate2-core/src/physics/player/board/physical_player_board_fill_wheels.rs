//! Same captured assembly, fresh current body reads, no contact-frame dependency.
use super::board_fill_forward::Update5BeforeBoardWheelPositions;
use crate::{
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError,
        },
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::board_fill_wheels::{
        BoardFillWheelPositionSource, BoardWheelPositionOutputError,
    },
    simulation_rigid_body_pool::{SimulationRigidBodyObservation, SimulationRigidBodyPoolError},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

/// Retained native assembly pointer f430/f438, never a snapshot of its bodies.
#[derive(Clone)]
pub(crate) struct BoardFillAssemblyDescriptor(SimulationSkaterAssemblyHandle);
impl BoardFillAssemblyDescriptor {
    pub(crate) fn capture(board: &SimulationSkaterAssemblyHandle) -> Self {
        Self(board.clone())
    }
    pub(crate) fn matches(&self, board: &SimulationSkaterAssemblyHandle) -> bool {
        self.0.same_as(board)
    }
}
#[derive(Debug)]
pub(crate) enum BoardWheelPositionSourceError {
    Pool(SimulationSkaterPoolError),
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
}
impl std::fmt::Display for BoardWheelPositionSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::Part(e) => e.fmt(f),
            Self::Body(e) => e.fmt(f),
        }
    }
}
#[derive(Debug)]
pub(crate) enum BoardWheelPositionError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    ForeignAssembly,
    PoisonedOutput,
    Output(BoardWheelPositionOutputError<BoardWheelPositionSourceError>),
}
impl std::fmt::Display for BoardWheelPositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("wheel output requires actual pool"),
            Self::Pair(e) => write!(f, "wheel output pair: {e:?}"),
            Self::ForeignOwner => f.write_str("wheel continuation has foreign owner"),
            Self::ForeignAssembly => {
                f.write_str("wheel continuation has foreign captured assembly")
            }
            Self::PoisonedOutput => f.write_str("wheel output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeBoardGroundState {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_wheel_positions(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardWheelPositions,
    ) -> Result<Update5BeforeBoardGroundState, BoardWheelPositionError> {
        use BoardWheelPositionError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardWheelPositions {
            owner: previous_owner,
            assembly,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        if !assembly.matches(board) {
            return Err(E::ForeignAssembly);
        }
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_fill_board_wheel_positions(&mut Source {
                world: self,
                assembly: &assembly,
            })
            .map_err(E::Output)?;
        Ok(Update5BeforeBoardGroundState { owner })
    }
}
struct Source<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    assembly: &'a BoardFillAssemblyDescriptor,
}
impl BoardFillWheelPositionSource for Source<'_> {
    type Error = BoardWheelPositionSourceError;
    type Parts = SimulationSkaterAssemblyHandle;
    type Body = SimulationRigidBodyObservation;
    fn capture_parts(&mut self) -> Result<Self::Parts, Self::Error> {
        // Validate the captured retained session; do not read any Part body yet.
        self.world
            .board(&self.assembly.0)
            .map_err(Self::Error::Pool)?;
        Ok(self.assembly.0.clone())
    }
    fn body(&mut self, parts: &Self::Parts, wheel: usize) -> Result<Self::Body, Self::Error> {
        self.world
            .board(parts)
            .map_err(Self::Error::Pool)?
            .try_part_contact_spy_observation(&self.world.backend, wheel)
            .map_err(Self::Error::Part)
    }
    fn position(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error> {
        let xyz = self
            .world
            .backend
            .collision_body_pool()
            .try_contact_spy_center_of_mass(*body)
            .map_err(Self::Error::Body)?;
        Ok(SkeletonRootVector { xyz, w: None })
    }
}
