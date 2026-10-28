//! Grounded update using the same registered player pair and admitted hooks.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError,
};
use crate::{
    dynamic_assembly_session::{
        hook_pool::SimulationHookAssemblyPool,
        skeleton_normal::general_owner::{
            SkeletonGeneralInput, SkeletonGroundedPassedOutput, SkeletonGroundedProgress,
            SkeletonGroundedUpdateError,
        },
    },
    reckoning::Reckoning,
    reckoning_attributes::ReckoningAttributeRead,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_spawn_math::SkateboardSpawnMathNumeric,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum SimulationSkeletonOffboardGeneralError<N, S, C> {
    Pair(crate::dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    MissingConstruction,
    MissingBindings,
    ForeignOwner,
    Poisoned,
    Update(SimulationSkeletonGroundedError<N, S, C>),
}

impl SimulationSkaterAssemblyPool {
    /// The OffBoard caller already published root transforms and orientation.
    /// Enter only the retained GeneralUpdate continuation on the same pair.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_paired_skeleton_offboard_general<N, S, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        expected: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        processed: &Arc<
            std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>,
        >,
        numeric: &mut N,
        sin_cos: &mut S,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        SimulationSkeletonOffboardGeneralError<
            N::Error,
            S::Error,
            <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
        >,
    >
    where
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        C: crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        use SimulationSkeletonOffboardGeneralError as E;
        let actual = self
            .try_with_jump_pair(backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        if !Arc::ptr_eq(&actual, expected) {
            return Err(E::ForeignOwner);
        }
        let skeleton_index = self.member_index(skeleton).map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &self.members[skeleton_index].content
        else {
            unreachable!("checked Skeleton member")
        };
        let construction = candidate.construction().ok_or(E::MissingConstruction)?;
        let bindings = construction
            .constructor
            .bindings
            .as_ref()
            .ok_or(E::MissingBindings)?;
        if !Arc::ptr_eq(&bindings.processed, processed)
            || !bindings
                .owner
                .upgrade()
                .is_some_and(|owner| Arc::ptr_eq(&owner, expected))
        {
            return Err(E::ForeignOwner);
        }
        let mut guard = processed.lock().map_err(|_| E::Poisoned)?;
        let input =
            crate::skeleton_processed_contact::grounded_runtime::GroundedProcessed::new(&mut guard);
        self.try_with_grounded_pair(
            backend,
            hooks,
            skeleton,
            board,
            |normal, backend, hooks, _, _| {
                normal.try_update_offboard_general_complete(
                    backend, hooks, &input, numeric, sin_cos, cvars,
                )
            },
        )
        .map_err(E::Update)
    }
}

#[derive(Debug)]
pub(crate) enum SimulationSkeletonGroundedError<N, S, C> {
    Pool(SimulationSkaterPoolError),
    DifferentPhysicalOwner,
    Detached,
    Update(SkeletonGroundedUpdateError<N, S, C>),
}
impl<N: fmt::Debug, S: fmt::Debug, C: fmt::Debug> fmt::Display
    for SimulationSkeletonGroundedError<N, S, C>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "grounded pair: {error}"),
            Self::DifferentPhysicalOwner => {
                f.write_str("grounded pair belongs to different players")
            }
            Self::Detached => f.write_str("grounded update requires attached bodies"),
            Self::Update(error) => write!(f, "{error}"),
        }
    }
}
impl<N: fmt::Debug, S: fmt::Debug, C: fmt::Debug> std::error::Error
    for SimulationSkeletonGroundedError<N, S, C>
{
}

#[path = "../../player/ground/physical_player_ground_skateboard_pool.rs"]
mod live;

impl SimulationSkaterAssemblyPool {
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "actual pair/hooks and separate source input/output/numerical owners"
    )]
    pub(crate) fn try_update_paired_skeleton_grounded<A, N, S, M, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
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
    ) -> Result<
        SkeletonGroundedProgress,
        SimulationSkeletonGroundedError<N::Error, S::Error, C::Error>,
    >
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
        C: crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars,
    {
        self.try_with_grounded_pair(
            backend,
            hooks,
            skeleton,
            board,
            |owner, backend, hooks, board, _constructor| {
                owner.try_update_grounded(
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
                    drive_cvars,
                )
            },
        )
    }
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_paired_skeleton_grounded_complete<A, N, S, M, H, C, P>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        root_time_delta: f32,
        retained: &SkeletonGeneralInput,
        passed: &mut SkeletonGroundedPassedOutput,
        processed: &P,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        SimulationSkeletonGroundedError<
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
        C: crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        self.try_with_grounded_pair(
            backend,
            hooks,
            skeleton,
            board,
            |owner, backend, hooks, board, _constructor| {
                owner.try_update_grounded_complete(
                    backend,
                    hooks,
                    board,
                    root_time_delta,
                    retained,
                    passed,
                    processed,
                    reckoning,
                    numeric,
                    sin_cos,
                    ortho,
                    cvars,
                )
            },
        )
    }

    /// Shared paired-owner/attachment checks and invalidation for both bounded
    /// prefix diagnostics and the complete active-IK continuation.
    fn try_with_grounded_pair<T, N, S, C, H>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        update: impl FnOnce(
            &mut crate::dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly,
            &mut SkateboardSimulationBackend,
            &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
            &crate::skateboard_assembly::SkateboardAssemblySession,
            Option<&crate::skeleton_assembly_construction::constructor::SkeletonConstructorRuntime>,
        ) -> Result<T, SkeletonGroundedUpdateError<N, S, C>>,
    ) -> Result<T, SimulationSkeletonGroundedError<N, S, C>> {
        use crate::dynamic_assembly_session::skater_pool::physical::SimulationPhysicalBodyKind;
        use SimulationSkeletonGroundedError as E;
        let skeleton_index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let board_index = self.member_index(board).map_err(E::Pool)?;
        let (kind, board_owner) = self
            .try_physical_member_owner(backend, board)
            .map_err(E::Pool)?;
        if kind != SimulationPhysicalBodyKind::Board {
            return Err(E::Pool(SimulationSkaterPoolError::WrongMemberKind));
        }
        let (_, skeleton_owner) = self
            .try_physical_member_owner(backend, skeleton)
            .map_err(E::Pool)?;
        if !Arc::ptr_eq(board_owner, skeleton_owner) {
            return Err(E::DifferentPhysicalOwner);
        }
        if board_owner.manager() != self.manager {
            return Err(E::Pool(SimulationSkaterPoolError::ForeignManager));
        }
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(|error| E::Pool(SimulationSkaterPoolError::Hook(error)))?;
        // Invalidate before any possible hook/body write, including failure.
        self.invalidate_refresh();
        let (board_member, skeleton_member) = if board_index < skeleton_index {
            let (before, after) = self.members.split_at_mut(skeleton_index);
            (&before[board_index], &mut after[0])
        } else {
            let (before, after) = self.members.split_at_mut(board_index);
            (&after[0], &mut before[skeleton_index])
        };
        let MemberContent::Board {
            candidate: board, ..
        } = &board_member.content
        else {
            unreachable!("kind checked")
        };
        let MemberContent::Skeleton {
            candidate: skeleton,
            ..
        } = &mut skeleton_member.content
        else {
            unreachable!("kind checked")
        };
        if !board.session().is_attached() || !skeleton.owner().session().is_attached() {
            return Err(E::Detached);
        }
        update(
            &mut skeleton.owner,
            backend,
            hooks,
            board.session(),
            skeleton.construction.as_deref().map(|v| &v.constructor),
        )
        .map_err(E::Update)
    }
}
