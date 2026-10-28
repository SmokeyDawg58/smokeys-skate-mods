//! Live retained targets for `SkateboardBody::UpdatePostPhysics`.
//! Register as an `input_cvar_runtime` child after definitions154..156 exist.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::simulation_assembly_backend::constraint_session::board_post_services::SkateboardPostCvars;

pub(crate) struct BoardPostFloatTarget(Target);
pub(crate) struct BoardPostCvars<'a> {
    references: &'a InputCvarReferences,
}
impl<'a> BoardPostCvars<'a> {
    pub(crate) const fn new(references: &'a InputCvarReferences) -> Self {
        Self { references }
    }
    fn capture(&self, index: usize) -> Result<BoardPostFloatTarget, InputCvarError> {
        self.references
            .targets
            .lock()
            .map_err(|_| InputCvarError::Lock)?[index]
            .clone()
            .map(BoardPostFloatTarget)
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))
    }
}
fn read(target: &BoardPostFloatTarget) -> Result<f32, InputCvarError> {
    match *target.0.lock().map_err(|_| InputCvarError::Lock)? {
        Value::Float(value) => Ok(value),
        _ => Err(InputCvarError::WrongType),
    }
}
impl SkateboardPostCvars for BoardPostCvars<'_> {
    type Error = InputCvarError;
    type DeckTarget = BoardPostFloatTarget;
    type LineTarget = BoardPostFloatTarget;
    type FreeWheelTarget = BoardPostFloatTarget;
    fn capture_deck_end(&mut self) -> Result<Self::DeckTarget, Self::Error> {
        self.capture(154)
    }
    fn deck_end(&mut self, target: &Self::DeckTarget) -> Result<f32, Self::Error> {
        read(target)
    }
    fn capture_line_distance(&mut self) -> Result<Self::LineTarget, Self::Error> {
        self.capture(156)
    }
    fn line_distance(&mut self, target: &Self::LineTarget) -> Result<f32, Self::Error> {
        read(target)
    }
    fn capture_free_wheel_drag(&mut self) -> Result<Self::FreeWheelTarget, Self::Error> {
        self.capture(155)
    }
    fn free_wheel_drag(&mut self, target: &Self::FreeWheelTarget) -> Result<f32, Self::Error> {
        read(target)
    }
    fn forced_surface(&mut self) -> Result<u32, Self::Error> {
        self.references.try_forced_world_surface()
    }
}

#[cfg(test)]
mod tests {
    use super::{BoardPostCvars, InputCvarError, InputCvarReferences, SkateboardPostCvars};
    use crate::input_cvar_runtime::InputCvarManager;

    #[test]
    fn captures_retain_identity_but_values_stay_live_and_surface_is_late() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut cvars = BoardPostCvars::new(&refs);
        assert!(matches!(
            cvars.capture_deck_end(),
            Err(InputCvarError::Unlinked("kMinDPWithZToBeConsideredDeckEnd"))
        ));
        let mut old = InputCvarManager::new();
        old.try_link(&refs).unwrap();
        let deck = cvars.capture_deck_end().unwrap();
        let wheel = cvars.capture_free_wheel_drag().unwrap();
        let line = cvars.capture_line_distance().unwrap();
        assert_eq!(cvars.deck_end(&deck).unwrap().to_bits(), 0x3f6b_851f);
        assert_eq!(
            cvars.free_wheel_drag(&wheel).unwrap().to_bits(),
            0x3d23_d70a
        );
        assert_eq!(cvars.line_distance(&line).unwrap().to_bits(), 0x3d8f_5c29);
        old.try_deserialize_startup(Some(b"[Config]\r\nkMinDPWithZToBeConsideredDeckEnd=-2\r\nkFreeSkateboardWheelInertia=3\r\nkSkateboard_LineDistToUseNormal=4\r\nkForcedWorldSurface=-1\r\n")).unwrap();
        assert_eq!(cvars.forced_surface().unwrap(), u32::MAX);
        let mut replacement = InputCvarManager::new();
        replacement.try_link(&refs).unwrap();
        assert_eq!(
            cvars.deck_end(&deck).unwrap().to_bits(),
            (-2.0_f32).to_bits()
        );
        assert_eq!(
            cvars.free_wheel_drag(&wheel).unwrap().to_bits(),
            3.0_f32.to_bits()
        );
        assert_eq!(
            cvars.line_distance(&line).unwrap().to_bits(),
            4.0_f32.to_bits()
        );
        assert_eq!(cvars.forced_surface().unwrap(), 0);
        let new_deck = cvars.capture_deck_end().unwrap();
        assert_eq!(cvars.deck_end(&new_deck).unwrap().to_bits(), 0x3f6b_851f);
    }
}
