//! Proposed `skater_pool` child: actual Skeleton mode write for Motion Begin.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    generated_motion_load::construct::control::PhysicsAnimationMode,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum MotionForcePhysicsError {
    Pool(SimulationSkaterPoolError),
    ForeignOwner,
}
impl std::fmt::Display for MotionForcePhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "Motion Skeleton mode: {error}"),
            Self::ForeignOwner => {
                f.write_str("Motion Skeleton mode belongs to another physical owner")
            }
        }
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_motion_force_physics(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
        mode: PhysicsAnimationMode,
    ) -> Result<(), MotionForcePhysicsError> {
        use MotionForcePhysicsError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let (_, owner) = self
            .try_physical_member_owner(backend, skeleton)
            .map_err(E::Pool)?;
        if !Arc::ptr_eq(owner, expected_owner) {
            return Err(E::ForeignOwner);
        }
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            return Err(E::Pool(SimulationSkaterPoolError::WrongMemberKind));
        };
        candidate.owner.set_motion_physics_animation_mode(mode);
        Ok(())
    }
}
