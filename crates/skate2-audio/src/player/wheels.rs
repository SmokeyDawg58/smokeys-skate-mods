//! `Sk8::Audio::SFXObj_Wheels` (PLAYER object 2, TypeInfo 0x00010020, `CreateObject` 0x8254d7a0):
//! the wheels spinning in the air, in a manual, and in the local skater's hands off the board.
//!
//! A kind that begins plays the Jump or the Man spin-down sample of `data\audio` (alternately)
//! through its own voice, from further into the sample the slower the board rolls, into the wheels
//! sub mix: a peaking and a high-shelf filter the local skater's game speed and wheel occlusion
//! steer, the flange and reverb sends, and the pan to the master mix. The sub mix is built on the
//! first attach (natively by the constructor, where the port has no audio core).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::physics::PlayerPhysics;
use super::skateboard::GrainFiles;
use super::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::nicotine::{DMixIoId, PresetType};
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::decoder::DataRef;
use crate::rwac::plugins::snd_player::PlayParams;
use crate::rwac::{PluginRef, VoiceId, VoiceState};
use crate::utils::clamp_fsel;
use crate::vault::{class, Collection, Vault};
use std::sync::Arc;

/// `GlobalAttributeCollection::aud_general_wheels` (`aud_general` "wheels").
const WHEELS: Collection = Collection::new(class::AUD_GENERAL, 0x03B7_10C8_0E1A_C13E);
/// The Jump and Man spins: sample and seek table names (`char*`), lengths in seconds.
const SAMPLE_NAME: [u64; 2] = [0x044F_B3EC_B9FC_B35F, 0x2431_17D2_CD2E_DC70];
const SEEK_TABLE_NAME: [u64; 2] = [0x82F1_E6D5_A030_0576, 0x6CD2_B3AB_6ACF_77C4];
const SAMPLE_LENGTH: [u64; 2] = [0xFD87_4514_FD49_261E, 0x8CC3_1309_D10F_1763];
/// `ProcessWheels`' board speed divisor.
const MAX_SPEED: u64 = 0x4890_392C_9182_9954;
/// `UpdateSubmix`: the peaking frequency occluded and open (`int`), its gain and Q; the high-shelf
/// frequency (`int`) and its gain occluded and open.
const PEAK_FREQUENCY_OCCLUDED: u64 = 0xCB85_086F_A8B9_31A1;
const PEAK_FREQUENCY: u64 = 0x5DDF_8C07_AC1D_35C4;
const PEAK_GAIN: u64 = 0x3EC2_F171_6EF2_0A56;
const PEAK_Q: u64 = 0xCF8F_01B5_7376_5924;
const SHELF_FREQUENCY: u64 = 0xE775_9E1A_6909_00F2;
const SHELF_GAIN_OCCLUDED: u64 = 0x28F5_7A87_418A_5CF3;
const SHELF_GAIN: u64 = 0x23B4_1479_F58E_FF7F;

/// `eWheelsType`.
const IN_AIR: usize = 0;
const IN_MANUAL: usize = 1;
const HOLDING_BOARD: usize = 2;

/// `__real_40666666` (3.6) and `__real_3e851eb8` (0.26): the board speed scale and the cap the
/// held board's speed fraction keeps.
const SPEED_SCALE: f32 = f32::from_bits(0x4066_6666);
const HOLDING_MAX: f32 = f32::from_bits(0x3E85_1EB8);

/// A voice's plug-ins (`StartWheelVoice`): SndPlayer1, Rechannel, Resample, Gain, Send.
const VOICE_RESAMPLE: u8 = 2;
const VOICE_GAIN: u8 = 3;
const VOICE_SEND: u8 = 4;
/// The sub mix's (`CreateWheelsSubmix`): SubMix, PeakingIir2, HighShelfIir2, the flange Send, the
/// reverb Send, Pan2D1 (six channels), the master Send.
const SUBMIX_PEAKING: u8 = 1;
const SUBMIX_HIGH_SHELF: u8 = 2;
const SUBMIX_FLANGE_SEND: u8 = 3;
const SUBMIX_REVERB_SEND: u8 = 4;
const SUBMIX_PAN: u8 = 5;
const SUBMIX_MASTER_SEND: u8 = 6;

/// `Sk8::Audio::SFXObj_Wheels` (100 bytes).
pub struct Wheels {
    pub io: Option<DMixIoId>,
    /// `mpWheelsData`, `mpSeekTable`, `mSampleLength`: Jump, Man.
    data: [Option<DataRef>; 2],
    seek_tables: [Option<DataRef>; 2],
    sample_length: [f32; 2],
    /// `mbPlayingAirWheels`, `mbPlayingManualWheels`, `mbPlayingHoldingBoardWheels`.
    playing: [bool; 3],
    /// `mpWheelsSubmix`.
    submix: Option<VoiceId>,
    /// `mpWheelsVoice`.
    voices: [Option<VoiceId>; 3],
    /// `mbPlayData1`: the next start plays the Jump spin.
    play_data_1: bool,
}

impl Wheels {
    /// The constructor 0x82544158: the spins' samples and seek tables by their `aud_general`
    /// names (`data\audio\%s`), and their lengths.
    pub fn new(vault: &Vault, files: &GrainFiles) -> Self {
        let file = |key: u64| {
            vault
                .text(WHEELS, key)
                .and_then(|name| files.get(&name.to_lowercase()))
                .map(|data| DataRef { bytes: Arc::clone(&data.bytes), offset: 0 })
        };
        Self {
            io: None,
            data: SAMPLE_NAME.map(&file),
            seek_tables: SEEK_TABLE_NAME.map(&file),
            sample_length: SAMPLE_LENGTH.map(|key| vault.f32(WHEELS, key)),
            playing: [false; 3],
            submix: None,
            voices: [None; 3],
            play_data_1: false,
        }
    }

    /// `InitSFX` 0x825151b0: the output on and the Jump spin next.
    pub fn init_sfx(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
        self.play_data_1 = true;
        if self.submix.is_none() {
            self.create_wheels_submix(ctx);
        }
    }

    /// `Destroy` 0x825445e8: the voices; the sub mix and the playing flags stay.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        for kind in [IN_AIR, IN_MANUAL, HOLDING_BOARD] {
            self.free_wheel_voice(kind, ctx);
        }
    }

    /// `ProcessUpdate` 0x8254f990 (the object has no `UpdateParams`).
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        self.process_wheels(IN_AIR, physics.in_air, physics, ctx);
        self.process_wheels(IN_MANUAL, physics.is_manual, physics, ctx);
        let holding = ctx.local && ((physics.is_off_board && physics.is_holding_board) || physics.is_hand_plant);
        self.process_wheels(HOLDING_BOARD, holding, physics, ctx);
        self.update_submix(physics, ctx);
    }

    /// `ProcessWheels` 0x8254d908: a kind starts its voice when it begins, follows mixer outputs 1
    /// (or 5 in a manual, 6 holding the board, 7 in a hand plant) and 2 while it lasts, and frees
    /// the voice when the kind ends or the voice is expelled.
    fn process_wheels(&mut self, kind: usize, play: bool, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !self.playing[kind] {
            if !play {
                return;
            }
            self.playing[kind] = true;
            let mut speed = clamp_fsel((physics.board_speed / ctx.vault.f32(WHEELS, MAX_SPEED)) * SPEED_SCALE, 1.0);
            if kind == HOLDING_BOARD && !(speed <= HOLDING_MAX) {
                speed = HOLDING_MAX;
            }
            let data = usize::from(!self.play_data_1);
            self.play_data_1 = !self.play_data_1;
            self.start_wheel_voice(kind, data, self.sample_length[data] * (1.0 - speed), ctx);
            if ctx.local {
                let flange = ctx.output(self.io, 4, PresetType::Volume) as f32 * VOLUME_SCALE;
                if let Some(submix) = self.submix {
                    ctx.core.set_attribute(PluginRef { voice: submix, index: SUBMIX_FLANGE_SEND }, 0, flange);
                }
            }
            return;
        }
        if !play {
            self.free_wheel_voice(kind, ctx);
            self.playing[kind] = false;
            return;
        }
        let Some(voice) = self.voices[kind] else { return };
        if ctx.core.voice_state(voice) == Some(VoiceState::Expelled) {
            self.free_wheel_voice(kind, ctx);
            return;
        }
        let mut volume = ctx.output(self.io, 1, PresetType::Volume) as f32 * VOLUME_SCALE;
        let output = if physics.is_manual {
            Some(5)
        } else if physics.is_holding_board {
            Some(6)
        } else if physics.is_hand_plant {
            Some(7)
        } else {
            None
        };
        if let Some(index) = output {
            volume = ctx.output(self.io, index, PresetType::Volume) as f32 * VOLUME_SCALE;
        }
        let pitch = ctx.output(self.io, 2, PresetType::Pitch) as f32 * PITCH_SCALE;
        self.update_wheels_voice(kind, volume, pitch, ctx);
    }

    /// `StartWheelVoice` 0x82544630: the spin from `time` seconds (event 5 with the seek table),
    /// silent until the next update, sent into the sub mix.
    fn start_wheel_voice(&mut self, kind: usize, data: usize, time: f32, ctx: &mut SfxCtx<'_>) {
        if self.voices[kind].is_some() {
            return;
        }
        let mono = |kind: PlugInKind| PlugInConfig { kind, output_channels: 1 };
        self.voices[kind] = ctx.core.create_voice(
            1,
            "Wheels",
            &[
                mono(PlugInKind::SndPlayer1 { max_requests: 1.0 }),
                mono(PlugInKind::Rechannel),
                mono(PlugInKind::Resample),
                mono(PlugInKind::Gain),
                mono(PlugInKind::Send),
            ],
        );
        let Some(voice) = self.voices[kind] else { return };
        let params = PlayParams {
            seek_time: f64::from(time),
            ram_sample: self.data[data].clone(),
            seek_table: self.seek_tables[data].clone(),
            expel_mode: 1.0,
            ..PlayParams::default()
        };
        ctx.core.play(PluginRef { voice, index: 0 }, params);
        self.update_wheels_voice(kind, 0.0, 1.0, ctx);
        if let Some(submix) = self.submix {
            ctx.core.connect_send(PluginRef { voice, index: VOICE_SEND }, Some(PluginRef { voice: submix, index: 0 }));
        }
    }

    /// `UpdateWheelsVoice` 0x82528188: the Gain's volume and the Resample's pitch.
    fn update_wheels_voice(&self, kind: usize, volume: f32, pitch: f32, ctx: &mut SfxCtx<'_>) {
        if let Some(voice) = self.voices[kind] {
            ctx.core.set_attribute(PluginRef { voice, index: VOICE_GAIN }, 0, volume);
            ctx.core.set_attribute(PluginRef { voice, index: VOICE_RESAMPLE }, 0, pitch);
        }
    }

    /// `FreeWheelVoice` 0x82528268.
    fn free_wheel_voice(&mut self, kind: usize, ctx: &mut SfxCtx<'_>) {
        if let Some(voice) = self.voices[kind].take() {
            ctx.core.release_voice(voice);
        }
    }

    /// `CreateWheelsSubmix` 0x825279e0: the stage-2 sub mix. The master Send feeds the master sub
    /// mix, the reverb Send `GlobalEffects::GetReverbSubMix` and the flange Send
    /// `GetFlangeSubMix(0x2000)`, which starts silent.
    fn create_wheels_submix(&mut self, ctx: &mut SfxCtx<'_>) {
        let config = |kind: PlugInKind, output_channels: u8| PlugInConfig { kind, output_channels };
        self.submix = ctx.core.create_voice(
            2,
            "Wheels",
            &[
                config(PlugInKind::SubMix { name: String::new() }, 1),
                config(PlugInKind::PeakingIir2, 1),
                config(PlugInKind::HighShelfIir2, 1),
                config(PlugInKind::Send, 1),
                config(PlugInKind::Send, 1),
                config(PlugInKind::Pan2D1 { constructor: None }, 6),
                config(PlugInKind::Send, 6),
            ],
        );
        let Some(submix) = self.submix else { return };
        ctx.core.connect_send(PluginRef { voice: submix, index: SUBMIX_MASTER_SEND }, Some(ctx.master_sub_mix));
        ctx.core.connect_send(PluginRef { voice: submix, index: SUBMIX_REVERB_SEND }, Some(ctx.global_effects.reverb_sub_mix()));
        ctx.core.connect_send(PluginRef { voice: submix, index: SUBMIX_FLANGE_SEND }, Some(ctx.global_effects.flange_sub_mix(0x2000)));
        ctx.core.set_attribute(PluginRef { voice: submix, index: SUBMIX_FLANGE_SEND }, 0, 0.0);
    }

    /// `UpdateSubmix` 0x82527d68 while a voice exists: the pan (output 0); for the local skater
    /// the filters (the game speed, at most 1, scales their frequencies and the wheel occlusion
    /// blends to the occluded values) and the flange send (output 4); the reverb send (output 3).
    fn update_submix(&self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let Some(submix) = self.submix else { return };
        if self.voices.iter().all(Option::is_none) {
            return;
        }
        let plugin = |index| PluginRef { voice: submix, index };
        let azimuth = ctx.output(self.io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE;
        ctx.core.set_attribute(plugin(SUBMIX_PAN), 0, azimuth);
        if ctx.local {
            let vault = ctx.vault;
            let occluded = vault.i32(WHEELS, PEAK_FREQUENCY_OCCLUDED) as f32;
            let open = vault.i32(WHEELS, PEAK_FREQUENCY) as f32;
            let peak_gain = vault.f32(WHEELS, PEAK_GAIN);
            let peak_q = vault.f32(WHEELS, PEAK_Q);
            let shelf_frequency = vault.i32(WHEELS, SHELF_FREQUENCY) as f32;
            let shelf_occluded = vault.f32(WHEELS, SHELF_GAIN_OCCLUDED);
            let shelf_open = vault.f32(WHEELS, SHELF_GAIN);
            let speed = if physics.game_speed <= 1.0 { physics.game_speed } else { 1.0 };
            let occlusion = physics.wheel_occlusion;
            let base = speed * open;
            let frequency = speed.mul_add(occluded, -base).mul_add(occlusion, base);
            ctx.core.set_attribute(plugin(SUBMIX_PEAKING), 0, frequency);
            ctx.core.set_attribute(plugin(SUBMIX_PEAKING), 1, peak_gain);
            ctx.core.set_attribute(plugin(SUBMIX_PEAKING), 2, peak_q);
            let flange = ctx.output(self.io, 4, PresetType::Volume) as f32 * VOLUME_SCALE;
            ctx.core.set_attribute(plugin(SUBMIX_HIGH_SHELF), 0, speed * shelf_frequency);
            ctx.core.set_attribute(plugin(SUBMIX_HIGH_SHELF), 1, (shelf_occluded - shelf_open).mul_add(occlusion, shelf_open));
            ctx.core.set_attribute(plugin(SUBMIX_FLANGE_SEND), 0, flange);
        }
        let reverb = ctx.output(self.io, 3, PresetType::Volume) as f32 * VOLUME_SCALE;
        ctx.core.set_attribute(plugin(SUBMIX_REVERB_SEND), 0, reverb);
    }
}
