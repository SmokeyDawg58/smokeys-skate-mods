//! Actual contiguous surface-motion caller fdd8..3004c, reusing the existing leaf.
use super::board_fill_deck::Update5BeforeBoardSurfaceMotion;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::surface_motion::{
        PhysOutSurfaceMotionError, PhysOutSurfaceMotionSource,
    },
    physical_player_reckoning_owner::PhysicalPlayerReckoningOwner,
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum BoardSurfaceSourceError {
    Poisoned(&'static str),
    Missing(&'static str),
}
impl std::fmt::Display for BoardSurfaceSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned(owner) => write!(f, "board surface {owner} lock poisoned"),
            Self::Missing(field) => write!(f, "board surface requires {field}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum BoardSurfaceOutputError<N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    PoisonedOutput,
    Output(PhysOutSurfaceMotionError<BoardSurfaceSourceError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for BoardSurfaceOutputError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("board surface output requires actual skater pool"),
            Self::Pair(e) => write!(f, "board surface pair: {e:?}"),
            Self::ForeignOwner => f.write_str("board surface continuation has foreign owner"),
            Self::PoisonedOutput => f.write_str("board surface output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeBoardWobble {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) assembly: super::board_fill_wheels::BoardFillAssemblyDescriptor,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_board_surface<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeBoardSurfaceMotion,
        numeric: &mut N,
    ) -> Result<Update5BeforeBoardWobble, BoardSurfaceOutputError<N::Error>> {
        use BoardSurfaceOutputError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let Update5BeforeBoardSurfaceMotion {
            owner: previous_owner,
            assembly,
        } = before;
        if !Arc::ptr_eq(&owner, &previous_owner) {
            return Err(E::ForeignOwner);
        }
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_publish_surface_motion(&mut Source { owner: &owner }, numeric)
            .map_err(E::Output)?;
        Ok(Update5BeforeBoardWobble { owner, assembly })
    }
}
struct Source<'a> {
    owner: &'a PhysicalPlayerAssemblyOwner,
}
impl<'a> PhysOutSurfaceMotionSource for Source<'a> {
    type Error = BoardSurfaceSourceError;
    type Reckoning = &'a Mutex<PhysicalPlayerReckoningOwner>;
    type Processed = Arc<Mutex<SkeletonProcessedContact>>;
    fn capture_reckoning(&mut self) -> Result<Self::Reckoning, Self::Error> {
        Ok(self.owner.reckoning())
    }
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error> {
        Ok(Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned("Reckoning"))?
                .processed(),
        ))
    }
    fn ground_normal(
        &mut self,
        reckoning: &Self::Reckoning,
    ) -> Result<SkeletonRootVector, Self::Error> {
        Ok(reckoning
            .lock()
            .map_err(|_| Self::Error::Poisoned("Reckoning"))?
            .reckoning()
            .ok_or(Self::Error::Missing("constructed Reckoning"))?
            .ground_normal())
    }
    fn deck_z(&mut self, processed: &Self::Processed) -> Result<SkeletonRootVector, Self::Error> {
        Ok(processed
            .lock()
            .map_err(|_| Self::Error::Poisoned("Processed"))?
            .deck_transform()
            .ok_or(Self::Error::Missing("Processed deck transform"))?
            .columns()[2])
    }
    fn ground_transform(
        &mut self,
        reckoning: &Self::Reckoning,
    ) -> Result<SkeletonRootMatrix, Self::Error> {
        Ok(*reckoning
            .lock()
            .map_err(|_| Self::Error::Poisoned("Reckoning"))?
            .reckoning()
            .ok_or(Self::Error::Missing("constructed Reckoning"))?
            .ground_transform())
    }
}
