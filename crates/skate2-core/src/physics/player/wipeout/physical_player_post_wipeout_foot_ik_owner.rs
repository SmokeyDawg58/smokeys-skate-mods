//! Child of `skeleton_ik`; exclusive field split for the actual postphysics call.
use super::SkeletonIk;
use crate::{
    dynamic_assembly_session::skeleton_normal::post_wipeout_foot_live::FootIkRetainedOwners,
    skeleton_ik_targets::SkeletonIkTargets,
};
impl SkeletonIk {
    pub(crate) fn post_wipeout_foot_owners<'a>(
        &'a mut self,
        targets: &'a SkeletonIkTargets,
    ) -> FootIkRetainedOwners<'a> {
        FootIkRetainedOwners {
            state: &mut self.state,
            data_in: &self.data_in,
            targets,
            geometry: &self.geometry,
            attributes: &mut self.attributes,
        }
    }
}
