//! Source collision publication at82d9a5ec; unregistered Entry child.
use super::SkateboardEntryState;
impl SkateboardEntryState {
    pub(crate) fn mark_collision_response(&mut self) {
        // SAME native530/80000000 field subsequently consumed by SpeedModel.
        self.reset_target_after_collision = true;
    }
}
