//! Child of `skeleton_current_animation`; same actual runtime and target owners.
use super::SkeletonCurrentAnimation;
use crate::dynamic_assembly_session::skeleton_normal::post_wipeout_foot_live::FootIkRetainedOwners;
impl SkeletonCurrentAnimation {
    pub(crate) fn post_wipeout_foot_parts_mut(&mut self) -> Option<FootIkRetainedOwners<'_>> {
        Some(self.ik_runtime.as_mut()?.post_wipeout_foot_owners(&self.ik))
    }
}
