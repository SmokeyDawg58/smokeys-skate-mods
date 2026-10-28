//! Writable MixMap inputs (doc 16 "L2"; `sdk.audio.set_mixmap_input`, capability `audio` = 4): mods
//! (and engine systems, [`MixMapInputs::set`]) drive retail's own MixMap controllers — the Master
//! category gains, the NIS / menu / speech duck flags, any instance's inputs — instead of only a
//! multiplier on a volume group, so a duck goes through retail's curves, envelopes and ducks.
//!
//! - A write names a controller (`slot`, `object`, `instance`; the same keys as the read-only
//!   watch) and one of its 16 inputs, with an integer word (or an f32 for the distance inputs:
//!   `float`). Checked when it is set: the slot name, the ranges, the controller must exist in the
//!   game's MixMap (an unknown one is a command error).
//! - Owned: the first mod to write an input owns it; 16 inputs per mod, 64 in all.
//! - Applied every pass right before the MixMap's evaluations (`native::mixmap_tick`), after the
//!   host's own writes, so a mod value holds against an input the host writes every pass (the
//!   Master gains) as well as one it never writes (the duck flags).
//! - Released by `nil`, the owner stopping / failing / reloading: the input gets back the value it
//!   had before the first write (an input the host writes takes the host's value again at the next
//!   pass anyway). Writes are kept across map changes; after a runtime restart (a new MixMap) the
//!   values are written again over the new MixMap's.
//! - With no writes nothing runs (one branch in the pass): the MixMap evaluates exactly as before.
use std::collections::BTreeMap;

use bevy::prelude::*;
use serde_json::{Value, json};
use skate_audio::mixmap::MixMap;

pub(crate) const MAX_PER_MOD: usize = 16;
pub(crate) const MAX_INPUTS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum InputValue {
    Word(i32),
    Float(f32),
}

#[derive(Clone, Debug)]
struct Write {
    owner: String,
    name: (String, u32, u32),
    value: InputValue,
    /// The input's value before the first write, in the runtime it was taken in.
    original: Option<(u64, i32)>,
}

#[derive(Resource, Default)]
pub(crate) struct MixMapInputs {
    writes: BTreeMap<(u32, usize), Write>,
    /// Released inputs to put back at the next pass: (key, input, value, runtime).
    restore: Vec<(u32, usize, i32, u64)>,
}

impl MixMapInputs {
    /// Write (`Some`) or release (`None`) `owner`'s value of one MixMap input.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set(
        &mut self,
        mixmap: Option<&MixMap>,
        owner: &str,
        slot: &str,
        object: u32,
        instance: u32,
        input: u32,
        value: Option<InputValue>,
    ) -> Result<(), String> {
        let s = super::mod_audio::slot_number(slot)
            .ok_or_else(|| format!("mixmap input: no MixMap slot {slot}"))?;
        if object > 127 || instance > 31 || input > 15 {
            return Err("mixmap input: object 0..127, instance 0..31, input 0..15".into());
        }
        let key = skate_audio::mixmap::keys::obj(s, object, instance);
        let id = (key, input as usize);
        if let Some(w) = self.writes.get(&id).filter(|w| w.owner != owner) {
            return Err(format!(
                "mixmap input {slot}/{object}/{instance}/{input} is owned by another mod ({})",
                w.owner
            ));
        }
        let Some(value) = value else {
            if let Some(w) = self.writes.remove(&id) {
                if let Some((runtime, original)) = w.original {
                    self.restore.push((key, input as usize, original, runtime));
                }
            }
            return Ok(());
        };
        match value {
            InputValue::Float(f) if !f.is_finite() => {
                return Err("mixmap input: a finite value".into());
            }
            _ => {}
        }
        let m = mixmap.ok_or("native audio is not running")?;
        if !m.has_controller(key) {
            return Err(format!(
                "mixmap input: the MixMap has no controller {slot}/{object}/{instance}"
            ));
        }
        if !self.writes.contains_key(&id) {
            if self.writes.values().filter(|w| w.owner == owner).count() >= MAX_PER_MOD {
                return Err(format!(
                    "mixmap input: {MAX_PER_MOD} inputs per mod maximum"
                ));
            }
            if self.writes.len() >= MAX_INPUTS {
                return Err(format!("mixmap input: {MAX_INPUTS} inputs in all maximum"));
            }
        }
        let original = self.writes.get(&id).and_then(|w| w.original);
        self.writes.insert(
            id,
            Write {
                owner: owner.to_owned(),
                name: (slot.to_owned(), object, instance),
                value,
                original,
            },
        );
        Ok(())
    }

    /// Every input `owner` writes is released.
    pub(crate) fn clear_owner(&mut self, owner: &str) {
        let ids: Vec<_> = self
            .writes
            .iter()
            .filter(|(_, w)| w.owner == owner)
            .map(|(k, _)| *k)
            .collect();
        for id in ids {
            if let Some(w) = self.writes.remove(&id) {
                if let Some((runtime, original)) = w.original {
                    self.restore.push((id.0, id.1, original, runtime));
                }
            }
        }
    }

    /// The pass, before the evaluations (`native::mixmap_tick`): released inputs back, then every
    /// write (the original taken first in this runtime).
    pub(crate) fn apply(&mut self, m: &mut MixMap, runtime: u64) {
        if self.writes.is_empty() && self.restore.is_empty() {
            return;
        }
        for (key, input, value, at) in self.restore.drain(..) {
            if at == runtime {
                m.set_input(key, input, value);
            }
        }
        for (&(key, input), w) in &mut self.writes {
            if w.original.is_none_or(|(at, _)| at != runtime) {
                w.original = Some((runtime, m.input(key, input)));
            }
            match w.value {
                InputValue::Word(v) => m.set_input(key, input, v),
                InputValue::Float(f) => m.set_input_f32(key, input, f),
            }
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.writes.len()
    }

    /// One mod's writes for its snapshot (`audio.inputs`).
    pub(crate) fn snapshot(&self, owner: &str) -> Option<Value> {
        let rows: Vec<Value> = self
            .writes
            .iter()
            .filter(|(_, w)| w.owner == owner)
            .map(|((_, input), w)| {
                let value = match w.value {
                    InputValue::Word(v) => json!(v),
                    InputValue::Float(f) => json!(f),
                };
                json!({"slot": w.name.0, "object": w.name.1, "instance": w.name.2, "input": input, "value": value})
            })
            .collect();
        (!rows.is_empty()).then(|| json!(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks, ownership, limits and the release (data-gated: the install's MixMap): a write holds
    /// against the host's own value of the input every pass, comes back on release, and drives the
    /// controller's outputs (the Master gains duck an emitter's level through retail's curves).
    #[test]
    #[ignore = "needs the private install data"]
    fn mixmap_inputs_drive_retail_controllers_and_release_exactly() {
        use skate_audio::mixmap::keys;
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
        let Ok(library) = super::super::Library::load(root) else {
            panic!("missing private data: no audio install")
        };
        let Ok(native) = super::super::Native::start(&library) else {
            panic!("missing private data: no AEMS install")
        };
        let mut m = native.mixmap.expect("a MixMap");
        let host = |m: &mut MixMap| {
            for id in 1..=4 {
                m.set_input(keys::MASTER, id, 32767);
            }
            for id in [1, 2, 5] {
                m.set_input(keys::MUSIC, id, 32767);
            }
            m.set_input(keys::REVERB, 5, 32767);
            m.set_input(keys::emitter_pos(0), keys::pos::DIST_CAMERA, 0);
            m.set_input_f32(keys::emitter_pos(0), keys::pos::DIST_CAMERA, 5.0);
            m.set_input_f32(keys::emitter_pos(0), keys::pos::DIST_SKATER, 5.0);
            m.set_input(keys::emitter_pos(0), keys::pos::FLAGS, 1);
        };
        let mut inputs = MixMapInputs::default();
        // Checks.
        assert!(
            inputs
                .set(
                    Some(&m),
                    "dev.a",
                    "nope",
                    2,
                    0,
                    1,
                    Some(InputValue::Word(0))
                )
                .is_err()
        );
        assert!(
            inputs
                .set(
                    Some(&m),
                    "dev.a",
                    "global",
                    2,
                    0,
                    16,
                    Some(InputValue::Word(0))
                )
                .is_err()
        );
        assert!(
            inputs
                .set(
                    Some(&m),
                    "dev.a",
                    "global",
                    120,
                    0,
                    1,
                    Some(InputValue::Word(0))
                )
                .is_err(),
            "no such controller"
        );
        assert!(
            inputs
                .set(None, "dev.a", "global", 2, 0, 1, Some(InputValue::Word(0)))
                .is_err()
        );
        assert!(
            inputs
                .set(
                    Some(&m),
                    "dev.a",
                    "global",
                    2,
                    0,
                    1,
                    Some(InputValue::Float(f32::NAN))
                )
                .is_err()
        );
        let level = |m: &mut MixMap, inputs: &mut MixMapInputs| {
            host(m);
            inputs.apply(m, 0);
            for _ in 0..30 {
                m.tick(skate_audio::mixmap::cadence::CONSOLE_DT);
            }
            m.level(keys::emitter(0), 4)
        };
        let retail = level(&mut m, &mut inputs);
        assert!(retail > 0, "an emitter at 5 m sounds");
        // The Master category gains to 0: retail's controllers duck the emitter.
        for id in 1..=4 {
            inputs
                .set(
                    Some(&m),
                    "dev.a",
                    "global",
                    2,
                    0,
                    id,
                    Some(InputValue::Word(0)),
                )
                .unwrap();
        }
        assert!(
            inputs
                .set(
                    Some(&m),
                    "dev.b",
                    "global",
                    2,
                    0,
                    1,
                    Some(InputValue::Word(5))
                )
                .is_err(),
            "first owner wins"
        );
        let ducked = level(&mut m, &mut inputs);
        assert!(ducked < retail, "ducked {ducked} < {retail}");
        assert_eq!(m.input(keys::MASTER, 1), 0, "held against the host's write");
        assert_eq!(
            inputs.snapshot("dev.a").unwrap().as_array().unwrap().len(),
            4
        );
        // Release: the inputs come back, and the level with them.
        inputs.clear_owner("dev.a");
        assert_eq!(inputs.count(), 0);
        let back = level(&mut m, &mut inputs);
        assert_eq!(back, retail, "released = retail");
        // Limits.
        for i in 0..MAX_PER_MOD as u32 {
            inputs
                .set(
                    Some(&m),
                    "dev.c",
                    "global",
                    13,
                    0,
                    i,
                    Some(InputValue::Word(1)),
                )
                .unwrap();
        }
        assert!(
            inputs
                .set(
                    Some(&m),
                    "dev.c",
                    "global",
                    6,
                    0,
                    0,
                    Some(InputValue::Word(1))
                )
                .is_err(),
            "16 per mod"
        );
    }
}
