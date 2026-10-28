//! Actual recognizer cvar references and source once-copied gesture references.
//! `UpdateInput826e18d8` captures deadzone; successful match lazily copies both
//! gesture refs once, including their then-current target pointers.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::input_pattern_matcher::{InputPatternCvar, InputPatternCvars, InputPatternStick};
use std::sync::Arc;

#[cfg(test)]
#[path = "input_recognizer_cvars_tests.rs"]
mod tests;

pub(crate) struct InputRecognizerCvars {
    references: InputCvarReferences,
}
pub(crate) struct InputRecognizerTarget(Target);
impl InputCvarReferences {
    pub(crate) fn recognizer_provider(&self) -> InputRecognizerCvars {
        InputRecognizerCvars {
            references: self.clone(),
        }
    }
    fn capture_recognizer_target(
        &self,
        index: usize,
    ) -> Result<InputRecognizerTarget, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(InputRecognizerTarget(Arc::clone(
            targets[index]
                .as_ref()
                .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?,
        )))
    }
}
impl InputRecognizerTarget {
    fn read(&self) -> Result<Value, InputCvarError> {
        Ok(*self.0.lock().map_err(|_| InputCvarError::Lock)?)
    }
}
fn stick_index(stick: InputPatternStick) -> usize {
    match stick {
        InputPatternStick::Left => 0,
        InputPatternStick::Right => 1,
    }
}
impl InputRecognizerCvars {
    pub(crate) fn capture_deadzone(&mut self) -> Result<InputRecognizerTarget, InputCvarError> {
        self.references.capture_recognizer_target(11)
    }
    #[expect(
        clippy::unused_self,
        reason = "paired capture/read adapter matches GameInputRecognizerServices contract"
    )]
    pub(crate) fn deadzone(
        &mut self,
        target: &InputRecognizerTarget,
    ) -> Result<[f32; 2], InputCvarError> {
        match target.read()? {
            Value::Vector2(value) => Ok([value[0], value[1]]),
            _ => Err(InputCvarError::WrongType),
        }
    }
    /// Source copies reference targets only once, shared across all players.
    /// A copied NULL remains NULL even if original reference is later linked.
    pub(crate) fn ensure_gesture_display_references(&mut self) -> Result<(), InputCvarError> {
        let mut copies = self
            .references
            .gesture_copies
            .lock()
            .map_err(|_| InputCvarError::Lock)?;
        if copies.is_none() {
            let targets = self
                .references
                .targets
                .lock()
                .map_err(|_| InputCvarError::Lock)?;
            *copies = Some([targets[9].clone(), targets[10].clone()]);
        }
        Ok(())
    }
    pub(crate) fn gesture_display_value(
        &mut self,
        stick: InputPatternStick,
    ) -> Result<bool, InputCvarError> {
        let index = stick_index(stick);
        let copies = self
            .references
            .gesture_copies
            .lock()
            .map_err(|_| InputCvarError::Lock)?;
        let references = copies
            .as_ref()
            .ok_or(InputCvarError::Uncaptured(DEFINITIONS[9 + index].label))?;
        let target = references[index]
            .as_ref()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[9 + index].label))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    pub(crate) fn capture_input_display(
        &mut self,
        stick: InputPatternStick,
    ) -> Result<InputRecognizerTarget, InputCvarError> {
        self.references
            .capture_recognizer_target(7 + stick_index(stick))
    }
    #[expect(
        clippy::unused_self,
        reason = "paired capture/read adapter matches GameInputRecognizerServices contract"
    )]
    pub(crate) fn input_display_value(
        &mut self,
        target: &InputRecognizerTarget,
    ) -> Result<bool, InputCvarError> {
        match target.read()? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
impl InputPatternCvars for InputRecognizerCvars {
    type Target = InputRecognizerTarget;
    type Error = InputCvarError;
    fn capture(&mut self, key: InputPatternCvar) -> Result<Self::Target, Self::Error> {
        let index = match key {
            InputPatternCvar::FastestRatio => 12,
            InputPatternCvar::SlowestRatio => 13,
            InputPatternCvar::MinError => 14,
            InputPatternCvar::MaxError => 15,
        };
        self.references.capture_recognizer_target(index)
    }
    fn value(&mut self, target: &Self::Target) -> Result<f32, Self::Error> {
        match target.read()? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
