//! Retained `TransitionBlendAnim82d7ae70` / `TransitionAnim82d7aa68` producer.
use super::{
    ActorAnimationConstruction, AnimationFixedStorage, AnimationPointerState,
    AnimationTransitionHandle, AnimationTransitionStore,
    play_anim::{BodyAnimationRoot, PlayBodyAnimationError as E},
};
use crate::play_animation::FastString36;

/// Native `TransitionAnim` type1/type2 and late flags31/30 publication.
#[derive(Clone, Copy)]
pub(crate) enum TransitionBlendKind {
    Ordinary,
    Channel { use_from_weights: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact retained clock and length after failed initialization"
    )]
    fn invalid_length_keeps_allocation_and_child_stores_but_old_clock_flags() {
        for length in [0.0, -1.0, f32::NAN] {
            let mut store = AnimationTransitionStore::new();
            let index = store.new_retained_blend().unwrap();
            assert_eq!(index, 2);
            store.blends[index].length = 0.75;
            store.blends[index].time = 0.25;
            let result = store.init_retained_blend(
                index,
                BodyAnimationReference::TransitionBlend(0),
                BodyAnimationReference::TransitionBlend(1),
                length,
                true,
            );
            assert!(result.is_err());
            assert_eq!(store.free_blend_count, 2);
            assert_eq!(
                store.transition_order.slots[0],
                Some(AnimationTransitionHandle::Blend(2))
            );
            assert_eq!(store.blend_order.slots[0], Some(2));
            let blend = &store.blends[index];
            assert_eq!(blend.length, 0.75);
            assert_eq!(blend.time, 0.25);
            assert_eq!(
                blend.runtime.as_ref().unwrap().to,
                BodyAnimationReference::TransitionBlend(0)
            );
            assert_eq!(blend.to_animation, AnimationPointerState::Constructed);
            assert_eq!(blend.runtime.as_ref().unwrap().flags, 0);
        }
    }

    #[test]
    fn channel_kind_replaces_only_high_flags_after_init_and_reused_regular_clears_them() {
        for current in [false, true] {
            for use_from_weights in [false, true] {
                let mut store = AnimationTransitionStore::new();
                let index = store.new_retained_blend().unwrap();
                store
                    .init_retained_blend(
                        index,
                        BodyAnimationReference::TransitionBlend(0),
                        BodyAnimationReference::TransitionBlend(1),
                        0.25,
                        current,
                    )
                    .unwrap();
                store
                    .set_retained_blend_kind(
                        index,
                        TransitionBlendKind::Channel { use_from_weights },
                    )
                    .unwrap();
                let expected =
                    0x8000_0000 | (u32::from(use_from_weights) << 30) | (u32::from(current) << 29);
                assert_eq!(
                    store.blends[index].runtime.as_ref().unwrap().flags,
                    expected
                );
                assert_eq!(
                    store.blends[index].flags_known_zero_mask,
                    0xe000_0000 & !expected
                );
                store
                    .set_retained_blend_kind(index, TransitionBlendKind::Ordinary)
                    .unwrap();
                assert_eq!(
                    store.blends[index].runtime.as_ref().unwrap().flags,
                    u32::from(current) << 29
                );
                assert_eq!(
                    store.blends[index].flags_known_zero_mask,
                    0xe000_0000 & !(u32::from(current) << 29)
                );
            }
        }
    }

    #[test]
    fn reused_init_preserves_lower_flags_and_known_zero_evidence() {
        let mut store = AnimationTransitionStore::new();
        let index = store.new_retained_blend().unwrap();
        store
            .init_retained_blend(
                index,
                BodyAnimationReference::TransitionBlend(0),
                BodyAnimationReference::TransitionBlend(1),
                0.25,
                false,
            )
            .unwrap();
        store.blends[index].runtime.as_mut().unwrap().flags = 0xc123_4567;
        store.blends[index].flags_known_zero_mask = 0x0008_0000;
        store
            .init_retained_blend(
                index,
                BodyAnimationReference::TransitionBlend(1),
                BodyAnimationReference::TransitionBlend(0),
                0.5,
                true,
            )
            .unwrap();
        assert_eq!(
            store.blends[index].runtime.as_ref().unwrap().flags,
            0x2123_4567
        );
        assert_eq!(store.blends[index].flags_known_zero_mask, 0xc008_0000);
        store
            .set_retained_blend_kind(
                index,
                TransitionBlendKind::Channel {
                    use_from_weights: true,
                },
            )
            .unwrap();
        assert_eq!(
            store.blends[index].runtime.as_ref().unwrap().flags,
            0xe123_4567
        );
        assert_eq!(store.blends[index].flags_known_zero_mask, 0x0008_0000);
    }

    #[test]
    fn release_preserves_payload_and_next_allocation_reuses_last_freed_slot() {
        let mut store = AnimationTransitionStore::new();
        let first = store.new_retained_blend().unwrap();
        let second = store.new_retained_blend().unwrap();
        store
            .init_retained_blend(
                first,
                BodyAnimationReference::TransitionBlend(0),
                BodyAnimationReference::TransitionBlend(1),
                0.3,
                true,
            )
            .unwrap();
        store.blends[first].evaluated = true;
        let before = store.blends[first].clone();
        store.release_retained_blend(first).unwrap();
        assert_eq!(store.blends[first], before);
        assert_eq!(store.transition_order.len, 1);
        assert_eq!(
            store.transition_order.slots[0],
            Some(AnimationTransitionHandle::Blend(second))
        );
        assert_eq!(store.new_retained_blend().unwrap(), first);
        assert_eq!(store.blends[first], before);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BodyAnimationReference {
    Tree(BodyAnimationRoot),
    TransitionBlend(usize),
    TransitionSequence(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TransitionBlendRuntime {
    pub(crate) to: BodyAnimationReference,
    pub(crate) from: BodyAnimationReference,
    /// Only high three native flag bits are known; lower 29 remain unknown.
    pub(crate) flags: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TransitionSequenceRuntime {
    pub(crate) to: BodyAnimationReference,
    pub(crate) from: BodyAnimationReference,
}

fn append<T: Copy, const N: usize>(
    list: &mut AnimationFixedStorage<T, N>,
    value: T,
) -> Result<(), E> {
    let slot = list.slots.get_mut(list.len).ok_or(E::InvalidStore)?;
    *slot = Some(value);
    list.len += 1;
    Ok(())
}
fn remove<T: Copy + PartialEq, const N: usize>(
    list: &mut AnimationFixedStorage<T, N>,
    value: T,
) -> Result<(), E> {
    let position = list.slots[..list.len]
        .iter()
        .position(|entry| *entry == Some(value))
        .ok_or(E::InvalidStore)?;
    for i in position..list.len - 1 {
        list.slots[i] = list.slots[i + 1];
    }
    list.len -= 1;
    // Unused list storage is not a native zero-write claim.
    Ok(())
}

impl AnimationTransitionStore {
    pub(super) fn retained_blend(&self, index: usize) -> Result<&TransitionBlendRuntime, E> {
        if !self.transition_order.slots[..self.transition_order.len]
            .contains(&Some(AnimationTransitionHandle::Blend(index)))
        {
            return Err(E::InvalidStore);
        }
        self.blends
            .get(index)
            .and_then(|blend| blend.runtime.as_ref())
            .ok_or(E::InvalidStore)
    }

    pub(super) fn retained_sequence(&self, index: usize) -> Result<&TransitionSequenceRuntime, E> {
        if !self.transition_order.slots[..self.transition_order.len]
            .contains(&Some(AnimationTransitionHandle::Sequence(index)))
        {
            return Err(E::InvalidStore);
        }
        self.sequence_runtime
            .get(index)
            .and_then(Option::as_ref)
            .ok_or(E::InvalidStore)
    }

    pub(super) fn transition_children(
        &self,
        handle: AnimationTransitionHandle,
    ) -> Result<TransitionSequenceRuntime, E> {
        let (to, from) = match handle {
            AnimationTransitionHandle::Blend(index) => {
                let runtime = self.retained_blend(index)?;
                (&runtime.to, &runtime.from)
            }
            AnimationTransitionHandle::Sequence(index) => {
                let runtime = self.retained_sequence(index)?;
                (&runtime.to, &runtime.from)
            }
        };
        Ok(TransitionSequenceRuntime {
            to: to.clone(),
            from: from.clone(),
        })
    }

    pub(super) fn replace_transition_child(
        &mut self,
        handle: AnimationTransitionHandle,
        from: bool,
        reference: BodyAnimationReference,
    ) -> Result<(), E> {
        let runtime = match handle {
            AnimationTransitionHandle::Blend(index) => {
                let runtime = self
                    .blends
                    .get_mut(index)
                    .ok_or(E::InvalidStore)?
                    .runtime
                    .as_mut()
                    .ok_or(E::InvalidStore)?;
                if from {
                    &mut runtime.from
                } else {
                    &mut runtime.to
                }
            }
            AnimationTransitionHandle::Sequence(index) => {
                let runtime = self
                    .sequence_runtime
                    .get_mut(index)
                    .ok_or(E::InvalidStore)?
                    .as_mut()
                    .ok_or(E::InvalidStore)?;
                if from {
                    &mut runtime.from
                } else {
                    &mut runtime.to
                }
            }
        };
        *runtime = reference;
        Ok(())
    }

    pub(super) fn new_retained_sequence(&mut self) -> Result<usize, E> {
        if self.transition_order.len == self.max_trees - 1 {
            return Err(E::InvalidStore);
        }
        let remaining = self
            .free_sequence_count
            .checked_sub(1)
            .ok_or(E::InvalidStore)?;
        let index = *self.free_sequences.get(remaining).ok_or(E::InvalidStore)?;
        self.free_sequence_count = remaining;
        append(
            &mut self.transition_order,
            AnimationTransitionHandle::Sequence(index),
        )?;
        Ok(index)
    }

    pub(super) fn init_retained_sequence(
        &mut self,
        index: usize,
        to: BodyAnimationReference,
        from: BodyAnimationReference,
    ) -> Result<(), E> {
        let sequence = self.sequences.get_mut(index).ok_or(E::InvalidStore)?;
        sequence.to_animation = AnimationPointerState::Constructed;
        sequence.from_animation = AnimationPointerState::Constructed;
        *self
            .sequence_runtime
            .get_mut(index)
            .ok_or(E::InvalidStore)? = Some(TransitionSequenceRuntime { to, from });
        sequence.playing_new_animation = false;
        Ok(())
    }

    fn release_retained_sequence(&mut self, index: usize) -> Result<(), E> {
        self.retained_sequence(index)?;
        if self.free_sequences[..self.free_sequence_count].contains(&index) {
            return Err(E::InvalidStore);
        }
        *self
            .free_sequences
            .get_mut(self.free_sequence_count)
            .ok_or(E::InvalidStore)? = index;
        self.free_sequence_count += 1;
        remove(
            &mut self.transition_order,
            AnimationTransitionHandle::Sequence(index),
        )
    }

    /// Pop free slot, append blend list, append all-transitions list; payload survives.
    fn new_retained_blend(&mut self) -> Result<usize, E> {
        if self.transition_order.len == self.max_trees - 1
            || self.blend_order.len == self.max_transition_blends
        {
            return Err(E::InvalidStore);
        }
        let remaining = self
            .free_blend_count
            .checked_sub(1)
            .ok_or(E::InvalidStore)?;
        let index = *self.free_blends.get(remaining).ok_or(E::InvalidStore)?;
        self.free_blend_count = remaining;
        append(&mut self.blend_order, index)?;
        append(
            &mut self.transition_order,
            AnimationTransitionHandle::Blend(index),
        )?;
        Ok(index)
    }

    fn init_retained_blend(
        &mut self,
        index: usize,
        to: BodyAnimationReference,
        from: BodyAnimationReference,
        length: f32,
        current_frame: bool,
    ) -> Result<(), E> {
        let blend = self.blends.get_mut(index).ok_or(E::InvalidStore)?;
        let flags = blend.runtime.as_ref().map_or(0, |runtime| runtime.flags);
        // 82d6b83c/840 precede the strict-positive assertion at b848.
        blend.to_animation = AnimationPointerState::Constructed;
        blend.from_animation = AnimationPointerState::Constructed;
        blend.runtime = Some(TransitionBlendRuntime { to, from, flags });
        if length.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(E::InvalidStore);
        }
        blend.length = length;
        blend.time = 0.0; // virtual ResetTransitionTime82d6df88
        let current_bit = if current_frame { 0x2000_0000 } else { 0 };
        blend.runtime.as_mut().ok_or(E::InvalidStore)?.flags = (flags & 0x1fff_ffff) | current_bit;
        blend.flags_known_zero_mask =
            (blend.flags_known_zero_mask & 0x1fff_ffff) | (0xe000_0000 & !current_bit);
        // TransitionAnim publishes type/use-from-weights bits only after this
        // successful Init, immediately before the new parent/root pointer.
        Ok(())
    }

    /// `TransitionAnim82d7ad78/ade4`, after successful Init, before parent/root publication.
    fn set_retained_blend_kind(
        &mut self,
        index: usize,
        kind: TransitionBlendKind,
    ) -> Result<(), E> {
        let blend = self.blends.get_mut(index).ok_or(E::InvalidStore)?;
        let runtime = blend.runtime.as_mut().ok_or(E::InvalidStore)?;
        let bits = match kind {
            TransitionBlendKind::Ordinary => 0,
            TransitionBlendKind::Channel { use_from_weights } => {
                0x8000_0000 | (u32::from(use_from_weights) << 30)
            }
        };
        runtime.flags = (runtime.flags & 0x3fff_ffff) | bits;
        blend.flags_known_zero_mask =
            (blend.flags_known_zero_mask & 0x3fff_ffff) | (0xc000_0000 & !bits);
        Ok(())
    }

    fn release_retained_blend(&mut self, index: usize) -> Result<(), E> {
        self.retained_blend(index)?;
        if self.free_blends[..self.free_blend_count].contains(&index) {
            return Err(E::InvalidStore);
        }
        *self
            .free_blends
            .get_mut(self.free_blend_count)
            .ok_or(E::InvalidStore)? = index;
        self.free_blend_count += 1;
        remove(&mut self.blend_order, index)?;
        remove(
            &mut self.transition_order,
            AnimationTransitionHandle::Blend(index),
        )
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Shared by capacity pruning and the completed-transition time consumer.
    pub(crate) fn try_force_transition_release(
        &mut self,
        handle: AnimationTransitionHandle,
    ) -> Result<(), E> {
        let store = &self
            .base
            .as_ref()
            .ok_or(E::MissingOwner("Animatable"))?
            .transition_store;
        let runtime = store.transition_children(handle)?;
        let target = match handle {
            AnimationTransitionHandle::Blend(index) => {
                BodyAnimationReference::TransitionBlend(index)
            }
            AnimationTransitionHandle::Sequence(index) => {
                BodyAnimationReference::TransitionSequence(index)
            }
        };
        let mut parent = None;
        for entry in &store.transition_order.slots[..store.transition_order.len] {
            let entry = entry.ok_or(E::InvalidStore)?;
            let candidate = store.transition_children(entry)?;
            if candidate.from == target {
                parent = Some((entry, true));
                break;
            }
            if candidate.to == target {
                parent = Some((entry, false));
                break;
            }
        }
        if let Some((entry, from)) = parent {
            self.base
                .as_mut()
                .ok_or(E::MissingOwner("Animatable"))?
                .transition_store
                .replace_transition_child(entry, from, runtime.to)?;
        } else {
            self.try_set_body_animation_reference(runtime.to)?;
        }
        self.release_body_animation_reference(runtime.from)?;
        let store = &mut self
            .base
            .as_mut()
            .ok_or(E::MissingOwner("Animatable"))?
            .transition_store;
        match handle {
            AnimationTransitionHandle::Blend(index) => store.release_retained_blend(index),
            AnimationTransitionHandle::Sequence(index) => store.release_retained_sequence(index),
        }
    }

    fn release_body_animation_reference(
        &mut self,
        reference: BodyAnimationReference,
    ) -> Result<(), E> {
        match reference {
            BodyAnimationReference::TransitionBlend(index) => {
                let runtime = self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .retained_blend(index)?
                    .clone();
                self.release_body_animation_reference(runtime.from)?;
                self.release_body_animation_reference(runtime.to)?;
                self.base
                    .as_mut()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .release_retained_blend(index)
            }
            BodyAnimationReference::TransitionSequence(index) => {
                let runtime = self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .retained_sequence(index)?
                    .clone();
                self.release_body_animation_reference(runtime.from)?;
                self.release_body_animation_reference(runtime.to)?;
                self.base
                    .as_mut()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .release_retained_sequence(index)
            }
            BodyAnimationReference::Tree(root) => {
                let initialized = self
                    .initialization
                    .as_ref()
                    .ok_or(E::MissingOwner("initialized animation"))?;
                initialized
                    .tree_pools
                    .get(root.pool_index)
                    .ok_or(E::InvalidStore)?
                    .runtime
                    .node(&root.node)
                    .map_err(E::Tree)?;
                let store = &mut self
                    .base
                    .as_mut()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .tree_store;
                if root.pool_index >= store.max_trees
                    || store.free_order[..store.free_count].contains(&root.pool_index)
                {
                    return Err(E::InvalidStore);
                }
                *store
                    .free_order
                    .get_mut(store.free_count)
                    .ok_or(E::InvalidStore)? = root.pool_index;
                store.free_count += 1;
                Ok(())
            }
        }
    }

    pub(crate) fn try_blend_body_animation(
        &mut self,
        name: FastString36,
        speed: f32,
        length: f32,
        time: f32,
        current_frame: bool,
        placement: u32,
    ) -> Result<bool, E> {
        self.try_transition_blend_body_animation(
            name,
            speed,
            length,
            time,
            current_frame,
            placement,
            TransitionBlendKind::Ordinary,
        )
    }

    /// Shared `TransitionAnim82d7aa68` continuation; type2 uses the same retained pools.
    #[expect(
        clippy::too_many_arguments,
        reason = "native transition arguments plus recovered blend kind"
    )]
    pub(crate) fn try_transition_blend_body_animation(
        &mut self,
        name: FastString36,
        speed: f32,
        length: f32,
        time: f32,
        current_frame: bool,
        placement: u32,
        kind: TransitionBlendKind,
    ) -> Result<bool, E> {
        let base = self.base.as_ref().ok_or(E::MissingOwner("Animatable"))?;
        if base.base_animation == AnimationPointerState::Null {
            if base.body_root.is_some() {
                return Err(E::InvalidStore);
            }
            return self.try_play_body_animation(name, speed, 0.0);
        }
        if base.body_root.is_none() {
            return Err(E::InvalidStore);
        }
        // Both independent equality checks occur before allocating the new tree.
        if base.transition_store.blend_order.len == base.transition_store.max_transition_blends {
            let oldest = base.transition_store.blend_order.slots[0].ok_or(E::InvalidStore)?;
            self.try_force_transition_release(AnimationTransitionHandle::Blend(oldest))?;
        }
        let base = self.base.as_ref().ok_or(E::MissingOwner("Animatable"))?;
        if base.transition_store.blend_order.len == base.transition_store.max_transition_blends {
            return Err(E::InvalidStore);
        }
        if base.transition_store.transition_order.len == base.transition_store.max_trees - 1 {
            let oldest = base.transition_store.transition_order.slots[0].ok_or(E::InvalidStore)?;
            self.try_force_transition_release(oldest)?;
        }
        let base = self.base.as_ref().ok_or(E::MissingOwner("Animatable"))?;
        if base.transition_store.transition_order.len == base.transition_store.max_trees - 1 {
            return Err(E::InvalidStore);
        }
        let Some(tree) = self.try_allocate_body_animation_tree(name)? else {
            let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
            base.transition_store.clear_play_lists();
            base.tree_store.clear_play_list();
            return Ok(false);
        };
        self.base
            .as_mut()
            .ok_or(E::MissingOwner("Animatable"))?
            .base_animation_name = name;
        let pool = &mut self
            .initialization
            .as_mut()
            .ok_or(E::MissingOwner("initialized animation"))?
            .tree_pools[tree.pool_index]
            .runtime;
        pool.set_speed(&tree.node, speed).map_err(E::Tree)?;
        if time > 0.0 {
            pool.set_time(&tree.node, time).map_err(E::Tree)?;
        }
        let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
        let store = &mut base.transition_store;
        if store.transition_order.len != 0 && placement != 0 {
            let youngest = store.transition_order.slots[store.transition_order.len - 1]
                .ok_or(E::InvalidStore)?;
            let from = store.transition_children(youngest)?.to;
            let index = store.new_retained_blend()?;
            store.init_retained_blend(
                index,
                BodyAnimationReference::Tree(tree),
                from,
                length,
                current_frame,
            )?;
            store.set_retained_blend_kind(index, kind)?;
            store.replace_transition_child(
                youngest,
                false,
                BodyAnimationReference::TransitionBlend(index),
            )?;
        } else {
            let index = store.new_retained_blend()?;
            let from = base.body_root.clone().ok_or(E::InvalidStore)?;
            store.init_retained_blend(
                index,
                BodyAnimationReference::Tree(tree),
                from,
                length,
                current_frame,
            )?;
            store.set_retained_blend_kind(index, kind)?;
            self.try_set_body_animation_reference(BodyAnimationReference::TransitionBlend(index))?;
        }
        Ok(true)
    }
}
