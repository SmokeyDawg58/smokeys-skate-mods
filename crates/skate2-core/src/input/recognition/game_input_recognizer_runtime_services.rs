//! Live collaborators for `PlayerInput::UpdateRecognizer826e2008`.
//! The local-index query remains an explicit late dependency on `SkaterManager`.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "live recognizer services are exercised by the actual-manager corpus; Simulation scheduling is pending"
    )
)]
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    game_input_player::recognizer::GameInputRecognizerServices,
    input_cvar_runtime::{
        InputCvarError, InputCvarReferences,
        recognizer::{InputRecognizerCvars, InputRecognizerTarget},
    },
    input_pattern_matcher::{InputPatternCvar, InputPatternCvars, InputPatternStick},
};
use std::sync::Arc;

/// Must query the retained Simulation -> `ISkaterManager` collaborator. No default
/// index is supplied: `SetLocalIndex82600380` can remap the first player to8.
pub(crate) trait GameInputLocalPlayerIndex {
    fn try_local_player_index(&mut self) -> Result<u32, GameInputRuntimeServicesError>;
}

#[derive(Debug)]
pub(crate) enum GameInputRuntimeServicesError {
    Cvar(InputCvarError),
    Vault(AttributeRuntimeError),
    Overlay(DebugOverlayError),
    MissingLocalPlayerIndex,
}
impl std::fmt::Display for GameInputRuntimeServicesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(e) => write!(f, "recognizer cvar: {e:?}"),
            Self::Vault(e) => write!(f, "recognizer debug attributes: {e:?}"),
            Self::Overlay(e) => write!(f, "recognizer overlay: {e}"),
            Self::MissingLocalPlayerIndex => {
                f.write_str("gesture display requires the actual SkaterManager local-index owner")
            }
        }
    }
}

pub(crate) struct GameInputRuntimeServices<'a> {
    cvars: InputRecognizerCvars,
    vault: &'a Arc<AttributeRuntimeVault>,
    overlay: &'a DebugOverlaySlot,
    local_index: Option<&'a mut dyn GameInputLocalPlayerIndex>,
}
impl<'a> GameInputRuntimeServices<'a> {
    pub(crate) fn new(
        cvars: &InputCvarReferences,
        vault: &'a Arc<AttributeRuntimeVault>,
        overlay: &'a DebugOverlaySlot,
        local_index: Option<&'a mut dyn GameInputLocalPlayerIndex>,
    ) -> Self {
        Self {
            cvars: cvars.recognizer_provider(),
            vault,
            overlay,
            local_index,
        }
    }
}
impl InputPatternCvars for GameInputRuntimeServices<'_> {
    type Target = InputRecognizerTarget;
    type Error = GameInputRuntimeServicesError;
    fn capture(&mut self, key: InputPatternCvar) -> Result<Self::Target, Self::Error> {
        self.cvars.capture(key).map_err(Self::Error::Cvar)
    }
    fn value(&mut self, target: &Self::Target) -> Result<f32, Self::Error> {
        self.cvars.value(target).map_err(Self::Error::Cvar)
    }
}
impl GameInputRecognizerServices for GameInputRuntimeServices<'_> {
    type DeadzoneTarget = InputRecognizerTarget;
    type DisplayTarget = InputRecognizerTarget;
    type DebugInstance = AttributeRuntimeInstance;
    fn capture_deadzone(&mut self) -> Result<Self::DeadzoneTarget, Self::Error> {
        self.cvars.capture_deadzone().map_err(Self::Error::Cvar)
    }
    fn deadzone(&mut self, target: &Self::DeadzoneTarget) -> Result<[f32; 2], Self::Error> {
        self.cvars.deadzone(target).map_err(Self::Error::Cvar)
    }
    fn ensure_gesture_display_references(&mut self) -> Result<(), Self::Error> {
        self.cvars
            .ensure_gesture_display_references()
            .map_err(Self::Error::Cvar)
    }
    fn gesture_display_value(&mut self, stick: InputPatternStick) -> Result<bool, Self::Error> {
        self.cvars
            .gesture_display_value(stick)
            .map_err(Self::Error::Cvar)
    }
    fn local_player_index(&mut self) -> Result<u32, Self::Error> {
        self.local_index
            .as_deref_mut()
            .ok_or(Self::Error::MissingLocalPlayerIndex)?
            .try_local_player_index()
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
    fn capture_debug_instance(
        &mut self,
        stick: InputPatternStick,
    ) -> Result<Self::DebugInstance, Self::Error> {
        Ok(self
            .vault
            .instance(0x7f08_6875_d621_c88c, stick.collection()))
    }
    fn debug_lookup(
        &mut self,
        instance: &Self::DebugInstance,
        key: u64,
        _default_bytes: usize,
    ) -> Result<(), Self::Error> {
        // The selected executable discards these returned pointers. Missing data
        // selects the generated fallback; it does not read a synthesized scalar.
        instance
            .try_get_data(key, 0)
            .map(|_| ())
            .map_err(Self::Error::Vault)
    }
    fn release_debug_instance(&mut self, instance: Self::DebugInstance) -> Result<(), Self::Error> {
        drop(instance);
        Ok(())
    }
    fn capture_input_display(
        &mut self,
        stick: InputPatternStick,
    ) -> Result<Self::DisplayTarget, Self::Error> {
        self.cvars
            .capture_input_display(stick)
            .map_err(Self::Error::Cvar)
    }
    fn input_display_value(&mut self, target: &Self::DisplayTarget) -> Result<bool, Self::Error> {
        self.cvars
            .input_display_value(target)
            .map_err(Self::Error::Cvar)
    }
}
