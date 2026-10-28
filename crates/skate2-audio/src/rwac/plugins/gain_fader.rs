//! `rw::audio::core::GainFader` (`Process` 0x82bebe90, `StartFadeHandler` 0x82bf6128): a gain
//! that fades to a target over a time from a start time, with the gain-vector kernels
//! `GainVectorLinearAmplitude` 0x82bdeea8 and `GainVectorLinearPower` 0x82bdf368.
//!
//! The game's only fades are `GrainPlayer::AudBoard::SampleDef::FadeIn`/`FadeOut` (0x829c9210,
//! 0x829c9290: start 0, curve 1). `GainVectorSine` 0x82bdfa10 (curve 2) is not ported; curve 2
//! runs as the linear-amplitude curve.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::nicotine::shape::fctiwz;
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;
use crate::rwac::FRAME_SAMPLES;

/// `fnmsubs`: -(a * c - b) with one rounding.
fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    -a.mul_add(c, -b)
}

/// `fctiwz` of a double: truncation, saturating, NaN to `i32::MIN`.
fn fctiwz_f64(value: f64) -> i32 {
    if value.is_nan() { i32::MIN } else { value as i32 }
}

/// One lane of the kernels' square root: `vrsqrtefp` (seeded exactly) with one Newton step, then
/// `x * y`; a lane whose step turns NaN without the estimate being NaN (0, infinity) keeps `x`.
fn vmx_sqrt(x: f32) -> f32 {
    let y0 = 1.0 / x.sqrt();
    let t = fnmsubs(x * 0.5, y0 * y0, 0.5);
    let y1 = y0.mul_add(t, y0);
    if t.is_nan() != y0.is_nan() { x } else { x * y1 }
}

/// The part every kernel shares: the samples before the fade starts get `start` and the scalar
/// head up to vector alignment uses the linear formulas. Returns (first unwritten sample,
/// position reached).
fn lead_in(out: &mut [f32], start: f32, end: f32, offset: i32, limit: i32, slope: f32, length: f32) -> (usize, i32) {
    if offset >= 0 {
        return (0, offset);
    }
    let lead = -offset;
    let mut head = 4 - lead % 4;
    if head == 4 {
        head = 0;
    }
    let mut k = 0usize;
    for _ in 0..lead {
        out[k] = start;
        k += 1;
    }
    let mut p = 0;
    if p <= limit {
        let increasing = !(end - start < 0.0);
        while head > 0 {
            p += 1;
            out[k] = if increasing { (p as f32).mul_add(slope, start) } else { fnmsubs(length - p as f32, slope, end) };
            k += 1;
            head -= 1;
            if p > limit {
                break;
            }
        }
    }
    (k, p)
}

/// `GainVectorLinearAmplitude` 0x82bdeea8: `start + q * (end - start) / length` at fade position
/// `q` (1-based; `offset` is the position before the first sample).
fn linear_amplitude(out: &mut [f32], start: f32, end: f32, offset: i32, length: i32) {
    let last = out.len() as i32 + offset - 1;
    let limit = if last > length - 1 { length - 1 } else { last };
    let length_f = length as f32;
    let slope = (end - start) / length_f;
    let (mut k, mut p) = lead_in(out, start, end, offset, limit, slope, length_f);
    let groups = (limit - p + 1) / 4;
    for _ in 0..4 * groups {
        p += 1;
        out[k] = slope.mul_add(p as f32, start);
        k += 1;
    }
    while p <= limit {
        p += 1;
        out[k] = (p as f32).mul_add(slope, start);
        k += 1;
    }
    out[k..].fill(end);
}

/// `GainVectorLinearPower` 0x82bdf368: `start + sqrt(q) * (end - start) / sqrt(length)` rising,
/// `end + sqrt(length - q) * (start - end) / sqrt(length)` falling (the scalar head before vector
/// alignment uses the linear formulas, as natively).
fn linear_power(out: &mut [f32], start: f32, end: f32, offset: i32, length: i32) {
    let last = out.len() as i32 + offset - 1;
    let limit = if last > length - 1 { length - 1 } else { last };
    let length_f = length as f32;
    let slope = (end - start) / length_f.sqrt();
    let (mut k, mut p) = lead_in(out, start, end, offset, limit, slope, length_f);
    let increasing = !(end - start < 0.0);
    let groups = (limit - p + 1) / 4;
    for _ in 0..4 * groups {
        p += 1;
        out[k] = if increasing { slope.mul_add(vmx_sqrt(p as f32), start) } else { (slope * -1.0).mul_add(vmx_sqrt(length_f - p as f32), end) };
        k += 1;
    }
    while p <= limit {
        p += 1;
        out[k] = if increasing { (p as f32).sqrt().mul_add(slope, start) } else { fnmsubs((length_f - p as f32).sqrt(), slope, end) };
        k += 1;
    }
    out[k..].fill(end);
}

/// `rw::audio::core::GainFader`.
pub struct GainFader {
    base: PlugInBase,
    /// +0x30..+0x40: the fade `StartFadeHandler` queued.
    pending_start: f64,
    pending_duration: f32,
    pending_target: f32,
    pending_curve: i32,
    /// +0x48..+0x60: the fade in progress.
    start_time: f64,
    duration: f32,
    length: i32,
    offset: i32,
    start_gain: f32,
    end_gain: f32,
    /// +0x64.
    gain: f32,
    /// +0x68, +0x69 (0 idle, 1 waiting for the start time, 2 fading), +0x6A.
    pending: bool,
    state: u8,
    curve: u8,
}

impl GainFader {
    /// `CreateInstance` 0x82bfd640: gain 1.
    pub fn new(input_channels: u8, output_channels: u8) -> Self {
        let mut base = PlugInBase::new(&super::GAIN_FADER, input_channels, output_channels);
        base.attributes[0] = 1.0;
        Self {
            base,
            pending_start: 0.0,
            pending_duration: 0.0,
            pending_target: 0.0,
            pending_curve: 0,
            start_time: 0.0,
            duration: 0.0,
            length: 0,
            offset: 0,
            start_gain: 0.0,
            end_gain: 0.0,
            gain: 1.0,
            pending: false,
            state: 0,
            curve: 0,
        }
    }

    /// `StartFadeHandler` 0x82bf6128 (event 0; `EventEvent` 0x82bfd6b8 truncates the curve): a
    /// fade starting now with no duration sets the gain at once.
    pub fn start_fade(&mut self, start: f64, duration: f32, target: f32, curve: i32) {
        if start == 0.0 && duration == 0.0 {
            self.gain = target;
            self.state = 0;
            self.pending = false;
            return;
        }
        self.pending_start = start;
        self.pending_duration = duration;
        self.pending_target = target;
        self.pending_curve = curve;
        self.pending = true;
    }
}

impl PlugIn for GainFader {
    impl_plugin_any!();

    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let rate = services.mixer.params_sample_rate;
        if self.pending {
            self.start_gain = self.gain;
            self.start_time = self.pending_start;
            self.curve = self.pending_curve as u8;
            self.duration = self.pending_duration;
            self.end_gain = self.pending_target;
            self.length = fctiwz(self.pending_duration * rate);
            if self.length <= 0 {
                self.length = 1;
            }
            self.state = 1;
            self.pending = false;
        }
        if self.state == 1 {
            let wait = if self.start_time == 0.0 { 0.0 } else { self.start_time - services.mixer.mix_time };
            let samples = fctiwz_f64(f64::from(rate) * wait);
            if samples < FRAME_SAMPLES as i32 {
                let offset = -samples;
                self.offset = offset;
                if offset > self.length - 1 {
                    self.state = 0;
                } else {
                    if offset > 0 {
                        self.offset = 0;
                    }
                    self.state = 2;
                }
            }
        }
        let mut gains = [0.0f32; FRAME_SAMPLES];
        if self.state == 2 {
            match self.curve {
                1 => linear_power(&mut gains, self.start_gain, self.end_gain, self.offset, self.length),
                _ => linear_amplitude(&mut gains, self.start_gain, self.end_gain, self.offset, self.length),
            }
            self.offset += FRAME_SAMPLES as i32;
            if self.offset >= self.length {
                self.state = 0;
            }
        } else {
            if self.gain == 1.0 {
                return BufferStatus::Available;
            }
            gains = [self.gain; FRAME_SAMPLES];
        }
        for channel in 0..usize::from(self.base.input_channels) {
            for (sample, gain) in services.mixer.current_mut(channel).iter_mut().zip(gains) {
                *sample *= gain;
            }
        }
        self.gain = gains[FRAME_SAMPLES - 1];
        self.base.attributes[0] = f64::from(self.gain);
        BufferStatus::Available
    }
}
