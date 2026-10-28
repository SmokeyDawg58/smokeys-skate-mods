//! Outer `Skeleton::Init` named indices, 82b62768..82b62934. This owner is
//! retained through Body handoff; it does not stand in for the remaining Init.
use crate::{ragdoll::SkeletonPartIndex, skeleton_data::SkeletonData};
use skate2_formats::andale::PackedFastString;

#[cfg(test)]
#[path = "skeleton_init_state_tests.rs"]
mod tests;

const NAMES: [&str; 7] = [
    "Truck_Back",
    "Truck_Front",
    "Left_WheelFront",
    "Right_WheelFront",
    "Left_WheelBack",
    "Right_WheelBack",
    "Head",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonInitIndicesError {
    AlreadyAttempted,
    MissingHead,
    HeadParentUnavailable { head: usize },
    InvalidHeadParent { parent: i32, animation_bones: usize },
    UnmappedHeadParent { parent: usize },
}
impl std::fmt::Display for SkeletonInitIndicesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Skeleton Init indices: {self:?}")
    }
}
impl std::error::Error for SkeletonInitIndicesError {}

pub(crate) struct SkeletonInitState {
    /// Constructor82b56e70/e74 initializes these before Init writes them again.
    pub(crate) animation_bones: usize,
    pub(crate) trajectory_node: i32,
    /// Signed -1 survives a missing truck/wheel lookup. None means unwritten.
    pub(crate) named_bones: [Option<i32>; 7],
    pub(crate) head_parent: Option<i32>,
    pub(crate) head_parent_part: Option<SkeletonPartIndex>,
    /// Outer None means unwritten; inner None is source `INVALID_PART_INDEX`.
    #[expect(
        clippy::option_option,
        reason = "distinct native unwritten, written-invalid, and written-valid Part states"
    )]
    pub(crate) head_grandparent_part: Option<Option<SkeletonPartIndex>>,
    attempted: bool,
    complete: bool,
}
impl SkeletonInitState {
    pub(crate) const fn new() -> Self {
        Self {
            animation_bones: 0,
            trajectory_node: 0,
            named_bones: [None; 7],
            head_parent: None,
            head_parent_part: None,
            head_grandparent_part: None,
            attempted: false,
            complete: false,
        }
    }

    /// Invoke after SAME Data.Init and IK.Init, before `SkeletonBody` allocation.
    /// Missing truck/wheel names are valid source -1 values. The head parent
    /// read and two Part assertions stop at their own stages, preserving writes.
    pub(crate) fn try_initialize(
        &mut self,
        data: &SkeletonData,
    ) -> Result<(), SkeletonInitIndicesError> {
        use SkeletonInitIndicesError as Error;
        if self.attempted {
            return Err(Error::AlreadyAttempted);
        }
        self.attempted = true;
        let hierarchy = data.hierarchy();
        self.trajectory_node = 0;
        self.animation_bones = hierarchy.bone_names.len();
        for (slot, name) in self.named_bones.iter_mut().zip(NAMES) {
            let name = PackedFastString::<5>::encode(name).expect("original fixed bone name");
            *slot = Some(
                hierarchy
                    .packed_bone_names
                    .iter()
                    .position(|candidate| *candidate == name)
                    .map_or(-1, |index| {
                        i32::try_from(index).expect("checked SkeletonData capacity")
                    }),
            );
        }
        let head = self.named_bones[6].expect("completed head lookup");
        let head = usize::try_from(head).map_err(|_| Error::MissingHead)?;
        let parent = *hierarchy
            .parents
            .get(head)
            .ok_or(Error::HeadParentUnavailable { head })?;
        self.head_parent = Some(parent);
        let parent_index = usize::try_from(parent)
            .ok()
            .filter(|index| *index < self.animation_bones)
            .ok_or(Error::InvalidHeadParent {
                parent,
                animation_bones: self.animation_bones,
            })?;
        let part = data.part_from_bone()[parent_index].ok_or(Error::UnmappedHeadParent {
            parent: parent_index,
        })?;
        self.head_parent_part = Some(part);
        self.head_grandparent_part = Some(data.parents()[part.index()]);
        self.complete = true;
        Ok(())
    }

    pub(crate) const fn complete(&self) -> bool {
        self.complete
    }
}
