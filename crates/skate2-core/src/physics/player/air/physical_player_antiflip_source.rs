//! same-Processed/feet/overlay reads for actual `AntiFlipTorque`.
use crate::{
    attribute_runtime::AttributeRuntimeError,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    skateboard_foot_attributes::SkateboardFootVaultAttributes,
    skateboard_force_runtime::antiflip::AntiFlipSource,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum AntiFlipRuntimeError {
    Missing(&'static str),
    Poisoned,
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for AntiFlipRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "anti-flip missing {s}"),
            Self::Poisoned => f.write_str("anti-flip Processed lock poisoned"),
            Self::Attribute(e) => e.fmt(f),
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
pub(crate) struct AntiFlipRuntime<'a> {
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) feet: &'a SkateboardFootVaultAttributes,
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
impl AntiFlipRuntime<'_> {
    fn read<T>(
        &self,
        name: &'static str,
        get: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, AntiFlipRuntimeError> {
        let p = self
            .processed
            .lock()
            .map_err(|_| AntiFlipRuntimeError::Poisoned)?;
        get(&p).ok_or(AntiFlipRuntimeError::Missing(name))
    }
}
impl AntiFlipSource for AntiFlipRuntime<'_> {
    type Error = AntiFlipRuntimeError;
    fn manual_flag(&mut self) -> Result<bool, Self::Error> {
        self.read("manual8c4/20000000", |p| {
            let (bits, known) = p.known_flags_8c4();
            (known & 0x2000_0000 != 0).then_some(bits & 0x2000_0000 != 0)
        })
    }
    fn balance(&mut self) -> Result<f32, Self::Error> {
        self.read("balance9a0", |p| p.physics_state_fields().balance)
    }
    fn deck_z(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("deck Z60", |p| p.deck_transform().map(|m| m.columns()[2]))
    }
    fn up(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("up220", |p| {
            p.prior_output().reckoning_up().map(|v| v.value)
        })
    }
    fn deck_x(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("deck X40", |p| p.deck_transform().map(|m| m.columns()[0]))
    }
    fn feet_graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error> {
        self.feet
            .try_antiflip_graph(offset, input)
            .map_err(Self::Error::Attribute)
    }
    fn draw(&mut self) -> Result<bool, Self::Error> {
        self.references
            .try_draw_antiflip_torque()
            .map_err(Self::Error::Cvar)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
