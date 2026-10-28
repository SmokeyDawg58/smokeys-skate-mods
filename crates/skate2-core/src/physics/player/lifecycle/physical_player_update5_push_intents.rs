//! Retained `HiLOD` push history and actual input reads82e060d0..61e8.
use super::PhysicalPlayerAssemblyOwner;
use crate::physical_player_phys_out::construct::update5_push_intents::{
    Update5PushIntentError, Update5PushIntentSource,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum PushIntentRuntimeError {
    Poisoned,
    Missing(&'static str),
}
impl std::fmt::Display for PushIntentRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("push intent owner poisoned"),
            Self::Missing(field) => write!(f, "push intent requires {field}"),
        }
    }
}
struct Source<'a>(&'a Arc<PhysicalPlayerAssemblyOwner>);
impl Update5PushIntentSource for Source<'_> {
    type Error = PushIntentRuntimeError;
    type Processed =
        Arc<std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>>;
    fn processed_flags_8c4(&mut self) -> Result<(u32, u32), Self::Error> {
        let processed = Arc::clone(
            self.0
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        );
        Ok(processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .known_flags_8c4())
    }
    fn regular_and_switch(&mut self) -> Result<(bool, bool), Self::Error> {
        let animation = Arc::clone(
            self.0
                .animation_physics_input
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .as_ref()
                .ok_or(Self::Error::Missing("AnimOutPhysIn2b4"))?,
        );
        let input = animation.lock().map_err(|_| Self::Error::Poisoned)?;
        Ok((input.flags.regular_stance, input.flags.riding_switch))
    }
    fn hilod_push_history(&mut self) -> Result<(bool, bool), Self::Error> {
        let prefix = self.0.prefix().lock().map_err(|_| Self::Error::Poisoned)?;
        let flags = prefix
            .as_ref()
            .ok_or(Self::Error::Missing("HiLOD prefix"))?
            .non_collision_flags;
        Ok((flags & 0x0040_0000 != 0, flags & 0x0080_0000 != 0))
    }
    fn capture_history_and_processed(&mut self) -> Result<(u32, Self::Processed), Self::Error> {
        let history = self
            .0
            .prefix()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .as_ref()
            .ok_or(Self::Error::Missing("HiLOD prefix"))?
            .non_collision_flags;
        let processed = Arc::clone(
            self.0
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        );
        Ok((history, processed))
    }
    fn store_push_history(&mut self, value: u32) -> Result<(), Self::Error> {
        let mut prefix = self.0.prefix().lock().map_err(|_| Self::Error::Poisoned)?;
        let flags = &mut prefix
            .as_mut()
            .ok_or(Self::Error::Missing("HiLOD prefix"))?
            .non_collision_flags;
        *flags = value;
        Ok(())
    }
    fn processed_flags_8c8(&mut self, processed: &Self::Processed) -> Result<u32, Self::Error> {
        processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .known_flags_8c8()
            .ok_or(Self::Error::Missing("Processed8c8"))
    }
}
pub(crate) fn try_publish_push_intents(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<(), Update5PushIntentError<PushIntentRuntimeError>> {
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| Update5PushIntentError::Source(PushIntentRuntimeError::Poisoned))?;
    output.try_publish_update5_push_intents(&mut Source(owner))
}
