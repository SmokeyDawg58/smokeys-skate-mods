//! Workspace.Init828a7ed8/7edc constructs the actual empty Entity vector.
use crate::{
    actor_construction::ActorConstruction,
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
};
use std::sync::{Arc, Mutex};
pub(crate) struct SimulationEntityRoster {
    manager: SimulationManagerContactAuthority,
    actors: Mutex<Vec<Arc<ActorConstruction>>>,
}
#[derive(Debug)]
pub(crate) enum EntityRosterError {
    ForeignManager,
    Poisoned,
    Allocation,
    FirstSpawnRequired,
}
impl SimulationEntityRoster {
    /// Borrowed frame membership keeps the same retained Actors and vector order.
    pub(crate) fn snapshot(&self) -> Result<Vec<Arc<ActorConstruction>>, EntityRosterError> {
        Ok(self.actors.lock().map_err(|_| EntityRosterError::Poisoned)?.clone())
    }
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "workspace boot currently exercised by original corpus"
        )
    )]
    pub(crate) fn construct(manager: SimulationManagerContactAuthority) -> Arc<Self> {
        Arc::new(Self {
            manager,
            actors: Mutex::new(Vec::new()),
        })
    }
    pub(crate) fn belongs_to(&self, actor: &ActorConstruction) -> bool {
        actor.belongs_to_physics_manager(self.manager)
    }
    pub(crate) fn append_first(
        &self,
        actor: &Arc<ActorConstruction>,
    ) -> Result<(), EntityRosterError> {
        if !self.belongs_to(actor) {
            return Err(EntityRosterError::ForeignManager);
        }
        let mut actors = self
            .actors
            .lock()
            .map_err(|_| EntityRosterError::Poisoned)?;
        if !actors.is_empty() {
            return Err(EntityRosterError::FirstSpawnRequired);
        }
        actors
            .try_reserve(1)
            .map_err(|_| EntityRosterError::Allocation)?;
        actors.push(Arc::clone(actor));
        Ok(())
    }
    /// `SpawnPlayer` 0x82600630's `EntityManager` append for a later skater.
    pub(crate) fn append(&self, actor: &Arc<ActorConstruction>) -> Result<(), EntityRosterError> {
        if !self.belongs_to(actor) {
            return Err(EntityRosterError::ForeignManager);
        }
        let mut actors = self
            .actors
            .lock()
            .map_err(|_| EntityRosterError::Poisoned)?;
        if actors.is_empty() {
            return Err(EntityRosterError::FirstSpawnRequired);
        }
        actors
            .try_reserve(1)
            .map_err(|_| EntityRosterError::Allocation)?;
        actors.push(Arc::clone(actor));
        Ok(())
    }
    /// `DestroyPlayer` 0x825fff80's `EntityManager` removal (`eastl::remove`, then erase).
    pub(crate) fn remove(&self, actor: &Arc<ActorConstruction>) -> Result<(), EntityRosterError> {
        self.actors
            .lock()
            .map_err(|_| EntityRosterError::Poisoned)?
            .retain(|entry| !Arc::ptr_eq(entry, actor));
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn contains(
        &self,
        actor: &Arc<ActorConstruction>,
    ) -> Result<bool, EntityRosterError> {
        Ok(self
            .actors
            .lock()
            .map_err(|_| EntityRosterError::Poisoned)?
            .iter()
            .any(|entry| Arc::ptr_eq(entry, actor)))
    }
}
