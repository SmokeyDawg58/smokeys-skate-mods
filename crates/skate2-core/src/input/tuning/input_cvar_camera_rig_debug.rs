//! The three live booleans at the actual `Rig.DoUpdate` return.
use super::{InputCvarError, InputCvarReferences};

#[derive(Clone, Copy)]
pub(crate) enum RigDebugBoolean {
    Tracker,
    Avoidance,
    Intended,
}
impl InputCvarReferences {
    pub(crate) fn try_camera_rig_debug(
        &self,
        field: RigDebugBoolean,
    ) -> Result<bool, InputCvarError> {
        self.try_current_boolean(match field {
            RigDebugBoolean::Tracker => 333,
            RigDebugBoolean::Avoidance => 334,
            RigDebugBoolean::Intended => 335,
        })
    }
}
