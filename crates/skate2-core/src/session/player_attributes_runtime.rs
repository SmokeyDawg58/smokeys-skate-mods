//! PlayerLoader.ApplyPlayerAttributes828158f0 on the existing retained owners.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "native loader continuation awaits mounted application bootstrap; tests exercise original owners"
    )
)]
use super::{
    PlayerLoadAction, PlayerLoadPhase, PlayerLoaderError, PlayerLoaderSlot, PlayerSelection,
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    players_backend_settings::{PlayersBackendSettings, PlayersSettingsError},
    skater_manager_local_identity::{LocalIdentityError, SkaterManagerLocalIdentity},
};
use std::sync::Arc;

const MARQUEE_CLASS: u64 = 0x91bc_a669_3efc_9aa7;
const HAT_KEY: u64 = 0x3441_8cad_1c23_bc94;

pub(crate) struct PlayerAttributeServices {
    pub(crate) profile: Arc<crate::players_backend_settings::create::ProfileNameOwner>,
    pub(crate) vault: Arc<AttributeRuntimeVault>,
    pub(crate) settings: Arc<PlayersBackendSettings>,
    pub(crate) skaters: Arc<SkaterManagerLocalIdentity>,
}
#[derive(Debug)]
pub(crate) enum PlayerAttributesError {
    Protocol(PlayerLoaderError),
    Attribute(AttributeRuntimeError),
    Settings(PlayersSettingsError),
    Identity(LocalIdentityError),
    WrongPhase(PlayerLoadPhase),
    CacPending,
    InvalidHatWidth(usize),
    Reload(String),
}
impl std::fmt::Display for PlayerAttributesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(e) => write!(f, "player attributes protocol: {e}"),
            Self::Attribute(e) => write!(f, "player attributes Vault: {e}"),
            Self::Settings(e) => write!(f, "player attributes backend: {e}"),
            Self::Identity(e) => write!(f, "player attributes roster: {e}"),
            Self::WrongPhase(phase) => write!(f, "player attributes phase: {phase:?}"),
            Self::CacPending => f.write_str("native CAC ApplyPlayerAttributes remains pending"),
            Self::InvalidHatWidth(width) => write!(f, "marquee hat field width {width} is not4"),
            Self::Reload(error) => write!(f, "Actor character reload: {error}"),
        }
    }
}
impl PlayerAttributeServices {
    /// Sk8CACLoader::ApplyAttributesImpl82819cb8, before the outer player
    /// loader callback. Unlike that callback, this stage restores saved style.
    pub(crate) fn apply_cac_loader_attributes(
        &self,
        record: &skate2_formats::skater_profile::CacRecord,
        player: u32,
    ) -> Result<(), PlayerAttributesError> {
        use crate::players_backend_settings::PhysicsSetting;
        use PlayerAttributesError as E;
        for (setting, bits) in [
            PhysicsSetting::TruckTightness,
            PhysicsSetting::WheelHardness,
        ]
        .into_iter()
        .zip(record.data.tuning_bits)
        {
            self.settings
                .try_set(player, setting, f32::from_bits(bits))
                .map_err(E::Settings)?;
        }
        self.settings
            .try_set_hat(player, record.data.hat_type)
            .map_err(E::Settings)?;
        self.settings
            .try_set_character(player, u64::MAX)
            .map_err(E::Settings)?;
        self.settings
            .try_set_stance_byte(player, u8::from(record.data.stance == 1))
            .map_err(E::Settings)?;
        self.settings
            .try_set_style(player, record.data.style)
            .map_err(E::Settings)?;
        self.settings
            .try_set_gestures(player, record.data.gestures)
            .map_err(E::Settings)?;
        self.settings
            .try_set_character_is_male(player, record.data.is_male)
            .map_err(E::Settings)?;
        Ok(())
    }
    /// Native CAC branch82815920..b40. Returns whether the caller must complete
    /// GameModule::ReloadLocalPlayerAttributeData on its existing UI owner.
    /// Style is deliberately not written here: source Apply does not write it.
    pub(crate) fn apply_cac_record(
        &self,
        record: &skate2_formats::skater_profile::CacRecord,
        player: u32,
    ) -> Result<bool, PlayerAttributesError> {
        use crate::players_backend_settings::PhysicsSetting;
        use PlayerAttributesError as E;
        self.settings
            .try_set_gestures(player, record.data.gestures)
            .map_err(E::Settings)?;
        self.settings
            .try_set_character_is_male(player, record.data.is_male)
            .map_err(E::Settings)?;
        for (setting, bits) in [
            PhysicsSetting::TruckTightness,
            PhysicsSetting::WheelHardness,
        ]
        .into_iter()
        .zip(record.data.tuning_bits)
        {
            self.settings
                .try_set(player, setting, f32::from_bits(bits))
                .map_err(E::Settings)?;
        }
        self.settings
            .try_set_character(player, u64::MAX)
            .map_err(E::Settings)?;
        self.settings
            .try_set_stance_byte(player, u8::from(record.data.stance == 1))
            .map_err(E::Settings)?;
        self.settings
            .try_set_hat(player, record.data.hat_type)
            .map_err(E::Settings)?;
        let local = self.skaters.try_local_index().map_err(E::Identity)?;
        let actor = self.skaters.try_find_by_index(local).map_err(E::Identity)?;
        if let Some(actor) = actor {
            crate::actor_animation_construction::edit_skater_attributes::reload_character_data(
                &actor,
                &self.settings,
            )
            .map_err(E::Reload)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn apply(&self, selection: PlayerSelection, player: u32) -> Result<(), PlayerAttributesError> {
        use PlayerAttributesError as E;
        let PlayerSelection::Marquee { definition_id, .. } = selection else {
            return Err(E::CacPending);
        };
        self.apply_marquee(definition_id, player)?;
        // ApplyPlayerAttributes82815ae8: the local Actor reloads its character data; the
        // caller's GameModule::ReloadLocalPlayerAttributeData follows, as for a CAC record.
        let local = self.skaters.try_local_index().map_err(E::Identity)?;
        if let Some(actor) = self.skaters.try_find_by_index(local).map_err(E::Identity)? {
            crate::actor_animation_construction::edit_skater_attributes::reload_character_data(
                &actor,
                &self.settings,
            )
            .map_err(E::Reload)?;
        }
        Ok(())
    }
    /// `PlayerLoaderImpl::ApplyPlayerAttributes` 0x828158f0 for a loaded skater outside its load:
    /// a created skater's attributes from the profile's parts record, a marquee's from its
    /// definition, then the local Actor's character reload.
    pub(crate) fn apply_player_attributes(
        &self,
        selection: PlayerSelection,
        player: u32,
        record: Option<&skate2_formats::skater_profile::CacRecord>,
    ) -> Result<(), PlayerAttributesError> {
        if matches!(selection, PlayerSelection::CreateACharacter { .. }) {
            let record = record.ok_or(PlayerAttributesError::CacPending)?;
            return self.apply_cac_record(record, player).map(drop);
        }
        self.apply(selection, player)
    }
    /// `ApplyPlayerAttributes` 0x828158f0's marquee writes: `SetPlayerCharacter`,
    /// `SetPlayerStance(IsSkaterRegular)` 0x82cfea68 and `SetHatType(GetSkaterHatType)`
    /// 0x82cfe9e8. Two independent generated wrappers, not a cached combined profile.
    /// Generated constructor824b8eb0 substitutes `DefaultDataArea20` only for a
    /// null layout; hat uses `GetAttributePointer82cfea24` and `DefaultDataArea4`.
    pub(crate) fn apply_marquee(&self, definition_id: u64, player: u32) -> Result<(), PlayerAttributesError> {
        use PlayerAttributesError as E;
        let regular = {
            let instance = self.vault.instance(MARQUEE_CLASS, definition_id);
            instance
                .try_cached_layout_field(16, 1)
                .map_err(E::Attribute)?
                .map_or(0, |field| field.bytes()[0])
        };
        let hat = {
            let instance = self.vault.instance(MARQUEE_CLASS, definition_id);
            match instance.try_get_data(HAT_KEY, 0).map_err(E::Attribute)? {
                None => 0,
                Some(data) => {
                    let bytes = data.bytes();
                    let bytes: [u8; 4] = bytes
                        .try_into()
                        .map_err(|_| E::InvalidHatWidth(bytes.len()))?;
                    u32::from_be_bytes(bytes)
                }
            }
        };
        self.settings
            .try_set_character(player, definition_id)
            .map_err(E::Settings)?;
        self.settings
            .try_set_stance_byte(player, regular)
            .map_err(E::Settings)?;
        self.settings
            .try_set_hat(player, hat)
            .map_err(E::Settings)?;
        Ok(())
    }
}
impl PlayerLoaderSlot {
    /// CAC variant of the native82817fb8 attribute phase. UI reload callbacks
    /// belong to the caller's existing GameModule; Actor reload is completed.
    pub(crate) fn try_prepare_cac_loaded_attributes(
        &mut self,
        services: &PlayerAttributeServices,
        record: &skate2_formats::skater_profile::CacRecord,
    ) -> Result<(bool, bool), PlayerAttributesError> {
        use PlayerAttributesError as E;
        if !matches!(self.phase, PlayerLoadPhase::ApplyingAttributes { .. }) {
            return Err(E::WrongPhase(self.phase));
        }
        if !matches!(self.current, Some(PlayerSelection::CreateACharacter { .. })) {
            return Err(E::CacPending);
        }
        services.apply_cac_record(record, self.player_index)?;
        let backend = services.skaters.backend();
        if backend.try_game_mode().map_err(E::Identity)? != 2
            && backend.try_ghost_enabled().map_err(E::Identity)?
        {
            services.apply_cac_record(record, 8)?;
        }
        let apply = !backend.try_is_network_game().map_err(E::Identity)?;
        let ghost = backend.try_ghost_enabled().map_err(E::Identity)?;
        Ok((ghost, apply))
    }
    /// Native82817fb8..8010: actual primary Apply, lazy game-mode/ghost gate,
    /// optional slot8 Apply, then network and a second ghost sample for Register.
    /// Failures retain the already-stored loaded asset and completed backend writes.
    pub(crate) fn try_apply_loaded_attributes(
        &mut self,
        services: &PlayerAttributeServices,
    ) -> Result<PlayerLoadAction, PlayerAttributesError> {
        let (ghost, apply) = self.try_prepare_loaded_attributes(services)?;
        self.finish_loaded_attributes(ghost, apply)
            .map_err(PlayerAttributesError::Protocol)
    }
    /// Keep attribute and live backend sampling before real request allocation.
    pub(super) fn try_prepare_loaded_attributes(
        &mut self,
        services: &PlayerAttributeServices,
    ) -> Result<(bool, bool), PlayerAttributesError> {
        use PlayerAttributesError as E;
        if !matches!(self.phase, PlayerLoadPhase::ApplyingAttributes { .. }) {
            return Err(E::WrongPhase(self.phase));
        }
        let selection = self
            .current
            .ok_or(E::Protocol(PlayerLoaderError::MissingCurrentSelection))?;
        services.apply(selection, self.player_index)?;
        let backend = services.skaters.backend();
        if backend.try_game_mode().map_err(E::Identity)? != 2
            && backend.try_ghost_enabled().map_err(E::Identity)?
        {
            services.apply(selection, 8)?;
        }
        let apply_attributes = !backend.try_is_network_game().map_err(E::Identity)?;
        let ghost = backend.try_ghost_enabled().map_err(E::Identity)?;
        Ok((ghost, apply_attributes))
    }
}

#[cfg(test)]
#[path = "player_attributes_runtime_tests.rs"]
mod tests;
