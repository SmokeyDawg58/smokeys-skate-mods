//! LWCollideableObjectMan's assembly -> entity map, shared implementation for
//! the native per-category managers. Register82e5faf8, GetOwner82e5ceb8,
//! DeInit82e5e9c0. This is NOT an Island body pool or another entity roster.
use crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle;

#[derive(Default)]
pub(crate) struct CollideableObjectMan {
    entries: Vec<(SimulationDmoAssemblyHandle, u32)>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AssemblyOwnerError {
    DuplicateAssembly,
    Capacity,
}
impl CollideableObjectMan {
    pub(crate) const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    pub(crate) fn register(
        &mut self,
        assembly: &SimulationDmoAssemblyHandle,
        entity: u32,
    ) -> Result<(), AssemblyOwnerError> {
        // Native find precedes insertion; a duplicate asserts instead of
        // replacing ownership, even when the map has reached capacity128.
        if self
            .entries
            .iter()
            .any(|(handle, _)| handle.same_as(assembly))
        {
            return Err(AssemblyOwnerError::DuplicateAssembly);
        }
        if self.entries.len() == 128 {
            return Err(AssemblyOwnerError::Capacity);
        }
        self.entries.push((assembly.clone(), entity));
        Ok(())
    }
    pub(crate) fn owner_of(&self, assembly: &SimulationDmoAssemblyHandle) -> u32 {
        self.entries
            .iter()
            .find(|(handle, _)| handle.same_as(assembly))
            .map_or(0, |(_, entity)| *entity)
    }
    pub(crate) fn remove(&mut self, assembly: &SimulationDmoAssemblyHandle) {
        if let Some(index) = self
            .entries
            .iter()
            .position(|(handle, _)| handle.same_as(assembly))
        {
            self.entries.swap_remove(index);
        }
    }
}
