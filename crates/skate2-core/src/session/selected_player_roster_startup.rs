//! Source-ordered continuation after the same Actor physical constructor returns.
use super::{SelectedPlayerStartupError, SelectedPlayerStartupOwners};
use crate::{
    actor_construction::{ActorConstruction, graph_startup::ActorGraphStartupOwners},
    simulation_entity_roster::SimulationEntityRoster,
    skater_manager_local_identity::roster::RosterAdmissionError,
};
use std::{convert::Infallible, sync::Arc};
#[derive(Debug)]
pub(crate) enum SelectedRosterStartupError {
    Roster(RosterAdmissionError),
    MissingGraphOwner,
    GraphLock,
    Graphs(SelectedPlayerStartupError<Infallible, Infallible>),
}
impl std::fmt::Display for SelectedRosterStartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGraphOwner => f.write_str("shared graph owner missing"),
            Self::GraphLock => f.write_str("shared graph owner lock poisoned"),
            Self::Roster(error) => error.fmt(f),
            Self::Graphs(error) => write!(f, "roster graph startup: {error:?}"),
        }
    }
}
impl SelectedPlayerStartupOwners {
    pub(crate) fn try_admit_first_actor(
        &mut self,
        actor: &Arc<ActorConstruction>,
        entities: &SimulationEntityRoster,
    ) -> Result<(), SelectedRosterStartupError> {
        use SelectedRosterStartupError as E;
        self.services
            .skaters
            .try_publish_first_actor(actor, entities)
            .map_err(E::Roster)?;
        self.try_load_player_graphs(actor)?;
        self.services
            .skaters
            .try_finish_first_player_observers(actor)
            .map_err(E::Roster)
    }
    /// Retain the one boot-owned cache before camera observer capture.
    /// Enabling/loading the Actor graphs remains at the original `InitPlayer` seam.
    pub(crate) fn try_ensure_player_graph_owners(
        &mut self,
    ) -> Result<(), SelectedRosterStartupError> {
        self.graphs
            .as_mut()
            .ok_or(SelectedRosterStartupError::MissingGraphOwner)?
            .lock().map_err(|_|SelectedRosterStartupError::GraphLock)?
            .try_preload_living_world_controllers(&self.cvars)
            .map_err(|e| SelectedRosterStartupError::Graphs(SelectedPlayerStartupError::Graphs(e)))
    }
    pub(crate) fn try_load_player_graphs(
        &mut self,
        actor: &ActorConstruction,
    ) -> Result<(), SelectedRosterStartupError> {
        self.loaded_graphs = Some(self.try_load_skater_graphs(actor)?);
        Ok(())
    }
    /// `InitPlayer` 0x826000b0: enable the generated graphs for this skater, then load
    /// its action and motion graphs from the shared state cache.
    pub(crate) fn try_load_skater_graphs(
        &mut self,
        actor: &ActorConstruction,
    ) -> Result<[bool; 2], SelectedRosterStartupError> {
        use SelectedRosterStartupError as E;
        self.try_ensure_player_graph_owners()?;
        let mut graphs = self.graphs.as_ref().ok_or(E::MissingGraphOwner)?.lock().map_err(|_|E::GraphLock)?;
        graphs
            .try_enable_player(actor, &self.cvars)
            .map_err(|e| E::Graphs(SelectedPlayerStartupError::Graphs(e)))?;
        let action = actor
            .try_load_action_graph(&mut graphs)
            .map_err(|e| E::Graphs(SelectedPlayerStartupError::Graphs(e)))?;
        let motion = actor
            .try_load_motion_graph(&mut graphs)
            .map_err(|e| E::Graphs(SelectedPlayerStartupError::Graphs(e)))?;
        Ok([action, motion])
    }
}
