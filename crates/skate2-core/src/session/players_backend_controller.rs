//! Signed controller identity on the existing nine native backend rows.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained controller field awaits selected PlayerUI bootstrap composition"
    )
)]
use super::{PlayersBackendSettings, PlayersSettingsError, index};
use crate::player::PlayerLoaderSlot;

/// `ResetGame8273aab4` samples controller once before `PlayerUI` construction.
/// A later backend setter does not rewrite that UI's retained controller field.
pub(crate) struct BackendControllerSelection<'a> {
    pub(crate) loader: &'a PlayerLoaderSlot,
    controller: i32,
}
impl<'a> BackendControllerSelection<'a> {
    pub(crate) fn try_capture(
        settings: &PlayersBackendSettings,
        loader: &'a PlayerLoaderSlot,
    ) -> Result<Self, PlayersSettingsError> {
        Ok(Self {
            loader,
            controller: settings.try_controller_index(loader.player_index())?,
        })
    }
    /// Preserve raw signed word; the actual cache access rejects invalid indices.
    /// No FE negative-to-zero clamp is introduced at this different source stage.
    pub(crate) fn device_index(&self) -> usize {
        usize::try_from(u32::from_ne_bytes(self.controller.to_ne_bytes()))
            .expect("native32 controller word fits supported host")
    }
}
impl PlayersBackendSettings {
    /// `GetPlayerControllerIndex8255afe8`; unlike device lookup, no clamp to0..3.
    pub(crate) fn try_controller_index(&self, player: u32) -> Result<i32, PlayersSettingsError> {
        let index = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .controller_index)
    }
    /// `SetPlayerControllerIndex8255af88`; FE selection/clamping is its caller.
    pub(crate) fn try_set_controller_index(
        &self,
        player: u32,
        controller: i32,
    ) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        self.players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .controller_index = controller;
        Ok(())
    }
}
#[cfg(test)]
#[path = "players_backend_controller_tests.rs"]
mod tests;
