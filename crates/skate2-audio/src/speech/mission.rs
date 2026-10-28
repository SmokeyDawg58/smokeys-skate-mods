//! `Sk8SpeechSystem`'s mission speech: `PlayMissionSpeech` 0x8250b678 for a challenge's line
//! (`AudioSystem::PlayMissionSpeech` 0x82504ed0 posts `cMsgPlayMissionSpeech`, which
//! `PlayMissionSpeechMsg` 0x8250d0a8 handles in the audio update) or a scene's speech below id
//! 5000 (`AudioSystem::PlayNISSpeech` 0x8250c6a0), `UpdateMissionSpeech` 0x8250bb18, and the
//! watched events through which the challenge system hears `cMsgSpeechEnded` once a line has
//! ended or did not play.
//!
//! A line from id 5000 is a resident NIS sample the voices instance queues and holds: once its
//! stream has buffered it the line starts (`cMsgSpeechStarted`), once its request is done it ends
//! (`cMsgSpeechEnded`). A line below 5000 is the cameraman's, an announcer's, the living world's
//! or the main cast's: [`mapping`] gives its speech event and the context system's trigger
//! function it calls with its parameters (the living world's play through
//! `PlayLivingWorldSpeech`), and the line is watched until a speech event of its speaker
//! finishes (`EventFinished`) or fails to play (`EventNotPlayedImpl`).

#[path = "mission_mapping.rs"]
mod mapping;

use std::collections::VecDeque;

use super::context::Group;
use super::nis::{CHARACTER_CAMERA_MAN, CHARACTER_CUZ, CHARACTER_CYRUS, CHARACTER_DYRDEK, CHARACTER_MUSIC_POST, CHARACTER_SFX_POST};
use super::{CHANNEL_ANNOUNCER, CHANNEL_CAMERA_MAN, CHANNEL_MAIN_CAST, INSTANCE_CHARACTERS, INSTANCE_VOICES, LivingWorldParams, Sk8SpeechSystem, SpeechCtx, lw_event};
use crate::vault::{Collection, Vault};
use crate::world::pedestrian::{self as ped, AUD_PEDESTRIAN};

/// `eSk8Characters`.
const CHARACTER_PLAYER: i32 = 0;
const CHARACTER_UNKNOWN: i32 = 39;
const CHARACTER_MAX: i32 = 100;
/// The most watched events `mWatchedEventList` keeps.
const WATCHED_EVENTS: usize = 10;
/// The cameraman's line after which `mPlay5342` asks for 5342.
const EVENT_BEFORE_5342: i32 = 5340;
/// `Audio_Speech` `kEnableCMSpeech`, `kEnableMCSpeech` and `kEnableANSpeech` at their registered
/// values.
const ENABLE_CM_SPEECH: bool = true;
const ENABLE_MC_SPEECH: bool = true;
const ENABLE_AN_SPEECH: bool = true;
/// The `aud_pedestrian` announcer ids only the announcer's mission speech reads.
const AN_PRO_ID: u64 = 0x14B2_3B45_27AF_919E;
const AN_IP_ID: u64 = 0x90B7_0315_7171_6686;
/// `SPCHType_3_EventID` events whose parameters `ConvertMissionSpeechParameters` 0x8250c530 turns.
const AN_T_403_START: u16 = 24607;
const AN_T_411_NEW_LEAD: u16 = 24611;
const AN_T_413_LEADLATE: u16 = 24627;
const AN_T_425_LEADERBOARD: u16 = 24620;
/// `SPCHType_0_EventID` events whose parameters `ConvertMissionSpeechParameters` 0x824fc2f8 turns.
const MC_T_1202_END_OF_GAME: u16 = 54;
const MC_T_1207_PC_LAND_SK8: u16 = 59;
/// `Attrib::Gen::scoring_trick`, whose leaf collections in order are the trick database
/// (`TrickDatabase::mTrickScoringKeyList`), and the layout word `mIntensityTrick` is at.
const SCORING_TRICK: u64 = 0x6918_4699_84A8_C596;
const SCORING_TRICK_INTENSITY: u32 = 42;

/// `eMissionSpeechStatus`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Status {
    #[default]
    None,
    Loading,
    Playing,
}

/// `Sk8SpeechSystem::tWatchedEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WatchedEvent {
    pub(super) character: i32,
    pub(super) sed_event: u16,
    pub(super) event_id: i32,
    pub(super) instance: i32,
    pub(super) channel: i32,
}

/// `Sk8SpeechSystem::tOutGoingMessage`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OutgoingMessage {
    character: i32,
    event_id: i32,
}

/// What the speech system tells the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechMessage {
    /// `cMsgSpeechStarted` to the challenge system.
    Started { character: i32, id: i32 },
    /// `cMsgSpeechEnded` to the challenge system.
    Ended { character: i32, id: i32 },
    /// `cMsgCallSpeechStarted` to the front end.
    CallStarted { character: i32, id: i32 },
    /// `cMsgCallSpeechEnded` to the front end.
    CallEnded { character: i32, id: i32 },
}

/// The mission speech state of `Sk8SpeechSystem`. `mInterrupting` is never set in this build, so
/// the replay of an interrupted line `UpdateMissionSpeech` would make never happens, and
/// `mCachedEvents` keep the constructor's empty events.
pub(super) struct MissionSpeech {
    /// `mMissionSpeechStatus`.
    status: Status,
    /// `mMissionSpeechCharacter` / `mMissionSpeechId` and `mScriptedMissionSpeechCharacter` /
    /// `mScriptedMissionSpeechId`: the player and -1 without a line.
    character: i32,
    id: i32,
    scripted_character: i32,
    scripted_id: i32,
    /// `mbError`.
    error: bool,
    /// `mLastRaceWinner`.
    last_race_winner: i32,
    /// `mWatchedEventList`, newest first.
    watched: VecDeque<WatchedEvent>,
    /// `mOutGoingWatchedEventList` and `mOutGoingWatchedScriptedEventList`.
    outgoing: VecDeque<OutgoingMessage>,
    outgoing_scripted: VecDeque<OutgoingMessage>,
    /// The messages sent since the game last took them.
    messages: Vec<SpeechMessage>,
}

impl Default for MissionSpeech {
    /// The constructor 0x82508df0's values.
    fn default() -> Self {
        Self {
            status: Status::None,
            character: CHARACTER_PLAYER,
            id: -1,
            scripted_character: CHARACTER_PLAYER,
            scripted_id: -1,
            error: false,
            last_race_winner: CHARACTER_MAX,
            watched: VecDeque::new(),
            outgoing: VecDeque::new(),
            outgoing_scripted: VecDeque::new(),
            messages: Vec::new(),
        }
    }
}

impl MissionSpeech {
    pub(super) fn playing(&self) -> bool {
        self.status == Status::Playing
    }

    /// The line's `cMsgSpeechStarted` and `cMsgCallSpeechStarted`.
    fn send_started(&mut self) {
        self.messages.push(SpeechMessage::Started { character: self.character, id: self.id });
        self.messages.push(SpeechMessage::CallStarted { character: self.scripted_character, id: self.scripted_id });
    }

    /// The line's `cMsgSpeechEnded` and `cMsgCallSpeechEnded`, and no line.
    fn send_ended(&mut self) {
        self.messages.push(SpeechMessage::Ended { character: self.character, id: self.id });
        self.messages.push(SpeechMessage::CallEnded { character: self.scripted_character, id: self.scripted_id });
        self.clear_line();
    }

    fn clear_line(&mut self) {
        self.character = CHARACTER_PLAYER;
        self.scripted_character = CHARACTER_PLAYER;
        self.id = -1;
        self.scripted_id = -1;
    }

    /// `CheckForRaceWinner` 0x824ea8e0: 909 (an AI won the race) remembers its speaker, 910 (the
    /// player won) forgets, and 999 becomes 909 for the remembered winner or else 910.
    fn check_for_race_winner(&mut self, id: &mut i32, character: &mut i32) {
        match *id {
            909 => self.last_race_winner = *character,
            910 => self.last_race_winner = CHARACTER_MAX,
            999 if self.last_race_winner == CHARACTER_MAX => *id = 910,
            999 => {
                *id = 909;
                *character = self.last_race_winner;
                self.last_race_winner = CHARACTER_MAX;
            }
            _ => {}
        }
    }

    /// `PushNotifyOnFinish` 0x8250a350: newest first, the oldest dropped past ten.
    fn push_watched(&mut self, event: WatchedEvent) {
        self.watched.push_front(event);
        if self.watched.len() > WATCHED_EVENTS {
            self.watched.pop_back();
        }
    }

    /// The first watched event of the character with the speech event or the id.
    fn find_watched(&self, character: i32, sed_event: u16, event_id: i32) -> Option<usize> {
        self.watched.iter().position(|w| w.character == character && (w.sed_event == sed_event || w.event_id == event_id))
    }
}

/// `tCameraManParams` (`SPCH2Type_*`, all invalid at 0).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct CameraManParams {
    pub(super) score_comp: i32,
    pub(super) slam_int: i32,
    pub(super) filmer: i32,
    pub(super) lw_gender: i32,
    pub(super) contest_type: i32,
    pub(super) gvr: i32,
    pub(super) which_mag: i32,
    pub(super) sponsor: i32,
    pub(super) photogs: i32,
    pub(super) chall_type: i32,
    pub(super) lw_type: i32,
    pub(super) cloth_type: i32,
    pub(super) pro_id: i32,
    pub(super) trick_level: i32,
    pub(super) ip_id: i32,
}

/// `tMainCastParams` (`SPCH0Type_*`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct MainCastParams {
    pub(super) pro_id: i32,
    pub(super) ip_id: i32,
    pub(super) near_far: i32,
    pub(super) lw_class: i32,
    pub(super) pro_class: i32,
    pub(super) ip_class: i32,
    pub(super) win_lose: i32,
    pub(super) intensity_level: i32,
    pub(super) intensity_trick: i32,
    pub(super) plan_b: i32,
    pub(super) direct: i32,
}

/// `tAnnouncerParams` (`SPCH3Type_*`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct AnnouncerParams {
    pub(super) character: i32,
    pub(super) trick: i32,
    pub(super) pro_id: i32,
    pub(super) trick_val: i32,
    pub(super) obstacle: i32,
    pub(super) style: i32,
    pub(super) slam_int: i32,
    pub(super) round: i32,
    pub(super) gender: i32,
    pub(super) sponsor: i32,
    pub(super) hype: i32,
    pub(super) trick2: i32,
    pub(super) ip_id: i32,
}

/// `kSPCH2_filmer_dyrdek` in a challenge Dyrdek hosts, else `kSPCH2_filmer_Reda`.
fn filmer(host: Option<i32>) -> i32 {
    if host.unwrap_or(CHARACTER_MAX) == CHARACTER_DYRDEK { 1 } else { 2 }
}

/// `GetLWSEDEvent` 0x824eb890: the living-world event of a mission id,
/// `kSPCH1_EventID_MaxEventID` without one.
fn lw_sed_event(id: i32) -> u16 {
    mapping::LIVING_WORLD.iter().find(|&&(row, _)| row == id).map_or(lw_event::MAX, |&(_, event)| event)
}

/// `GetMCCSISEvent` 0x824ebbe0: the main cast's trigger function of a mission id.
fn mc_csis_event(id: i32) -> Option<&'static str> {
    mapping::MAIN_CAST.iter().find(|row| row.id == id).map(|row| row.function)
}

/// `GetCMSEDEventId` 0x824ebd50, `GetANSEDEventId` 0x824eb7b0, `GetLWSEDEventId` 0x824eb8e0 and
/// `GetMCSEDEventId` 0x824ebc30 as `EventNotPlayedImpl` picks them by the character: the speech
/// event of the row whose id is the given value, -1 without one.
fn sed_event_id(character: i32, value: u16) -> i32 {
    let value = i32::from(value);
    let find = |rows: &[mapping::Mapping]| rows.iter().find(|row| row.id == value).map_or(-1, |row| i32::from(row.sed_event));
    if (CHARACTER_CUZ..=CHARACTER_CYRUS).contains(&character) {
        find(&mapping::CONTEST)
    } else if character == CHARACTER_CAMERA_MAN {
        find(&mapping::CAMERA_MAN)
    } else if character >= CHARACTER_UNKNOWN {
        mapping::LIVING_WORLD.iter().find(|&&(row, _)| row == value).map_or(-1, |&(_, event)| i32::from(event))
    } else {
        find(&mapping::MAIN_CAST)
    }
}

impl Sk8SpeechSystem {
    /// `GetSpeechCharacteristicsKey` 0x824ec7a8: the `aud_pedestrian` collection of the
    /// character's `aud_speech` entry (its key at byte 40).
    pub(super) fn speech_characteristics(vault: &Vault, character: i32) -> Collection {
        let entry = vault.bytes(super::SPEECH_DEFAULT, super::CHARACTER_ENTRIES, character.clamp(0, CHARACTER_MAX - 1) as u32, 0x38);
        Collection::new(AUD_PEDESTRIAN, u64::from_be_bytes(entry[40..48].try_into().expect("eight bytes")))
    }

    /// `AudioSystem::PlayMissionSpeechMsg` 0x8250d0a8: an additional parameter of -2 makes the
    /// line an incoming call (the cell phone speaks it), and 1207 plays as 1211 while
    /// `mSketchyLand` counts down.
    pub fn play_mission_speech_msg(&mut self, character: i32, id: i32, additional_parameter: i32, ctx: &mut SpeechCtx<'_>) {
        if additional_parameter == -2 {
            self.incoming_call = true;
        }
        let id = if id == 1207 && self.sketchy_land > 0 { 1211 } else { id };
        self.play_mission_speech(character, id, false, additional_parameter, ctx);
    }

    /// `AudioSystem::CameraManCsisEventHandler` 0x824ff7a0: `PlayCameraManSpeech` of the message's
    /// trigger with its parameters, here all unset.
    pub fn camera_man_csis_event_msg(&mut self, function: &str, ctx: &mut SpeechCtx<'_>) {
        self.play_camera_man_speech(function, &mut CameraManParams::default(), ctx);
    }

    /// A scene's line below id 5000 (`AudioSystem::PlayNISSpeech` 0x8250c6a0): `PlayMissionSpeech`
    /// from a NIS with parameter 0.
    pub fn play_scene_mission_speech(&mut self, character: i32, id: i32, ctx: &mut SpeechCtx<'_>) {
        self.play_mission_speech(character, id, true, 0, ctx);
    }

    /// `PlayMissionSpeech` 0x8250b678. From id 5000 the line is the character's NIS sample on the
    /// voices instance's stream 0, queued and held (`mQueueSpeech`, `mHoldOnFinish`), unless a line
    /// is loading or playing; a line not found or not taken sets `mbError`. Below 5000 the line is
    /// the cameraman's (also Dyrdek's in a challenge he hosts), an announcer's, an unknown
    /// speaker's (`FindCharacterToSpeak`), the living world's for a character with a
    /// living-world class, else the main cast's.
    pub(super) fn play_mission_speech(&mut self, character: i32, event_id: i32, from_nis: bool, param: i32, ctx: &mut SpeechCtx<'_>) -> bool {
        if event_id >= 5000 {
            let mut played = true;
            if self.mission.status != Status::Playing {
                if self.mission.status == Status::None {
                    self.queue_speech(INSTANCE_VOICES, 0, true);
                    self.set_hold_on_finish(INSTANCE_VOICES, 0, true);
                    self.mission.character = character;
                    self.mission.id = event_id;
                    self.mission.scripted_character = character;
                    self.mission.scripted_id = event_id;
                    match self.nis_sample(character, event_id) {
                        Some(sample) => {
                            self.stop_all_speech_on_instance(INSTANCE_VOICES, ctx);
                            self.play_nis_sample(INSTANCE_VOICES, 0, &sample, event_id, ctx);
                            self.mission.status = Status::Loading;
                        }
                        None => played = false,
                    }
                } else {
                    played = false;
                }
            }
            self.mission.error = !played;
            return played;
        }
        let (mut id, mut character) = (event_id, character);
        self.mission.check_for_race_winner(&mut id, &mut character);
        let host = self.game.challenge_host;
        if character == CHARACTER_CAMERA_MAN || (host == Some(character) && character == CHARACTER_DYRDEK) {
            self.play_camera_man_mission_speech(id, param, ctx);
        } else if character == CHARACTER_CUZ || character == CHARACTER_CYRUS {
            self.play_announcer_mission_speech(character, id, param, ctx);
        } else if character == CHARACTER_UNKNOWN {
            self.play_unknown_speaker_mission_speech(id, from_nis, ctx);
        } else if ctx.vault.i32(Self::speech_characteristics(ctx.vault, character), ped::MC_LW_CLASS) != 0 {
            self.play_living_world_mission_speech(character, id, ctx);
        } else {
            self.play_main_cast_mission_speech(character, id, param, ctx);
        }
        true
    }

    /// The unknown speaker's line (`eSk8Character_Unknown`): `FindCharacterToSpeak` picks one of
    /// the attached NIS characters (`CSTATEMGR_NISCharacter`, for a scene's line) or skaters
    /// (`CSTATEMGR_Player`); neither state attaches in the port, so the speaker stays unknown with
    /// id -1 and empty parameters. A speaker with a living-world class plays the line's
    /// living-world event, else the main cast's trigger.
    fn play_unknown_speaker_mission_speech(&mut self, id: i32, _from_nis: bool, ctx: &mut SpeechCtx<'_>) {
        let (speaker, speaker_id) = (CHARACTER_UNKNOWN, -1i32);
        let (lw_params, mc_params) = (LivingWorldParams::default(), MainCastParams::default());
        if ctx.vault.i32(Self::speech_characteristics(ctx.vault, speaker), ped::MC_LW_CLASS) != 0 {
            let event = lw_sed_event(id);
            if event != lw_event::MAX {
                self.play_living_world_speech(ctx, speaker, speaker_id as u32, event, &lw_params, 0, false);
            }
        } else if let Some(function) = mc_csis_event(id)
            && ENABLE_MC_SPEECH
        {
            self.play_main_cast_csis_event(function, &mc_params, ctx);
        }
    }

    /// `PlayCameraManMissionSpeech` 0x8250adb0: the mapped line calls its trigger with the
    /// filmer (Dyrdek in a challenge he hosts, else Reda) and the converted parameter; the line is
    /// watched on the voices instance, and one not mapped did not play.
    fn play_camera_man_mission_speech(&mut self, id: i32, param: i32, ctx: &mut SpeechCtx<'_>) {
        let mut watched = WatchedEvent { character: CHARACTER_CAMERA_MAN, sed_event: 0, event_id: id, instance: INSTANCE_VOICES as i32, channel: CHANNEL_CAMERA_MAN };
        let row = mapping::CAMERA_MAN.iter().find(|row| row.id == id).copied();
        if let Some(row) = row {
            let mut params = CameraManParams { filmer: filmer(self.game.challenge_host), ..CameraManParams::default() };
            watched.sed_event = row.sed_event;
            convert_camera_man_parameters(row.sed_event, param, &mut params);
            self.play_camera_man_speech(row.function, &mut params, ctx);
        }
        self.mission.push_watched(watched);
        if row.is_none() {
            self.event_not_played(&watched);
        }
    }

    /// `PlayCameraManSpeech` 0x824fc3a0: with the cameraman's speech enabled, an unset filmer
    /// becomes the challenge's, then `PlayCameraManCsisEvent`.
    pub(super) fn play_camera_man_speech(&mut self, function: &str, params: &mut CameraManParams, ctx: &mut SpeechCtx<'_>) {
        if !ENABLE_CM_SPEECH {
            return;
        }
        if params.filmer == 0 {
            params.filmer = filmer(self.game.challenge_host);
        }
        let args = camera_man_csis_args(function, params);
        self.trigger_context_event(Group::CameraMan, function, &args, ctx);
    }

    /// AudioSystem::PlayerOverallResultHandler825007b8: leader's IP/pro ids,
    /// stop voices stream0, then T_426_results or T_445_brd_no_change.
    pub fn leaderboard_overall(&mut self, character: i32, unchanged: bool, ctx: &mut SpeechCtx<'_>) {
        let leader = Self::speech_characteristics(ctx.vault, character);
        let mut params = AnnouncerParams {
            ip_id: ctx.vault.i32(leader, AN_IP_ID),
            pro_id: ctx.vault.i32(leader, AN_PRO_ID),
            ..AnnouncerParams::default()
        };
        self.stop_speech_on_instance_stream(INSTANCE_VOICES, 0, ctx);
        self.play_announcer_speech(if unchanged { "T_445_brd_no_change" } else { "T_426_results" }, &mut params, ctx);
    }

    /// `PlayAnnouncerMissionSpeech` 0x8250a618: the mapped line takes the speaker's announcer id
    /// (`SPCH3Type_char_ID_Ann` at `aud_pedestrian` 0x6F9C8A27_E4CD37DC), its converted
    /// parameter, and calls its trigger; the line is watched on the voices instance, and one not
    /// mapped did not play.
    fn play_announcer_mission_speech(&mut self, character: i32, id: i32, param: i32, ctx: &mut SpeechCtx<'_>) {
        let mut watched = WatchedEvent { character, sed_event: 0, event_id: id, instance: INSTANCE_VOICES as i32, channel: CHANNEL_ANNOUNCER };
        let row = mapping::CONTEST.iter().find(|row| row.id == id).copied();
        if let Some(row) = row {
            watched.sed_event = row.sed_event;
            let mut params = AnnouncerParams { character: ctx.vault.i32(Self::speech_characteristics(ctx.vault, character), ped::AN_CHAR_ID), ..AnnouncerParams::default() };
            self.convert_announcer_parameters(row.sed_event, param, &mut params, ctx);
            self.play_announcer_speech(row.function, &mut params, ctx);
        }
        self.mission.push_watched(watched);
        if row.is_none() {
            self.event_not_played(&watched);
        }
    }

    /// `ConvertMissionSpeechParameters` 0x8250c530 (announcer): a contest start also asks for the
    /// unknown speaker's 400; a new leader's parameter is that leader, whose pro and IP ids the
    /// line takes, and who says 401; the leaderboard's is the round (bit `param - 1`).
    fn convert_announcer_parameters(&mut self, sed_event: u16, param: i32, params: &mut AnnouncerParams, ctx: &mut SpeechCtx<'_>) {
        match sed_event {
            AN_T_403_START => {
                self.play_mission_speech(CHARACTER_UNKNOWN, 400, false, 0, ctx);
            }
            AN_T_411_NEW_LEAD | AN_T_413_LEADLATE => {
                if param > -1 {
                    let leader = Self::speech_characteristics(ctx.vault, param);
                    params.pro_id = ctx.vault.i32(leader, AN_PRO_ID);
                    params.ip_id = ctx.vault.i32(leader, AN_IP_ID);
                    self.play_mission_speech(param, 401, false, 0, ctx);
                }
            }
            AN_T_425_LEADERBOARD => params.round = if param - 1 >= 0 { 1 << (param - 1) } else { 0 },
            _ => {}
        }
    }

    /// `PlayAnnouncerSpeech` 0x824fbf00: with the announcers' speech enabled, an unset announcer
    /// becomes the challenge host's (`AN_Char_ID`), then `PlayAnnouncerCsisEvent` 0x824eb808.
    pub(super) fn play_announcer_speech(&mut self, function: &str, params: &mut AnnouncerParams, ctx: &mut SpeechCtx<'_>) {
        if !ENABLE_AN_SPEECH {
            return;
        }
        if params.character == 0
            && let Some(host) = self.game.challenge_host
            && host < CHARACTER_MAX
        {
            params.character = ctx.vault.i32(Self::speech_characteristics(ctx.vault, host), ped::AN_CHAR_ID);
        }
        let p = *params;
        let args = [p.trick2, p.hype, p.sponsor, p.gender, p.round, p.slam_int, p.style, p.obstacle, p.pro_id, p.trick_val, p.trick, p.character, p.ip_id];
        self.trigger_context_event(Group::Announcer, function, &args, ctx);
    }

    /// `PlayLivingWorldMissionSpeech` 0x82503698: a mapped line of an attached skater
    /// (`CSTATEMGR_Player::GetAttatchedSkaterID`, no skater speech states in the port) or of the
    /// first pedestrian state of the character (`GetAttatchedPedestrianID` 0x82550188) plays its
    /// living-world event for that speaker on stream 0. It is not watched.
    fn play_living_world_mission_speech(&mut self, character: i32, id: i32, ctx: &mut SpeechCtx<'_>) {
        let Some(&(_, event)) = mapping::LIVING_WORLD.iter().find(|&&(row, _)| row == id) else { return };
        let Some((pedestrian_id, lw_params, _)) = self.attached_pedestrian(character) else { return };
        self.play_living_world_speech(ctx, character, pedestrian_id, event, &lw_params, 0, false);
    }

    /// `CSTATEMGR_Pedestrian::GetAttatchedPedestrianID` 0x82550188: the first state of the
    /// character, attached or not, with its living-world and main-cast parameters (near while
    /// within its near/far distance); none when its pedestrian id is 0.
    fn attached_pedestrian(&self, character: i32) -> Option<(u32, LivingWorldParams, MainCastParams)> {
        let state = self.pedestrian_states.iter().find(|state| i64::from(state.character) == i64::from(character))?;
        let near = state.distance <= state.near_far_distance;
        let lw =
            LivingWorldParams { char_type: state.char_type, char_id: state.char_id, near_far: if near { 2 } else { 1 }, lw_class: state.lw_class, ..LivingWorldParams::default() };
        let mc = MainCastParams {
            pro_id: state.mc_pro_id as i32,
            ip_id: state.mc_ip_id as i32,
            near_far: if near { 1 } else { 2 },
            lw_class: state.mc_lw_class as i32,
            pro_class: state.mc_pro_class as i32,
            ip_class: state.mc_ip_class as i32,
            ..MainCastParams::default()
        };
        (state.pedestrian_id != 0).then_some((state.pedestrian_id, lw, mc))
    }

    /// `PlayMainCastMissionSpeech` 0x8250aa58: a mapped line takes its converted parameter and
    /// the speaker's main-cast parameters (an attached pedestrian's, else the character's
    /// `aud_pedestrian` ids and classes) and calls its trigger; the line is watched on the
    /// characters instance, and one not mapped did not play.
    fn play_main_cast_mission_speech(&mut self, character: i32, id: i32, param: i32, ctx: &mut SpeechCtx<'_>) {
        let mut watched = WatchedEvent { character, sed_event: 0, event_id: id, instance: INSTANCE_CHARACTERS as i32, channel: CHANNEL_MAIN_CAST };
        if let Some(row) = mapping::MAIN_CAST.iter().find(|row| row.id == id).copied() {
            let mut params = MainCastParams::default();
            convert_main_cast_parameters(row.sed_event, param, &mut params, ctx.vault);
            watched.sed_event = row.sed_event;
            match self.attached_pedestrian(character) {
                Some((_, _, mc)) => params = MainCastParams { win_lose: params.win_lose, intensity_trick: params.intensity_trick, ..mc },
                None => {
                    let collection = Self::speech_characteristics(ctx.vault, character);
                    params.ip_id = ctx.vault.i32(collection, ped::MC_IP_ID);
                    params.ip_class = ctx.vault.i32(collection, ped::MC_IP_CLASS);
                    params.pro_class = ctx.vault.i32(collection, ped::MC_PRO_CLASS);
                    params.pro_id = ctx.vault.i32(collection, ped::MC_PRO_ID);
                    params.lw_class = ctx.vault.i32(collection, ped::MC_LW_CLASS);
                }
            }
            if ENABLE_MC_SPEECH {
                self.play_main_cast_csis_event(row.function, &params, ctx);
            }
        }
        self.mission.push_watched(watched);
        if watched.sed_event == 0 {
            self.event_not_played(&watched);
        }
    }

    /// `PlayMainCastSpeech` 0x824fc2d8: with the main cast's speech enabled,
    /// `PlayMainCastCsisEvent`.
    pub(super) fn play_main_cast_speech(&mut self, function: &str, params: &MainCastParams, ctx: &mut SpeechCtx<'_>) {
        if ENABLE_MC_SPEECH {
            self.play_main_cast_csis_event(function, params, ctx);
        }
    }

    /// `PlayMainCastCsisEvent` 0x824ebc88: the trigger's parameter words, an unset intensity
    /// level a random one of the three (`1 << Random() % 3`).
    fn play_main_cast_csis_event(&mut self, function: &str, params: &MainCastParams, ctx: &mut SpeechCtx<'_>) {
        let intensity_level = if params.intensity_level == 0 { 1 << (ctx.rng.next_u32() % 3) } else { params.intensity_level };
        let p = params;
        let args = [p.ip_id, p.pro_id, p.lw_class, p.pro_class, 0, p.ip_class, p.near_far, p.win_lose, intensity_level, p.intensity_trick, p.plan_b, p.direct];
        self.trigger_context_event(Group::MainCast, function, &args, ctx);
    }

    /// `UpdateMissionSpeech` 0x8250bb18: a loading line starts once its stream has buffered it
    /// (`cMsgSpeechStarted`, `cMsgCallSpeechStarted`; the stream stops queueing); a playing line
    /// ends once it is not held or its request is done (`cMsgSpeechEnded`, `cMsgCallSpeechEnded`;
    /// the stream is released). Without a line, an error answers the failed one at once.
    pub(super) fn update_mission_speech(&mut self, ctx: &mut SpeechCtx<'_>) {
        match self.mission.status {
            Status::Loading => {
                if self.speech_queued(INSTANCE_VOICES, 0, ctx.core) {
                    self.mission.send_started();
                    self.start_queued_speech(INSTANCE_VOICES, 0, ctx.core);
                    self.queue_speech(INSTANCE_VOICES, 0, false);
                    self.mission.status = Status::Playing;
                }
            }
            Status::Playing => {
                if !self.hold_on_finish(INSTANCE_VOICES, 0) || self.is_request_done(INSTANCE_VOICES, 0, ctx.core) {
                    let (character, id, scripted_character, scripted_id) = (self.mission.character, self.mission.id, self.mission.scripted_character, self.mission.scripted_id);
                    self.mission.messages.push(SpeechMessage::Ended { character, id });
                    self.mission.messages.push(SpeechMessage::CallEnded { character: scripted_character, id: scripted_id });
                    self.release_held(INSTANCE_VOICES, 0);
                    self.set_hold_on_finish(INSTANCE_VOICES, 0, false);
                    self.mission.status = Status::None;
                    self.mission.clear_line();
                }
            }
            Status::None => {
                if self.mission.error {
                    self.queue_speech(INSTANCE_VOICES, 0, false);
                    self.set_hold_on_finish(INSTANCE_VOICES, 0, false);
                    self.mission.send_started();
                    self.mission.send_ended();
                    self.mission.error = false;
                }
            }
        }
    }

    /// `EventFinished` 0x825077f0 (through `EventFinished(VoiceInfo*)` 0x825086a0 for a stream
    /// with a speaker): an event with an id answers the newest watched event of its character with
    /// its speech event or id (`cMsgSpeechEnded`, enqueued at once), else any mission line but for
    /// a post (`cMsgSpeechEnded`, `cMsgCallSpeechEnded`); 5340 asks for the cameraman's 5342.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn event_finished(&mut self, sed_event: u16, event_id: i32, _instance: i32, _channel: i32, _stream: i32, character: i32, _speaker_id: u32) {
        if event_id <= 0 {
            return;
        }
        match self.mission.find_watched(character, sed_event, event_id) {
            Some(index) => {
                let watched = self.mission.watched.remove(index).expect("found");
                self.mission.messages.push(SpeechMessage::Ended { character: watched.character, id: watched.event_id });
            }
            None => {
                if character != CHARACTER_SFX_POST
                    && character != CHARACTER_MUSIC_POST
                    && self.mission.character != CHARACTER_PLAYER
                    && self.mission.scripted_character != CHARACTER_PLAYER
                {
                    self.mission.send_ended();
                }
            }
        }
        if event_id == EVENT_BEFORE_5342 {
            self.play_5342 = true;
        }
    }

    /// `EventNotPlayedImpl` 0x8250a3d8: the newest watched event of the character with the
    /// speech event or id is answered by the next `SendOutGoingMessages` (an id of -1 found from
    /// the speech event by the speaker's mapping); without one, any mission line but for a post
    /// is.
    pub(super) fn event_not_played(&mut self, unplayed: &WatchedEvent) {
        if let Some(index) = self.mission.find_watched(unplayed.character, unplayed.sed_event, unplayed.event_id) {
            let watched = self.mission.watched.remove(index).expect("found");
            let event_id = if unplayed.event_id == -1 { sed_event_id(unplayed.character, unplayed.sed_event) } else { unplayed.event_id };
            self.mission.outgoing.push_back(OutgoingMessage { character: watched.character, event_id });
            return;
        }
        if unplayed.character != CHARACTER_SFX_POST
            && unplayed.character != CHARACTER_MUSIC_POST
            && self.mission.character != CHARACTER_PLAYER
            && self.mission.scripted_character != CHARACTER_PLAYER
        {
            self.mission.outgoing.push_back(OutgoingMessage { character: self.mission.character, event_id: self.mission.id });
            self.mission.outgoing_scripted.push_back(OutgoingMessage { character: self.mission.scripted_character, event_id: self.mission.scripted_id });
            self.mission.clear_line();
        }
    }

    /// `SendOutGoingMessages` 0x82507680: the first message of each outgoing list
    /// (`cMsgSpeechEnded` to the challenge system, `cMsgCallSpeechEnded` to the front end).
    pub(super) fn send_outgoing_messages(&mut self) {
        if let Some(message) = self.mission.outgoing.pop_front() {
            self.mission.messages.push(SpeechMessage::Ended { character: message.character, id: message.event_id });
        }
        if let Some(message) = self.mission.outgoing_scripted.pop_front() {
            self.mission.messages.push(SpeechMessage::CallEnded { character: message.character, id: message.event_id });
        }
    }

    /// The messages sent since the last call, in order.
    pub fn take_messages(&mut self) -> Vec<SpeechMessage> {
        std::mem::take(&mut self.mission.messages)
    }
}

/// `ConvertMissionSpeechParameters` 0x824ebda8 (cameraman): the parameter is the score
/// comparison of 705, the contest type of 4017, the GVR of 4025, the magazine of 4030, 4039 and
/// 4040, the sponsor of 4036 and the clothing of 841.
fn convert_camera_man_parameters(sed_event: u16, param: i32, params: &mut CameraManParams) {
    match sed_event {
        32774 => params.score_comp = param,
        32960 => params.contest_type = param,
        32962 => params.gvr = param,
        32966 | 32975 | 32976 => params.which_mag = param,
        32972 => params.sponsor = param,
        33047 => params.cloth_type = param,
        _ => {}
    }
}

/// `ConvertMissionSpeechParameters` 0x824fc2f8 (main cast): the end of a game's parameter is
/// win or lose; a landed trick's is its trick index (`TrickScoringSettings`), whose
/// `scoring_trick` intensity (layout word 42) the line takes.
fn convert_main_cast_parameters(sed_event: u16, param: i32, params: &mut MainCastParams, vault: &Vault) {
    match sed_event {
        MC_T_1202_END_OF_GAME => params.win_lose = if param != 0 { 1 } else { 2 },
        MC_T_1207_PC_LAND_SK8 => {
            let trick = usize::try_from(param).ok().and_then(|index| vault.leaf_collections(SCORING_TRICK).get(index).copied()).unwrap_or(0);
            params.intensity_trick = vault.layout_word(Collection::new(SCORING_TRICK, trick), SCORING_TRICK_INTENSITY) as i32;
        }
        _ => {}
    }
}

/// `PlayCameraManCsisEvent` 0x824ebf60: the parameter words each cameraman trigger takes.
fn camera_man_csis_args(function: &str, p: &CameraManParams) -> Vec<i32> {
    match function {
        "T_101_pos_cam" => vec![p.trick_level],
        "T_315_SktPstLos" | "T_314_SktPstWin" => vec![p.pro_id, p.ip_id],
        "T_1401_flw_Far" | "T_1403_flw_Lst" | "T_150_pro_pos_cam" => vec![p.pro_id],
        "T_841_NewCloth" => vec![p.cloth_type],
        "T_811_NPC_Knck" | "T_604_Chsewrn" | "T_609_Catch" => vec![p.lw_type],
        "T_4085_Done" => vec![p.chall_type],
        "T_4037_3rdArtCmp" | "T_4038_3magcmp" | "T_4045_Circ" | "T_1301_PhtSuc" => vec![p.photogs],
        "T_4036_AcpTeam" => vec![p.sponsor],
        "T_4025_BowlGVR" => vec![p.gvr],
        "T_4017_1stBrndCont" => vec![p.contest_type],
        "T_4030_1stMagArticle" | "T_4039_Cover" | "T_4040_othermag" => vec![p.which_mag],
        "T_701_40_start" | "T_707_40_lose_time" | "T_711_40_almost" | "T_721_40_LW_warn" | "T_730_40_setup" | "T_712_replay" | "T_713_timewarn" | "T_702_40_retry" => {
            vec![p.filmer]
        }
        "T_715_40_collide_lw" => vec![p.filmer, p.lw_gender],
        "T_705_40_win" => vec![p.score_comp, p.filmer],
        "T_706_40_lose_slam" => vec![p.filmer, 4],
        "T_104_slam_cam" => vec![p.slam_int],
        "T_4999_In_Prox" => vec![p.ip_id, p.pro_id, p.filmer],
        _ => Vec::new(),
    }
}

/// Every mapped line's trigger, its context system and the parameter words the port passes.
#[cfg(test)]
pub(super) fn mapped_triggers() -> Vec<(Group, &'static str, usize)> {
    let camera_man = mapping::CAMERA_MAN.iter().map(|row| (Group::CameraMan, row.function, camera_man_csis_args(row.function, &CameraManParams::default()).len()));
    let main_cast = mapping::MAIN_CAST.iter().map(|row| (Group::MainCast, row.function, 12));
    let contest = mapping::CONTEST.iter().map(|row| (Group::Announcer, row.function, 13));
    camera_man.chain(main_cast).chain(contest).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mappings_hold_the_training_lines() {
        let camera_man = |id: i32| mapping::CAMERA_MAN.iter().find(|row| row.id == id).map(|row| (row.sed_event, row.function));
        assert_eq!(camera_man(701), Some((32768, "T_701_40_start")));
        assert_eq!(camera_man(2727), Some((32843, "T_2727_SpnsrMeRmd")));
        assert_eq!(camera_man(4004), Some((32953, "T_4004_Rmd_Atiba")));
        assert_eq!(mc_csis_event(101), Some("T_101_pos"));
        assert_eq!(lw_sed_event(601), 8192);
        assert_eq!(lw_sed_event(1), lw_event::MAX);
    }

    #[test]
    fn race_winner_carries_to_999() {
        let mut mission = MissionSpeech::default();
        let (mut id, mut character) = (999, 0);
        mission.check_for_race_winner(&mut id, &mut character);
        assert_eq!((id, character), (910, 0));
        let (mut id, mut character) = (909, 29);
        mission.check_for_race_winner(&mut id, &mut character);
        let (mut id, mut character) = (999, 0);
        mission.check_for_race_winner(&mut id, &mut character);
        assert_eq!((id, character, mission.last_race_winner), (909, 29, CHARACTER_MAX));
    }

    #[test]
    fn watched_events_keep_the_newest_ten() {
        let mut mission = MissionSpeech::default();
        for id in 0..12u16 {
            mission.push_watched(WatchedEvent { character: CHARACTER_CAMERA_MAN, sed_event: 100 + id, event_id: i32::from(id), instance: 1, channel: CHANNEL_CAMERA_MAN });
        }
        assert_eq!(mission.watched.len(), WATCHED_EVENTS);
        assert_eq!(mission.watched.front().map(|w| w.event_id), Some(11));
        assert_eq!(mission.find_watched(CHARACTER_CAMERA_MAN, 0, 5), Some(6));
        assert_eq!(mission.find_watched(CHARACTER_CAMERA_MAN, 104, -1), Some(7));
        assert_eq!(mission.find_watched(CHARACTER_CAMERA_MAN, 0, 1), None);
    }
}
