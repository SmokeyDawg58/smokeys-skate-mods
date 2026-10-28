//! Child of Reckoning: native Skeleton.GetLaunchInfo reads +e0 verbatim.
use super::Reckoning;
use crate::skeleton_root_transforms::SkeletonRootMatrix;

impl<A> Reckoning<A> {
    /// Read the SAME inverse retained by the original constructor/update.
    /// No completed-phase requirement or homogeneous-W reconstruction.
    pub(crate) fn launch_inverse_transform(&self) -> &SkeletonRootMatrix {
        &self.inverse_transform
    }
}
