//! Actual Actor fields and cvar/Processed owners for the existing constructors.
//! This is a service handoff, not player selection or a complete startup facade.
use super::{
    PhysicalPlayerAssemblyConstruction, PhysicalPlayerAssemblyServices,
    PhysicalPlayerSkeletonFinishError,
};
use crate::{
    actor_construction::ActorInitialFields,
    input_cvar_runtime::{
        InputCvarError, InputCvarReferences,
        physical_constructor::ConstructorDriveCvars,
        postphysics_history_input::{HistoryInputError, PostPhysicsHistoryInput},
    },
    skateboard_spawn::FallibleSkateboardSpawnServices,
};
use std::sync::Arc;

/// Borrow the actual constructed Actor prefix fields, never a copied flag word.
/// The caller retains its existing Actor prefix guard for this constructor scope;
/// release it before any later operation that acquires the Actor again.
pub(crate) struct PhysicalConstructorServices<'a, A> {
    pub(crate) fields: &'a ActorInitialFields,
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) assertion: A,
}
impl<A: FnMut()> FallibleSkateboardSpawnServices for PhysicalConstructorServices<'_, A> {
    fn try_current_orthogonal_tolerance(&mut self) -> Result<f32, InputCvarError> {
        self.references.try_orthogonal_tolerance()
    }
    fn orthogonality_assertion_failed(&mut self) {
        (self.assertion)();
    }
}
impl<A: FnMut()> PhysicalPlayerAssemblyServices for PhysicalConstructorServices<'_, A> {
    fn current_entity_flags(&mut self) -> u32 {
        self.fields.flags
    }
}

#[derive(Debug)]
pub(crate) enum ConstructorFinishError {
    ReckoningPoisoned,
    Finish(PhysicalPlayerSkeletonFinishError<InputCvarError, HistoryInputError>),
}
impl std::fmt::Display for ConstructorFinishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReckoningPoisoned => f.write_str("constructor Reckoning owner poisoned"),
            Self::Finish(error) => write!(f, "live skeleton constructor: {error}"),
        }
    }
}
impl PhysicalPlayerAssemblyConstruction<'_> {
    /// Reuse the original drives -> placement -> history -> children sequence.
    /// Processed's constructor already established Up; no extra Reset is issued.
    pub(crate) fn try_finish_skeleton_from_live_services(
        &mut self,
    ) -> Result<(), ConstructorFinishError> {
        let references = self.state.owner.inputs().cvars.clone();
        let processed = Arc::clone(
            self.state
                .owner
                .reckoning()
                .lock()
                .map_err(|_| ConstructorFinishError::ReckoningPoisoned)?
                .processed(),
        );
        self.try_finish_skeleton(
            &mut ConstructorDriveCvars(&references),
            &mut PostPhysicsHistoryInput {
                references: &references,
                processed: &processed,
            },
        )
        .map_err(ConstructorFinishError::Finish)
    }
}

#[cfg(test)]
#[path = "physical_player_constructor_services_tests.rs"]
mod tests;
