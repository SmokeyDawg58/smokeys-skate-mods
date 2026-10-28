//! Live target used by `SkateboardAnchorPoint::GetRawPosition82e31b08`.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use std::sync::Arc;

pub(crate) struct CameraStandingOffsetTarget(Target);
impl CameraStandingOffsetTarget {
    pub(crate) fn read(&self) -> Result<f32, InputCvarError> {
        match *self.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
impl InputCvarReferences {
    /// Capture precedes the two vector loads; reading the float follows them.
    pub(crate) fn try_capture_camera_standing_offset(
        &self,
    ) -> Result<CameraStandingOffsetTarget, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(CameraStandingOffsetTarget(Arc::clone(
            targets[295]
                .as_ref()
                .ok_or(InputCvarError::Unlinked(DEFINITIONS[295].label))?,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::InputCvarReferences;
    use crate::input_cvar_runtime::InputCvarManager;

    #[test]
    fn captured_offset_keeps_target_and_reads_live_value_after_relink() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let captured = refs.try_capture_camera_standing_offset().unwrap();
        assert_eq!(captured.read().unwrap().to_bits(), 0x3e99_999a);
        manager
            .try_deserialize_startup(Some(b"[Config]\r\nkStandingOffset=-0\r\n"))
            .unwrap();
        let mut replacement = InputCvarManager::new();
        replacement.try_link(&refs).unwrap();
        assert_eq!(captured.read().unwrap().to_bits(), (-0.0_f32).to_bits());
        assert_eq!(
            refs.try_capture_camera_standing_offset()
                .unwrap()
                .read()
                .unwrap()
                .to_bits(),
            0x3e99_999a
        );
    }
}
