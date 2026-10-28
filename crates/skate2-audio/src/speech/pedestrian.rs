//! `SFXObj_PedestrianSpeech` (PEDESTRIAN object 0, 0xA4 bytes, `CreateObject` 0x8252e920): a
//! pedestrian's speech. Its audio state picks living-world events (or, for main-cast and
//! announcer characters, context triggers) and its mixer outputs become the `VoiceInfo` the
//! speech instance playing for it follows; a pedestrian answering its phone rings first.

use super::mission::{CameraManParams, MainCastParams};
use super::{LivingWorldParams, Sk8SpeechSystem, SpeechCtx, VoiceInfoRef, CHANNEL_LIVING_WORLD, SPEECH_DEFAULT};
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::physics::point_graph;
use crate::splicer::{SpliceId, SpliceManager, SplicePlayParams, SpliceUpdateParams};
use crate::vault::Vault;

/// `ePedestrianAudioState`.
const STATE_COLLISION_STANDING: u32 = 7;
const STATE_COLLIDING: u32 = 8;
const STATE_PICTURE_TAKING: u32 = 29;
const STATE_BUMPED_WHILE_TAKING_PICTURES: u32 = 30;
const STATE_CELL_HELLO: u32 = 49;
const STATE_PED_GRUNT: u32 = 51;
const STATE_CELL_GREET_SPEECH: u32 = 70;
const STATE_MAX: u32 = 72;
/// `SPCH1Type_near_far`.
const FAR: u32 = 1;
const NEAR: u32 = 2;
/// `SPCH1Type_CharType` `securityguard` and `kSPCH1_EventID_102_Radio`.
const SECURITY_GUARD: u32 = 0x40;
const RADIO: u16 = 8277;
/// `SPCH0Type_IP_ID`: Atiba and Burnette (the photographers), and Big Black.
const IP_ATIBA: u32 = 128;
const IP_BURNETTE: u32 = 256;
const IP_BIG_BLACK: u32 = 1;
/// The ring splice: `Splicer::Splice(27, 5, AudioCore::GetMasterSubMix())`.
const RING_BANK: usize = 27;
const RING_INDEX: i32 = 5;
/// `aud_speech` "default": the volume at or below which a speaking pedestrian counts as quiet,
/// how many quiet updates stop it, the updates between delay updates, the distance factor of the
/// delay and the cameraman's proximity distance.
const LOW_VOLUME: u64 = 0x6995_C510_258C_9AF6;
const LOW_VOLUME_UPDATES: u64 = 0x3D8C_D05C_962F_F399;
const DELAY_UPDATES: u64 = 0x510B_AFA3_2A76_B340;
const DISTANCE_FACTOR: u64 = 0xBF48_032D_B145_C5B4;
const CAMERAMAN_TRIGGER_DISTANCE: u64 = 0x8EA5_0947_E846_3836;
/// `1 / 344` (the speed of sound) and the longest delay.
const SPEED_OF_SOUND_RECIP: f32 = f32::from_bits(0x3B3E_82FA);
const MAX_DELAY: f32 = f32::from_bits(0x3E19_999A);
/// `1 / 32767`, `1 / 4096` and `360 / 65535`.
const VOLUME_SCALE: f32 = f32::from_bits(0x3800_0100);
const PITCH_SCALE: f32 = f32::from_bits(0x3980_0000);
const AZIMUTH_SCALE: f32 = f32::from_bits(0x3BB4_00B4);

/// The `PedAudioState` fields the object reads.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PedestrianSpeechInput {
    pub pedestrian_id: u32,
    pub character: u32,
    pub state: u32,
    pub was_player_collision: bool,
    pub char_type: u32,
    pub char_spch_id: u32,
    pub lw_class: u32,
    pub topic: u32,
    pub mc_pro_id: u32,
    pub mc_ip_id: u32,
    pub mc_pro_class: u32,
    pub mc_ip_class: u32,
    pub mc_lw_class: u32,
    pub cm_lw_type: u32,
    pub cm_lw_gender: u32,
    pub cm_pro_id: u32,
    pub cm_ip_id: u32,
    pub an_char_id: u32,
    pub distance: f32,
    pub volume_factor: f32,
    pub near_far_distance: f32,
}

impl PedestrianSpeechInput {
    /// `SPCH1Type_near_far`: near within `near_far_distance`.
    fn near_far(&self) -> u32 {
        if self.distance <= self.near_far_distance {
            NEAR
        } else {
            FAR
        }
    }

    fn living_world_params(&self) -> LivingWorldParams {
        LivingWorldParams {
            char_type: self.char_type,
            char_id: self.char_spch_id,
            near_far: self.near_far(),
            lw_class: self.lw_class,
            topic: self.topic,
            cm_lw_type: self.cm_lw_type,
            cm_lw_gender: self.cm_lw_gender,
        }
    }

    /// `UpdateSpeech`'s `tMainCastParams` for a main-cast pedestrian: its ids and classes, near
    /// or far.
    fn main_cast_params(&self) -> MainCastParams {
        MainCastParams {
            pro_id: self.mc_pro_id as i32,
            ip_id: self.mc_ip_id as i32,
            near_far: if self.near_far() == FAR { 2 } else { 1 },
            lw_class: self.mc_lw_class as i32,
            pro_class: self.mc_pro_class as i32,
            ip_class: self.mc_ip_class as i32,
            ..MainCastParams::default()
        }
    }
}

/// `SFXObj_PedestrianSpeech` (its `mVoiceInfo` is the speech system's for its state).
pub struct PedestrianSpeech {
    pub io: Option<DMixIoId>,
    state_index: usize,
    previous_audio_state: u32,
    time_in_audio_state: f32,
    speech_event_id: u16,
    cell_phone_ring: Option<SpliceId>,
    low_volume_count: i32,
    played_proximity_speech: bool,
    delay_count: i32,
}

impl PedestrianSpeech {
    /// The constructor (`mPreviousAudioState` = `ePedestrianAudioState_Max`, `mDelayCount` = -1).
    #[must_use]
    pub fn new(state_index: usize, io: Option<DMixIoId>) -> Self {
        Self {
            io,
            state_index,
            previous_audio_state: STATE_MAX,
            time_in_audio_state: 0.0,
            speech_event_id: super::lw_event::MAX,
            cell_phone_ring: None,
            low_volume_count: 0,
            played_proximity_speech: false,
            delay_count: -1,
        }
    }

    fn voice_info(&self) -> VoiceInfoRef {
        VoiceInfoRef::Pedestrian(self.state_index)
    }

    /// `InitSFX` 0x82516810: the output on; the voice is the pedestrian's, attached, on no stream.
    pub fn init_sfx(&mut self, input: &PedestrianSpeechInput, speech: &mut Sk8SpeechSystem, map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        let info = speech.voice_info_mut(self.voice_info());
        info.speaker_id = input.pedestrian_id;
        info.character = input.character as i32;
        info.is_attached = true;
        info.instance_stream = 2;
    }

    /// `Detach` 0x82516908: the voice detaches and the ring is deleted.
    pub fn detach(&mut self, speech: &mut Sk8SpeechSystem, splicer: &mut SpliceManager, core: &mut crate::rwac::core::AudioCore) {
        speech.voice_info_mut(self.voice_info()).is_attached = false;
        self.low_volume_count = 0;
        self.played_proximity_speech = false;
        if let Some(ring) = self.cell_phone_ring.take() {
            splicer.release_splice(core, ring);
        }
    }

    /// `UpdateParams` 0x8254dfe0: `UpdateSpeech`, then `ProximitySpeech`.
    pub fn update_params(&mut self, t: f32, input: &PedestrianSpeechInput, speech: &mut Sk8SpeechSystem, splicer: &mut SpliceManager, ctx: &mut SpeechCtx<'_>) {
        self.update_speech(t, input, speech, splicer, ctx);
        self.proximity_speech(input, speech, ctx);
    }

    /// `UpdateSpeech` 0x82545eb0: a new audio state (or taking pictures again) speaks: a phone
    /// rings; a bump becomes a grunt (or, while taking pictures, the bumped state); announcer
    /// characters send the announcer a trigger, living-world characters play the state's event and
    /// the main cast send theirs.
    fn update_speech(&mut self, t: f32, input: &PedestrianSpeechInput, speech: &mut Sk8SpeechSystem, splicer: &mut SpliceManager, ctx: &mut SpeechCtx<'_>) {
        let state = input.state;
        if state == self.previous_audio_state && state != STATE_PICTURE_TAKING {
            self.previous_audio_state = state;
            self.time_in_audio_state += t;
            return;
        }
        let info = *speech.voice_info(self.voice_info());
        let stream = info.instance_stream;
        self.time_in_audio_state = 0.0;
        if state == STATE_CELL_HELLO {
            self.play_cell_ring(t, splicer, ctx);
            self.previous_audio_state = STATE_CELL_HELLO;
            return;
        }
        let mut speech_state = state;
        if state == STATE_COLLISION_STANDING || state == STATE_COLLIDING {
            speech_state = if self.previous_audio_state == STATE_PICTURE_TAKING { STATE_BUMPED_WHILE_TAKING_PICTURES } else { STATE_PED_GRUNT };
        }
        let speaking = info.is_actually_playing;
        if input.an_char_id != 0 {
            speech.play_announcer_speech_sample(ctx, speech_state, input.an_char_id);
        } else if input.mc_lw_class != 0 {
            let params = input.living_world_params();
            self.speech_event_id = speech.play_pedestrian_speech_sample(ctx, input.character as i32, &params, speech_state, input.pedestrian_id, stream, speaking, input.was_player_collision);
        } else {
            speech.play_main_cast_speech_sample(ctx, speech_state, &input.main_cast_params(), stream, speaking, input.was_player_collision);
        }
        self.previous_audio_state = state;
    }

    /// `ProximitySpeech` 0x8252ed88: once a pedestrian comes within the cameraman's trigger
    /// distance the cameraman's proximity trigger goes out with its cameraman pro and IP ids
    /// (`PlayCameraManSpeech`), and Big Black's arrival to the main cast with his IP id
    /// (`PlayMainCastSpeech`).
    fn proximity_speech(&mut self, input: &PedestrianSpeechInput, speech: &mut Sk8SpeechSystem, ctx: &mut SpeechCtx<'_>) {
        let distance = input.distance;
        if !(distance > 0.0) || !(distance < ctx.vault.f32(SPEECH_DEFAULT, CAMERAMAN_TRIGGER_DISTANCE)) || self.played_proximity_speech {
            return;
        }
        let mut camera_man = CameraManParams { ip_id: input.cm_ip_id as i32, pro_id: input.cm_pro_id as i32, ..CameraManParams::default() };
        speech.play_camera_man_speech("T_4999_In_Prox", &mut camera_man, ctx);
        self.played_proximity_speech = true;
        if input.mc_ip_id == IP_BIG_BLACK {
            let main_cast = MainCastParams { ip_id: input.mc_ip_id as i32, ..MainCastParams::default() };
            speech.play_main_cast_speech("T_621_Arrival", &main_cast, ctx);
        }
    }

    /// `PlayCellRing` 0x8252eca8: a new silent ring splice.
    fn play_cell_ring(&mut self, t: f32, splicer: &mut SpliceManager, ctx: &mut SpeechCtx<'_>) {
        if let Some(ring) = self.cell_phone_ring.take() {
            splicer.release_splice(ctx.core, ring);
        }
        self.cell_phone_ring = splicer.create_splice(ctx.core, RING_BANK, RING_INDEX, ctx.master_sub_mix);
        if let Some(ring) = self.cell_phone_ring {
            let update = SpliceUpdateParams { vol: 0.0, pitch: 1.0, az: 0.0, delta: t, spread: 1.0, time_stretch: 1.0 };
            splicer.play_splice(ctx.core, ring, SplicePlayParams::default(), &update);
        }
    }

    /// `ProcessUpdate` 0x82545e98: `UpdatePedVoiceInfo` while attached.
    pub fn process_update(&mut self, attached: bool, delta_time: f32, input: &PedestrianSpeechInput, speech: &mut Sk8SpeechSystem, splicer: &mut SpliceManager, map: &MixMap, ctx: &mut SpeechCtx<'_>) {
        if attached {
            self.update_ped_voice_info(delta_time, input, speech, splicer, map, ctx);
        }
    }

    /// `UpdatePedVoiceInfo` 0x825459c8: the mixer's azimuth, pitch, the dry and wet volumes of the
    /// pedestrian's kind (pros, photographers, other main cast, guards on the radio, guards,
    /// others; each near or far) scaled by its volume factor, the delay send and filters, the
    /// notch by azimuth, the delay by distance every few updates; a quiet speaker stops.
    fn update_ped_voice_info(&mut self, delta_time: f32, input: &PedestrianSpeechInput, speech: &mut Sk8SpeechSystem, splicer: &mut SpliceManager, map: &MixMap, ctx: &mut SpeechCtx<'_>) {
        let io = self.io;
        let output = |index: i32, kind: PresetType| io.map_or(0, |io| map.output(io, index, kind));
        let at = self.voice_info();
        let previous = *speech.voice_info(at);
        let far = i32::from(previous.is_far);
        let (dry, wet) = if input.mc_pro_id != 0 {
            (9 + far, 19)
        } else if input.mc_ip_id != 0 {
            if input.mc_ip_id == IP_ATIBA || input.mc_ip_id == IP_BURNETTE {
                (11 + far, 20)
            } else {
                (7 + far, 18)
            }
        } else if input.mc_ip_class != 0 {
            (7 + far, 18)
        } else if input.char_type == SECURITY_GUARD {
            if previous.is_actually_playing && previous.sed_event == RADIO {
                (6, 17)
            } else {
                (4 + far, 16)
            }
        } else {
            (2 + far, 15)
        };
        let factor = input.volume_factor;
        let scaled = |value: i32| (value as f32 * factor) as i32;
        let mut info = previous;
        info.azimuth = output(0, PresetType::Azimuth);
        info.pitch = output(1, PresetType::Pitch);
        info.vol = scaled(output(dry, PresetType::Volume));
        info.wet_vol = scaled(output(wet, PresetType::Volume));
        info.delay_vol = scaled(output(21, PresetType::Volume));
        info.delay_high_pass = output(22, PresetType::Frequency);
        info.delay_low_pass = output(23, PresetType::Frequency);
        *speech.voice_info_mut(at) = info;
        let stream = info.instance_stream;
        if info.is_actually_playing {
            let mut reset = true;
            if info.vol <= ctx.vault.i32(SPEECH_DEFAULT, LOW_VOLUME) {
                self.low_volume_count += 1;
                if self.low_volume_count <= ctx.vault.i32(SPEECH_DEFAULT, LOW_VOLUME_UPDATES) {
                    reset = false;
                } else {
                    speech.stop_speech_by_channel(CHANNEL_LIVING_WORLD, stream, ctx);
                }
            }
            if reset {
                self.low_volume_count = 0;
            }
            if let Some(speaking) = usize::try_from(stream).ok().and_then(|stream| speech.speaking_ped_info_mut(stream)) {
                speaking.current_volume = info.vol;
                speaking.current_distance = input.distance;
            }
        }
        let azimuth = info.azimuth as u32;
        let info = speech.voice_info_mut(at);
        info.notch_gain = notch_graph(ctx.vault, 80, azimuth);
        info.notch_freq = notch_graph(ctx.vault, 160, azimuth);
        info.notch_q = notch_graph(ctx.vault, 0, azimuth);
        if self.delay_count <= 0 {
            let delay = ctx.vault.f32(SPEECH_DEFAULT, DISTANCE_FACTOR) * input.distance * SPEED_OF_SOUND_RECIP;
            info.delay = if MAX_DELAY - delay >= 0.0 { delay } else { MAX_DELAY };
        }
        self.delay_count = if self.delay_count >= 0 { self.delay_count - 1 } else { ctx.vault.i32(SPEECH_DEFAULT, DELAY_UPDATES) };
        info.high_pass = output(13, PresetType::Frequency);
        info.low_pass = output(14, PresetType::Frequency);
        self.update_cell_phone_ring(delta_time, input, speech, splicer, ctx);
    }

    /// `UpdateCellPhoneRing` 0x8252ead0: the ring follows the voice; when it ends the pedestrian
    /// greets its caller (`CellGreetSpeech`).
    fn update_cell_phone_ring(&mut self, delta_time: f32, input: &PedestrianSpeechInput, speech: &mut Sk8SpeechSystem, splicer: &mut SpliceManager, ctx: &mut SpeechCtx<'_>) {
        let Some(ring) = self.cell_phone_ring else { return };
        let info = *speech.voice_info(self.voice_info());
        if splicer.is_playing(ring) {
            let mut update = SpliceUpdateParams {
                vol: info.vol as f32 * VOLUME_SCALE,
                pitch: info.pitch as f32 * PITCH_SCALE,
                az: info.azimuth as f32 * AZIMUTH_SCALE,
                delta: delta_time,
                spread: 0.0,
                time_stretch: 1.0,
            };
            splicer.update_splice(ctx.core, ring, &mut update);
            return;
        }
        splicer.release_splice(ctx.core, ring);
        self.cell_phone_ring = None;
        let params = input.living_world_params();
        self.speech_event_id = speech.play_pedestrian_speech_sample(
            ctx,
            input.character as i32,
            &params,
            STATE_CELL_GREET_SPEECH,
            input.pedestrian_id,
            info.instance_stream,
            info.is_actually_playing,
            false,
        );
    }
}

/// `GetNotchQ` 0x8252f170 / `GetNotchGain` 0x8252f0c0 / `GetNotchFreq` 0x8252f010: an 8-point
/// graph of the `aud_speech` "default" layout (x at `base + 16`, y at `base + 48`) over the
/// azimuth folded to 0..32768.
fn notch_graph(vault: &Vault, base: u32, azimuth: u32) -> f32 {
    let value = azimuth as f32;
    let folded = if value > 32767.0 { 65536.0 - value } else { value };
    let read = |offset: u32| -> [f32; 8] { std::array::from_fn(|i| vault.layout_f32(SPEECH_DEFAULT, (offset + 4 * i as u32) / 4)) };
    point_graph(&read(base + 16), &read(base + 48), folded)
}
