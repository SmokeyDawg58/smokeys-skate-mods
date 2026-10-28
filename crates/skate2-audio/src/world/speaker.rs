//! The SPEAKER state (`eMAINMAPSTATES` 9): `CSTATEMGR_Speaker` (constructor 0x82550b40) and its
//! four `CSTATE_Speaker`s (`EnterGamePlay` 0x82550b98, created on the first entry only), each
//! with an `SFXObj_Speaker` (`CreateObject` 0x82549730), an `SFXCTL_SpeakerPhysics` (`CreateObject`
//! 0x8253ebc0) and an `SFXCTL_3DSpeakerPos` (`CreateObject` 0x8251b038, an `SFXCTL_3DObjPos`): the
//! public-address speakers of challenge venues, which carry the speech and music through two
//! `MultiSpeaker` voices.
//!
//! `AudioSystem::PlayMissionSFXImp` 0x824fecb8 turns the multi-speaker mode on off line for the
//! venues `LoadCrowdsAndSpeakerSystem` 0x824e7d58 names (Demo and Best Trick challenges, one Jam)
//! and off at the contest's unload. In that mode `UpdateParams` builds the voices, routes the
//! voices instance's stream to the speech `MultiSpeaker` and attaches a state to each of the
//! first four speaker volumes of `speakers.ems` the emitter system reports; every speaker volume
//! also needs the emitter game state the contest sets. The music's contest presets route to the
//! music `MultiSpeaker` (`MusicSystem::MapStreamChannels` 0x82501310).

use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::physics::point_graph;
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::SfxCtx;
use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
use crate::rwac::{PluginRef, VoiceId};
use crate::speech::{Sk8SpeechSystem, SpeechCtx};
use crate::vault::{class, Collection, Vault};
use crate::vmx::{self, Vec3, ZERO};
use crate::world::emitter_system::{Sk8EmitterSystem, VOLUME_SPEAKER};

const STATE: u32 = 9;
const STATES: u32 = 4;
/// `eSpeakerType`.
pub const SPEAKER_SPEECH: usize = 0;
pub const SPEAKER_MUSIC: usize = 1;
const SPEAKER_TYPES: usize = 2;
/// `tMultiSpeakerParms::numSpeakers`.
const SPEAKERS: usize = 4;
/// The speech instances: the characters, whose delay voices the mode frees, and the voices,
/// whose stream 0 the speech `MultiSpeaker` takes.
const SPEECH_CHARACTERS: usize = 0;
const SPEECH_VOICES: usize = 1;

/// `aud_general` "multispeaker" (`GlobalAttributeCollection::aud_general_multispeaker`): each
/// type's longest delay, and how many updates a delay time holds.
const MULTISPEAKER: Collection = Collection::new(class::AUD_GENERAL, 0xBA01_52E2_B687_BF9F);
const MAX_DELAY: [u64; SPEAKER_TYPES] = [0xC5DD_8C16_C17E_40AF, 0x5082_6A72_4457_E49D];
const DELAY_UPDATE_COUNT: u64 = 0xCFDE_BEDF_121C_9071;
/// The speaker volume's `aud_emitter` attributes `CSTATE_Speaker`'s getters read
/// (0x82512f28-0x825134e8), each of a type the speech's then the music's.
const AUD_EMITTER: u64 = 0xF0CE_F367_088E_FFF8;
const NICOTINE_DISTANCE: u64 = 0x817D_6D83_F86B_D907;
const DISTANCE_FACTOR: [u64; SPEAKER_TYPES] = [0x5626_F721_0A93_785E, 0x9AA2_86A6_47A6_FE17];
const DELAY_TIME_MAX: [u64; SPEAKER_TYPES] = [0x559D_6E72_C13D_422C, 0xED4E_969D_7172_95E9];
const PAN_DISTANCE: [u64; SPEAKER_TYPES] = [0x3AAF_2380_78D6_A545, 0x4A39_CE99_3C70_6C48];
const MAX_GAIN: [u64; SPEAKER_TYPES] = [0x9E41_1F36_BBEB_61E1, 0xA875_B2E1_84C3_43CE];
const NOTCH_FREQ: [u64; SPEAKER_TYPES] = [0x4B8C_DA79_621C_BA3A, 0x236D_142D_62E4_371F];
const NOTCH_GAIN: [u64; SPEAKER_TYPES] = [0x7845_71C0_E02B_5B8E, 0x5BFB_8FD7_ADC6_AACB];
const NOTCH_Q: [u64; SPEAKER_TYPES] = [0xDE9B_F5C1_0AB2_C866, 0x5965_DAB3_1AC8_04E1];
/// `1 / 344` (the speed of sound), `1 / 1000`, `1 / 32767` and `360 / 65536`.
const INVERSE_SPEED_OF_SOUND: f32 = f32::from_bits(0x3B3E_82FA);
const MILLISECONDS: f32 = f32::from_bits(0x3A83_126F);
const VOLUME_SCALE: f32 = f32::from_bits(0x3800_0100);
const AZIMUTH_SCALE: f32 = f32::from_bits(0x3BB4_0000);
/// `SFXObj_Speaker`'s mixer outputs: the azimuth, the low pass, the high pass and the volume; the
/// music's own volume, low pass and high pass; and in a large arena each type's volume, low pass
/// and high pass (read as volumes).
const OUT_AZIMUTH: i32 = 0;
const OUT_VOLUME: i32 = 1;
const OUT_LOW_PASS: i32 = 2;
const OUT_HIGH_PASS: i32 = 3;
const OUT_MUSIC_VOLUME: i32 = 4;
const OUT_MUSIC_LOW_PASS: i32 = 5;
const OUT_MUSIC_HIGH_PASS: i32 = 6;
const OUT_ARENA_VOLUME: [i32; SPEAKER_TYPES] = [7, 10];
const OUT_ARENA_LOW_PASS: [i32; SPEAKER_TYPES] = [8, 11];
const OUT_ARENA_HIGH_PASS: [i32; SPEAKER_TYPES] = [9, 12];
/// The plug-ins of a `Speaker` voice.
const SPEAKER_SUB_MIX: u8 = 0;
const SPEAKER_GAIN: u8 = 1;
const SPEAKER_HIGH_PASS: u8 = 2;
const SPEAKER_DELAY: u8 = 3;
const SPEAKER_LOW_PASS: u8 = 4;
const SPEAKER_NOTCH: u8 = 5;
const SPEAKER_PAN: u8 = 6;
const SPEAKER_SEND: u8 = 7;

/// `Sk8::PointGraphEval::Evaluate(8, x, y, value)` over a 0x50-byte `aud_emitter` graph (x at
/// byte 0x10, y at byte 0x30).
fn graph(vault: &Vault, collection: Collection, key: u64, value: f32) -> f32 {
    let bytes = vault.bytes(collection, key, 0, 0x50);
    let float = |offset: usize| f32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
    let x: [f32; 8] = std::array::from_fn(|i| float(0x10 + 4 * i));
    let y: [f32; 8] = std::array::from_fn(|i| float(0x30 + 4 * i));
    point_graph(&x, &y, value)
}

/// `GetNotchFreq` 0x82513298 / `GetNotchGain` 0x825133c0: the graph over the azimuth folded to
/// 0..32768.
fn notch_graph(vault: &Vault, collection: Collection, key: u64, azimuth: i32) -> f32 {
    let value = azimuth as f32;
    let folded = if value > 32767.0 { 65536.0 - value } else { value };
    graph(vault, collection, key, folded)
}

/// `Sk8::Audio::tSpeakerParams` (0x34 bytes).
#[derive(Debug, Clone, Copy, PartialEq)]
struct SpeakerParams {
    gain: f32,
    highpass_freq: f32,
    delay_max_delay: f32,
    delay_feedback: f32,
    delay_time: f32,
    lowpass_freq: f32,
    notch_freq: f32,
    notch_gain: f32,
    notch_q: f32,
    pan_distance: f32,
    pan_out_channels: u8,
    azimuth: i32,
    processing_stage: u8,
}

impl SpeakerParams {
    /// The values `UpdateParams`, `ProcessUpdate` and `Detach` start from.
    const DEFAULT: Self = Self {
        gain: 0.0,
        highpass_freq: 0.0,
        delay_max_delay: 2.0,
        delay_feedback: 0.0,
        delay_time: 0.0,
        lowpass_freq: 96000.0,
        notch_freq: 96000.0,
        notch_gain: 1.0,
        notch_q: 3.0,
        pan_distance: 0.0,
        pan_out_channels: 6,
        azimuth: 0,
        processing_stage: 6,
    };
}

/// `Sk8::Audio::Speaker`: a voice of `SubMix`, `Gain`, `HighPassIir2`, `Delay`, `LowPassIir2`,
/// `PeakingIir2`, `Pan2D1` and a `Send` to the speech or music master.
struct Speaker {
    voice: VoiceId,
}

impl Speaker {
    /// `CreateSpeakerVoice` 0x824f4aa0.
    fn create(params: &SpeakerParams, master: Option<PluginRef>, core: &mut AudioCore) -> Option<Self> {
        let mono = |kind: PlugInKind| PlugInConfig { kind, output_channels: 1 };
        let voice = core.create_voice(
            params.processing_stage,
            "Speaker",
            &[
                mono(PlugInKind::SubMix { name: "Speaker SubMix".into() }),
                mono(PlugInKind::Gain),
                mono(PlugInKind::HighPassIir2),
                mono(PlugInKind::Delay { max_time: Some(params.delay_max_delay) }),
                mono(PlugInKind::LowPassIir2),
                mono(PlugInKind::PeakingIir2),
                PlugInConfig { kind: PlugInKind::Pan2D1 { constructor: Some((45.0, 135.0, 0.0)) }, output_channels: params.pan_out_channels },
                PlugInConfig { kind: PlugInKind::Send, output_channels: params.pan_out_channels },
            ],
        )?;
        let speaker = Self { voice };
        core.set_attribute(speaker.plugin(SPEAKER_GAIN), 0, params.gain);
        core.set_attribute(speaker.plugin(SPEAKER_HIGH_PASS), 0, params.highpass_freq);
        core.set_attribute(speaker.plugin(SPEAKER_DELAY), 0, params.delay_time);
        core.set_attribute(speaker.plugin(SPEAKER_DELAY), 1, params.delay_feedback);
        core.set_attribute(speaker.plugin(SPEAKER_LOW_PASS), 0, params.lowpass_freq);
        core.set_attribute(speaker.plugin(SPEAKER_NOTCH), 0, params.notch_freq);
        core.set_attribute(speaker.plugin(SPEAKER_NOTCH), 1, params.notch_gain);
        core.set_attribute(speaker.plugin(SPEAKER_NOTCH), 2, params.notch_q);
        core.connect_send(speaker.plugin(SPEAKER_SEND), master);
        Some(speaker)
    }

    fn plugin(&self, index: u8) -> PluginRef {
        PluginRef { voice: self.voice, index }
    }

    /// `UpdateSpeakerVoice` 0x824f4ec0: each value that changed (all when forced) but the
    /// delay's feedback.
    fn update(&self, params: &SpeakerParams, force: bool, last: &SpeakerParams, core: &mut AudioCore) {
        let mut set = |changed: bool, plugin: u8, index: usize, value: f32| {
            if force || changed {
                core.set_attribute(self.plugin(plugin), index, value);
            }
        };
        set(params.gain != last.gain, SPEAKER_GAIN, 0, params.gain);
        set(params.highpass_freq != last.highpass_freq, SPEAKER_HIGH_PASS, 0, params.highpass_freq);
        set(params.delay_time != last.delay_time, SPEAKER_DELAY, 0, params.delay_time);
        set(params.lowpass_freq != last.lowpass_freq, SPEAKER_LOW_PASS, 0, params.lowpass_freq);
        set(params.notch_freq != last.notch_freq, SPEAKER_NOTCH, 0, params.notch_freq);
        set(params.notch_gain != last.notch_gain, SPEAKER_NOTCH, 1, params.notch_gain);
        set(params.notch_q != last.notch_q, SPEAKER_NOTCH, 2, params.notch_q);
        set(params.azimuth != last.azimuth, SPEAKER_PAN, 0, params.azimuth as f32 * AZIMUTH_SCALE);
        set(params.pan_distance != last.pan_distance, SPEAKER_PAN, 1, params.pan_distance);
    }
}

/// `Sk8::Audio::MultiSpeaker` (0x30 bytes): a stage 5 voice of a `SubMix` and a `Send` to each
/// speaker's `SubMix`.
struct MultiSpeaker {
    voice: Option<VoiceId>,
    speakers: [Option<Speaker>; SPEAKERS],
}

impl MultiSpeaker {
    /// `CreateMultiSpeakerVoice` 0x82500d38.
    fn create(params: &[SpeakerParams; SPEAKERS], master: Option<PluginRef>, core: &mut AudioCore) -> Self {
        let mut configs = vec![PlugInConfig { kind: PlugInKind::SubMix { name: "MultiSpeaker Submix".into() }, output_channels: 1 }];
        configs.extend((0..SPEAKERS).map(|_| PlugInConfig { kind: PlugInKind::Send, output_channels: 1 }));
        let voice = core.create_voice(5, "MultiSpeaker", &configs);
        let mut speakers = [None, None, None, None];
        for (index, params) in params.iter().enumerate() {
            let speaker = Speaker::create(params, master, core);
            if let (Some(voice), Some(speaker)) = (voice, speaker.as_ref()) {
                core.connect_send(PluginRef { voice, index: index as u8 + 1 }, Some(speaker.plugin(SPEAKER_SUB_MIX)));
            }
            speakers[index] = speaker;
        }
        Self { voice, speakers }
    }

    /// `CSTATEMGR_Speaker::GetMultiSpeakerSubmix` 0x82550c18 of this voice.
    fn sub_mix(&self) -> Option<PluginRef> {
        self.voice.map(|voice| PluginRef { voice, index: 0 })
    }

    /// `UpdateSingleSpeaker` 0x824f5190.
    fn update_single_speaker(&self, params: &SpeakerParams, index: usize, force: bool, last: &SpeakerParams, core: &mut AudioCore) {
        if let Some(speaker) = self.speakers.get(index).and_then(Option::as_ref) {
            speaker.update(params, force, last, core);
        }
    }

    /// `DestroyMultiSpeakerVoice` 0x82505ce8: the voice, then each speaker's
    /// (`DestroySpeakerVoice` 0x824ed5d8).
    fn destroy(self, core: &mut AudioCore) {
        if let Some(voice) = self.voice {
            core.release_voice(voice);
        }
        for speaker in self.speakers.into_iter().flatten() {
            core.release_voice(speaker.voice);
        }
    }
}

/// `SFXCTL_SpeakerPhysics` (0x60 bytes): the speaker's position (its velocity stays zero), and
/// each type's travel delay and pan distance.
struct SpeakerPhysics {
    io: Option<DMixIoId>,
    position: Vec3,
    velocity: Vec3,
    delay_time: [f32; SPEAKER_TYPES],
    pan_distance: [f32; SPEAKER_TYPES],
}

impl SpeakerPhysics {
    /// `InitSFX` 0x82512de0: the mixer output on, the position and velocity zeroed and handed to
    /// the position controller.
    fn init_sfx(&mut self, map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        self.velocity = ZERO;
        self.position = ZERO;
    }

    /// `UpdateParams` 0x8251b178: the speaker's position; the camera's distance through the
    /// volume's nicotine-distance graph to input 0 (clamped to 0..0x7FFF); each type's delay,
    /// the distance scaled by its factor over the speed of sound up to its longest, and pan
    /// distance.
    fn update_params(&mut self, speaker_pos: Vec3, attrib: Collection, mics: &MicrophoneMan, vault: &Vault, map: &mut MixMap) {
        self.position = speaker_pos;
        let distance = vmx::length(vmx::sub(mics.microphones[0].cur_position, self.position));
        let nicotine = fctiwz(graph(vault, attrib, NICOTINE_DISTANCE, distance)).clamp(0, 0x7FFF);
        if let Some(io) = self.io {
            map.set_input(io, 0, nicotine);
        }
        for kind in 0..SPEAKER_TYPES {
            let delay = vault.f32(attrib, DISTANCE_FACTOR[kind]) * distance * INVERSE_SPEED_OF_SOUND;
            let longest = vault.f32(attrib, DELAY_TIME_MAX[kind]) * MILLISECONDS;
            self.delay_time[kind] = if delay <= longest { delay } else { longest };
            self.pan_distance[kind] = graph(vault, attrib, PAN_DISTANCE[kind], distance);
        }
    }
}

/// `SFXObj_Speaker` (0xAC bytes).
struct SpeakerObject {
    io: Option<DMixIoId>,
    /// `mpMultiSpeaker`: whether the attachment carried each type's voice.
    multi_speaker: [bool; SPEAKER_TYPES],
    valid_last_params: [bool; SPEAKER_TYPES],
    last_params: [SpeakerParams; SPEAKER_TYPES],
    update_count: [i32; SPEAKER_TYPES],
    last_delay_time: [f32; SPEAKER_TYPES],
}

impl SpeakerObject {
    fn volume(&self, map: &MixMap, index: i32) -> i32 {
        self.io.map_or(0, |io| map.output(io, index, PresetType::Volume))
    }

    fn frequency(&self, map: &MixMap, index: i32) -> i32 {
        self.io.map_or(0, |io| map.output(io, index, PresetType::Frequency))
    }

    /// `InitSFX` 0x82518150: the mixer output on, the attachment's voices, no last parameters,
    /// held delay or update count.
    fn init_sfx(&mut self, multi_speaker: [bool; SPEAKER_TYPES], map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        self.multi_speaker = multi_speaker;
        self.valid_last_params = [false; SPEAKER_TYPES];
        self.last_delay_time = [0.0; SPEAKER_TYPES];
        self.update_count = [0; SPEAKER_TYPES];
    }

    /// `ProcessUpdate` 0x82536a48: for each type the mixer's volume times the volume's maximum
    /// gain, its filters (a large arena's own outputs, else the music's), a travel delay held for
    /// the multi-speaker update count, the azimuth's notch, and the pan distance, into the type's
    /// speaker (forced on the first update).
    fn process_update(&mut self, speaker_index: usize, attrib: Collection, physics: &SpeakerPhysics, large_arena: bool, voices: &[Option<MultiSpeaker>; SPEAKER_TYPES], vault: &Vault, map: &MixMap, core: &mut AudioCore) {
        for kind in 0..SPEAKER_TYPES {
            let music = kind == SPEAKER_MUSIC;
            let mut params = SpeakerParams::DEFAULT;
            let max_gain = vault.f32(attrib, MAX_GAIN[kind]);
            let mut volume = self.volume(map, OUT_VOLUME);
            if large_arena {
                volume = self.volume(map, OUT_ARENA_VOLUME[kind]);
            } else if music {
                volume = self.volume(map, OUT_MUSIC_VOLUME);
            }
            params.gain = volume as f32 * max_gain * VOLUME_SCALE;
            params.highpass_freq = self.frequency(map, OUT_HIGH_PASS) as f32;
            if large_arena {
                params.highpass_freq = self.volume(map, OUT_ARENA_HIGH_PASS[kind]) as f32;
            } else if music {
                params.highpass_freq = self.frequency(map, OUT_MUSIC_HIGH_PASS) as f32;
            }
            if self.update_count[kind] > 0 {
                self.update_count[kind] -= 1;
                params.delay_time = self.last_delay_time[kind];
            } else {
                params.delay_time = physics.delay_time[kind];
                self.update_count[kind] = vault.i32(MULTISPEAKER, DELAY_UPDATE_COUNT).wrapping_sub(1);
                self.last_delay_time[kind] = params.delay_time;
            }
            params.lowpass_freq = self.frequency(map, OUT_LOW_PASS) as f32;
            if large_arena {
                params.lowpass_freq = self.volume(map, OUT_ARENA_LOW_PASS[kind]) as f32;
            } else if music {
                params.lowpass_freq = self.frequency(map, OUT_MUSIC_LOW_PASS) as f32;
            }
            let azimuth = self.io.map_or(0, |io| map.output(io, OUT_AZIMUTH, PresetType::Azimuth));
            params.notch_freq = notch_graph(vault, attrib, NOTCH_FREQ[kind], azimuth);
            params.notch_gain = notch_graph(vault, attrib, NOTCH_GAIN[kind], azimuth);
            params.notch_q = vault.f32(attrib, NOTCH_Q[kind]);
            params.pan_distance = physics.pan_distance[kind];
            params.azimuth = azimuth;
            if self.multi_speaker[kind]
                && let Some(voice) = voices[kind].as_ref()
            {
                voice.update_single_speaker(&params, speaker_index, !self.valid_last_params[kind], &self.last_params[kind], core);
            }
            self.valid_last_params[kind] = true;
            self.last_params[kind] = params;
        }
    }

    /// `Detach` 0x82536f68: each type's speaker takes the defaults, forced (silent).
    fn detach(&self, speaker_index: usize, voices: &[Option<MultiSpeaker>; SPEAKER_TYPES], core: &mut AudioCore) {
        for kind in 0..SPEAKER_TYPES {
            if self.multi_speaker[kind]
                && let Some(voice) = voices[kind].as_ref()
            {
                voice.update_single_speaker(&SpeakerParams::DEFAULT, speaker_index, true, &self.last_params[kind], core);
            }
        }
    }
}

/// One `CSTATE_Speaker` (0xA0 bytes) with its controllers and object.
struct SpeakerState {
    attached: bool,
    /// `mbDetach`: set by every `ProcessUpdate`, cleared while a reported volume matches.
    pending_detach: bool,
    /// `mSpeakerState.emitterId`: the attached volume's identifier.
    emitter_id: u32,
    /// `mvSpeakerPos` and `mSpeakerAttrib` (the volume's `aud_emitter` collection).
    speaker_pos: Vec3,
    attrib: Collection,
    /// `mSpeakerIndex`: the state's speaker in each `MultiSpeaker`.
    speaker_index: usize,
    delta_time: f32,
    position: Obj3d,
    physics: SpeakerPhysics,
    object: SpeakerObject,
}

impl SpeakerState {
    fn new(instance: u32, vault: &Vault, map: Option<&MixMap>) -> Self {
        let io = |kind: u32, sfx: u32| map.and_then(|map| map.dmixio(kind | (STATE << 16) | (instance << 11) | (sfx << 4)));
        let mut position = Obj3d::new(vault);
        position.io = io(0x6000_0000, 0);
        Self {
            attached: false,
            pending_detach: false,
            emitter_id: 0,
            speaker_pos: ZERO,
            attrib: Collection::new(AUD_EMITTER, 0),
            speaker_index: instance as usize,
            delta_time: 0.0,
            position,
            physics: SpeakerPhysics { io: io(0x6000_0000, 1), position: ZERO, velocity: ZERO, delay_time: [0.0; SPEAKER_TYPES], pan_distance: [0.0; SPEAKER_TYPES] },
            object: SpeakerObject {
                io: io(0x4000_0000, 0),
                multi_speaker: [false; SPEAKER_TYPES],
                valid_last_params: [false; SPEAKER_TYPES],
                last_params: [SpeakerParams::DEFAULT; SPEAKER_TYPES],
                update_count: [0; SPEAKER_TYPES],
                last_delay_time: [0.0; SPEAKER_TYPES],
            },
        }
    }

    /// `CSTATE_Speaker::Attach` 0x825568d8: the volume's identifier, position (`mvSpeakerPos`) and
    /// `aud_emitter` collection; then `CSTATE_Base::Attach` 0x82a7e2d0: the controllers' `InitSFX`
    /// in index order (the position's output on, then the physics'), then the object's.
    fn attach(&mut self, multi_speaker: [bool; SPEAKER_TYPES], identifier: u32, pos: [f32; 3], key: u64, map: &mut MixMap) {
        self.emitter_id = identifier;
        self.speaker_pos = [pos[0], pos[1], pos[2], 0.0];
        self.attrib = Collection::new(AUD_EMITTER, key);
        self.attached = true;
        if let Some(io) = self.position.io {
            map.set_output_enabled(io, true);
        }
        self.physics.init_sfx(map);
        self.object.init_sfx(multi_speaker, map);
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} speaker {} attach volume {identifier:#x} at {pos:?}, key {key:#018x}", self.speaker_index);
        }
    }

    /// `CSTATE_Speaker::Detach` 0x82551a80: `CSTATE_Base::Detach` 0x82a7e350 (the object's
    /// `Detach` with its mixer output off and inputs cleared, then the controllers'), and the
    /// attachment cleared.
    fn detach(&mut self, voices: &[Option<MultiSpeaker>; SPEAKER_TYPES], ctx: &mut SfxCtx<'_>) {
        self.attached = false;
        self.object.detach(self.speaker_index, voices, ctx.core);
        if let Some(io) = self.object.io {
            ctx.map.set_output_enabled(io, false);
            ctx.map.clear_input(io);
        }
        self.position.detach();
        for io in [self.position.io, self.physics.io].into_iter().flatten() {
            ctx.map.clear_input(io);
        }
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} speaker {} detach volume {:#x}", self.speaker_index, self.emitter_id);
        }
        self.emitter_id = 0;
    }
}

/// `CSTATEMGR_Speaker` (0x28 bytes).
pub struct SpeakerManager {
    states: Vec<SpeakerState>,
    /// `mpMultiSpeaker`, per `eSpeakerType`.
    multi_speakers: [Option<MultiSpeaker>; SPEAKER_TYPES],
    /// `mbIsMultiSpeakerMode`, which a loaded contest sets.
    multi_speaker_mode: bool,
    /// `mbSubmixConnected`.
    submix_connected: bool,
}

impl SpeakerManager {
    /// The constructor and `EnterGamePlay`: four states, `mSpeakerIndex` their instance.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        Self { states: (0..STATES).map(|instance| SpeakerState::new(instance, vault, map)).collect(), multi_speakers: [None, None], multi_speaker_mode: false, submix_connected: false }
    }

    /// `mbIsMultiSpeakerMode`, which `PlayMissionSFXImp` sets for a contest's load and clears for
    /// its unload.
    pub fn set_multi_speaker_mode(&mut self, on: bool) {
        self.multi_speaker_mode = on;
    }

    #[must_use]
    pub fn multi_speaker_mode(&self) -> bool {
        self.multi_speaker_mode
    }

    /// `GetMultiSpeakerSubmix` 0x82550c18: the type's `MultiSpeaker` submix, none without it.
    #[must_use]
    pub fn multi_speaker_submix(&self, kind: usize) -> Option<PluginRef> {
        self.multi_speakers.get(kind)?.as_ref()?.sub_mix()
    }

    /// `ConnectSubmix` 0x825558a8: the voices instance's stream 0 sends to the speech
    /// `MultiSpeaker`.
    fn connect_submix(&mut self, speech: &Sk8SpeechSystem, core: &mut AudioCore) {
        let sub_mix = self.multi_speakers[SPEAKER_SPEECH].as_ref().and_then(MultiSpeaker::sub_mix);
        speech.set_instance_sub_mix(SPEECH_VOICES, sub_mix, 0, core);
        self.submix_connected = true;
    }

    /// `DisconnectSubmix` 0x82555948: the voices instance's speech stops and its stream 0 sends to
    /// the speech master again.
    fn disconnect_submix(&mut self, speech: &mut Sk8SpeechSystem, ctx: &mut SfxCtx<'_>) {
        if !self.submix_connected {
            return;
        }
        let reverb_sub_mix = ctx.global_effects.reverb_sub_mix();
        let mut speech_ctx = SpeechCtx { core: &mut *ctx.core, vault: ctx.vault, rng: &mut *ctx.rng, master_sub_mix: ctx.master_sub_mix, reverb_sub_mix };
        speech.stop_all_speech_on_instance(SPEECH_VOICES, &mut speech_ctx);
        if self.multi_speakers[SPEAKER_SPEECH].is_some() {
            speech.set_instance_sub_mix(SPEECH_VOICES, speech.speech_master_sub_mix(), 0, ctx.core);
        }
        self.submix_connected = false;
    }

    /// `Cleanup` 0x82555a18: the submix disconnects, every attached state detaches, the voices are
    /// freed and the characters instance gets its delay voices back.
    fn cleanup(&mut self, speech: &mut Sk8SpeechSystem, ctx: &mut SfxCtx<'_>) {
        self.disconnect_submix(speech, ctx);
        let Self { states, multi_speakers, .. } = self;
        for state in states.iter_mut().filter(|state| state.attached) {
            state.detach(multi_speakers, ctx);
        }
        for voice in multi_speakers.iter_mut() {
            if let Some(voice) = voice.take() {
                voice.destroy(ctx.core);
            }
        }
        speech.create_instance_delay_voice(SPEECH_CHARACTERS, ctx.core);
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} speakers cleaned up");
        }
    }

    /// `UpdateParams` 0x82557d58. In multi-speaker mode: the first update frees the characters'
    /// delay voices, builds both `MultiSpeaker`s (four speakers each, the type's longest delay
    /// from "multispeaker") and connects the submix; later ones match the first four speaker
    /// volumes to the states by identifier, detach the states no volume kept and attach a free
    /// state to each new volume. Out of the mode the voices are cleaned up. Then the base pass:
    /// each attached state's position controller, then its physics.
    pub fn update_params(
        &mut self,
        t: f32,
        emitter_system: &Sk8EmitterSystem,
        mics: &MicrophoneMan,
        shared: &mut Pos3dShared,
        speech: &mut Sk8SpeechSystem,
        music_master: PluginRef,
        ctx: &mut SfxCtx<'_>,
    ) {
        if self.multi_speaker_mode {
            if self.multi_speakers[SPEAKER_SPEECH].is_none() {
                speech.destroy_instance_delay_voice(SPEECH_CHARACTERS, ctx.core);
                for kind in 0..SPEAKER_TYPES {
                    let params = SpeakerParams { delay_max_delay: ctx.vault.f32(MULTISPEAKER, MAX_DELAY[kind]), ..SpeakerParams::DEFAULT };
                    let master = if kind == SPEAKER_MUSIC { Some(music_master) } else { speech.speech_master_sub_mix() };
                    self.multi_speakers[kind] = Some(MultiSpeaker::create(&[params; SPEAKERS], master, ctx.core));
                }
                self.connect_submix(speech, ctx.core);
                if let Some(elapsed) = crate::splicer::trace_elapsed() {
                    let built = self.multi_speakers.each_ref().map(|voice| voice.as_ref().map_or(0, |voice| voice.speakers.iter().flatten().count()));
                    eprintln!("{elapsed:9.3} speakers built: speech {} and music {} speaker voices", built[SPEAKER_SPEECH], built[SPEAKER_MUSIC]);
                }
            } else if !self.submix_connected {
                self.connect_submix(speech, ctx.core);
            } else {
                self.update_attachments(emitter_system, ctx);
            }
        } else if self.multi_speakers[SPEAKER_SPEECH].is_some() {
            self.cleanup(speech, ctx);
        }
        for state in &mut self.states {
            state.delta_time = t;
            if !state.attached {
                continue;
            }
            let refs = ObjRefs { dir: None, pos: Some(state.physics.position), vel: Some(state.physics.velocity) };
            state.position.update_params(t, refs, mics, shared, ctx.map);
            state.physics.update_params(state.speaker_pos, state.attrib, mics, ctx.vault, ctx.map);
        }
    }

    /// `UpdateParams`' connected pass over `GetVolumes(eVolumeType_Speaker, 4)`.
    fn update_attachments(&mut self, emitter_system: &Sk8EmitterSystem, ctx: &mut SfxCtx<'_>) {
        let mut volumes = [None; SPEAKERS];
        for (slot, id) in volumes.iter_mut().zip(emitter_system.volumes(VOLUME_SPEAKER, SPEAKERS, ctx.vault)) {
            *slot = emitter_system.volume_status(id).and_then(|view| view.volume.map(|(identifier, key)| (identifier, view.pos, key)));
        }
        let mut new = [false; SPEAKERS];
        for (slot, volume) in volumes.iter().enumerate() {
            let matched = volume.and_then(|(identifier, _, _)| self.states.iter_mut().find(|state| state.emitter_id == identifier));
            match matched {
                Some(state) => state.pending_detach = false,
                None => new[slot] = volume.is_some(),
            }
        }
        let Self { states, multi_speakers, .. } = self;
        for state in states.iter_mut().filter(|state| state.pending_detach && state.attached) {
            state.detach(multi_speakers, ctx);
        }
        let carried = [multi_speakers[SPEAKER_SPEECH].is_some(), multi_speakers[SPEAKER_MUSIC].is_some()];
        for (slot, volume) in volumes.iter().enumerate() {
            let Some((identifier, pos, key)) = *volume else { continue };
            if !new[slot] {
                continue;
            }
            if let Some(state) = states.iter_mut().find(|state| !state.attached) {
                state.attach(carried, identifier, pos, key, ctx.map);
            }
        }
    }

    /// `ProcessUpdate` 0x82550c10 (`CSTATEMGR_Base::ProcessUpdate`): each state's
    /// `CSTATE_Speaker::ProcessUpdate` 0x82551ae0 marks it for detaching, then its object updates
    /// while attached.
    pub fn process_update(&mut self, large_arena: bool, ctx: &mut SfxCtx<'_>) {
        let Self { states, multi_speakers, .. } = self;
        for state in states.iter_mut() {
            state.pending_detach = true;
            ctx.delta_time = state.delta_time;
            if !state.attached {
                continue;
            }
            state.object.process_update(state.speaker_index, state.attrib, &state.physics, large_arena, multi_speakers, ctx.vault, ctx.map, ctx.core);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn master(core: &mut AudioCore) -> PluginRef {
        let voice = core.create_voice(200, "master", &[PlugInConfig { kind: PlugInKind::SubMix { name: "Master".into() }, output_channels: 6 }]).expect("master");
        PluginRef { voice, index: 0 }
    }

    fn connected(core: &AudioCore, send: PluginRef) -> bool {
        core.plugin::<crate::rwac::plugins::send::Send>(send).is_some_and(crate::rwac::plugins::send::Send::is_connected)
    }

    #[test]
    fn a_multi_speaker_feeds_four_speakers_into_the_master_and_frees_them() {
        let mut core = AudioCore::new();
        let master = master(&mut core);
        let voice = MultiSpeaker::create(&[SpeakerParams::DEFAULT; SPEAKERS], Some(master), &mut core);
        core.render_frame();
        let multi = voice.voice.expect("multi-speaker voice");
        let speakers: Vec<VoiceId> = voice.speakers.iter().map(|speaker| speaker.as_ref().expect("speaker").voice).collect();
        for (index, speaker) in speakers.iter().enumerate() {
            assert!(connected(&core, PluginRef { voice: multi, index: index as u8 + 1 }));
            assert!(connected(&core, PluginRef { voice: *speaker, index: SPEAKER_SEND }));
        }
        voice.destroy(&mut core);
        core.render_frame();
        assert!(core.voice(multi).is_none());
        assert!(speakers.iter().all(|speaker| core.voice(*speaker).is_none()));
    }

    #[test]
    fn a_speaker_sets_what_changed_unless_forced() {
        let mut core = AudioCore::new();
        let master = master(&mut core);
        let speaker = Speaker::create(&SpeakerParams::DEFAULT, Some(master), &mut core).expect("speaker");
        core.set_attribute(speaker.plugin(SPEAKER_GAIN), 0, 0.5);
        core.render_frame();
        speaker.update(&SpeakerParams::DEFAULT, false, &SpeakerParams::DEFAULT, &mut core);
        core.render_frame();
        assert_eq!(core.attribute(speaker.plugin(SPEAKER_GAIN), 0), 0.5);
        speaker.update(&SpeakerParams::DEFAULT, true, &SpeakerParams::DEFAULT, &mut core);
        core.render_frame();
        assert_eq!(core.attribute(speaker.plugin(SPEAKER_GAIN), 0), 0.0);
        let turned = SpeakerParams { gain: 0.25, azimuth: 16384, ..SpeakerParams::DEFAULT };
        speaker.update(&turned, false, &SpeakerParams::DEFAULT, &mut core);
        core.render_frame();
        assert_eq!(core.attribute(speaker.plugin(SPEAKER_GAIN), 0), 0.25);
        assert_eq!(core.attribute(speaker.plugin(SPEAKER_PAN), 0), 90.0);
    }
}
