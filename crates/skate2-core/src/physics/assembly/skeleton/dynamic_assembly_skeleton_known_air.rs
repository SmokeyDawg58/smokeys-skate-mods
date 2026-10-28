//! Actual `UpdateKnownAir82b5d408` and shared `GeneralUpdate` continuation.
use super::live_input::GroundedInputRead;
use super::{
    AssemblyPartMaterial, GroundedInputError, GroundedOutputWrite, Reckoning,
    ReckoningAttributeRead, ReckoningNumeric, ReckoningSinCosNumeric, SimulationHookAssemblyPool,
    SkateboardAssemblySession, SkateboardSimulationBackend, SkateboardSpawnMathNumeric,
    SkeletonGeneralCvars, SkeletonGroundedProgress, SkeletonGroundedUpdateError,
    SkeletonNormalAssembly, SkeletonRootMatrix,
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::hook_pool::{SimulationHookAssemblyHandle, SimulationHookPoolError},
    dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    physical_player_state_changer::attributes::StateChangerAttributes,
    skateboard_steering::{SkateboardSteeringState, animated::AnimatedBlendError},
    skeleton_processed_contact::grounded_runtime::GroundedProcessed,
    skeleton_root_transforms::SkeletonRootAssertion,
};

#[derive(Debug)]
pub(crate) enum KnownAirSkeletonError<N, S, C> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pool(crate::dynamic_assembly_session::skater_pool::SimulationSkaterPoolError),
    Pair(crate::dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError),
    General(SkeletonGroundedUpdateError<N, S, C>),
    Processed(GroundedInputError),
    Assertion(SkeletonRootAssertion),
    Blend(AnimatedBlendError<AttributeRuntimeError, N, S>),
    Part(DynamicAssemblyPartTransformError),
    Hook(SimulationHookPoolError),
    Velocity(crate::skateboard_set_velocity::SkateboardSetVelocityError),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Display
    for KnownAirSkeletonError<N, S, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("KnownAir Skeleton: Poisoned"),
            Self::ForeignOwner => f.write_str("KnownAir Skeleton: ForeignOwner"),
            Self::Missing(error) => write!(f, "KnownAir Skeleton Missing: {error:?}"),
            Self::Pool(error) => write!(f, "KnownAir Skeleton Pool: {error:?}"),
            Self::Pair(error) => write!(f, "KnownAir Skeleton Pair: {error:?}"),
            Self::General(error) => write!(f, "KnownAir Skeleton General: {error:?}"),
            Self::Processed(error) => write!(f, "KnownAir Skeleton Processed: {error:?}"),
            Self::Assertion(error) => write!(f, "KnownAir Skeleton Assertion: {error:?}"),
            Self::Blend(error) => write!(f, "KnownAir Skeleton Blend: {error:?}"),
            Self::Part(error) => write!(f, "KnownAir Skeleton Part: {error:?}"),
            Self::Hook(error) => write!(f, "KnownAir Skeleton Hook: {error:?}"),
            Self::Velocity(error) => write!(f, "KnownAir Skeleton Velocity: {error:?}"),
        }
    }
}

impl SkeletonNormalAssembly {
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_known_air_complete<A, N, S, M, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        board: &mut SkateboardAssemblySession,
        board_hook: Option<&SimulationHookAssemblyHandle>,
        steering: &mut SkateboardSteeringState,
        air_attributes: &StateChangerAttributes,
        root_dt: f32,
        target_com: crate::skeleton_root_transforms::SkeletonRootVector,
        processed: &GroundedProcessed<'_>,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        rounding: &mut S,
        ortho: &mut M,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        KnownAirSkeletonError<
            N::Error,
            S::Error,
            <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
        >,
    >
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
        C: SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        use KnownAirSkeletonError as E;
        self.root_transforms.update_known_air(
            reckoning.transform(),
            self.current_animation.record().centre_of_mass(),
            target_com,
            ortho,
        );
        let mapped = self
            .animation_targets
            .volume_row(0)
            .ok_or(E::Missing("mapped volume zero"))?;
        // Native flag store precedes the Processed orientation matrix stores.
        processed.set_animation_orientation_flag();
        let orientation = self
            .root_transforms
            .animation_to_world()
            .compose(&SkeletonRootMatrix::known(*mapped));
        processed
            .write_orientation(orientation)
            .map_err(E::Processed)?;
        let applied = self.try_apply_offboard_board_animation(
            backend,
            hooks,
            board_hook,
            steering,
            air_attributes,
            processed,
            numeric,
            rounding,
        )?;
        // TargetPositionWithVelocity82b2a038: scalar reciprocal first, then
        // actual deck position read, subtraction, multiplication, all-Part store.
        let target = applied.columns()[3];
        crate::skateboard_set_velocity::try_target_position_with_velocity(
            board,
            backend,
            processed.time_delta().map_err(E::Processed)?,
            &target,
        )
        .map_err(|e| match e {
            crate::skateboard_set_velocity::TargetPositionError::Transform(e) => E::Part(e),
            crate::skateboard_set_velocity::TargetPositionError::Velocity(e) => E::Velocity(e),
        })?;
        self.general_update.set_animated_skate_root(applied);
        self.root_transforms.publish_board_prediction(target);
        let root_input = self
            .current_animation
            .root_input(root_dt)
            .ok_or(E::Missing("current mapped animation"))?;
        let prefix = self
            .try_update_general_prefix(
                backend,
                hooks,
                processed,
                &root_input.skate_animation,
                numeric,
                rounding,
                cvars,
            )
            .map_err(E::General)?;
        let progress = self
            .try_finish_general_complete(
                backend, hooks, processed, processed, prefix, numeric, rounding, cvars,
            )
            .map_err(E::General)?;
        Ok(progress)
    }
}

impl SkeletonNormalAssembly {
    /// ApplyBoardAnimation82b357f0 and caller PhysicsSkateboardMtx publication.
    /// No board velocity/SkateRoot/GeneralUpdate side effects belong to this child.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_apply_offboard_board_animation<
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        H,
        C,
    >(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        board_hook: Option<&SimulationHookAssemblyHandle>,
        steering: &mut SkateboardSteeringState,
        air_attributes: &StateChangerAttributes,
        processed: &GroundedProcessed<'_>,
        numeric: &mut N,
        rounding: &mut S,
    ) -> Result<SkeletonRootMatrix, KnownAirSkeletonError<N::Error, S::Error, C>> {
        use KnownAirSkeletonError as E;
        // ApplyBoardAnimation repeats both native assertions before copying.
        let applied = processed.animation_orientation().map_err(E::Processed)?;
        if applied.has_nan_xyz() {
            return Err(E::Assertion(SkeletonRootAssertion::AnimationToBoard));
        }
        if !processed
            .animation_orientation_flag()
            .map_err(E::Processed)?
        {
            return Err(E::Missing("ApplyBoardAnimation animation-valid flag"));
        }
        let applied = steering
            .try_blend_animated(
                applied,
                true,
                |fast, angle| air_attributes.try_board_animation_blend(fast, angle),
                numeric,
                rounding,
            )
            .map_err(E::Blend)?;
        hooks
            .try_set_board_hook_part_typed(
                backend,
                board_hook.ok_or(E::Missing("board hook Assembly"))?,
                &applied,
            )
            .map_err(E::Hook)?;
        self.general_update.set_animated_physics_skateboard(applied);
        Ok(applied)
    }
}
