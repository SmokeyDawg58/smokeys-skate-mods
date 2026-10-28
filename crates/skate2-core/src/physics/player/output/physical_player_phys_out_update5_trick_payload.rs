//! Actual Update5 output reads/copies82e05b78..5ccc after current-state Fill.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut, trick::AnimOutTrickData};
use super::prior_input::PriorInputReadError;

pub(crate) trait Update5TrickPayloadSource {
    type Error;
    fn trick(&mut self) -> Result<AnimOutTrickData, Self::Error>;
    fn capture_animation(&mut self) -> Result<(), Self::Error>;
    fn gesture(&mut self) -> Result<u32, Self::Error>;
    fn gesturing(&mut self) -> Result<bool, Self::Error>;
    fn wipeout_x(&mut self) -> Result<f32, Self::Error>;
    fn wipeout_y(&mut self) -> Result<f32, Self::Error>;
    fn flags_8d0(&mut self) -> Result<u32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum Update5TrickPayloadError<E> {
    Source(E),
    Output(PriorInputReadError),
}
impl<E: std::fmt::Debug> std::fmt::Display for Update5TrickPayloadError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "Update5 trick payload source: {e:?}"),
            Self::Output(e) => write!(f, "Update5 trick payload output: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn trick_payload_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_publish_update5_trick_payload<S: Update5TrickPayloadSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), Update5TrickPayloadError<S::Error>> {
        use Update5TrickPayloadError as E;
        self.trick_payload_guard(PhysOutComponent::Grinds)
            .map_err(E::Output)?;
        let substate = self.concrete.grinds.as_ref().unwrap().grind_substate;
        let grinding = matches!(substate, 1 | 2);
        self.trick_payload_guard(PhysOutComponent::Grinds)
            .map_err(E::Output)?;
        self.concrete.grinds.as_mut().unwrap().is_grinding = grinding;
        self.trick_payload_guard(PhysOutComponent::Trick)
            .map_err(E::Output)?;
        // Same typed payload as native operator=825f6f80: thirteen words,
        // followed by two floats. Source and destination are distinct owners.
        self.concrete.trick.as_mut().unwrap().current_animation =
            source.trick().map_err(E::Source)?;
        self.trick_payload_guard(PhysOutComponent::Trick)
            .map_err(E::Output)?;
        source.capture_animation().map_err(E::Source)?;
        let gesture = source.gesture().map_err(E::Source)?;
        self.concrete.trick.as_mut().unwrap().gesture = i32::from_ne_bytes(gesture.to_ne_bytes());
        self.concrete.trick.as_mut().unwrap().is_gesturing =
            source.gesturing().map_err(E::Source)?;
        self.trick_payload_guard(PhysOutComponent::Trick)
            .map_err(E::Output)?;
        source.capture_animation().map_err(E::Source)?;
        self.concrete.trick.as_mut().unwrap().wipeout_gesture_x =
            source.wipeout_x().map_err(E::Source)?;
        self.concrete.trick.as_mut().unwrap().wipeout_gesture_y =
            source.wipeout_y().map_err(E::Source)?;
        let flags = source.flags_8d0().map_err(E::Source)?;
        self.trick_payload_guard(PhysOutComponent::Trick)
            .map_err(E::Output)?;
        self.concrete.trick.as_mut().unwrap().is_push_tricking = flags & 0x0000_4000 != 0;
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_update5_trick_payload_tests.rs"]
mod tests;
