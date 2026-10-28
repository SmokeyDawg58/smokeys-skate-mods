//! `rw::audio::core::Iir2` (`Filter` 0x82be04e0 and the coefficient calculators) and the plug-ins
//! built on it: `HighPassIir2` (`Process` 0x82beca08), `LowPassIir2` (0x82bed478),
//! `HighShelfIir2` (0x82becb80) and `PeakingIir2` (0x82bf85f8). Every plug-in filters whole
//! 256-sample frames, so only the eight-sample unrolled `Filter` path runs.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;

/// `__real_219392ef` (1e-18), added to every feed-forward sum.
const DENORMAL: f32 = f32::from_bits(0x2193_92EF);
/// The w range the plug-ins filter in (`__real_3b4de32f`, `__real_4048dc62`) and 2 pi.
const MIN_W: f32 = f32::from_bits(0x3B4D_E32F);
const MAX_W: f32 = f32::from_bits(0x4048_DC62);
const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
/// `__real_3f3504ef` (0.70710653).
const HALF_SQRT2: f32 = f32::from_bits(0x3F35_04EF);
/// The Q range `PeakingIir2::Process` clamps to (`__real_3e4ccccd`, `__real_41a00000`).
const MIN_Q: f32 = f32::from_bits(0x3E4C_CCCD);
const MAX_Q: f32 = f32::from_bits(0x41A0_0000);

/// `fnmsubs`: -(a * c - b) with one rounding.
fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    -a.mul_add(c, -b)
}

/// `Iir2::FilterCoefficients`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Coefficients {
    pub a1: f32,
    pub a2: f32,
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
}

impl Coefficients {
    /// `Iir2::CalculateLowPassFilterCoefficients` 0x82be0690.
    #[must_use]
    pub fn low_pass(w: f32) -> Self {
        let s = f64::from(w).sin() as f32;
        let c = f64::from(w).cos() as f32;
        let alpha = s * 0.5;
        let one_minus_c = 1.0 - c;
        let a0 = alpha + 1.0;
        let inv = 1.0 / a0;
        let twice = a0 * 2.0;
        Self { a1: (c * -2.0) * inv, a2: (1.0 - alpha) * inv, b0: one_minus_c / twice, b1: one_minus_c * inv, b2: one_minus_c / twice }
    }

    /// `Iir2::CalculateHighPassFilterCoefficients` 0x82be0818.
    #[must_use]
    pub fn high_pass(w: f32) -> Self {
        let s = f64::from(w).sin() as f32;
        let c = f64::from(w).cos() as f32;
        let alpha = s * 0.5;
        let one_plus_c = c + 1.0;
        let a0 = alpha + 1.0;
        let inv = 1.0 / a0;
        let twice = a0 * 2.0;
        Self { a1: (c * -2.0) * inv, a2: (1.0 - alpha) * inv, b0: one_plus_c / twice, b1: -(one_plus_c * inv), b2: one_plus_c / twice }
    }

    /// `Iir2::CalculateHighShelfFilterCoefficients` 0x82be08d0.
    #[must_use]
    pub fn high_shelf(w: f32, gain: f32) -> Self {
        let s = f64::from(w).sin() as f32;
        let c = f64::from(w).cos() as f32;
        let a = gain.sqrt();
        let plus = a + 1.0;
        let minus = a - 1.0;
        let k = a.sqrt() * (s * HALF_SQRT2);
        let p = fnmsubs(minus, c, plus);
        let inv = 1.0 / k.mul_add(2.0, p);
        let a1 = (fnmsubs(plus, c, minus) * inv) * 2.0;
        let a2 = fnmsubs(k, 2.0, p) * inv;
        let b0 = ((k.mul_add(2.0, minus * c) + plus) * inv) * a;
        let b1 = ((plus.mul_add(c, minus) * inv) * a) * -2.0;
        let b2 = (fnmsubs(k, 2.0, minus.mul_add(c, plus)) * inv) * a;
        Self { a1, a2, b0, b1, b2 }
    }

    /// `PeakingIir2::CalculateFilterCoefficients` 0x82bf04f8: `gain` is linear, its root the
    /// shelf amplitude.
    #[must_use]
    pub fn peaking(w: f32, gain: f32, q: f32) -> Self {
        let s = f64::from(w).sin() as f32;
        let c = f64::from(w).cos() as f32;
        let a = gain.sqrt();
        let alpha = s / (q * 2.0);
        let c2 = c * -2.0;
        let over = alpha / a;
        let under = a * alpha;
        let inv = 1.0 / (over + 1.0);
        Self { a1: inv * c2, a2: (1.0 - over) * inv, b0: (under + 1.0) * inv, b1: inv * c2, b2: (1.0 - under) * inv }
    }
}

/// `rw::audio::core::Iir2`: one channel's input and output history.
#[derive(Debug, Clone, Copy, Default)]
pub struct Iir2 {
    in1: f32,
    in2: f32,
    out1: f32,
    out2: f32,
}

impl Iir2 {
    /// `Iir2::Filter` 0x82be04e0, eight samples per step (the length is a multiple of eight).
    pub fn filter(&mut self, dst: &mut [f32], src: &[f32], c: &Coefficients) {
        let (a1, a2, b0, b1, b2) = (c.a1, c.a2, c.b0, c.b1, c.b2);
        let (mut d1, mut d2, mut y1, mut y2) = (self.in1, self.in2, self.out1, self.out2);
        for (x, y) in src.chunks_exact(8).zip(dst.chunks_exact_mut(8)) {
            let p0 = x[0] * b0;
            let p26 = x[2] * b1;
            let p30 = x[1] * b0;
            let p22 = x[4] * b1;
            let p20 = x[6] * b1;
            let p28 = x[1] * b1;
            let p24 = x[3] * b1;
            let p21 = x[5] * b1;
            let s0 = d2.mul_add(b2, p0);
            let s3 = x[1].mul_add(b2, p26);
            let s1 = x[0].mul_add(b1, p30);
            let s5 = x[3].mul_add(b2, p22);
            let s7 = x[5].mul_add(b2, p20);
            let s2 = x[0].mul_add(b2, p28);
            let s4 = x[2].mul_add(b2, p24);
            let s6 = x[4].mul_add(b2, p21);
            let f0 = d1.mul_add(b1, s0) + DENORMAL;
            let f3 = x[3].mul_add(b0, s3) + DENORMAL;
            let f1 = d1.mul_add(b2, s1) + DENORMAL;
            let f7 = x[7].mul_add(b0, s7) + DENORMAL;
            let f2 = x[2].mul_add(b0, s2) + DENORMAL;
            let f4 = x[4].mul_add(b0, s4) + DENORMAL;
            let f5 = x[5].mul_add(b0, s5) + DENORMAL;
            let f6 = x[6].mul_add(b0, s6) + DENORMAL;
            d2 = x[6];
            d1 = x[7];
            let o0 = fnmsubs(y2, a2, fnmsubs(y1, a1, f0));
            let o1 = fnmsubs(y1, a2, fnmsubs(o0, a1, f1));
            let o2 = fnmsubs(o0, a2, fnmsubs(o1, a1, f2));
            let o3 = fnmsubs(o1, a2, fnmsubs(o2, a1, f3));
            let o4 = fnmsubs(o2, a2, fnmsubs(o3, a1, f4));
            let o5 = fnmsubs(o3, a2, fnmsubs(o4, a1, f5));
            let o6 = fnmsubs(o4, a2, fnmsubs(o5, a1, f6));
            let o7 = fnmsubs(o5, a2, fnmsubs(o6, a1, f7));
            y.copy_from_slice(&[o0, o1, o2, o3, o4, o5, o6, o7]);
            y1 = o7;
            y2 = o6;
        }
        *self = Self { in1: d1, in2: d2, out1: y1, out2: y2 };
    }
}

/// The w of a cut-off attribute: `(frequency / rate) * 2 pi`.
fn omega(frequency: f32, rate: f32) -> f32 {
    (frequency / rate) * TWO_PI
}

/// Filters every output channel from the current buffer into the scratch buffer, then swaps.
fn filter_channels(states: &mut [Iir2], coefficients: &Coefficients, channels: u8, services: &mut Services<'_>) {
    for channel in 0..usize::from(channels) {
        let (current, scratch) = services.mixer.current_and_scratch_channel(channel);
        states[channel].filter(scratch, current, coefficients);
    }
    services.mixer.swap();
}

/// The six channel histories `PlugIn::Initialize` clears.
const CHANNELS: usize = 6;

/// `rw::audio::core::HighPassIir2` and `LowPassIir2` (`CreateInstance` 0x82bf6ac8).
pub struct PassIir2 {
    base: PlugInBase,
    high: bool,
    states: [Iir2; CHANNELS],
    coefficients: Coefficients,
    /// +0xA4: the w of the coefficients (the frequency attribute until the first process).
    last_w: f32,
}

impl PassIir2 {
    /// `CreateInstance`: 450 decay samples.
    pub fn new(core: &mut VoiceCore, high: bool, input_channels: u8, output_channels: u8) -> Self {
        let desc = if high { &super::HIGH_PASS_IIR2 } else { &super::LOW_PASS_IIR2 };
        let mut base = PlugInBase::new(desc, input_channels, output_channels);
        let last_w = base.attr(0);
        core.decay_samples += 450.0 - base.decay_samples;
        base.decay_samples = 450.0;
        Self { base, high, states: [Iir2::default(); CHANNELS], coefficients: Coefficients::default(), last_w }
    }
}

impl PlugIn for PassIir2 {
    impl_plugin_any!();

    /// `HighPassIir2::Process` 0x82beca08 / `LowPassIir2::Process` 0x82bed478.
    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let mut w = omega(self.base.attr(0), services.mixer.params_sample_rate);
        let active = if self.high { w > MIN_W } else { w < MAX_W };
        if !active {
            let was_active = if self.high { self.last_w > MIN_W } else { self.last_w < MAX_W };
            if was_active {
                self.states = [Iir2::default(); CHANNELS];
            }
            self.last_w = w;
            return BufferStatus::Available;
        }
        if self.high {
            if !(w <= MAX_W) {
                w = MAX_W;
            }
        } else if !(w >= MIN_W) {
            w = MIN_W;
        }
        if !(w == self.last_w) {
            self.coefficients = if self.high { Coefficients::high_pass(w) } else { Coefficients::low_pass(w) };
            self.last_w = w;
        }
        filter_channels(&mut self.states, &self.coefficients, self.base.output_channels, services);
        BufferStatus::Available
    }
}

/// `rw::audio::core::HighShelfIir2` (`CreateInstance` 0x82bf71e0).
pub struct HighShelfIir2 {
    base: PlugInBase,
    states: [Iir2; CHANNELS],
    /// +0x98: set while filtering.
    active: bool,
    coefficients: Coefficients,
    last_w: f32,
    last_gain: f32,
}

impl HighShelfIir2 {
    /// `CreateInstance`: 700 decay samples.
    pub fn new(core: &mut VoiceCore, input_channels: u8, output_channels: u8) -> Self {
        let mut base = PlugInBase::new(&super::HIGH_SHELF_IIR2, input_channels, output_channels);
        let (last_w, last_gain) = (base.attr(0), base.attr(1));
        core.decay_samples += 700.0 - base.decay_samples;
        base.decay_samples = 700.0;
        Self { base, states: [Iir2::default(); CHANNELS], active: false, coefficients: Coefficients::default(), last_w, last_gain }
    }
}

impl PlugIn for HighShelfIir2 {
    impl_plugin_any!();

    /// `HighShelfIir2::Process` 0x82becb80.
    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let mut w = omega(self.base.attr(0), services.mixer.params_sample_rate);
        let gain = self.base.attr(1);
        if !(w < MAX_W) || gain == 1.0 {
            if self.active {
                self.states = [Iir2::default(); CHANNELS];
                self.active = false;
            }
            self.last_w = w;
            self.last_gain = gain;
            return BufferStatus::Available;
        }
        self.active = true;
        if !(w >= MIN_W) {
            w = MIN_W;
        }
        if !(w == self.last_w) || !(gain == self.last_gain) {
            self.coefficients = Coefficients::high_shelf(w, gain);
            self.last_w = w;
            self.last_gain = gain;
        }
        filter_channels(&mut self.states, &self.coefficients, self.base.output_channels, services);
        BufferStatus::Available
    }
}

/// `rw::audio::core::PeakingIir2` (`CreateInstance` 0x82bf8580): attributes frequency, linear gain
/// and Q.
pub struct PeakingIir2 {
    base: PlugInBase,
    states: [Iir2; CHANNELS],
    /// +0xA0: set while filtering.
    active: bool,
    coefficients: Coefficients,
    /// +0xB8..+0xC0: the w, gain and Q of the coefficients (the attributes until the first
    /// process).
    last_w: f32,
    last_gain: f32,
    last_q: f32,
}

impl PeakingIir2 {
    /// `CreateInstance`: 1500 decay samples.
    pub fn new(core: &mut VoiceCore, input_channels: u8, output_channels: u8) -> Self {
        let mut base = PlugInBase::new(&super::PEAKING_IIR2, input_channels, output_channels);
        let (last_w, last_gain, last_q) = (base.attr(0), base.attr(1), base.attr(2));
        core.decay_samples += 1500.0 - base.decay_samples;
        base.decay_samples = 1500.0;
        Self { base, states: [Iir2::default(); CHANNELS], active: false, coefficients: Coefficients::default(), last_w, last_gain, last_q }
    }
}

impl PlugIn for PeakingIir2 {
    impl_plugin_any!();

    /// `PeakingIir2::Process` 0x82bf85f8: unity gain passes the signal through.
    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        let mut w = omega(self.base.attr(0), services.mixer.params_sample_rate);
        if w < MIN_W {
            w = MIN_W;
        } else if !(w <= MAX_W) {
            w = MAX_W;
        }
        let gain = self.base.attr(1);
        let q = self.base.attr(2);
        if gain == 1.0 {
            if self.active {
                self.states = [Iir2::default(); CHANNELS];
                self.active = false;
            }
            self.last_w = w;
            self.last_gain = gain;
            self.last_q = q;
            return BufferStatus::Available;
        }
        self.active = true;
        if !(w == self.last_w) || !(gain == self.last_gain) || !(q == self.last_q) {
            let clamped = if q < MIN_Q {
                MIN_Q
            } else if !(q <= MAX_Q) {
                MAX_Q
            } else {
                q
            };
            self.coefficients = Coefficients::peaking(w, gain, clamped);
            self.last_w = w;
            self.last_gain = gain;
            self.last_q = q;
        }
        filter_channels(&mut self.states, &self.coefficients, self.base.output_channels, services);
        BufferStatus::Available
    }
}

#[cfg(test)]
mod tests {
    use super::{omega, Coefficients};

    /// |H(e^jw)| of the biquad.
    fn magnitude(c: &Coefficients, w: f32) -> f64 {
        let w = f64::from(w);
        let (b0, b1, b2) = (f64::from(c.b0), f64::from(c.b1), f64::from(c.b2));
        let (a1, a2) = (f64::from(c.a1), f64::from(c.a2));
        let (nr, ni) = (b0 + b1 * w.cos() + b2 * (2.0 * w).cos(), -(b1 * w.sin() + b2 * (2.0 * w).sin()));
        let (dr, di) = (1.0 + a1 * w.cos() + a2 * (2.0 * w).cos(), -(a1 * w.sin() + a2 * (2.0 * w).sin()));
        (nr.hypot(ni)) / (dr.hypot(di))
    }

    #[test]
    fn peaking_cuts_its_band_to_the_gain_and_passes_the_rest() {
        let w = omega(4500.0, 48000.0);
        let c = Coefficients::peaking(w, 0.1, 20.0);
        assert!((magnitude(&c, w) - 0.1).abs() < 1e-3);
        assert!((magnitude(&c, 0.0) - 1.0).abs() < 1e-3);
        assert!((magnitude(&c, 3.0) - 1.0).abs() < 1e-2);
    }
}
