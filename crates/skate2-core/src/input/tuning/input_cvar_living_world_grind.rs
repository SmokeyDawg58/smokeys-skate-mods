//! Live refs for the selected `LivingWorld` dynamic-grind provider.
use super::{InputCvarError, InputCvarReferences};
impl InputCvarReferences {
    /// Ref830a7634, init82f8c160/table82fd5494.
    pub(crate) fn try_feed_dmo_grab_splines(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(855)
    }
    /// Ref830a6df8, init82f8b5a8/table82fd53ec; native spelling preserved.
    pub(crate) fn try_max_vehicle_grind_object_size(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(67)
    }
    /// Ref830a7284, init82f8c1b0/table82fd5498.
    pub(crate) fn try_feed_dmo_grind_splines(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(68)
    }
    /// Ref830a75b8, init82f8cff8/table82fd5550.
    pub(crate) fn try_max_dmo_grind_object_size(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(69)
    }
}

#[cfg(test)]
mod tests {
    use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};
    #[test]
    fn original_defaults_and_live_unclamped_relink() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        assert!(refs.try_feed_dmo_grind_splines().unwrap());
        assert_eq!(
            refs.try_max_dmo_grind_object_size().unwrap().to_bits(),
            (15.0_f32).to_bits()
        );
        assert_eq!(
            refs.try_max_vehicle_grind_object_size().unwrap().to_bits(),
            (15.0_f32).to_bits()
        );
        manager.try_deserialize_startup(Some(b"[Config]\r\nkFeedDMOGrindSplines=0\r\nkMaxDynamicObjectSize=-7\r\nkMaxVechileObjectSize=123\r\n")).unwrap();
        assert!(!refs.try_feed_dmo_grind_splines().unwrap());
        assert_eq!(
            refs.try_max_dmo_grind_object_size().unwrap().to_bits(),
            (-7.0_f32).to_bits()
        );
        assert_eq!(
            refs.try_max_vehicle_grind_object_size().unwrap().to_bits(),
            (123.0_f32).to_bits()
        );
        let mut replacement = InputCvarManager::new();
        replacement.try_link(&refs).unwrap();
        assert!(refs.try_feed_dmo_grind_splines().unwrap());
        assert_eq!(
            refs.try_max_vehicle_grind_object_size().unwrap().to_bits(),
            (15.0_f32).to_bits()
        );
    }
}
impl super::InputCvarReferences {
    pub(crate) fn try_draw_offboard_grab(&self) -> Result<bool, super::InputCvarError> {
        self.try_current_boolean(859)
    }
    pub(crate) fn try_grab_line_test_count(&self) -> Result<i32, super::InputCvarError> {
        self.try_current_integer(856)
    }
    pub(crate) fn try_grab_line_test_radius(&self) -> Result<f32, super::InputCvarError> {
        self.try_current_float(857)
    }
    pub(crate) fn try_grab_line_test_vertical_offset(&self) -> Result<f32, super::InputCvarError> {
        self.try_current_float(858)
    }
}
