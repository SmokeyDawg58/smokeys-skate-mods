//! `Sk8::Audio::Utils`: `PathLine`, a value that moves through up to five timed stages
//! (`ClearStages` 0x824e83d8, `AddStage` 0x824e8430, `AddLinkedStage` 0x824e8508, `Update`
//! 0x824f29a0), and `Slope`, a clamped linear map (`Initialize` 0x824e85a0, `GetValue`
//! 0x824e85f0).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::nicotine::shape::{curve_output, fctiwz};

/// `eCURVETYPE` values `Update` tells apart; every other value is linear.
pub const CURVE_LINEAR: i32 = 0;
const CURVE_EASE_IN: i32 = 1;
const CURVE_EASE_OUT: i32 = 2;
const CURVE_MIX_TABLE: i32 = 5;

/// `fnmsubs`: -(a * c - b) with one rounding.
fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    -a.mul_add(c, -b)
}

/// `fsel(a, b, c)`: `b` when `a >= 0`, otherwise (and for NaN) `c`.
#[must_use]
pub fn fsel(a: f32, b: f32, c: f32) -> f32 {
    if a >= 0.0 { b } else { c }
}

/// The `fsel` pair the audio code clamps ratios with: below 0 gives 0, above `max` gives `max`,
/// NaN gives `max`.
#[must_use]
pub fn clamp_fsel(value: f32, max: f32) -> f32 {
    let low = fsel(-value, 0.0, value);
    fsel(max - low, low, max)
}

/// `Sk8::Audio::Utils::PathLine` (124 bytes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathLine {
    elapsed_time: f32,
    length: [f32; 6],
    start: [f32; 6],
    finish: [f32; 6],
    is_linked: [bool; 6],
    curve_types: [i32; 6],
    num_stages: i32,
    cur_stage: i32,
    /// `CurValue`.
    pub cur_value: f32,
    /// `bComplete`.
    pub complete: bool,
}

impl Default for PathLine {
    /// The constructor 0x824f2970: `ClearStages`.
    fn default() -> Self {
        Self {
            elapsed_time: 0.0,
            length: [0.0; 6],
            start: [0.0; 6],
            finish: [0.0; 6],
            is_linked: [false; 6],
            curve_types: [CURVE_LINEAR; 6],
            num_stages: 0,
            cur_stage: 0,
            cur_value: 0.0,
            complete: true,
        }
    }
}

impl PathLine {
    pub fn clear_stages(&mut self) {
        *self = Self::default();
    }

    /// Stage lengths are milliseconds; a length that is not positive becomes 10 ms. The first
    /// stage sets the value to its start. Five stages at most (-1 after).
    pub fn add_stage(&mut self, start: f32, finish: f32, length_ms: i32, curve: i32) -> i32 {
        if self.num_stages == 5 {
            return -1;
        }
        let stage = self.num_stages as usize;
        self.length[stage] = length_ms as f32 * f32::from_bits(0x3A83_126F);
        if !(self.length[stage] > 0.0) {
            self.length[stage] = f32::from_bits(0x3C23_D70A);
        }
        self.finish[stage] = finish;
        self.start[stage] = start;
        self.curve_types[stage] = curve;
        self.is_linked[stage] = false;
        self.complete = false;
        if stage == 0 {
            self.cur_value = self.start[0];
        }
        self.num_stages += 1;
        self.num_stages
    }

    /// A stage that starts where the previous one finished (at the time it is reached).
    pub fn add_linked_stage(&mut self, finish: f32, length_ms: i32, curve: i32) -> i32 {
        if self.num_stages == 0 {
            return -1;
        }
        self.add_stage(0.0, finish, length_ms, curve);
        self.is_linked[self.num_stages as usize - 1] = true;
        self.num_stages
    }

    /// The time left over at the end of a stage carries into the next one.
    pub fn update(&mut self, delta_time: f32) {
        if self.complete || self.num_stages == 0 {
            return;
        }
        let stage = self.cur_stage as usize;
        self.elapsed_time += delta_time;
        let elapsed = self.elapsed_time;
        let length = self.length[stage];
        if !(elapsed <= length) {
            self.cur_value = self.finish[stage];
            if self.cur_stage < self.num_stages - 1 {
                self.elapsed_time = elapsed - length;
                self.cur_stage += 1;
                let next = self.cur_stage as usize;
                if self.is_linked[next] {
                    self.start[next] = self.finish[next - 1];
                }
            } else {
                self.complete = true;
            }
            return;
        }
        let (start, finish) = (self.start[stage], self.finish[stage]);
        self.cur_value = match self.curve_types[stage] {
            CURVE_EASE_IN => {
                let t = elapsed / length;
                ((finish - start) * t).mul_add(t, start)
            }
            CURVE_EASE_OUT => {
                let t = (1.0 / length).mul_add(elapsed, -1.0);
                fnmsubs(t, t, 1.0).mul_add(finish - start, start)
            }
            CURVE_MIX_TABLE => {
                let q15 = fctiwz((elapsed / length) * 32767.0).clamp(0, 0x7FFF);
                (curve_output(3, q15) as f32 * f32::from_bits(0x3800_0100)).mul_add(finish - start, start)
            }
            _ => (finish - start).mul_add(elapsed / length, start),
        };
    }
}

/// `Sk8::Audio::Utils::SlopeParams`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SlopeParams {
    pub min_input: f32,
    pub max_input: f32,
    pub min_output: f32,
    pub max_output: f32,
}

/// `Sk8::Audio::Utils::Slope` (the constructor zeroes it).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Slope {
    params: SlopeParams,
}

impl Slope {
    /// An input range narrower than 1e-6 is widened by 1e-6.
    pub fn initialize(&mut self, params: SlopeParams) {
        self.params = params;
        let epsilon = f32::from_bits(0x3586_37BD);
        if !((self.params.max_input - self.params.min_input).abs() >= epsilon) {
            self.params.max_input += epsilon;
        }
    }

    #[must_use]
    pub fn value(&self, input: f32) -> f32 {
        let p = &self.params;
        let t = clamp_fsel((input - p.min_input) / (p.max_input - p.min_input), 1.0);
        t.mul_add(p.max_output - p.min_output, p.min_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_line_runs_its_stages_and_carries_the_remainder() {
        let mut line = PathLine::default();
        assert!(line.complete);
        line.add_stage(1.0, 0.5, 100, CURVE_LINEAR);
        line.add_linked_stage(1.0, 100, CURVE_LINEAR);
        assert_eq!(line.cur_value, 1.0);
        line.update(0.05);
        assert!((line.cur_value - 0.75).abs() < 1e-6);
        // Past the first stage: its finish, and the second starts there with the remainder.
        line.update(0.06);
        assert_eq!(line.cur_value, 0.5);
        line.update(0.04);
        assert!((line.cur_value - 0.75).abs() < 1e-5);
        line.update(0.1);
        assert_eq!(line.cur_value, 1.0);
        assert!(line.complete);
    }

    #[test]
    fn slope_clamps_and_maps_nan_to_the_top() {
        let mut slope = Slope::default();
        slope.initialize(SlopeParams { min_input: 0.0, max_input: 2.0, min_output: 1.0, max_output: 3.0 });
        assert_eq!(slope.value(-1.0), 1.0);
        assert_eq!(slope.value(1.0), 2.0);
        assert_eq!(slope.value(5.0), 3.0);
        assert_eq!(slope.value(f32::NAN), 3.0);
    }
}
