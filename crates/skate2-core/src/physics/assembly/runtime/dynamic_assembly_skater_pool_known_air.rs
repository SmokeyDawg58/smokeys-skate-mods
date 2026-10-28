//! Actual attached pair for Skeleton.UpdateKnownAir, preserving bodies.
use super::{
    Arc, AssemblyPartMaterial, MemberContent, SimulationHookAssemblyPool,
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
    SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::general_owner::{
        SkeletonGeneralCvars, SkeletonGroundedProgress, known_air::KnownAirSkeletonError,
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_state_changer::attributes::StateChangerAttributes,
    reckoning::Reckoning,
    reckoning_attributes::ReckoningAttributeRead,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_spawn_math::SkateboardSpawnMathNumeric,
    skeleton_processed_contact::{SkeletonProcessedContact, grounded_runtime::GroundedProcessed},
};
use std::sync::Mutex;

impl SimulationSkaterAssemblyPool {
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_paired_skeleton_known_air<A, N, S, M, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        reckoning: &Reckoning<A>,
        air_attributes: &StateChangerAttributes,
        target_com: crate::skeleton_root_transforms::SkeletonRootVector,
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
        let actual = self
            .try_with_jump_pair(backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        if !Arc::ptr_eq(&actual, expected) {
            return Err(E::ForeignOwner);
        }
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(|e| E::Pool(SimulationSkaterPoolError::Hook(e)))?;
        let bi = self.member_index(board).map_err(E::Pool)?;
        let si = self.member_index(skeleton).map_err(E::Pool)?;
        self.invalidate_refresh();
        let (bm, sm) = if bi < si {
            let (before, after) = self.members.split_at_mut(si);
            (&mut before[bi], &mut after[0])
        } else {
            let (before, after) = self.members.split_at_mut(bi);
            (&mut after[0], &mut before[si])
        };
        let MemberContent::Board {
            candidate: board, ..
        } = &mut bm.content
        else {
            unreachable!("checked pair");
        };
        let MemberContent::Skeleton {
            candidate: skeleton,
            ..
        } = &mut sm.content
        else {
            unreachable!("checked pair");
        };
        let constructor = &skeleton
            .construction
            .as_ref()
            .ok_or(E::Missing("Skeleton constructor"))?
            .constructor;
        let bindings = constructor
            .bindings
            .as_ref()
            .ok_or(E::Missing("Skeleton bindings"))?;
        if !Arc::ptr_eq(&bindings.processed, processed)
            || !bindings
                .owner
                .upgrade()
                .is_some_and(|v| Arc::ptr_eq(&v, expected))
        {
            return Err(E::ForeignOwner);
        }
        let root_dt = constructor
            .outer
            .as_ref()
            .ok_or(E::Missing("Skeleton remainder"))?
            .time_delta;
        let runtime = board
            .force_runtime
            .as_mut()
            .ok_or(E::Missing("outer board"))?;
        let mut guard = processed.lock().map_err(|_| E::Poisoned)?;
        let input = GroundedProcessed::new(&mut guard);
        skeleton.owner.try_update_known_air_complete(
            backend,
            hooks,
            &mut board.session,
            board.hook.as_ref(),
            &mut runtime.steering,
            air_attributes,
            root_dt,
            target_com,
            &input,
            reckoning,
            numeric,
            rounding,
            ortho,
            cvars,
        )
    }
}

impl SimulationSkaterAssemblyPool {
    /// OffBoard/Air8d0&40000 selects fast ApplyBoardAnimation before GeneralUpdate.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_apply_pair_offboard_board_animation<
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        H,
    >(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        air_attributes: &StateChangerAttributes,
        numeric: &mut N,
        rounding: &mut S,
    ) -> Result<(), KnownAirSkeletonError<N::Error, S::Error, ()>> {
        use KnownAirSkeletonError as E;
        let actual = self
            .try_with_jump_pair(backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        if !Arc::ptr_eq(&actual, expected) {
            return Err(E::ForeignOwner);
        }
        hooks
            .check_authorities(self.manager, self.body_pool)
            .map_err(|e| E::Pool(SimulationSkaterPoolError::Hook(e)))?;
        let bi = self.member_index(board).map_err(E::Pool)?;
        let si = self.member_index(skeleton).map_err(E::Pool)?;
        self.invalidate_refresh();
        let (bm, sm) = if bi < si {
            let (before, after) = self.members.split_at_mut(si);
            (&mut before[bi], &mut after[0])
        } else {
            let (before, after) = self.members.split_at_mut(bi);
            (&mut after[0], &mut before[si])
        };
        let MemberContent::Board {
            candidate: board, ..
        } = &mut bm.content
        else {
            unreachable!("checked pair");
        };
        let MemberContent::Skeleton {
            candidate: skeleton,
            ..
        } = &mut sm.content
        else {
            unreachable!("checked pair");
        };
        let constructor = &skeleton
            .construction
            .as_ref()
            .ok_or(E::Missing("Skeleton constructor"))?
            .constructor;
        let bindings = constructor
            .bindings
            .as_ref()
            .ok_or(E::Missing("Skeleton bindings"))?;
        if !Arc::ptr_eq(&bindings.processed, processed)
            || !bindings
                .owner
                .upgrade()
                .is_some_and(|v| Arc::ptr_eq(&v, expected))
        {
            return Err(E::ForeignOwner);
        }
        let runtime = board
            .force_runtime
            .as_mut()
            .ok_or(E::Missing("outer board"))?;
        let mut guard = processed.lock().map_err(|_| E::Poisoned)?;
        let input = GroundedProcessed::new(&mut guard);
        skeleton.owner.try_apply_offboard_board_animation(
            backend,
            hooks,
            board.hook.as_ref(),
            &mut runtime.steering,
            air_attributes,
            &input,
            numeric,
            rounding,
        )?;
        Ok(())
    }
}
