//! `PlayerGrabSpline.InvalidateQuery82dcc290`, actual manager before ready stores.
use super::{GrabConstructorError, PlayerGrabSpline};
#[derive(Debug)]
pub(crate) enum GrabInvalidateError {
    MissingManager,
    ForeignWorld,
    Manager(GrabConstructorError),
    MissingBatchOwner,
    Line(crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineLeaseError),
    QueryPoisoned,
}
impl std::fmt::Display for GrabInvalidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingManager => f.write_str("grab invalidation missing actual manager"),
            Self::ForeignWorld => f.write_str("grab invalidation foreign manager"),
            Self::Manager(e) => e.fmt(f),
            Self::MissingBatchOwner => f.write_str("grab invalidation missing World line owner"),
            Self::Line(e) => e.fmt(f),
            Self::QueryPoisoned => f.write_str("grab invalidation query lock poisoned"),
        }
    }
}
impl PlayerGrabSpline {
    pub(crate) fn try_invalidate_query(
        &mut self,
        lines: Option<&mut crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueries>,
    ) -> Result<(), GrabInvalidateError> {
        use GrabInvalidateError as E;
        let manager = self.manager.as_ref().ok_or(E::MissingManager)?;
        if !manager.belongs_to(self.world) {
            return Err(E::ForeignWorld);
        }
        manager.remove_deferred(&self.query).map_err(E::Manager)?; //c2bc
        if let Some(batch) = self.line_batch.as_ref() {
            let lines = lines.ok_or(E::MissingBatchOwner)?;
            lines.try_sync(batch).map_err(E::Line)?; //c2dc
            lines.try_release(batch).map_err(E::Line)?; //c2f0
            self.line_batch = None;
        }
        let old_flags = self.flags; //c2f8 capture before ready stores
        self.query.lock().map_err(|_| E::QueryPoisoned)?.ready = false; //c2fc
        self.data.lock().map_err(|_| E::QueryPoisoned)?.ready = false; //c304
        self.alternate.lock().map_err(|_| E::QueryPoisoned)?.ready = false; //c308
        self.flags = old_flags & 0x3f; //c30c, preserves lower6 and all query payloads
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::PlayerGrabSpline;
    #[test]
    fn clears_only_source_readiness_and_high_flags_with_real_constructed_manager() {
        let (_, owner) = crate::physical_player_assembly::tests::issued_owner_for_grind_tests();
        let manager = std::sync::Arc::new(
            crate::physical_player_grab_spline::manager::GrabQueryManager::new(owner.manager()),
        );
        let mut grab = PlayerGrabSpline::new_prefix(&owner, Some(manager));
        grab.try_finish().unwrap();
        grab.query.lock().unwrap().ready = true;
        grab.query.lock().unwrap().result.result_count = 3;
        grab.data.lock().unwrap().ready = true;
        grab.alternate.lock().unwrap().ready = true;
        grab.flags = 0xed;
        grab.interactable.lock().unwrap().ready = true;
        grab.try_invalidate_query(None).unwrap();
        assert!(!grab.query.lock().unwrap().ready);
        assert_eq!(grab.query.lock().unwrap().result.result_count, 3);
        assert!(!grab.data.lock().unwrap().ready && !grab.alternate.lock().unwrap().ready);
        assert_eq!(grab.flags, 0x2d);
        assert!(grab.interactable.lock().unwrap().ready);
    }
}
