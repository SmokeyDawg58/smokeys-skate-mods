//! Actual `UpdateAnimatedSkateboard82b5bc58` and shared `GeneralUpdate` continuation.
use super::{
    AssemblyPartMaterial, GroundedInputError, GroundedOutputWrite, LiveBoard, Reckoning,
    ReckoningAttributeRead, ReckoningNumeric, ReckoningSinCosNumeric, SimulationHookAssemblyPool,
    SkateboardAssemblySession, SkateboardSimulationBackend, SkateboardSpawnMathNumeric,
    SkeletonGeneralCvars, SkeletonGroundedProgress, SkeletonGroundedUpdateError,
    SkeletonNormalAssembly, SkeletonRootMatrix,
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::hook_pool::{SimulationHookAssemblyHandle, SimulationHookPoolError},
    physical_player_state_changer::attributes::StateChangerAttributes,
    skateboard_steering::{SkateboardSteeringState, animated::AnimatedBlendError},
    skeleton_processed_contact::grounded_runtime::GroundedProcessed,
    skeleton_root_transforms::SkeletonRootAssertion,
};

#[derive(Debug)]
pub(crate) enum AnimatedSkeletonError<N, S, C> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pool(crate::dynamic_assembly_session::skater_pool::SimulationSkaterPoolError),
    Pair(crate::dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError),
    General(SkeletonGroundedUpdateError<N, S, C>),
    Processed(GroundedInputError),
    Assertion(SkeletonRootAssertion),
    Blend(AnimatedBlendError<AttributeRuntimeError, N, S>),
    Hook(SimulationHookPoolError),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Display
    for AnimatedSkeletonError<N, S, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("animated Skeleton: Poisoned"),
            Self::ForeignOwner => f.write_str("animated Skeleton: ForeignOwner"),
            Self::Missing(error) => write!(f, "animated Skeleton Missing: {error:?}"),
            Self::Pool(error) => write!(f, "animated Skeleton Pool: {error:?}"),
            Self::Pair(error) => write!(f, "animated Skeleton Pair: {error:?}"),
            Self::General(error) => write!(f, "animated Skeleton General: {error:?}"),
            Self::Processed(error) => write!(f, "animated Skeleton Processed: {error:?}"),
            Self::Assertion(error) => write!(f, "animated Skeleton Assertion: {error:?}"),
            Self::Blend(error) => write!(f, "animated Skeleton Blend: {error:?}"),
            Self::Hook(error) => write!(f, "animated Skeleton Hook: {error:?}"),
        }
    }
}

impl SkeletonNormalAssembly {
    /// UpdateOffBoard 0x82b5c5ec reads mapped animation volume row zero.
    pub(crate) fn offboard_volume_root_row(&self) -> Option<SkeletonRootMatrix> {
        self.animation_targets
            .volume_row(0)
            .map(|row| SkeletonRootMatrix::known(*row))
    }
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_animated_complete<A, N, S, M, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        board: &mut SkateboardAssemblySession,
        board_hook: Option<&SimulationHookAssemblyHandle>,
        steering: &mut SkateboardSteeringState,
        air_attributes: &StateChangerAttributes,
        fast_blend: bool,
        root_dt: f32,
        processed: &GroundedProcessed<'_>,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        rounding: &mut S,
        ortho: &mut M,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        AnimatedSkeletonError<
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
        use AnimatedSkeletonError as E;
        if self.root_transforms.animation_to_board().has_nan_xyz() {
            return Err(E::Assertion(SkeletonRootAssertion::AnimationToBoard));
        }
        self.assert_animated_prediction()?;
        let root_input = self
            .current_animation
            .root_input(root_dt)
            .ok_or(E::Missing("current mapped animation"))?;
        {
            let mut live_board = LiveBoard {
                session: board,
                backend,
            };
            self.root_transforms
                .try_update_native(
                    &root_input,
                    reckoning,
                    &mut live_board,
                    numeric,
                    rounding,
                    ortho,
                )
                .map_err(|e| E::General(SkeletonGroundedUpdateError::Roots(e)))?;
        }
        self.assert_animated_prediction()?;
        if self.root_transforms.animation_to_world().has_nan_xyz() {
            return Err(E::Assertion(SkeletonRootAssertion::AnimationToWorld));
        }
        let mapped = self
            .animation_targets
            .volume_row(0)
            .ok_or(E::Missing("mapped volume zero"))?;
        let local = self
            .root_transforms
            .animation_to_board()
            .compose(&SkeletonRootMatrix::known(*mapped));
        let orientation =
            SkeletonRootMatrix::translation(self.root_transforms.predicted_board_position())
                .compose(&local);
        if orientation.has_nan_xyz() {
            return Err(E::Assertion(SkeletonRootAssertion::AnimationToBoard));
        }
        processed
            .write_orientation(orientation)
            .map_err(E::Processed)?;
        processed.set_animation_orientation_flag();
        self.general_update.set_animated_skate_root(orientation);
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
        let blending = steering.physics_error_blending();
        let applied = steering
            .try_blend_animated(
                applied,
                fast_blend,
                |fast, angle| air_attributes.try_board_animation_blend(fast, angle),
                numeric,
                rounding,
            )
            .map_err(E::Blend)?;
        if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            eprintln!(
                "known air: board target fast={fast_blend} anim_z={:?} applied_z={:?} blending={blending}->{} error={:?}",
                orientation.columns()[2].xyz,
                applied.columns()[2].xyz,
                steering.physics_error_blending(),
                steering.physics_error()
            );
        }
        hooks
            .try_set_board_hook_part_typed(
                backend,
                board_hook.ok_or(E::Missing("board hook Assembly"))?,
                &applied,
            )
            .map_err(E::Hook)?;
        self.general_update.set_animated_physics_skateboard(applied);
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
        self.general_update.finish_grounded_trajectory();
        Ok(progress)
    }

    fn assert_animated_prediction<N, S, C>(&self) -> Result<(), AnimatedSkeletonError<N, S, C>> {
        if self
            .root_transforms
            .predicted_board_position()
            .xyz
            .iter()
            .any(|v| v.is_nan())
        {
            Err(AnimatedSkeletonError::Assertion(
                SkeletonRootAssertion::PredictedBoardPosition,
            ))
        } else {
            Ok(())
        }
    }
}
