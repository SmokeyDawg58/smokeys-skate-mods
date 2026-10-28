//! ChannelAnimatable::StartTransition82d7ee68 with its inline Sequence and
//! TransitionBlend owners, plus the channel time queries.
use super::super::{
    ActorAnimationConstruction, AnimationPointerState,
    initialization::{InitializedAnimationChannel as Channel, OwnedAnimationPool},
};
use super::{
    ChannelParams, GestureChannelError as E, NewGestureChannelStatus, add_bind_pose,
    add_pedestrian_bind_pose,
};
use crate::{
    actor_animation_tree::{AnimationNodeHandle, AnimationTreeError, time::AnimationProperties},
    play_animation::FastString36,
};
#[path = "actor_animation_channel_transition_eval.rs"]
mod evaluation;
pub(crate) use evaluation::{eval, eval_blend_pose};

/// `Animatable::eTransType`: SequenceTo82d80d40 passes 0, TransitionTo82d80da0 1.
/// Channel+0x44 then targets the inline Sequence (+0x48) or TransitionBlend (+0x64).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChannelTransitionKind {
    Sequence,
    Blend,
}

/// The trailing StartTransition bools, in native argument order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChannelStartFlags {
    pub(crate) resurrect_if_blending_out: bool,
    pub(crate) create_if_missing: bool,
    pub(crate) use_attributes: bool,
}

/// A transitioning channel whose Channel+0x44 is the inline Sequence.
pub(crate) fn is_sequence(channel: &Channel) -> Result<bool, E> {
    if !channel.transitioning {
        return Ok(false);
    }
    Ok(channel
        .transition_kind
        .ok_or(E::MissingOwner("channel transition kind"))?
        == ChannelTransitionKind::Sequence)
}

/// Transition's GetTime/GetLength select its TO child for both kinds.
pub(crate) fn target(channel: &Channel) -> Result<(usize, AnimationNodeHandle), E> {
    if channel.transitioning {
        Ok((
            channel
                .transition_tree
                .ok_or(E::MissingOwner("transition tree"))?,
            channel
                .transition_to
                .clone()
                .ok_or(E::MissingOwner("inline transition TO"))?,
        ))
    } else {
        from_child(channel)
    }
}

/// The FROM child is the channel animation retained in the channel tree.
fn from_child(channel: &Channel) -> Result<(usize, AnimationNodeHandle), E> {
    Ok((
        channel
            .channel_tree
            .ok_or(E::MissingOwner("channel tree"))?,
        channel
            .runtime
            .as_ref()
            .ok_or(E::MissingOwner("channel runtime"))?
            .animation
            .clone(),
    ))
}

/// GetAttributes/Eval/EvalBlendPose child. Sequence82d6de00/dd00/dd38 keep
/// FROM until PlayingNewAnim (its TransitionComplete); Transition uses TO.
pub(crate) fn attribute_target(channel: &Channel) -> Result<(usize, AnimationNodeHandle), E> {
    if is_sequence(channel)? && !channel.sequence.playing_new_animation {
        from_child(channel)
    } else {
        target(channel)
    }
}

/// Sequence.SetAttributes82d6ded0 sets TO, then FROM until PlayingNewAnim;
/// Transition.SetAttributes82a35158 forwards to TO only.
pub(crate) fn settable_from(channel: &Channel) -> Result<Option<(usize, AnimationNodeHandle)>, E> {
    if is_sequence(channel)? && !channel.sequence.playing_new_animation {
        from_child(channel).map(Some)
    } else {
        Ok(None)
    }
}

/// TransitionBlend.AddTime82d6df98: FROM local properties, TO caller properties,
/// then clock publication. StartTransition selects blendFromCurrentFrame=false.
pub(crate) fn add_time(
    channel: &mut Channel,
    trees: &mut [OwnedAnimationPool],
    dt: f32,
    properties: &mut AnimationProperties,
) -> Result<(), E> {
    if is_sequence(channel)? {
        return add_sequence_time(channel, trees, dt, properties);
    }
    // dfc8 BO4/BI24 is LT-clear (IDA aliases it BGE), including unordered.
    if channel.transitioning && dt < 0.0 {
        return Err(E::Tree(AnimationTreeError::NegativeTime));
    }
    if channel.transitioning {
        let (tree, node) = from_child(channel)?;
        trees
            .get_mut(tree)
            .ok_or(E::InvalidChannel)?
            .runtime
            .add_time(&node, dt, &mut AnimationProperties::default())
            .map_err(E::Tree)?;
    }
    let (tree, node) = target(channel)?;
    trees
        .get_mut(tree)
        .ok_or(E::InvalidChannel)?
        .runtime
        .add_time(&node, dt, properties)
        .map_err(E::Tree)?;
    if channel.transitioning {
        channel.transition_blend.time += dt;
    }
    Ok(())
}

/// Sequence.AddTime82d6dbd8: FROM advances with a local property block; its
/// expiry latches PlayingNewAnim and hands the overshoot to TO.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn add_sequence_time(
    channel: &mut Channel,
    trees: &mut [OwnedAnimationPool],
    dt: f32,
    properties: &mut AnimationProperties,
) -> Result<(), E> {
    // dc0c BGE skips the assertion; only ordered negative time asserts.
    if dt < 0.0 {
        return Err(E::Tree(AnimationTreeError::NegativeTime));
    }
    let (to_tree, to) = target(channel)?;
    if channel.sequence.playing_new_animation {
        return trees
            .get_mut(to_tree)
            .ok_or(E::InvalidChannel)?
            .runtime
            .add_time(&to, dt, properties)
            .map_err(E::Tree);
    }
    let (from_tree, from) = from_child(channel)?;
    let mut local = AnimationProperties::default();
    trees
        .get_mut(from_tree)
        .ok_or(E::InvalidChannel)?
        .runtime
        .add_time(&from, dt, &mut local)
        .map_err(E::Tree)?;
    if local.expired_or_looped {
        channel.sequence.playing_new_animation = true;
        // dc84 BGT skips the assertion, so unordered overshoot asserts.
        if !(local.expired_by > 0.0) {
            return Err(E::MissingOwner("Sequence positive expiry assertion"));
        }
        return trees
            .get_mut(to_tree)
            .ok_or(E::InvalidChannel)?
            .runtime
            .add_time(&to, local.expired_by, properties)
            .map_err(E::Tree);
    }
    // dcc8..dce8: only the expired byte and time-to-expire are written.
    properties.expired_or_looped = false;
    properties.time_to_expire = trees
        .get(to_tree)
        .ok_or(E::InvalidChannel)?
        .runtime
        .length(&to)
        .map_err(E::Tree)?
        + local.time_to_expire;
    Ok(())
}

fn clamp_unit(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

/// FSEL chooses its nonnegative operand for either zero and its negative
/// operand for unordered input.
fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// FakieHead82caf338's TransitionTo82d80da0: priority0, cycle, speed1,
    /// blends0.3, transition0.1, resurrect and create, no channel attributes.
    pub(crate) fn try_start_fakie_channel(
        &mut self,
        name: FastString36,
    ) -> Result<NewGestureChannelStatus, E> {
        self.try_transition_fakie_channel(
            name,
            FastString36::encode("B_FAKIE_CHANNEL").expect("native literal"),
        )
    }
    fn try_transition_fakie_channel(
        &mut self,
        name: FastString36,
        animation_name: FastString36,
    ) -> Result<NewGestureChannelStatus, E> {
        self.try_transition_channel_to(
            name,
            animation_name,
            0.1,
            ChannelParams {
                priority: 0,
                cycle: true,
                mirror: false,
                playback_speed: 1.0,
                blend_in_length: 0.3,
                blend_in_to_first_frame: false,
                blend_out_length: 0.3,
                blend_out_from_last_frame: false,
            },
            ChannelStartFlags {
                resurrect_if_blending_out: true,
                create_if_missing: true,
                use_attributes: false,
            },
        )
    }

    /// `TransitionTo82d80da0`: StartTransition with eTransType 1.
    pub(crate) fn try_transition_channel_to(
        &mut self,
        name: FastString36,
        animation_name: FastString36,
        transition_length: f32,
        params: ChannelParams,
        flags: ChannelStartFlags,
    ) -> Result<NewGestureChannelStatus, E> {
        self.try_start_channel_transition(
            name,
            animation_name,
            transition_length,
            params,
            flags,
            ChannelTransitionKind::Blend,
        )
    }

    /// `SequenceTo82d80d40`: StartTransition with length 0 and eTransType 0.
    pub(crate) fn try_sequence_channel_to(
        &mut self,
        name: FastString36,
        animation_name: FastString36,
        params: ChannelParams,
        flags: ChannelStartFlags,
    ) -> Result<NewGestureChannelStatus, E> {
        self.try_start_channel_transition(
            name,
            animation_name,
            0.0,
            params,
            flags,
            ChannelTransitionKind::Sequence,
        )
    }

    /// `StartTransition82d7ee68`. This is the actual channel's inline
    /// transition; body animation pools are not involved.
    #[expect(
        clippy::too_many_lines,
        reason = "native lookup, allocation, construction and inline publication order"
    )]
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn try_start_channel_transition(
        &mut self,
        name: FastString36,
        animation_name: FastString36,
        transition_length: f32,
        params: ChannelParams,
        flags: ChannelStartFlags,
        kind: ChannelTransitionKind,
    ) -> Result<NewGestureChannelStatus, E> {
        let channels = self
            .initialization
            .as_ref()
            .and_then(|s| s.channels.as_ref())
            .ok_or(E::MissingOwner("channels"))?;
        let Some(&index) = channels.dictionary.get(&name) else {
            // eed8..ef28: NewChannel with the same params and useChannelAttributes.
            return if flags.create_if_missing {
                self.try_new_channel(name, animation_name, params, flags.use_attributes)
            } else {
                Ok(NewGestureChannelStatus::AlreadyPresent)
            };
        };
        let channel = channels.channels.get(index).ok_or(E::InvalidChannel)?;
        // ef4c BGT builds a strict boolean; unordered is not blending out.
        let blending = channel
            .blend_out_time
            .ok_or(E::MissingOwner("blend out time"))?
            > 0.0;
        if blending && !flags.resurrect_if_blending_out {
            return Ok(NewGestureChannelStatus::BlendingOut);
        }
        if channel.transitioning {
            self.try_complete_channel_transition(index)?;
        }
        let initialized = self
            .initialization
            .as_mut()
            .ok_or(E::MissingOwner("Initialize"))?;
        let channels = initialized
            .channels
            .as_mut()
            .ok_or(E::MissingOwner("channels"))?;
        let tree = *channels
            .free_trees
            .last()
            .ok_or(E::MissingOwner("free transition tree"))?;
        channels
            .channels
            .get_mut(index)
            .ok_or(E::InvalidChannel)?
            .transition_tree = Some(tree);
        let pool = &mut channels
            .trees
            .get_mut(tree)
            .ok_or(E::InvalidChannel)?
            .runtime;
        let map = &self
            .base
            .as_ref()
            .ok_or(E::MissingOwner("construction map"))?
            .construction_map;
        let map = map.slots[..map.len]
            .iter()
            .map(|e| {
                e.as_ref()
                    .copied()
                    .ok_or(E::MissingOwner("construction map entry"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let playback = initialized
            .playback
            .as_ref()
            .ok_or(E::MissingOwner("PlaybackData"))?;
        let Some(node) = playback
            .lock()
            .map_err(|_| E::Lock)?
            .try_construct_animation_tree(pool, &animation_name, &map)
            .map_err(E::Tree)?
        else {
            return Ok(NewGestureChannelStatus::AnimationMissing);
        };
        pool.set_speed(&node, params.playback_speed)
            .map_err(E::Tree)?;
        let node = if self.flavor == super::super::AnimationConstructionFlavor::Pedestrian {
            add_pedestrian_bind_pose(pool, node)?
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
            )?
        };
        // f05c..f09c: optional Mirror over the AddBindPose result, UseTraj2.
        let node = if params.mirror {
            pool.push_mirror(initialized.hierarchy.clone(), node, 2)
                .map_err(E::Tree)?
        } else {
            node
        };
        let channel = channels.channels.get_mut(index).ok_or(E::InvalidChannel)?;
        match kind {
            ChannelTransitionKind::Sequence => {
                // f0ac..f0d4: TO, PlayingNewAnim cleared, FROM = channel anim.
                // ResetTransitionTime82d6db80 only calls FROM GetLength and TO
                // GetTime, discarding both results.
                channel.sequence.to_animation = AnimationPointerState::Constructed;
                channel.sequence.playing_new_animation = false;
                channel.sequence.from_animation = AnimationPointerState::Constructed;
            }
            ChannelTransitionKind::Blend => {
                // TransitionBlend.Init82d6b810: pointers, positive-length
                // assertion, length, ResetTransitionTime, high three flags cleared.
                channel.transition_blend.to_animation = AnimationPointerState::Constructed;
                channel.transition_blend.from_animation = AnimationPointerState::Constructed;
                if !(transition_length > 0.0) {
                    return Err(E::MissingOwner("TransitionBlend positive length assertion"));
                }
                channel.transition_blend.length = transition_length;
                channel.transition_blend.time = 0.0;
                channel.transition_blend.flags_known_zero_mask |= 0xe000_0000;
            }
        }
        channel.transition_to = Some(node);
        let out_time = channel
            .blend_out_time
            .ok_or(E::MissingOwner("blend out time"))?;
        channel.transition = AnimationPointerState::Constructed;
        channel.transition_kind = Some(kind);
        channel.transitioning = true;
        // f10c BGT builds a strict boolean; NaN does not resurrect.
        if out_time > 0.0 && flags.resurrect_if_blending_out {
            // Resurrect82d6d8b0 with the params blend-in length.
            let fraction = out_time
                / channel
                    .blend_out_length
                    .ok_or(E::MissingOwner("blend out length"))?;
            channel.resurrect_length = Some(params.blend_in_length);
            channel.resurrect_time = Some(clamp_unit(1.0 - fraction) * params.blend_in_length);
        }
        // InitForTransition82d6d5a0 preserves blend-in clock/coefficient,
        // influence, first-frame and expired; replaces cycle/last/early/out/use.
        let blending_out = params.blend_out_length > 0.0;
        channel.blend_out_length = Some(params.blend_out_length);
        channel.channel_animation = AnimationPointerState::Constructed;
        channel.blend_out_time = Some(0.0);
        let runtime = channel
            .runtime
            .as_mut()
            .ok_or(E::MissingOwner("channel runtime"))?;
        runtime.cycle = params.cycle;
        runtime.blend_out_last_frame = params.blend_out_from_last_frame;
        runtime.blend_out_early = false;
        runtime.blending_out = blending_out;
        runtime.use_attributes = flags.use_attributes;
        let channel_flags = channel
            .channel_flags
            .as_mut()
            .ok_or(E::MissingOwner("channel flags"))?;
        *channel_flags = (*channel_flags & 0x85ff_ffff)
            | if params.blend_out_from_last_frame {
                0x4000_0000
            } else {
                0
            }
            | if params.cycle { 0x1000_0000 } else { 0 }
            | if blending_out { 0x0800_0000 } else { 0 }
            | if flags.use_attributes { 0x0200_0000 } else { 0 };
        channels.free_trees.pop();
        Ok(NewGestureChannelStatus::Created)
    }

    /// CompleteTransition82d7d7d8: promote first, release old tree, capacity
    /// assertion, then clear transitioning and swap pool. Inline payload stays.
    pub(crate) fn try_complete_channel_transition(&mut self, index: usize) -> Result<(), E> {
        let channels = self
            .initialization
            .as_mut()
            .and_then(|s| s.channels.as_mut())
            .ok_or(E::MissingOwner("channels"))?;
        let channel = channels.channels.get_mut(index).ok_or(E::InvalidChannel)?;
        let to = channel
            .transition_to
            .clone()
            .ok_or(E::MissingOwner("transition TO"))?;
        channel
            .runtime
            .as_mut()
            .ok_or(E::MissingOwner("channel runtime"))?
            .animation = to;
        let old = channel
            .channel_tree
            .ok_or(E::MissingOwner("channel tree"))?;
        channels
            .trees
            .get_mut(old)
            .ok_or(E::InvalidChannel)?
            .runtime
            .clear();
        channels.free_trees.push(old);
        let maximum = self
            .channel
            .as_ref()
            .and_then(|c| c.max_channels)
            .and_then(|v| usize::try_from(v).ok())
            .ok_or(E::MissingOwner("channel maximum"))?;
        if channels.free_trees.len() > maximum.checked_mul(2).ok_or(E::ChannelCapacity)? {
            return Err(E::ChannelCapacity);
        }
        let tree = channel
            .transition_tree
            .ok_or(E::MissingOwner("transition tree"))?;
        channel.transitioning = false;
        channel.channel_tree = Some(tree);
        Ok(())
    }

    /// UpdateTransitions82d74818: dictionary order. TransitionComplete is the
    /// Blend's strict time>length (BEFORE dt) or the Sequence's PlayingNewAnim.
    pub(crate) fn try_update_channel_transitions(&mut self) -> Result<(), E> {
        let channels = self
            .initialization
            .as_ref()
            .and_then(|s| s.channels.as_ref())
            .ok_or(E::MissingOwner("channels"))?;
        if channels.root.is_none() {
            return Ok(());
        }
        let indices = channels.dictionary.values().copied().collect::<Vec<_>>();
        for index in indices {
            let channel = &self
                .initialization
                .as_ref()
                .expect("retained")
                .channels
                .as_ref()
                .expect("retained")
                .channels[index];
            if channel.transitioning
                && if is_sequence(channel)? {
                    channel.sequence.playing_new_animation
                } else {
                    channel.transition_blend.time > channel.transition_blend.length
                }
            {
                self.try_complete_channel_transition(index)?;
            }
        }
        Ok(())
    }

    /// `HasChannel82cc0a30`: dictionary presence only.
    pub(crate) fn has_channel(&self, name: &FastString36) -> Result<bool, E> {
        Ok(self
            .initialization
            .as_ref()
            .and_then(|s| s.channels.as_ref())
            .ok_or(E::MissingOwner("channels"))?
            .dictionary
            .contains_key(name))
    }

    /// The channel animation's GetLength then GetTime (TO while transitioning).
    fn channel_length_and_time(&self, name: &FastString36) -> Result<Option<(f32, f32)>, E> {
        let channels = self
            .initialization
            .as_ref()
            .and_then(|s| s.channels.as_ref())
            .ok_or(E::MissingOwner("channels"))?;
        let Some(&index) = channels.dictionary.get(name) else {
            return Ok(None);
        };
        let channel = channels.channels.get(index).ok_or(E::InvalidChannel)?;
        let (tree, node) = target(channel)?;
        let pool = &channels.trees.get(tree).ok_or(E::InvalidChannel)?.runtime;
        let length = pool.length(&node).map_err(E::Tree)?;
        let time = pool.time(&node).map_err(E::Tree)?;
        Ok(Some((length, time)))
    }

    /// `TimeElapsedInChannel82d7d3e0`: 0 when absent, else time clamped to
    /// [0, length] with the source FSEL pair.
    pub(crate) fn time_elapsed_in_channel(&self, name: &FastString36) -> Result<f32, E> {
        let Some((length, time)) = self.channel_length_and_time(name)? else {
            return Ok(0.0);
        };
        let lower = select(-time, 0.0, time);
        Ok(select(length - lower, lower, length))
    }

    /// `TimeLeftInChannel82d7d520`: 0 when absent, else FSEL(length-time, 0).
    pub(crate) fn time_left_in_channel(&self, name: &FastString36) -> Result<f32, E> {
        let Some((length, time)) = self.channel_length_and_time(name)? else {
            return Ok(0.0);
        };
        let left = length - time;
        Ok(select(left, left, 0.0))
    }
}

#[cfg(test)]
#[path = "actor_animation_channel_transition_tests.rs"]
mod tests;
