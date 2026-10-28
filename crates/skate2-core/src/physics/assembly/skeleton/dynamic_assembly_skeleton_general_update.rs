//! Actual grounded root/suffix, velocity, hooks and conditional body/history reset.
//! Active IK remains an explicit continuation. Native state 0x2be skips IK and
//! proceeds through retained-row drives and the remaining force/grounded tail.
use crate::physics_state_ids::{PHYSICS_STATE_LANDINGONDECK, PHYSICS_STATE_TELEPORTING};
use super::{
    SkeletonNormalAssembly, drive_owner::SkeletonDriveOwnerStage, reset_owner::SkeletonResetError,
};
use crate::{
    dynamic_assembly_session::{
        hook_pool::{
            SimulationHookAssemblyHandle, SimulationHookAssemblyPool, SimulationHookPoolError,
        },
        skater_pool::{LiveBoard, SkeletonRootBoardAccessError},
    },
    reckoning::Reckoning,
    reckoning_attributes::ReckoningAttributeRead,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skateboard_spawn_math::SkateboardSpawnMathNumeric,
    skeleton_general_update::{
        SkeletonGeneralUpdateHooks, SkeletonGroundedSuffixError, SkeletonHookUpdateError,
    },
    skeleton_root_transforms::{SkeletonRootError, SkeletonRootMatrix},
};
use skate2_formats::collision::AssemblyPartMaterial;

#[path = "../../skeleton/movement/skeleton_grounded_input.rs"]
pub(crate) mod live_input;
use live_input::{GroundedInputError, GroundedInputRead, GroundedOutputWrite};

/// Lazy reads owned by the `GeneralUpdate` caller, in addition to drive cvars.
/// Source Initial/External callbacks are sampled only when their stage runs.
pub(crate) trait SkeletonGeneralCvars:
    crate::skeleton_drives::update::SkeletonDrivesUpdateCvars
{
    fn ik_draw_feet_on_deck(&mut self) -> Result<u8, Self::Error>;
    fn ik_require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn ik_maximum_external_normal_angle(&mut self) -> Result<f32, Self::Error>;
    fn ik_target_normal_blend_speed(&mut self) -> Result<f32, Self::Error>;
    fn ik_maximum_local_delta(&mut self) -> Result<f32, Self::Error>;
    fn ik_maximum_world_offset(&mut self) -> Result<f32, Self::Error>;
}
struct InitialCvars<'a, C>(&'a mut C);
impl<C: SkeletonGeneralCvars> crate::skeleton_ik_state::initial::SkeletonIkInitialCvars
    for InitialCvars<'_, C>
{
    type Error = C::Error;
    fn draw_feet_on_deck(&mut self) -> Result<u8, Self::Error> {
        self.0.ik_draw_feet_on_deck()
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.0.ik_require_debug_overlay()
    }
    fn maximum_external_normal_angle(&mut self) -> Result<f32, Self::Error> {
        self.0.ik_maximum_external_normal_angle()
    }
    fn target_normal_blend_speed(&mut self) -> Result<f32, Self::Error> {
        self.0.ik_target_normal_blend_speed()
    }
    fn maximum_local_delta(&mut self) -> Result<f32, Self::Error> {
        self.0.ik_maximum_local_delta()
    }
    fn maximum_world_offset(&mut self) -> Result<f32, Self::Error> {
        self.0.ik_maximum_world_offset()
    }
}

/// Mutable PASSED `ProcessedPhysIn` fields; distinct from the retained input.
pub(crate) struct SkeletonGroundedPassedOutput {
    pub(crate) orientation: SkeletonRootMatrix,
    pub(crate) flags_8c4: u32,
}
/// Current fields read through Skeleton's retained processed pointer.
pub(crate) struct SkeletonGeneralInput {
    pub(crate) time_delta: f32,
    /// Retained processed +964, distinct from the owning simulation gravity.
    pub(crate) gravity: f32,
    /// +8e8, not the separate physics state at+8e4.
    pub(crate) current_physics_state: i32,
    pub(crate) flags_8c8: u32,
    pub(crate) flags_8d0: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonGeneralContinuation {
    UpdateIkContact,
    Complete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkeletonGroundedProgress {
    pub(crate) reset_performed: bool,
    pub(crate) next: SkeletonGeneralContinuation,
}
#[derive(Debug)]
pub(crate) enum SkeletonLiveHookError {
    MissingDrives,
    IncompleteDrives,
    Hook(SimulationHookPoolError),
}
impl std::fmt::Display for SkeletonLiveHookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDrives => f.write_str("Skeleton drives owner is absent"),
            Self::IncompleteDrives => f.write_str("Skeleton hook admission is incomplete"),
            Self::Hook(error) => write!(f, "Skeleton hook: {error}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum SkeletonGroundedUpdateError<N, S, C> {
    AnimationNotCurrent,
    Input(GroundedInputError),
    Roots(SkeletonRootError<SkeletonRootBoardAccessError, N, S>),
    Suffix(SkeletonGroundedSuffixError<SkeletonRootBoardAccessError>),
    Velocity(N),
    Hooks(SkeletonHookUpdateError<SkeletonLiveHookError, N>),
    Reset(SkeletonResetError),
    Ik(crate::skeleton_ik::SkeletonIkStagesError<N, S, C>),
    IkSuffix(
        crate::skeleton_ik::SkeletonIkSuffixError<
            crate::skeleton_animation_targets::SkeletonIkPoseError,
            crate::attribute_runtime::AttributeRuntimeError,
            C,
            N,
            S,
        >,
    ),
    Drives(super::drive_owner::update_owner::SkeletonDriveFeedbackError<C>),
    Displacement {
        part: usize,
        source:
            crate::dynamic_assembly_session::part_transform::DynamicAssemblyPartDisplacementError<
                N,
            >,
    },
}
impl<N: std::fmt::Debug, S: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Display
    for SkeletonGroundedUpdateError<N, S, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(error) => write!(f, "grounded input: {error}"),
            Self::AnimationNotCurrent => f.write_str("grounded update requires current animation"),
            Self::Roots(error) => write!(f, "grounded roots: {error}"),
            Self::Suffix(error) => write!(f, "grounded output: {error}"),
            Self::Velocity(error) => write!(f, "grounded velocity: {error:?}"),
            Self::Hooks(error) => write!(f, "grounded hooks: {error}"),
            Self::Reset(error) => write!(f, "grounded reset: {error}"),
            Self::Ik(error) => write!(f, "grounded IK: {error}"),
            Self::IkSuffix(error) => write!(f, "grounded IK suffix: {error:?}"),
            Self::Drives(error) => write!(f, "grounded drives: {error}"),
            Self::Displacement { part, source } => {
                write!(f, "grounded displacement Part {part}: {source}")
            }
        }
    }
}
impl<N: std::fmt::Debug, S: std::fmt::Debug, C: std::fmt::Debug> std::error::Error
    for SkeletonGroundedUpdateError<N, S, C>
{
}

struct LiveHooks<'a, H> {
    pool: &'a mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
    backend: &'a mut SkateboardSimulationBackend,
    handle: Option<SimulationHookAssemblyHandle>,
    stage: Option<SkeletonDriveOwnerStage>,
}
impl<H> LiveHooks<'_, H> {
    fn current_handle(&self) -> Result<SimulationHookAssemblyHandle, SkeletonLiveHookError> {
        match self.stage {
            None => return Err(SkeletonLiveHookError::MissingDrives),
            Some(SkeletonDriveOwnerStage::Complete) => {}
            Some(_) => return Err(SkeletonLiveHookError::IncompleteDrives),
        }
        self.handle
            .clone()
            .ok_or(SkeletonLiveHookError::IncompleteDrives)
    }
}
impl<H> SkeletonGeneralUpdateHooks for LiveHooks<'_, H> {
    type Error = SkeletonLiveHookError;
    fn transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error> {
        let handle = self.current_handle()?;
        self.pool
            .try_skeleton_hook_part_transform(self.backend, &handle, part)
            .map_err(Self::Error::Hook)
    }
    fn set_transform(
        &mut self,
        part: usize,
        transform: &SkeletonRootMatrix,
    ) -> Result<(), Self::Error> {
        let handle = self.current_handle()?;
        self.pool
            .try_set_skeleton_hook_part_typed(self.backend, &handle, part, transform)
            .map_err(Self::Error::Hook)
    }
}

impl SkeletonNormalAssembly {
    /// UpdateOffBoard caller 0x82b5c8d4..ec resets retained NextTrajectory
    /// after GeneralUpdate, in the same [0,3,1,2] column order as Grounded.
    pub(crate) fn finish_offboard_next_trajectory(&mut self) {
        self.general_update.finish_grounded_trajectory();
    }
    /// UpdateOffBoard 0x82b5c874 enters GeneralUpdate with its roots and
    /// orientation already published. Reuse only GeneralUpdate's prefix and
    /// completion; the grounded/animated callers would redo those writes.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_offboard_general_complete<N, S, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        processed: &crate::skeleton_processed_contact::grounded_runtime::GroundedProcessed<'_>,
        numeric: &mut N,
        sin_cos: &mut S,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        SkeletonGroundedUpdateError<
            N::Error,
            S::Error,
            <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
        >,
    >
    where
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        C: SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        let skate_animation = *self.current_animation.skateboard();
        let progress = self.try_update_general_prefix(
            backend,
            hooks,
            processed,
            &skate_animation,
            numeric,
            sin_cos,
            cvars,
        )?;
        self.try_finish_general_complete(
            backend, hooks, processed, processed, progress, numeric, sin_cos, cvars,
        )
    }
    /// Controlled world-space collision-fixture admission, not a gameplay tick.
    /// Reuses actual retained `UpdateHooks` and its required `ResetSkeleton` branch.
    #[cfg(test)]
    pub(crate) fn admit_world_pose_hooks_for_test<H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        pose: &[crate::animation::AnimationMatrix],
    ) -> bool {
        let identity = crate::skeleton_root_transforms::SkeletonRootTransforms::new();
        assert_eq!(
            self.root_transforms
                .animation_to_world()
                .xyz_scratch()
                .map(f32::to_bits),
            identity
                .animation_to_world()
                .xyz_scratch()
                .map(f32::to_bits)
        );
        // Init's mapped buffers contain identity, not the retained supplied pose.
        // Reuse the existing isolated-fixture ProcessData mapping producer first.
        self.map_original_pose_for_animation_query(pose).unwrap();
        let hips = *self.animation_targets.volume_row(23).unwrap();
        let skate = *self.animation_targets.volume_row(0).unwrap();
        self.general_update
            .set_animated_skate_root(SkeletonRootMatrix::known(skate));
        let accepted = {
            let mut actual_hooks = LiveHooks {
                pool: hooks,
                backend,
                handle: self.drives.as_ref().and_then(|drive| drive.hook().cloned()),
                stage: self
                    .drives
                    .as_ref()
                    .map(super::drive_owner::SkeletonDriveRuntime::stage),
            };
            self.general_update
                .try_update_hooks(
                    &self.root_transforms,
                    &hips,
                    &skate,
                    &mut self.state.proxy_volume_target,
                    &mut actual_hooks,
                    &mut crate::reckoning_numeric::HostReckoningNumeric,
                )
                .unwrap()
        };
        eprintln!(
            "collision fixture mapped volume targets: hips={:?} root={:?} accepted={accepted}",
            &hips[12..15],
            &skate[12..15]
        );
        if !accepted {
            self.try_reset_skeleton(backend).unwrap();
        }
        !accepted
    }
    /// Complete `GeneralUpdate` on the same retained owners. The bounded prefix
    /// remains available for diagnostics; this entry consumes its continuation
    /// immediately, without repeating roots, hooks, reset or IK transforms.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_grounded_complete<A, N, S, M, H, C, P>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        board: &SkateboardAssemblySession,
        root_time_delta: f32,
        retained: &SkeletonGeneralInput,
        passed: &mut SkeletonGroundedPassedOutput,
        processed_contact: &P,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        SkeletonGroundedUpdateError<
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
        P: crate::skeleton_ik_state::contact::SkeletonIkContactRead,
        C: SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        let passed = std::cell::RefCell::new(passed);
        self.try_update_grounded_complete_read(
            backend,
            hooks,
            board,
            root_time_delta,
            retained,
            &passed,
            processed_contact,
            reckoning,
            numeric,
            sin_cos,
            ortho,
            cvars,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same native bodies, processed owners and distinct numerical collaborators"
    )]
    pub(crate) fn try_update_grounded_complete_read<A, N, S, M, H, C, P>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        board: &SkateboardAssemblySession,
        root_time_delta: f32,
        retained: &impl GroundedInputRead,
        passed: &impl GroundedOutputWrite,
        processed_contact: &P,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        SkeletonGroundedUpdateError<
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
        P: crate::skeleton_ik_state::contact::SkeletonIkContactRead,
        C: SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        let progress = self.try_update_grounded_read(
            backend,
            hooks,
            board,
            root_time_delta,
            retained,
            passed,
            reckoning,
            numeric,
            sin_cos,
            ortho,
            cvars,
        )?;
        if progress.next == SkeletonGeneralContinuation::Complete {
            return Ok(progress);
        }
        let progress = self.try_finish_general_complete(
            backend,
            hooks,
            retained,
            processed_contact,
            progress,
            numeric,
            sin_cos,
            cvars,
        )?;
        self.general_update.finish_grounded_trajectory();
        Ok(progress)
    }

    /// Shared `GeneralUpdate` tail; caller-specific `NextTrajectory` reset is separate.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    fn try_finish_general_complete<N, S, H, C, P>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        retained: &impl GroundedInputRead,
        processed_contact: &P,
        progress: SkeletonGroundedProgress,
        numeric: &mut N,
        sin_cos: &mut S,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        SkeletonGroundedUpdateError<
            N::Error,
            S::Error,
            <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
        >,
    >
    where
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        P: crate::skeleton_ik_state::contact::SkeletonIkContactRead,
        C: SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        if progress.next == SkeletonGeneralContinuation::Complete {
            return Ok(progress);
        }
        // The actual global animation provider is established by current pose
        // processing. No replacement Instance or prephysics tuning is bound here.
        let attributes = self
            .current_animation_attributes
            .as_mut()
            .ok_or(SkeletonGroundedUpdateError::AnimationNotCurrent)?;
        self.current_animation
            .try_update_ik_suffix(
                processed_contact,
                &mut self.animation_targets,
                attributes,
                cvars,
                numeric,
                sin_cos,
            )
            .map_err(SkeletonGroundedUpdateError::IkSuffix)?;
        self.try_finish_grounded_forces(backend, hooks, retained, numeric, cvars)?;
        Ok(SkeletonGroundedProgress {
            reset_performed: progress.reset_performed,
            next: SkeletonGeneralContinuation::Complete,
        })
    }

    /// The pool checks actual owner/attachment authority and invalidates its
    /// collision publication before entry. All later source prefixes persist.
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "actual bodies/hooks, distinct processed pointers and numerical services"
    )]
    pub(crate) fn try_update_grounded<A, N, S, M, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        board: &SkateboardAssemblySession,
        root_time_delta: f32,
        retained: &SkeletonGeneralInput,
        passed: &mut SkeletonGroundedPassedOutput,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
        drive_cvars: &mut C,
    ) -> Result<SkeletonGroundedProgress, SkeletonGroundedUpdateError<N::Error, S::Error, C::Error>>
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
        C: SkeletonGeneralCvars,
    {
        let passed = std::cell::RefCell::new(passed);
        self.try_update_grounded_read(
            backend,
            hooks,
            board,
            root_time_delta,
            retained,
            &passed,
            reckoning,
            numeric,
            sin_cos,
            ortho,
            drive_cvars,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same native bodies, processed owners and distinct numerical collaborators"
    )]
    pub(crate) fn try_update_grounded_read<A, N, S, M, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        board: &SkateboardAssemblySession,
        root_time_delta: f32,
        retained: &impl GroundedInputRead,
        passed: &impl GroundedOutputWrite,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
        drive_cvars: &mut C,
    ) -> Result<SkeletonGroundedProgress, SkeletonGroundedUpdateError<N::Error, S::Error, C::Error>>
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
        C: SkeletonGeneralCvars,
    {
        use SkeletonGroundedUpdateError as E;
        let root_input = self
            .current_animation
            .root_input(root_time_delta)
            .ok_or(E::AnimationNotCurrent)?;
        {
            let mut board = LiveBoard {
                session: board,
                backend,
            };
            self.root_transforms
                .try_update_grounded(&root_input, reckoning, &mut board, numeric, sin_cos, ortho)
                .map_err(E::Roots)?;
            self.general_update
                .try_grounded_suffix_write(
                    &self.root_transforms,
                    self.animation_targets
                        .volume_row(0)
                        .expect("completed current animation mapping"),
                    passed,
                    &mut board,
                )
                .map_err(E::Suffix)?;
        }
        let progress = self.try_update_general_prefix(
            backend,
            hooks,
            retained,
            &root_input.skate_animation,
            numeric,
            sin_cos,
            drive_cvars,
        )?;
        if progress.next == SkeletonGeneralContinuation::Complete {
            self.general_update.finish_grounded_trajectory();
        }
        Ok(progress)
    }

    /// `GeneralUpdate82b5baa0` after the caller has published roots/board state.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    fn try_update_general_prefix<N, S, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        retained: &impl GroundedInputRead,
        skate_animation: &crate::animation::AnimationMatrix,
        numeric: &mut N,
        sin_cos: &mut S,
        drive_cvars: &mut C,
    ) -> Result<SkeletonGroundedProgress, SkeletonGroundedUpdateError<N::Error, S::Error, C::Error>>
    where
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        C: SkeletonGeneralCvars,
    {
        use SkeletonGroundedUpdateError as E;
        self.general_update
            .try_update_velocity(
                &self.root_transforms,
                retained.time_delta().map_err(E::Input)?,
                numeric,
            )
            .map_err(E::Velocity)?;
        let accepted = {
            // Missing/incomplete hooks fail at the actual first hook read,
            // after the source animation-to-physics cache has been committed.
            let mut actual_hooks = LiveHooks {
                pool: hooks,
                backend,
                handle: self
                    .drives
                    .as_ref()
                    .and_then(|drives| drives.hook().cloned()),
                stage: self
                    .drives
                    .as_ref()
                    .map(super::drive_owner::SkeletonDriveRuntime::stage),
            };
            self.general_update
                .try_update_hooks(
                    &self.root_transforms,
                    self.current_animation.hips(),
                    skate_animation,
                    &mut self.state.proxy_volume_target,
                    &mut actual_hooks,
                    numeric,
                )
                .map_err(E::Hooks)?
        };
        if !accepted {
            self.try_reset_skeleton(backend).map_err(E::Reset)?;
            self.general_update
                .mark_want_respawn_after_reset(retained.current_state().map_err(E::Input)?);
        }
        if retained.current_state().map_err(E::Input)? != PHYSICS_STATE_TELEPORTING {
            self.try_update_ik_before_contact(retained, numeric, sin_cos, drive_cvars)
                .map_err(E::Ik)?;
            return Ok(SkeletonGroundedProgress {
                reset_performed: !accepted,
                next: SkeletonGeneralContinuation::UpdateIkContact,
            });
        }
        self.try_finish_grounded_forces(backend, hooks, retained, numeric, drive_cvars)?;
        Ok(SkeletonGroundedProgress {
            reset_performed: !accepted,
            next: SkeletonGeneralContinuation::Complete,
        })
    }

    fn try_finish_grounded_forces<N: ReckoningNumeric, H, C: SkeletonGeneralCvars, S>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        retained: &impl GroundedInputRead,
        numeric: &mut N,
        drive_cvars: &mut C,
    ) -> Result<(), SkeletonGroundedUpdateError<N::Error, S, C::Error>> {
        use SkeletonGroundedUpdateError as E;
        self.try_update_drives_from_retained_volumes(backend, hooks, drive_cvars)
            .map_err(E::Drives)?;
        let displacement = crate::skeleton_general_force::displacement_from_gravity(
            retained.gravity().map_err(E::Input)?,
        );
        for part in 0..24 {
            self.session
                .try_apply_part_displacement(backend, part, displacement, numeric)
                .map_err(|source| E::Displacement { part, source })?;
        }
        Ok(())
    }

    /// Input update82b414d8 on actual current roots, adjusted hips, previous
    /// animation record and collision data. Native row and owner references
    /// remain these same enclosing fields for subsequent stages.
    #[expect(
        clippy::type_complexity,
        reason = "distinct numerical and cvar failures"
    )]
    fn try_update_ik_before_contact<
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
        C: SkeletonGeneralCvars,
    >(
        &mut self,
        input: &impl GroundedInputRead,
        numeric: &mut N,
        rounding: &mut R,
        cvars: &mut C,
    ) -> Result<(), crate::skeleton_ik::SkeletonIkStagesError<N::Error, R::Error, C::Error>> {
        let data = self
            .try_build_ik_data(input)
            .map_err(crate::skeleton_ik::SkeletonIkStagesError::Prefix)?;
        self.current_animation
            .try_update_ik_prefix_read(
                data,
                || input.flags_8c8(0x600),
                || input.flags_8d0(0x1000_0000).map(|v| v != 0),
            )
            .map_err(crate::skeleton_ik::SkeletonIkStagesError::Prefix)?;
        self.current_animation
            .try_update_ik_transforms(&mut InitialCvars(cvars), numeric, rounding)
    }
}

#[path = "dynamic_assembly_skeleton_animated.rs"]
pub(crate) mod animated;

#[path = "dynamic_assembly_skeleton_known_air.rs"]
pub(crate) mod known_air;

#[path = "dynamic_assembly_skeleton_fixed_bone.rs"]
pub(crate) mod fixed_bone;

#[path = "dynamic_assembly_skeleton_grab_targets.rs"]
pub(crate) mod grab_targets;
#[path = "dynamic_assembly_skeleton_hand_plant.rs"]
pub(crate) mod hand_plant;

impl SkeletonNormalAssembly {
    fn try_build_ik_data(
        &self,
        input: &impl GroundedInputRead,
    ) -> Result<crate::skeleton_ik::SkeletonIkDataIn, crate::skeleton_ik::SkeletonIkUpdateError>
    {
        let roots = &self.root_transforms;
        let (skateboard, physics_skateboard, inverse_skateboard) = if input
            .current_state()
            .map_err(crate::skeleton_ik::SkeletonIkUpdateError::Input)?
            == PHYSICS_STATE_LANDINGONDECK
        {
            let board = roots
                .animation_to_world()
                .compose(&SkeletonRootMatrix::known(
                    self.current_animation.record().poses()[0],
                ));
            let inverse = SkeletonRootMatrix::known(
                crate::skateboard_spawn_math::skateboard_spawn_rigid_inverse(&board.xyz_scratch()),
            );
            (board, board, inverse)
        } else {
            (
                *roots.board(),
                *self.general_update.physics_skateboard(),
                *roots.board_inverse(),
            )
        };
        // This source read41798 is AFTER all local branch math. The actual
        // collision object always exists here; unknown packed W stays unknown.
        let normal = self.state.collision.most_vertical_toe_contact_normal;
        Ok(crate::skeleton_ik::SkeletonIkDataIn {
            animation_to_world: *roots.animation_to_world(),
            world_to_animation: *roots.world_to_animation(),
            physics_skateboard,
            inverse_skateboard,
            hips: SkeletonRootMatrix::known(*self.current_animation.hips()),
            skateboard,
            toe_contact_normal: crate::skeleton_root_transforms::SkeletonRootVector {
                xyz: *normal.xyz(),
                w: normal.known_lanes().map(|lanes| lanes[3]),
            },
            hips_offset: self.hips_offset,
        })
    }
}
#[path = "dynamic_assembly_skeleton_wipeout_update.rs"]
pub(crate) mod wipeout;
