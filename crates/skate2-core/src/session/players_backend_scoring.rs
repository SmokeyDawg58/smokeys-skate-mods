//! PlayersData.Reset82561ec4..ed0 and original destructive PopReset getters.
use super::{PlayersBackendSettings, PlayersSettingsError, index};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlayerScoringFields {
    pub(crate) manuals_are_connectors: bool,
    pub(crate) reset_sequence: bool,
    pub(crate) reset_scoring: bool,
    pub(crate) reset_damage: bool,
}
impl PlayerScoringFields {
    pub(super) const fn reset() -> Self {
        Self {
            manuals_are_connectors: true,
            reset_sequence: false,
            reset_scoring: false,
            reset_damage: false,
        }
    }
}
impl PlayersBackendSettings {
    /// `SetManualsAreConnectors`/`SetResetSequence`/`SetResetScoring`
    /// (0x8255ac18/0x8255ace8/0x8255adc8).
    pub(crate) fn try_set_scoring(
        &self,
        player: u32,
        set: impl FnOnce(&mut PlayerScoringFields),
    ) -> Result<(), PlayersSettingsError> {
        let i = index(player)?;
        set(&mut self.players.lock().map_err(|_| PlayersSettingsError::Poisoned)?[i].scoring);
        Ok(())
    }
    /// World.UpdateConditioners reads connectors then pops sequence/scoring/
    /// damage (8255ad48/ae28/af08) from this same actual player row.
    pub(crate) fn try_pop_scoring(
        &self,
        player: u32,
    ) -> Result<PlayerScoringFields, PlayersSettingsError> {
        let i = index(player)?;
        let mut players = self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?;
        let fields = &mut players[i].scoring;
        let result = *fields;
        fields.reset_sequence = false;
        fields.reset_scoring = false;
        fields.reset_damage = false;
        Ok(result)
    }
    /// The scoring part of `ResetPlayers` (0x82564338): every row's `PlayersData.Reset`
    /// (0x82561e68) scoring fields, so manuals connect again and no reset is pending.
    pub(crate) fn try_reset_scoring(&self) -> Result<(), PlayersSettingsError> {
        let mut players = self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?;
        for player in &mut *players {
            player.scoring = PlayerScoringFields::reset();
        }
        Ok(())
    }
}

/// The challenge natives' view of this back end; an index past 8 asserts natively and reads
/// the empty row here.
impl crate::challenge::players::PlayersBackEnd for PlayersBackendSettings {
    fn player_type(&self, player: i32) -> i32 {
        u32::try_from(player).ok().and_then(|p| self.try_player_type(p).ok()).map_or(0, |t| t.cast_signed())
    }
    fn player_name(&self, player: i32) -> String {
        u32::try_from(player).ok().and_then(|p| self.try_player_name(p).ok()).unwrap_or_default()
    }
    fn is_stance_regular(&self, player: i32) -> bool {
        u32::try_from(player).ok().and_then(|p| self.try_stance_byte(p).ok()).is_some_and(|b| b != 0)
    }
    fn peer_id(&self, player: i32) -> i64 {
        u32::try_from(player).ok().and_then(|p| self.try_peer_id(p).ok()).unwrap_or(-1)
    }
    fn set_manuals_are_connectors(&self, player: i32, value: bool) {
        if let Ok(p) = u32::try_from(player) {
            let _ = self.try_set_scoring(p, |s| s.manuals_are_connectors = value);
        }
    }
    fn set_reset_sequence(&self, player: i32, value: bool) {
        if let Ok(p) = u32::try_from(player) {
            let _ = self.try_set_scoring(p, |s| s.reset_sequence = value);
        }
    }
    fn character(&self, player: i32) -> u64 {
        u32::try_from(player).ok().and_then(|p| self.try_character(p).ok()).unwrap_or(u64::MAX)
    }
    fn num_players(&self) -> i32 {
        let created = (0..8).filter(|&p| self.try_player_type(p).is_ok_and(|t| t != 0)).count();
        i32::try_from(created).unwrap_or(0)
    }
    fn set_reset_scoring(&self, player: i32, value: bool) {
        if let Ok(p) = u32::try_from(player) {
            let _ = self.try_set_scoring(p, |s| s.reset_scoring = value);
        }
    }
    fn set_player_rank(&self, player: i32, rank: i32) {
        if let Ok(p) = u32::try_from(player) {
            let _ = self.try_set_player_rank(p, rank);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pops_clear_only_requested_flags_on_the_actual_player() {
        let backend = PlayersBackendSettings::construct_fields();
        {
            let mut rows = backend.players.lock().unwrap();
            rows[8].scoring = PlayerScoringFields {
                manuals_are_connectors: false,
                reset_sequence: true,
                reset_scoring: true,
                reset_damage: true,
            };
        }
        let first = backend.try_pop_scoring(8).unwrap();
        assert_eq!(
            first,
            PlayerScoringFields {
                manuals_are_connectors: false,
                reset_sequence: true,
                reset_scoring: true,
                reset_damage: true
            }
        );
        let second = backend.try_pop_scoring(8).unwrap();
        assert_eq!(
            second,
            PlayerScoringFields {
                manuals_are_connectors: false,
                reset_sequence: false,
                reset_scoring: false,
                reset_damage: false
            }
        );
        assert_eq!(
            backend.try_pop_scoring(0).unwrap(),
            PlayerScoringFields::reset()
        );
        assert_eq!(
            backend.try_pop_scoring(9),
            Err(PlayersSettingsError::InvalidPlayer(9))
        );
    }
    #[test]
    fn a_players_reset_makes_manuals_connectors_again_on_every_row() {
        let backend = PlayersBackendSettings::construct_fields();
        {
            let mut rows = backend.players.lock().unwrap();
            for row in &mut *rows {
                row.scoring = PlayerScoringFields {
                    manuals_are_connectors: false,
                    reset_sequence: true,
                    reset_scoring: true,
                    reset_damage: true,
                };
            }
        }
        backend.try_reset_scoring().unwrap();
        for player in 0..9 {
            assert_eq!(
                backend.try_pop_scoring(player).unwrap(),
                PlayerScoringFields::reset()
            );
        }
    }
}
