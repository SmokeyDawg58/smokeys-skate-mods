//! Retained `PlayerUI` focus fields and command prefix, not full UI admission.
use super::{CommandQueueError, SelectedSimCommandQueue};
use crate::game_input_manager::EncodedGameInputCommand;

pub(crate) struct SelectedPlayerFocus {
    player_index: u32,
    has_focus: bool,
}
impl SelectedPlayerFocus {
    /// `PlayerUICtor828ae0f0` stores the supplied index and `mHasFocus=false`.
    pub(crate) fn construct_fields(player_index: u32) -> Self {
        Self {
            player_index,
            has_focus: false,
        }
    }
    /// `PartyPlayUI::SetPlayer`828aa4c0 moves the same PlayerUI's `mPlayerIndex`.
    pub(crate) fn set_player_index(&mut self, player_index: u32) {
        self.player_index = player_index;
    }
    /// Update828af9f8 through828afa74; false means native no-simulation return.
    /// A true result still precedes session-marker/network-query collaborators,
    /// ordinary sampling, spawner/reset and proxy Tick. Caller admits those.
    pub(crate) fn try_update_prefix(
        &mut self,
        can_simulate: bool,
        has_focus: bool,
        queue: &mut SelectedSimCommandQueue,
    ) -> Result<bool, CommandQueueError> {
        if !can_simulate {
            if !has_focus && self.has_focus {
                self.has_focus = false;
                if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
                    eprintln!("PlayerUI {} focus false (paused)", self.player_index);
                }
            }
            return Ok(false);
        }
        if has_focus != self.has_focus {
            self.has_focus = has_focus;
            if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
                eprintln!("PlayerUI {} focus {has_focus}", self.player_index);
            }
            if has_focus {
                // SetData<PlayerInputGainFocusData>826c2ff0 writes no bytes.
                queue.try_queue(&EncodedGameInputCommand {
                    receiver_id: self.player_index.wrapping_add(10),
                    kind: 2,
                    payload: &[],
                })?;
            }
        }
        Ok(true)
    }
}
#[cfg(test)]
#[path = "selected_player_focus_tests.rs"]
mod tests;
