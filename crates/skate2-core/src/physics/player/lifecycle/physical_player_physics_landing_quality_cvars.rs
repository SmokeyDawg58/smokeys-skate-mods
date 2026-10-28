//! Live `CalcLandingQuality` controls; reserved definitions205..212.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
pub(crate) struct LandingTarget(Target);
pub(crate) struct LandingCvars<'a>(pub(crate) &'a InputCvarReferences);
impl LandingCvars<'_> {
    pub(crate) fn capture(&self, index: usize) -> Result<LandingTarget, InputCvarError> {
        self.0.targets.lock().map_err(|_| InputCvarError::Lock)?[index]
            .clone()
            .map(LandingTarget)
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))
    }
    pub(crate) fn read(target: &LandingTarget) -> Result<f32, InputCvarError> {
        match *target.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(v) => Ok(v),
            _ => Err(InputCvarError::WrongType),
        }
    }
    pub(crate) fn value(&self, index: usize) -> Result<f32, InputCvarError> {
        Self::read(&self.capture(index)?)
    }
}
