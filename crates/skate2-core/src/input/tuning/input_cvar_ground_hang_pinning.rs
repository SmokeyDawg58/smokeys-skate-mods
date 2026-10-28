//! Proposed live integer references150..153; native tables5748/5750/5768/576c.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Value};
impl InputCvarReferences {
    fn ground_hang_pin_integer(&self, index: usize) -> Result<i32, InputCvarError> {
        let target = self.targets.lock().map_err(|_| InputCvarError::Lock)?[index]
            .clone()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?;
        let value = *target.lock().map_err(|_| InputCvarError::Lock)?;
        match value {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    pub(crate) fn try_hang_frames(&self) -> Result<i32, InputCvarError> {
        self.ground_hang_pin_integer(150)
    }
    pub(crate) fn try_hung_up_wipeout_frames(&self) -> Result<i32, InputCvarError> {
        self.ground_hang_pin_integer(151)
    }
    pub(crate) fn try_max_ground_pin_frames(&self) -> Result<i32, InputCvarError> {
        self.ground_hang_pin_integer(152)
    }
    pub(crate) fn try_max_ground_pin_start_frames(&self) -> Result<i32, InputCvarError> {
        self.ground_hang_pin_integer(153)
    }
}
