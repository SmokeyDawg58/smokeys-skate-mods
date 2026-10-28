//! Same-pair selected surface motion publication. The successful Reckoning
//! state phase is retained across solve; motion publication must match this
//! current contact frame. Neither check manufactures a native Actor schedule.
use super::{
    SimulationAssemblyConstraintSession, physical_phys_out::PhysicalPlayerPhysOutOwnerError,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_phys_out::surface_motion::{
        PhysOutSurfaceMotionError, PhysOutSurfaceMotionSource,
    },
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    reckoning_numeric::ReckoningNumeric,
    skateboard_hook::SkateboardHookDefinition,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardSurfaceMotionError<N> {
    Pair(PhysicalPlayerPhysOutOwnerError),
    PoisonedReckoning,
    PoisonedProcessed,
    StaleReckoning,
    WrongFrame,
    ForeignProcessedOutput,
    IncompleteDeck,
    Publication(PhysOutSurfaceMotionError<PhysicalPlayerSurfaceSourceError, N>),
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PhysicalPlayerSurfaceSourceError {
    ForeignReckoning,
    ForeignProcessed,
    IncompleteDeck,
}
struct Source<'a> {
    reckoning: &'a Reckoning<ReckoningVaultAttributes>,
    processed: &'a SkeletonProcessedContact,
}
impl<'a> PhysOutSurfaceMotionSource for Source<'a> {
    type Error = PhysicalPlayerSurfaceSourceError;
    type Reckoning = &'a Reckoning<ReckoningVaultAttributes>;
    type Processed = &'a SkeletonProcessedContact;
    fn capture_reckoning(&mut self) -> Result<Self::Reckoning, Self::Error> {
        Ok(self.reckoning)
    }
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error> {
        Ok(self.processed)
    }
    fn ground_normal(
        &mut self,
        token: &Self::Reckoning,
    ) -> Result<SkeletonRootVector, Self::Error> {
        if !std::ptr::eq(*token, self.reckoning) {
            return Err(Self::Error::ForeignReckoning);
        }
        Ok(token.ground_normal())
    }
    fn deck_z(&mut self, token: &Self::Processed) -> Result<SkeletonRootVector, Self::Error> {
        if !std::ptr::eq(*token, self.processed) {
            return Err(Self::Error::ForeignProcessed);
        }
        token
            .completed_deck_transform()
            .map(|matrix| matrix.columns()[2])
            .ok_or(Self::Error::IncompleteDeck)
    }
    fn ground_transform(
        &mut self,
        token: &Self::Reckoning,
    ) -> Result<SkeletonRootMatrix, Self::Error> {
        if !std::ptr::eq(*token, self.reckoning) {
            return Err(Self::Error::ForeignReckoning);
        }
        Ok(*token.ground_transform())
    }
}
impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    pub(crate) fn try_publish_pair_board_surface_motion<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        phase: &PhysicalPlayerReckoningPhase,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerBoardSurfaceMotionError<N::Error>> {
        use PhysicalPlayerBoardSurfaceMotionError as E;
        // Existing bridge validates actual board/skeleton membership and shared owner.
        self.try_with_pair_phys_out(board, skeleton, |_| ())
            .map_err(E::Pair)?;
        let owner = Arc::clone(
            self.physical_body_owner_for_contacts(board)
                .map_err(|e| E::Pair(PhysicalPlayerPhysOutOwnerError::Body(e)))?,
        );
        let retained = owner.reckoning().lock().map_err(|_| E::PoisonedReckoning)?;
        let reckoning = retained
            .try_completed(phase)
            .map_err(|_| E::StaleReckoning)?;
        let processed = Arc::clone(retained.processed());
        let processed = processed.lock().map_err(|_| E::PoisonedProcessed)?;
        let mut output = owner
            .phys_out()
            .lock()
            .map_err(|_| E::Pair(PhysicalPlayerPhysOutOwnerError::Poisoned))?;
        if !processed.prior_output().matches_output_occurrence(&output) {
            return Err(E::ForeignProcessedOutput);
        }
        if processed.completed_deck_transform().is_none() {
            return Err(E::IncompleteDeck);
        }
        let frame = self.contact_spies.as_ref().ok_or(E::WrongFrame)?;
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(|_| E::WrongFrame)?;
        if !output
            .motion_source_frame()
            .is_some_and(|source| Arc::ptr_eq(source, frame))
        {
            return Err(E::WrongFrame);
        }
        // Readiness checks above do not copy numerical input fields. Each actual value
        // read remains inside the leaf's source-ordered callback, under these borrows.
        output
            .try_publish_surface_motion(
                &mut Source {
                    reckoning,
                    processed: &processed,
                },
                numeric,
            )
            .map_err(E::Publication)
    }
}
impl<N: std::fmt::Debug> std::fmt::Display for PhysicalPlayerBoardSurfaceMotionError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(e) => write!(f, "surface motion pair: {e}"),
            Self::PoisonedReckoning => f.write_str("Reckoning lock poisoned"),
            Self::PoisonedProcessed => f.write_str("processed lock poisoned"),
            Self::StaleReckoning => f.write_str("Reckoning phase is not current"),
            Self::WrongFrame => f.write_str("surface motion lacks current frame deck publication"),
            Self::ForeignProcessedOutput => {
                f.write_str("processed owner is not bound to pair output")
            }
            Self::IncompleteDeck => f.write_str("processed deck prefix is incomplete"),
            Self::Publication(e) => write!(f, "surface motion: {e}"),
        }
    }
}
