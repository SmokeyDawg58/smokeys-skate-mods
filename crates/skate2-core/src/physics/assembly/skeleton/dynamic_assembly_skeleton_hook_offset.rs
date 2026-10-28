//! Same actual hook Part0 then predicted-board stores, `AddOffsetToHooks`.
use super::{SkeletonDriveOwnerError, SkeletonDriveOwnerStage, SkeletonNormalAssembly};
use crate::{
    dynamic_assembly_session::hook_pool::SimulationHookAssemblyPool,
    skateboard_assembly::SkateboardSimulationBackend,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use skate2_formats::collision::AssemblyPartMaterial;
impl SkeletonNormalAssembly {
    pub(crate) fn try_add_offset_to_hooks<H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        offset: SkeletonRootVector,
    ) -> Result<(), SkeletonDriveOwnerError> {
        self.check_drive_backend(backend)?;
        let runtime = self
            .drives
            .as_ref()
            .ok_or(SkeletonDriveOwnerError::Missing)?;
        if runtime.stage != SkeletonDriveOwnerStage::Complete {
            return Err(SkeletonDriveOwnerError::Incomplete);
        }
        let hook = runtime
            .hook
            .as_ref()
            .ok_or(SkeletonDriveOwnerError::Incomplete)?;
        let matrix = hooks
            .skeleton_hook_session(hook)
            .map_err(SkeletonDriveOwnerError::Hook)?
            .try_live_part_transform_typed(backend, 0)
            .map_err(|e| {
                SkeletonDriveOwnerError::Hook(
                    crate::dynamic_assembly_session::hook_pool::SimulationHookPoolError::Transform(
                        e,
                    ),
                )
            })?;
        let mut columns = *matrix.columns();
        columns[3] = columns[3].zip(offset, |a, b| a + b);
        hooks
            .try_set_skeleton_hook_part_typed(
                backend,
                hook,
                0,
                &SkeletonRootMatrix::from_columns(columns),
            )
            .map_err(SkeletonDriveOwnerError::Hook)?;
        // Source does not alter Part1. Predicted position changes only AFTER
        // successful actual Part.SetTransform, including its body/cache writes.
        let previous = self.root_transforms.predicted_board_position();
        self.root_transforms
            .publish_board_prediction(previous.zip(offset, |a, b| a + b));
        Ok(())
    }
}
