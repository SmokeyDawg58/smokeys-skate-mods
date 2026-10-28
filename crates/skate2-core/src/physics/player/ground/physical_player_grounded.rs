//! Grounded update through hooks/reset, active IK and retained drives/forces.
//! The bounded prefix entry remains available for stage-specific diagnostics;
//! the complete entry continues ordinary states through Contact/DriveFrames.
//! Passed processed output, retained processed input and root timestep remain
//! separate caller projections until the native gameplay owner is connected.
use super::{SimulationAssemblyConstraintSession, physical_island::SimulationPhysicalBodyError};
use crate::{
    dynamic_assembly_session::{
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonGroundedError,
        },
        skeleton_normal::general_owner::{
            SkeletonGeneralInput, SkeletonGroundedPassedOutput, SkeletonGroundedProgress,
        },
    },
    reckoning::Reckoning,
    reckoning_attributes::ReckoningAttributeRead,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_hook::SkateboardHookDefinition,
    skateboard_spawn_math::SkateboardSpawnMathNumeric,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::fmt;

#[path = "physical_player_ground_skateboard.rs"]
pub(crate) mod live;

#[path = "physical_player_grounded_services.rs"]
mod services;
pub(crate) use services::GroundedWorldServiceError;

#[derive(Debug)]
pub(crate) enum PhysicalPlayerGroundedError<N, S, C> {
    Owner(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Update(SimulationSkeletonGroundedError<N, S, C>),
}
impl<N: fmt::Debug, S: fmt::Debug, C: fmt::Debug> fmt::Display
    for PhysicalPlayerGroundedError<N, S, C>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "grounded player owner: {error}"),
            Self::Pool(error) => write!(f, "grounded player pool: {error}"),
            Self::Update(error) => write!(f, "{error}"),
        }
    }
}
impl<N: fmt::Debug, S: fmt::Debug, C: fmt::Debug> std::error::Error
    for PhysicalPlayerGroundedError<N, S, C>
{
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Full ordinary General continuation, with the selected session's actual
    /// query owners. Ground remains an explicit borrow shared with the authored
    /// solver. Processed data is sampled before entry, never rebuilt at contact.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_skeleton_grounded_complete<A, N, S, M, C>(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        ground: &crate::static_ground_pool::StaticGroundPool,
        root_time_delta: f32,
        retained: &SkeletonGeneralInput,
        passed: &mut SkeletonGroundedPassedOutput,
        processed: &crate::skeleton_processed_contact::SkeletonProcessedContactReady,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        PhysicalPlayerGroundedError<
            N::Error,
            S::Error,
            GroundedWorldServiceError<
                <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
        >,
    >
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
        C: crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        use PhysicalPlayerGroundedError as E;
        self.physical_body_owner_for_contacts(skeleton)
            .map_err(E::Owner)?;
        self.physical_body_owner_for_contacts(board)
            .map_err(E::Owner)?;
        let mut services = services::GroundedWorldServices {
            cvars,
            queries: self.world_line_queries.as_mut(),
            dmos: self.dmos.as_ref(),
            ground,
        };
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_update_paired_skeleton_grounded_complete(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                root_time_delta,
                retained,
                passed,
                processed,
                reckoning,
                numeric,
                sin_cos,
                ortho,
                &mut services,
            )
            .map_err(E::Update)
    }

    #[cfg(test)]
    pub(crate) fn admit_world_pose_hooks_for_test(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        pose: &[crate::animation::AnimationMatrix],
    ) -> bool {
        self.skaters
            .as_mut()
            .expect("constructed pool")
            .admit_world_pose_hooks_for_test(&mut self.backend, &mut self.hooks, skeleton, pose)
    }

    pub(crate) fn try_skeleton_hook_part_transform(
        &self,
        hook: &crate::dynamic_assembly_session::hook_pool::SimulationHookAssemblyHandle,
        part: usize,
    ) -> Result<
        crate::skeleton_root_transforms::SkeletonRootMatrix,
        crate::dynamic_assembly_session::hook_pool::SimulationHookPoolError,
    > {
        self.hooks
            .try_skeleton_hook_part_transform(&self.backend, hook, part)
    }
    pub(crate) fn try_cached_skeleton_hook_part_transform(
        &self,
        hook: &crate::dynamic_assembly_session::hook_pool::SimulationHookAssemblyHandle,
        part: usize,
    ) -> Result<
        crate::skeleton_root_transforms::SkeletonRootMatrix,
        crate::dynamic_assembly_session::hook_pool::SimulationHookPoolError,
    > {
        self.hooks
            .skeleton_hook_session(hook)?
            .try_cached_part_transform_typed(part)
            .map_err(crate::dynamic_assembly_session::hook_pool::SimulationHookPoolError::Transform)
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same actual pair and distinct source collaborators"
    )]
    pub(crate) fn try_update_skeleton_grounded<A, N, S, M, C>(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        root_time_delta: f32,
        retained: &SkeletonGeneralInput,
        passed: &mut SkeletonGroundedPassedOutput,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
        drive_cvars: &mut C,
    ) -> Result<SkeletonGroundedProgress, PhysicalPlayerGroundedError<N::Error, S::Error, C::Error>>
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
        C: crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars,
    {
        use PhysicalPlayerGroundedError as E;
        self.physical_body_owner_for_contacts(skeleton)
            .map_err(E::Owner)?;
        self.physical_body_owner_for_contacts(board)
            .map_err(E::Owner)?;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_update_paired_skeleton_grounded(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                root_time_delta,
                retained,
                passed,
                reckoning,
                numeric,
                sin_cos,
                ortho,
                drive_cvars,
            )
            .map_err(E::Update)
    }
}

#[path = "../skeleton/physical_player_animated_skeleton.rs"]
pub(crate) mod animated;

#[path = "../air/physical_player_known_air.rs"]
pub(crate) mod known_air;

#[path = "../runtime/physical_player_fixed_bone_skeleton.rs"]
pub(crate) mod fixed_bone;

#[path = "../runtime/physical_player_hand_plant_skeleton.rs"]
pub(crate) mod hand_plant;

#[path = "../wipeout/physical_player_wipeout_update.rs"]
pub(crate) mod wipeout;

#[path = "../wipeout/physical_player_wipeout_control.rs"]
pub(crate) mod wipeout_control;
