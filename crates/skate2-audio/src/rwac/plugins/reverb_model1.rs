//! `rw::audio::core::ReverbModel1` (`CreateInstance` 0x82bfeba8, `Process` 0x82bf99e8,
//! `RwacTimerClient` 0x82bf9428): six low-pass comb filters (`CombFilterUnoptimized` 0x82bd9a00)
//! summed, then one all-pass per output pair (`AllPassFilterFunc` 0x82bd8b68). Attribute 0 is
//! the reverb time, 1 the space size (comb lengths), 2 the brightness.
//!
//! A changed attribute is configured over three frames: `ConfigModelProcSide` 0x82bf9508
//! recomputes after the frame (state 1 to 2), the timer's `ConfigureModel` 0x82bf11b0 applies the
//! gains and lengths (to 3), and the next `Process` books the latency and decay (to 4).

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::float_cmp)]

use crate::math::{fmadd, fnmsub};
use crate::rwac::delay_line::{Chunk, DelayLine, Location};
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;
use crate::rwac::FRAME_SAMPLES;
use std::sync::OnceLock;

const COMBS: usize = 6;
/// `__real_3e2aaaab`: each comb's share of the sum.
const COMB_SCALE: f32 = f32::from_bits(0x3E2A_AAAB);
/// `G1_SAMPLERATES` and the reciprocals of their spacings (`__real_388bcf65`,
/// `__real_3827c5ac`).
const G1_RATES: [f32; 3] = [10000.0, 25000.0, 50000.0];
const G1_LOW_SPACING: f32 = f32::from_bits(0x388B_CF65);
const G1_HIGH_SPACING: f32 = f32::from_bits(0x3827_C5AC);
/// `mG1Values`: the comb distance axis (metres), then the low-pass gain at each distance for
/// each rate.
const G1_DISTANCES: [f32; 9] = [0.0, 12.5, 25.0, 37.5, 50.0, 62.5, 75.0, 87.5, 100.0];
const G1_VALUES: [[u32; 9]; 3] = [
    [0, 0x3CF5_C28F, 0x3D8F_5C29, 0x3DE1_47AE, 0x3E0F_5C29, 0x3E23_D70A, 0x3E38_51EC, 0x3E61_47AE, 0x3E75_C28F],
    [0, 0x3DF5_C28F, 0x3E6B_851F, 0x3E99_999A, 0x3EB3_3333, 0x3ECC_CCCD, 0x3EDC_28F6, 0x3EEB_851F, 0x3F00_0000],
    [0, 0x3E9E_B852, 0x3EE6_6666, 0x3F07_AE14, 0x3F11_EB85, 0x3F1C_28F6, 0x3F23_D70A, 0x3F2B_851F, 0x3F33_3333],
];
/// `__real_3a83126f`: how far the brightness divisor stays above the last comb's gain.
const BRIGHTNESS_MARGIN: f32 = f32::from_bits(0x3A83_126F);
/// `__real_3b3e11d2`: seconds per metre.
const SECONDS_PER_METRE: f32 = f32::from_bits(0x3B3E_11D2);
/// `__real_3ebb645a`: the shortest reverb time `CalculateG2Values` uses.
const MIN_TIME: f32 = f32::from_bits(0x3EBB_645A);
/// `ALLPASSDELAY_BASE`, `ALLPASSGAIN_BASE` and the other all-pass delays and gains
/// (`__real_3bda740e`, `__real_3f2147ae`, `__real_3bb0f27b`, `__real_3f471c72`).
const ALL_PASS_DELAY_BASE: f32 = f32::from_bits(0x3BC4_9BA6);
const ALL_PASS_GAIN_BASE: f32 = f32::from_bits(0x3F33_3333);
const ALL_PASS_DELAY_FRONT: f32 = f32::from_bits(0x3BDA_740E);
const ALL_PASS_GAIN_FRONT: f32 = f32::from_bits(0x3F21_47AE);
const ALL_PASS_DELAY_REAR: f32 = f32::from_bits(0x3BB0_F27B);
const ALL_PASS_GAIN_REAR: f32 = f32::from_bits(0x3F47_1C72);
/// `__real_219392ef`, added and taken away to flush tiny all-pass values.
const DENORMAL: f32 = f32::from_bits(0x2193_92EF);
/// The `log10` scale of the decay (`Compressor1::MAX_ATTACKTIME`'s 10.0, folded).
const DECAY_SCALE: f32 = 10.0;
const DEFAULT_RATE: f32 = 48000.0;

/// `mPrimesTo14000`: the 0x674 primes below 14000.
fn primes() -> &'static [f32] {
    static PRIMES: OnceLock<Vec<f32>> = OnceLock::new();
    PRIMES.get_or_init(|| {
        let mut composite = vec![false; 14000];
        let mut primes = Vec::new();
        for n in 2..14000 {
            if !composite[n] {
                primes.push(n as f32);
                for multiple in (n * n..14000).step_by(n) {
                    composite[multiple] = true;
                }
            }
        }
        primes
    })
}

/// `CombFilter` (`SetGains` 0x82bd9db0, `CombFilterResetFunc` 0x82bd9da0).
#[derive(Debug, Clone, Copy, Default)]
struct Comb {
    /// +0x10, +0x14, +0x18, +0x1C.
    a_1: f32,
    a_m: f32,
    b_m_plus_1: f32,
    scale: f32,
    /// +0x20: the last sample written.
    previous: f32,
}

/// `AllPassFilter` (`SetGains` 0x82bd8f78): +0x10 gain, +0x14 scale.
#[derive(Debug, Clone, Copy, Default)]
struct AllPass {
    gain: f32,
    scale: f32,
}

pub struct ReverbModel1 {
    base: PlugInBase,
    /// +0x40, +0x88.
    all_pass: [AllPass; 3],
    all_pass_lines: [DelayLine; 3],
    /// +0x154..+0x160: what the last configuration was made for. The size is not initialised
    /// natively; any real size differs from 0.
    last_rate: f32,
    last_time: f32,
    last_size: f32,
    last_brightness: f32,
    /// +0x164, +0x17C, +0x1AC, +0x1C4.
    distances: [f32; COMBS],
    comb_delays: [i32; COMBS],
    g1: [f32; COMBS],
    g2: [f32; COMBS],
    /// +0x1DC, +0x2B4.
    combs: [Comb; COMBS],
    comb_lines: [DelayLine; COMBS],
    /// +0x41C, +0x428, +0x434.
    all_pass_gains: [f32; 3],
    all_pass_delays: [i32; 3],
    all_pass_scale: f32,
    /// +0x438: the all-passes are set up.
    configured: bool,
    /// +0x439.
    all_pass_count: usize,
    /// +0x43C: 0 silent, 1 reconfiguring, 2 waiting for `ConfigureModel`, 3 configured,
    /// 4 running.
    state: u32,
    /// +0x444.
    sample_rate: f32,
}

impl ReverbModel1 {
    /// `CreateInstance`: one all-pass for mono, two for stereo and quad, three otherwise; the
    /// combs' lines hold the constructor's space size (none without it).
    pub fn new(input_channels: u8, output_channels: u8, space_size: Option<f32>) -> Self {
        let base = PlugInBase::new(&super::REVERB_MODEL1, input_channels, output_channels);
        let all_pass_count = match output_channels {
            1 => 1,
            2 | 4 => 2,
            _ => 3,
        };
        let mut comb_delays = [0; COMBS];
        if let Some(mut size) = space_size {
            let distances = calculate_comb_distances(&mut size);
            calculate_comb_delays(&distances, &mut comb_delays, DEFAULT_RATE);
            for delay in &mut comb_delays {
                *delay += 3;
            }
        }
        let mut comb_lines: [DelayLine; COMBS] = Default::default();
        for (line, delay) in comb_lines.iter_mut().zip(comb_delays) {
            line.init(1, delay, 2);
        }
        let mut all_pass_lines: [DelayLine; 3] = Default::default();
        for line in &mut all_pass_lines[..all_pass_count] {
            line.init(1, 0, 1);
        }
        let channels = f32::from(output_channels);
        let all_pass_scale = if output_channels > 4 { 2.0 / (channels - 1.0) } else { 2.0 / channels };
        Self {
            base,
            all_pass: [AllPass::default(); 3],
            all_pass_lines,
            last_rate: 0.0,
            last_time: 0.0,
            last_size: 0.0,
            last_brightness: 1.0,
            distances: [0.0; COMBS],
            comb_delays: [0; COMBS],
            g1: [0.0; COMBS],
            g2: [0.0; COMBS],
            combs: [Comb::default(); COMBS],
            comb_lines,
            all_pass_gains: [0.0; 3],
            all_pass_delays: [0; 3],
            all_pass_scale,
            configured: false,
            all_pass_count,
            state: 0,
            sample_rate: DEFAULT_RATE,
        }
    }

    /// `CalculateG1Values` 0x82bf1300: each comb's low-pass gain from its distance, the table
    /// interpolated by rate; a brightness change divides them by the brightness (at least the
    /// last gain plus 0.001) and silences the combs.
    fn calculate_g1_values(&mut self, rate: f32) {
        let (row, weight) = if !(rate < G1_RATES[2]) {
            (1, (G1_RATES[2] - G1_RATES[2]) * G1_HIGH_SPACING)
        } else if !(rate > G1_RATES[0]) {
            (0, (G1_RATES[1] - G1_RATES[0]) * G1_LOW_SPACING)
        } else if rate > G1_RATES[1] {
            (1, (G1_RATES[2] - rate) * G1_HIGH_SPACING)
        } else {
            (0, (G1_RATES[1] - rate) * G1_LOW_SPACING)
        };
        let rest = 1.0 - weight;
        let values: [f32; 9] = std::array::from_fn(|k| {
            let low = f32::from_bits(G1_VALUES[row][k]) * weight;
            fmadd(f32::from_bits(G1_VALUES[row + 1][k]), rest, low)
        });
        let spacing = 1.0 / (G1_DISTANCES[1] - G1_DISTANCES[0]);
        for (g1, distance) in self.g1.iter_mut().zip(self.distances) {
            let mut j = 1;
            while j < 8 && distance > G1_DISTANCES[j] {
                j += 1;
            }
            let part = (G1_DISTANCES[j] - distance) * spacing;
            *g1 = fmadd(values[j], 1.0 - part, values[j - 1] * part);
        }
        let brightness = self.base.attr(2);
        if brightness != self.last_brightness {
            let mut divisor = self.g1[5] + BRIGHTNESS_MARGIN;
            if brightness > divisor {
                divisor = brightness;
            }
            let inverse = 1.0 / divisor;
            for (comb, g1) in self.combs.iter_mut().zip(&mut self.g1) {
                comb.previous = 0.0;
                *g1 *= inverse;
            }
            for (line, delay) in self.comb_lines.iter_mut().zip(self.comb_delays) {
                if !line.resize_needed(delay + 3) {
                    line.reset(delay + 1);
                }
            }
        }
    }

    /// `ConfigModelProcSide`: a size change recomputes the comb lengths and gains, a rate change
    /// also redoes the all-passes, a brightness change only the gains; the feedback gains always.
    fn config_model_proc_side(&mut self) {
        if self.state != 1 {
            return;
        }
        let mut size_changed = false;
        let size = self.base.attr(1);
        if size != self.last_size {
            let mut size = size;
            self.distances = calculate_comb_distances(&mut size);
            self.base.attributes[1] = f64::from(size);
            calculate_comb_delays(&self.distances, &mut self.comb_delays, self.sample_rate);
            self.calculate_g1_values(self.sample_rate);
            size_changed = true;
        } else if self.sample_rate != self.last_rate {
            calculate_comb_delays(&self.distances, &mut self.comb_delays, self.sample_rate);
            self.calculate_g1_values(self.sample_rate);
            self.configured = false;
        } else if self.base.attr(2) != self.last_brightness {
            self.calculate_g1_values(self.sample_rate);
        }
        self.g2 = calculate_g2_values(self.base.attr(0), &self.g1);
        if !self.configured {
            let rate = self.sample_rate;
            let samples = |seconds: f32| {
                let scaled = rate * seconds;
                if scaled < 0.0 { (scaled - 0.5) as i32 } else { (scaled + 0.5) as i32 }
            };
            let (count, gains, delays) = match self.base.output_channels {
                1 => (1, [ALL_PASS_GAIN_BASE, 0.0, 0.0], [samples(ALL_PASS_DELAY_BASE), 0, 0]),
                2 | 4 => (2, [ALL_PASS_GAIN_FRONT, ALL_PASS_GAIN_REAR, 0.0], [samples(ALL_PASS_DELAY_FRONT), samples(ALL_PASS_DELAY_REAR), 0]),
                _ => (
                    3,
                    [ALL_PASS_GAIN_FRONT, ALL_PASS_GAIN_BASE, ALL_PASS_GAIN_REAR],
                    [samples(ALL_PASS_DELAY_FRONT), samples(ALL_PASS_DELAY_BASE), samples(ALL_PASS_DELAY_REAR)],
                ),
            };
            self.all_pass_count = count;
            self.all_pass_gains[..count].copy_from_slice(&gains[..count]);
            self.all_pass_delays[..count].copy_from_slice(&delays[..count]);
        } else if size_changed {
            for (line, delay) in self.all_pass_lines.iter_mut().zip(self.all_pass_delays).take(self.all_pass_count) {
                line.reset(delay);
            }
        }
        self.state = 2;
        self.last_time = self.base.attr(0);
        self.last_size = self.base.attr(1);
        self.last_brightness = self.base.attr(2);
        self.last_rate = self.sample_rate;
    }

    /// `ConfigureModel`: the comb gains (feedback through a one-pole low-pass, 1/6 each), new
    /// comb lengths, and the all-passes once.
    fn configure_model(&mut self) {
        for i in 0..COMBS {
            let comb = &mut self.combs[i];
            comb.a_1 = -self.g1[i];
            comb.a_m = -self.g2[i];
            comb.b_m_plus_1 = -self.g1[i];
            comb.scale = COMB_SCALE;
            let delay = self.comb_delays[i];
            if self.comb_lines[i].delay() != delay + 1 {
                self.comb_lines[i].resize(delay + 3);
                self.comb_lines[i].reset(delay + 1);
            }
        }
        if !self.configured {
            for i in 0..self.all_pass_count {
                self.all_pass[i] = AllPass { gain: self.all_pass_gains[i], scale: self.all_pass_scale };
                self.all_pass_lines[i].resize(self.all_pass_delays[i] + 2);
                self.all_pass_lines[i].reset(self.all_pass_delays[i]);
            }
            self.configured = true;
        }
    }

    /// `UpdateLatencyAndDecay` 0x82bf98c0: the longest comb's and all-pass's ring-out
    /// (`length - 10 * length / log10(gain)`), the first comb's feedback gain and the largest
    /// all-pass gain.
    fn update_latency_and_decay(&mut self, voice: &mut VoiceCore) {
        let comb_length = self.comb_delays[5] as f32;
        let comb_decay = comb_length - (comb_length * DECAY_SCALE) / f64::from(self.g2[0]).log10() as f32;
        let count = self.all_pass_count;
        let gain = self.all_pass_gains[..count].iter().fold(0.0f32, |max, &gain| if gain > max { gain } else { max });
        let length = self.all_pass_delays[..count].iter().fold(0, |max, &delay| if delay > max { delay } else { max }) as f32;
        let decay = (length - (length * DECAY_SCALE) / f64::from(gain).log10() as f32) + comb_decay;
        voice.decay_samples = (decay - self.base.decay_samples) + voice.decay_samples;
        self.base.decay_samples = decay;
    }

    /// Copies output channel 0 of the scratch buffer to `channel`.
    fn copy_first_channel(services: &mut Services<'_>, channel: usize) {
        let mut first = [0.0f32; FRAME_SAMPLES];
        first.copy_from_slice(services.mixer.scratch_mut(0));
        services.mixer.scratch_mut(channel).copy_from_slice(&first);
    }

    fn all_pass(&mut self, index: usize, services: &mut Services<'_>) {
        let all_pass = self.all_pass[index];
        self.all_pass_lines[index].apply_filter(services.mixer, |location, input, output, chunk| all_pass_filter(location, input, output, chunk, all_pass));
    }
}

/// `CalculateCombDistances` 0x82bf1598: the size clamped to 2..83.3 metres; the combs span
/// 0.8 to 1.2 times it in five equal steps (at most 100 metres).
fn calculate_comb_distances(size: &mut f32) -> [f32; COMBS] {
    if *size > f32::from_bits(0x42A6_999A) {
        *size = f32::from_bits(0x42A6_999A);
    } else if !(*size >= 2.0) {
        *size = 2.0;
    }
    let mut shortest = *size * f32::from_bits(0x3F4C_CCCD);
    let mut longest = shortest * 1.5;
    if longest > 100.0 {
        longest = 100.0;
        shortest = f32::from_bits(0x4285_5555);
        *size = f32::from_bits(0x42A6_AAAA);
    }
    let step = (longest - shortest) * f32::from_bits(0x3E4C_CCCD);
    let mut distances = [shortest, 0.0, 0.0, 0.0, 0.0, longest];
    for i in 1..5 {
        distances[i] = distances[i - 1] + step;
    }
    distances
}

/// `CalculateCombDelays` 0x82bf1648: each comb's travel time in samples rounded up to the next
/// prime, every comb a later prime than the one before (a comb past the table keeps its delay,
/// the last starting from 0); rates above 48 kHz scale the 48 kHz primes.
fn calculate_comb_delays(distances: &[f32; COMBS], delays: &mut [i32; COMBS], rate: f32) {
    let primes = primes();
    delays[5] = 0;
    let mut next = 0;
    for (delay, &distance) in delays.iter_mut().zip(distances) {
        let seconds = distance * SECONDS_PER_METRE;
        let (base, scale) = if rate > DEFAULT_RATE { (DEFAULT_RATE, rate * f32::from_bits(0x37AE_C33E)) } else { (rate, 1.0) };
        let samples = base * seconds;
        while next < primes.len() {
            let prime = primes[next];
            next += 1;
            if prime > samples {
                *delay = prime as i32;
                break;
            }
        }
        if scale > 1.0 {
            *delay = (*delay as f32 * scale) as i32;
        }
    }
}

/// `CalculateG2Values` 0x82bf1508: the feedback gain that rings for the reverb time (at least
/// 0.366 s) under each comb's low-pass.
fn calculate_g2_values(time: f32, g1: &[f32; COMBS]) -> [f32; COMBS] {
    let time = if time > MIN_TIME { time } else { MIN_TIME };
    let decay = 1.0 - MIN_TIME / time;
    std::array::from_fn(|i| (1.0 - g1[i]) * decay)
}

/// `CombFilterApplyFunc` 0x82bd9d18 / `CombFilterUnoptimized`: `w = x - a_1 w[-1] - a_M w[-M]`,
/// out `scale * (b w[-M-1] + w[-M])` written (mode 0) or added (mode 1). A cross-fading line
/// outputs silence and leaves its write buffer unset (natively stale scratch memory, here 0).
fn comb_filter(location: &mut Location<'_>, input: &[f32], output: &mut [f32], chunk: Chunk, comb: &mut Comb, accumulate: bool) {
    let range = chunk.start..chunk.start + chunk.count;
    if chunk.fade.is_some() {
        output[range].fill(0.0);
        return;
    }
    let mut previous = comb.previous;
    for i in range {
        let feedback = location.tap(i + 1, chunk.delay);
        let written = fnmsub(feedback, comb.a_m, fnmsub(previous, comb.a_1, input[i]));
        location.write[i] = written;
        let out = fmadd(location.tap(i, chunk.delay), comb.b_m_plus_1, location.tap(i + 1, chunk.delay));
        output[i] = if accumulate { fmadd(out, comb.scale, output[i]) } else { out * comb.scale };
        previous = written;
    }
    comb.previous = location.write[chunk.start + chunk.count - 1];
}

/// `AllPassFilterApplyFunc` 0x82bd8f30 / `AllPassFilterFunc`: `w = x - g d`, out
/// `scale * (g w + d)`, the delayed sample and `w` flushed of tiny values. A cross-fading line
/// outputs silence.
fn all_pass_filter(location: &mut Location<'_>, input: &[f32], output: &mut [f32], chunk: Chunk, all_pass: AllPass) {
    let range = chunk.start..chunk.start + chunk.count;
    if chunk.fade.is_some() {
        output[range].fill(0.0);
        return;
    }
    for i in range {
        let delayed = (location.tap(i, chunk.delay) + DENORMAL) - DENORMAL;
        location.set_tap(i, chunk.delay, delayed);
        let written = (fnmsub(delayed, all_pass.gain, input[i]) + DENORMAL) - DENORMAL;
        location.write[i] = written;
        output[i] = fmadd(written, all_pass.gain, delayed) * all_pass.scale;
    }
}

impl PlugIn for ReverbModel1 {
    impl_plugin_any!();

    /// `Process`: silence (and state 0) while the time is not positive; a changed attribute in
    /// state 0 silences the frame and starts configuring, otherwise the frame runs on the old
    /// configuration. The combs read input channel 0; the all-passes fill the outputs from the
    /// comb sum.
    fn process(&mut self, voice: &mut VoiceCore, services: &mut Services<'_>, _discontinuity: bool) -> BufferStatus {
        if self.state == 3 {
            self.update_latency_and_decay(voice);
            self.state = 4;
        }
        let outputs = usize::from(self.base.output_channels);
        let time = self.base.attr(0);
        if !(time > 0.0) {
            for channel in 0..outputs {
                services.mixer.current_mut(channel).fill(0.0);
            }
            self.state = 0;
            return BufferStatus::Available;
        }
        let changed = time != self.last_time
            || self.base.attr(1) != self.last_size
            || self.base.attr(2) != self.last_brightness
            || self.sample_rate != self.last_rate;
        if changed {
            if self.state == 0 {
                for channel in 0..outputs {
                    services.mixer.current_mut(channel).fill(0.0);
                }
                self.state = 1;
                self.config_model_proc_side();
                return BufferStatus::Available;
            }
            self.state = 1;
        }
        for (index, (comb, line)) in self.combs.iter_mut().zip(&mut self.comb_lines).enumerate() {
            line.apply_filter(services.mixer, |location, input, output, chunk| comb_filter(location, input, output, chunk, comb, index != 0));
        }
        services.mixer.swap();
        match outputs {
            1 => self.all_pass(0, services),
            2 => {
                self.all_pass(1, services);
                Self::copy_first_channel(services, 1);
                self.all_pass(0, services);
            }
            4 => {
                self.all_pass(1, services);
                Self::copy_first_channel(services, 1);
                Self::copy_first_channel(services, 3);
                self.all_pass(0, services);
                Self::copy_first_channel(services, 2);
            }
            _ => {
                self.all_pass(2, services);
                Self::copy_first_channel(services, 2);
                Self::copy_first_channel(services, 4);
                self.all_pass(1, services);
                Self::copy_first_channel(services, 1);
                self.all_pass(0, services);
                Self::copy_first_channel(services, 3);
                services.mixer.scratch_mut(5).fill(0.0);
            }
        }
        services.mixer.swap();
        self.config_model_proc_side();
        BufferStatus::Available
    }

    /// `RwacTimerClient`: runs `ConfigureModel` for a waiting configuration; with no reverb time
    /// after one, silences every line each frame.
    fn timer(&mut self, _voice: &mut VoiceCore, _services: &mut Services<'_>) {
        self.sample_rate = DEFAULT_RATE;
        if self.base.attr(0) > 0.0 {
            if self.state == 2 {
                self.configure_model();
                self.state = 3;
            }
        } else if self.last_time > 0.0 {
            for i in 0..COMBS {
                self.comb_lines[i].reset(self.comb_delays[i] + 1);
                self.combs[i].previous = 0.0;
            }
            for i in 0..self.all_pass_count {
                self.all_pass_lines[i].reset(self.all_pass_delays[i]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primes_match_the_table() {
        let primes = primes();
        assert_eq!(primes.len(), 0x674);
        assert_eq!(primes[..5], [2.0, 3.0, 5.0, 7.0, 11.0]);
        assert_eq!(primes[0x673], 13999.0);
    }

    #[test]
    fn comb_lengths_are_rising_primes() {
        let mut size = 45.0;
        let distances = calculate_comb_distances(&mut size);
        let mut delays = [0; COMBS];
        calculate_comb_delays(&distances, &mut delays, DEFAULT_RATE);
        assert!(delays.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(delays[0] > (36.0 * SECONDS_PER_METRE * DEFAULT_RATE) as i32);
    }
}
