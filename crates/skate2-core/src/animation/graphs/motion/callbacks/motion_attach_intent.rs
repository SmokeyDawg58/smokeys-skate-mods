//! `AttachIntent` Update82cc9f30; Create82bb3da8 returns NULL, Begin/End are BLR.
use crate::play_animation::{FastString30, FastString36};

pub(crate) trait AttachIntentSource {
    type Error;
    fn resolve_animatable(&mut self) -> Result<(), Self::Error>;
    fn resolve_motion(&mut self) -> Result<(), Self::Error>;
    fn resolve_skater_motion(&mut self) -> Result<(), Self::Error>;
    fn require_interfaces(&mut self) -> Result<(), Self::Error>;
    fn intent(&mut self, name: &FastString36) -> Result<Option<f32>, Self::Error>;
    fn append_motion_attribute(
        &mut self,
        name: FastString30,
        value: f32,
    ) -> Result<(), Self::Error>;
    fn settable_attribute(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
}

pub(crate) fn update<S: AttachIntentSource>(
    source: &mut S,
    intent: &FastString36,
    attribute: FastString30,
    settable: bool,
) -> Result<(), S::Error> {
    source.resolve_animatable()?;
    source.resolve_motion()?;
    source.resolve_skater_motion()?;
    source.require_interfaces()?;
    if let Some(value) = source.intent(intent)? {
        source.append_motion_attribute(attribute, value)?;
        if settable {
            source.settable_attribute(attribute, value)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "motion_attach_intent_tests.rs"]
mod tests;
