//! Actual attached pair for Skeleton.UpdateFixedWorldPositionOfBone, preserving bodies.
use super::{
    Arc, AssemblyPartMaterial, MemberContent, SimulationHookAssemblyPool,
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
    SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::general_owner::{
        SkeletonGeneralCvars, SkeletonGroundedProgress, fixed_bone::FixedBoneSkeletonError,
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_state_changer::attributes::StateChangerAttributes,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_processed_contact::{SkeletonProcessedContact, grounded_runtime::GroundedProcessed},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Mutex;

impl SimulationSkaterAssemblyPool {
    /// `UpdateRootTransformsFixedPart` 0x82b42110 on the attached Skeleton.
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_update_paired_fixed_part_roots<N, S, C>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        reckoning: &SkeletonRootMatrix,
        position: SkeletonRootVector,
        part: usize,
        raw_bone: bool,
    ) -> Result<(), FixedBoneSkeletonError<N, S, C>> {
        self.try_with_jump_pair(backend, board, skeleton, |normal, owner| {
            if !Arc::ptr_eq(owner, expected) {
                return Err(FixedBoneSkeletonError::ForeignOwner);
            }
            normal.try_update_fixed_part_roots(reckoning, position, part, raw_bone)
        })
        .map_err(FixedBoneSkeletonError::Pair)?
    }

    /// `UpdateFixedWorldPositionOfBone` 0x82b5d5b0..da1c after the roots and
    /// the trajectory/root-pose identity stores, on the same attached pair.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_paired_skeleton_fixed_bone<N, S, H, C>(
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
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        FixedBoneSkeletonError<
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
        use FixedBoneSkeletonError as E;
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
        skeleton.owner.try_finish_fixed_bone_complete(
            backend,
            hooks,
            board.hook.as_ref(),
            &mut runtime.steering,
            air_attributes,
            root_dt,
            &input,
            numeric,
            rounding,
            cvars,
        )
    }
}
