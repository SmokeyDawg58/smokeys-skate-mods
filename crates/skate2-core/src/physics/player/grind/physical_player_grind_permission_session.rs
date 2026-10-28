//! `Update1` world-only flag3ac8 and actual permission latch call3acc.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    skeleton_processed_contact::grind_permission::PermissionLatchError,
};
use std::{fmt, sync::Arc};
#[derive(Debug)]
pub(crate) enum PermissionSessionError {
    Missing(&'static str),
    Poisoned,
    Pair(SimulationSkeletonJumpPairError),
    Latches(PermissionLatchError),
}
impl fmt::Display for PermissionSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "permission session missing {v}"),
            Self::Poisoned => f.write_str("permission owner lock poisoned"),
            Self::Pair(e) => write!(f, "permission pair: {e:?}"),
            Self::Latches(e) => e.fmt(f),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_permission_latches(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), PermissionSessionError> {
        use PermissionSessionError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut processed = processed.lock().map_err(|_| E::Poisoned)?;
        let world_only = owner
            .tail()
            .lock()
            .map_err(|_| E::Poisoned)?
            .collide_with_world_only;
        processed.publish_world_only_collision(world_only);
        let mut prefix = owner.prefix().lock().map_err(|_| E::Poisoned)?;
        processed
            .try_update_permission_latches(
                prefix.as_mut().ok_or(E::Missing("HiLOD prefix"))?,
                cvars,
                &self.debug_overlay,
            )
            .map_err(E::Latches)
    }
}
