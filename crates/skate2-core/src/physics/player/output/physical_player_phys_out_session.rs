//! Same retained pair output occurrence. Individual lifecycle/publication calls
//! remain explicit `HiLOD` stages; borrowing does not execute a native frame.
use super::{SimulationAssemblyConstraintSession, physical_island::SimulationPhysicalBodyError};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    },
    physical_player_phys_out::PhysicalPlayerPhysOut,
    skateboard_hook::SkateboardHookDefinition,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum BoardSurfacePublicationReadError {
    Poisoned,
    NotSelected,
    WrongFrame,
}
#[derive(Debug)]
pub(crate) enum PhysicalPlayerSurfaceOutputError {
    Board(super::physical_board_forces::PhysicalPlayerBoardForceError),
    Pair(PhysicalPlayerPhysOutOwnerError),
    Publication(
        crate::physical_player_phys_out::PhysOutPublicationError<BoardSurfacePublicationReadError>,
    ),
}
impl std::fmt::Display for PhysicalPlayerSurfaceOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Board(e) => write!(f, "surface output board: {e}"),
            Self::Pair(e) => write!(f, "{e}"),
            Self::Publication(e) => write!(f, "surface output: {e:?}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum PhysicalPlayerPhysOutOwnerError {
    Body(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    DifferentPair,
    Poisoned,
}
impl std::fmt::Display for PhysicalPlayerPhysOutOwnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Body(e) => write!(f, "output physical owner: {e}"),
            Self::Pool(e) => write!(f, "output component: {e}"),
            Self::DifferentPair => {
                f.write_str("output board and skeleton belong to different pairs")
            }
            Self::Poisoned => f.write_str("retained output lock poisoned"),
        }
    }
}
impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Selected last-valid copies30334/30370, after preceding Fill stages.
    /// Same pair and frame are host provenance guards. Body pointer identity is
    /// stable under this world borrow; its retained vector is read twice.
    pub(crate) fn try_publish_pair_board_ground_up(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalPlayerSurfaceOutputError> {
        use PhysicalPlayerSurfaceOutputError as E;
        let source = self
            .try_with_board_force_runtime(board, |runtime| Arc::clone(&runtime.contact_surface))
            .map_err(E::Board)?;
        let frame = self.contact_spies.clone();
        let frame_current = frame.as_ref().is_some_and(|frame| {
            frame
                .try_validate_current(&self.contacts, self.backend.collision_body_pool())
                .is_ok()
        });
        self.try_with_pair_phys_out(board, skeleton, |output| {
            output.try_publish_last_ground_up(|| {
                let state = source
                    .lock()
                    .map_err(|_| BoardSurfacePublicationReadError::Poisoned)?;
                if !frame_current
                    || !state
                        .normal_frame
                        .as_ref()
                        .zip(frame.as_ref())
                        .is_some_and(|(a, b)| Arc::ptr_eq(a, b))
                {
                    return Err(BoardSurfacePublicationReadError::WrongFrame);
                }
                state
                    .collision_state
                    .last_valid_normal
                    .ok_or(BoardSurfacePublicationReadError::NotSelected)
            })
        })
        .map_err(E::Pair)?
        .map_err(E::Publication)
    }
    /// Selected `FillPhysOut` store 0x82b2fc20. The actual retained source value
    /// is sampled only after the same pair output's writable check succeeds.
    pub(crate) fn try_publish_pair_board_surface(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalPlayerSurfaceOutputError> {
        use PhysicalPlayerSurfaceOutputError as E;
        let source = self
            .try_with_board_force_runtime(board, |runtime| Arc::clone(&runtime.contact_surface))
            .map_err(E::Board)?;
        self.try_with_pair_phys_out(board, skeleton, |output| {
            output.try_publish_collision_type(|| {
                source
                    .lock()
                    .map_err(|_| BoardSurfacePublicationReadError::Poisoned)?
                    .overall()
                    .ok_or(BoardSurfacePublicationReadError::NotSelected)
            })
        })
        .map_err(E::Pair)?
        .map_err(E::Publication)
    }
    pub(crate) fn try_with_pair_phys_out<T>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(&mut PhysicalPlayerPhysOut) -> T,
    ) -> Result<T, PhysicalPlayerPhysOutOwnerError> {
        use PhysicalPlayerPhysOutOwnerError as E;
        self.board(board).map_err(E::Pool)?;
        self.skeleton(skeleton).map_err(E::Pool)?;
        let board_owner = self
            .physical_body_owner_for_contacts(board)
            .map_err(E::Body)?;
        let skeleton_owner = self
            .physical_body_owner_for_contacts(skeleton)
            .map_err(E::Body)?;
        if !Arc::ptr_eq(board_owner, skeleton_owner) {
            return Err(E::DifferentPair);
        }
        let mut output = board_owner.phys_out().lock().map_err(|_| E::Poisoned)?;
        Ok(operation(&mut output))
    }
}
