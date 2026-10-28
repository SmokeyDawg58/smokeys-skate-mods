//! Live `input_cvar_runtime::ground_wipeout` references193/194.
use super::{InputCvarError, InputCvarReferences, Target, Value};
pub(crate) struct GroundDrawTarget(Target);
impl InputCvarReferences {
    pub(crate) fn try_capture_ground_wipeout_draw(
        &self,
    ) -> Result<GroundDrawTarget, InputCvarError> {
        self.targets.lock().map_err(|_| InputCvarError::Lock)?[193]
            .clone()
            .map(GroundDrawTarget)
            .ok_or(InputCvarError::Unlinked("kDrawSkeletonCollisionWipeout"))
    }
    pub(crate) fn try_ground_wipeout_on_collision(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(194)
    }
}
impl GroundDrawTarget {
    pub(crate) fn read(&self) -> Result<bool, InputCvarError> {
        match *self.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
