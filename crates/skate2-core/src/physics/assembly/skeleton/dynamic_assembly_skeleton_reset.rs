//! In-place `Skeleton::ResetSkeleton` (0x82b45bf0..0x82b45dd4).
//! The existing Part/body owners are changed before the same history rereads
//! them. This is neither fresh construction nor the distinct `TeleportReset`.

use super::SkeletonNormalAssembly;
use crate::{
    dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    ragdoll::SKELETON_PART_COUNT, skateboard_assembly::SkateboardSimulationBackend,
    skeleton_physics_history::SkeletonPhysicsHistoryError,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::{convert::Infallible, fmt};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonResetError {
    MissingMappedVolume {
        part: usize,
    },
    Part {
        part: usize,
        source: DynamicAssemblyPartTransformError,
    },
    Motion {
        part: usize,
        source: DynamicAssemblyPartTransformError,
    },
    History(SkeletonPhysicsHistoryError<DynamicAssemblyPartTransformError, Infallible>),
}
impl fmt::Display for SkeletonResetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMappedVolume { part } => {
                write!(f, "Skeleton reset missing mapped Part {part}")
            }
            Self::Part { part, source } => write!(f, "Skeleton reset Part {part}: {source}"),
            Self::Motion { part, source } => write!(f, "Skeleton reset motion {part}: {source}"),
            Self::History(error) => write!(f, "Skeleton reset history: {error}"),
        }
    }
}
impl std::error::Error for SkeletonResetError {}

impl SkeletonNormalAssembly {
    /// Source-ordered reset over current mapped rows and actual Part getters.
    /// Typed Part caches retain unknown W; attached getters retain the existing
    /// explicit canonical-W host body boundary. Failures keep completed stores.
    pub(crate) fn try_reset_skeleton(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
    ) -> Result<(), SkeletonResetError> {
        self.general_update.clear_add_skateboard_error();
        for part in 0..SKELETON_PART_COUNT {
            let mapped = self
                .animation_targets
                .volume_row(part)
                .ok_or(SkeletonResetError::MissingMappedVolume { part })?;
            let requested = self
                .root_transforms
                .animation_to_world()
                .compose(&SkeletonRootMatrix::known(*mapped));
            self.session
                .try_set_part_transform_typed(backend, part, &requested)
                .map_err(|source| SkeletonResetError::Part { part, source })?;
            self.session
                .try_reset_part_motion_host(backend, part)
                .map_err(|source| SkeletonResetError::Motion { part, source })?;
        }
        let session = &self.session;
        self.physics_history
            .try_reset_typed(|part| session.try_live_part_transform_typed(backend, part))
            .map_err(SkeletonResetError::History)?;
        self.current_animation
            .reset_record_after_physics_reset(&mut self.state.proxy_volume_target);
        Ok(())
    }
}

#[cfg(test)]
#[path = "dynamic_assembly_skeleton_reset_tests.rs"]
mod tests;
