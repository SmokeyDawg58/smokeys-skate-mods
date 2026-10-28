//! Selected workspace `CommandManager`825e58d0: retained receiver map and CRC.
//! Dispatch825e5610 is synchronous. No proxy queue or full `SyncLog` buffers.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected gameplay command services await offline scheduling and Actor wiring"
    )
)]
use crate::{
    game_input_command::{GameInputRead, GameInputStreamError, try_unpack_player_input},
    game_input_player::{
        GameInputCommand, GameInputConstructionError, GameInputPlayer, GameInputPlayerError,
        GameInputReceiver, GameInputReceiverRegistration, PlayerInputData,
    },
    input_pad_cache::PublishedInputPadSample,
};
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex, Weak},
};

pub(crate) struct EncodedGameInputCommand<'a> {
    pub(crate) receiver_id: u32,
    pub(crate) kind: u32,
    pub(crate) payload: &'a [u8],
}
pub(crate) trait GameInputSyncLevel {
    type Error;
    /// Actual signed global `syncLevel`, read only when `addToCRC` is true.
    fn sync_level(&mut self) -> Result<i32, Self::Error>;
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GameInputRegistrationError {
    DuplicateReceiver(u32),
}
#[derive(Debug)]
pub(crate) enum GameInputManagerError<E> {
    Source(E),
    PlayerPoisoned,
    PlayerExpired(u32),
    MissingRegistration(u32),
    PayloadTooLarge,
    Construction(GameInputConstructionError<GameInputRegistrationError>),
    Decode(GameInputStreamError),
    Player(GameInputPlayerError),
}
impl<E: fmt::Debug> fmt::Display for GameInputManagerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "command sync-level source: {error:?}"),
            Self::PlayerPoisoned => f.write_str("gameplay command player lock poisoned"),
            Self::PlayerExpired(id) => write!(f, "registered gameplay player {id} expired"),
            Self::MissingRegistration(id) => write!(f, "gameplay receiver {id} is not registered"),
            Self::PayloadTooLarge => f.write_str("gameplay payload exceeds native size word"),
            Self::Construction(error) => write!(f, "gameplay registration: {error}"),
            Self::Decode(error) => write!(f, "gameplay input decode: {error}"),
            Self::Player(error) => write!(f, "gameplay receiver: {error}"),
        }
    }
}
/// `SkaterManager` is command receiver 1 (spawn, teleport, pin, wipeout).
pub(crate) const SKATER_MANAGER_RECEIVER: u32 = 1;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GameInputDispatch {
    MissingReceiver,
    Delivered,
    /// Receiver 1: the stage owner runs `SkaterManager::OnCommand`82600920
    /// with its Actor and World.
    SkaterManager {
        kind: u32,
        payload: Vec<u8>,
    },
}
struct Registration {
    receiver: GameInputReceiver,
    player: Weak<Mutex<GameInputPlayer>>,
}
struct PlayerRegistrar<'a> {
    receivers: &'a mut BTreeMap<u32, Registration>,
    player: Weak<Mutex<GameInputPlayer>>,
}
impl GameInputReceiverRegistration for PlayerRegistrar<'_> {
    type Error = GameInputRegistrationError;
    fn register(&mut self, receiver: &GameInputReceiver) -> Result<(), Self::Error> {
        // Source first ID lookup/assert, then second ID read for insertion.
        let lookup = receiver.id();
        if self.receivers.contains_key(&lookup) {
            return Err(GameInputRegistrationError::DuplicateReceiver(lookup));
        }
        self.receivers.insert(
            receiver.id(),
            Registration {
                receiver: receiver.clone(),
                player: self.player.clone(),
            },
        );
        Ok(())
    }
}
/// Native workspace manager's selected map/CRC projection. Inline pool storage
/// and ancillary `SyncLog` buffers are outside this semantic owner.
pub(crate) struct GameInputCommandManager {
    receivers: BTreeMap<u32, Registration>,
    crc: u32,
}
impl GameInputCommandManager {
    pub(crate) const fn new() -> Self {
        Self {
            receivers: BTreeMap::new(),
            crc: 0,
        }
    }
    /// Bind registration exclusively to this locked player's occurrence. The
    /// private registrar never locks it recursively or accepts another player.
    pub(crate) fn try_construct_player(
        &mut self,
        player: &Arc<Mutex<GameInputPlayer>>,
    ) -> Result<GameInputReceiver, GameInputManagerError<std::convert::Infallible>> {
        let mut locked = player
            .lock()
            .map_err(|_| GameInputManagerError::PlayerPoisoned)?;
        let mut registrar = PlayerRegistrar {
            receivers: &mut self.receivers,
            player: Arc::downgrade(player),
        };
        locked
            .try_construct(&mut registrar)
            .map_err(GameInputManagerError::Construction)
    }
    /// `Unregister`825e5710 is a source key lookup/erase, not pointer comparison.
    pub(crate) fn try_unregister(
        &mut self,
        receiver: &GameInputReceiver,
    ) -> Result<(), GameInputManagerError<std::convert::Infallible>> {
        let id = receiver.id();
        if self.receivers.remove(&id).is_none() {
            return Err(GameInputManagerError::MissingRegistration(id));
        }
        Ok(())
    }
    /// Selected numeric part of `Reset`825e5490/Clear82a6e6e0. Membership stays.
    /// Full native `SyncLog` ancillary buffer reset is not claimed.
    pub(crate) fn clear_selected_crc(&mut self) {
        self.crc = 0;
    }
    pub(crate) const fn crc(&self) -> u32 {
        self.crc
    }
    fn log_word(&mut self, value: u32) {
        self.crc = self.crc.wrapping_mul(33);
        self.crc = self.crc.wrapping_add(game_input_crc(&value.to_be_bytes()));
    }
    pub(crate) fn try_do_command<S: GameInputSyncLevel>(
        &mut self,
        command: &EncodedGameInputCommand<'_>,
        add_to_crc: bool,
        source: &mut S,
    ) -> Result<GameInputDispatch, GameInputManagerError<S::Error>> {
        if add_to_crc && source.sync_level().map_err(GameInputManagerError::Source)? >= 2 {
            self.log_word(command.receiver_id);
            self.log_word(command.kind);
            let size = u32::try_from(command.payload.len())
                .map_err(|_| GameInputManagerError::PayloadTooLarge)?;
            self.log_word(size);
            self.log_word(game_input_crc(command.payload));
        }
        if command.receiver_id == SKATER_MANAGER_RECEIVER {
            return Ok(GameInputDispatch::SkaterManager {
                kind: command.kind,
                payload: command.payload.to_vec(),
            });
        }
        let Some(registration) = self.receivers.get(&command.receiver_id) else {
            return Ok(GameInputDispatch::MissingReceiver);
        };
        let player = registration
            .player
            .upgrade()
            .ok_or(GameInputManagerError::PlayerExpired(command.receiver_id))?;
        let mut player = player
            .lock()
            .map_err(|_| GameInputManagerError::PlayerPoisoned)?;
        let decoded = match command.kind {
            1 => {
                // Actual local constructor826df5f8..618, before GetData/Unpack.
                let mut data = PlayerInputData {
                    sample: PublishedInputPadSample {
                        count: 0,
                        values: [0.0; 24],
                    },
                    allow_sensor: false,
                    has_focus: true,
                };
                try_unpack_player_input(&mut data, &mut GameInputRead::new(command.payload))
                    .map_err(GameInputManagerError::Decode)?;
                GameInputCommand::Input(data)
            }
            2 => GameInputCommand::GainFocus,
            kind => GameInputCommand::Unknown(kind),
        };
        player
            .try_on_command(&registration.receiver, decoded)
            .map_err(GameInputManagerError::Player)?;
        Ok(GameInputDispatch::Delivered)
    }
}
/// `GetCRC`829b30e8. Original table821f16c0, all256 words independently verified
/// against polynomial `04c11db7` (SHA-256 `8887a8c13e8406291f39913650f596ab592d817c993486ec86bcb26ed372004b`).
fn game_input_crc(bytes: &[u8]) -> u32 {
    let Some(first) = bytes.get(..4) else {
        return 0;
    };
    let mut crc = !u32::from_be_bytes(first.try_into().expect("four-byte slice"));
    for byte in &bytes[4..] {
        let mut table = (crc >> 24) << 24;
        for _ in 0..8 {
            table = (table << 1)
                ^ if table & 0x8000_0000 != 0 {
                    0x04c1_1db7
                } else {
                    0
                };
        }
        crc = ((crc << 8) | u32::from(*byte)) ^ table;
    }
    let result = !crc;
    if result == 0 { u32::MAX } else { result }
}
#[cfg(test)]
#[path = "game_input_manager_tests.rs"]
mod tests;
