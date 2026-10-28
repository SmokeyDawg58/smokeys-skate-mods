//! Native outer bad-data store37d68 on the existing constructor-owned byte101.
use super::SkateboardEntryState;

impl SkateboardEntryState {
    pub(crate) fn request_respawn_for_bad_data(&mut self) {
        self.wants_respawn = true;
    }
}
