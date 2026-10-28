//! The session marker teleport after `Flow::Manager::SendMessage`. The next
//! `Sk8AppMan::PreUpdate` pumps Flow: `StateMainGameFlow::FilterImpl` (in-game
//! state) calls `StateLoading::DoTeleport`827767e8 with location key -1, whose
//! no-load-screen branch moves the stream focus and `SendTeleportMsg`82758388
//! enqueues SetSpawnStance then Teleport on eEQGameSim. `SimController::Update1`
//! flushes them into `PlayerUI::ReceiveSetSpawnStanceMsg`828aa418 and
//! `ReceiveTeleportMsg`828ad120, which queues Command 5 for receiver 1; the next
//! admitted batch runs `SkaterManager::OnCommand`82600920 case 5.
use super::{FootForward, SessionMarker, player_ui::TeleportToPosition};
use crate::{
    game_input_manager::{EncodedGameInputCommand, SKATER_MANAGER_RECEIVER},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::collections::VecDeque;

/// `Command::mType` for `PlayerTeleportData`; OnCommand switches on `mType - 3`.
pub(crate) const TELEPORT_COMMAND: u32 = 5;
/// `Command::mType` for `PlayerPinData`.
pub(crate) const PIN_COMMAND: u32 = 7;
/// `ePauseState::kPAUSE_STATE_MANUALCAM`.
const PAUSE_STATE_MANUALCAM: u32 = 6;
/// `PlayerTeleportData::Pack`82a8ca40 byte count.
const PACKED_LENGTH: usize = 4 + 4 + 1 + 1 + 12 * 4;

/// `PlayerTeleportData` as carried by the command (Pack82a8ca40/Unpack82a8cb00).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlayerTeleportData {
    pub(crate) where_: SkeletonRootMatrix,
    pub(crate) player_index: i32,
    pub(crate) foot_forward: FootForward,
    pub(crate) on_board: bool,
    pub(crate) session_marker: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TeleportCommandError {
    /// The command payload is not the packed `PlayerTeleportData` length.
    PayloadLength(usize),
    /// `OnCommand` assertion 816 for an unhandled command type.
    UnhandledCommand(u32),
}

impl PlayerTeleportData {
    /// Big-endian ByteStream writes: index, stance, on-board, marker, then
    /// only the xyz of each of the four rows.
    pub(crate) fn pack(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PACKED_LENGTH);
        bytes.extend_from_slice(&self.player_index.to_be_bytes());
        bytes.extend_from_slice(&self.foot_forward.to_be_bytes());
        bytes.push(u8::from(self.on_board));
        bytes.push(u8::from(self.session_marker));
        for column in self.where_.columns() {
            for lane in column.xyz {
                bytes.extend_from_slice(&lane.to_bits().to_be_bytes());
            }
        }
        bytes
    }

    /// `OnCommand` case 5 defaults (identity axes, zero translation, index -1,
    /// RIGHT, on-board, not a marker) then Unpack; every w lane stays zero.
    pub(crate) fn try_unpack(bytes: &[u8]) -> Result<Self, TeleportCommandError> {
        if bytes.len() != PACKED_LENGTH {
            return Err(TeleportCommandError::PayloadLength(bytes.len()));
        }
        let word =
            |offset: usize| u32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("word"));
        let mut lanes = [0.0; 16];
        for row in 0..4 {
            for lane in 0..3 {
                lanes[row * 4 + lane] = f32::from_bits(word(10 + (row * 3 + lane) * 4));
            }
        }
        Ok(Self {
            where_: SkeletonRootMatrix::known(lanes),
            player_index: i32::from_be_bytes(word(0).to_be_bytes()),
            foot_forward: word(4),
            // ReadBytes copies the raw byte; nonzero is true.
            on_board: bytes[8] != 0,
            session_marker: bytes[9] != 0,
        })
    }
}

/// `PlayerPinData` as carried by command 7 (`PlayerUnSpawnData::Pack`/`Unpack`
/// 82a8c7c0/82a8c818, which the pin data shares): the index's four bytes, then
/// the pin byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlayerPinData {
    pub(crate) player_index: i32,
    pub(crate) pin: bool,
}

impl PlayerPinData {
    pub(crate) fn pack(self) -> [u8; 5] {
        let [a, b, c, d] = self.player_index.to_be_bytes();
        [a, b, c, d, u8::from(self.pin)]
    }

    pub(crate) fn try_unpack(bytes: &[u8]) -> Result<Self, TeleportCommandError> {
        let [a, b, c, d, pin] = bytes else {
            return Err(TeleportCommandError::PayloadLength(bytes.len()));
        };
        Ok(Self { player_index: i32::from_be_bytes([*a, *b, *c, *d]), pin: *pin != 0 })
    }
}

/// `SkaterManager` owners used by `OnCommand` cases 5 and 7.
pub(crate) trait SkaterManagerTeleport {
    type Error;
    /// `SkaterManager` virtual +24: the Actor for the command's player index.
    fn has_actor(&mut self, player_index: i32) -> Result<bool, Self::Error>;
    /// `SkaterManager::AdjustSpawnPosition`825f1a20.
    fn adjust_spawn_position(
        &mut self,
        transform: &SkeletonRootMatrix,
    ) -> Result<SkeletonRootMatrix, Self::Error>;
    /// `Actor::SpawnFootForwardStance`825ed638: SkaterAnim spawn stance, then
    /// `mLastSetFootForward`.
    fn spawn_foot_forward_stance(&mut self, foot_forward: FootForward) -> Result<(), Self::Error>;
    /// `Actor::Teleport`825ed1c8.
    fn teleport(
        &mut self,
        transform: &SkeletonRootMatrix,
        on_board: bool,
    ) -> Result<(), Self::Error>;
    /// Case 7 (82600cd8): the index's Actor, when there is one, has `Actor+0x258`
    /// bit `0x04000000` set or cleared.
    fn pin(&mut self, player_index: i32, pin: bool) -> Result<(), Self::Error>;
    /// `SendMsg<cMsgUseMarker>` with the command's player index.
    fn send_use_marker(&mut self, player_index: i32) -> Result<(), Self::Error>;
    fn command(&mut self, error: TeleportCommandError) -> Self::Error;
}

/// `SkaterManager::OnCommand`82600920 for a receiver-1 command. The teleport
/// and pin types have producers in the port; any other type is the native
/// line-816 assertion.
pub(crate) fn try_on_command<S: SkaterManagerTeleport>(
    services: &mut S,
    kind: u32,
    payload: &[u8],
) -> Result<(), S::Error> {
    if kind == PIN_COMMAND {
        let data = PlayerPinData::try_unpack(payload).map_err(|e| services.command(e))?;
        return services.pin(data.player_index, data.pin);
    }
    if kind != TELEPORT_COMMAND {
        return Err(services.command(TeleportCommandError::UnhandledCommand(kind)));
    }
    let data = PlayerTeleportData::try_unpack(payload).map_err(|e| services.command(e))?;
    // 82600b18: no Actor for the index goes straight to UpdateLocalPlayer.
    if !services.has_actor(data.player_index)? {
        return Ok(());
    }
    let transform = services.adjust_spawn_position(&data.where_)?;
    // 82600bcc CensusMan::ClearPoint has no census population in the port.
    services.spawn_foot_forward_stance(data.foot_forward)?;
    services.teleport(&transform, data.on_board)?;
    // 82600c08: a session marker teleport sends cMsgUseMarker; UpdateLocalPlayer
    // keeps the same local Actor.
    if data.session_marker {
        services.send_use_marker(data.player_index)?;
    }
    Ok(())
}

/// PlayerUI owners reached by its eEQGameSim handlers.
pub(crate) trait PlayerUiTeleportReceiver {
    type Error;
    /// `PlayerUI::DoCommand`: `mProxy` (+0xc) vtable +8.
    fn do_command(&mut self, command: &EncodedGameInputCommand<'_>) -> Result<(), Self::Error>;
    /// `PauseMan::Pull`82733df8.
    fn pull_pause(&mut self, state: u32) -> Result<(), Self::Error>;
    /// `PartyPlayUI::SetPlayer`828aa4c0: the UI's player and its controller
    /// (`mControllers[player]`); the player's stance becomes `mFootForward`.
    fn set_player(&mut self, player: i32) -> Result<FootForward, Self::Error>;
}

enum GameSimMessage {
    /// `cMsgSetSpawnStance`8bac4885.
    SetSpawnStance(FootForward),
    /// `cMsgSetPlayer`48d42c2f: `PartyPlayUI::ReceiveSetPlayer`828aa548, sent
    /// only while the UI is a PartyPlayUI.
    SetPlayer(i32),
    /// `cMsgPinPlayer`794a2bc1: `PlayerUI::ReceivePinMsg`828acc18 sends
    /// command 7 for the UI's player.
    Pin(bool),
    /// `cMsgTeleport`8fcdc5e2.
    Teleport {
        transform: SkeletonRootMatrix,
        on_board: bool,
        session_marker: bool,
    },
    /// `cMsgSetSessionMarker`9a96b5f7 from `cChallengeTeleportManager`.
    SetSessionMarker {
        transform: SkeletonRootMatrix,
        foot_forward: FootForward,
    },
    /// `cMsgClearSessionMarker`523ed4cc from `SessionMarker.ClearSessionMarker`82da9350.
    ClearSessionMarker,
    /// `PlayerUI::Update`'s network-game branch (0x828afa98): Command 5 straight from the UI,
    /// to the suggested breadcrumb with its own stance.
    Breadcrumb {
        transform: SkeletonRootMatrix,
        foot_forward: FootForward,
        on_board: bool,
    },
}

/// The native queues between `UpdateSessionMarker` and the command stage.
pub(crate) struct SessionMarkerTeleportQueues {
    /// `Flow::Manager::mMessages`.
    flow: VecDeque<TeleportToPosition>,
    /// eEQGameSim messages from `SendTeleportMsg`.
    game_sim: VecDeque<GameSimMessage>,
    /// PlayerUI+0xa0 `mFootForward`; the constructor stores RIGHT.
    player_ui_foot_forward: FootForward,
    /// Whether the game's UI is a `PartyPlayUI` (`GameModule::ResetGame` 0x8273a680 makes one in
    /// `GM_PartyPlay`), the only UI whose constructor (0x828aeae0) subscribes to `cMsgSetPlayer`.
    party_play_ui: bool,
}

impl SessionMarkerTeleportQueues {
    /// PlayerUI::ReloadAttributeData828aa470 retains IsPlayerStanceRegular
    /// as its FootForward enum in the same owner used by teleport messages.
    pub(crate) fn reload_stance(&mut self,regular:bool){self.player_ui_foot_forward=u32::from(regular);}
    pub(crate) fn construct() -> Self {
        Self {
            flow: VecDeque::new(),
            game_sim: VecDeque::new(),
            player_ui_foot_forward: 0,
            party_play_ui: false,
        }
    }

    /// The UI `ResetGame` makes for the game mode: a `PartyPlayUI` in `GM_PartyPlay`.
    pub(crate) fn set_party_play_ui(&mut self, party_play: bool) {
        self.party_play_ui = party_play;
    }

    /// `Flow::Manager::SendMessage`82576c90 appends under its lock.
    pub(crate) fn send_flow(&mut self, message: TeleportToPosition) {
        self.flow.push_back(message);
    }

    /// `cMsgSetPlayer` on eEQGameSim.
    pub(crate) fn send_set_player(&mut self, player: i32) {
        self.game_sim.push_back(GameSimMessage::SetPlayer(player));
    }

    /// `cMsgPinPlayer` on eEQGameSim.
    pub(crate) fn send_pin(&mut self, pin: bool) {
        self.game_sim.push_back(GameSimMessage::Pin(pin));
    }

    /// `cTeleportManager::TeleportPlayer`828a9888 without a load screen:
    /// `cMsgTeleport` on eEQGameSim, for the UI's player.
    pub(crate) fn send_teleport(&mut self, transform: SkeletonRootMatrix, on_board: bool) {
        self.game_sim.push_back(GameSimMessage::Teleport { transform, on_board, session_marker: false });
    }

    /// `PlayerUI::Update` in a network game with the skater outside the game's boundaries: the
    /// teleport to the suggested breadcrumb, not a session marker's.
    pub(crate) fn send_breadcrumb_teleport(&mut self, transform: SkeletonRootMatrix, foot_forward: FootForward, on_board: bool) {
        self.game_sim.push_back(GameSimMessage::Breadcrumb { transform, foot_forward, on_board });
    }

    /// `cMsgSetSessionMarker` on eEQGameSim (`cChallengeTeleportManager::SetSessionMarker`
    /// 0x82da6e40, `TeleportPlayerNow` 0x82da6a00).
    pub(crate) fn send_set_session_marker(&mut self, transform: SkeletonRootMatrix, foot_forward: FootForward) {
        self.game_sim.push_back(GameSimMessage::SetSessionMarker { transform, foot_forward });
    }

    /// `cMsgClearSessionMarker` on the same broadcaster.
    pub(crate) fn send_clear_session_marker(&mut self) {
        self.game_sim.push_back(GameSimMessage::ClearSessionMarker);
    }

    /// `Flow::Manager::Update` from `Sk8AppMan::PreUpdate`: the ready branch
    /// performs `MoveStreamFocus(wAxis)` then `SendTeleportMsg`. Map travel
    /// waits for collision registration and presentation loading before it
    /// enqueues here; marker requests retain their existing caller policy.
    pub(crate) fn try_pump_flow<E>(
        &mut self,
        mut move_stream_focus: impl FnMut(SkeletonRootVector) -> Result<(), E>,
    ) -> Result<(), E> {
        while let Some(message) = self.flow.pop_front() {
            move_stream_focus(message.transform.columns()[3])?;
            self.game_sim
                .push_back(GameSimMessage::SetSpawnStance(message.foot_forward));
            self.game_sim.push_back(GameSimMessage::Teleport {
                transform: message.transform,
                on_board: message.on_board,
                session_marker: message.session_marker,
            });
        }
        Ok(())
    }

    /// The eEQGameSim flush in `SimController::Update1`: the PlayerUI
    /// handlers. `ReceiveTeleportMsg` skips an all-zero xyz translation;
    /// otherwise it sends Command 5 to receiver 1 through the proxy.
    /// `ReceiveSetSessionMarkerMsg`828aab38 forces the marker when the UI has one.
    pub(crate) fn try_dispatch_game_sim<R: PlayerUiTeleportReceiver>(
        &mut self,
        mut player_index: i32,
        receiver: &mut R,
        mut marker: Option<&mut SessionMarker>,
    ) -> Result<(), R::Error> {
        while let Some(message) = self.game_sim.pop_front() {
            match message {
                GameSimMessage::SetSpawnStance(foot_forward) => {
                    self.player_ui_foot_forward = foot_forward;
                }
                GameSimMessage::SetPlayer(player) => {
                    if self.party_play_ui {
                        self.player_ui_foot_forward = receiver.set_player(player)?;
                        player_index = player;
                    }
                }
                GameSimMessage::Pin(pin) => {
                    receiver.do_command(&EncodedGameInputCommand {
                        receiver_id: SKATER_MANAGER_RECEIVER,
                        kind: PIN_COMMAND,
                        payload: &PlayerPinData { player_index, pin }.pack(),
                    })?;
                }
                GameSimMessage::Teleport {
                    transform,
                    on_board,
                    session_marker,
                } => {
                    let [x, y, z] = transform.columns()[3].xyz;
                    if x == 0.0 && y == 0.0 && z == 0.0 {
                        continue;
                    }
                    let payload = PlayerTeleportData {
                        where_: transform,
                        player_index,
                        foot_forward: self.player_ui_foot_forward,
                        on_board,
                        session_marker,
                    }
                    .pack();
                    receiver.do_command(&EncodedGameInputCommand {
                        receiver_id: SKATER_MANAGER_RECEIVER,
                        kind: TELEPORT_COMMAND,
                        payload: &payload,
                    })?;
                    receiver.pull_pause(PAUSE_STATE_MANUALCAM)?;
                    // 828ad24c: cMsgSetManualCamState(0) goes to Presentation's
                    // broadcaster; the port has no manual camera.
                }
                GameSimMessage::Breadcrumb { transform, foot_forward, on_board } => {
                    let payload = PlayerTeleportData { where_: transform, player_index, foot_forward, on_board, session_marker: false }.pack();
                    receiver.do_command(&EncodedGameInputCommand {
                        receiver_id: SKATER_MANAGER_RECEIVER,
                        kind: TELEPORT_COMMAND,
                        payload: &payload,
                    })?;
                }
                GameSimMessage::SetSessionMarker { transform, foot_forward } => {
                    if let Some(marker) = marker.as_deref_mut() {
                        marker.set_forced(&transform, foot_forward, true);
                    }
                }
                // ReceiveClearSessionMarkerMsg828aa3f0: ClearSessionMarker828aa550 when the UI
                // has a marker.
                GameSimMessage::ClearSessionMarker => {
                    if let Some(marker) = marker.as_deref_mut() {
                        marker.clear_for_world_travel();
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "session_marker_teleport_tests.rs"]
mod tests;
