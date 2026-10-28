//! Checked existing Skeleton owner -> current animation preparation. No body,
//! drive, publication, or physics-history authority is recreated or changed.
use super::super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
#[cfg(test)]
use crate::skeleton_current_animation::{SkeletonAdjustmentInput, SkeletonCurrentAnimationCvars};
use crate::{
    animation::AnimationMatrix, attribute_runtime::AttributeRuntimeError,
    skeleton_current_animation::SkeletonCurrentAnimationError,
    skeleton_skateboard_offset::SkeletonSkateboardOffsetCvars,
};
use std::fmt;

#[derive(Debug)]
pub(crate) enum SimulationSkeletonAnimationError<C> {
    Pool(SimulationSkaterPoolError),
    Detached,
    NotInitialized,
    OffsetCvar(C),
    Update(SkeletonCurrentAnimationError<AttributeRuntimeError, C>),
}
impl<C: fmt::Debug> fmt::Display for SimulationSkeletonAnimationError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(e) => write!(f, "current animation owner: {e}"),
            Self::Detached => f.write_str("current animation owner is detached"),
            Self::NotInitialized => {
                f.write_str("current animation targets/provider not initialized")
            }
            Self::OffsetCvar(e) => write!(f, "skateboard offset cvar: {e:?}"),
            Self::Update(e) => write!(f, "{e}"),
        }
    }
}
impl<C: fmt::Debug> std::error::Error for SimulationSkeletonAnimationError<C> {}

impl SimulationSkaterAssemblyPool {
    /// Checked host ownership around source setter 0x82b3b178. The native
    /// skitching caller's transform producer remains a separate dependency.
    pub(crate) fn try_set_skateboard_offset<C: SkeletonSkateboardOffsetCvars>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        transform: &AnimationMatrix,
        cvars: &mut C,
    ) -> Result<(), SimulationSkeletonAnimationError<C::Error>> {
        use SimulationSkeletonAnimationError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        self.try_physical_member_owner(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        if !candidate.owner().session().is_attached() {
            return Err(E::Detached);
        }
        candidate
            .owner
            .try_set_skateboard_offset(transform, cvars)
            .map_err(E::OffsetCvar)
    }

    #[cfg(test)]
    pub(crate) fn try_process_current_animation<C: SkeletonCurrentAnimationCvars>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        pose: &[AnimationMatrix],
        input: &SkeletonAdjustmentInput,
        passed_deck_zy_delta: &mut f32,
        cvars: &mut C,
    ) -> Result<(), SimulationSkeletonAnimationError<C::Error>> {
        use SimulationSkeletonAnimationError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        self.try_physical_member_owner(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("kind checked")
        };
        if !candidate.owner().session().is_attached() {
            return Err(E::Detached);
        }
        if !candidate.owner().current_animation_initialized() {
            return Err(E::NotInitialized);
        }
        candidate
            .owner
            .try_process_current_animation(pose, input, passed_deck_zy_delta, cvars)
            .map_err(E::Update)
    }
}
