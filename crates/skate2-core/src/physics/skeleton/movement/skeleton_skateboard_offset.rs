//! Retained `Skeleton::UpdateSkateboardOffsetTransform` (0x82b41c80).
//! Scalar f32 decay, full-lane affine composition and the accepted host
//! normalization graph preserve source ordering. Xenon exceptional-result
//! bits, reciprocal estimates and FP status remain unproven.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "offset activation awaits live Skeleton producers")
)]

use crate::{
    animation::AnimationMatrix,
    skateboard_spawn_math::{
        SkateboardSpawnMathNumeric, skateboard_spawn_compose, skateboard_spawn_normalize,
    },
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_ik_targets::SkeletonIkTargets,
};

pub(crate) trait SkeletonSkateboardOffsetCvars {
    type Error;
    /// Current `kLandingAdjustFramesToBlendAwayOffset`: reference8308e9d4,
    /// target+28, float+d8. Sample after the full transform and `JustSet` stores.
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error>;
}

/// One non-Clone offset projection on the existing Skeleton owner. The
/// offboard and landing producers activate its shared full/Y state. Counts
/// retain their original float type and unusual values.
pub(crate) struct SkeletonSkateboardOffset {
    pub(crate) transform: AnimationMatrix,
    pub(crate) full_count: f32,
    pub(crate) vertical_count: f32,
    pub(crate) full_just_set: bool,
    pub(crate) vertical_just_set: bool,
}

impl SkeletonSkateboardOffset {
    /// Initialized projection of Skeleton ctor0x82b56d70: I/J/K/P0,
    /// all four W lanes zero, counts0 and both `JustSet` bytes0.
    pub(crate) const fn new() -> Self {
        Self {
            transform: FRESH_VOLUME_TRANSFORM,
            full_count: 0.0,
            vertical_count: 0.0,
            full_just_set: false,
            vertical_just_set: false,
        }
    }

    /// `SetSkateboardOffsetTransform`0x82b3b178..0x82b3b1c8. A failed late
    /// cvar read retains the newly copied transform and `JustSet1`, with old count.
    /// The independent Y count/JustSet are untouched by this source setter.
    pub(crate) fn try_set_full_transform<C: SkeletonSkateboardOffsetCvars>(
        &mut self,
        transform: &AnimationMatrix,
        cvars: &mut C,
    ) -> Result<(), C::Error> {
        self.transform = *transform;
        self.full_just_set = true;
        self.full_count = cvars.frames_to_blend_away_offset()?;
        Ok(())
    }

    /// Source0x82b41c80..0x82b42068. Full then Y decay; update volume0,
    /// update retained IK targets, then clear both `JustSet` bytes. No source
    /// assertion, cvar or fallible callback occurs inside this routine.
    pub(crate) fn update(
        &mut self,
        volume: &mut AnimationMatrix,
        ik: &mut SkeletonIkTargets,
        numeric: &mut impl SkateboardSpawnMathNumeric,
    ) {
        let mut apply = false;
        if active_count(self.full_count) {
            apply = true;
            if !self.full_just_set {
                let difference = self.full_count - 1.0;
                let ratio = difference / self.full_count;
                let weight = ratio * ratio;
                let identity_weight = 1.0 - weight;
                // Native interpolation commits X, Y, Z before the inline
                // normalization. It is not identity + (old-identity)*weight.
                for axis in 0..3 {
                    for lane in 0..4 {
                        let index = axis * 4 + lane;
                        let identity_product = FRESH_VOLUME_TRANSFORM[index] * identity_weight;
                        self.transform[index] =
                            self.transform[index].mul_add(weight, identity_product);
                    }
                }
                let mut position: [f32; 4] =
                    std::array::from_fn(|lane| self.transform[12 + lane] * weight);
                // vrlimi mask4 at0x82b41e54 restores original Y after the
                // full4 multiply. W remains scaled; Y has its own counter.
                position[1] = self.transform[13];
                let mut normalized = skateboard_spawn_normalize(&self.transform, numeric);
                normalized[12..16].copy_from_slice(&position);
                self.transform = normalized;
                self.full_count -= 1.0;
            }
        }
        if active_count(self.vertical_count) {
            apply = true;
            if !self.vertical_just_set {
                let difference = self.vertical_count - 1.0;
                let ratio = difference / self.vertical_count;
                let weight = ratio * ratio;
                self.transform[13] *= weight;
                self.vertical_count -= 1.0;
            }
        }
        if apply {
            // Applied even if this call's decrement makes a count <=0.
            // Inner W is unused; offset's outer W participates in full4 FMAs.
            *volume = skateboard_spawn_compose(&self.transform, volume);
            ik.add_offset_transform(&self.transform);
        }
        self.full_just_set = false;
        self.vertical_just_set = false;
    }
}

fn active_count(count: f32) -> bool {
    // Raw 0x40990258/0x40990084 at0x82b41cbc/0x82b41f1c are BO4/BI25:
    // branch when CR6.GT is CLEAR, including unordered. Only ordered >0 enters.
    count > 0.0
}

#[cfg(test)]
#[path = "skeleton_skateboard_offset_tests.rs"]
mod tests;
