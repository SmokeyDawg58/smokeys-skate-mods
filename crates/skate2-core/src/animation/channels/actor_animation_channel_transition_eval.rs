//! Inline channel TransitionBlend uses the same native math and commands as
//! body transitions, but resolves both children from the channel's own pools.
use super::{Channel, OwnedAnimationPool, attribute_target, is_sequence, target};
use crate::{
    actor_animation_construction::transition_time::transition_blend_weight,
    actor_animation_tree::eval::{AnimationEvalCommands, AnimationEvalError, AnimationEvalParams},
    actor_face_playback::blend_pose_stream::{BlendPoseBuffer, BlendPoseError, BlendPoseTokenPool},
};

/// TransitionBlend.Eval82d6e198: both children even at weight endpoints;
/// positive-length assertion follows children and evaluated follows commands.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub(crate) fn eval<C: AnimationEvalCommands>(
    channel: &mut Channel,
    trees: &mut [OwnedAnimationPool],
    params: &AnimationEvalParams,
    do_eval: bool,
    commands: &mut C,
) -> Result<bool, AnimationEvalError<C::Error>> {
    use AnimationEvalError as E;
    if is_sequence(channel).map_err(|_| E::MissingOwner("channel transition kind"))? {
        // Sequence.Eval82d6dd00 tail-calls FROM until PlayingNewAnim, then TO.
        let (tree, node) =
            attribute_target(channel).map_err(|_| E::MissingOwner("channel Sequence child"))?;
        return trees
            .get_mut(tree)
            .ok_or(E::MissingOwner("channel pool"))?
            .runtime
            .eval(&node, params, do_eval, commands);
    }
    let (to_tree, to) = target(channel).map_err(|_| E::MissingOwner("channel target"))?;
    if !channel.transitioning {
        return trees
            .get_mut(to_tree)
            .ok_or(E::MissingOwner("channel pool"))?
            .runtime
            .eval(&to, params, do_eval, commands);
    }
    let weight = transition_blend_weight(
        channel.transition_blend.time,
        channel.transition_blend.length,
    );
    let from_tree = channel
        .channel_tree
        .ok_or(E::MissingOwner("channel FROM pool"))?;
    let from = channel
        .runtime
        .as_ref()
        .ok_or(E::MissingOwner("channel runtime"))?
        .animation
        .clone();
    let first = trees
        .get_mut(from_tree)
        .ok_or(E::MissingOwner("channel FROM pool"))?
        .runtime
        .eval(&from, params, do_eval, commands)?;
    let second = trees
        .get_mut(to_tree)
        .ok_or(E::MissingOwner("channel TO pool"))?
        .runtime
        .eval(&to, params, do_eval, commands)?;
    if !(channel.transition_blend.length > 0.0) {
        return Err(E::MissingOwner("TransitionBlend positive length assertion"));
    }
    if do_eval && first && second {
        commands.blend(weight).map_err(E::Command)?;
    }
    channel.transition_blend.evaluated = first || second;
    Ok(first || second)
}

/// EvalBlendPose82d78f30: length first; TO-only at >=1/unordered, FROM
/// otherwise, optional TO stream in alternate token buffer only for weight>0.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub(crate) fn eval_blend_pose(
    channel: &Channel,
    trees: &[OwnedAnimationPool],
    buffer: &mut BlendPoseBuffer,
    pool: &mut BlendPoseTokenPool,
) -> Result<(), BlendPoseError> {
    use BlendPoseError as E;
    if is_sequence(channel).map_err(|_| E::Missing("channel transition kind"))? {
        // Sequence.EvalBlendPose82d6dd38 tail-calls FROM until PlayingNewAnim.
        let (tree, node) =
            attribute_target(channel).map_err(|_| E::Missing("channel Sequence child"))?;
        return trees
            .get(tree)
            .ok_or(E::Missing("channel pool"))?
            .runtime
            .eval_blend_pose(&node, buffer, pool);
    }
    if channel.transitioning && !(channel.transition_blend.length > 0.0) {
        return Err(E::Missing("TransitionBlend positive length assertion"));
    }
    let (to_tree, to) = target(channel).map_err(|_| E::Missing("channel target"))?;
    let weight = if channel.transitioning {
        transition_blend_weight(
            channel.transition_blend.time,
            channel.transition_blend.length,
        )
    } else {
        1.0
    };
    // 78f9c BO4/BI24 is LT-clear, including unordered.
    if !(weight < 1.0) {
        return trees
            .get(to_tree)
            .ok_or(E::Missing("channel TO pool"))?
            .runtime
            .eval_blend_pose(&to, buffer, pool);
    }
    let from_tree = channel
        .channel_tree
        .ok_or(E::Missing("channel FROM pool"))?;
    let from = &channel
        .runtime
        .as_ref()
        .ok_or(E::Missing("channel runtime"))?
        .animation;
    trees
        .get(from_tree)
        .ok_or(E::Missing("channel FROM pool"))?
        .runtime
        .eval_blend_pose(from, buffer, pool)?;
    if weight > 0.0 {
        let mut temporary = pool.buffer(buffer.pool_index.wrapping_sub(1) & 1)?;
        trees
            .get(to_tree)
            .ok_or(E::Missing("channel TO pool"))?
            .runtime
            .eval_blend_pose(&to, &mut temporary, pool)?;
        pool.blend(buffer, &temporary, weight)?;
    }
    Ok(())
}
