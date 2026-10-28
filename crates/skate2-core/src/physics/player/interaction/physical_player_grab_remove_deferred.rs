//! Actual locked stable removal, `RemoveDeferredGrabSplineQuery827b0648`.
use super::{GrabConstructorError, GrabQueryFuture, GrabQueryManager};
use std::sync::{Arc, Mutex};
impl GrabQueryManager {
    pub(crate) fn remove_deferred(
        &self,
        output: &Arc<Mutex<GrabQueryFuture>>,
    ) -> Result<(), GrabConstructorError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| GrabConstructorError::ManagerPoisoned)?;
        // Native removes every matching occurrence, shifting successors left;
        // it never advances past the shifted record after a removal.
        state
            .queries
            .retain(|query| !Arc::ptr_eq(&query.output, output));
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{Arc, GrabQueryFuture, Mutex};
    #[test]
    fn actual_identity_removes_all_matches_without_reordering_other_queries() {
        let (_, owner) = crate::physical_player_assembly::tests::issued_owner_for_grind_tests();
        let manager = super::GrabQueryManager::new(owner.manager());
        let make = || {
            Arc::new(Mutex::new(GrabQueryFuture {
                result: crate::physical_player_grab_spline::GrabQueryBuffer::in_sim_allocation(),
                ready: true,
            }))
        };
        let a = make();
        let b = make();
        let c = make();
        for output in [&a, &b, &a, &a, &c] {
            manager.state.lock().unwrap().queries.push(
                super::super::DeferredQuery::identity_fixture(Arc::clone(output)),
            );
        }
        manager.remove_deferred(&a).unwrap();
        let state = manager.state.lock().unwrap();
        assert_eq!(state.queries.len(), 2);
        assert!(Arc::ptr_eq(&state.queries[0].output, &b));
        assert!(Arc::ptr_eq(&state.queries[1].output, &c));
        assert!(
            a.lock().unwrap().ready,
            "manager removal alone does not clear future ready"
        );
    }
}
