//! `rw::audio::core::Pan2D1` and `Pan2DKernel`/`Pan2DKernelBase`: pans 1, 2, 4 or 5 emitters
//! over 2, 4 or 5 speakers (plus LFE) from an angle, distance, size and twist.
//!
//! Speakers sit at the front angle (FL), 0 (C), minus the front angle (FR), the rear angle (SL)
//! and minus the rear angle (SR). An emitter's level per speaker is an interior term (distance to
//! each speaker, weighted front/rear by the emitter's x) plus a perimeter term (the emitter's
//! direction projected on the two speakers of its sector), normalised to unit power.

use crate::math::{fmadd, fnmsub};
use crate::rwac::kernels::{copy_with_gain, copy_with_gain_ramp, mix_with_gain, mix_with_gain_ramp};
use crate::rwac::plugin::{impl_plugin_any, BufferStatus, PlugIn, PlugInBase, Services};
use crate::rwac::voice::VoiceCore;
use crate::rwac::FRAME_SAMPLES;

/// `0x3c8efa35`.
const DEG_TO_RAD: f32 = 0.017_453_292;
const TWO_PI: f32 = 6.283_185_5;
const HALF_PI: f32 = 1.570_796_4;
const PI: f32 = std::f32::consts::PI;

fn cos(x: f32) -> f32 {
    f64::from(x).cos() as f32
}

fn sin(x: f32) -> f32 {
    f64::from(x).sin() as f32
}

#[derive(Debug, Clone, Copy, Default)]
struct Speaker {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Emitter {
    x: f32,
    y: f32,
    /// Squared distance, clamped to 1.0.
    dist: f32,
    angle: f32,
}

#[derive(Debug, Default)]
struct Kernel {
    speakers: [Speaker; 5],
    num_speakers: i32,
    front_angle: f32,
    rear_angle: f32,
    front_projection: f32,
    front_xform: [[f32; 2]; 2],
    right_xform: [[f32; 2]; 2],
    rear_xform: [[f32; 2]; 2],
    left_xform: [[f32; 2]; 2],
    num_emitters: i32,
    emitters: [Emitter; 5],
}

impl Kernel {
    /// `Pan2DKernelBase::SpeakerConfig` 0x82be6f40.
    fn speaker_config(&mut self, num_speakers: i32, front_degrees: f32, rear_degrees: f32) {
        self.front_angle = front_degrees * DEG_TO_RAD;
        self.rear_angle = rear_degrees * DEG_TO_RAD;
        self.num_speakers = num_speakers;
        self.front_projection = cos(self.front_angle) * 2.0;
        let front = self.front_angle;
        let rear = self.rear_angle;
        self.speakers[0] = Speaker { x: cos(front), y: sin(front) };
        self.speakers[2] = Speaker { x: cos(-front), y: sin(-front) };
        self.speakers[3] = Speaker { x: cos(rear), y: sin(rear) };
        self.speakers[4] = Speaker { x: cos(-rear), y: sin(-rear) };
        self.speakers[1] = Speaker { x: 1.0, y: 0.0 };
        let [s0, _, s2, s3, s4] = self.speakers;

        let inv = 1.0 / fmadd(s0.x, s2.y, -(s0.y * s2.x));
        self.front_xform = [[inv * s0.x, -(inv * s0.y)], [-(inv * s2.x), inv * s2.y]];

        let inv = 1.0 / fmadd(s3.x, s0.y, -(s3.y * s0.x));
        self.left_xform = [[inv * s3.x, -(inv * s3.y)], [-(inv * s0.x), inv * s0.y]];

        let inv = 1.0 / fmadd(s4.x, s3.y, -(s4.y * s3.x));
        self.rear_xform = [[inv * s4.x, -(inv * s4.y)], [-(inv * s3.x), inv * s3.y]];

        let inv = 1.0 / fmadd(s2.x, s4.y, -(s2.y * s4.x));
        self.right_xform = [[inv * s2.x, -(inv * s2.y)], [-(inv * s4.x), inv * s4.y]];
    }

    /// One emitter at `(cx + x, cy + y)`: squared distance, clamp to the unit circle, angle.
    fn place(x: f32, y: f32) -> Emitter {
        let mut e = Emitter { x, y, dist: fmadd(y, y, x * x), angle: 0.0 };
        if e.dist > 1.0 {
            let inv = 1.0 / e.dist.sqrt();
            e.dist = 1.0;
            e.x = inv * x;
            e.y = inv * y;
        }
        e.angle = f64::from(e.y).atan2(f64::from(e.x)) as f32;
        e
    }

    fn offset_emitter(angle: f32, size: f32, x: f32, y: f32) -> Emitter {
        let c = cos(angle) * size;
        let s = sin(angle) * size;
        Self::place(c + x, s + y)
    }

    /// `Pan2DKernel::EmitterConfig` 0x82be1da0.
    #[allow(clippy::too_many_arguments)]
    fn emitter_config(
        &mut self,
        num_emitters: i32,
        front_degrees: f32,
        rear_degrees: f32,
        pan_angle: f32,
        pan_distance: f32,
        pan_size: f32,
        pan_twist: f32,
    ) {
        let mut a = pan_angle * -DEG_TO_RAD;
        let twist = pan_twist * -DEG_TO_RAD;
        self.num_emitters = num_emitters;
        let x = cos(a) * pan_distance;
        let y = sin(a) * pan_distance;
        match num_emitters {
            1 => {
                let mut e = Emitter { x, y, dist: fmadd(y, y, x * x), angle: 0.0 };
                if e.dist > 1.0 {
                    let inv = 1.0 / e.dist.sqrt();
                    e.dist = 1.0;
                    e.x = inv * x;
                    e.y = inv * y;
                }
                if pan_distance <= 0.0 {
                    a += PI;
                }
                e.angle = a;
                self.emitters[0] = e;
            }
            2 => {
                if front_degrees != 90.0 {
                    let w = front_degrees * DEG_TO_RAD;
                    self.emitters[0] = Self::offset_emitter((w + a) + twist, pan_size, x, y);
                    self.emitters[1] = Self::offset_emitter((a + twist) - w, pan_size, x, y);
                } else {
                    let angle = (a + twist) + HALF_PI;
                    let c = cos(angle) * pan_size;
                    let s = sin(angle) * pan_size;
                    self.emitters[0] = Self::place(c + x, s + y);
                    self.emitters[1] = Self::place(x - c, y - s);
                }
            }
            4 | 5 => {
                if num_emitters == 5 {
                    self.emitters[1] = Self::offset_emitter(a + twist, pan_size, x, y);
                }
                let (i1, i2, i3) = if num_emitters == 4 { (1, 2, 3) } else { (2, 3, 4) };
                let w = front_degrees * DEG_TO_RAD;
                let r = rear_degrees * DEG_TO_RAD;
                self.emitters[0] = Self::offset_emitter((w + a) + twist, pan_size, x, y);
                self.emitters[i1] = Self::offset_emitter((a + twist) - w, pan_size, x, y);
                self.emitters[i2] = Self::offset_emitter((r + a) + twist, pan_size, x, y);
                self.emitters[i3] = Self::offset_emitter((a + twist) - r, pan_size, x, y);
            }
            _ => {}
        }
    }

    /// `Pan2DKernelBase::ComputeInteriorTerm` 0x82be19a8 (writes all five levels).
    fn interior_term(&self, e: &Emitter, center_level: f32, out: &mut [f32; 5]) {
        let [s0, s1, s2, s3, s4] = self.speakers;
        let dx0 = s0.x - e.x;
        let dx2 = s2.x - e.x;
        let dx3 = s3.x - e.x;
        let dx4 = s4.x - e.x;
        let dy0 = s0.y - e.y;
        let dy2 = s2.y - e.y;
        let dy3 = s3.y - e.y;
        let dy4 = s4.y - e.y;
        let mut front = (e.x + 1.0) * 0.5;
        let r0 = fmadd(dy0, dy0, dx0 * dx0).sqrt();
        let r2 = fmadd(dy2, dy2, dx2 * dx2).sqrt();
        let r3 = fmadd(dy3, dy3, dx3 * dx3).sqrt();
        let r4 = fmadd(dy4, dy4, dx4 * dx4).sqrt();
        let g0 = fnmsub(r0, 0.5, 1.0);
        let g2 = fnmsub(r2, 0.5, 1.0);
        let g3 = fnmsub(r3, 0.5, 1.0);
        let g4 = fnmsub(r4, 0.5, 1.0);
        const EPS: f32 = 0.0005;
        if front.abs() < EPS {
            front = 0.0;
        }
        let mut rear = 1.0 - front;
        if rear.abs() < EPS {
            rear = 0.0;
        }
        let (l0, l1, l2);
        if self.num_speakers == 5 {
            let dx1 = s1.x - e.x;
            let dy1 = s1.y - e.y;
            let sum = fmadd(g2, g2, g0 * g0);
            let r1 = fmadd(dy1, dy1, dx1 * dx1).sqrt();
            let c = fnmsub(r1, 0.5, 1.0) * center_level;
            let k = (front / fmadd(c, c, sum)).sqrt();
            l0 = k * g0;
            l2 = k * g2;
            l1 = k * c;
        } else {
            let k = (front / fmadd(g2, g2, g0 * g0)).sqrt();
            l0 = k * g0;
            l2 = k * g2;
            l1 = 0.0;
        }
        let w = fnmsub(e.dist, e.dist, 1.0).sqrt();
        let k2 = (rear / fmadd(g4, g4, g3 * g3)).sqrt();
        out[0] = w * l0;
        out[2] = w * l2;
        out[1] = w * l1;
        out[3] = (k2 * g3) * w;
        out[4] = (k2 * g4) * w;
    }

    /// `Pan2DKernelBase::ComputePerimeterTerm` 0x82be1b50 (adds to the levels).
    fn perimeter_term(&self, e: &Emitter, center_level: f32, out: &mut [f32; 5]) {
        let turns = (e.angle + self.front_angle) * 0.159_154_94;
        let floor = f64::from(turns).floor() as f32;
        let wrapped = fmadd(turns - floor, TWO_PI, -self.front_angle);
        let s = sin(wrapped);
        let c = cos(wrapped);
        let project = |m: &[[f32; 2]; 2]| (fmadd(m[0][1], c, m[0][0] * s), fmadd(m[1][1], c, m[1][0] * s));
        if wrapped < self.front_angle {
            let (mut p0, mut p1) = project(&self.front_xform);
            let mut center = 0.0;
            if self.num_speakers == 5 {
                let m = if p0 >= p1 { p1 } else { p0 } * center_level;
                p0 -= m;
                p1 -= m;
                center = self.front_projection * m;
            }
            let n = fmadd(p0, p0, fmadd(center, center, p1 * p1)).sqrt();
            let k = e.dist / n;
            out[2] = fmadd(k, p0, out[2]);
            out[0] = fmadd(k, p1, out[0]);
            out[1] = fmadd(k, center, out[1]);
        } else if wrapped < self.rear_angle {
            let (q0, q1) = project(&self.left_xform);
            let k = e.dist / fmadd(q1, q1, q0 * q0).sqrt();
            out[0] = fmadd(q0, k, out[0]);
            out[3] = fmadd(q1, k, out[3]);
        } else if wrapped < TWO_PI - self.rear_angle {
            let (q0, q1) = project(&self.rear_xform);
            let k = e.dist / fmadd(q1, q1, q0 * q0).sqrt();
            out[3] = fmadd(q0, k, out[3]);
            out[4] = fmadd(q1, k, out[4]);
        } else {
            let (q0, q1) = project(&self.right_xform);
            let k = e.dist / fmadd(q1, q1, q0 * q0).sqrt();
            out[4] = fmadd(q0, k, out[4]);
            out[2] = fmadd(q1, k, out[2]);
        }
    }

    /// `Pan2DKernel::ComputeLevels` 0x82be72e8 / `ComputeLevels5` 0x82be7170.
    fn compute_levels(&self, main_level: f32, center_level: f32, normalization: f32, levels: &mut [[f32; 5]; 5]) {
        let emitters = self.num_emitters.max(0) as usize;
        if self.num_speakers == 2 {
            for (e, out) in self.emitters.iter().zip(levels.iter_mut()).take(emitters) {
                let left = (e.y + 1.0) * 0.5;
                let right = 1.0 - left;
                let g = ((1.0 / fmadd(right, right, left * left).sqrt()) * main_level) * normalization;
                out[0] = g * left;
                out[1] = right * g;
            }
        } else {
            for (e, out) in self.emitters.iter().zip(levels.iter_mut()).take(emitters) {
                self.interior_term(e, center_level, out);
                self.perimeter_term(e, center_level, out);
                let power = fmadd(out[4], out[4], fmadd(out[3], out[3], fmadd(out[2], out[2], fmadd(out[0], out[0], out[1] * out[1]))));
                let g = ((1.0 / power.sqrt()) * main_level) * normalization;
                for level in out.iter_mut() {
                    *level *= g;
                }
            }
        }
    }
}

pub struct Pan2D1 {
    base: PlugInBase,
    kernel: Kernel,
    /// angle, distance, size, twist, centre, main, LFE, source front angle, source rear angle.
    previous: [f32; 9],
    normalization: f32,
    num_emitters: usize,
    num_speakers: usize,
    level: [[f32; 5]; 5],
    lfe_level: f32,
}

/// `mDefaultConstructorFrontAngle`, `mDefaultConstructorRearAngle`, `mDefaultConstructorNormMode`.
pub const DEFAULT_CONSTRUCTOR: (f32, f32, f32) = (45.0, 135.0, 2.0);

impl Pan2D1 {
    /// `Pan2D1::CreateInstance` 0x82bf7ba8. `constructor` is (front angle, rear angle, norm mode).
    pub fn new(input_channels: u8, output_channels: u8, constructor: Option<(f32, f32, f32)>) -> Self {
        let mut base = PlugInBase::new(&super::PAN2D1, input_channels, output_channels);
        let num_emitters = if input_channels == 6 { 5 } else { usize::from(input_channels) };
        let num_speakers = if output_channels == 6 { 5 } else { usize::from(output_channels) };
        let (mut front, rear, norm_mode) = constructor.unwrap_or(DEFAULT_CONSTRUCTOR);
        if num_speakers == 2 {
            front = 90.0;
        }
        let normalization = if norm_mode == 0.0 {
            1.0
        } else if norm_mode == 1.0 {
            1.0 / num_emitters as f32
        } else if norm_mode == 2.0 {
            1.0 / (num_emitters as f32).sqrt()
        } else {
            0.0
        };
        base.attributes[7] = f64::from(front);
        base.attributes[8] = f64::from(rear);
        let previous = [
            base.attr(0),
            base.attr(1),
            base.attr(2),
            base.attr(3),
            base.attr(4),
            base.attr(5),
            base.attr(6),
            front,
            rear,
        ];
        let lfe_level = base.attr(6);
        let mut kernel = Kernel::default();
        kernel.speaker_config(num_speakers as i32, front, rear);
        Self {
            base,
            kernel,
            previous,
            normalization,
            num_emitters,
            num_speakers,
            level: [[0.0; 5]; 5],
            lfe_level,
        }
    }

    fn speaker_map(&self) -> [usize; 5] {
        if self.num_speakers == 4 { [0, 2, 3, 4, 4] } else { [0, 1, 2, 3, 4] }
    }

    /// `Pan2D1::PanOutput` 0x82bedfd8.
    fn pan_output(&self, services: &mut Services<'_>) {
        let map = self.speaker_map();
        let sources: Vec<Vec<f32>> = (0..self.num_emitters.max(1)).map(|e| services.mixer.current(e).to_vec()).collect();
        for k in 0..self.num_speakers {
            let dst = services.mixer.scratch_mut(k);
            copy_with_gain(dst, &sources[0], self.level[0][map[k]]);
            for e in 1..self.num_emitters {
                mix_with_gain(dst, &sources[e], self.level[e][map[k]]);
            }
        }
        if self.base.output_channels == 6 {
            if self.base.input_channels == 6 {
                let lfe = services.mixer.current(5).to_vec();
                copy_with_gain(services.mixer.scratch_mut(5), &lfe, self.lfe_level);
            } else {
                let dst = services.mixer.scratch_mut(5);
                copy_with_gain(dst, &sources[0], self.lfe_level);
                for source in sources.iter().take(self.num_emitters).skip(1) {
                    mix_with_gain(dst, source, self.lfe_level);
                }
            }
        }
    }

    /// `Pan2D1::RampPanOutput` 0x82bedcc8: levels ramp from `old` over 64 samples.
    fn ramp_pan_output(&self, services: &mut Services<'_>, old: &[[f32; 5]; 5], old_lfe: f32) {
        let map = self.speaker_map();
        let sources: Vec<Vec<f32>> = (0..self.num_emitters.max(1)).map(|e| services.mixer.current(e).to_vec()).collect();
        let step = |new: f32, old: f32| (new - old) * 0.015_625;
        for k in 0..self.num_speakers {
            let dst = services.mixer.scratch_mut(k);
            let level = map[k];
            copy_with_gain_ramp(dst, &sources[0], old[0][level], step(self.level[0][level], old[0][level]));
            for e in 1..self.num_emitters {
                mix_with_gain_ramp(dst, &sources[e], old[e][level], step(self.level[e][level], old[e][level]));
            }
        }
        if self.base.output_channels == 6 {
            let inc = step(self.lfe_level, old_lfe);
            if self.base.input_channels == 6 {
                let lfe = services.mixer.current(5).to_vec();
                copy_with_gain_ramp(services.mixer.scratch_mut(5), &lfe, old_lfe, inc);
            } else {
                let dst = services.mixer.scratch_mut(5);
                copy_with_gain_ramp(dst, &sources[0], old_lfe, inc);
                for source in sources.iter().take(self.num_emitters).skip(1) {
                    mix_with_gain_ramp(dst, source, old_lfe, inc);
                }
            }
        }
    }

    fn configure(&mut self, p: &[f32; 9]) {
        self.kernel.emitter_config(self.num_emitters as i32, p[7], p[8], p[0], p[1], p[2], p[3]);
        let mut level = self.level;
        self.kernel.compute_levels(p[5], p[4], self.normalization, &mut level);
        self.level = level;
    }
}

impl PlugIn for Pan2D1 {
    impl_plugin_any!();

    /// `Pan2D1::Process` 0x82bee200.
    fn process(&mut self, _voice: &mut VoiceCore, services: &mut Services<'_>, discontinuity: bool) -> BufferStatus {
        let p: [f32; 9] = std::array::from_fn(|i| self.base.attr(i));
        let unchanged = p.iter().zip(self.previous.iter()).all(|(a, b)| a == b);
        if unchanged {
            if discontinuity {
                self.configure(&p);
            }
            self.pan_output(services);
        } else {
            self.lfe_level = p[6];
            let old = self.level;
            self.configure(&p);
            if discontinuity {
                self.pan_output(services);
            } else {
                self.ramp_pan_output(services, &old, self.previous[6]);
            }
            self.previous = p;
        }
        services.mixer.swap();
        let _ = FRAME_SAMPLES;
        BufferStatus::Available
    }
}
