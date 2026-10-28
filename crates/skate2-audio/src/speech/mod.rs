//! `Sk8::Audio::Sk8SpeechSystem` (over `Sk8::Audio::SpeechSystem`): the speech projects, the
//! `spch` library that picks their samples ([`spch`]), the speech instances that stream them
//! ([`instance`]), the resident NIS and post samples the scenes play ([`nis`]), the mission
//! speech the challenges ask for ([`mission`]), and the game-side rules for when the living
//! world, the main cast, the cameraman and the announcer speak.
//!
//! `AudioSystem::Initialize` builds it (`SpeechSystem::Initialize` 0x82d40ce8: three instances
//! with their voices, the 'Spch' stream pool, `SPCH_Init`), `LoadInGameData` loads the front-end
//! posts (`LoadEarlyResidentData`), `SecondaryLoad` 0x824fe178 the English projects (instance 0:
//! main cast on channel 0, living world on 1; instance 1: cameraman on 2, announcer on 3) and the
//! NIS and post samples (`LoadResidentData`), and `AudioSystem::Update` runs `Update` 0x8250d438
//! in group A after the music system.
//!
//! The game reaches the cameraman, announcer and main-cast projects through their context
//! systems ([`context`]): it calls a trigger function (`T_*`), and the system that picks a
//! context calls that context's output function (`O_*`), which adds the project's `spch` event.

mod context;
mod instance;
mod mission;
mod nis;
pub mod pedestrian;
pub mod spch;

pub use context::{GameValues, game_time};
pub use mission::SpeechMessage;
pub use nis::CHARACTER_SFX_POST;

use std::sync::Arc;

use crate::nicotine::MixMap;
use crate::random::RwRandom;
use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
use crate::rwac::stream_pool::StreamFiles;
use crate::rwac::PluginRef;
use crate::vault::{class, Collection, Vault};
use context::{ContextSystems, Group};
use instance::SpeechInstance;
use spch::{EventSpec, Spch};

/// `eSpeechChannels`.
pub const CHANNEL_MAIN_CAST: i32 = 0;
pub const CHANNEL_LIVING_WORLD: i32 = 1;
pub const CHANNEL_CAMERA_MAN: i32 = 2;
pub const CHANNEL_ANNOUNCER: i32 = 3;
/// `eSpeechInstances`: the characters (two distance-based streams), the voices (one stereo
/// stream) and the post (one 6-channel stream).
const INSTANCES: usize = 3;
const INSTANCE_CHARACTERS: usize = 0;
const INSTANCE_VOICES: usize = 1;
const INSTANCE_POST: usize = 2;
/// `eSpeechStreams`: two per instance; 2 (`eNumStreamsPerInstance`) means none.
const STREAMS: usize = 2;
/// The pedestrian states (`CSTATEMGR_Pedestrian::EnterGamePlay`).
pub const PEDESTRIAN_STATES: usize = 15;
/// `eSk8Character_Max` (the `mLWTimeBetweenEvents` rows) and the living-world events
/// (`gLWSortedSpeechEventArray` 0x820159e0).
const CHARACTERS: usize = 100;
const LW_EVENTS: [u16; 61] = [
    0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200A, 0x200B, 0x200C, 0x200D, 0x200E, 0x200F, 0x2010, 0x2011,
    0x2012, 0x2013, 0x2014, 0x2015, 0x2016, 0x2017, 0x2018, 0x2019, 0x201A, 0x201B, 0x201C, 0x2020, 0x2023, 0x2024, 0x2025, 0x2026, 0x2027, 0x2028,
    0x2029, 0x202A, 0x202B, 0x202C, 0x202D, 0x202E, 0x202F, 0x2030, 0x2031, 0x2033, 0x2034, 0x2035, 0x2037, 0x2038, 0x203A, 0x204D, 0x204E, 0x204F,
    0x2050, 0x2051, 0x2052, 0x2053, 0x2054, 0x2055, 0x2056,
];
/// `SPCH_Init`'s data rate: `SPCH_GetSampleDataRate(22050, 16, kSPCH_Compression_None)`.
const DATA_RATE: i32 = spch::sample_data_rate(22050, 16);
/// `StreamPool::CreateInstance('Spch', 3, 0x40000, ...)`.
const STREAM_POOL: u32 = 0x5370_6368;
const STREAM_POOL_STREAMS: usize = 3;
const STREAM_POOL_BYTES: u64 = 0x4_0000;

/// `aud_speech` "default" (`GlobalAttributeCollection::aud_speech_default`, `mSpeechAttrib`), its
/// per-character entries (0x38 bytes: the tuning key at +16, the characteristics key at +40) and
/// the pedestrian voice attributes `UpdatePedVoiceInfo` reads.
pub const SPEECH_DEFAULT: Collection = Collection::new(class::AUD_SPEECH, 0xD7ED_BD36_2D7D_2152);
const CHARACTER_ENTRIES: u64 = 0x60CA_5F2E_959E_C361;
/// `aud_speechtuning` and the per-channel `{u16 event, ..., u64 event key at +16}` arrays
/// (`Num_MC/LW/CM/AN_Event_Tuning`), and the event key a channel falls back to.
const AUD_SPEECHTUNING: u64 = 0x2278_258F_AE60_31C1;
const EVENT_TUNING: [u64; 4] = [0x355D_0465_9873_929C, 0xC3E2_421F_FA83_0BB6, 0x2822_6E0C_C3C0_3F13, 0x10A8_410E_4030_7891];
const DEFAULT_EVENT_KEY: [u64; 5] = [0xF2C1_FBBC_D885_7533, 0xF97B_9086_CAA5_0C53, 0xC455_0055_EBDD_05B5, 0x3BC7_B323_7690_50E9, 0xD7ED_BD36_2D7D_2152];
/// `aud_speechevent` (`tSpeechTuning`, 0x18 bytes of layout).
const AUD_SPEECHEVENT: u64 = 0x816B_9DBE_D74E_FCDF;
/// `Audio_Speech` `kEnableLWSpeech` at its registered value.
const ENABLE_LW_SPEECH: bool = true;
/// The `aud_speech` "default" volume arrays `UpdateVoiceInfo` scales the announcer (in a large
/// arena or not) and the cameraman by.
const ANNOUNCER_LARGE_ARENA_VOL: u64 = 0x60E0_221F_D3D6_F04B;
const ANNOUNCER_VOL: u64 = 0x49AE_841B_E63F_9EB7;
const CAMERA_MAN_VOL: u64 = 0xF0B8_2620_0D2E_52A4;
/// The cameraman's events that stop his stream, and all speech, when they start
/// (`GetVoiceInfoForSample`).
const CM_EVENT_STOP_STREAM: u16 = 33055;
const CM_EVENTS_STOP_ALL: [u16; 2] = [33147, 32938];

/// `Sk8::Audio::VoiceInfo` (0x5C bytes): what a speaking object tells the speech instance
/// playing for it, and what the instance reports back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceInfo {
    pub speaker_id: u32,
    pub character: i32,
    pub vol: i32,
    pub azimuth: i32,
    pub pitch: i32,
    pub high_pass: i32,
    pub low_pass: i32,
    pub wet_vol: i32,
    pub notch_freq: f32,
    pub notch_gain: f32,
    pub notch_q: f32,
    pub delay: f32,
    pub common_event_id: i32,
    pub sed_event: u16,
    pub channel: i32,
    pub instance: i32,
    pub is_attached: bool,
    pub is_far: bool,
    pub instance_stream: i32,
    pub is_actually_playing: bool,
    pub delay_vol: i32,
    pub delay_high_pass: i32,
    pub delay_low_pass: i32,
    pub is_queueing: bool,
    pub is_voiceover: bool,
}

impl Default for VoiceInfo {
    /// The values `Sk8SpeechSystem`'s constructor 0x82508df0 and `SFXObj_PedestrianSpeech`'s
    /// give their voice infos.
    fn default() -> Self {
        Self {
            speaker_id: 0,
            character: -1,
            vol: 0,
            azimuth: 0,
            pitch: 0,
            high_pass: 0,
            low_pass: 96000,
            wet_vol: 0,
            notch_freq: 96000.0,
            notch_gain: 1.0,
            notch_q: 3.0,
            delay: 0.0,
            common_event_id: -1,
            sed_event: 0,
            channel: -1,
            instance: -1,
            is_attached: false,
            is_far: false,
            instance_stream: -1,
            is_actually_playing: false,
            delay_vol: 0,
            delay_high_pass: 0,
            delay_low_pass: 96000,
            is_queueing: false,
            is_voiceover: false,
        }
    }
}

/// Where a `VoiceInfo` lives: the speech system's own (`AnnouncerVoiceInfo` ... `MainCastVOVoiceInfo`)
/// or a pedestrian state's `SFXObj_PedestrianSpeech`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceInfoRef {
    Announcer,
    CameraMan,
    Post,
    Nis,
    NisPhotographer,
    CellPhone,
    MainCastVo,
    Pedestrian(usize),
}

/// What `CSTATEMGR_Pedestrian::GetPedestrianVoiceInfoFromState` 0x82553ee0 / 0x82553f30 and
/// `GetAttatchedPedestrianID` 0x82550188 read of a pedestrian state's `PedAudioState`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PedestrianSpeechState {
    pub attached: bool,
    pub pedestrian_id: u32,
    pub character: u32,
    /// `charSpchId`, `lw_class` and `type`.
    pub char_id: u32,
    pub lw_class: u32,
    pub char_type: u32,
    pub mc_pro_id: u32,
    pub mc_ip_id: u32,
    pub mc_pro_class: u32,
    pub mc_ip_class: u32,
    pub mc_lw_class: u32,
    pub distance: f32,
    pub near_far_distance: f32,
}

/// `tGernericSpeechTuningSys`: an `aud_speechevent` layout (`tSpeechTuning`) and its event.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SpeechTuning {
    delay: f32,
    interruptable: bool,
    interrupt: bool,
    interrupt_stream: bool,
    priority: i32,
    frequency: f32,
    min_time_between_events: f32,
    is_voice_over: bool,
    event_id: u16,
}

impl Default for SpeechTuning {
    fn default() -> Self {
        Self {
            delay: 0.0,
            interruptable: false,
            interrupt: false,
            interrupt_stream: false,
            priority: 0,
            frequency: 100.0,
            min_time_between_events: 0.0,
            is_voice_over: false,
            event_id: 0,
        }
    }
}

/// `Sk8SpeechSystem::tPedestrianSpeechInfo`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PedestrianSpeechInfo {
    pub pedestrian_id: u32,
    pub current_volume: i32,
    pub current_distance: f32,
}

/// What the speech system works with during an update.
pub struct SpeechCtx<'a> {
    pub core: &'a mut AudioCore,
    pub vault: &'a Vault,
    /// `rw::math::Random`.
    pub rng: &'a mut RwRandom,
    pub master_sub_mix: PluginRef,
    /// `Sk8SpeechSystem::GetReverbVoice` 0x824fbad0: `GlobalEffects::mpEnvSendSubPlugInPtrs[0]`.
    pub reverb_sub_mix: PluginRef,
}

/// The game state `Update` sees, kept for the objects that speak later in the same audio update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeechGameState {
    /// AudioSystem's latched tChaseAudio_Data, UpdatePresentationData82504848.
    pub chase_flags: u8,
    /// `Sk8SpeechSystem::GetTick` 0x824fa4e0: `Simulation::mTick`.
    pub tick: u32,
    /// `CanPlayLivingWorldSpeech` 0x824fc0a8's inputs: a `Presentation` exists and is not in a
    /// replay, and the game is not online. `AudioCore +0x50` (sample capture) never runs and every
    /// mode the port plays is one of the allowed `GameMode`s.
    pub presentation_live: bool,
    pub online_game: bool,
    /// `AudioSystem::mChallengeType`.
    pub challenge_type: i32,
    /// `AudioSystem::mCurrentChallenge`'s host (layout word 0, an `eSk8Characters`); none without
    /// a current challenge.
    pub challenge_host: Option<i32>,
    /// `AudioSystem::GetCurrentChallengeLargeArena` 0x824e3938: the current challenge's attribute
    /// 0xF819DAE7_4AA347B5.
    pub challenge_large_arena: bool,
}

impl Default for SpeechGameState {
    /// The `AudioSystem` constructor's challenge type, `eChallengeTypes_Unknown`.
    fn default() -> Self {
        Self { chase_flags: 0, tick: 0, presentation_live: false, online_game: false, challenge_type: CHALLENGE_UNKNOWN, challenge_host: None, challenge_large_arena: false }
    }
}

/// `eChallengeTypes_Unknown`.
const CHALLENGE_UNKNOWN: i32 = 52;

/// The `SFXObj_CameraMan` / `SFXObj_Announcer` fields `ProcessUpdate` 0x82515218 reads from their
/// mixer outputs (+0x1C..+0x30).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpeakerObject {
    /// Volume output 2.
    pub vol: i32,
    /// Azimuth output 0 and pitch output 1.
    pub azimuth: i32,
    pub pitch: i32,
    /// Frequency outputs 4 and 3.
    pub high_pass: i32,
    pub low_pass: i32,
    /// Volume output 5.
    pub wet_vol: i32,
}

/// The `SFXObj_NIS` fields `UpdateVoiceInfo` reads: `mScriptedSpeechVol`, `mScriptedSpeechPitch`,
/// `GetNISVol` for the current scene, `mNisPhotographerVol`, `mCellPhoneVol` and `mNISProVol`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NisSpeechObject {
    pub scripted_speech_vol: i32,
    pub scripted_speech_pitch: i32,
    pub nis_vol: i32,
    pub nis_photographer_vol: i32,
    pub cell_phone_vol: i32,
    pub nis_pro_vol: i32,
}

/// What `UpdateVoiceInfo` 0x824fbb48 reads through `pAnnouncerSFXObj`, `pCameraManSFXObj` and
/// `pNISSFXObj`; none until the MAIN state made the object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpeechObjects {
    pub announcer: Option<SpeakerObject>,
    pub camera_man: Option<SpeakerObject>,
    pub nis: Option<NisSpeechObject>,
}

/// `Sk8::Audio::Sk8SpeechSystem` (0x6718 bytes).
pub struct Sk8SpeechSystem {
    spch: Spch,
    instances: [SpeechInstance; INSTANCES],
    /// The mixes the instances' voices send to.
    sub_mixes: instance::SubMixes,
    files: Option<Arc<dyn StreamFiles>>,
    /// `mLWTimeBetweenEvents[character][event]`: when each character last played each
    /// living-world event, in audio-core seconds (single precision).
    lw_time_between_events: Vec<[f32; LW_EVENTS.len()]>,
    announcer: VoiceInfo,
    camera_man: VoiceInfo,
    post: VoiceInfo,
    nis: VoiceInfo,
    nis_photographer: VoiceInfo,
    cell_phone: VoiceInfo,
    main_cast_vo: VoiceInfo,
    /// `SFXObj_PedestrianSpeech::mVoiceInfo` of each pedestrian state, and what the speech system
    /// looks the states up by.
    pedestrian_voice_infos: [VoiceInfo; PEDESTRIAN_STATES],
    pedestrian_states: [PedestrianSpeechState; PEDESTRIAN_STATES],
    /// `mbIncomingCall`.
    incoming_call: bool,
    /// `mCurrentTuningParams[channel][stream]`.
    current_tuning: [[SpeechTuning; STREAMS]; 4],
    /// `mSpeakingPedInfo`.
    speaking_ped_info: [PedestrianSpeechInfo; STREAMS],
    game: SpeechGameState,
    /// The resident NIS samples and the scenes' playing ones.
    nis_samples: nis::NisSpeech,
    /// The mission speech, its watched events and their messages.
    mission: mission::MissionSpeech,
    /// `mPlay5342`: event 5340 finished, so the cameraman's 5342 follows.
    play_5342: bool,
    /// `mSketchyLand`: `AudioSystem::AirCompletedHandler` 0x82505b40 sets it to 10 on a human
    /// player's sketchy landing in a main challenge (`cMsgAirCompleted`, which the port's audio
    /// does not receive) and each update counts it down; while positive, mission speech 1207
    /// plays as 1211.
    sketchy_land: i32,
    /// `Sk8ContextSysMan`'s context systems, once `AudioSystem::Initialize` made them.
    contexts: Option<ContextSystems>,
    /// `mContextCommands[channel].wasChannelActuallySpeaking`.
    channel_was_speaking: [bool; 4],
}

impl Sk8SpeechSystem {
    /// The constructor 0x82508df0 and `SpeechSystem::Initialize` 0x82d40ce8: the speech master
    /// voice (`AudioCore::CreateSpeechMasterVoice` 0x82a7d820: "Speech Master" at stage 200, its
    /// VU meter not ported), the instances and their voices (`GetVoiceStyle`) and `SPCH_Init`
    /// seeded with `game_seed` ([`time64_seed`]). The 'Spch' stream pool comes later
    /// ([`Self::create_stream_pool`]).
    pub fn new(core: &mut AudioCore, master_sub_mix: PluginRef, reverb_sub_mix: PluginRef, game_seed: u32) -> Self {
        let speech_master = core.create_voice(
            200,
            "Speech Master",
            &[
                PlugInConfig { kind: PlugInKind::SubMix { name: "Speech Master".into() }, output_channels: 6 },
                PlugInConfig { kind: PlugInKind::Send, output_channels: 6 },
            ],
        );
        if let Some(voice) = speech_master {
            core.connect_send(PluginRef { voice, index: 1 }, Some(master_sub_mix));
        }
        let speech_master_sub_mix = speech_master.map(|voice| PluginRef { voice, index: 0 });
        let sub_mixes = instance::SubMixes { speech_master: speech_master_sub_mix, master: master_sub_mix, reverb: reverb_sub_mix };
        let instances = [0, 1, 2].map(|index| SpeechInstance::new(index, &sub_mixes, core));
        Self {
            spch: Spch::new(game_seed, DATA_RATE),
            instances,
            sub_mixes,
            files: None,
            lw_time_between_events: vec![[0.0; LW_EVENTS.len()]; CHARACTERS],
            announcer: VoiceInfo { is_attached: true, ..VoiceInfo::default() },
            camera_man: VoiceInfo { is_attached: true, ..VoiceInfo::default() },
            post: VoiceInfo { is_attached: true, ..VoiceInfo::default() },
            nis: VoiceInfo { is_attached: true, ..VoiceInfo::default() },
            nis_photographer: VoiceInfo { is_attached: true, ..VoiceInfo::default() },
            cell_phone: VoiceInfo { is_attached: true, ..VoiceInfo::default() },
            main_cast_vo: VoiceInfo { is_attached: true, ..VoiceInfo::default() },
            pedestrian_voice_infos: [VoiceInfo::default(); PEDESTRIAN_STATES],
            pedestrian_states: [PedestrianSpeechState::default(); PEDESTRIAN_STATES],
            incoming_call: false,
            current_tuning: [[SpeechTuning::default(); STREAMS]; 4],
            speaking_ped_info: [PedestrianSpeechInfo::default(); STREAMS],
            game: SpeechGameState::default(),
            nis_samples: nis::NisSpeech::default(),
            mission: mission::MissionSpeech::default(),
            play_5342: false,
            sketchy_land: 0,
            contexts: None,
            channel_was_speaking: [false; 4],
        }
    }

    /// `AudioCore::GetSpeechMasterSubMix` 0x82a7d8e0.
    #[must_use]
    pub fn speech_master_sub_mix(&self) -> Option<PluginRef> {
        self.sub_mixes.speech_master
    }

    /// `SpeechInstance::SetSubMix` 0x82d3e6f0 of an instance's stream.
    pub fn set_instance_sub_mix(&self, instance: usize, sub_mix: Option<PluginRef>, stream: usize, core: &mut AudioCore) {
        if let Some(this) = self.instances.get(instance)
            && stream < STREAMS
        {
            this.set_sub_mix(sub_mix, stream, core);
        }
    }

    /// `SpeechSystem::CreateInstanceDelayVoice` 0x82d402a8: each stream's delay voice.
    pub fn create_instance_delay_voice(&mut self, instance: usize, core: &mut AudioCore) {
        let sub_mixes = self.sub_mixes;
        if let Some(this) = self.instances.get_mut(instance) {
            for stream in 0..STREAMS {
                this.create_delay_voice(stream, &sub_mixes, core);
            }
        }
    }

    /// `SpeechSystem::DestroyInstanceDelayVoice` 0x82d40308.
    pub fn destroy_instance_delay_voice(&mut self, instance: usize, core: &mut AudioCore) {
        if let Some(this) = self.instances.get_mut(instance) {
            for stream in 0..STREAMS {
                this.destroy_delay_voice(stream, core);
            }
        }
    }

    /// `SpeechSystem::Initialize` 0x82d40ce8's 'Spch' stream pool, made where
    /// `AudioSystem::Initialize` 0x82509c18 initializes the speech system (0x82509fbc). Its
    /// streams read the host's files, so the pool is made once they are set.
    pub fn create_stream_pool(core: &mut AudioCore) {
        core.stream_pools.create(STREAM_POOL, STREAM_POOL_STREAMS, STREAM_POOL_BYTES);
    }

    /// `AudioSystem::Initialize` 0x82509c18's context systems (`CreateContextSystem` and
    /// `kPostLoad` for "Sk8CameraMan", "Sk8Announcer" and "Sk8MainCast", seeded with the time
    /// base) at audio time `now` (ms), through the host's files.
    pub fn create_context_systems(&mut self, seed: u32, now: u32) {
        if let Some(files) = self.files.clone() {
            self.contexts = Some(ContextSystems::load(files.as_ref(), seed, now));
        }
    }

    /// The host's files (`rw::core::filesys`) the projects, resident samples and streams read.
    pub fn set_files(&mut self, files: Option<Arc<dyn StreamFiles>>) {
        self.files = files;
    }

    /// `AudioSystem::SecondaryLoad` 0x824fe178's `SpeechInstance::Load` calls for `language`
    /// ("english"): instance 0 takes the main cast (channel 0) and the living world (1),
    /// instance 1 the cameraman (2) and the announcer (3); then `LoadResidentData`.
    pub fn load_projects(&mut self, language: &str) {
        let projects = [(0, CHANNEL_MAIN_CAST, "maincast"), (0, CHANNEL_LIVING_WORLD, "livingworld"), (1, CHANNEL_CAMERA_MAN, "cameraman"), (1, CHANNEL_ANNOUNCER, "announcer")];
        for (instance, channel, project) in projects {
            self.load(instance, channel, project, language);
        }
        self.load_resident_data(language);
    }

    pub(crate) fn voice_info(&self, at: VoiceInfoRef) -> &VoiceInfo {
        match at {
            VoiceInfoRef::Announcer => &self.announcer,
            VoiceInfoRef::CameraMan => &self.camera_man,
            VoiceInfoRef::Post => &self.post,
            VoiceInfoRef::Nis => &self.nis,
            VoiceInfoRef::NisPhotographer => &self.nis_photographer,
            VoiceInfoRef::CellPhone => &self.cell_phone,
            VoiceInfoRef::MainCastVo => &self.main_cast_vo,
            VoiceInfoRef::Pedestrian(state) => &self.pedestrian_voice_infos[state],
        }
    }

    pub(crate) fn voice_info_mut(&mut self, at: VoiceInfoRef) -> &mut VoiceInfo {
        match at {
            VoiceInfoRef::Announcer => &mut self.announcer,
            VoiceInfoRef::CameraMan => &mut self.camera_man,
            VoiceInfoRef::Post => &mut self.post,
            VoiceInfoRef::Nis => &mut self.nis,
            VoiceInfoRef::NisPhotographer => &mut self.nis_photographer,
            VoiceInfoRef::CellPhone => &mut self.cell_phone,
            VoiceInfoRef::MainCastVo => &mut self.main_cast_vo,
            VoiceInfoRef::Pedestrian(state) => &mut self.pedestrian_voice_infos[state],
        }
    }

    /// What the pedestrian manager reports of state `index` after its update.
    pub fn set_pedestrian_state(&mut self, index: usize, state: PedestrianSpeechState) {
        if let Some(slot) = self.pedestrian_states.get_mut(index) {
            *slot = state;
        }
    }

    /// `mSpeakingPedInfo[stream]`.
    pub fn speaking_ped_info_mut(&mut self, stream: usize) -> Option<&mut PedestrianSpeechInfo> {
        self.speaking_ped_info.get_mut(stream)
    }

    /// `Update` 0x8250d438: the cameraman's 5342 after 5340, the mission speech, the NIS slots,
    /// `SpeechSystem::Update`, `UpdateVoiceInfo`, the music override's countdown
    /// (`XMPRestoreBackgroundMusic` has no user music to restore), the context systems' interrupt
    /// modes, one outgoing message of each list and the `mSketchyLand` countdown.
    pub fn update(&mut self, ctx: &mut SpeechCtx<'_>, game: SpeechGameState, objects: &SpeechObjects) {
        self.game = game;
        self.spch.tick = game.tick;
        if self.play_5342 {
            self.play_mission_speech(nis::CHARACTER_CAMERA_MAN, 5342, false, 0, ctx);
            self.play_5342 = false;
        }
        self.update_mission_speech(ctx);
        self.update_playing_nis_speech();
        self.speech_system_update(ctx);
        self.update_voice_info(ctx.vault, objects);
        if self.nis_samples.xmp_count > 0 {
            self.nis_samples.xmp_count -= 1;
        }
        self.update_context_systems();
        self.send_outgoing_messages();
        if self.sketchy_land > 0 {
            self.sketchy_land -= 1;
        }
    }

    /// `UpdateVoiceInfo` 0x824fbb48: the announcer's and cameraman's voices follow their objects,
    /// scaled by the speaker's `aud_speech` volume (the announcer's per large arena, Cuz at
    /// index 0; the cameraman's, Dyrdek at index 0); the NIS, post, photographer, cell phone and
    /// main-cast voice-over voices take `SFXObj_NIS`'s volumes, unfiltered and centred.
    fn update_voice_info(&mut self, vault: &Vault, objects: &SpeechObjects) {
        if let Some(object) = objects.announcer {
            let key = if self.game.challenge_large_arena { ANNOUNCER_LARGE_ARENA_VOL } else { ANNOUNCER_VOL };
            let scale = vault.f32_at(SPEECH_DEFAULT, key, u32::from(self.announcer.character != nis::CHARACTER_CUZ));
            let info = &mut self.announcer;
            info.azimuth = object.azimuth;
            info.pitch = object.pitch;
            info.vol = crate::nicotine::shape::fctiwz(scale * object.vol as f32);
            info.high_pass = object.high_pass;
            info.low_pass = object.low_pass;
            info.wet_vol = object.wet_vol;
        }
        if let Some(object) = objects.camera_man {
            let scale = vault.f32_at(SPEECH_DEFAULT, CAMERA_MAN_VOL, u32::from(self.camera_man.character != nis::CHARACTER_DYRDEK));
            let info = &mut self.camera_man;
            info.azimuth = object.azimuth;
            info.pitch = object.pitch;
            info.vol = crate::nicotine::shape::fctiwz(scale * object.vol as f32);
            info.high_pass = object.high_pass;
            info.low_pass = object.low_pass;
            info.wet_vol = object.wet_vol;
        }
        if let Some(object) = objects.nis {
            let set = |info: &mut VoiceInfo, vol: i32, pitch: i32| {
                info.azimuth = 0;
                info.pitch = pitch;
                info.vol = vol;
                info.high_pass = 0;
                info.low_pass = 25000;
                info.wet_vol = 0;
            };
            set(&mut self.nis, object.scripted_speech_vol, object.scripted_speech_pitch);
            set(&mut self.post, object.nis_vol, 4096);
            set(&mut self.nis_photographer, object.nis_photographer_vol, object.scripted_speech_pitch);
            set(&mut self.cell_phone, object.cell_phone_vol, object.scripted_speech_pitch);
            set(&mut self.main_cast_vo, object.nis_pro_vol, object.scripted_speech_pitch);
        }
    }

    /// `UpdateContextSystems` 0x824fb928: while a channel speaks, its context system only takes
    /// interrupting contexts (`kInterruptsOn` as it starts, `kInterruptsOff` as it stops): the
    /// announcer's and the cameraman's voices actually playing, and the main cast's when both
    /// character streams actually play (`UpdateContextSystem` 0x824fb7b0).
    fn update_context_systems(&mut self) {
        let characters = &self.instances[INSTANCE_CHARACTERS];
        let main_cast = (0..STREAMS).all(|stream| characters.speech_actually_playing(stream));
        let channels = [
            (CHANNEL_ANNOUNCER, Group::Announcer, self.announcer.is_actually_playing),
            (CHANNEL_CAMERA_MAN, Group::CameraMan, self.camera_man.is_actually_playing),
            (CHANNEL_MAIN_CAST, Group::MainCast, main_cast),
        ];
        for (channel, group, speaking) in channels {
            let was_speaking = std::mem::replace(&mut self.channel_was_speaking[channel as usize], speaking);
            if speaking != was_speaking
                && let Some(contexts) = self.contexts.as_mut()
            {
                contexts.set_interrupts_only(group, speaking);
            }
        }
    }

    /// `Sk8ContextSysMan::Update` (`ContextSystemManager::Update` 0x82aaf8b8), which
    /// `AudioSystem::Update` runs after this system's update: each context system picks and
    /// emits its context, the output adding its event to `spch`.
    pub fn update_contexts(&mut self, ctx: &mut SpeechCtx<'_>, values: &GameValues) {
        if let Some(contexts) = self.contexts.as_mut() {
            contexts.update(game_time(ctx.core.system_time), values, ctx.rng, &mut self.spch);
        }
    }

    /// `ScriptedSpeechPlaying` 0x824eb478: the mission speech is playing.
    #[must_use]
    pub fn scripted_speech_playing(&self) -> bool {
        self.mission.playing()
    }

    // ---- the living world -------------------------------------------------------------------

    /// `Utils::ConvertSpeechEventToIndex` 0x824ea888: the event's place in
    /// `gLWSortedSpeechEventArray` (0 when it is not there).
    fn lw_event_index(event: u16) -> usize {
        let (mut lo, mut hi) = (0i32, 60i32);
        loop {
            let mid = (hi + lo) / 2;
            let value = LW_EVENTS[mid as usize];
            if event <= value {
                if event >= value {
                    return ((hi + lo) / 2) as usize;
                }
                hi = mid - 1;
            } else {
                lo = mid + 1;
            }
            if lo > hi {
                return 0;
            }
        }
    }

    /// `GetSpeechTuningKey` 0x824ec840: the `aud_speechtuning` key of the character's entry.
    fn speech_tuning_key(vault: &Vault, character: i32) -> u64 {
        let entry = vault.bytes(SPEECH_DEFAULT, CHARACTER_ENTRIES, character.clamp(0, CHARACTERS as i32 - 1) as u32, 0x38);
        u64::from_be_bytes(entry[16..24].try_into().expect("eight bytes"))
    }

    /// `GetSpeechEventKey` 0x824fb138: the event's `aud_speechevent` key (at +16) in the tuning
    /// collection's array for the channel (entries of 32 bytes matched on the low half of their
    /// first word), else the channel's default key.
    fn speech_event_key(vault: &Vault, channel: i32, event: u16, tuning_key: u64) -> u64 {
        let Some(&array) = EVENT_TUNING.get(channel as usize) else {
            return DEFAULT_EVENT_KEY[4];
        };
        let tuning = Collection::new(AUD_SPEECHTUNING, tuning_key);
        for index in 0..vault.length(tuning, array) {
            let entry = vault.bytes(tuning, array, index, 0x20);
            if u16::from_be_bytes([entry[2], entry[3]]) == event {
                return u64::from_be_bytes(entry[16..24].try_into().expect("eight bytes"));
            }
        }
        DEFAULT_EVENT_KEY[channel as usize]
    }

    /// The `aud_speechevent` layout of an event key.
    fn speech_event(vault: &Vault, key: u64) -> SpeechTuning {
        let collection = Collection::new(AUD_SPEECHEVENT, key);
        let word = |i: u32| vault.layout_word(collection, i);
        let byte = |offset: u32| (word(offset / 4) >> (24 - 8 * (offset % 4))) as u8;
        SpeechTuning {
            delay: f32::from_bits(word(0)),
            interruptable: byte(4) != 0,
            interrupt: byte(5) != 0,
            interrupt_stream: byte(6) != 0,
            priority: word(2) as i32,
            frequency: f32::from_bits(word(3)),
            min_time_between_events: f32::from_bits(word(4)),
            is_voice_over: byte(20) != 0,
            event_id: 0,
        }
    }

    /// `UpdateTuningData` 0x824fafe0: the stream's tuning becomes the event's (priority and
    /// frequency at most 100).
    fn update_tuning_data(&mut self, vault: &Vault, channel: i32, stream: usize, event: u16, key: u64) {
        let mut tuning = Self::speech_event(vault, key);
        tuning.priority = tuning.priority.min(100);
        if tuning.frequency > 100.0 {
            tuning.frequency = 100.0;
        }
        tuning.event_id = event;
        self.current_tuning[channel as usize][stream] = tuning;
    }

    /// `CheckTimeBetweenEvents` 0x824ec8d8.
    fn check_time_between_events(&self, core: &AudioCore, character: i32, event: u16, min_time: f32) -> bool {
        let Some(row) = usize::try_from(character).ok().and_then(|c| self.lw_time_between_events.get(c)) else { return false };
        core.system_time - f64::from(row[Self::lw_event_index(event)]) > f64::from(min_time)
    }

    /// `UpdateEventTime` 0x824ec990.
    fn update_event_time(&mut self, core: &AudioCore, character: i32, event: u16) {
        if let Some(row) = usize::try_from(character).ok().and_then(|c| self.lw_time_between_events.get_mut(c)) {
            row[Self::lw_event_index(event)] = core.system_time as f32;
        }
    }

    /// `ShouldPlayEvent` 0x824eb328: `Random() % 1000 / 10` below the frequency, or at 99.9.
    fn should_play_event(rng: &mut RwRandom, frequency: f32) -> bool {
        let draw = (rng.next_u32() % 1000) as f32 * 0.1;
        f64::from(draw) < f64::from(frequency) || f64::from(draw) >= f64::from(99.900_002_f32)
    }

    /// `SpeechStreamFreeOnInstance` 0x824eac78: no stream of the instance actually playing.
    fn speech_stream_free_on_instance(&self, instance: usize) -> bool {
        (0..STREAMS).all(|stream| !self.instances[instance].speech_actually_playing(stream))
    }

    /// `CheckForInterrupt` 0x824fb538: an interrupting event of higher priority stops the
    /// stream's interruptable speech, or (when both character streams are busy and it interrupts
    /// streams) the stream it would take. The current tuning is `mCurrentTuningParams` read flat
    /// at `channel * 2 + stream`, so the no-stream value 2 reads the next channel's first stream.
    fn check_for_interrupt(&mut self, ctx: &mut SpeechCtx<'_>, event_key: u64, channel: i32, stream: i32, is_speaking: bool) {
        let event = Self::speech_event(ctx.vault, event_key);
        let flat = (channel * 2 + stream).clamp(0, 7) as usize;
        let current = self.current_tuning[flat / 2][flat % 2];
        let outranked = current.interruptable && current.priority < event.priority;
        if event.interrupt && outranked && is_speaking {
            if (0..2).contains(&stream) {
                self.stop_speech_on_instance_stream(INSTANCE_CHARACTERS, stream as usize, ctx);
            }
            return;
        }
        if !self.speech_stream_free_on_instance(INSTANCE_CHARACTERS) && event.interrupt_stream && outranked {
            if (0..2).contains(&stream) {
                self.stop_speech_on_instance_stream(INSTANCE_CHARACTERS, stream as usize, ctx);
            } else if stream >= 2 {
                self.stop_speech_on_instance_stream(INSTANCE_CHARACTERS, 1, ctx);
            }
        }
    }

    /// `StopSpeechByChannel` 0x824fb668 (`eSpeechStreams` 2 and -1 stop nothing).
    pub fn stop_speech_by_channel(&mut self, channel: i32, stream: i32, ctx: &mut SpeechCtx<'_>) {
        if !(0..2).contains(&stream) {
            return;
        }
        match channel {
            CHANNEL_MAIN_CAST => self.stop_speech_on_instance_stream(0, stream as usize, ctx),
            CHANNEL_LIVING_WORLD => {
                self.stop_speech_on_instance_stream(0, stream as usize, ctx);
                if let Some(info) = self.speaking_ped_info.get_mut(stream as usize) {
                    info.pedestrian_id = 0;
                    info.current_volume = 0;
                }
            }
            CHANNEL_CAMERA_MAN | CHANNEL_ANNOUNCER => self.stop_speech_on_instance_stream(1, stream as usize, ctx),
            _ => {}
        }
    }

    /// `PlayLWSEDEvent` 0x824eb930: the living-world event (project 1, dat 0) with the character
    /// type and id and one more parameter (near/far, class or topic by event).
    fn play_lw_sed_event(&mut self, pedestrian_id: u32, event: u16, params: &LivingWorldParams) {
        let spec = EventSpec::new(1, 0, event);
        let third = match event {
            0x2000 | 0x2001 | 0x2002 | 0x200A | 0x200C | 0x200F | 0x2012 | 0x201C => Some(params.near_far),
            0x2014 | 0x2015 => Some(params.lw_class),
            0x204D..=0x2054 => Some(params.topic),
            _ => None,
        };
        match third {
            Some(value) => self.spch.add_smart_event(pedestrian_id, spec, &[params.char_type, params.char_id, value]),
            None => self.spch.add_smart_event(pedestrian_id, spec, &[params.char_type, params.char_id]),
        };
    }

    /// `PlayLivingWorldSpeech` 0x824fc198: allowed in this game state, past the character's
    /// minimum time since it last played the event and through the event's frequency draw, the
    /// event may interrupt and is queued for the pedestrian.
    #[allow(clippy::too_many_arguments)]
    pub fn play_living_world_speech(&mut self, ctx: &mut SpeechCtx<'_>, character: i32, pedestrian_id: u32, event: u16, params: &LivingWorldParams, stream: i32, is_speaking: bool) {
        if !(self.game.presentation_live && !self.game.online_game) {
            return;
        }
        let tuning_key = Self::speech_tuning_key(ctx.vault, character);
        let event_key = Self::speech_event_key(ctx.vault, CHANNEL_LIVING_WORLD, event, tuning_key);
        let tuning = Self::speech_event(ctx.vault, event_key);
        if !self.check_time_between_events(ctx.core, character, event, tuning.min_time_between_events) {
            return;
        }
        if !Self::should_play_event(ctx.rng, tuning.frequency) {
            return;
        }
        self.check_for_interrupt(ctx, event_key, CHANNEL_LIVING_WORLD, stream, is_speaking);
        self.play_lw_sed_event(pedestrian_id, event, params);
        self.update_event_time(ctx.core, character, event);
    }

    /// `PlayPedestrianSpeechSample` 0x82503848: the living-world event a pedestrian audio state
    /// plays (some also send the cameraman a trigger); the event, or `kSPCH1_EventID_MaxEventID`.
    /// The chase flags come from AudioSystem's retained presentation data.
    #[allow(clippy::too_many_arguments)]
    pub fn play_pedestrian_speech_sample(&mut self, ctx: &mut SpeechCtx<'_>, character: i32, params: &LivingWorldParams, state: u32, pedestrian_id: u32, stream: i32, is_speaking: bool, player_collision: bool) -> u16 {
        use lw_event as e;
        let mut event = e::MAX;
        if !ENABLE_LW_SPEECH {
            return event;
        }
        let chase_flags = self.game.chase_flags;
        match state {
            1 | 3 => {
                if params.char_type == CHAR_TYPE_SECURITY_GUARD {
                    event = e::RADIO;
                } else if params.char_type == CHAR_TYPE_BUM {
                    event = match ctx.rng.next_u32() % 3 {
                        1 => e::PAN_HANDLE,
                        2 => e::RANDOM_BUM,
                        _ => e::SHOUT,
                    };
                } else {
                    return event;
                }
            }
            2 => event = e::SHOUT,
            4 | 5 => event = e::GASP,
            6 => {
                event = e::IMPACT_REACT;
                if player_collision {
                    self.play_camera_man_lw_speech("T_715_40_collide_lw", params, ctx);
                }
            }
            9 => {
                if player_collision {
                    self.play_camera_man_lw_speech("T_811_NPC_Knck", params, ctx);
                    return 8279;
                }
                return event;
            }
            10 => event = if ctx.rng.next_u32() & 1 == 1 { e::CONCERN } else { e::SPEC_COLLISION },
            11 | 66 => event = e::CHASE_TERMINATE,
            12 => event = e::COOL_DOWN,
            14 => {
                event = if ctx.rng.next_u32() & 1 == 1 { e::CHASE_START } else { e::CHASE_JOIN };
                if chase_flags & 0x40 != 0 {
                    self.play_camera_man_lw_speech("T_604_Chsewrn", params, ctx);
                }
            }
            15 => event = e::CHASE_JOIN,
            16 => {
                event = e::CHASE_TERMINATE;
                self.play_camera_man_lw_speech("T_600_GetAway", params, ctx);
            }
            17 => event = e::LUNGE,
            18 => {
                event = e::HOT_PURSUIT;
                if chase_flags & 0x40 != 0 {
                    self.play_camera_man_lw_speech("T_603_CtchFail", params, ctx);
                }
            }
            19 => event = e::CATCH,
            20..=22 => event = if ctx.rng.next_u32() & 1 == 1 { e::FLEE } else { e::HELP },
            23 | 24 => event = e::POS,
            25 => event = e::SPEC_SLAM,
            26 => event = e::REPEAT,
            33 => event = e::INTRO_SHORT,
            34 => event = e::INTRO_QUESTION,
            35 => event = e::INTRO_STATEMENT,
            36 => event = e::OPINION,
            37 => event = e::STATEMENT,
            38 => event = e::QUESTION,
            39 => event = e::ANSWER,
            40 => event = e::OUTRO,
            48 => event = e::CONV_CELL,
            50 => event = e::CELL_BYE,
            51 => event = e::GRUNT,
            59 | 60 => {
                event = if params.char_type == CHAR_TYPE_SECURITY_GUARD { 0x2000 + u16::from(chase_flags & 0x80 != 0) } else { e::WARN };
            }
            62 => event = e::CLS_INTERACT,
            65 => event = e::BLOCK,
            69 => event = e::CLS_REACT,
            70 => event = e::CELL_GREET,
            71 => {
                event = e::CATCH;
                self.play_camera_man_lw_speech("T_609_Catch", params, ctx);
            }
            _ => return event,
        }
        if event != e::MAX {
            self.play_living_world_speech(ctx, character, pedestrian_id, event, params, stream, is_speaking);
        }
        event
    }

    /// AudioSystem.UpdatePresentationData82504950..c8: no-skate-zone exit,
    /// zeroed parameters except kSPCH2_LW_Type_guard=2.
    pub(crate) fn left_no_skate_zone(&mut self, ctx: &mut SpeechCtx<'_>) {
        let mut params = mission::CameraManParams { lw_type: 2, ..mission::CameraManParams::default() };
        self.play_camera_man_speech("T_762_LvNoSk8Zn", &mut params, ctx);
    }

    /// The cameraman's trigger a pedestrian audio state sends, with the pedestrian's cameraman
    /// type (`tCameraManParams::cm_lw_type`) and nothing else (`PlayCameraManSpeech`).
    fn play_camera_man_lw_speech(&mut self, function: &str, params: &LivingWorldParams, ctx: &mut SpeechCtx<'_>) {
        let mut camera_man = mission::CameraManParams { lw_type: params.cm_lw_type as i32, ..mission::CameraManParams::default() };
        self.play_camera_man_speech(function, &mut camera_man, ctx);
    }

    /// `PlayMainCastSpeechSample` 0x82503ee8: the main-cast context system's trigger for a
    /// pedestrian audio state, with the pedestrian's main-cast parameters
    /// (`PlayMainCastCsisEvent`). A speaking pedestrian taken down or grunting first stops its
    /// stream; a grunt is Big Black's hit when the player hit him, else a light grunt or an impact
    /// reaction by `Random() & 1`.
    #[allow(clippy::too_many_arguments)]
    fn play_main_cast_speech_sample(&mut self, ctx: &mut SpeechCtx<'_>, state: u32, params: &mission::MainCastParams, stream: i32, is_speaking: bool, player_collision: bool) {
        if matches!(state, 19 | 30 | 51) && is_speaking && (0..2).contains(&stream) {
            self.stop_speech_on_instance_stream(INSTANCE_CHARACTERS, stream as usize, ctx);
        }
        let trigger = match state {
            10 => "T_205_Spectate_Collision",
            14 | 53 | 54 | 65 => "T_663_Chase",
            17 => "T_609_Lunge",
            19 => "T_624_Post_Takedown",
            28 => "T_601_ai_greet",
            29 => "T_1002_amb_chat",
            30 | 51 if params.ip_id == IP_BIG_BLACK as i32 && player_collision => "T_626_BBHit",
            30 | 51 if ctx.rng.next_u32() & 1 != 0 => "T_201_Light_Impact_Grunt",
            30 | 51 => "T_202_impact_react",
            59 | 60 => "T_501_Warn",
            _ => return,
        };
        self.play_main_cast_speech(trigger, params, ctx);
    }

    /// `PlayAnnouncerSpeechSample` 0x824fc020: the announcer's context system greets
    /// (`GreetPlayer`) or, by `Random() & 1`, grunts or reacts to a hit (`PedGrunt`) through
    /// `PlayAnnouncerSpeech` 0x824fbf00, with the pedestrian's announcer id.
    pub fn play_announcer_speech_sample(&mut self, ctx: &mut SpeechCtx<'_>, state: u32, an_char_id: u32) {
        let trigger = match state {
            28 => "T_601_greet",
            51 if ctx.rng.next_u32() & 1 != 0 => "T_201_grunt",
            51 => "T_202_hit_react",
            _ => return,
        };
        let mut params = mission::AnnouncerParams { character: an_char_id as i32, ..mission::AnnouncerParams::default() };
        self.play_announcer_speech(trigger, &mut params, ctx);
    }

    /// A call of a context system's trigger function (`T_*` in `<group>context.big`'s
    /// `Trigger_Events.csi`, through `PlayCameraManCsisEvent` / `PlayMainCastCsisEvent` /
    /// `PlayAnnouncerCsisEvent`) with its parameter words.
    fn trigger_context_event(&mut self, group: Group, function: &str, params: &[i32], ctx: &mut SpeechCtx<'_>) {
        if let Some(contexts) = self.contexts.as_mut() {
            contexts.trigger(group, function, params, game_time(ctx.core.system_time));
        }
    }

    /// `IsCharacterRangeSpeaking` 0x824eb3f8 as `SecuritySpeaking` 0x824ebbd0 asks it: the range
    /// test is `character >= low || character <= high`, true for every character, so any
    /// character-instance voice actually playing.
    pub fn security_speaking(&self) -> bool {
        (0..STREAMS).any(|stream| self.instances[INSTANCE_CHARACTERS].voice_info(stream).is_some_and(|at| self.voice_info(at).is_actually_playing))
    }

    /// A character-instance stream actually playing the channel's speech.
    fn character_channel_speaking(&self, channel: i32) -> bool {
        let instance = &self.instances[INSTANCE_CHARACTERS];
        (0..STREAMS).any(|stream| instance.speech_actually_playing(stream) && instance.cur_channel(stream) == channel)
    }

    /// What `SFXObj_CameraMan::UpdateParams` 0x825287d0 and `SFXObj_Announcer::UpdateParams`
    /// 0x825284f0 ask: the voices instance's stream 0 actually plays the channel's speech.
    #[must_use]
    pub fn voices_speaking_on_channel(&self, channel: i32) -> bool {
        let voices = &self.instances[INSTANCE_VOICES];
        voices.speech_actually_playing(0) && voices.cur_channel(0) == channel
    }

    /// `IsAMainCastCharacterSpeaking` 0x824eaa78 (`SFXObj_Master` input 10).
    pub fn is_a_main_cast_character_speaking(&self) -> bool {
        self.character_channel_speaking(CHANNEL_MAIN_CAST)
    }

    /// `IsALivingWorldCharacterSpeaking` 0x824eab78 (`SFXObj_Master` input 11).
    pub fn is_a_living_world_character_speaking(&self) -> bool {
        self.character_channel_speaking(CHANNEL_LIVING_WORLD)
    }

    /// `IsProVoPlaying` 0x824eb490: a main-cast voice-over on a character stream.
    pub fn is_pro_vo_playing(&self) -> bool {
        (0..STREAMS).any(|stream| {
            self.instances[INSTANCE_CHARACTERS].voice_info(stream).is_some_and(|at| {
                let info = self.voice_info(at);
                info.is_actually_playing && info.channel == CHANNEL_MAIN_CAST && info.is_voiceover
            })
        })
    }

    /// `CSTATEMGR_Pedestrian::GetPedestrianVoiceInfoFromState(eSk8Characters, &id)` 0x82553f30:
    /// the first attached pedestrian state of the character, with its pedestrian id.
    fn pedestrian_voice_info_for_character(&self, character: i32) -> Option<(VoiceInfoRef, u32)> {
        let state = self.pedestrian_states.iter().position(|state| state.attached && i64::from(state.character) == i64::from(character))?;
        Some((VoiceInfoRef::Pedestrian(state), self.pedestrian_states[state].pedestrian_id))
    }

    /// `GetVoiceInfoForSample` 0x8250be58: the speaker a sample plays for, by channel.
    ///
    /// - Main cast: the main-cast voice-over for a voice-over event, else a NIS sample's post
    ///   (instance 2) or NIS voice, else the character's NIS character, skater or pedestrian.
    /// - Living world: the NIS photographer for a voice-over event, else by the callback data: a
    ///   NIS or mission sample's (0xDEADBEEF) character as for the main cast, a pedestrian id, a
    ///   skater id (below 0x400), or a NIS character.
    /// - Cameraman: a pedestrian of the character, else the cameraman (whose events 33055 stop
    ///   his stream and 33147 / 32938 all speech).
    /// - Announcer: a NIS character, else a pedestrian of the character outside challenges for a
    ///   bank sample, else the announcer.
    ///
    /// An incoming call takes the cell phone instead. A speaker found records the character and
    /// distance class the name gives and takes the event's tuning, unless it is already speaking
    /// or queueing; without one the event did not play (`EventNotPlayedImpl`). The NIS characters'
    /// voices (`CSTATEMGR_NISCharacter`, `crate::world::nis_character`) attach to no state in the
    /// port and the skaters' (`SFXObj_PlayerSpeech`) are not ported, so those lookups find none.
    #[allow(clippy::too_many_arguments)]
    fn voice_info_for_sample(
        &mut self,
        ctx: &mut SpeechCtx<'_>,
        instance: usize,
        channel: i32,
        stream: usize,
        event: u16,
        filename: &str,
        callback_data: u32,
    ) -> (Option<VoiceInfoRef>, i32) {
        let (character, common_event_id, is_nis, is_far) = parse_event_file_name(filename);
        let tuning_key = Self::speech_tuning_key(ctx.vault, character);
        let event_key = Self::speech_event_key(ctx.vault, channel, event, tuning_key);
        let tuning = Self::speech_event(ctx.vault, event_key);
        let mut speaking_pedestrian = None;
        let mut found = match channel {
            CHANNEL_MAIN_CAST if tuning.is_voice_over => Some(VoiceInfoRef::MainCastVo),
            CHANNEL_MAIN_CAST if is_nis => Some(if instance == INSTANCE_POST { VoiceInfoRef::Post } else { VoiceInfoRef::Nis }),
            CHANNEL_MAIN_CAST => self.pedestrian_voice_info_for_character(character).map(|(at, id)| {
                speaking_pedestrian = Some(id);
                at
            }),
            CHANNEL_LIVING_WORLD if tuning.is_voice_over => Some(VoiceInfoRef::NisPhotographer),
            CHANNEL_LIVING_WORLD if callback_data == spch::NO_CALLBACK_DATA => self.pedestrian_voice_info_for_character(character).map(|(at, id)| {
                speaking_pedestrian = Some(id);
                at
            }),
            CHANNEL_LIVING_WORLD if callback_data >= 0x0100_0001 => {
                let at = self.pedestrian_states.iter().position(|state| state.attached && state.pedestrian_id == callback_data).map(VoiceInfoRef::Pedestrian);
                if at.is_some() {
                    self.speaking_ped_info[stream].pedestrian_id = callback_data;
                    self.speaking_ped_info[stream].current_distance = 0.0;
                }
                at
            }
            CHANNEL_LIVING_WORLD => None,
            CHANNEL_CAMERA_MAN => match self.pedestrian_voice_info_for_character(character) {
                Some((at, id)) => {
                    speaking_pedestrian = Some(id);
                    Some(at)
                }
                None => {
                    if event == CM_EVENT_STOP_STREAM {
                        self.stop_speech_on_instance_stream(instance, stream, ctx);
                    }
                    if CM_EVENTS_STOP_ALL.contains(&event) {
                        self.stop_all_speech(ctx);
                    }
                    Some(VoiceInfoRef::CameraMan)
                }
            },
            CHANNEL_ANNOUNCER => match self.pedestrian_voice_info_for_character(character) {
                Some((at, id)) if !is_nis && self.game.challenge_type == CHALLENGE_UNKNOWN => {
                    speaking_pedestrian = Some(id);
                    Some(at)
                }
                _ => Some(VoiceInfoRef::Announcer),
            },
            _ => None,
        };
        if let Some(id) = speaking_pedestrian {
            self.speaking_ped_info[stream].current_distance = 0.0;
            self.speaking_ped_info[stream].pedestrian_id = id;
        }
        if self.incoming_call {
            found = Some(VoiceInfoRef::CellPhone);
            self.incoming_call = false;
        }
        let Some(at) = found else {
            let unplayed = mission::WatchedEvent { character, sed_event: event, event_id: common_event_id, instance: instance as i32, channel };
            self.event_not_played(&unplayed);
            return (None, common_event_id);
        };
        let info = self.voice_info_mut(at);
        info.character = character;
        info.is_far = is_far;
        if info.is_actually_playing || info.is_queueing {
            return (None, common_event_id);
        }
        self.update_tuning_data(ctx.vault, channel, stream, event, event_key);
        (Some(at), common_event_id)
    }

    /// `StopAllSpeech` 0x824eb2d8: every instance's streams.
    pub fn stop_all_speech(&mut self, ctx: &mut SpeechCtx<'_>) {
        for instance in 0..INSTANCES {
            self.stop_all_speech_on_instance(instance, ctx);
        }
    }

    /// `GetEventStartTime` 0x82508628: the stream's tuned delay.
    fn event_start_time(&self, channel: i32, stream: usize) -> f32 {
        self.current_tuning[channel as usize][stream].delay
    }
}

/// The `eSk8Characters` the speech system treats specially: a bank name without its two-digit
/// character (`ParseEventFileName` leaves `eSk8Character_Max`).
const CHARACTER_MAX: i32 = 100;
/// `SPCH1Type_CharType`.
const CHAR_TYPE_SECURITY_GUARD: u32 = 0x40;
const CHAR_TYPE_BUM: u32 = 0x4000;
/// `kSPCH0_IP_ID_26_Big`.
const IP_BIG_BLACK: u32 = 1;

/// `tLivingWorldParams` (the parameters `PlayLWSEDEvent` reads).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LivingWorldParams {
    pub char_type: u32,
    pub char_id: u32,
    pub near_far: u32,
    pub lw_class: u32,
    pub topic: u32,
    pub cm_lw_type: u32,
    pub cm_lw_gender: u32,
}

/// `SPCHType_1_EventID`.
pub mod lw_event {
    pub const CHASE_START: u16 = 8194;
    pub const HOT_PURSUIT: u16 = 8195;
    pub const CHASE_TERMINATE: u16 = 8196;
    pub const CATCH: u16 = 8197;
    pub const CHASE_JOIN: u16 = 8198;
    pub const LUNGE: u16 = 8199;
    pub const BLOCK: u16 = 8200;
    pub const COOL_DOWN: u16 = 8201;
    pub const POS: u16 = 8202;
    pub const REPEAT: u16 = 8203;
    pub const SPEC_SLAM: u16 = 8204;
    pub const GRUNT: u16 = 8206;
    pub const IMPACT_REACT: u16 = 8207;
    pub const GASP: u16 = 8208;
    pub const SPEC_COLLISION: u16 = 8209;
    pub const WARN: u16 = 8210;
    pub const CONV_CELL: u16 = 8211;
    pub const CLS_INTERACT: u16 = 8212;
    pub const CLS_REACT: u16 = 8213;
    pub const SHOUT: u16 = 8214;
    pub const CELL_GREET: u16 = 8215;
    pub const CELL_BYE: u16 = 8216;
    pub const HELP: u16 = 8217;
    pub const CONCERN: u16 = 8220;
    pub const FLEE: u16 = 8224;
    pub const PAN_HANDLE: u16 = 8247;
    pub const RANDOM_BUM: u16 = 8248;
    pub const INTRO_SHORT: u16 = 8269;
    pub const ANSWER: u16 = 8270;
    pub const INTRO_QUESTION: u16 = 8271;
    pub const INTRO_STATEMENT: u16 = 8272;
    pub const OPINION: u16 = 8273;
    pub const STATEMENT: u16 = 8274;
    pub const QUESTION: u16 = 8275;
    pub const OUTRO: u16 = 8276;
    pub const RADIO: u16 = 8277;
    pub const MAX: u16 = 8279;
}

/// `ParseEventFileName` 0x824eb5d0: a NIS name is `CC_EEEE...` (character, event); a bank name
/// is `EEE_CC_..._f.dat` (event, character, far when it ends in `_f`). Returns the character
/// (`eSk8Character_Max` when there is none), the event, NIS and far.
fn parse_event_file_name(name: &str) -> (i32, i32, bool, bool) {
    let bytes = name.as_bytes();
    let byte = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let mut character = CHARACTER_MAX;
    let mut event = -1;
    let mut is_far = false;
    if byte(2) == b'_' {
        if byte(0).is_ascii_digit() && byte(1).is_ascii_digit() {
            character = atoi(&bytes[..2]);
        }
        event = atoi(bytes.get(3..7).unwrap_or(&[]));
    }
    if character != CHARACTER_MAX {
        return (character, event, true, false);
    }
    if let Some(underscore) = name.find('_') {
        event = atoi(&bytes[..underscore.min(4)]);
        character = atoi(bytes.get(underscore + 1..underscore + 3).unwrap_or(&[]));
        if let Some(dot) = name.find('.') {
            is_far = dot >= 2 && &bytes[dot - 2..dot] == b"_f";
        }
    }
    (character, event, false, is_far)
}

/// `SPCH_Init`'s game seed: the low word of `time64(NULL)`.
#[must_use]
pub fn time64_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs() as u32)
}

/// `rw::core::stdc::ConvertAToI` over a short field.
fn atoi(bytes: &[u8]) -> i32 {
    let mut value = 0i32;
    for &b in bytes {
        if !b.is_ascii_digit() {
            break;
        }
        value = value.wrapping_mul(10).wrapping_add(i32::from(b - b'0'));
    }
    value
}

/// The `SFXObj_Speech` (MAIN 8) mixer inputs: 1 while any character voice plays
/// (`SecuritySpeaking`), 2 while the mission speech plays (`ScriptedSpeechPlaying`), 3 while a
/// main-cast voice-over plays (`IsProVoPlaying`); 0 and 4 come from a getter that returns false
/// in this build.
pub fn speech_object_inputs(speech: Option<&Sk8SpeechSystem>) -> [i32; 6] {
    let mut inputs = [0; 6];
    if let Some(speech) = speech {
        if speech.security_speaking() {
            inputs[1] = 0x7FFF;
        }
        if speech.scripted_speech_playing() {
            inputs[2] = 0x7FFF;
        }
        if speech.is_pro_vo_playing() {
            inputs[3] = 0x7FFF;
        }
    }
    inputs
}

/// `SFXObj_Speech`'s objectID: MAIN 8.
pub const SPEECH_OBJECT_ID: u32 = 0x4000_0080;

/// The `SFXObj_Speech` (MAIN 8, `UpdateParams` 0x825333a0) inputs on its mixer IO.
pub fn update_speech_object(map: &mut MixMap, io: Option<crate::nicotine::DMixIoId>, speech: Option<&Sk8SpeechSystem>) {
    let Some(io) = io else { return };
    for (index, value) in speech_object_inputs(speech).into_iter().enumerate() {
        map.set_input(io, index, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_file_names() {
        assert_eq!(parse_event_file_name("101_41_adtm1_SpecPos_f.dat"), (41, 101, false, true));
        assert_eq!(parse_event_file_name("101_41_adtm1_SpecPos_n.dat"), (41, 101, false, false));
        assert_eq!(parse_event_file_name("04_5003_3b_01_dway_big.sns"), (4, 5003, true, false));
    }

    #[test]
    fn lw_event_indices() {
        assert_eq!(Sk8SpeechSystem::lw_event_index(0x2000), 0);
        assert_eq!(Sk8SpeechSystem::lw_event_index(0x200A), 10);
        assert_eq!(Sk8SpeechSystem::lw_event_index(0x2056), 60);
        assert_eq!(Sk8SpeechSystem::lw_event_index(0x2021), 0);
    }
}
