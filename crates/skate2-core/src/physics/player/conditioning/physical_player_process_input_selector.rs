//! Retained selector polling at Update1 82e03a84..3aa4.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_trajectory_owners::OnboardTrajectoryOwner,
};
use std::{fmt, sync::Arc};
#[derive(Debug)]
pub(crate) enum SelectorPollError {
    Missing(&'static str),
    Poisoned,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    CompletedQueryPending,
}
impl fmt::Display for SelectorPollError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "selector polling missing {field}"),
            Self::Poisoned => f.write_str("selector polling owner lock poisoned"),
            Self::Pair(e) => write!(f, "selector polling pair: {e:?}"),
            Self::ForeignOwner => {
                f.write_str("selector belongs to another physical or processed owner")
            }
            Self::CompletedQueryPending => {
                f.write_str("completed selector query awaits native scoring continuation")
            }
        }
    }
}
impl OnboardTrajectoryOwner {
    /// Update6544..655c and6910: an unfinished query returns retained ready.
    pub(crate) fn try_poll_before_scoring(&mut self) -> Result<bool, SelectorPollError> {
        let started = self.just_started_query;
        self.has_just_changed = false; //6548, even before a pending continuation
        if started && self.queries[0].ready {
            return Err(SelectorPollError::CompletedQueryPending);
        }
        Ok(self.ready)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_poll_pair_trajectory_selector(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SelectorPollError> {
        use SelectorPollError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let ready = {
            let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_mut()
                .ok_or(E::Missing("onboard selector"))?;
            if !selector.bindings.matches(&owner, &processed) {
                return Err(E::ForeignOwner);
            }
            selector.try_poll_before_scoring()?
        };
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .publish_trajectory_selector_ready(ready);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{OnboardTrajectoryOwner, SelectorPollError};
    use crate::{
        physical_player_assembly::tests::issued_owner_for_grind_tests,
        physical_player_trajectory_owners::TrajectoryOwnerBindings,
        skeleton_processed_contact::SkeletonProcessedContact,
    };
    use std::sync::{Arc, Mutex};
    #[test]
    fn retained_ready_is_not_replaced_by_an_assumed_idle_value() {
        let (_world, owner) = issued_owner_for_grind_tests();
        let processed = Arc::new(Mutex::new(SkeletonProcessedContact::new()));
        let mut selector =
            OnboardTrajectoryOwner::new(TrajectoryOwnerBindings::new(&owner, processed), false);
        assert!(!selector.try_poll_before_scoring().unwrap());
        // Controlled future states exercise the actual retained polling routine.
        selector.ready = true;
        selector.queries[0].ready = true;
        selector.has_just_changed = true;
        assert!(selector.try_poll_before_scoring().unwrap()); // unstarted ignores ready future
        assert!(!selector.has_just_changed);
        selector.just_started_query = true;
        selector.queries[0].ready = false;
        assert!(selector.try_poll_before_scoring().unwrap());
        selector.queries[0].ready = true;
        selector.has_just_changed = true;
        assert!(matches!(
            selector.try_poll_before_scoring(),
            Err(SelectorPollError::CompletedQueryPending)
        ));
        assert!(!selector.has_just_changed);
        assert!(selector.ready && selector.just_started_query && selector.queries[0].ready);
    }
}
