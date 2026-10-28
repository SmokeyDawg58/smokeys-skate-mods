//! `GrainPlayer::AudBoard` (map library `GrainPlayer`): the rolling-sound granulator. Two
//! one-shot "slices" of a `.grain` file alternate; each fades in, sustains and fades out while
//! the next starts at a fresh offset inside a window that the intensity slides across the file.
//! Offsets are drawn from the holes the recent grains leave, so the same stretch does not repeat.
//!
//! `Process` runs as a `TIMERSTAGE_BEFORECOMMANDS` timer ("Grain Player"), so the voices and
//! events it issues execute in the same mixer frame.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::nicotine::shape::fctiwz;
use crate::random::RwRandom;
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::decoder::DataRef;
use crate::rwac::plugins::snd_player::PlayParams;
use crate::rwac::{AudioCore, PluginRef, VoiceId};
use std::sync::Arc;

/// `GrainPlayer::UpdateParams`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpdateParams {
    pub volume: f32,
    pub pitch: f32,
    pub azimuth: i32,
    pub intensity: f32,
}

/// `GrainPlayer::GrainParams` (the `aud_skateboard` grain attribute's five floats).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrainParams {
    pub fade_in_time: f32,
    pub sustain_time: f32,
    pub fade_out_time: f32,
    pub window_size: f32,
    pub percentage_drift: f32,
}

/// `AudBoard::GrainDescription`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GrainDescription {
    start: f32,
    end: f32,
    next: i16,
}

const UNUSED: GrainDescription = GrainDescription { start: -1.0, end: -1.0, next: 0 };
/// `__real_30000000`: 2^-31.
const RANDOM_SCALE: f32 = f32::from_bits(0x3000_0000);
const MAX_HOLES: usize = 64;

/// `SampleDef::eEnvelopeState`.
const FADING_IN: i32 = 1;
const SUSTAINING: i32 = 2;
const FADING_OUT: i32 = 3;

/// `AudBoard::SampleDef`: one slice's voice (`SndPlayer1`, `Resample`, `GainFader`, `Send`).
#[derive(Debug, Clone, Copy)]
struct SampleDef {
    voice: Option<VoiceId>,
    time_left_of_state: f32,
    start_time: f32,
    intensity: f32,
    envelope_state: i32,
}

impl SampleDef {
    const EMPTY: Self = Self { voice: None, time_left_of_state: 0.0, start_time: 0.0, intensity: 0.0, envelope_state: FADING_IN };

    fn plugin(&self, index: u8) -> Option<PluginRef> {
        self.voice.map(|voice| PluginRef { voice, index })
    }

    /// `SampleDef::Reset` 0x829c90f8.
    fn reset(&mut self, core: &mut AudioCore) {
        if let Some(voice) = self.voice {
            core.release_voice(voice);
        }
        *self = Self::EMPTY;
    }

    /// `SampleDef::FadeIn` 0x829c9210: the `GainFader` rises to 1 along curve 1.
    fn fade_in(&mut self, core: &mut AudioCore, time: f32) {
        if let Some(fader) = self.plugin(2) {
            core.start_fade(fader, 0.0, time, 1.0, 1.0);
        }
        self.time_left_of_state = time;
        self.envelope_state = FADING_IN;
    }

    /// `SampleDef::FadeOut` 0x829c9290.
    fn fade_out(&mut self, core: &mut AudioCore, time: f32) {
        self.time_left_of_state = time;
        self.envelope_state = FADING_OUT;
        if let Some(fader) = self.plugin(2) {
            core.start_fade(fader, 0.0, time, 0.0, 1.0);
        }
    }
}

/// A `.grain` file: its SNR offset, length in seconds, seek table from byte 8 and SNR sample.
#[derive(Clone)]
pub struct GrainData {
    pub bytes: Arc<[u8]>,
}

impl GrainData {
    fn word(&self, at: usize) -> u32 {
        self.bytes.get(at..at + 4).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// `GrainPlayer::AudBoard`.
pub struct AudBoard {
    pub params: UpdateParams,
    pub grain_params: GrainParams,
    is_debug_playback: bool,
    sub_mix: Option<PluginRef>,
    initialized: bool,
    file_length: f32,
    sek_data: Option<DataRef>,
    snr_data: Option<DataRef>,
    samples: [SampleDef; 2],
    last_created: i32,
    /// The timer id the owner gave `Initialize` (`mTimerHandle`).
    timer: u32,
    usage: [GrainDescription; 16],
    start_used_list: i16,
    last_used: i16,
    start_of_unused_list: i16,
}

impl Default for AudBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl AudBoard {
    /// `AudBoard::AudBoard` 0x829c9578.
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: UpdateParams { volume: 1.0, pitch: 1.0, azimuth: 0, intensity: 0.0 },
            grain_params: GrainParams {
                fade_in_time: f32::from_bits(0x3C23_D70A),
                sustain_time: 0.5,
                fade_out_time: f32::from_bits(0x3C23_D70A),
                window_size: 4.0,
                percentage_drift: f32::from_bits(0x3D4C_CCCD),
            },
            is_debug_playback: false,
            sub_mix: None,
            initialized: false,
            file_length: 0.0,
            sek_data: None,
            snr_data: None,
            samples: [SampleDef::EMPTY; 2],
            last_created: 0,
            timer: 0,
            usage: [UNUSED; 16],
            start_used_list: 0,
            last_used: 0,
            start_of_unused_list: 0,
        }
    }

    #[must_use]
    pub fn initialized(&self) -> bool {
        self.initialized
    }

    #[must_use]
    pub fn timer(&self) -> u32 {
        self.timer
    }

    /// `AudBoard::InitVariables` 0x829c9d38: the usage table holds the sentinel grain 0
    /// (-2..-1) and the free list 1..15; the timer starts.
    fn init_variables(&mut self, core: &mut AudioCore, timer: u32) {
        self.last_created = 0;
        self.last_used = 0;
        self.start_used_list = 0;
        self.start_of_unused_list = 1;
        for (index, entry) in self.usage.iter_mut().enumerate() {
            *entry = GrainDescription { next: index as i16 + 1, ..UNUSED };
        }
        self.usage[0].end = -1.0;
        self.is_debug_playback = false;
        self.usage[15].next = -1;
        self.usage[0].next = -1;
        self.usage[0].start = -2.0;
        self.timer = timer;
        core.add_before_commands_timer(timer);
        self.initialized = true;
    }

    /// `AudBoard::Initialize` 0x829c9e20.
    pub fn initialize(&mut self, core: &mut AudioCore, sub_mix: PluginRef, data: &GrainData, timer: u32) {
        self.init_variables(core, timer);
        self.sub_mix = Some(sub_mix);
        let snr_offset = data.word(0) as usize;
        self.file_length = f32::from_bits(data.word(4));
        self.sek_data = Some(DataRef { bytes: Arc::clone(&data.bytes), offset: 8 });
        self.snr_data = Some(DataRef { bytes: Arc::clone(&data.bytes), offset: snr_offset });
    }

    /// `AudBoard::Update` 0x829799f0.
    pub fn update(&mut self, params: UpdateParams) {
        self.params = params;
    }

    /// `AudBoard::Start` 0x829c9aa0.
    pub fn start(&mut self, core: &mut AudioCore, rng: &mut RwRandom) {
        let offset = self.next_grain_offset(rng);
        self.create_new_slice(core, 0, offset);
    }

    /// `AudBoard::Destroy` 0x829c9160.
    pub fn destroy(&mut self, core: &mut AudioCore) {
        if !self.initialized {
            return;
        }
        for sample in &mut self.samples {
            sample.reset(core);
        }
        core.remove_before_commands_timer(self.timer);
        self.initialized = false;
    }

    /// `AudBoard::CreateNewSlice` 0x829c9300.
    fn create_new_slice(&mut self, core: &mut AudioCore, index: usize, start_time: f32) {
        let configs = [
            PlugInConfig { kind: PlugInKind::SndPlayer1 { max_requests: 1.0 }, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::Resample, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::GainFader, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::Send, output_channels: 1 },
        ];
        let sample = &mut self.samples[index];
        sample.voice = core.create_voice(0, "Unknown", &configs);
        let start_time = if !(start_time >= 0.0) {
            0.0
        } else if !(start_time <= self.file_length) {
            self.file_length
        } else {
            start_time
        };
        if let Some(player) = sample.plugin(0) {
            core.play(
                player,
                PlayParams {
                    start_time: 0.0,
                    stream_file_offset: 0.0,
                    seek_time: f64::from(start_time),
                    stream_path: None,
                    ram_sample: self.snr_data.clone(),
                    seek_table: self.sek_data.clone(),
                    stream_pool: 0,
                    expel_mode: 1.0,
                },
            );
        }
        if let Some(fader) = sample.plugin(2) {
            core.start_fade(fader, 0.0, 0.0, 0.0, 0.0);
        }
        sample.fade_in(core, self.grain_params.fade_in_time);
        if let Some(send) = sample.plugin(3) {
            core.connect_send(send, self.sub_mix);
            core.set_attribute(send, 0, self.params.volume);
        }
        sample.start_time = start_time;
        sample.intensity = self.params.intensity;
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!(
                "{elapsed:9.3} grain timer {} slice {index} start {start_time:.3} intensity {:.3} volume {:.4} pitch {:.4} window {:.2} sustain {:.2} fade {:.2}/{:.2} length {:.2}",
                self.timer,
                self.params.intensity,
                self.params.volume,
                self.params.pitch,
                self.grain_params.window_size,
                self.grain_params.sustain_time,
                self.grain_params.fade_in_time,
                self.grain_params.fade_out_time,
                self.file_length
            );
        }
    }

    /// `AudBoard::GetNextGrainOffset` 0x829c96c0: a free grain-sized stretch of the window,
    /// chosen at random and recorded in the start-sorted usage list. With no free stretch, or
    /// once all fifteen entries were used, the list restarts from the last grain; a window of
    /// one grain or less then returns its start.
    fn next_grain_offset(&mut self, rng: &mut RwRandom) -> f32 {
        loop {
            let grain = &self.grain_params;
            let grain_size = (grain.sustain_time + grain.fade_out_time) + grain.fade_in_time;
            let window_start = (self.file_length - grain.window_size) * self.params.intensity;
            let window_end = grain.window_size + window_start;
            let grains_per_window = fctiwz(grain.window_size / grain_size);
            let mut holes = [UNUSED; MAX_HOLES];
            let mut count = 0usize;
            let first = self.usage[self.start_used_list as usize].start;
            if !(window_start >= first) {
                let mut hole = GrainDescription { start: window_start, end: first, next: 0 };
                count_holes(&mut hole, &mut holes, &mut count, grain_size, window_start, window_end);
            }
            let mut index = self.start_used_list;
            while index != -1 {
                let entry = self.usage[index as usize];
                let end = if entry.next == -1 { self.file_length } else { self.usage[entry.next as usize].start };
                let mut hole = GrainDescription { start: entry.end, end, next: 0 };
                count_holes(&mut hole, &mut holes, &mut count, grain_size, window_start, window_end);
                index = entry.next;
            }
            if count != 0 && self.last_used != 15 {
                return self.use_random_hole(rng, &holes, count);
            }
            self.usage[0] = self.usage[self.last_used as usize];
            self.usage[0].next = -1;
            self.start_used_list = 0;
            self.last_used = 0;
            self.start_of_unused_list = 1;
            for index in 1..16 {
                self.usage[index] = GrainDescription { next: index as i16 + 1, ..UNUSED };
            }
            self.usage[15].next = -1;
            if grains_per_window <= 1 {
                return window_start;
            }
        }
    }

    /// The tail of `GetNextGrainOffset`: `rw::math::Random` picks a hole, which takes the head
    /// of the free list and is linked in by start.
    fn use_random_hole(&mut self, rng: &mut RwRandom, holes: &[GrainDescription; MAX_HOLES], count: usize) -> f32 {
        let random = rng.next_u32() as f32;
        let new = self.start_of_unused_list;
        self.start_of_unused_list = self.usage[new as usize].next;
        let pick = fctiwz(((random * RANDOM_SCALE) * 0.5) * count as f32);
        // A draw that rounds to 1.0 reads the next, still unused, hole as natively.
        self.usage[new as usize] = holes.get(pick as usize).copied().unwrap_or(UNUSED);
        self.last_used = new;
        let start = self.usage[new as usize].start;
        let mut previous: i16 = -1;
        let mut current = self.start_used_list;
        if !(self.usage[current as usize].start >= start) {
            loop {
                previous = current;
                current = self.usage[current as usize].next;
                if current == -1 || !(self.usage[current as usize].start < start) {
                    break;
                }
            }
        }
        if current == -1 {
            self.usage[previous as usize].next = new;
            self.usage[new as usize].next = -1;
        } else if previous == -1 {
            self.start_used_list = new;
            self.usage[new as usize].next = current;
        } else {
            self.usage[new as usize].next = current;
            self.usage[previous as usize].next = new;
        }
        start
    }

    /// `AudBoard::Process` 0x829c9ad8 (the timer, with the timer period): a slice whose
    /// intensity drifted too far from the current one fades out for a new slice; each live
    /// slice follows the volume and pitch, and its envelope advances (sustain at the pitch's
    /// rate). A slice whose sustain ends fades out and the other one restarts.
    pub fn process(&mut self, core: &mut AudioCore, rng: &mut RwRandom, dt: f32) {
        let last = self.last_created as usize;
        let drift = self.grain_params.percentage_drift;
        let difference = self.samples[last].intensity - self.params.intensity;
        if (difference > drift || !(difference >= -drift)) && !self.is_debug_playback && self.samples[last].voice.is_some() {
            let other = ((self.last_created + 1) % 2) as usize;
            if self.samples[other].voice.is_some() {
                if self.samples[other].envelope_state != FADING_OUT {
                    self.samples[other].fade_out(core, self.grain_params.fade_out_time);
                }
            } else {
                if self.samples[last].envelope_state != FADING_OUT {
                    self.samples[last].fade_out(core, self.grain_params.fade_out_time);
                }
                let offset = self.next_grain_offset(rng);
                self.create_new_slice(core, other, offset);
                self.last_created = (self.last_created + 1) % 2;
            }
        }
        for index in 0..2 {
            let Some(voice) = self.samples[index].voice else { continue };
            core.set_attribute(PluginRef { voice, index: 3 }, 0, self.params.volume);
            core.set_attribute(PluginRef { voice, index: 1 }, 0, self.params.pitch);
            let sample = &mut self.samples[index];
            let state = sample.envelope_state;
            sample.time_left_of_state = if state == SUSTAINING {
                -self.params.pitch.mul_add(dt, -sample.time_left_of_state)
            } else {
                sample.time_left_of_state - dt
            };
            if !(sample.time_left_of_state < 0.0) {
                continue;
            }
            match state {
                FADING_IN => {
                    sample.envelope_state = SUSTAINING;
                    sample.time_left_of_state = self.grain_params.sustain_time;
                }
                SUSTAINING => {
                    sample.fade_out(core, self.grain_params.fade_out_time);
                    if !self.is_debug_playback {
                        let other = (index + 1) % 2;
                        if self.samples[other].voice.is_some() {
                            self.samples[other].reset(core);
                        }
                        let offset = self.next_grain_offset(rng);
                        self.create_new_slice(core, other, offset);
                        self.last_created = other as i32;
                    }
                }
                FADING_OUT => sample.reset(core),
                _ => {}
            }
        }
    }
}

/// `AudBoard::CountHoles` 0x829c94b0: the grain-sized pieces of `hole` (clipped to the window)
/// that fit in it, up to 64 in all.
fn count_holes(hole: &mut GrainDescription, free: &mut [GrainDescription; MAX_HOLES], count: &mut usize, grain_size: f32, window_start: f32, window_end: f32) {
    if *count >= MAX_HOLES || hole.end < window_start || hole.start > window_end {
        return;
    }
    if !(hole.start >= window_start) {
        hole.start = window_start;
    }
    if !(hole.end <= window_end) {
        hole.end = window_end;
    }
    if hole.end - hole.start < grain_size {
        return;
    }
    loop {
        free[*count].start = hole.start;
        free[*count].end = hole.start + grain_size;
        *count += 1;
        let start = hole.start + grain_size;
        let remaining = hole.end - start;
        hole.start = start;
        if remaining < grain_size || start + grain_size > window_end || *count >= MAX_HOLES {
            return;
        }
    }
}
