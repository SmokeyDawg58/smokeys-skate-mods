//! Physical-owner checked current pose preparation, through `ProcessData` f25c.
//! Explicit input and IK initialization boundaries await the native Actor.
use super::{SimulationAssemblyConstraintSession, physical_island::SimulationPhysicalBodyError};
#[cfg(test)]
use crate::skeleton_current_animation::{SkeletonAdjustmentInput, SkeletonCurrentAnimationCvars};
use crate::{
    animation::AnimationMatrix,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonAnimationError,
    },
    skateboard_hook::SkateboardHookDefinition,
    skeleton_skateboard_offset::SkeletonSkateboardOffsetCvars,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::fmt;

#[derive(Debug)]
pub(crate) enum PhysicalPlayerAnimationError<C> {
    Owner(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Update(SimulationSkeletonAnimationError<C>),
}
impl<C: fmt::Debug> fmt::Display for PhysicalPlayerAnimationError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "current animation physical owner: {e}"),
            Self::Pool(e) => write!(f, "current animation pool: {e}"),
            Self::Update(e) => write!(f, "{e}"),
        }
    }
}
impl<C: fmt::Debug> std::error::Error for PhysicalPlayerAnimationError<C> {}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Actual Update1 call at82e03718 through Skeleton output/teleport suffix.
    pub(crate) fn try_process_player_skeleton_data_prefix<
        N: crate::reckoning_numeric::ReckoningNumeric,
    >(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        continuation: &super::physical_update1_prefix::Update1AfterTeleportCheck,
        numeric: &mut N,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::ProcessDataPrefixError<N::Error>>
    {
        use crate::dynamic_assembly_session::skater_pool::ProcessDataPrefixError as E;
        for handle in [board, skeleton] {
            let owner = self
                .physical_body_owner_for_contacts(handle)
                .map_err(E::Owner)?;
            if !std::sync::Arc::ptr_eq(owner, &continuation.owner) {
                return Err(E::ForeignOwner);
            }
        }
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_process_paired_skeleton_data_prefix(
                &mut self.backend,
                board,
                skeleton,
                &continuation.owner,
                &continuation.input,
                &continuation.processed,
                &self.debug_overlay,
                numeric,
            )
    }

    pub(crate) fn try_set_skeleton_skateboard_offset<C: SkeletonSkateboardOffsetCvars>(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        transform: &AnimationMatrix,
        cvars: &mut C,
    ) -> Result<(), PhysicalPlayerAnimationError<C::Error>> {
        self.physical_body_owner_for_contacts(skeleton)
            .map_err(PhysicalPlayerAnimationError::Owner)?;
        self.skaters
            .as_mut()
            .ok_or(PhysicalPlayerAnimationError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_set_skateboard_offset(&self.backend, skeleton, transform, cvars)
            .map_err(PhysicalPlayerAnimationError::Update)
    }

    #[cfg(test)]
    pub(crate) fn try_process_skeleton_current_animation<C: SkeletonCurrentAnimationCvars>(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        pose: &[AnimationMatrix],
        input: &SkeletonAdjustmentInput,
        passed_deck_zy_delta: &mut f32,
        cvars: &mut C,
    ) -> Result<(), PhysicalPlayerAnimationError<C::Error>> {
        self.physical_body_owner_for_contacts(skeleton)
            .map_err(PhysicalPlayerAnimationError::Owner)?;
        self.skaters
            .as_mut()
            .ok_or(PhysicalPlayerAnimationError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_process_current_animation(
                &self.backend,
                skeleton,
                pose,
                input,
                passed_deck_zy_delta,
                cvars,
            )
            .map_err(PhysicalPlayerAnimationError::Update)
    }
}
