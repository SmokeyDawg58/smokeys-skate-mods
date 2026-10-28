//! Live references consumed by the actual `GrindManager` prefix.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Value};
impl InputCvarReferences {
    pub(super) fn try_current_integer(&self, index: usize) -> Result<i32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[index]
            .as_ref()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    pub(crate) fn try_grind_detection_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(70)
    }
    pub(crate) fn try_debug_nearby_grinds(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(71)
    }
    pub(crate) fn try_enable_usual_grinds(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(72)
    }
    pub(crate) fn try_deck_flicker_score(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(73)
    }
    pub(crate) fn try_truck_flicker_score(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(74)
    }
    pub(crate) fn try_too_much_flicker_threshold(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(75)
    }
    pub(crate) fn try_state_flicker_avoid_frames(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(76)
    }
    pub(crate) fn try_grind_jump_air_frames(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(77)
    }
    pub(crate) fn try_state_changer_min_air_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(78)
    }
}

#[cfg(test)]
mod tests {
    use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};
    #[test]
    fn native_defaults_and_unclamped_live_targets() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert_eq!(
            refs.try_grind_detection_radius().unwrap().to_bits(),
            0x3f99_999a
        );
        assert!(!refs.try_debug_nearby_grinds().unwrap());
        assert!(refs.try_enable_usual_grinds().unwrap());
        assert_eq!(refs.try_deck_flicker_score().unwrap(), 50);
        assert_eq!(refs.try_truck_flicker_score().unwrap(), 20);
        assert_eq!(refs.try_too_much_flicker_threshold().unwrap(), 151);
        assert_eq!(refs.try_state_flicker_avoid_frames().unwrap(), 90);
        assert_eq!(refs.try_grind_jump_air_frames().unwrap(), 10);
        assert_eq!(
            refs.try_state_changer_min_air_time().unwrap().to_bits(),
            0x3da3_d70a
        );
        manager.try_deserialize_startup(Some(b"[Config]\r\nkGrindableDetectionRadius=-2\r\nkDeckFlickerScore=2147483647\r\nkEnableUsualGrinds=0\r\n")).unwrap();
        assert_eq!(
            refs.try_grind_detection_radius().unwrap().to_bits(),
            (-2.0_f32).to_bits()
        );
        assert_eq!(refs.try_deck_flicker_score().unwrap(), i32::MAX);
        assert!(!refs.try_enable_usual_grinds().unwrap());
    }
}
