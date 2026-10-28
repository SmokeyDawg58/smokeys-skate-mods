//! SAME-pair Skeleton COM publication from retained outer post-physics records.
//! This diagnostic wrapper additionally requires a current completed history frame.
use crate::{
    dynamic_assembly_session::{
        skater_pool::{SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError},
        skeleton_normal::history_owner::SkeletonHistoryFrameProgress,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_phys_out::{PhysOutPublicationError, com::PhysOutComSource},
    skeleton_post_physics_records::SkeletonPostPhysicsRecords,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum PhysicalPlayerComError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    WrongFrame,
    IncompleteHistory,
    PoisonedOutput,
    Publication(PhysOutPublicationError<Infallible>),
}
struct Source<'a>(&'a SkeletonPostPhysicsRecords);
impl PhysOutComSource for Source<'_> {
    type Error = Infallible;
    fn centre_of_mass_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.0.centre_of_mass_velocity_typed())
    }
    fn centre_of_mass(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.0.centre_of_mass_typed())
    }
}
impl PhysicalAssemblyWorld {
    /// Host freshness checks do not sample numeric history before native loads.
    /// No full Skeleton Fill or Actor phase completion is implied.
    pub(crate) fn try_publish_pair_skeleton_com(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalPlayerComError> {
        use PhysicalPlayerComError as E;
        let frame = self.contact_spies.as_ref().ok_or(E::WrongFrame)?;
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(|_| E::WrongFrame)?;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |skeleton, owner| {
                let state = skeleton.physics_history_frame_state();
                if state.progress() != SkeletonHistoryFrameProgress::Complete {
                    return Err(E::IncompleteHistory);
                }
                if !state
                    .frame()
                    .is_some_and(|history| Arc::ptr_eq(history, frame))
                {
                    return Err(E::WrongFrame);
                }
                owner
                    .phys_out()
                    .lock()
                    .map_err(|_| E::PoisonedOutput)?
                    .try_publish_skeleton_com(&mut Source(skeleton.post_physics_records()))
                    .map_err(E::Publication)
            })
            .map_err(E::Pair)?
    }
}
impl std::fmt::Display for PhysicalPlayerComError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("COM skater pool unavailable"),
            Self::Pair(e) => write!(f, "COM pair: {e}"),
            Self::WrongFrame => f.write_str("COM history does not match current world frame"),
            Self::IncompleteHistory => f.write_str("COM history incomplete"),
            Self::PoisonedOutput => f.write_str("COM output lock poisoned"),
            Self::Publication(e) => write!(f, "COM publication: {e:?}"),
        }
    }
}
