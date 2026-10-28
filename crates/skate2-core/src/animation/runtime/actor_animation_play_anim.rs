//! Native `Animatable::PlayAnim82d7a7a0` on the retained animation owner.
use super::{
    ActorAnimationConstruction, AnimationPointerState, AnimationTransitionStore,
    AnimationTreeStore,
    blend_anim::BodyAnimationReference,
    channel_runtime::{ChannelBaseReference, GestureChannelError, add_bind_pose},
};
use crate::{
    actor_animation_tree::{AnimationNodeHandle, AnimationTreeError},
    play_animation::FastString36,
    play_animation_lifecycle::AnimationDispatch,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BodyAnimationRoot {
    pub(crate) pool_index: usize,
    pub(crate) node: AnimationNodeHandle,
}

#[derive(Debug)]
pub(crate) enum PlayBodyAnimationError {
    MissingOwner(&'static str),
    InvalidStore,
    Lock,
    Tree(AnimationTreeError),
    BindPose(GestureChannelError),
    UnsupportedDispatch(&'static str),
}
impl std::fmt::Display for PlayBodyAnimationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOwner(owner) => write!(f, "PlayAnim requires {owner}"),
            Self::InvalidStore => f.write_str("PlayAnim store ownership is inconsistent"),
            Self::Lock => f.write_str("PlayAnim playback lock failed"),
            Self::Tree(error) => write!(f, "PlayAnim tree: {error}"),
            Self::BindPose(error) => write!(f, "PlayAnim bind pose: {error}"),
            Self::UnsupportedDispatch(kind) => {
                write!(f, "native {kind} producer is not reconstructed")
            }
        }
    }
}

impl AnimationTransitionStore {
    /// `TransitionStore::Clear82d7a080` preserves blend and sequence payloads.
    pub(super) fn clear_play_lists(&mut self) {
        self.transition_order.len = 0;
        self.blend_order.len = 0;
        self.free_blends = [0, 1, 2];
        self.free_sequences = [0, 1, 2];
        self.free_blend_count = self.max_transition_blends;
        self.free_sequence_count = self.max_trees - 1;
    }
}
impl AnimationTreeStore {
    /// `TreeStore::Clear82d79e18` resets only the free list, not pool contents.
    pub(super) fn clear_play_list(&mut self) {
        self.free_order = [0, 1, 2, 3];
        self.free_count = self.max_trees;
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Animatable.Transitioning82d720f0: empty order exits before GetBaseAnim;
    /// otherwise the actual base must occur in the retained transition list.
    pub(crate) fn try_is_transitioning(&self) -> Result<bool, PlayBodyAnimationError> {
        let base = self.base().ok_or(PlayBodyAnimationError::MissingOwner("Animatable"))?;
        let order = &base.transition_store.transition_order;
        if order.len() == 0 { return Ok(false); }
        let root = base.body_root.as_ref()
            .ok_or(PlayBodyAnimationError::MissingOwner("base animation"))?;
        for entry in order.occupied_slots() {
            let reference = match entry.ok_or(PlayBodyAnimationError::MissingOwner("transition list entry"))? {
                super::AnimationTransitionHandle::Blend(index) => BodyAnimationReference::TransitionBlend(index),
                super::AnimationTransitionHandle::Sequence(index) => BodyAnimationReference::TransitionSequence(index),
            };
            if root == &reference { return Ok(true); }
        }
        Err(PlayBodyAnimationError::InvalidStore)
    }
    /// Real database/pool/owner integration, called after the two-channel corpus.
    #[cfg(test)]
    pub(super) fn check_play_body_animation(&mut self) {
        let name = FastString36::encode("B_GSTR_SKY_BOTH").unwrap();
        assert!(self.base.as_ref().unwrap().body_root.is_none());
        assert_eq!(
            self.try_dispatch_owned_animation(AnimationDispatch::Blend {
                name,
                playback_speed_bits: 1.0_f32.to_bits(),
                time_bits: 9.0_f32.to_bits(),
                blend_length_bits: f32::NAN.to_bits(),
                blend_with_current_frame: true,
                placement_raw: 1,
            })
            .unwrap(),
            1
        );
        let base = self.base.as_ref().unwrap();
        let root = base.body_root.clone().unwrap();
        let BodyAnimationReference::Tree(tree) = &root else {
            panic!("NULL-base Blend must Play");
        };
        assert_eq!(tree.pool_index, 3);
        assert_eq!(base.tree_store.free_count, 3);
        assert_eq!(base.base_animation_name, name);
        let initialized = self.initialization.as_ref().unwrap();
        let pool = &initialized.tree_pools[3].runtime;
        assert!(pool.length(&tree.node).unwrap() > 0.0);
        assert!(!pool.node(&tree.node).unwrap().evaluated);
        assert_eq!(pool.time(&tree.node).unwrap().to_bits(), 0.0_f32.to_bits());
        let channels = initialized.channels.as_ref().unwrap();
        assert!(
            matches!(&channels.channels[5].runtime.as_ref().unwrap().base,
            ChannelBaseReference::Body(actual) if actual == &root)
        );
        assert!(matches!(
            channels.channels[4].runtime.as_ref().unwrap().base,
            ChannelBaseReference::Channel(5)
        ));
        // Same native pool is reset/reused; positive offset runs wrapped SetTime.
        assert!(self.try_play_body_animation(name, 2.0, 0.1).unwrap());
        let replacement = self.base.as_ref().unwrap().body_root.clone().unwrap();
        let BodyAnimationReference::Tree(replacement_tree) = &replacement else {
            panic!("Play must publish tree");
        };
        assert_eq!(replacement_tree.pool_index, 3);
        let before = self.initialization.as_ref().unwrap().tree_pools[3]
            .runtime
            .time(&replacement_tree.node)
            .unwrap();
        self.try_add_animation_time(0.01).unwrap();
        let base = self.base.as_ref().unwrap();
        assert!(base.is_playing);
        assert!(!base.expired_or_looped);
        let initialized = self.initialization.as_ref().unwrap();
        assert!(
            initialized.tree_pools[3]
                .runtime
                .time(&replacement_tree.node)
                .unwrap()
                > before
        );
        let channels = initialized.channels.as_ref().unwrap();
        assert_eq!(channels.channels[5].blend_out_time, Some(0.01));
        assert_eq!(channels.channels[4].blend_in_time, Some(0.01));
        assert_eq!(channels.channels[4].blend_coefficient, Some(0.0));
        let commands = std::sync::Arc::clone(
            &initialized
                .playback
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .commands,
        );
        initialized.tree_pools[3]
            .runtime
            .check_clip_fetch_plans(&replacement_tree.node, &commands);
        assert!(
            !self
                .try_play_body_animation(
                    FastString36::encode("NO_SUCH_CORPUS_ANIMATION").unwrap(),
                    1.0,
                    0.0
                )
                .unwrap()
        );
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.body_root.as_ref(), Some(&replacement));
        assert_eq!(base.base_animation_name, name);
        assert_eq!(base.tree_store.free_count, 4);
        // Failed native lookup leaves the pointer unchanged after pool clear.
        // Checked access exposes that invalidated node instead of supplying a pose.
        assert!(
            self.initialization.as_ref().unwrap().tree_pools[3]
                .runtime
                .node(&replacement_tree.node)
                .is_err()
        );
        self.check_retained_blend_corpus();
    }

    /// Terminal adapter for the already recovered `PlayAnimationBackend` request.
    pub(crate) fn try_dispatch_owned_animation(
        &mut self,
        request: AnimationDispatch,
    ) -> Result<u8, PlayBodyAnimationError> {
        match request {
            AnimationDispatch::Play {
                name,
                playback_speed_bits,
                time_bits,
            } => self
                .try_play_body_animation(
                    name,
                    f32::from_bits(playback_speed_bits),
                    f32::from_bits(time_bits),
                )
                .map(u8::from),
            AnimationDispatch::Blend {
                name,
                playback_speed_bits,
                blend_length_bits,
                time_bits,
                blend_with_current_frame,
                placement_raw,
            } => self
                .try_blend_body_animation(
                    name,
                    f32::from_bits(playback_speed_bits),
                    f32::from_bits(blend_length_bits),
                    f32::from_bits(time_bits),
                    blend_with_current_frame,
                    placement_raw,
                )
                .map(u8::from),
            AnimationDispatch::ChannelBlend {
                name,
                playback_speed_bits,
                blend_length_bits,
                time_bits,
                blend_with_current_frame,
                placement_raw,
                use_channel_from_weights,
            } => self
                .try_transition_blend_body_animation(
                    name,
                    f32::from_bits(playback_speed_bits),
                    f32::from_bits(blend_length_bits),
                    f32::from_bits(time_bits),
                    blend_with_current_frame,
                    placement_raw,
                    super::blend_anim::TransitionBlendKind::Channel {
                        use_from_weights: use_channel_from_weights,
                    },
                )
                .map(u8::from),
            AnimationDispatch::Sequence {
                name,
                playback_speed_bits,
                time_bits,
                placement_raw,
            } => self
                .try_sequence_body_animation(
                    name,
                    f32::from_bits(playback_speed_bits),
                    f32::from_bits(time_bits),
                    placement_raw,
                )
                .map(u8::from),
        }
    }

    /// `PlayAnim82d7a7a0`, including virtual `SkaterAnim::GetAnimTree82c8b710`
    /// and `ChannelAnimatable::SetBaseAnim82c8a4e8` on this same owner.
    pub(crate) fn try_play_body_animation(
        &mut self,
        name: FastString36,
        speed: f32,
        time: f32,
    ) -> Result<bool, PlayBodyAnimationError> {
        use PlayBodyAnimationError as E;
        let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
        base.transition_store.clear_play_lists();
        base.tree_store.clear_play_list();
        let Some(root) = self.try_allocate_body_animation_tree(name)? else {
            let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
            base.transition_store.clear_play_lists();
            base.tree_store.clear_play_list();
            return Ok(false);
        };
        let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
        let pool = &mut self
            .initialization
            .as_mut()
            .ok_or(E::MissingOwner("Initialize"))?
            .tree_pools[root.pool_index]
            .runtime;
        let node = &root.node;
        base.base_animation_name = name;
        pool.set_speed(node, speed).map_err(E::Tree)?;
        // Raw4099001c at82d7a8a4 tests CR6.GT false: NaN skips SetTime.
        if time > 0.0 {
            pool.set_time(node, time).map_err(E::Tree)?;
        }
        self.try_set_body_animation_reference(BodyAnimationReference::Tree(root))?;
        Ok(true)
    }

    /// `SequenceAnim82d7ae30` -> `TransitionAnim82d7aa68` type zero.
    pub(crate) fn try_sequence_body_animation(
        &mut self,
        name: FastString36,
        speed: f32,
        time: f32,
        placement: u32,
    ) -> Result<bool, PlayBodyAnimationError> {
        use PlayBodyAnimationError as E;
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
            let index = store.new_retained_sequence()?;
            store.init_retained_sequence(index, BodyAnimationReference::Tree(tree), from)?;
            let runtime = store.retained_sequence(index)?.clone();
            self.check_sequence_reset_reads(&runtime)?;
            self.base
                .as_mut()
                .ok_or(E::MissingOwner("Animatable"))?
                .transition_store
                .replace_transition_child(
                    youngest,
                    false,
                    BodyAnimationReference::TransitionSequence(index),
                )?;
        } else {
            let index = store.new_retained_sequence()?;
            let from = base.body_root.clone().ok_or(E::InvalidStore)?;
            store.init_retained_sequence(index, BodyAnimationReference::Tree(tree), from)?;
            let runtime = store.retained_sequence(index)?.clone();
            self.check_sequence_reset_reads(&runtime)?;
            self.try_set_body_animation_reference(BodyAnimationReference::TransitionSequence(
                index,
            ))?;
        }
        Ok(true)
    }

    fn check_sequence_reset_reads(
        &self,
        runtime: &super::blend_anim::TransitionSequenceRuntime,
    ) -> Result<(), PlayBodyAnimationError> {
        self.body_reference_clock(&runtime.from, false)?;
        self.body_reference_clock(&runtime.to, true)?;
        Ok(())
    }

    pub(crate) fn body_reference_clock(
        &self,
        reference: &BodyAnimationReference,
        time: bool,
    ) -> Result<f32, PlayBodyAnimationError> {
        use PlayBodyAnimationError as E;
        match reference {
            BodyAnimationReference::Tree(root) => {
                let pool = &self
                    .initialization
                    .as_ref()
                    .ok_or(E::MissingOwner("Initialize"))?
                    .tree_pools
                    .get(root.pool_index)
                    .ok_or(E::InvalidStore)?
                    .runtime;
                if time {
                    pool.time(&root.node)
                } else {
                    pool.length(&root.node)
                }
                .map_err(E::Tree)
            }
            BodyAnimationReference::TransitionBlend(index) => {
                let to = &self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .retained_blend(*index)?
                    .to;
                self.body_reference_clock(to, time)
            }
            BodyAnimationReference::TransitionSequence(index) => {
                let to = &self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .retained_sequence(*index)?
                    .to;
                self.body_reference_clock(to, time)
            }
        }
    }

    pub(super) fn try_allocate_body_animation_tree(
        &mut self,
        name: FastString36,
    ) -> Result<Option<BodyAnimationRoot>, PlayBodyAnimationError> {
        use PlayBodyAnimationError as E;
        let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
        let remaining = base
            .tree_store
            .free_count
            .checked_sub(1)
            .ok_or(E::InvalidStore)?;
        let pool_index = *base
            .tree_store
            .free_order
            .get(remaining)
            .ok_or(E::InvalidStore)?;
        base.tree_store.free_count = remaining;
        let initialized = self
            .initialization
            .as_mut()
            .ok_or(E::MissingOwner("Initialize"))?;
        let pool = &mut initialized
            .tree_pools
            .get_mut(pool_index)
            .ok_or(E::MissingOwner("body animation pool"))?
            .runtime;
        pool.clear();
        let map = base.construction_map.slots[..base.construction_map.len]
            .iter()
            .map(|entry| {
                entry
                    .as_ref()
                    .copied()
                    .ok_or(E::MissingOwner("construction map entry"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let playback = initialized
            .playback
            .as_ref()
            .ok_or(E::MissingOwner("PlaybackData"))?;
        let node = playback
            .lock()
            .map_err(|_| E::Lock)?
            .try_construct_animation_tree(pool, &name, &map)
            .map_err(E::Tree)?;
        let Some(node) = node else {
            return Ok(None);
        };
        let node = if self.flavor == super::AnimationConstructionFlavor::Pedestrian {
            super::channel_runtime::add_pedestrian_bind_pose(pool, node).map_err(E::BindPose)?
        } else {
            let player = self
                .player
                .as_mut()
                .ok_or(E::MissingOwner("SkaterAnim flags"))?;
            add_bind_pose(
                pool,
                node,
                initialized.bind_pose.as_ref(),
                initialized.board_backwards.as_ref(),
                initialized.board_backwards_ik.as_ref(),
                initialized.hierarchy.as_ref(),
                &mut player.flags_bits,
                &mut player.flags_known_mask,
            )
            .map_err(E::BindPose)?
        };
        Ok(Some(BodyAnimationRoot { pool_index, node }))
    }

    pub(super) fn try_set_body_animation_reference(
        &mut self,
        root: BodyAnimationReference,
    ) -> Result<(), PlayBodyAnimationError> {
        use PlayBodyAnimationError as E;
        let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
        let initialized = self
            .initialization
            .as_mut()
            .ok_or(E::MissingOwner("Initialize"))?;
        base.body_root = Some(root.clone());
        base.base_animation = AnimationPointerState::Constructed;
        let channels = initialized
            .channels
            .as_mut()
            .ok_or(E::MissingOwner("channel store"))?;
        if let Some(mut bottom) = channels.root {
            loop {
                let channel = channels.channels.get_mut(bottom).ok_or(E::InvalidStore)?;
                if let Some(next) = channel.base {
                    bottom = next;
                } else {
                    channel
                        .runtime
                        .as_mut()
                        .ok_or(E::MissingOwner("bottom channel runtime"))?
                        .base = ChannelBaseReference::Body(root);
                    channel.base_animation = AnimationPointerState::Constructed;
                    break;
                }
            }
        }
        Ok(())
    }
}
