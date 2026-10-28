//! Source `SpawnPlayer` roster/entity prefix; actual observer dispatch stays pending.
use super::{LocalIdentityError, SkaterManagerLocalIdentity};
use crate::{
    actor_construction::{ActorConstruction, physical_mount::ActorPhysicalMountError},
    simulation_entity_roster::{EntityRosterError, SimulationEntityRoster},
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum RosterAdmissionError {
    Identity(LocalIdentityError),
    Actor(ActorPhysicalMountError),
    Entity(EntityRosterError),
    FirstSpawnRequired,
    Allocation,
    Observers(super::observers::SkaterObserverError),
}
impl std::fmt::Display for RosterAdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identity(error) => write!(f, "roster identity: {error:?}"),
            Self::Actor(error) => write!(f, "roster Actor: {error:?}"),
            Self::Entity(error) => write!(f, "roster entity: {error:?}"),
            Self::Observers(error) => error.fmt(f),
            other => write!(f, "selected roster admission: {other:?}"),
        }
    }
}
impl SkaterManagerLocalIdentity {
    pub(crate) fn try_count(&self) -> Result<usize, LocalIdentityError> {
        Ok(self
            .fields
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?
            .skater_list
            .len())
    }
    pub(crate) fn try_actor(
        &self,
        index: usize,
    ) -> Result<Option<Arc<ActorConstruction>>, LocalIdentityError> {
        Ok(self
            .fields
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?
            .skater_list
            .get(index)
            .cloned())
    }
    /// Source constructor return precedes roster append826006f4, Entity append
    ///82600758 and sort82600780. A single first entry needs no sort comparison.
    pub(crate) fn try_publish_first_actor(
        &self,
        actor: &Arc<ActorConstruction>,
        entities: &SimulationEntityRoster,
    ) -> Result<(), RosterAdmissionError> {
        use RosterAdmissionError as E;
        actor.cached_physical_allocation().map_err(E::Actor)?;
        if !entities.belongs_to(actor) {
            return Err(E::Entity(EntityRosterError::ForeignManager));
        }
        {
            let mut fields = self
                .fields
                .lock()
                .map_err(|_| E::Identity(LocalIdentityError::Poisoned))?;
            if !fields.reset || !fields.skater_list.is_empty() {
                return Err(E::FirstSpawnRequired);
            }
            fields
                .skater_list
                .try_reserve(1)
                .map_err(|_| E::Allocation)?;
            fields.skater_list.push(Arc::clone(actor));
        }
        entities.append_first(actor).map_err(E::Entity)?;
        Ok(())
    }
    /// `SpawnPlayer` 0x82600630 for a skater after the first: the skater list
    /// append, the `EntityManager` append, then `SortByIndex` over the list.
    pub(crate) fn try_publish_actor(
        &self,
        actor: &Arc<ActorConstruction>,
        entities: &SimulationEntityRoster,
    ) -> Result<(), RosterAdmissionError> {
        use RosterAdmissionError as E;
        actor.cached_physical_allocation().map_err(E::Actor)?;
        if !entities.belongs_to(actor) {
            return Err(E::Entity(EntityRosterError::ForeignManager));
        }
        {
            let mut fields = self
                .fields
                .lock()
                .map_err(|_| E::Identity(LocalIdentityError::Poisoned))?;
            if !fields.reset || fields.skater_list.is_empty() {
                return Err(E::FirstSpawnRequired);
            }
            fields
                .skater_list
                .try_reserve(1)
                .map_err(|_| E::Allocation)?;
            fields.skater_list.push(Arc::clone(actor));
        }
        entities.append(actor).map_err(E::Entity)?;
        let mut fields = self
            .fields
            .lock()
            .map_err(|_| E::Identity(LocalIdentityError::Poisoned))?;
        let mut keyed = Vec::with_capacity(fields.skater_list.len());
        for skater in fields.skater_list.drain(..) {
            let index = skater
                .prefix()
                .map_err(|_| E::Identity(LocalIdentityError::Poisoned))?
                .fields()
                .ok_or(E::Identity(LocalIdentityError::ActorFieldsUnavailable))?
                .player_index;
            keyed.push((index, skater));
        }
        keyed.sort_by_key(|(index, _)| *index);
        fields.skater_list.extend(keyed.into_iter().map(|(_, skater)| skater));
        Ok(())
    }
    /// `DestroyPlayer` 0x825fff80 for a skater after the first: the player's Actor leaves
    /// the `EntityManager` and the skater list (whose `SortByIndex` order its erase keeps),
    /// then `UpdateLocalPlayer` and the observers' Destroyed notification. The caller then
    /// deletes the returned Actor; an absent player is no work.
    pub(crate) fn try_destroy_player(
        &self,
        player: i32,
        entities: &SimulationEntityRoster,
    ) -> Result<Option<Arc<ActorConstruction>>, RosterAdmissionError> {
        use RosterAdmissionError as E;
        let Some(actor) = self.try_find_by_index(player).map_err(E::Identity)? else {
            return Ok(None);
        };
        entities.remove(&actor).map_err(E::Entity)?;
        self.fields
            .lock()
            .map_err(|_| E::Identity(LocalIdentityError::Poisoned))?
            .skater_list
            .retain(|skater| !Arc::ptr_eq(skater, &actor));
        self.try_update_local_player().map_err(E::Observers)?;
        self.try_notify_destroyed_actor(&actor).map_err(E::Observers)?;
        Ok(Some(actor))
    }
    /// `InitPlayer` 0x826000b0's Created notification and `UpdateLocalPlayer` for
    /// a skater after the first.
    pub(crate) fn try_finish_player_observers(
        &self,
        actor: &Arc<ActorConstruction>,
    ) -> Result<(), RosterAdmissionError> {
        self.try_notify_first_actor(actor)
            .map_err(RosterAdmissionError::Observers)
    }
    /// Native Created notification and `UpdateLocalPlayer` continuation.
    pub(crate) fn try_finish_first_player_observers(
        &self,
        actor: &Arc<ActorConstruction>,
    ) -> Result<(), RosterAdmissionError> {
        self.try_notify_first_actor(actor)
            .map_err(RosterAdmissionError::Observers)
    }
}
#[cfg(test)]
#[path = "skater_manager_roster_tests.rs"]
mod tests;
