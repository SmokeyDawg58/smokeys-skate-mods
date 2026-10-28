//! Existing paired `GroundSkateboard` executor with actual constructor/processed owners.
use super::{
    Arc, AssemblyPartMaterial, Reckoning, ReckoningAttributeRead, ReckoningNumeric,
    ReckoningSinCosNumeric, SimulationHookAssemblyPool, SimulationSkaterAssemblyHandle,
    SimulationSkaterAssemblyPool, SimulationSkeletonGroundedError, SkateboardSimulationBackend,
    SkateboardSpawnMathNumeric, SkeletonGroundedProgress, SkeletonGroundedUpdateError,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    skeleton_processed_contact::{SkeletonProcessedContact, grounded_runtime::GroundedProcessed},
};
use std::sync::Mutex;
impl SimulationSkaterAssemblyPool {
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same pair, retained owners and independent numerical collaborators"
    )]
    pub(crate) fn try_update_paired_skeleton_grounded_live<A, N, S, M, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
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
            |normal, backend, hooks, board, constructor| {
                use SkeletonGroundedUpdateError as E;
                let constructor =
                    constructor.ok_or(E::Input(GroundedInputError::Missing("constructor")))?;
                let bindings =
                    constructor
                        .bindings
                        .as_ref()
                        .ok_or(E::Input(GroundedInputError::Missing(
                            "constructor bindings",
                        )))?;
                if !Arc::ptr_eq(&bindings.processed, processed)
                    || !bindings
                        .owner
                        .upgrade()
                        .is_some_and(|v| Arc::ptr_eq(&v, expected))
                {
                    return Err(E::Input(GroundedInputError::ForeignOwner));
                }
                let root_dt = constructor
                    .outer
                    .as_ref()
                    .ok_or(E::Input(GroundedInputError::Missing(
                        "constructor remainder",
                    )))?
                    .time_delta;
                let mut guard = processed
                    .lock()
                    .map_err(|_| E::Input(GroundedInputError::Poisoned))?;
                let input = GroundedProcessed::new(&mut guard);
                normal.try_update_grounded_complete_read(
                    backend, hooks, board, root_dt, &input, &input, &input, reckoning, numeric,
                    sin_cos, ortho, cvars,
                )
            },
        )
    }
}
