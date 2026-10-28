//! `rw::audio::core::Delay` (`CreateInstance` 0x82bfd348, `Process` 0x82bfd438,
//! `RwacTimerClient` 0x82bebcf8) and its `DelayFilter` (`DelayFilterApplyFunc` 0x82bdd660):
//! attribute 0 is the delay in seconds, attribute 1 the feedback. The output is the delayed
//! signal alone.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::math::fmadd;
use crate::nicotine::shape::fctiwz;
use crate::rwac::delay_line::{Chunk, DelayLine, Location};
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;

/// `DelayFilter::MAX_FEEDBACK_MAGNITUDE` and `__real_bf7d70a4`.
const MAX_FEEDBACK: f32 = f32::from_bits(0x3F7D_70A4);
const MIN_FEEDBACK: f32 = f32::from_bits(0xBF7D_70A4);
/// The rate the timer sizes the line for (`__real_473b8000`).
const LINE_RATE: f32 = 48000.0;

pub struct Delay {
    base: PlugInBase,
    /// +0x38: 0 off, 1 delaying, 2 holding the last delay until the history covers a longer one.
    state: u32,
    /// +0x3C.
    line_rate: f32,
    /// +0x40: the longest delay in seconds the line holds (the constructor parameter, grown by
    /// the timer).
    max_time: f32,
    /// `DelayFilter` +0x10 and +0x14: the feedback and the one before it.
    feedback: f32,
    old_feedback: f32,
    /// +0x5C.
    line: DelayLine,
}

impl Delay {
    /// `CreateInstance`: a line for the constructor's time at 48 kHz (`DelayFilter` reads one
    /// sample per output).
    pub fn new(input_channels: u8, output_channels: u8, max_time: Option<f32>) -> Self {
        let base = PlugInBase::new(&super::DELAY, input_channels, output_channels);
        let max_time = max_time.unwrap_or(0.0);
        let mut line = DelayLine::default();
        line.init(i32::from(input_channels), fctiwz(max_time * LINE_RATE), 1);
        Self { base, state: 0, line_rate: LINE_RATE, max_time, feedback: 0.0, old_feedback: 0.0, line }
    }

    /// `DelayFilter::SetFeedback` 0x82bdd6f8: at most 0.99 either way.
    fn set_feedback(&mut self, feedback: f32) {
        let feedback = if feedback.abs() <= MAX_FEEDBACK {
            feedback
        } else if feedback > 0.0 {
            MAX_FEEDBACK
        } else {
            MIN_FEEDBACK
        };
        self.old_feedback = self.feedback;
        self.feedback = feedback;
    }

    /// `UpdateLatencyAndDecay` 0x82bf5c20: while delaying, the latency is the delay and the decay
    /// adds the 100 dB tail of the feedback (`delay - 5 * delay / log10(|feedback|)`).
    fn update_latency_and_decay(&mut self, voice: &mut VoiceCore) {
        if self.state == 1 {
            let latency = self.line.delay() as f32;
            let mut decay = latency;
            if self.feedback != 0.0 {
                let scaled = latency * 5.0;
                decay = latency - scaled / f64::from(self.feedback.abs()).log10() as f32;
            }
            let change = decay - self.base.decay_samples;
            self.base.latency_samples = latency;
            voice.decay_samples = change + voice.decay_samples;
            self.base.decay_samples = decay;
        } else {
            self.base.latency_samples = 0.0;
            voice.decay_samples = -self.base.decay_samples + voice.decay_samples;
            self.base.decay_samples = 0.0;
        }
    }
}

/// `DelayFilterApplyFunc` 0x82bdd660: `DelayFilterFunc` 0x82bdd0b8 writes the input plus the
/// fed-back tap and outputs the tap.
fn filter(location: &mut Location<'_>, input: &[f32], output: &mut [f32], chunk: Chunk, feedback: f32, old_feedback: f32) {
    let Some(old_delay) = chunk.fade else {
        for i in chunk.start..chunk.start + chunk.count {
            let tap = location.tap(i, chunk.delay);
            location.write[i] = fmadd(tap, feedback, input[i]);
            output[i] = tap;
        }
        return;
    };
    cross_fade(location, input, output, chunk, old_delay, feedback, old_feedback);
}

/// `DelayFilterCrossFadeFunc` 0x82bdd210 over the samples after a delay change: the output
/// moves from the old tap to the new one (`(127 - k) / 128` of the old), the write buffer feeds
/// each back with its feedback. With both taps 16-byte aligned in the local buffer (both delays
/// multiples of 4) the VMX loop runs 16 samples at a time, its products rounded and every load
/// before the stores (a delay under 16 reads unwritten samples there, natively stale scratch
/// memory, here 0); otherwise the scalar loop, four-way unrolled, whose first two lanes associate
/// the products differently from the rest.
fn cross_fade(location: &mut Location<'_>, input: &[f32], output: &mut [f32], chunk: Chunk, old_delay: i32, feedback: f32, old_feedback: f32) {
    let vector = if chunk.delay % 4 == 0 && old_delay % 4 == 0 { chunk.count / 16 * 16 } else { 0 };
    let unrolled = vector + (chunk.count - vector) / 4 * 4;
    let mut k = 0;
    while k < vector {
        let mut taps = [(0.0f32, 0.0f32); 16];
        for (j, tap) in taps.iter_mut().enumerate() {
            let i = chunk.start + k + j;
            *tap = (location.tap(i, chunk.delay), location.tap(i, old_delay));
        }
        for (j, (new, old)) in taps.into_iter().enumerate() {
            let i = chunk.start + k + j;
            let fade = (127 - (k + j)) as f32 / 128.0;
            let new_part = (1.0 - fade) * new;
            let old_part = fade * old;
            output[i] = old_part + new_part;
            location.write[i] = fmadd(new_part, feedback, old_part * old_feedback) + input[i];
        }
        k += 16;
    }
    while k < chunk.count {
        let i = chunk.start + k;
        let fade = (127 - k) as f32 / 128.0;
        let rest = 1.0 - fade;
        let new = location.tap(i, chunk.delay);
        let old = location.tap(i, old_delay);
        let lane = if k < unrolled { (k - vector) % 4 } else { 2 };
        location.write[i] = match lane {
            0 => fmadd(rest * feedback, new, (fade * old_feedback) * old),
            1 => fmadd(rest * new, feedback, (fade * old) * old_feedback),
            _ => fmadd(rest * feedback, new, (old * fade) * old_feedback),
        } + input[i];
        output[i] = fmadd(rest, new, fade * old);
        k += 1;
    }
}

impl PlugIn for Delay {
    impl_plugin_any!();

    /// `Process`: the delay rounded to whole samples; a longer delay than the history holds
    /// keeps the old one (state 2) until the line has filled.
    fn process(&mut self, voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let scaled = services.mixer.params_sample_rate * self.base.attr(0);
        let delay = if scaled < 0.0 { fctiwz(scaled - 0.5) } else { fctiwz(scaled + 0.5) };
        let feedback = self.base.attr(1);
        match self.state {
            0 => {
                if delay > 0 {
                    self.feedback = 0.0;
                    self.old_feedback = 0.0;
                    self.set_feedback(feedback);
                    self.line.reset(delay);
                    self.state = 1;
                }
            }
            1 => {
                if delay <= 0 {
                    self.state = 0;
                } else if self.line.valid_delay_samples() < delay {
                    self.state = 2;
                } else {
                    self.set_feedback(feedback);
                    self.line.set_delay(delay);
                }
            }
            2 => {
                if delay <= 0 {
                    self.state = 0;
                } else if delay <= self.line.valid_delay_samples() {
                    self.set_feedback(feedback);
                    self.line.set_delay(delay);
                    self.state = 1;
                }
            }
            _ => {}
        }
        if self.state != 0 {
            let (feedback, old_feedback) = (self.feedback, self.old_feedback);
            self.line.apply_filter(services.mixer, |location, input, output, chunk| filter(location, input, output, chunk, feedback, old_feedback));
            services.mixer.swap();
        }
        self.update_latency_and_decay(voice);
        BufferStatus::Available
    }

    /// `RwacTimerClient`: the line grows to the longest delay asked for.
    fn timer(&mut self, _voice: &mut VoiceCore, _services: &mut Services<'_>) {
        let time = self.base.attr(0);
        if !(self.max_time >= time) {
            self.max_time = time;
        }
        if self.line_rate != LINE_RATE {
            self.line_rate = LINE_RATE;
            self.state = 0;
        }
        let samples = fctiwz(self.max_time * self.line_rate);
        if self.line.max_delay() < samples {
            self.line.resize(samples);
        }
    }
}
