//! Actual captured vibration cvar targets; null low-scale target skips tuning.
use super::{InputCvarError, InputCvarReferences, Target, Value};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(crate) enum PadVibrationFloat {
    LowScale,
    LowOffset,
    HighScale,
    HighOffset,
}
pub(crate) struct PadVibrationTarget(Target);
impl PadVibrationTarget {
    pub(crate) fn read(&self) -> Result<f32, InputCvarError> {
        match *self.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
impl InputCvarReferences {
    pub(crate) fn capture_pad_vibration(
        &self,
        field: PadVibrationFloat,
    ) -> Result<Option<PadVibrationTarget>, InputCvarError> {
        let index = match field {
            PadVibrationFloat::LowScale => 282,
            PadVibrationFloat::LowOffset => 283,
            PadVibrationFloat::HighScale => 284,
            PadVibrationFloat::HighOffset => 285,
        };
        Ok(
            self.targets.lock().map_err(|_| InputCvarError::Lock)?[index]
                .as_ref()
                .map(|target| PadVibrationTarget(Arc::clone(target))),
        )
    }
}
#[cfg(test)]
#[path = "input_cvar_pad_vibration_tests.rs"]
mod tests;
