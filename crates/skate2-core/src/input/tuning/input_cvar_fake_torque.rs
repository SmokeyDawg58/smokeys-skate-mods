//! Draft live refs125..127; native static order is Apply, Value, Exponent.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Value};
use crate::skateboard_force_runtime::fake_torque::FakeTorqueRead;
pub(crate) struct FakeTorqueCvars<'a>(pub(crate) &'a InputCvarReferences);
impl FakeTorqueRead for FakeTorqueCvars<'_> {
    type Error = InputCvarError;
    fn enabled(&mut self) -> Result<bool, Self::Error> {
        self.0.try_current_boolean(125)
    }
    fn exponent(&mut self) -> Result<i32, Self::Error> {
        let target = self.0.targets.lock().map_err(|_| InputCvarError::Lock)?[127]
            .clone()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[127].label))?;
        let value = *target.lock().map_err(|_| InputCvarError::Lock)?;
        match value {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    fn multiplier(&mut self) -> Result<f32, Self::Error> {
        self.0.try_current_float(126)
    }
}
