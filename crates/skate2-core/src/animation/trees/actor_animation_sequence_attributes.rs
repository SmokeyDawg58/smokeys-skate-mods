//! Sequence/Selector attribute virtuals. Child order and IDs are authored.
use super::attributes::{SettableTreeAttribute, TreeAttributeError as E, append_attribute};
use super::blend_space::update::BlendSpaceAttributeContext;
use super::sequence::FRAME;
use super::{AnimationNodeHandle as H, AnimationTreePool};
use crate::{
    play_animation::{FastString30, FastString36},
    simulation_animation_startup::AnimationAttributePool,
};
use skate2_formats::andale::{AndaleAttributeValue, AndaleRuntimeAttribute};

fn scalar(name: &str, value: f32) -> AndaleRuntimeAttribute {
    AndaleRuntimeAttribute {
        sequence_id: -1,
        begin_seconds: -1.0,
        end_seconds: -1.0,
        name: name.into(),
        status: 6,
        value: AndaleAttributeValue::Float(value),
    }
}
fn extended(name: FastString30) -> FastString36 {
    FastString36 {
        words: [
            name.words[0],
            name.words[1],
            name.words[2],
            name.words[3],
            name.words[4],
            0,
        ],
    }
}

impl AnimationTreePool {
    /// GetAttributes82d80400 ignores supplied filter, visits window wrapping once.
    pub(crate) fn sequence_attributes(
        &self,
        h: &H,
        scratch: &mut AnimationAttributePool,
        out: &mut Vec<AndaleRuntimeAttribute>,
    ) -> Result<(), E> {
        let selected = self.selected_child(h)?;
        let n = self.sequence(h)?;
        let start = n.selected.ok_or(E::Missing("Sequence selected"))?;
        let time = self.time(&selected)? - (FRAME / n.speed);
        append_attribute(
            out,
            scalar("SequenceStateRawTime", if time >= 0.0 { time } else { 0.0 }),
        )?;
        for i in (start..n.children.len()).chain(0..start).take(n.window) {
            let child = &n.children[i].animation;
            let length = (self.length(child)? * n.speed) - FRAME;
            append_attribute(out, scalar("SequenceStateLength", length))?;
            self.get_attributes(child, 15, scratch, out)?;
            // Native walks backward until sequenceID != -1. Exhaustion reaches
            // actual Attributes+18 mpCapacity (begin-8), a non--1 aligned pointer.
            for row in out.iter_mut().rev() {
                if row.sequence_id != -1 {
                    break;
                }
                row.sequence_id = i32::try_from(i).map_err(|_| E::Capacity)?;
            }
        }
        Ok(())
    }
    /// SetAttributes82d7ffc0 partitions explicit IDs0..11 and global-1; skip
    /// publishes immediately, then each window child gets globals before locals.
    pub(crate) fn set_sequence_attributes(
        &mut self,
        h: &H,
        input: &[SettableTreeAttribute],
        context: &mut Option<BlendSpaceAttributeContext<'_>>,
    ) -> Result<bool, E> {
        if self.sequence(h)?.children.is_empty() {
            return Err(E::Missing("Sequence children assertion"));
        }
        let mut local: [Vec<SettableTreeAttribute>; 13] = std::array::from_fn(|_| vec![]);
        let mut global = vec![];
        let skip = FastString30::encode("SequenceSkip").expect("native literal");
        for row in input {
            if !(-1..12).contains(&row.sequence_id) {
                return Err(E::Missing("Sequence ID -1..11 assertion"));
            }
            if row.name == skip {
                // -1 is allowed by the ID assertion, but is an invalid native
                // preceding-child write for SequenceSkip: retain that boundary.
                let i = usize::try_from(row.sequence_id)
                    .map_err(|_| E::Missing("SequenceSkip child before begin"))?;
                self.sequence_mut(h)?
                    .children
                    .get_mut(i)
                    .ok_or(E::Missing("SequenceSkip child"))?
                    .enabled = false;
            } else if row.sequence_id == -1 {
                global.push(row.clone());
            } else {
                local[usize::try_from(row.sequence_id).map_err(|_| E::Capacity)?].push(row.clone());
            }
        }
        let n = self.sequence(h)?;
        let start = n.selected.ok_or(E::Missing("Sequence selected"))?;
        let children = (start..n.children.len())
            .chain(0..start)
            .take(n.window)
            .map(|i| (i, n.children[i].animation.clone()))
            .collect::<Vec<_>>();
        let mut result = true;
        for (i, child) in children {
            if !global.is_empty() {
                let changed = self.set_attributes_with_blend_space(&child, &global, context)?;
                result &= changed;
            }
            let rows = local
                .get(i)
                .ok_or(E::Missing("Sequence attribute partition"))?;
            if !rows.is_empty() {
                let changed = self.set_attributes_with_blend_space(&child, rows, context)?;
                result &= changed;
            }
        }
        Ok(result)
    }
    /// Selector.SetAttributes82d7bb18: ordered input rows, child order, name
    /// existence rather than truth. Nonpositive/unordered switch preserves time.
    pub(crate) fn set_selector_attributes(
        &mut self,
        h: &H,
        input: &[SettableTreeAttribute],
        context: &mut Option<BlendSpaceAttributeContext<'_>>,
    ) -> Result<bool, E> {
        let n = self.selector(h)?;
        if n.children.is_empty() {
            return Err(E::Missing("Selector children assertion"));
        }
        if n.next.is_some() {
            return Err(E::Missing("Selector next must equal end"));
        }
        if n.transitioning {
            return Err(E::Missing("Selector transition child"));
        }
        for row in input {
            let name = extended(row.name);
            let count = self.selector(h)?.children.len();
            for i in 0..count {
                let n = self.selector(h)?;
                let child = n.children[i].0.clone();
                let selected = self.selected_child(h)?;
                if child == selected
                    || n.next.is_some_and(|j| n.children[j].0 == child)
                    || n.children[i].1 != name
                {
                    continue;
                }
                self.selector_mut(h)?.transition_length = row.value;
                if row.value > 0.0 {
                    let n = self.selector_mut(h)?;
                    n.transition_time = 0.0;
                    n.next = Some(i);
                    n.transitioning = true;
                    return self.set_attributes_with_blend_space(&child, input, context);
                }
                let time = self.time(&selected)?;
                self.set_time(&child, time)?;
                self.selector_mut(h)?.selected = Some(i);
            }
        }
        if self.selector(h)?.next.is_some() {
            return Err(E::Missing("Selector next must equal end"));
        }
        let child = self.selected_child(h)?;
        self.set_attributes_with_blend_space(&child, input, context)
    }
}
