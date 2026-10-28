//! `Snd9::InterpretModule` 0x82c19c20: one pass over a module instance's bytecode.
//!
//! Each node is `{u8 op, u8 numRoutes, pad, routes {s32 src, s32 dst}[], s32 advance}`; the op
//! runs on the state at the current data offset and returns a value, each route copies that value
//! (`src == -1`) or another word of the current state to `dst`, and the data offset moves on by
//! `advance`. Ops 0-5 and 37-39 go through `sndaemsfuncs`; the rest are inlined in the switch.
//! "Round" below is the interpreter's `fctiwz(x +/- 0.5)` on single precision.

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::float_cmp)]

use super::{rd16, rd32, rd8, rdf, snd_sin, wr16, wr32, wr8, wrf, Aems, INSTANCE_HEADER};
use crate::csis::Handle;
use crate::rwac::AudioCore;

/// `fctiwz`: truncation, saturating, NaN to `i32::MIN`.
fn fctiwz(value: f32) -> i32 {
    if value.is_nan() {
        i32::MIN
    } else {
        value as i32
    }
}

/// The interpreter's rounding: half away from zero (`f29` = 0.5).
fn round(value: f32) -> i32 {
    if value < 0.0 {
        fctiwz(value - 0.5)
    } else {
        fctiwz(value + 0.5)
    }
}

/// `(f32)(s32)x` (`extsw`, `fcfid`, `frsp`).
fn float(x: i32) -> f32 {
    f64::from(x) as f32
}

impl Aems {
    fn data(&self, slot: usize) -> &[u8] {
        self.instances[slot].as_ref().map_or(&[], |i| i.data.as_slice())
    }

    fn data_mut(&mut self, slot: usize) -> &mut [u8] {
        self.instances[slot].as_mut().map_or(&mut [], |i| i.data.as_mut_slice())
    }

    /// The bytes the bank's static-data pointers (kept as bank offsets) refer to.
    fn bank_bytes(&self, slot: usize) -> &[u8] {
        self.instances[slot].as_ref().and_then(|i| self.banks.get(i.bank)?.as_ref()).map_or(&[], |b| b.resident.as_slice())
    }

    /// One `InterpretModule` call for `slot`. A triggered DESTROY (always the module's last node)
    /// frees the instance and ends the pass.
    pub(super) fn interpret(&mut self, core: &mut AudioCore, slot: usize) {
        let Some(instance) = self.instances[slot].as_ref() else { return };
        let Some(bank) = self.banks.get(instance.bank).and_then(Option::as_ref) else { return };
        let Some(module) = bank.modules.get(instance.module) else { return };
        let code: Vec<u8> = bank.resident.get(module.pcode..).map(<[u8]>::to_vec).unwrap_or_default();
        let mut pc = 0usize;
        let mut at = INSTANCE_HEADER;
        let trace = std::env::var_os("SKATE2_TRACE_AEMS_OPS").is_some();
        if trace {
            eprintln!("aems {slot} pass {}#{}", bank.name, module.id);
        }
        let mut node = 0usize;
        loop {
            let op = rd8(&code, pc);
            if op == 0xFF {
                return;
            }
            let routes = usize::from(rd8(&code, pc + 1));
            let (value, destroyed) = self.execute(core, slot, op, at);
            if trace {
                eprintln!("aems {slot} node {node} @{at:#x} op {op} -> {value}");
            }
            node += 1;
            if destroyed {
                return;
            }
            let data = self.data_mut(slot);
            for route in 0..routes {
                let src = rd32(&code, pc + 4 + 8 * route);
                let dst = rd32(&code, pc + 8 + 8 * route);
                let word = if src == -1 { value } else { rd32(data, at.wrapping_add(src as usize)) };
                wr32(data, at.wrapping_add(dst as usize), word);
            }
            let advance = rd32(&code, pc + 4 + 8 * routes);
            pc += 8 + 8 * routes;
            at = at.wrapping_add(advance as usize);
        }
    }

    /// Runs node `op` on the state at `at`; returns its value and whether it destroyed the
    /// instance.
    fn execute(&mut self, core: &mut AudioCore, slot: usize, op: u8, at: usize) -> (i32, bool) {
        let period = self.timer.period;
        let value = match op {
            // SNDAEMSI_UpdateClassDestructor 0x82c174c8.
            0 => {
                let data = self.data_mut(slot);
                let value = rd32(data, at + 0x10);
                wr32(data, at + 0x10, 0);
                value
            }
            // ClassData (`lwz r3, 0x14(r3)`), GlobalVariable (`lwz r3, 0x18(r3)`).
            1 => rd32(self.data(slot), at + 0x14),
            2 => rd32(self.data(slot), at + 0x18),
            // SNDAEMSI_updatecreate 0x82c174e0.
            3 => {
                let data = self.data_mut(slot);
                let value = rd32(data, at);
                wr32(data, at, 0);
                value
            }
            // SNDAEMSI_updatedestroy 0x82c174f8.
            4 => {
                if rd32(self.data(slot), at + 0xC) != 0 {
                    self.destroy_instance(core, slot);
                    return (0, true);
                }
                0
            }
            5 => self.call_function_node(slot, at),
            6 => counter(self.data_mut(slot), at),
            7 => {
                if rd32(self.data(slot), at + 0xC) == 0 {
                    rd32(self.data(slot), at + 8)
                } else {
                    let random = self.random.next_u32();
                    let data = self.data_mut(slot);
                    let range = rd32(data, at + 4) as u32;
                    let value = match random.checked_rem(range) {
                        Some(r) => r.wrapping_add(rd32(data, at) as u32) as i32,
                        None => rd32(data, at + 8),
                    };
                    wr32(data, at + 8, value);
                    value
                }
            }
            8 => self.random_shuffle(slot, at),
            9 => self.random_weighted(slot, at),
            10 => range_trigger(self.data_mut(slot), at),
            11 => delay_trigger(self.data_mut(slot), at, period),
            12 => state_generator(self.data_mut(slot), at),
            13 => {
                let data = self.data(slot);
                let count = usize::from(rd8(data, at));
                i32::from((0..count).any(|i| rd32(data, at + 4 + 4 * i) != 0))
            }
            14 => envelope(self.data_mut(slot), at, period),
            15 => self.table(slot, at),
            16 => delay_line(self.data_mut(slot), at, period),
            17 => {
                let data = self.data(slot);
                let control = rd32(data, at + 4);
                if control <= 0 || control > i32::from(rd8(data, at)) {
                    0
                } else {
                    rd32(data, at + 4 * (control as usize + 1))
                }
            }
            18 => demux(self.data_mut(slot), at),
            19 | 20 => {
                let data = self.data(slot);
                let count = rd8(data, at);
                let mut value = rd32(data, at + 4);
                if count > 1 {
                    for i in 1..usize::from(count) {
                        let input = rd32(data, at + 4 + 4 * i);
                        if (op == 19 && input < value) || (op == 20 && input > value) {
                            value = input;
                        }
                    }
                }
                value
            }
            21 => {
                let data = self.data(slot);
                let count = rd8(data, at);
                let mut product = float(rd32(data, at + 8));
                if count > 1 {
                    for i in 1..usize::from(count) {
                        product = float(rd32(data, at + 8 + 4 * i)) * product;
                    }
                }
                round(rdf(data, at + 4) * product)
            }
            22 => {
                let data = self.data(slot);
                let count = rd8(data, at);
                let mut sum = rd32(data, at + 4);
                if count > 1 {
                    for i in 1..usize::from(count) {
                        sum = rd32(data, at + 4 + 4 * i).wrapping_add(sum);
                    }
                }
                sum
            }
            23 => {
                let data = self.data(slot);
                rd32(data, at).wrapping_sub(rd32(data, at + 4))
            }
            24 => {
                let data = self.data(slot);
                rd32(data, at + 4).wrapping_mul(rd32(data, at))
            }
            25 | 26 => {
                let data = self.data(slot);
                let (a, b) = (rd32(data, at), rd32(data, at + 4));
                if b == 0 {
                    0
                } else if op == 25 {
                    a.wrapping_div(b)
                } else {
                    a.wrapping_sub(a.wrapping_div(b).wrapping_mul(b))
                }
            }
            27 => self.player_node(core, slot, at),
            28 => oscillator(self.data_mut(slot), at, period),
            29 => ramp(self.data_mut(slot), at, period),
            30 => {
                let data = self.data(slot);
                let count = rd8(data, at);
                let mut sum = rd32(data, at + 8);
                if count > 1 {
                    for i in 1..usize::from(count) {
                        sum = sum.wrapping_add(rd32(data, at + 8 + 4 * i));
                    }
                }
                let max = rd32(data, at + 4);
                if sum > max {
                    max
                } else {
                    sum
                }
            }
            31 => {
                let data = self.data(slot);
                let difference = rd32(data, at + 4).wrapping_sub(rd32(data, at + 8));
                let min = rd32(data, at);
                if difference >= min {
                    difference
                } else {
                    min
                }
            }
            32 => {
                let data = self.data(slot);
                let product = rd32(data, at + 8).wrapping_mul(rd32(data, at + 4));
                let max = rd32(data, at);
                if product <= max {
                    product
                } else {
                    max
                }
            }
            33 | 34 => {
                let data = self.data(slot);
                let (a, b) = (rd32(data, at), rd32(data, at + 4));
                if (op == 33 && a < b) || (op == 34 && a > b) {
                    a
                } else {
                    b
                }
            }
            35 => {
                let data = self.data(slot);
                round((float(rd32(data, at + 8)) * float(rd32(data, at + 4))) * rdf(data, at))
            }
            36 => {
                let data = self.data(slot);
                rd32(data, at + 4).wrapping_add(rd32(data, at))
            }
            // UpdateFunction 0x82c18b60.
            37 => {
                let data = self.data_mut(slot);
                let value = i32::from(rd8(data, at + 0x19));
                wr8(data, at + 0x19, 0);
                value
            }
            38 => self.control_class_node(slot, at),
            39 => self.set_global_node(slot, at),
            _ => 0,
        };
        (value, false)
    }

    /// `UpdateCallFunction` 0x82c17748: optionally clamp the parameters to their
    /// `ParameterClampDesc`s, and call the function while `triggered` is set.
    fn call_function_node(&mut self, slot: usize, at: usize) -> i32 {
        let data = self.data_mut(slot);
        let clamp = rd8(data, at + 8) != 0;
        let count = usize::from(rd8(data, at + 9));
        let inputs = if clamp { at + 0xC + 8 * count } else { at + 0xC };
        if clamp {
            for i in 0..count {
                let (min, max) = (rd32(data, at + 0xC + 8 * i), rd32(data, at + 0x10 + 8 * i));
                let mut value = rd32(data, inputs + 4 + 4 * i);
                if value < min {
                    value = min;
                } else if value > max {
                    value = max;
                }
                wr32(data, inputs + 4 + 4 * i, value);
            }
        }
        if rd32(data, inputs) != 0 {
            let mut handle = Handle { private: rd32(data, at) as u32, key: rd32(data, at + 4) };
            let parameters: Vec<i32> = (0..count).map(|i| rd32(data, inputs + 4 + 4 * i)).collect();
            self.function_call(&mut handle, &parameters);
            let data = self.data_mut(slot);
            wr32(data, at, handle.private as i32);
            wr32(data, at + 4, handle.key);
        }
        0
    }

    /// `UpdateControlClass` 0x82c177f8 (no shipped bank uses it): destroy releases the class;
    /// create instantiates it once; otherwise the parameters update it. Returns its refcount.
    fn control_class_node(&mut self, slot: usize, at: usize) -> i32 {
        let data = self.data_mut(slot);
        let clamp = rd8(data, at + 0xC) != 0;
        let count = usize::from(rd8(data, at + 0xD));
        let inputs = if clamp { at + 0x10 + 8 * count } else { at + 0x10 };
        let clamp_parameters = |data: &mut [u8]| {
            if clamp {
                for i in 0..count {
                    let (min, max) = (rd32(data, at + 0x10 + 8 * i), rd32(data, at + 0x14 + 8 * i));
                    let mut value = rd32(data, inputs + 8 + 4 * i);
                    if value < min {
                        value = min;
                    } else if value > max {
                        value = max;
                    }
                    wr32(data, inputs + 8 + 4 * i, value);
                }
            }
        };
        let class = rd32(data, at + 8) as u32;
        if rd32(data, inputs + 4) != 0 {
            if class != 0 {
                self.class_release(class);
                wr32(self.data_mut(slot), at + 8, 0);
            }
        } else if rd32(data, inputs) != 0 {
            if class == 0 {
                clamp_parameters(data);
                let parameters: Vec<i32> = (0..count).map(|i| rd32(data, inputs + 8 + 4 * i)).collect();
                let mut handle = Handle { private: rd32(data, at) as u32, key: rd32(data, at + 4) };
                let (result, created) = self.class_create(&mut handle, &parameters);
                let data = self.data_mut(slot);
                wr32(data, at, handle.private as i32);
                wr32(data, at + 4, handle.key);
                wr32(data, at + 8, if result < 0 { 0 } else { created as i32 });
            }
        } else if class != 0 {
            clamp_parameters(data);
            let parameters: Vec<i32> = (0..count).map(|i| rd32(data, inputs + 8 + 4 * i)).collect();
            self.send_parameters(class, &parameters);
        }
        let class = rd32(self.data(slot), at + 8) as u32;
        self.csis.class(class).map_or(0, |c| c.ref_count)
    }

    /// `UpdateSetGlobalVariable` 0x82c179a0: a changed input, clamped, sets the global.
    fn set_global_node(&mut self, slot: usize, at: usize) -> i32 {
        let data = self.data_mut(slot);
        let value = rd32(data, at + 0x14);
        if rd32(data, at + 0x10) == value {
            return 0;
        }
        wr32(data, at + 0x10, value);
        let (min, max) = (rd32(data, at + 8), rd32(data, at + 0xC));
        let clamped = if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        };
        let mut handle = Handle { private: rd32(data, at) as u32, key: rd32(data, at + 4) };
        self.global_set(&mut handle, clamped);
        let data = self.data_mut(slot);
        wr32(data, at, handle.private as i32);
        wr32(data, at + 4, handle.key);
        0
    }

    /// RANDOMSHUFFLE: a shuffle bag of `range` entries from `numbersetindex` on; a pass boundary
    /// sets `avoidrepeat` so the last pick is not drawn first.
    fn random_shuffle(&mut self, slot: usize, at: usize) -> i32 {
        let input = usize::from(rd16(self.data(slot), at));
        if rd32(self.data(slot), at + input) == 0 {
            return rd32(self.data(slot), at + 0xC);
        }
        let random = self.random.next_u32();
        let data = self.data_mut(slot);
        let index = u32::from(rd16(data, at + 8));
        let avoid = rd8(data, at + 3) as i8 as i32 as u32;
        let range = u32::from(rd16(data, at + 0xA));
        let span = range.wrapping_sub(avoid).wrapping_sub(index);
        let Some(offset) = random.checked_rem(span) else { return rd32(data, at + 0xC) };
        let pick = index.wrapping_add(offset) as usize;
        let index = index as usize;
        if rd8(data, at + 2) == 1 {
            let chosen = rd8(data, at + 0x10 + pick);
            wr32(data, at + 0xC, i32::from(chosen));
            let swapped = rd8(data, at + 0x10 + index);
            wr8(data, at + 0x10 + pick, swapped);
            wr8(data, at + 0x10 + index, chosen);
        } else {
            let chosen = rd16(data, at + 0x10 + 2 * pick);
            wr32(data, at + 0xC, i32::from(chosen));
            let swapped = rd16(data, at + 0x10 + 2 * index);
            wr16(data, at + 0x10 + 2 * pick, swapped);
            wr16(data, at + 0x10 + 2 * index, chosen);
        }
        let next = rd16(data, at + 8).wrapping_add(1);
        let value = rd32(data, at + 4).wrapping_add(rd32(data, at + 0xC));
        wr16(data, at + 8, next);
        wr32(data, at + 0xC, value);
        if next < rd16(data, at + 0xA) {
            wr8(data, at + 3, 0);
        } else {
            wr16(data, at + 8, 0);
            wr8(data, at + 3, 1);
        }
        value
    }

    /// RANDOMWEIGHTED: `rand() % 100` against the running sum of the table's signed 8-bit weights.
    fn random_weighted(&mut self, slot: usize, at: usize) -> i32 {
        if rd32(self.data(slot), at + 0x10) == 0 {
            return rd32(self.data(slot), at + 0xC);
        }
        let draw = self.random.next_u32() % 100;
        let table = rd32(self.data(slot), at) as usize;
        let range = rd32(self.data(slot), at + 8);
        let mut sum = 0u32;
        let mut chosen = None;
        for i in 0..range.max(0) as usize {
            sum = sum.wrapping_add(rd8(self.bank_bytes(slot), table + 0x10 + i) as i8 as i32 as u32);
            if sum > draw {
                chosen = Some(i as i32);
                break;
            }
        }
        let data = self.data_mut(slot);
        if let Some(i) = chosen {
            wr32(data, at + 0xC, rd32(data, at + 4).wrapping_add(i));
        }
        rd32(data, at + 0xC)
    }

    /// TABLE: a changed input, clamped to the table's range, reads an entry directly (resolution
    /// 1) or interpolates between `round(x * resolution - 0.5)` and the next entry.
    fn table(&mut self, slot: usize, at: usize) -> i32 {
        let data = self.data(slot);
        let input = rd32(data, at + 0xC);
        if input == rd32(data, at + 4) {
            return rd32(data, at + 8);
        }
        let table = rd32(data, at) as usize;
        let bank = self.bank_bytes(slot);
        let (min, max) = (rd32(bank, table + 4), rd32(bank, table + 8));
        let x = if input < min {
            min
        } else if input > max {
            max
        } else {
            input
        }
        .wrapping_sub(min);
        let resolution = rdf(bank, table + 0xC);
        let size = rd8(bank, table);
        let entry = |i: i32| -> i32 {
            let base = table as isize + 0x10;
            match size {
                2 => rd16(bank, (base + 2 * i as isize) as usize) as i16 as i32,
                1 => rd8(bank, (base + i as isize) as usize) as i8 as i32,
                _ => rd32(bank, (base + 4 * i as isize) as usize),
            }
        };
        let value = if resolution == 1.0 {
            entry(x)
        } else {
            let position = float(x) * resolution;
            let index = round(position - 0.5);
            let fraction = position - float(index);
            let mut next = index.wrapping_add(1);
            let count = i32::from(rd16(bank, table + 2));
            if !(next < count) {
                next = count - 1;
            }
            let (a, b) = (float(entry(index)), float(entry(next)));
            round((b - a).mul_add(fraction, a))
        };
        let data = self.data_mut(slot);
        wr32(data, at + 4, input);
        wr32(data, at + 8, value);
        value
    }
}

/// COUNTER: an override inside [min, max] wins; a trigger steps by the signed direction and
/// wraps.
fn counter(data: &mut [u8], at: usize) -> i32 {
    let override_value = rd32(data, at + 0x14);
    let (min, max) = (rd32(data, at), rd32(data, at + 4));
    if !(override_value < min) && override_value <= max {
        return override_value;
    }
    let value = rd32(data, at + 8);
    if rd32(data, at + 0x10) <= 0 {
        return value;
    }
    let value = (rd8(data, at + 0xC) as i8 as i32).wrapping_add(value);
    wr32(data, at + 8, value);
    if value > max {
        wr32(data, at + 8, min);
        return min;
    }
    if value < min {
        wr32(data, at + 8, max);
    }
    rd32(data, at + 8)
}

/// RANGETRIG: one pulse on entering the trip range, re-armed inside the reset range.
fn range_trigger(data: &mut [u8], at: usize) -> i32 {
    let input = rd32(data, at + 0x14);
    if input >= rd32(data, at) && input <= rd32(data, at + 4) {
        if rd8(data, at + 0x10) == 0 {
            wr8(data, at + 0x10, 1);
            wr8(data, at + 0x11, 1);
            return 1;
        }
    } else if input >= rd32(data, at + 8) && input <= rd32(data, at + 0xC) {
        wr8(data, at + 0x10, 0);
    }
    wr8(data, at + 0x11, 0);
    0
}

/// DELAYTRIG: a trigger restarts the clock; one pulse when it reaches the delay (ms).
fn delay_trigger(data: &mut [u8], at: usize, period: f32) -> i32 {
    if rd32(data, at + 8) != 0 {
        wrf(data, at, 0.0);
    } else if rdf(data, at) < 0.0 {
        wr8(data, at + 4, 0);
        return 0;
    }
    let time = rdf(data, at);
    if !(time < float(rd32(data, at + 0xC))) {
        wrf(data, at, -1.0);
        wr8(data, at + 4, 1);
        return 1;
    }
    wrf(data, at, time + period);
    wr8(data, at + 4, 0);
    0
}

/// STATEGEN: the value of the first set trigger becomes the output.
fn state_generator(data: &mut [u8], at: usize) -> i32 {
    let inputs = at + usize::from(rd16(data, at));
    for i in 0..usize::from(rd8(data, at + 2)) {
        if rd32(data, inputs + 4 * i) != 0 {
            let value = rd32(data, at + 8 + 4 * i);
            wr32(data, at + 4, value);
            return value;
        }
    }
    rd32(data, at + 4)
}

/// ENVELOPE: control 1 starts (from `initialvalue`), 3 jumps to the release segment, 2 holds;
/// a finished last segment, or any other control, leaves 0.
fn envelope(data: &mut [u8], at: usize, period: f32) -> i32 {
    let input = at + usize::from(rd16(data, at));
    let control = rd32(data, input);
    let previous = rd8(data, at + 2);
    let set_segment = |data: &mut [u8], segment: usize, from: f32| {
        let duration = rdf(data, at + 0x18 + 8 * segment);
        wrf(data, at + 4, duration);
        let delta = (rdf(data, at + 0x1C + 8 * segment) - from) / duration;
        wrf(data, at + 8, delta);
        wrf(data, at + 8, delta * period);
    };
    let release = rd16(data, at + 0x12) as i16;
    if control == 1 && previous == 0 {
        wr8(data, at + 3, 0);
        let initial = rdf(data, at + 0x14);
        wrf(data, at + 0xC, initial);
        set_segment(data, 0, initial);
    } else if control == 3 && previous != 3 && i32::from(rd8(data, at + 3)) < i32::from(release) {
        let segment = release as u8;
        wr8(data, at + 3, segment);
        let output = rdf(data, at + 0xC);
        set_segment(data, usize::from(segment), output);
    } else if (control == 1 || control == 3) && rd8(data, at + 3) < rd8(data, at + 0x10) {
        let remaining = rdf(data, at + 4) - period;
        wrf(data, at + 4, remaining);
        if remaining > 0.0 {
            let output = rdf(data, at + 8) + rdf(data, at + 0xC);
            wrf(data, at + 0xC, output);
        } else {
            let segment = usize::from(rd8(data, at + 3));
            let target = rdf(data, at + 0x1C + 8 * segment);
            let next = rd8(data, at + 3).wrapping_add(1);
            wr8(data, at + 3, next);
            wrf(data, at + 0xC, target);
            if next < rd8(data, at + 0x10) {
                set_segment(data, usize::from(next), target);
            } else {
                wrf(data, at + 0xC, 0.0);
            }
        }
    } else if control != 2 {
        wrf(data, at + 0xC, 0.0);
    }
    wr8(data, at + 2, rd32(data, input) as u8);
    round(rdf(data, at + 0xC))
}

/// DELAYLINE: a changed delay (ms, negative written back as 0) moves the write slot that many
/// timer periods ahead of the read slot; each pass writes the input and reads the delayed value.
fn delay_line(data: &mut [u8], at: usize, period: f32) -> i32 {
    let inputs = at + usize::from(rd16(data, at));
    let delay = rd32(data, inputs + 4);
    if delay != rd32(data, at + 8) {
        wr32(data, at + 8, delay);
        if delay < 0 {
            wr32(data, inputs + 4, 0);
        }
        let slots = i32::from(rd16(data, at + 2));
        let mut steps = fctiwz(float(rd32(data, inputs + 4)) / period + 0.5);
        if !(steps < slots) {
            steps = slots - 1;
        }
        wr16(data, at + 4, (i32::from(rd16(data, at + 6)) + steps) as u16);
    }
    let slots = rd16(data, at + 2);
    let write = rd16(data, at + 4);
    if !(write < slots) {
        wr16(data, at + 4, write.wrapping_sub(slots));
    }
    if !(rd16(data, at + 6) < slots) {
        wr16(data, at + 6, 0);
    }
    let write = usize::from(rd16(data, at + 4));
    let value = rd32(data, inputs);
    wr32(data, at + 0xC + 4 * write, value);
    let read = rd16(data, at + 6);
    let output = rd32(data, at + 0xC + 4 * usize::from(read));
    wr16(data, at + 4, rd16(data, at + 4).wrapping_add(1));
    wr16(data, at + 6, read.wrapping_add(1));
    output
}

/// `SNDAEMSI_updatedemux` 0x82c183a8: the previous output slot clears, the selected one takes
/// the value; returns the first output.
fn demux(data: &mut [u8], at: usize) -> i32 {
    let previous = rd16(data, at + 2) as i16 as isize;
    wr32(data, (at as isize + 4 * (previous + 2)) as usize, 0);
    let control = rd32(data, at + 4);
    if control > 0 && control <= i32::from(rd8(data, at)) {
        let value = rd32(data, at + 8);
        wr32(data, at + 4 * (control as usize + 2), value);
        wr16(data, at + 2, control as u16);
    }
    rd32(data, at + 0xC)
}

/// OSCILLATOR: sine (`iSNDsin` of the phase in 1024ths, times amplitude / 65536), square, saw or
/// triangle of the phase, then the phase advances by the timer period over the period (ms).
fn oscillator(data: &mut [u8], at: usize, period: f32) -> i32 {
    let cycle = rd32(data, at + 8);
    if cycle <= 0 {
        return 0;
    }
    let amplitude = float(rd32(data, at + 0xC));
    let step = period / float(cycle);
    while !(rdf(data, at + 4) < 1.0) {
        wrf(data, at + 4, rdf(data, at + 4) - 1.0);
    }
    let phase = rdf(data, at + 4);
    let value = match rd8(data, at) {
        0 => (float(snd_sin(round(phase * 1024.0))) * amplitude) * f32::from_bits(0x3780_0000),
        1 => {
            if phase < 0.5 {
                0.0
            } else {
                amplitude
            }
        }
        2 => phase * amplitude,
        _ => {
            if phase < 0.5 {
                (phase * amplitude) * 2.0
            } else {
                ((1.0 - phase) * amplitude) * 2.0
            }
        }
    };
    wrf(data, at + 4, rdf(data, at + 4) + step);
    round(value)
}

/// RAMP: a changed target or duration sets the step `(target - current) / duration * period /
/// 4096`; each pass moves `scale` steps and stops at the target.
fn ramp(data: &mut [u8], at: usize, period: f32) -> i32 {
    let target_value = rd32(data, at + 0x18);
    let target = float(target_value);
    let current = rdf(data, at);
    if target == current {
        return target_value;
    }
    if target_value != rd32(data, at + 8) || rd32(data, at + 0x10) != rd32(data, at + 0xC) {
        let duration = rd32(data, at + 0x10);
        wr32(data, at + 8, target_value);
        wr32(data, at + 0xC, duration);
        if duration <= 0 {
            wrf(data, at, target);
            return target_value;
        }
        let delta = target - current;
        wrf(data, at + 4, delta);
        wrf(data, at + 4, ((delta / float(duration)) * period) * f32::from_bits(0x3980_0000));
    }
    let delta = rdf(data, at + 4);
    let next = float(rd32(data, at + 0x14)).mul_add(delta, current);
    wrf(data, at, next);
    if (delta < 0.0 && !(next >= target)) || (!(delta < 0.0) && !(next <= target)) {
        wrf(data, at, target);
    }
    round(rdf(data, at))
}
