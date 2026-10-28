//! Actual skater-pool ownership for `Skeleton.AddOffsetToHooks`.
use super::{
    MemberContent, SimulationHookAssemblyPool, SimulationSkaterAssemblyHandle,
    SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
};
use crate::{
    skateboard_assembly::SkateboardSimulationBackend, skeleton_root_transforms::SkeletonRootVector,
};
use skate2_formats::collision::AssemblyPartMaterial;
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_add_skeleton_hook_offset<H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        handle: &SimulationSkaterAssemblyHandle,
        offset: SkeletonRootVector,
    ) -> Result<(), SimulationSkaterPoolError> {
        let index = self.skeleton_mutation_index(backend, handle)?;
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(SimulationSkaterPoolError::Hook)?;
        self.invalidate_refresh();
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("checked skeleton kind")
        };
        candidate
            .owner
            .try_add_offset_to_hooks(backend, hooks, offset)
            .map_err(SimulationSkaterPoolError::SkeletonDrive)
    }
}
