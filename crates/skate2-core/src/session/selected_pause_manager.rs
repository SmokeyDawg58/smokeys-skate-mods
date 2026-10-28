//! Retained native `PauseMan` state; no window-focus or FE policy is implied.
use std::sync::{Arc, Mutex};

// kFlagsTable820d8548, audited indices0..25.
const FLAGS: [u32; 26] = [
    0,
    0xffff_fffd,
    0xffff_ffe5,
    0xffff_ffe5,
    0xffff_ffec,
    0xffff_ffe1,
    0xffff_ffe5,
    0xffff_ffe1,
    0xffff_ffe5,
    0xffff_ffe5,
    0xffff_fff5,
    0xffff_fff1,
    0xffff_fff1,
    0xffff_ffe5,
    0xffff_fff5,
    0xffff_fff5,
    0xffff_fff5,
    0xffff_fff5,
    0xffff_ffe5,
    0xffff_ffe5,
    0xffff_fffd,
    0,
    0xffff_fff5,
    0xffff_fff5,
    0xffff_fff5,
    0xffff_fff5,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseError {
    Lock,
    UnsupportedState(u32),
    StackCapacity,
}
impl std::fmt::Display for PauseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native pause state: {self:?}")
    }
}
impl std::error::Error for PauseError {}
struct Fields {
    stack: Vec<u32>,
    flags: u32,
    mask: u32,
}
/// One shared native stack, flags and mask, synchronized across all consumers.
pub struct SelectedPauseManager {
    fields: Mutex<Fields>,
}
impl SelectedPauseManager {
    /// Constructor82738d68: empty54-entry storage, flags0, maskFFFFFFFF,
    /// then Push(INIT=0). INIT's actual table value is0; SIM is active.
    #[must_use]
    pub fn construct() -> Arc<Self> {
        let mut stack = Vec::with_capacity(54);
        stack.push(0);
        Arc::new(Self {
            fields: Mutex::new(Fields {
                stack,
                flags: FLAGS[0],
                mask: u32::MAX,
            }),
        })
    }
    /// Push827381c8: append then replace flags with the last state's table entry.
    /// # Errors
    /// Refuses unknown table indices, poisoned locks, or native capacity assertion.
    pub fn try_push(&self, state: u32) -> Result<(), PauseError> {
        let flags = *FLAGS
            .get(usize::try_from(state).map_err(|_| PauseError::UnsupportedState(state))?)
            .ok_or(PauseError::UnsupportedState(state))?;
        let mut fields = self.fields.lock().map_err(|_| PauseError::Lock)?;
        if fields.stack.len() >= 53 {
            return Err(PauseError::StackCapacity);
        }
        fields.stack.push(state);
        fields.flags = flags;
        Ok(())
    }
    /// Pull82733df8: remove every matching entry, preserve survivor order.
    /// # Errors
    /// Refuses a poisoned owner lock. Absent states leave the stack unchanged.
    pub fn try_pull(&self, state: u32) -> Result<(), PauseError> {
        let mut fields = self.fields.lock().map_err(|_| PauseError::Lock)?;
        fields.stack.retain(|entry| *entry != state);
        fields.flags = fields
            .stack
            .last()
            .map_or(0, |index| FLAGS[*index as usize]);
        Ok(())
    }
    /// `Net::ResumeGame` (`0x82791ca8`) clears `kPAUSE_FLAG_SIM`, `kPAUSE_FLAG_CHALLENGE` and
    /// `kPAUSE_FLAG_DATASYSRESTORE` from the mask while `STATE_OnlineGame` is active, so no pause
    /// state (the front end's included) stops an online game's simulation; `Net::PauseGame`
    /// (`0x82791a88`) sets them again.
    /// # Errors
    /// Refuses a poisoned owner lock.
    pub fn try_set_network_resumed(&self, resumed: bool) -> Result<(), PauseError> {
        const NETWORK_FLAGS: u32 = 1 | 4 | 0x10;
        let mut fields = self.fields.lock().map_err(|_| PauseError::Lock)?;
        if resumed {
            fields.mask &= !NETWORK_FLAGS;
        } else {
            fields.mask |= NETWORK_FLAGS;
        }
        Ok(())
    }

    /// CheckForPauseState826477b0 admits INIT0 and MANUALCAM6 owners.
    pub(crate) fn try_can_open_frontend(&self) -> Result<bool, PauseError> {
        let fields = self.fields.lock().map_err(|_| PauseError::Lock)?;
        Ok(matches!(fields.stack.last(), Some(0 | 6)))
    }

    /// `IsActive82603b30` reads mask then flags under the same owner lock.
    /// # Errors
    /// Refuses a poisoned owner lock.
    pub fn try_is_active(&self, flag: u32) -> Result<bool, PauseError> {
        let fields = self.fields.lock().map_err(|_| PauseError::Lock)?;
        Ok((fields.mask & fields.flags & flag) == 0)
    }

    /// `IsPaused` (`0x824e4778`): the flag is set in the top state's table entry and the mask.
    /// # Errors
    /// Refuses a poisoned owner lock.
    pub fn try_is_paused(&self, flag: u32) -> Result<bool, PauseError> {
        let fields = self.fields.lock().map_err(|_| PauseError::Lock)?;
        Ok((fields.mask & fields.flags & flag) != 0)
    }
}
impl crate::input_pad::device_manager::selected_runtime::player_sample::PlayerInputPause
    for Arc<SelectedPauseManager>
{
    type Error = PauseError;
    fn try_is_simulation_active(&mut self) -> Result<bool, Self::Error> {
        self.try_is_active(1)
    }
}
#[cfg(test)]
#[path = "selected_pause_manager_tests.rs"]
mod tests;
