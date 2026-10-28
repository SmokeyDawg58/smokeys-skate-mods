//! Pedestrian specialization, CreateExecutionGroups82e9ee10/Init82e9ef00.
//! Targets reference the sole EntityMan40; no entity payloads are copied.
use super::{ExecutionMan, ExecutionEntity, ExecutionError};
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_grind_services::entity_core::CommonEntityCore,
    input_cvar_runtime::InputCvarReferences};
pub(crate) type PedestrianExecutionMan = ExecutionMan<40, PedestrianConstruction>;
pub(crate) use super::{ExecutionTarget as PedestrianExecutionTarget,
    ExecutionTaskError as PedestrianExecutionTaskError};
impl ExecutionEntity for PedestrianConstruction {
    fn execution_core(&self) -> Option<&CommonEntityCore> { self.core.as_ref() }
    fn execution_core_mut(&mut self) -> Option<&mut CommonEntityCore> { self.core.as_mut() }
}
impl ExecutionMan<40, PedestrianConstruction> {
    pub(crate) fn initialize(&mut self, cvars: &InputCvarReferences) -> Result<(), ExecutionError> {
        self.initialize_with(|| cvars.try_pedestrian_execution_rates())
    }
}
