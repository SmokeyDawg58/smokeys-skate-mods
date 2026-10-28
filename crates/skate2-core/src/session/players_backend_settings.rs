//! Retained selected `PlayersBackEnd` settings fields. Character/CAC loading must
//! write this same owner; construction defaults are not forced frame inputs.
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
pub(crate) enum PhysicsSetting {
    TruckTightness,
    WheelHardness,
}
impl PhysicsSetting {
    const fn index(self) -> usize {
        match self {
            Self::TruckTightness => 0,
            Self::WheelHardness => 1,
        }
    }
}
#[path = "players_backend_selection.rs"]
pub(crate) mod selection;
#[path = "players_backend_scoring.rs"]
pub(crate) mod scoring;
#[derive(Clone, Copy)]
struct PlayerSettingsRow {
    controller_index: i32,
    physics: [f32; 2],
    selection: selection::SelectionFields,
    creation: create::PlayerCreationFields,
    scoring: scoring::PlayerScoringFields,
}
pub(crate) struct PlayersBackendSettings {
    creation: Mutex<create::BackendCreationFields>,
    players: Mutex<[PlayerSettingsRow; 9]>,
    // Native mPlayerRecipe is separate and unwritten by constructor/ResetPlayers.
    recipes: Mutex<[Option<u64>; 9]>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PlayersSettingsError {
    InvalidPlayer(u32),
    UnwrittenCharacter(u32),
    Poisoned,
}
impl std::fmt::Display for PlayersSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlayer(index) => write!(f, "invalid PlayersBackEnd player index {index}"),
            Self::UnwrittenCharacter(index) => {
                write!(f, "PlayersBackEnd character {index} is unwritten")
            }
            Self::Poisoned => f.write_str("PlayersBackEnd settings lock poisoned"),
        }
    }
}
impl PlayersBackendSettings {
    /// Selected constructor8256bf30: nine PlayersData.Reset calls in ascending
    /// address order, then `ResetPlayers82564338` repeats the nine resets.
    /// Other backend fields and module registration belong to startup.
    pub(crate) fn construct_fields() -> Arc<Self> {
        let mut players = [PlayerSettingsRow {
            controller_index: 0,
            physics: [0.0; 2],
            selection: selection::SelectionFields::reset(),
            creation: create::PlayerCreationFields::before_reset(),
            scoring: scoring::PlayerScoringFields::reset(),
        }; 9];
        for _ in 0..2 {
            for player in &mut players {
                reset(player);
            }
        }
        Arc::new(Self {
            players: Mutex::new(players),
            creation: Mutex::new(create::BackendCreationFields::reset()),
            recipes: Mutex::new([None; 9]),
        })
    }
    /// Direct source `GetPhysicsSetting8255b318`: backend+a0+160*index+4*setting.
    /// Both enum alternatives are valid original ePhysicsSettings values.
    pub(crate) fn try_get(
        &self,
        player: u32,
        setting: PhysicsSetting,
    ) -> Result<f32, PlayersSettingsError> {
        let index = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .physics[setting.index()])
    }
    /// `SetPhysicsSetting8255b3a0` stores without clamping. Actual CAC loaders
    /// call this twice in enum order; no private Actor tuning is substituted.
    pub(crate) fn try_set(
        &self,
        player: u32,
        setting: PhysicsSetting,
        value: f32,
    ) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        self.players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[index]
            .physics[setting.index()] = value;
        Ok(())
    }
    /// Selected settings portion of PlayersData.Reset82561e68 for one actual
    /// row. Whole `ResetPlayers` has additional live-player assertions.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "selected reset awaits actual backend player lifecycle"
        )
    )]
    pub(crate) fn try_reset_player_settings(
        &self,
        player: u32,
    ) -> Result<(), PlayersSettingsError> {
        let index = index(player)?;
        reset(
            &mut self
                .players
                .lock()
                .map_err(|_| PlayersSettingsError::Poisoned)?[index],
        );
        Ok(())
    }
}
fn index(player: u32) -> Result<usize, PlayersSettingsError> {
    if player > 8 {
        return Err(PlayersSettingsError::InvalidPlayer(player));
    }
    Ok(usize::try_from(player).expect("checked nine native indices"))
}
fn reset(player: &mut PlayerSettingsRow) {
    player.controller_index = 0; // PlayersData.Reset82561e98, before remaining fields.
    player.selection = selection::SelectionFields::reset();
    player.creation.reset();
    player.scoring = scoring::PlayerScoringFields::reset();
    player.physics[1] = f32::from_bits(0x3f33_3333); //82561ef4 original8202e96c
    player.physics[0] = f32::from_bits(0x3e99_999a); //82561efc following constant
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_settings_accept_slot_eight_unclamped_values_and_independent_resets() {
        let owner = PlayersBackendSettings::construct_fields();
        let same = Arc::clone(&owner);
        assert_eq!(
            owner
                .try_get(8, PhysicsSetting::TruckTightness)
                .unwrap()
                .to_bits(),
            0x3e99_999a
        );
        assert_eq!(
            owner
                .try_get(8, PhysicsSetting::WheelHardness)
                .unwrap()
                .to_bits(),
            0x3f33_3333
        );
        owner
            .try_set(8, PhysicsSetting::TruckTightness, -4.0)
            .unwrap();
        owner
            .try_set(
                0,
                PhysicsSetting::WheelHardness,
                f32::from_bits(0x7fc0_1234),
            )
            .unwrap();
        assert_eq!(
            same.try_get(8, PhysicsSetting::TruckTightness)
                .unwrap()
                .to_bits(),
            (-4.0_f32).to_bits()
        );
        owner.try_reset_player_settings(8).unwrap();
        assert_eq!(
            same.try_get(0, PhysicsSetting::WheelHardness)
                .unwrap()
                .to_bits(),
            0x7fc0_1234
        );
        assert_eq!(
            owner.try_get(9, PhysicsSetting::TruckTightness),
            Err(PlayersSettingsError::InvalidPlayer(9))
        );
    }
}

#[path = "players_backend_controller.rs"]
pub(crate) mod controller;

#[path = "players_backend_create.rs"]
pub(crate) mod create;
