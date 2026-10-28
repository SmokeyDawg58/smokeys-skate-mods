//! Actual retained decision controls. Root appends definitions159..190.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
pub(crate) struct WipeoutEnableTarget(Target);
pub(crate) struct WipeoutDecisionCvars<'a>(&'a InputCvarReferences);
impl<'a> WipeoutDecisionCvars<'a> {
    pub(crate) const fn new(references: &'a InputCvarReferences) -> Self {
        Self(references)
    }
    fn capture(&self, index: usize) -> Result<WipeoutEnableTarget, InputCvarError> {
        self.0.targets.lock().map_err(|_| InputCvarError::Lock)?[index]
            .clone()
            .map(WipeoutEnableTarget)
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))
    }
    pub(crate) fn read_enable(target: &WipeoutEnableTarget) -> Result<bool, InputCvarError> {
        match *target.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    pub(crate) fn disable_all(&self) -> Result<bool, InputCvarError> {
        Self::read_enable(&self.capture(159)?)
    }
    pub(crate) fn display_debug(&self) -> Result<bool, InputCvarError> {
        Self::read_enable(&self.capture(160)?)
    }
    pub(crate) fn capture_enable(
        &self,
        index: usize,
    ) -> Result<WipeoutEnableTarget, InputCvarError> {
        assert!(index < 30, "source WipeoutType index");
        self.capture(161 + index)
    }
}
