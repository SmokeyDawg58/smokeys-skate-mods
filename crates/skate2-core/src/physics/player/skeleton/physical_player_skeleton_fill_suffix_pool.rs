//! Unregistered `skeleton_members` child: complete the SAME Skeleton.Fill call.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    debug_overlay::DebugOverlaySlot,
    dynamic_assembly_session::{
        skater_pool::SimulationSkeletonJumpPairError,
        skeleton_normal::fill_live::SkeletonFillLiveError,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::construct::skeleton_fill_suffix::SkeletonFillSuffixError,
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum PairedSkeletonSuffixError<N> {
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    ForeignOwner,
    MissingConstructor,
    MissingBindings,
    ReckoningPoisoned,
    OutputPoisoned,
    Fill(SkeletonFillSuffixError<SkeletonFillLiveError<N>, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for PairedSkeletonSuffixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(error) => error.fmt(f),
            Self::Pool(error) => error.fmt(f),
            Self::ForeignOwner => f.write_str("Skeleton suffix foreign owner or Processed binding"),
            Self::MissingConstructor => f.write_str("Skeleton suffix missing actual constructor"),
            Self::MissingBindings => f.write_str("Skeleton suffix missing constructor bindings"),
            Self::ReckoningPoisoned => f.write_str("Skeleton suffix Reckoning owner lock poisoned"),
            Self::OutputPoisoned => f.write_str("Skeleton suffix PhysOut lock poisoned"),
            Self::Fill(error) => error.fmt(f),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    /// Host identity checks admit the actual call; no physics/tuning values are
    /// gathered here. The passed velocity is v124 from the earlier585b0 load.
    #[expect(
        clippy::too_many_arguments,
        reason = "same actual owners, output and numeric services remain explicit"
    )]
    pub(crate) fn try_fill_paired_skeleton_suffix<N: ReckoningNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
        captured_com_velocity: SkeletonRootVector,
        references: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), PairedSkeletonSuffixError<N::Error>> {
        use PairedSkeletonSuffixError as E;
        let owner = self
            .try_with_jump_pair(backend, board, skeleton, |_, actual| Arc::clone(actual))
            .map_err(E::Pair)?;
        if !Arc::ptr_eq(&owner, expected_owner) {
            return Err(E::ForeignOwner);
        }
        // Resolve only the existing Processed identity here. The actual
        // Reckoning.up value must be read later by59658/5c's short callback.
        let processed = {
            let reckoning = owner.reckoning().lock().map_err(|_| E::ReckoningPoisoned)?;
            Arc::clone(reckoning.processed())
        };
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let MemberContent::Skeleton { candidate, .. } = &mut self.members[index].content else {
            unreachable!("validated Skeleton member")
        };
        let construction = candidate
            .construction
            .as_ref()
            .ok_or(E::MissingConstructor)?;
        let bindings = construction
            .constructor
            .bindings
            .as_ref()
            .ok_or(E::MissingBindings)?;
        if !Arc::ptr_eq(&bindings.processed, &processed)
            || bindings
                .owner
                .upgrade()
                .is_none_or(|bound| !Arc::ptr_eq(&bound, &owner))
        {
            return Err(E::ForeignOwner);
        }
        // This is the SAME continuing allocation. Child kernels borrow it;
        // none relocks this mutex, resets it, or publishes a replacement.
        let mut output = owner.phys_out().lock().map_err(|_| E::OutputPoisoned)?;
        candidate
            .owner
            .try_fill_output_suffix_live(
                backend,
                &owner,
                &processed,
                &owner.inputs().vault,
                references,
                overlay,
                &mut output,
                captured_com_velocity,
                numeric,
            )
            .map_err(E::Fill)
    }
}
