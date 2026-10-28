//! Retained `JuiceRPCNG` trick globals: `DoTrick`826e3f40, `IsTrickActive826e39e8`,
//! `GetTrickName`826e3a38. Native mutex static constructor82f5cee0.
//! This owns actual mutable state; transport/RPC registration is separate.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained Juice owner awaits production boot and RPC transport; corpus/tests exercise actual state"
    )
)]
use crate::action_graph_input_listener::fill::ActionInputJuice;
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[path = "juice_trick_input_tests.rs"]
mod tests;

struct TrickState {
    name: [u8; 64],
    active: bool,
}
/// One runtime-owned occurrence of native static name8304e540 and active8304e606.
/// Clone shares both values and the SAME mutex; do not construct one per Actor.
#[derive(Clone)]
pub(crate) struct JuiceTrickInput {
    state: Arc<Mutex<TrickState>>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum JuiceTrickInputError {
    Poisoned,
    /// Native null %s argument after `GetString` has no recovered safe behavior.
    MissingString,
    /// Source copy/latch consumption completed; selected string-intent sink
    /// cannot yet represent arbitrary non-UTF8 C bytes. No lossy substitution.
    UnsupportedNameEncoding,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum JuiceTrickRequestError<E> {
    State(JuiceTrickInputError),
    Reader(E),
}
/// Successful `GetString` collaborator output, retaining its separate status and
/// pointer. `DoTrick` ignores the bool and consumes the resulting pointer.
pub(crate) struct JuiceInputString {
    pub(crate) succeeded: bool,
    pub(crate) bytes: Option<Vec<u8>>,
}
pub(crate) trait JuiceTrickReader {
    type Error;
    fn get_string(&mut self) -> Result<JuiceInputString, Self::Error>;
}

impl JuiceTrickInput {
    /// Source image .data contains64 zero name bytes and zero active byte.
    /// Static initializer82f5cee0 constructs the shared mutex (null params,true)
    /// and registers its destructor; it does not rewrite either trick global.
    pub(crate) fn new_static_initialized() -> Self {
        Self {
            state: Arc::new(Mutex::new(TrickState {
                name: [0; 64],
                active: false,
            })),
        }
    }
    /// `DoTrick`826e3f40 locks before `GetString`; snprintf64 precedes active=true.
    /// Reader/null failures retain the previous name and active latch.
    pub(crate) fn try_do_trick<R: JuiceTrickReader>(
        &self,
        reader: &mut R,
    ) -> Result<(), JuiceTrickRequestError<R::Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JuiceTrickRequestError::State(JuiceTrickInputError::Poisoned))?;
        let value = reader
            .get_string()
            .map_err(JuiceTrickRequestError::Reader)?;
        // 826e3f7c return status is not read by source; only pointer at3f88.
        let _ = value.succeeded;
        let bytes = value.bytes.ok_or(JuiceTrickRequestError::State(
            JuiceTrickInputError::MissingString,
        ))?;
        copy_name(&bytes, &mut state.name);
        state.active = true;
        Ok(())
    }
    /// Native `GetTrickName` called with the actual64-byte Fill destination.
    /// Copies through first C NUL, preserving destination bytes beyond it,
    /// then clears active while holding the SAME source mutex.
    pub(crate) fn try_get_trick_name(
        &self,
        output: &mut [u8; 64],
    ) -> Result<usize, JuiceTrickInputError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JuiceTrickInputError::Poisoned)?;
        let length = copy_name(&state.name, output);
        state.active = false;
        Ok(length)
    }
    pub(crate) fn try_is_trick_active(&self) -> Result<bool, JuiceTrickInputError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| JuiceTrickInputError::Poisoned)?
            .active)
    }
}
fn copy_name(input: &[u8], output: &mut [u8; 64]) -> usize {
    let length = input
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(input.len())
        .min(63);
    output[..length].copy_from_slice(&input[..length]);
    output[length] = 0;
    length
}
impl ActionInputJuice for JuiceTrickInput {
    type Error = JuiceTrickInputError;
    fn is_trick_active(&mut self) -> Result<bool, Self::Error> {
        self.try_is_trick_active()
    }
    fn take_trick_name(&mut self) -> Result<String, Self::Error> {
        let mut output = [0; 64];
        let length = self.try_get_trick_name(&mut output)?;
        String::from_utf8(output[..length].to_vec())
            .map_err(|_| JuiceTrickInputError::UnsupportedNameEncoding)
    }
}

#[path = "juice_input_boot.rs"]
pub(crate) mod boot;
