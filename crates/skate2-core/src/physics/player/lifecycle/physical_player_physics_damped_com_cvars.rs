//! Retained targets used by `UpdateDampedComPos`82e2a720; definitions195..201.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
pub(crate) struct DampedComTarget(Target);
pub(crate) struct DampedComCvars<'a>(pub(crate) &'a InputCvarReferences);
impl DampedComCvars<'_> {
    pub(crate) fn capture(&self, index: usize) -> Result<DampedComTarget, InputCvarError> {
        self.0.targets.lock().map_err(|_| InputCvarError::Lock)?[index]
            .clone()
            .map(DampedComTarget)
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))
    }
    pub(crate) fn float(target: &DampedComTarget) -> Result<f32, InputCvarError> {
        match *target.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    pub(crate) fn draw(&self) -> Result<bool, InputCvarError> {
        match *self
            .capture(201)?
            .0
            .lock()
            .map_err(|_| InputCvarError::Lock)?
        {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
