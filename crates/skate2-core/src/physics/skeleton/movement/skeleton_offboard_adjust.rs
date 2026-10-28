//! Active `Skeleton::UpdateOffboardAdjust`0x82b4c834..0x82b4c924.
//! The caller owns the original category500/state!=505 gate and supplies the
//! current mirrored bit. Current raw pose and retained IK/data authority feed
//! the existing offset owner; this helper neither applies offsets nor solves IK.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "Offboard animation awaits live Actor scheduling")
)]

use crate::{
    animation::AnimationMatrix,
    skeleton_ik_targets::{SkeletonIkTargetError, SkeletonIkTargets},
    skeleton_skateboard_offset::{SkeletonSkateboardOffset, SkeletonSkateboardOffsetCvars},
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonOffboardAdjustError<C> {
    Ik(SkeletonIkTargetError),
    Cvar(C),
}
impl<C: std::fmt::Debug> std::fmt::Display for SkeletonOffboardAdjustError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ik(error) => write!(f, "Offboard hand mapping: {error}"),
            Self::Cvar(error) => write!(f, "Offboard offset blend cvar: {error:?}"),
        }
    }
}
impl<C: std::fmt::Debug> std::error::Error for SkeletonOffboardAdjustError<C> {}

/// Source gate has already selected the active branch. The full transform and
/// full `JustSet` precede the first cvar read; a second independent read precedes
/// the retained Y-position/flag/count stores. Errors preserve completed writes.
pub(crate) fn try_update_offboard_adjust<C: SkeletonSkateboardOffsetCvars>(
    ik: &SkeletonIkTargets,
    pose: &[AnimationMatrix],
    offset: &mut SkeletonSkateboardOffset,
    mirrored: bool,
    cvars: &mut C,
) -> Result<(), SkeletonOffboardAdjustError<C::Error>> {
    let mapping = ik
        .try_hand_to_animation_mapping(pose, mirrored)
        .map_err(SkeletonOffboardAdjustError::Ik)?;
    offset
        .try_set_full_transform(&mapping, cvars)
        .map_err(SkeletonOffboardAdjustError::Cvar)?;
    let vertical_count = cvars
        .frames_to_blend_away_offset()
        .map_err(SkeletonOffboardAdjustError::Cvar)?;
    // Raw c914 reads the SECOND value before c91c/c920/c924 publish these
    // Y stores. Restamping Y copies the original mapping bits, no arithmetic.
    offset.transform[13] = mapping[13];
    offset.vertical_just_set = true;
    offset.vertical_count = vertical_count;
    Ok(())
}

#[cfg(test)]
#[path = "skeleton_offboard_adjust_tests.rs"]
mod tests;
