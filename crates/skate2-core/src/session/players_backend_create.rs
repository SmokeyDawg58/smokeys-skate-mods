//! Offline human `CreatePlayer82563d50` on the retained backend, before loading.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "actual offline creation is exercised by original startup corpus; app boot caller remains pending"
    )
)]
use super::{PlayersBackendSettings, PlayersSettingsError, index};
use crate::skater_manager_local_identity::{LocalBackendOwner, LocalIdentityError};
use std::sync::{Arc, Mutex};

/// Narrow actual ProfileBackEnd+28 storage. Constructor825808a0 copies PLAYER 1;
/// remaining bytes are unwritten, not a substitute loaded profile or account.
pub(crate) struct ProfileNameOwner {
    name: Mutex<[Option<u8>; 32]>,
}
impl ProfileNameOwner {
    pub(crate) fn construct_name() -> Arc<Self> {
        let mut name = [None; 32];
        for (dst, src) in name.iter_mut().zip(b"PLAYER 1\0") {
            *dst = Some(*src);
        }
        Arc::new(Self {
            name: Mutex::new(name),
        })
    }
    /// The name `FECharacterManager::GetCharacter`8261be58 shows for the created skater.
    pub(crate) fn name(&self) -> Option<String> {
        self.read_c_string().ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }
    /// ProfileBackEnd::LoadProfileData8256c1a0 / SetProfileName825647c0.
    pub(crate) fn set_saved_name(&self, saved: [u8; 32]) -> Result<(), PlayerCreateError> {
        let mut name = self.name.lock().map_err(|_| PlayerCreateError::Poisoned)?;
        *name = saved.map(Some);
        name[31] = Some(0);
        Ok(())
    }
    fn read_c_string(&self) -> Result<Vec<u8>, PlayerCreateError> {
        let name = self.name.lock().map_err(|_| PlayerCreateError::Poisoned)?;
        let mut result = Vec::new();
        for byte in name.iter() {
            let byte = byte.ok_or(PlayerCreateError::UnknownProfileName)?;
            if byte == 0 {
                return Ok(result);
            }
            result.push(byte);
        }
        Err(PlayerCreateError::UnknownProfileName)
    }
}
#[derive(Clone, Copy)]
pub(super) struct PlayerCreationFields {
    pub(super) peer: i64,
    rank: i32,
    short_name: [Option<u8>; 32],
    long_name: [Option<u8>; 32],
}
impl PlayerCreationFields {
    pub(super) const fn before_reset() -> Self {
        Self {
            peer: -1,
            rank: 0,
            short_name: [None; 32],
            long_name: [None; 32],
        }
    }
    pub(super) fn reset(&mut self) {
        self.rank = 0;
        self.short_name[0] = Some(0);
        self.peer = -1;
    }
}
pub(super) struct BackendCreationFields {
    total: i32,
    humans: i32,
    rank_map: [i32; 9],
}
impl BackendCreationFields {
    pub(super) const fn reset() -> Self {
        Self {
            total: 0,
            humans: 0,
            rank_map: [0; 9],
        }
    }
}
#[derive(Debug)]
pub(crate) enum PlayerCreateError {
    Settings(PlayersSettingsError),
    Identity(LocalIdentityError),
    InvalidCreateIndex(u32),
    AlreadyPresent(u32),
    NotPresent(u32),
    NetworkNamePending,
    PartyNamePending,
    UnknownProfileName,
    InvalidRank(i32),
    TooManyHumans(i32),
    TooManyPlayers(i32),
    Poisoned,
}
impl std::fmt::Display for PlayerCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Settings(error) => write!(f, "offline player settings: {error:?}"),
            Self::Identity(error) => write!(f, "offline player identity: {error:?}"),
            Self::InvalidCreateIndex(index) => write!(f, "invalid create index {index}"),
            Self::AlreadyPresent(index) => write!(f, "player {index} already present"),
            Self::NotPresent(index) => write!(f, "player {index} not present"),
            Self::InvalidRank(rank) => write!(f, "invalid player rank {rank}"),
            Self::TooManyHumans(count) => write!(f, "too many humans: {count}"),
            Self::TooManyPlayers(count) => write!(f, "too many players: {count}"),
            other => write!(f, "offline player creation: {other:?}"),
        }
    }
}
impl PlayersBackendSettings {
    /// SetProfileName825647c0 immediately updates player zero's two names.
    pub(crate) fn try_update_profile_name(&self, profile: &ProfileNameOwner) -> Result<(), PlayerCreateError> {
        let name = profile.read_c_string()?;
        let mut rows = self.players.lock().map_err(|_| PlayerCreateError::Poisoned)?;
        copy_name(&mut rows[0].creation.short_name, &name);
        copy_name(&mut rows[0].creation.long_name, &name);
        Ok(())
    }
    /// Actual first-game call chooses `PT_Human` and peer=-1 at8277b158..168.
    /// Callers supply the retained backend/profile used by subsequent attributes.
    /// Network/party naming is not replaced by the ordinary offline branch.
    pub(crate) fn try_create_offline_human(
        &self,
        player: u32,
        profile: &Arc<ProfileNameOwner>,
        backend: &LocalBackendOwner,
    ) -> Result<(), PlayerCreateError> {
        use PlayerCreateError as E;
        if player >= 8 {
            return Err(E::InvalidCreateIndex(player));
        }
        let i = index(player).map_err(E::Settings)?;
        {
            let mut rows = self.players.lock().map_err(|_| E::Poisoned)?;
            if rows[i].selection.player_type != 0 {
                return Err(E::AlreadyPresent(player));
            }
            rows[i].selection.player_type = 1;
            rows[i].creation.peer = -1;
        }
        // UpdatePlayerName reads network, then only offline game mode.
        if backend.try_is_network_game().map_err(E::Identity)? {
            return Err(E::NetworkNamePending);
        }
        if backend.try_game_mode().map_err(E::Identity)? == 2 {
            return Err(E::PartyNamePending);
        }
        let captured_profile = Arc::clone(profile);
        let short = captured_profile.read_c_string()?;
        copy_name(
            &mut self.players.lock().map_err(|_| E::Poisoned)?[i]
                .creation
                .short_name,
            &short,
        );
        let long = captured_profile.read_c_string()?;
        copy_name(
            &mut self.players.lock().map_err(|_| E::Poisoned)?[i]
                .creation
                .long_name,
            &long,
        );
        self.players.lock().map_err(|_| E::Poisoned)?[i]
            .creation
            .peer = -1;
        let rank = self
            .creation
            .lock()
            .map_err(|_| E::Poisoned)?
            .total
            .wrapping_add(1);
        if !(1..=9).contains(&rank) {
            return Err(E::InvalidRank(rank));
        }
        self.players.lock().map_err(|_| E::Poisoned)?[i]
            .creation
            .rank = rank;
        self.creation.lock().map_err(|_| E::Poisoned)?.rank_map
            [usize::try_from(rank - 1).expect("checked native rank")] =
            i32::try_from(player).expect("checked player");
        let mut fields = self.creation.lock().map_err(|_| E::Poisoned)?;
        fields.humans = fields.humans.wrapping_add(1);
        if fields.humans > 8 {
            return Err(E::TooManyHumans(fields.humans));
        }
        fields.total = fields.total.wrapping_add(1);
        if fields.total > 8 {
            return Err(E::TooManyPlayers(fields.total));
        }
        Ok(())
    }
    /// `CreatePlayer` 0x82563d50 for a party player (`FrontEndState_PartyPlay::OnUpdate`
    /// 0x8264aac8): `PT_Human` with its peer, `UpdatePlayerName` 0x82563a88's
    /// `GM_PartyPlay` branch (the caller's `FormatString(ID_PARTY_PLAYER_PLAYER, index + 1)`
    /// as both names), `SetPlayerRank` to the new total, then the human and player counts.
    pub(crate) fn try_create_party_human(
        &self,
        player: u32,
        peer: i64,
        name: &[u8],
    ) -> Result<(), PlayerCreateError> {
        use PlayerCreateError as E;
        if player >= 8 {
            return Err(E::InvalidCreateIndex(player));
        }
        let i = index(player).map_err(E::Settings)?;
        {
            let mut rows = self.players.lock().map_err(|_| E::Poisoned)?;
            if rows[i].selection.player_type != 0 {
                return Err(E::AlreadyPresent(player));
            }
            rows[i].selection.player_type = 1;
            rows[i].creation.peer = peer;
            copy_name(&mut rows[i].creation.short_name, name);
            copy_name(&mut rows[i].creation.long_name, name);
        }
        let rank = self
            .creation
            .lock()
            .map_err(|_| E::Poisoned)?
            .total
            .wrapping_add(1);
        if !(1..=9).contains(&rank) {
            return Err(E::InvalidRank(rank));
        }
        self.players.lock().map_err(|_| E::Poisoned)?[i]
            .creation
            .rank = rank;
        self.creation.lock().map_err(|_| E::Poisoned)?.rank_map
            [usize::try_from(rank - 1).expect("checked native rank")] =
            i32::try_from(player).expect("checked player");
        let mut fields = self.creation.lock().map_err(|_| E::Poisoned)?;
        fields.humans = fields.humans.wrapping_add(1);
        if fields.humans > 8 {
            return Err(E::TooManyHumans(fields.humans));
        }
        fields.total = fields.total.wrapping_add(1);
        if fields.total > 8 {
            return Err(E::TooManyPlayers(fields.total));
        }
        Ok(())
    }
    /// `CreatePlayer` 0x82563d50 for an AI skater (`AISkaterManager::SpawnAISkater` 0x824cba18):
    /// its AI player type and `GetPeerIDFromSkaterID` peer, `SetPlayerRank` to the new total,
    /// then the player count. `UpdatePlayerName` 0x82563a88's AI names follow from
    /// `AI.SpawnAIAtLocation` ([`Self::try_set_ai_names`]).
    pub(crate) fn try_create_ai_player(
        &self,
        player: u32,
        player_type: u32,
        peer: i64,
    ) -> Result<(), PlayerCreateError> {
        use PlayerCreateError as E;
        if player >= 8 {
            return Err(E::InvalidCreateIndex(player));
        }
        let i = index(player).map_err(E::Settings)?;
        {
            let mut rows = self.players.lock().map_err(|_| E::Poisoned)?;
            if rows[i].selection.player_type != 0 {
                return Err(E::AlreadyPresent(player));
            }
            rows[i].selection.player_type = player_type;
            rows[i].creation.peer = peer;
        }
        let rank = self
            .creation
            .lock()
            .map_err(|_| E::Poisoned)?
            .total
            .wrapping_add(1);
        if !(1..=9).contains(&rank) {
            return Err(E::InvalidRank(rank));
        }
        self.players.lock().map_err(|_| E::Poisoned)?[i]
            .creation
            .rank = rank;
        let mut fields = self.creation.lock().map_err(|_| E::Poisoned)?;
        fields.rank_map[usize::try_from(rank - 1).expect("checked native rank")] =
            i32::try_from(player).expect("checked player");
        fields.total = fields.total.wrapping_add(1);
        if fields.total > 8 {
            return Err(E::TooManyPlayers(fields.total));
        }
        Ok(())
    }
    /// `SetPlayerRank` 0x8255a538: the row's rank and the player in the rank map; an index past
    /// 8 or a rank outside 1..=9 is the native's assertion.
    pub(crate) fn try_set_player_rank(&self, player: u32, rank: i32) -> Result<(), PlayerCreateError> {
        use PlayerCreateError as E;
        let i = index(player).map_err(E::Settings)?;
        if !(1..=9).contains(&rank) {
            return Err(E::InvalidRank(rank));
        }
        self.players.lock().map_err(|_| E::Poisoned)?[i].creation.rank = rank;
        self.creation.lock().map_err(|_| E::Poisoned)?.rank_map[usize::try_from(rank - 1).expect("checked native rank")] =
            i32::try_from(player).expect("checked player");
        Ok(())
    }
    /// `RemovePlayer` 0x8256bfa8: every later rank moves up one, a human leaves the human
    /// count, then `PlayersData::Reset` and the player count.
    pub(crate) fn try_remove_player(&self, player: u32) -> Result<(), PlayerCreateError> {
        use PlayerCreateError as E;
        if player >= 8 {
            return Err(E::InvalidCreateIndex(player));
        }
        let i = index(player).map_err(E::Settings)?;
        let mut rows = self.players.lock().map_err(|_| E::Poisoned)?;
        if rows[i].selection.player_type == 0 {
            return Err(E::NotPresent(player));
        }
        let mut fields = self.creation.lock().map_err(|_| E::Poisoned)?;
        let mut rank = rows[i].creation.rank;
        while rank < fields.total {
            if !(1..=8).contains(&rank) {
                return Err(E::InvalidRank(rank));
            }
            let slot = usize::try_from(rank).expect("checked native rank");
            let next = fields.rank_map[slot];
            let next_row = usize::try_from(next)
                .ok()
                .filter(|&row| row <= 8)
                .ok_or(E::InvalidRank(next))?;
            rows[next_row].creation.rank = rank;
            fields.rank_map[slot - 1] = next;
            rank += 1;
        }
        if rows[i].selection.player_type == 1 {
            fields.humans -= 1;
        }
        super::reset(&mut rows[i]);
        fields.total -= 1;
        Ok(())
    }
    /// `ResetPlayers` 0x82564338: the totals, then every row's `PlayersData::Reset` and its
    /// rank map entry.
    pub(crate) fn try_reset_players(&self) -> Result<(), PlayerCreateError> {
        *self.creation.lock().map_err(|_| PlayerCreateError::Poisoned)? = BackendCreationFields::reset();
        for row in self.players.lock().map_err(|_| PlayerCreateError::Poisoned)?.iter_mut() {
            super::reset(row);
        }
        Ok(())
    }
    /// `UpdatePlayerName` 0x82563a88's AI names: `Snprintf(mName, 0x20, ...)` of the short name
    /// and, when given, of the long one, each cut to 31 bytes and terminated.
    pub(crate) fn try_set_ai_names(&self, player: u32, short: &[u8], long: Option<&[u8]>) -> Result<(), PlayersSettingsError> {
        let i = index(player)?;
        let mut rows = self.players.lock().map_err(|_| PlayersSettingsError::Poisoned)?;
        copy_name(&mut rows[i].creation.short_name, short);
        if let Some(long) = long {
            copy_name(&mut rows[i].creation.long_name, long);
        }
        Ok(())
    }
    /// `GetPlayerName` 0x8255a728: the row's `mName`, up to its terminator.
    pub(crate) fn try_player_name(&self, player: u32) -> Result<String, PlayersSettingsError> {
        let i = index(player)?;
        let name = self.players.lock().map_err(|_| PlayersSettingsError::Poisoned)?[i].creation.short_name;
        let bytes: Vec<u8> = name.iter().map_while(|b| b.filter(|&b| b != 0)).collect();
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
    /// `GetPlayerPeerID8255aba8` reads backend+152+160*index, including index8.
    pub(crate) fn try_peer_id(&self, player: u32) -> Result<i64, PlayersSettingsError> {
        let i = index(player)?;
        Ok(self
            .players
            .lock()
            .map_err(|_| PlayersSettingsError::Poisoned)?[i]
            .creation
            .peer)
    }
}
fn copy_name(output: &mut [Option<u8>; 32], input: &[u8]) {
    let length = input.len().min(31);
    for (dst, src) in output.iter_mut().zip(&input[..length]) {
        *dst = Some(*src);
    }
    output[length] = Some(0);
}
#[cfg(test)]
#[path = "players_backend_create_tests.rs"]
mod tests;
