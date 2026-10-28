//! Narrow reads of SAME retained Reckoning fields for its actual Fill callback.
use super::Reckoning;
use crate::{
    reckoning_attributes::ReckoningAttributeRead, skeleton_root_transforms::SkeletonRootVector,
};
impl<A: ReckoningAttributeRead> Reckoning<A> {
    pub(crate) fn fill_heading(&self) -> SkeletonRootVector {
        self.heading
    }
    pub(crate) fn fill_is_flipping(&self) -> bool {
        self.is_flipping
    }
    pub(crate) fn fill_is_front_flipping(&self) -> bool {
        self.is_front_flipping
    }
}
