//! Selected `PlayerInput` embedded pad826e2440 and `GameInputSK8` receiver826df1c8.
//! Matcher/history allocation and actual `CommandManager` scheduling are separate.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "gameplay input requires the actual command manager and Actor owner"
    )
)]
use crate::{
    controller_input::{ControllerControl, TryMappedControllerInput},
    input_map::{InputMapReadError, SelectedInputMap},
    input_pad::{InputPad, InputPadError},
    input_pad_cache::PublishedInputPadSample,
};
use std::{fmt, sync::Arc};

/// Typed result of successful native `PlayerInputData` unpacking. No byte-stream
/// serialization or missing-payload defaults are implemented by this boundary.
#[derive(Clone, Copy)]
pub(crate) struct PlayerInputData {
    pub(crate) sample: PublishedInputPadSample,
    pub(crate) allow_sensor: bool,
    pub(crate) has_focus: bool,
}
#[derive(Clone, Copy)]
pub(crate) enum GameInputCommand {
    Input(PlayerInputData),
    GainFocus,
    Unknown(u32),
}
/// Exact receiver occurrence, retained by the registration collaborator.
#[derive(Clone, Debug)]
pub(crate) struct GameInputReceiver {
    occurrence: Arc<()>,
    id: u32,
}
impl GameInputReceiver {
    pub(crate) const fn id(&self) -> u32 {
        self.id
    }
}
/// Source registration826df294 must succeed before this selected component is
/// usable. This port does not implement the simulation command manager.
pub(crate) trait GameInputReceiverRegistration {
    type Error;
    fn register(&mut self, receiver: &GameInputReceiver) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GameInputPlayerError {
    ConstructionRequired,
    ConstructionAlreadyAttempted,
    ForeignReceiver,
    UnknownCommand(u32),
    MissingPayload(u32),
    Pad(InputPadError),
    Map(InputMapReadError<InputPadError>),
}
#[derive(Debug)]
pub(crate) enum GameInputConstructionError<E> {
    State(GameInputPlayerError),
    Registration(E),
}
impl fmt::Display for GameInputPlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "game input: {self:?}")
    }
}
impl<E: fmt::Debug> fmt::Display for GameInputConstructionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "game input construction: {self:?}")
    }
}
struct ReceiverFields {
    globals: [f32; 4],
    dpd_position_updated: bool,
    has_focus: bool,
    bounced: [bool; 4],
}
#[path = "recognition/game_input_recognizer.rs"]
pub(crate) mod recognizer;

/// Selected component storage, not a complete native `PlayerInput` constructor.
/// Retained histories and matchers have an explicit selected construction stage.
pub(crate) struct GameInputPlayer {
    pad: InputPad,
    map: Arc<SelectedInputMap>,
    receiver: GameInputReceiver,
    fields: Option<ReceiverFields>,
    attempted: bool,
    registered: bool,
    recognizer: Option<recognizer::GameInputRecognizer>,
    // PlayerInput+1b4, initialized before either history allocation.
    native_flags: u32,
}
impl GameInputPlayer {
    pub(crate) const fn selected_construction_complete(&self) -> bool {
        self.registered
    }
    pub(crate) fn player_index(&self) -> u32 {
        self.receiver.id.wrapping_sub(10)
    }
    /// Embedded pad constructor precedes the later `GameInputSK8` constructor.
    /// The supplied map must be the retained actual SIM input map occurrence.
    pub(crate) fn new(index: u32, map: Arc<SelectedInputMap>) -> Self {
        Self {
            pad: InputPad::new(),
            map,
            receiver: GameInputReceiver {
                occurrence: Arc::new(()),
                id: index.wrapping_add(10),
            },
            fields: None,
            attempted: false,
            registered: false,
            recognizer: None,
            native_flags: if index == 8 { 2 } else { 1 },
        }
    }
    /// `GameInputSK8` constructor fields precede actual receiver registration.
    /// A refused registration retains those fields and prevents constructor retry.
    pub(crate) fn try_construct<R: GameInputReceiverRegistration>(
        &mut self,
        registration: &mut R,
    ) -> Result<GameInputReceiver, GameInputConstructionError<R::Error>> {
        if self.attempted {
            return Err(GameInputConstructionError::State(
                GameInputPlayerError::ConstructionAlreadyAttempted,
            ));
        }
        self.attempted = true;
        self.fields = Some(ReceiverFields {
            globals: [0.0; 4],
            dpd_position_updated: true,
            has_focus: false,
            bounced: [false; 4],
        });
        registration
            .register(&self.receiver)
            .map_err(GameInputConstructionError::Registration)?;
        self.registered = true;
        Ok(self.receiver.clone())
    }
    fn require_registered(&self) -> Result<(), GameInputPlayerError> {
        if self.registered {
            Ok(())
        } else {
            Err(GameInputPlayerError::ConstructionRequired)
        }
    }
    /// Update826e2578 reads the actual input/pad and its count BEFORE `mFlags`.
    pub(crate) fn try_native_update_eligible(
        &self,
        mask: u32,
    ) -> Result<bool, GameInputPlayerError> {
        self.require_registered()?;
        if self.pad.num_buttons() == 0 {
            return Ok(false);
        }
        Ok(self.native_flags & mask != 0)
    }
    /// `PlayerInput` destructor826e12d0: left history/matchers, then right, then
    /// the receiver destructor's actual command-manager unregister callback.
    pub(crate) fn begin_native_destruction(&mut self) -> GameInputReceiver {
        self.recognizer = None;
        self.receiver.clone()
    }
    pub(crate) fn finish_native_destruction(&mut self) {
        // Retained safe-host aliases must not operate the deleted native owner.
        self.registered = false;
    }
    /// `OnCommand`826df568, after successful typed payload decoding. No focus
    /// gate is added around pad update and `allow_sensor` is source-unused here.
    pub(crate) fn try_on_command(
        &mut self,
        receiver: &GameInputReceiver,
        command: GameInputCommand,
    ) -> Result<(), GameInputPlayerError> {
        self.require_registered()?;
        if !Arc::ptr_eq(&self.receiver.occurrence, &receiver.occurrence) {
            return Err(GameInputPlayerError::ForeignReceiver);
        }
        match command {
            GameInputCommand::Input(mut data) => {
                let _ = data.allow_sensor;
                let fields = self.fields.as_mut().expect("constructed receiver");
                fields.has_focus = data.has_focus;
                // Native receiver ignores allow_sensor and cache-line count.
                // The four face-channel gates are independent and ordered12..15.
                for (gate, value) in fields
                    .bounced
                    .iter_mut()
                    .zip(&mut data.sample.values[12..16])
                {
                    if !*gate {
                        *gate = *value == 0.0;
                    }
                    if !*gate {
                        *value = 0.0;
                    }
                }
                self.pad
                    .try_update(&data.sample.values, 24)
                    .map_err(GameInputPlayerError::Pad)?;
            }
            GameInputCommand::GainFocus => {
                let fields = self.fields.as_mut().expect("constructed receiver");
                fields.bounced.fill(false);
                fields.has_focus = true;
            }
            GameInputCommand::Unknown(kind) => {
                return Err(if kind == 1 || kind == 2 {
                    GameInputPlayerError::MissingPayload(kind)
                } else {
                    GameInputPlayerError::UnknownCommand(kind)
                });
            }
        }
        Ok(())
    }
    pub(crate) fn pad(&self) -> Result<&InputPad, GameInputPlayerError> {
        self.require_registered()?;
        Ok(&self.pad)
    }
    pub(crate) fn globals(&self) -> Result<&[f32; 4], GameInputPlayerError> {
        self.require_registered()?;
        Ok(&self.fields.as_ref().expect("constructed receiver").globals)
    }
    pub(crate) fn dpd_position_updated(&self) -> Result<bool, GameInputPlayerError> {
        self.require_registered()?;
        Ok(self
            .fields
            .as_ref()
            .expect("constructed receiver")
            .dpd_position_updated)
    }
    pub(crate) fn has_focus(&self) -> Result<bool, GameInputPlayerError> {
        self.require_registered()?;
        Ok(self
            .fields
            .as_ref()
            .expect("constructed receiver")
            .has_focus)
    }
    /// Existing selected grammar uses no globals; unsupported global expressions
    /// already refuse in the retained map parser instead of substituting values.
    pub(crate) fn try_value(&mut self, id: u32) -> Result<f32, GameInputPlayerError> {
        self.require_registered()?;
        self.map
            .try_value(id, &mut self.pad)
            .map_err(GameInputPlayerError::Map)
    }
}
impl TryMappedControllerInput for GameInputPlayer {
    type Error = GameInputPlayerError;
    fn try_mapped_value(&mut self, control: ControllerControl) -> Result<f32, Self::Error> {
        self.try_value(u32::from(control as u8))
    }
}
#[cfg(test)]
#[path = "game_input_player_tests.rs"]
mod tests;
