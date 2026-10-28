//! `Sk8::Audio::MusicDspVoice`: a stage 2 voice of `SubMix`, an optional `Delay`, `PeakingIir2`,
//! `HighPassIir2`, `LowPassIir2` and `Send`, which the music stream's dry send (voice 0) or the
//! DJ locations' auxiliary channel set (voice 1) runs through. Its settings come from an
//! `aud_music` collection (the playlist's music, or `aux_dsp_info`) and the music object's wet
//! volume and low-pass outputs.

use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
use crate::rwac::{PluginRef, VoiceId};
use crate::vault::{Collection, Vault};

/// `Attrib::Gen::aud_music`.
pub const AUD_MUSIC: u64 = 0x87EC_8C46_B85E_1D98;
/// The `aud_music` collection the constructor starts from.
const DEFAULT_DSP_INFO: u64 = 0x0805_BEC9_C8BA_DDF4;
/// `aud_music::timeStretch` and `aud_music::pitchScalar`.
const TIME_STRETCH: u64 = 0xDE8A_8BF1_46FE_E36B;
const PITCH_SCALAR: u64 = 0x272C_8845_5CBA_E742;
/// `0.01f`.
const HUNDREDTH: f32 = f32::from_bits(0x3C23_D70A);

/// What `CheckAttribulator` reads from the music system: `CalcWetVolume` 0x824f7ff0 and
/// `GetNicotineLPF` 0x824f8078, and the `kTestLowPass`/`kGameSpeedLowPassScalar` percentages.
#[derive(Debug, Clone, Copy)]
pub struct DspInputs {
    pub wet_volume: f32,
    pub nicotine_lpf: f32,
    pub test_low_pass: i32,
    pub game_speed_low_pass_scalar: i32,
}

/// `Sk8::Audio::MusicDspVoice`.
#[derive(Debug, Clone)]
pub struct MusicDspVoice {
    mono_reverb_gain_changed: bool,
    game_speed_changed: bool,
    time_stretch_changed: bool,
    pitch_mult_changed: bool,
    /// `mPathTrackID`: the tracks `PATH_channelsvolume` sets the reverb channel set of (0 is none).
    pub path_track_id: u32,
    /// `mReverbChannelSet` (-1 is none).
    pub reverb_channel_set: i32,
    reverb_channel_set_gain: f32,
    mono_reverb_gain: f32,
    time_stretch: f32,
    pitch_mult: f32,
    voice: Option<VoiceId>,
    plugin_submix: Option<u8>,
    plugin_delay: Option<u8>,
    plugin_peaking: Option<u8>,
    plugin_high_pass: Option<u8>,
    plugin_low_pass: Option<u8>,
    plugin_send: Option<u8>,
    pub enable_delay: bool,
    enable_peaking: bool,
    enable_high_pass: bool,
    enable_low_pass: bool,
    force_update: bool,
    /// `mMusicDspInfo`'s collection key (0 without a collection).
    dsp_info: u64,
    pub delay_max_seconds: f32,
    delay_time: f32,
    delay_feedback: f32,
    peaking_frequency: f32,
    peaking_gain: f32,
    peaking_q: f32,
    high_pass_frequency: f32,
    low_pass_frequency: f32,
    send_gain: f32,
    delay_time_changed: bool,
    delay_feedback_changed: bool,
    peaking_frequency_changed: bool,
    peaking_gain_changed: bool,
    peaking_q_changed: bool,
    high_pass_frequency_changed: bool,
    low_pass_frequency_changed: bool,
    send_gain_changed: bool,
}

/// The `aud_music` collection key `FindCollection` resolves (0 for a missing one).
#[must_use]
pub fn find_music(vault: &Vault, key: u64) -> u64 {
    if key != 0 && vault.has_collection(Collection::new(AUD_MUSIC, key)) { key } else { 0 }
}

impl MusicDspVoice {
    /// `MusicDspVoice::MusicDspVoice` 0x824f68c0.
    #[must_use]
    pub fn new(vault: &Vault) -> Self {
        Self {
            mono_reverb_gain_changed: true,
            game_speed_changed: true,
            time_stretch_changed: true,
            pitch_mult_changed: true,
            path_track_id: 0,
            reverb_channel_set: -1,
            reverb_channel_set_gain: 1.0,
            mono_reverb_gain: 0.0,
            time_stretch: 1.0,
            pitch_mult: 1.0,
            voice: None,
            plugin_submix: None,
            plugin_delay: None,
            plugin_peaking: None,
            plugin_high_pass: None,
            plugin_low_pass: None,
            plugin_send: None,
            enable_delay: false,
            enable_peaking: true,
            enable_high_pass: true,
            enable_low_pass: true,
            force_update: true,
            dsp_info: find_music(vault, DEFAULT_DSP_INFO),
            delay_max_seconds: 0.0,
            delay_time: 0.0,
            delay_feedback: 0.0,
            peaking_frequency: 0.0,
            peaking_gain: 1.0,
            peaking_q: 3.0,
            high_pass_frequency: 0.0,
            low_pass_frequency: 24000.0,
            send_gain: 1.0,
            delay_time_changed: true,
            delay_feedback_changed: true,
            peaking_frequency_changed: true,
            peaking_gain_changed: true,
            peaking_q_changed: true,
            high_pass_frequency_changed: true,
            low_pass_frequency_changed: true,
            send_gain_changed: true,
        }
    }

    /// `SetDspInfoKey` 0x824f6a38.
    pub fn set_dsp_info_key(&mut self, vault: &Vault, key: u64) {
        if key != self.dsp_info {
            self.dsp_info = find_music(vault, key);
            self.force_update = true;
        }
    }

    /// `ResetDsp` 0x82500fd0's part for one voice: back to no collection.
    pub fn clear_dsp_info(&mut self) {
        if self.dsp_info != 0 {
            self.dsp_info = 0;
            self.force_update = true;
        }
    }

    /// `SetReverbChannelSetGain` 0x824e96c0.
    pub fn set_reverb_channel_set_gain(&mut self, gain: f32) {
        let mut difference = self.reverb_channel_set_gain - gain;
        if difference < 0.0 {
            difference = -difference;
        }
        if f64::from(difference) > 0.01 && !self.mono_reverb_gain_changed {
            self.reverb_channel_set_gain = gain;
            self.mono_reverb_gain_changed = true;
        }
    }

    /// `CreateDspSubMix` 0x824f6a98: the stage 2 voice, its send to `destination` (the music
    /// master submix when there is none); returns the `SubMix`.
    pub fn create_dsp_sub_mix(&mut self, core: &mut AudioCore, channels: u8, destination: Option<PluginRef>, music_master: PluginRef) -> Option<PluginRef> {
        let mut configs = Vec::new();
        let mut add = |kind: PlugInKind| {
            configs.push(PlugInConfig { kind, output_channels: channels });
            Some((configs.len() - 1) as u8)
        };
        self.plugin_submix = add(PlugInKind::SubMix { name: String::new() });
        self.plugin_delay = if self.enable_delay { add(PlugInKind::Delay { max_time: Some(self.delay_max_seconds) }) } else { None };
        self.plugin_peaking = if self.enable_peaking { add(PlugInKind::PeakingIir2) } else { None };
        self.plugin_high_pass = if self.enable_high_pass { add(PlugInKind::HighPassIir2) } else { None };
        self.plugin_low_pass = if self.enable_low_pass { add(PlugInKind::LowPassIir2) } else { None };
        self.plugin_send = add(PlugInKind::Send);
        self.voice = core.create_voice(2, "MusicDspVoice", &configs);
        let voice = self.voice?;
        let send = PluginRef { voice, index: self.plugin_send? };
        core.connect_send(send, Some(destination.unwrap_or(music_master)));
        Some(PluginRef { voice, index: self.plugin_submix? })
    }

    /// `CheckAttribulator` 0x82501a20.
    fn check_attribulator(&mut self, vault: &Vault, inputs: DspInputs) {
        if self.mono_reverb_gain != inputs.wet_volume && !self.mono_reverb_gain_changed {
            self.mono_reverb_gain = inputs.wet_volume;
            self.mono_reverb_gain_changed = true;
        }
        let feedback;
        if self.dsp_info != 0 {
            let info = Collection::new(AUD_MUSIC, self.dsp_info);
            let scalar = |key: u64| if vault.length(info, key) > 0 { vault.f32(info, key) } else { 1.0 };
            let time_stretch = scalar(TIME_STRETCH);
            if self.time_stretch != time_stretch && !self.time_stretch_changed {
                self.time_stretch = time_stretch;
                self.time_stretch_changed = true;
            }
            let pitch = scalar(PITCH_SCALAR);
            if self.pitch_mult != pitch && !self.pitch_mult_changed {
                self.pitch_mult = pitch;
                self.pitch_mult_changed = true;
            }
            let layout = |index: u32| vault.layout_f32(info, index);
            let peaking_frequency = layout(3);
            if self.peaking_frequency != peaking_frequency && !self.peaking_frequency_changed {
                self.peaking_frequency = peaking_frequency;
                self.peaking_frequency_changed = true;
            }
            let peaking_gain = layout(2);
            if self.peaking_gain != peaking_gain && !self.peaking_gain_changed {
                self.peaking_gain = peaking_gain;
                self.peaking_gain_changed = true;
            }
            let peaking_q = layout(1);
            if self.peaking_q != peaking_q && !self.peaking_q_changed {
                self.peaking_q = peaking_q;
                self.peaking_q_changed = true;
            }
            let mut low_pass = layout(4);
            if low_pass > inputs.nicotine_lpf {
                low_pass = inputs.nicotine_lpf;
            }
            let test = inputs.test_low_pass as f32 * HUNDREDTH;
            let game_speed = inputs.game_speed_low_pass_scalar as f32 * HUNDREDTH;
            let low_pass = game_speed * test * low_pass;
            if self.low_pass_frequency != low_pass && !self.low_pass_frequency_changed {
                self.low_pass_frequency = low_pass;
                self.low_pass_frequency_changed = true;
            }
            let high_pass = layout(5);
            if self.high_pass_frequency != high_pass && !self.high_pass_frequency_changed {
                self.high_pass_frequency = high_pass;
                self.high_pass_frequency_changed = true;
            }
            let send_gain = layout(0);
            if self.send_gain != send_gain && !self.send_gain_changed {
                self.send_gain = send_gain;
                self.send_gain_changed = true;
            }
            let delay_time = layout(6);
            if self.delay_time != delay_time && !self.delay_time_changed {
                self.delay_time = delay_time;
                self.delay_time_changed = true;
            }
            feedback = layout(7);
        } else {
            if self.time_stretch != 1.0 && !self.time_stretch_changed {
                self.time_stretch = 1.0;
                self.time_stretch_changed = true;
            }
            if self.pitch_mult != 1.0 && !self.pitch_mult_changed {
                self.pitch_mult = 1.0;
                self.pitch_mult_changed = true;
            }
            if self.peaking_frequency != 96000.0 && !self.peaking_frequency_changed {
                self.peaking_frequency = 96000.0;
                self.peaking_frequency_changed = true;
            }
            if self.peaking_gain != 1.0 && !self.peaking_gain_changed {
                self.peaking_gain = 1.0;
                self.peaking_gain_changed = true;
            }
            if self.peaking_q != 3.0 && !self.peaking_q_changed {
                self.peaking_q = 3.0;
                self.peaking_q_changed = true;
            }
            let mut low_pass = 24000.0;
            if !(inputs.nicotine_lpf >= 24000.0) {
                low_pass = inputs.nicotine_lpf;
            }
            if self.low_pass_frequency != low_pass && !self.low_pass_frequency_changed {
                self.low_pass_frequency = low_pass;
                self.low_pass_frequency_changed = true;
            }
            if self.high_pass_frequency != 0.0 && !self.high_pass_frequency_changed {
                self.high_pass_frequency = 0.0;
                self.high_pass_frequency_changed = true;
            }
            if self.send_gain != 1.0 && !self.send_gain_changed {
                self.send_gain = 1.0;
                self.send_gain_changed = true;
            }
            if self.delay_time != 0.0 && !self.delay_time_changed {
                self.delay_time = 0.0;
                self.delay_time_changed = true;
            }
            feedback = 0.0;
        }
        if self.delay_feedback != feedback && !self.delay_feedback_changed {
            self.delay_feedback = feedback;
            self.delay_feedback_changed = true;
        }
    }

    /// `SomethingChanged` 0x824e9720: the feedback is tested by value, not by its flag.
    fn something_changed(&self) -> bool {
        self.mono_reverb_gain_changed
            || self.game_speed_changed
            || self.time_stretch_changed
            || self.pitch_mult_changed
            || self.delay_time_changed
            || self.delay_feedback != 0.0
            || self.peaking_frequency_changed
            || self.peaking_gain_changed
            || self.peaking_q_changed
            || self.high_pass_frequency_changed
            || self.low_pass_frequency_changed
            || self.send_gain_changed
    }

    /// `Update` 0x82501fe0: the settings of `CheckAttribulator`, then the changed ones (all of them
    /// with `update_required`) applied; the reverb channel set's volume goes through
    /// `PATH_channelsvolume` (`channels_volume`).
    pub fn update(&mut self, mut update_required: bool, vault: &Vault, inputs: DspInputs, core: &mut AudioCore, channels_volume: &mut dyn FnMut(&mut AudioCore, u32, usize, i32)) {
        let Some(voice) = self.voice else { return };
        self.check_attribulator(vault, inputs);
        if self.force_update {
            update_required = true;
        }
        self.force_update = false;
        if !(self.something_changed() || update_required) {
            return;
        }
        if update_required || self.mono_reverb_gain_changed || self.time_stretch_changed || self.pitch_mult_changed {
            if update_required || self.mono_reverb_gain_changed {
                if let Ok(set) = usize::try_from(self.reverb_channel_set) {
                    if self.path_track_id != 0 {
                        let volume = crate::nicotine::shape::fctiwz(self.mono_reverb_gain * self.reverb_channel_set_gain * 127.0);
                        channels_volume(core, self.path_track_id, set, volume);
                    }
                }
                self.mono_reverb_gain_changed = false;
            }
            self.time_stretch_changed = false;
            self.pitch_mult_changed = false;
        }
        let plugin = |index: Option<u8>| index.map(|index| PluginRef { voice, index });
        if let Some(delay) = plugin(self.plugin_delay) {
            if update_required || self.delay_time_changed {
                core.set_attribute(delay, 0, self.delay_time);
                self.delay_time_changed = false;
            }
            if update_required || self.delay_feedback_changed {
                core.set_attribute(delay, 1, self.delay_feedback);
                self.delay_feedback_changed = false;
            }
        }
        if let Some(peaking) = plugin(self.plugin_peaking) {
            if update_required || self.peaking_gain_changed {
                core.set_attribute(peaking, 1, self.peaking_gain);
                self.peaking_gain_changed = false;
            }
            if update_required || self.peaking_frequency_changed {
                core.set_attribute(peaking, 0, self.peaking_frequency);
                self.peaking_frequency_changed = false;
            }
            if update_required || self.peaking_q_changed {
                core.set_attribute(peaking, 2, self.peaking_q);
                self.peaking_q_changed = false;
            }
        }
        if let Some(high_pass) = plugin(self.plugin_high_pass) {
            if update_required || self.high_pass_frequency_changed {
                core.set_attribute(high_pass, 0, self.high_pass_frequency);
                self.high_pass_frequency_changed = false;
            }
        }
        if let Some(low_pass) = plugin(self.plugin_low_pass) {
            if update_required || self.low_pass_frequency_changed {
                core.set_attribute(low_pass, 0, self.low_pass_frequency);
                self.low_pass_frequency_changed = false;
            }
        }
        if let Some(send) = plugin(self.plugin_send) {
            if update_required || self.send_gain_changed {
                core.set_attribute(send, 0, self.send_gain);
                self.send_gain_changed = false;
            }
        }
    }
}
