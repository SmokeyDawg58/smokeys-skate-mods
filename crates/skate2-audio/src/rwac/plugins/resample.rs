//! `rw::audio::core::Resample`: linear interpolation in 16.16 fixed point from the source rate
//! times the pitch attribute to the mixer rate, keeping up to 6 history samples per channel.

use crate::math::round_half_away;
use crate::rwac::kernels::linear_interpolate;
use crate::rwac::mixer::Mixer;
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;
use crate::rwac::FRAME_SAMPLES;

const HISTORY: usize = 6;

pub struct Resample {
    base: PlugInBase,
    actual_ratio: f32,
    requested_ratio: f32,
    previous_sample_rate: f32,
    incr: u32,
    acc: u32,
    history: Vec<[f32; HISTORY]>,
    output_samples_requested: u16,
    current_history_samples: u8,
    target_history_samples: u8,
}

impl Resample {
    /// `Resample::CreateInstance` 0x82bf8828; adds 6 decay samples to the voice.
    pub fn new(voice: &mut VoiceCore, input_channels: u8, output_channels: u8) -> Self {
        let mut base = PlugInBase::new(&super::RESAMPLE, input_channels, output_channels);
        voice.decay_samples += 6.0 - base.decay_samples;
        base.decay_samples = 6.0;
        base.latency_samples = 6.0;
        Self {
            base,
            actual_ratio: 0.0,
            requested_ratio: -1.0,
            previous_sample_rate: 48000.0,
            incr: 0,
            acc: 0,
            history: vec![[0.0; HISTORY]; usize::from(output_channels)],
            output_samples_requested: 0,
            current_history_samples: 0,
            target_history_samples: 2,
        }
    }

    /// `SetResampleIncrement` 0x82bf0f00.
    fn set_increment(&mut self, ratio: f32) {
        let increment = round_half_away(ratio * 65536.0);
        self.requested_ratio = ratio;
        if increment <= 0x40000 {
            self.actual_ratio = ratio;
            self.incr = increment as u32;
        } else {
            self.incr = 0x40000;
            self.actual_ratio = 4.0;
        }
    }
}

impl PlugIn for Resample {
    impl_plugin_any!();

    /// `Resample::PreProcess` 0x82bf0f80.
    fn pre_process(&mut self, mixer: &mut Mixer, _discontinuity: bool, requested: i32) -> i32 {
        let ratio = (self.previous_sample_rate / mixer.params_sample_rate) * self.base.attr(0);
        if self.requested_ratio != ratio {
            self.set_increment(ratio);
        }
        let position = self.incr.wrapping_mul(requested as u32).wrapping_add(self.acc) >> 16;
        let needed = position as i32 - i32::from(self.current_history_samples) + i32::from(self.target_history_samples);
        self.output_samples_requested = requested as u16;
        mixer.total_pitch *= self.actual_ratio;
        needed.max(0)
    }

    /// `Resample::Process` 0x82bf8e78.
    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let mixer = &mut *services.mixer;
        if mixer.src_sample_rate != self.previous_sample_rate {
            self.previous_sample_rate = mixer.src_sample_rate;
            mixer.src_sample_rate = mixer.params_sample_rate;
            return BufferStatus::Available;
        }
        let input = mixer.src_num_samples.max(0) as usize;
        let current_history = usize::from(self.current_history_samples);
        let total = current_history + input;
        let available = total as i64 - i64::from(self.target_history_samples) + 1;
        let mut outputs = if available <= 0 {
            0u32
        } else {
            ((available as u32) << 16).wrapping_sub(self.acc).wrapping_sub(1).checked_div(self.incr).unwrap_or(0x2000)
        };
        outputs = outputs.min(u32::from(self.output_samples_requested));
        let outputs = outputs as usize;
        let mut remaining = 0usize;
        let mut frac = self.acc;
        let mut work = vec![0.0f32; total + 2];
        for channel in 0..usize::from(self.base.output_channels) {
            work[..current_history].copy_from_slice(&self.history[channel][..current_history]);
            work[current_history..total].copy_from_slice(&mixer.current(channel)[..input]);
            let (whole, end_frac) = {
                let (_, scratch) = mixer.current_and_scratch_channel(channel);
                linear_interpolate(&mut scratch[..outputs], &work, 0, self.acc, self.incr)
            };
            remaining = total - whole as usize;
            let keep = remaining.min(HISTORY);
            self.history[channel][..keep].copy_from_slice(&work[whole as usize..whole as usize + keep]);
            frac = end_frac;
        }
        self.current_history_samples = remaining as u8;
        self.acc = frac;
        mixer.swap();
        mixer.src_num_samples = outputs as i32;
        mixer.src_sample_rate = mixer.params_sample_rate;
        let _ = FRAME_SAMPLES;
        BufferStatus::Available
    }
}
