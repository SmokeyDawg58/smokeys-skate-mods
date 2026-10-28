//! Unregistered real references146..149, in native initializer order.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    simulation_assembly_backend::constraint_session::antiflip_nudge::NudgeSource,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum NudgeRuntimeError {
    Missing(&'static str),
    Poisoned,
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for NudgeRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "nudge missing {s}"),
            Self::Poisoned => f.write_str("nudge Processed lock poisoned"),
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
pub(crate) struct NudgeTarget(Target);
pub(crate) struct NudgeRuntime<'a> {
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
impl NudgeRuntime<'_> {
    fn target(&self, index: usize) -> Result<NudgeTarget, NudgeRuntimeError> {
        self.references
            .targets
            .lock()
            .map_err(|_| NudgeRuntimeError::Cvar(InputCvarError::Lock))?[index]
            .clone()
            .map(NudgeTarget)
            .ok_or(NudgeRuntimeError::Cvar(InputCvarError::Unlinked(
                DEFINITIONS[index].label,
            )))
    }
}
impl NudgeSource for NudgeRuntime<'_> {
    type Error = NudgeRuntimeError;
    type Target = NudgeTarget;
    fn minimum_frames(&mut self) -> Result<i32, Self::Error> {
        let t = self.target(149)?;
        let value =
            *t.0.lock()
                .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?;
        match value {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(Self::Error::Cvar(InputCvarError::WrongType)),
        }
    }
    fn deck_speed(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .deck_speed()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("deck speed968"))
    }
    fn maximum_speed(&mut self) -> Result<f32, Self::Error> {
        self.references
            .try_current_float(147)
            .map_err(Self::Error::Cvar)
    }
    fn deck_z(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .deck_transform()
            .map(|v| v.columns()[2])
            .ok_or(Self::Error::Missing("deck Z60"))
    }
    fn capture_strength(&mut self) -> Result<Self::Target, Self::Error> {
        self.target(148)
    }
    fn load_strength(&mut self, target: &Self::Target) -> Result<f32, Self::Error> {
        match *target
            .0
            .lock()
            .map_err(|_| Self::Error::Cvar(InputCvarError::Lock))?
        {
            Value::Float(v) => Ok(v),
            _ => Err(Self::Error::Cvar(InputCvarError::WrongType)),
        }
    }
    fn debug(&mut self) -> Result<bool, Self::Error> {
        self.references
            .try_current_boolean(146)
            .map_err(Self::Error::Cvar)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
