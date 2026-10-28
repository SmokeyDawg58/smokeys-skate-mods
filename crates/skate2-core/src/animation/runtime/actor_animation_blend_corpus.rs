//! Real original-animation ownership regressions on the disposable corpus Actor.
use super::{
    ActorAnimationConstruction, blend_anim::BodyAnimationReference,
    channel_runtime::ChannelBaseReference, play_anim::BodyAnimationRoot,
};
use crate::{
    actor_animation_tree::time::AnimationProperties,
    actor_face_playback::blend_pose_stream::BlendPoseTokenPool, play_animation::FastString36,
};

fn tree(reference: &BodyAnimationReference) -> BodyAnimationRoot {
    let BodyAnimationReference::Tree(root) = reference else {
        panic!("expected actual tree");
    };
    root.clone()
}
fn blend(reference: &BodyAnimationReference) -> usize {
    let BodyAnimationReference::TransitionBlend(index) = reference else {
        panic!("expected transition");
    };
    *index
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Mount AFTER `check_play_body_animation`, on its disposable post-teardown Actor.
    /// A fresh native Play repairs the deliberately invalidated previous test root.
    #[expect(
        clippy::float_cmp,
        clippy::too_many_lines,
        reason = "one retained original-owner lifecycle with exact binary timestep endpoints"
    )]
    pub(super) fn check_retained_blend_corpus(&mut self) {
        let name = FastString36::encode("B_GSTR_SKY_BOTH").unwrap();
        assert!(self.try_play_body_animation(name, 1.0, 0.0).unwrap());
        let original = tree(self.base.as_ref().unwrap().body_root.as_ref().unwrap());
        assert!(
            self.initialization.as_ref().unwrap().tree_pools[original.pool_index]
                .runtime
                .length(&original.node)
                .unwrap()
                > 0.125
        );
        assert!(
            self.try_blend_body_animation(name, 1.0, 0.03125, 0.0, false, 0)
                .unwrap()
        );
        let reference = self.base.as_ref().unwrap().body_root.clone().unwrap();
        let index = blend(&reference);
        let runtime = self
            .base
            .as_ref()
            .unwrap()
            .transition_store
            .retained_blend(index)
            .unwrap()
            .clone();
        assert_eq!(runtime.from, BodyAnimationReference::Tree(original.clone()));
        let destination = tree(&runtime.to);
        assert_ne!(original.pool_index, destination.pool_index);
        let channels = self
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap();
        let mut bottom = channels.root.unwrap();
        while let Some(next) = channels.channels[bottom].base {
            bottom = next;
        }
        assert!(matches!(
            &channels.channels[bottom].runtime.as_ref().unwrap().base,
            ChannelBaseReference::Body(actual) if actual == &reference
        ));

        // Real hand/face stream production: coefficient zero must preserve FROM.
        let mut from_tokens = BlendPoseTokenPool::new();
        let mut from_buffer = from_tokens.buffer(0).unwrap();
        self.eval_body_blend_pose(&runtime.from, &mut from_buffer, &mut from_tokens)
            .unwrap();
        let mut transition_tokens = BlendPoseTokenPool::new();
        let mut transition_buffer = transition_tokens.buffer(0).unwrap();
        self.eval_body_blend_pose(&reference, &mut transition_buffer, &mut transition_tokens)
            .unwrap();
        for stream in 0..3 {
            assert_eq!(
                from_tokens.tokens(&from_buffer, stream).unwrap(),
                transition_tokens
                    .tokens(&transition_buffer, stream)
                    .unwrap()
            );
        }

        self.try_add_animation_time(0.015_625).unwrap();
        transition_tokens.reset();
        transition_buffer = transition_tokens.buffer(0).unwrap();
        self.eval_body_blend_pose(&reference, &mut transition_buffer, &mut transition_tokens)
            .unwrap();
        assert_eq!(self.try_transition_blend_weight(index).unwrap(), 0.5);
        for root in [&original, &destination] {
            assert_eq!(
                self.initialization.as_ref().unwrap().tree_pools[root.pool_index]
                    .runtime
                    .time(&root.node)
                    .unwrap(),
                0.015_625
            );
        }
        self.try_add_animation_time(0.015_625).unwrap();
        assert_eq!(
            self.base.as_ref().unwrap().transition_store.blends[index].time,
            0.03125
        );
        self.try_update_body_transitions().unwrap();
        assert_eq!(
            self.base
                .as_ref()
                .unwrap()
                .transition_store
                .transition_order
                .len,
            1
        );
        // Equality is incomplete. Crossing occurs in AddTime, pruning next call.
        self.try_add_animation_time(0.007_812_5).unwrap();
        assert_eq!(
            self.base
                .as_ref()
                .unwrap()
                .transition_store
                .transition_order
                .len,
            1
        );
        self.try_update_body_transitions().unwrap();
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.body_root.as_ref(), Some(&runtime.to));
        assert_eq!(base.transition_store.transition_order.len, 0);
        assert_eq!(base.tree_store.free_count, 3);
        assert_eq!(base.tree_store.free_order[2], original.pool_index);

        // Current-frame=true freezes the retained FROM but advances TO and clock.
        assert!(self.try_play_body_animation(name, 1.0, 0.0).unwrap());
        let frozen = tree(self.base.as_ref().unwrap().body_root.as_ref().unwrap());
        assert!(
            self.try_blend_body_animation(name, 1.0, 1.0, 0.0, true, 0)
                .unwrap()
        );
        let current = self.base.as_ref().unwrap().body_root.clone().unwrap();
        let current_index = blend(&current);
        let target = tree(
            &self
                .base
                .as_ref()
                .unwrap()
                .transition_store
                .retained_blend(current_index)
                .unwrap()
                .to,
        );
        self.add_body_reference_time(&current, 0.015_625, &mut AnimationProperties::default())
            .unwrap();
        assert_eq!(
            self.initialization.as_ref().unwrap().tree_pools[frozen.pool_index]
                .runtime
                .time(&frozen.node)
                .unwrap(),
            0.0
        );
        assert_eq!(
            self.initialization.as_ref().unwrap().tree_pools[target.pool_index]
                .runtime
                .time(&target.node)
                .unwrap(),
            0.015_625
        );
        assert_eq!(
            self.base.as_ref().unwrap().transition_store.blends[current_index].time,
            0.015_625
        );

        // Nonzero placement inserts into youngest.TO instead of replacing base.
        assert!(
            self.try_blend_body_animation(name, 1.0, 1.0, 0.0, false, 1)
                .unwrap()
        );
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.body_root.as_ref(), Some(&current));
        let nested = blend(
            &base
                .transition_store
                .retained_blend(current_index)
                .unwrap()
                .to,
        );
        assert_eq!(
            base.transition_store.retained_blend(nested).unwrap().from,
            BodyAnimationReference::Tree(target)
        );
        assert_eq!(base.transition_store.transition_order.len, 2);

        // A fourth blend prunes oldest and reuses its released actual tree pool.
        assert!(self.try_play_body_animation(name, 1.0, 0.0).unwrap());
        let initial = tree(self.base.as_ref().unwrap().body_root.as_ref().unwrap());
        for _ in 0..4 {
            assert!(
                self.try_blend_body_animation(name, 1.0, 100.0, 0.0, false, 0)
                    .unwrap()
            );
        }
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.transition_store.transition_order.len, 3);
        assert_eq!(
            base.transition_store.transition_order.slots,
            [
                Some(super::AnimationTransitionHandle::Blend(1)),
                Some(super::AnimationTransitionHandle::Blend(0)),
                Some(super::AnimationTransitionHandle::Blend(2)),
            ]
        );
        assert_eq!(base.transition_store.free_blend_count, 0);
        assert_eq!(base.tree_store.free_count, 0);
        let newest = blend(base.body_root.as_ref().unwrap());
        let survivor = base
            .transition_store
            .retained_blend(newest)
            .unwrap()
            .to
            .clone();
        assert_eq!(tree(&survivor).pool_index, initial.pool_index);
        self.try_force_transition_release(super::AnimationTransitionHandle::Blend(newest))
            .unwrap();
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.body_root.as_ref(), Some(&survivor));
        assert_eq!(base.transition_store.transition_order.len, 0);
        assert_eq!(base.transition_store.blend_order.len, 0);
        assert_eq!(base.transition_store.free_blend_count, 3);
        assert_eq!(base.tree_store.free_count, 3);
        assert_eq!(&base.tree_store.free_order[..3], &[2, 1, 0]);
        assert_eq!(base.transition_store.free_blends, [1, 0, 2]);
        let root = tree(&survivor);
        assert!(
            self.initialization.as_ref().unwrap().tree_pools[root.pool_index]
                .runtime
                .node(&root.node)
                .is_ok()
        );

        // A native Sequence waits on the existing tree's expiry, then advances
        // its TO tree by only the overrun and releases the FROM pool next frame.
        assert!(self.try_play_body_animation(name, 1.0, 0.0).unwrap());
        let from = tree(self.base.as_ref().unwrap().body_root.as_ref().unwrap());
        let from_length = self.initialization.as_ref().unwrap().tree_pools[from.pool_index]
            .runtime
            .length(&from.node)
            .unwrap();
        assert!(self.try_sequence_body_animation(name, 1.0, 0.0, 0).unwrap());
        let sequence = self.base.as_ref().unwrap().body_root.clone().unwrap();
        let BodyAnimationReference::TransitionSequence(sequence_index) = sequence.clone() else {
            panic!("SequenceAnim must publish a sequence root");
        };
        let runtime = self
            .base
            .as_ref()
            .unwrap()
            .transition_store
            .retained_sequence(sequence_index)
            .unwrap()
            .clone();
        assert_eq!(runtime.from, BodyAnimationReference::Tree(from));
        assert_eq!(
            self.base
                .as_ref()
                .unwrap()
                .transition_store
                .transition_order
                .slots[0],
            Some(super::AnimationTransitionHandle::Sequence(sequence_index))
        );
        let to = tree(&runtime.to);
        assert_ne!(to.pool_index, tree(&runtime.from).pool_index);
        self.try_add_animation_time(from_length + 0.03125).unwrap();
        assert!(
            self.base.as_ref().unwrap().transition_store.sequences[sequence_index]
                .playing_new_animation
        );
        assert!(
            self.initialization.as_ref().unwrap().tree_pools[to.pool_index]
                .runtime
                .time(&to.node)
                .unwrap()
                > 0.0
        );
        self.try_update_body_transitions().unwrap();
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.body_root.as_ref(), Some(&runtime.to));
        assert_eq!(base.transition_store.transition_order.len, 0);
        assert_eq!(base.transition_store.free_sequence_count, 3);
        assert_eq!(base.tree_store.free_count, 3);

        // UNDER attaches the new sequence to the youngest transition's TO
        // child while retaining the current body root. Forced release must
        // find that mixed-kind parent and publish the sequence's TO child.
        assert!(self.try_play_body_animation(name, 1.0, 0.0).unwrap());
        assert!(
            self.try_blend_body_animation(name, 1.0, 1.0, 0.0, false, 0)
                .unwrap()
        );
        let blend_root = self.base.as_ref().unwrap().body_root.clone().unwrap();
        let blend_index = blend(&blend_root);
        let old_to = self
            .base
            .as_ref()
            .unwrap()
            .transition_store
            .retained_blend(blend_index)
            .unwrap()
            .to
            .clone();
        assert!(self.try_sequence_body_animation(name, 1.0, 0.0, 1).unwrap());
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.body_root.as_ref(), Some(&blend_root));
        let BodyAnimationReference::TransitionSequence(nested_index) = base
            .transition_store
            .retained_blend(blend_index)
            .unwrap()
            .to
            .clone()
        else {
            panic!("UNDER must attach sequence to youngest TO");
        };
        let nested = base
            .transition_store
            .retained_sequence(nested_index)
            .unwrap()
            .clone();
        assert_eq!(nested.from, old_to);
        assert_eq!(base.transition_store.transition_order.len, 2);
        self.try_force_transition_release(super::AnimationTransitionHandle::Sequence(nested_index))
            .unwrap();
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.body_root.as_ref(), Some(&blend_root));
        assert_eq!(
            base.transition_store
                .retained_blend(blend_index)
                .unwrap()
                .to,
            nested.to
        );
        assert_eq!(base.transition_store.transition_order.len, 1);
        assert_eq!(base.transition_store.free_sequence_count, 3);

        // Failed native lookup clears pool/list ownership after the attempted
        // sequence allocation, retaining the old root pointer for diagnosis.
        let missing = FastString36::encode("NO_SUCH_CORPUS_ANIMATION").unwrap();
        assert!(
            !self
                .try_sequence_body_animation(missing, 1.0, 0.0, 0)
                .unwrap()
        );
        let base = self.base.as_ref().unwrap();
        assert_eq!(base.body_root.as_ref(), Some(&blend_root));
        assert_eq!(base.transition_store.transition_order.len, 0);
        assert_eq!(base.tree_store.free_count, 4);
        // The next disposable-channel corpus uses this same Actor. Repair the
        // intentionally invalidated root with native Play before handing it on.
        assert!(self.try_play_body_animation(name, 1.0, 0.0).unwrap());
    }
}
