//! Actual animation fields consumed by generated Motion lifecycle callbacks.
use super::ActorAnimationConstruction;
use crate::{generated_motion_load::runtime::MotionUpdateError, play_animation::FastString30};

impl<K, W> ActorAnimationConstruction<K, W> {
    /// The base `Animation::GetAttribute` (vtable +0x34) after
    /// `Animatable::GetBaseAnim`82592b48: Transition82d6db68 follows mToAnim;
    /// Sequence82d6de68 follows mToAnim only while `PlayingNewAnim` (+0x14),
    /// otherwise mFromAnim (+0x10).
    pub(crate) fn motion_base_attribute(
        &self,
        base: &super::blend_anim::BodyAnimationReference,
        name: FastString30,
        flags: u8,
    ) -> Result<Option<skate2_formats::andale::AndaleRuntimeAttribute>, MotionUpdateError> {
        use MotionUpdateError as E;
        let root = self.attribute_body_tree(base).map_err(E::AnimationPhase)?;
        self.initialization
            .as_ref()
            .ok_or(E::MissingOwner("base attribute initialized animation"))?
            .tree_pools
            .get(root.pool_index)
            .ok_or(E::MissingOwner("base attribute tree pool"))?
            .runtime
            .get_attribute(&root.node, name, flags)
            .map_err(|error| {
                E::AnimationPhase(super::frame_phase::AnimationPhaseError::Tree(error))
            })
    }

    /// JumpInto.Update82cab358: capture base, SetAttributes, query authored
    /// marker with flags31, then SetTime(marker.mBeginTime) if found.
    pub(crate) fn motion_jump_into(
        &mut self,
        name: FastString30,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), MotionUpdateError> {
        use super::blend_anim::BodyAnimationReference as R;
        use MotionUpdateError as E;
        let mut reference = self
            .base
            .as_ref()
            .and_then(|base| base.body_root.clone())
            .ok_or(E::MissingOwner("JumpInto base animation"))?;
        self.try_skater_set_attributes(cvars)
            .map_err(E::AnimationPhase)?;
        let Some(attribute) = self.motion_base_attribute(&reference, name, 31)? else {
            return Ok(());
        };
        // SetTime82d6db38 delegates to mToAnim at +0x0c for Transition and
        // Sequence alike, even before the Sequence starts playing To.
        let root = loop {
            let store = &self
                .base
                .as_ref()
                .ok_or(E::MissingOwner("JumpInto Animatable"))?
                .transition_store;
            reference = match reference {
                R::Tree(root) => break root,
                R::TransitionBlend(index) => store
                    .retained_blend(index)
                    .map_err(E::AnimationBody)?
                    .to
                    .clone(),
                R::TransitionSequence(index) => store
                    .retained_sequence(index)
                    .map_err(E::AnimationBody)?
                    .to
                    .clone(),
            };
        };
        let pool = &mut self
            .initialization
            .as_mut()
            .ok_or(E::MissingOwner("JumpInto initialized animation"))?
            .tree_pools
            .get_mut(root.pool_index)
            .ok_or(E::MissingOwner("JumpInto tree pool"))?
            .runtime;
        pool.set_time(&root.node, attribute.begin_seconds)
            .map_err(|error| {
                E::AnimationBody(super::play_anim::PlayBodyAnimationError::Tree(error))
            })?;
        if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            eprintln!(
                "known air: jump into begin={} ({:#010x}) time={:?} length={:?}",
                attribute.begin_seconds,
                attribute.begin_seconds.to_bits(),
                pool.time(&root.node).ok().map(f32::to_bits),
                pool.length(&root.node).ok().map(f32::to_bits)
            );
        }
        Ok(())
    }

    /// `Animatable::SetConstructionAttr`8259e7c8: fixed-map key assignment.
    pub(crate) fn motion_set_construction_attribute(
        &mut self,
        key: FastString30,
        value: FastString30,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let map = &mut self
            .base
            .as_mut()
            .ok_or(E::MissingOwner("Animatable construction map"))?
            .construction_map;
        let mut insertion = map.len;
        for (index, slot) in map.slots[..map.len].iter_mut().enumerate() {
            let (stored_key, stored_value) = slot
                .as_mut()
                .ok_or(E::MissingOwner("construction map entry"))?;
            if *stored_key == key {
                *stored_value = value;
                return Ok(());
            }
            if key < *stored_key {
                insertion = index;
                break;
            }
        }
        if map.len == map.slots.len() {
            return Err(E::Capacity);
        }
        for index in (insertion..map.len).rev() {
            map.slots[index + 1] = map.slots[index].take();
        }
        map.slots[insertion] = Some((key, value));
        map.len += 1;
        Ok(())
    }

    /// `Animatable::RemoveConstructionAttr`82593058 removes only a found key.
    pub(crate) fn motion_remove_construction_attribute(
        &mut self,
        key: FastString30,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let map = &mut self
            .base
            .as_mut()
            .ok_or(E::MissingOwner("Animatable construction map"))?
            .construction_map;
        for position in 0..map.len {
            let stored = map.slots[position]
                .as_ref()
                .ok_or(E::MissingOwner("construction map entry"))?
                .0;
            if stored == key {
                for index in position..map.len - 1 {
                    map.slots[index] = map.slots[index + 1].take();
                }
                map.len -= 1;
                map.slots[map.len] = None; // Host occupancy, not native clearing of unused bytes.
                return Ok(());
            }
            if stored > key {
                return Ok(());
            }
        }
        Ok(())
    }

    /// `SkaterAnim::SetIsRidingFakie`82cb7540 modifies only source flag20000000.
    pub(crate) fn motion_set_riding_fakie(
        &mut self,
        enabled: bool,
    ) -> Result<(), MotionUpdateError> {
        let player = self
            .player
            .as_mut()
            .ok_or(MotionUpdateError::MissingOwner("SkaterAnim player"))?;
        player.flags_bits =
            (player.flags_bits & !0x2000_0000) | if enabled { 0x2000_0000 } else { 0 };
        player.flags_known_mask |= 0x2000_0000;
        Ok(())
    }
}
