//! Selected `PlayersData` fields and separate `PlayersBackEnd` recipe array.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained selection awaits mounted bootstrap and animation consumer admission"
    )
)]
use super::{PlayersBackendSettings, PlayersSettingsError, index};

#[derive(Clone, Copy)]
pub(super) struct SelectionFields {
    pub(super) regular: u8,
    male: bool,
    style: u32,
    gestures: [u32; 4],
    pub(super) hat: u32,
    pub(super) player_type: u32,
}
impl SelectionFields {
    /// `PlayersData`.Reset82561ed4..ee4; GestureSet.Reset82561d70.
    pub(super) const fn reset() -> Self {
        Self {
            regular: 0,
            male: true,
            style: 0,
            gestures: [0, 1, 2, 3],
            hat: 0,
            player_type: 0,
        }
    }
}
impl PlayersBackendSettings {
    /// SetPlayerGestures8255b720, four native enum words in direction order.
    pub(crate) fn try_set_gestures(
        &self,
        player: u32,
        gestures: [u32; 4],
    ) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        self.players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .gestures = gestures;
        Ok(())
    }
    /// SetPlayerCharacterIsMale8255b120.
    pub(crate) fn try_set_character_is_male(
        &self,
        player: u32,
        male: bool,
    ) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        self.players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .male = male;
        Ok(())
    }
    /// SetPlayerStyle8255b640: no range clamp in the source setter.
    pub(crate) fn try_set_style(
        &self,
        player: u32,
        style: u32,
    ) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        self.players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .style = style;
        Ok(())
    }
    /// `SetPlayerCharacter8255b058`: only this setter accepts index-1 as a no-op.
    pub(crate) fn try_set_character(
        &self,
        player: u32,
        recipe: u64,
    ) -> Result<(), PlayersSettingsError> {
        if player == u32::MAX {
            return Ok(());
        }
        let index = index(player)?;
        self.recipes
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index] = Some(recipe);
        Ok(())
    }
    /// `GetPlayerCharacter8255b0b8` reads all64 bits. Constructor/reset never writes
    /// this separate array; reading an unwritten row is an explicit host error.
    pub(crate) fn try_character(&self, player: u32) -> Result<u64, PlayersSettingsError> {
        let index = index(player)?;
        self.recipes
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .ok_or(PlayersSettingsError::UnwrittenCharacter(player))
    }
    /// `SetPlayerStance8255b570` stores the original low byte, including noncanonical
    /// marquee payloads. Boolean consumers test nonzero at their own boundary.
    pub(crate) fn try_set_stance_byte(
        &self,
        player: u32,
        regular: u8,
    ) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        self.players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .regular = regular;
        Ok(())
    }
    pub(crate) fn try_stance_byte(&self, player: u32) -> Result<u8, PlayersSettingsError> {
        let index = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .regular)
    }
    /// `SetHatType8255b4a0` stores the raw enum word without clamping.
    pub(crate) fn try_set_hat(&self, player: u32, hat: u32) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        self.players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .hat = hat;
        Ok(())
    }
    pub(crate) fn try_hat(&self, player: u32) -> Result<u32, PlayersSettingsError> {
        let index = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .hat)
    }
    pub(crate) fn try_character_is_male(&self, player: u32) -> Result<bool, PlayersSettingsError> {
        let index = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .male)
    }
    pub(crate) fn try_style(&self, player: u32) -> Result<u32, PlayersSettingsError> {
        let index = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .style)
    }
    pub(crate) fn try_gestures(&self, player: u32) -> Result<[u32; 4], PlayersSettingsError> {
        let index = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .gestures)
    }
    /// `ConvertAiPlayerType` 0x8255aad0: an AI row takes another AI type.
    pub(crate) fn try_convert_ai_player_type(&self, player: u32, player_type: u32) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        self.players.lock().map_err(|_| PlayersSettingsError::Poisoned)?[index].selection.player_type = player_type;
        Ok(())
    }
    /// `GetPlayerType8255aa60` accepts indices0..8; no -1 exception.
    pub(crate) fn try_player_type(&self, player: u32) -> Result<u32, PlayersSettingsError> {
        let index = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .selection
            .player_type)
    }
}
