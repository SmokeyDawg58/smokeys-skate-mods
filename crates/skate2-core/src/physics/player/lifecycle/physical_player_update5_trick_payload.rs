//! Actual retained animation input to Update5 trick output, after Ground.Fill.
use crate::{
    actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::{
        construct::update5_trick_payload::{Update5TrickPayloadError, Update5TrickPayloadSource},
        trick::AnimOutTrickData,
    },
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum TrickPayloadRuntimeError {
    Poisoned,
    Missing(&'static str),
}
impl std::fmt::Display for TrickPayloadRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Update5 trick payload owner poisoned"),
            Self::Missing(field) => write!(f, "Update5 trick payload requires {field}"),
        }
    }
}
struct Source<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    captured: Option<Arc<Mutex<AnimOutPhysInOwner>>>,
}
impl Source<'_> {
    fn captured_animation<T>(
        &self,
        read: impl FnOnce(&AnimOutPhysInOwner) -> T,
    ) -> Result<T, TrickPayloadRuntimeError> {
        let input = self
            .captured
            .as_ref()
            .ok_or(TrickPayloadRuntimeError::Missing("captured AnimOutPhysIn"))?
            .lock()
            .map_err(|_| TrickPayloadRuntimeError::Poisoned)?;
        Ok(read(&input))
    }
    fn animation<T>(
        &self,
        read: impl FnOnce(&AnimOutPhysInOwner) -> T,
    ) -> Result<T, TrickPayloadRuntimeError> {
        let input = Arc::clone(
            self.owner
                .animation_physics_input
                .lock()
                .map_err(|_| TrickPayloadRuntimeError::Poisoned)?
                .as_ref()
                .ok_or(TrickPayloadRuntimeError::Missing("actual AnimOutPhysIn2b4"))?,
        );
        let input = input
            .lock()
            .map_err(|_| TrickPayloadRuntimeError::Poisoned)?;
        Ok(read(&input))
    }
}
impl Update5TrickPayloadSource for Source<'_> {
    type Error = TrickPayloadRuntimeError;
    fn trick(&mut self) -> Result<AnimOutTrickData, Self::Error> {
        self.animation(|a| a.trick.clone())
    }
    fn capture_animation(&mut self) -> Result<(), Self::Error> {
        self.captured = Some(Arc::clone(
            self.owner
                .animation_physics_input
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .as_ref()
                .ok_or(Self::Error::Missing("actual AnimOutPhysIn2b4"))?,
        ));
        Ok(())
    }
    fn gesture(&mut self) -> Result<u32, Self::Error> {
        self.captured_animation(|a| a.gesture)
    }
    fn gesturing(&mut self) -> Result<bool, Self::Error> {
        self.captured_animation(|a| a.gesturing)
    }
    fn wipeout_x(&mut self) -> Result<f32, Self::Error> {
        self.captured_animation(|a| a.wipeout_gesture[0])
    }
    fn wipeout_y(&mut self) -> Result<f32, Self::Error> {
        self.captured_animation(|a| a.wipeout_gesture[1])
    }
    fn flags_8d0(&mut self) -> Result<u32, Self::Error> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        );
        let processed = processed.lock().map_err(|_| Self::Error::Poisoned)?;
        processed
            .physics_state_fields()
            .flags_8d0
            .ok_or(Self::Error::Missing("Processed8d0"))
    }
}
/// Runs after `GrindManager::FillPhysOut` (or its explicit first-test
/// exclusion, which leaves the retained Grinds fields intact).
pub(crate) fn try_publish_update5_trick_payload(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<(), Update5TrickPayloadError<TrickPayloadRuntimeError>> {
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| Update5TrickPayloadError::Source(TrickPayloadRuntimeError::Poisoned))?;
    output.try_publish_update5_trick_payload(&mut Source {
        owner,
        captured: None,
    })
}
