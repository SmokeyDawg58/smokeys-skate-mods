//! Qualified hip-hook writes on the same physical pair's retained dynamics.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::{
        skater_pool::SimulationSkeletonJumpPairError,
        skeleton_normal::drive_owner::hook_soft::{HipHookSoftError, HipHookSoftKind},
    },
};

#[derive(Debug)]
pub(crate) enum PairHipHookSoftError<N> {
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    Hook(HipHookSoftError<AttributeRuntimeError, N>),
}
impl<N: std::fmt::Display> std::fmt::Display for PairHipHookSoftError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(e) => write!(f, "hip soft pair: {e}"),
            Self::Pool(e) => write!(f, "hip soft pool: {e}"),
            Self::Hook(e) => write!(f, "{e}"),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_pair_hip_hook_soft<N>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        kind: HipHookSoftKind,
        strength: f32,
        estimate: &mut impl FnMut(f32) -> Result<f32, N>,
    ) -> Result<(), PairHipHookSoftError<N>> {
        use PairHipHookSoftError as E;
        self.try_with_jump_pair(backend, board, skeleton, |_, _| ())
            .map_err(E::Pair)?;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton")
        };
        let reader = candidate
            .construction
            .as_mut()
            .and_then(|c| c.body_reader_mut())
            .ok_or(E::Pool(
                SimulationSkaterPoolError::MissingPhysicalConstruction,
            ))?;
        candidate
            .owner
            .try_set_hip_hook_soft(backend, reader, kind, strength, estimate)
            .map_err(E::Hook)
    }
}
