//! Actual OBA cvar targets used by Air.GetComTrajectoryLaunchInfo82d84050.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(crate) enum ComLaunchFloat {
    BoardDelta,
    TrajectoryDelta,
    ConeScalar,
}
pub(crate) struct ComLaunchFloatTarget(Target);
impl ComLaunchFloatTarget {
    pub(crate) fn read(&self) -> Result<f32, InputCvarError> {
        match *self.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
impl InputCvarReferences {
    pub(crate) fn try_capture_com_launch_float(
        &self,
        field: ComLaunchFloat,
    ) -> Result<ComLaunchFloatTarget, InputCvarError> {
        let index = match field {
            ComLaunchFloat::BoardDelta => 248,
            ComLaunchFloat::TrajectoryDelta => 249,
            ComLaunchFloat::ConeScalar => 250,
        };
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(ComLaunchFloatTarget(Arc::clone(
            targets[index]
                .as_ref()
                .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?,
        )))
    }
    pub(crate) fn try_com_launch_count(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(251)
    }
}
