//! `TransitionBlend` body Eval82d6e198 and hand/face `EvalBlendPose82d78f30`.
use super::{ActorAnimationConstruction, blend_anim::BodyAnimationReference};
use crate::{
    actor_animation_tree::eval::{AnimationEvalCommands, AnimationEvalError, AnimationEvalParams},
    actor_face_playback::blend_pose_stream::{BlendPoseBuffer, BlendPoseError, BlendPoseTokenPool},
};

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Both children receive the SAME doEval, even at blend endpoints. Native
    /// length assertion follows both calls; evaluated publishes after commands.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub(crate) fn eval_body_reference<C: AnimationEvalCommands>(
        &mut self,
        reference: &BodyAnimationReference,
        params: &AnimationEvalParams,
        do_eval: bool,
        commands: &mut C,
    ) -> Result<bool, AnimationEvalError<C::Error>> {
        use AnimationEvalError as E;
        match reference {
            BodyAnimationReference::Tree(root) => self
                .initialization
                .as_mut()
                .and_then(|state| state.tree_pools.get_mut(root.pool_index))
                .ok_or(E::MissingOwner("body tree pool"))?
                .runtime
                .eval(&root.node, params, do_eval, commands),
            BodyAnimationReference::TransitionBlend(index) => {
                let coefficient = self
                    .try_transition_blend_weight(*index)
                    .map_err(|_| E::MissingOwner("retained TransitionBlend weight"))?;
                let runtime = self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .retained_blend(*index)
                    .map_err(|_| E::MissingOwner("retained TransitionBlend"))?
                    .clone();
                let first = self.eval_body_reference(&runtime.from, params, do_eval, commands)?;
                let second = self.eval_body_reference(&runtime.to, params, do_eval, commands)?;
                // Read length AFTER child calls. e210 BO12/BI25 skips the
                // assertion only for ordered GT; unordered must fail too.
                let blend = self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .blends
                    .get(*index)
                    .ok_or(E::MissingOwner("TransitionBlend slot"))?;
                if !(blend.length > 0.0) {
                    return Err(E::MissingOwner("TransitionBlend positive length assertion"));
                }
                if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                    eprintln!(
                        "known air: transition blend index={index} coefficient={coefficient} time={} length={} do_eval={do_eval} first={first} second={second}",
                        blend.time, blend.length
                    );
                }
                if do_eval && first && second {
                    // e250 samples flags only after both child evaluations.
                    let flags = blend
                        .runtime
                        .as_ref()
                        .ok_or(E::MissingOwner("TransitionBlend runtime"))?
                        .flags;
                    if flags & 0x8000_0000 == 0 {
                        commands.blend(coefficient).map_err(E::Command)?;
                    } else {
                        commands
                            .channel_blend(coefficient, flags & 0x4000_0000 != 0)
                            .map_err(E::Command)?;
                    }
                }
                let result = first || second;
                self.base
                    .as_mut()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .blends
                    .get_mut(*index)
                    .ok_or(E::MissingOwner("TransitionBlend slot"))?
                    .evaluated = result;
                Ok(result)
            }
            BodyAnimationReference::TransitionSequence(index) => {
                let active = {
                    let store = &self
                        .base
                        .as_ref()
                        .ok_or(E::MissingOwner("Animatable"))?
                        .transition_store;
                    let runtime = store
                        .retained_sequence(*index)
                        .map_err(|_| E::MissingOwner("retained Sequence"))?;
                    let playing_new = store
                        .sequences
                        .get(*index)
                        .ok_or(E::MissingOwner("Sequence slot"))?
                        .playing_new_animation;
                    (if playing_new {
                        &runtime.to
                    } else {
                        &runtime.from
                    })
                    .clone()
                };
                self.eval_body_reference(&active, params, do_eval, commands)
            }
        }
    }

    /// Token evaluation differs from body Eval: positive length asserts first,
    /// and coefficient endpoint gates select which streams are visited.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub(crate) fn eval_body_blend_pose(
        &self,
        reference: &BodyAnimationReference,
        buffer: &mut BlendPoseBuffer,
        pool: &mut BlendPoseTokenPool,
    ) -> Result<(), BlendPoseError> {
        use BlendPoseError as E;
        match reference {
            BodyAnimationReference::Tree(root) => self
                .initialization
                .as_ref()
                .and_then(|state| state.tree_pools.get(root.pool_index))
                .ok_or(E::Missing("body tree pool"))?
                .runtime
                .eval_blend_pose(&root.node, buffer, pool),
            BodyAnimationReference::TransitionBlend(index) => {
                let store = &self
                    .base
                    .as_ref()
                    .ok_or(E::Missing("Animatable"))?
                    .transition_store;
                let runtime = store
                    .retained_blend(*index)
                    .map_err(|_| E::Missing("retained TransitionBlend"))?;
                let blend = store
                    .blends
                    .get(*index)
                    .ok_or(E::Missing("TransitionBlend slot"))?;
                if !(blend.length > 0.0) {
                    return Err(E::Missing("TransitionBlend positive length assertion"));
                }
                let coefficient = self
                    .try_transition_blend_weight(*index)
                    .map_err(|_| E::Missing("retained TransitionBlend weight"))?;
                // 78f9c=40980060 tests !LT, including unordered.
                if !(coefficient < 1.0) {
                    return self.eval_body_blend_pose(&runtime.to, buffer, pool);
                }
                self.eval_body_blend_pose(&runtime.from, buffer, pool)?;
                // 78fb8=40990058 skips for !GT, including unordered.
                if coefficient > 0.0 {
                    let mut temporary = pool.buffer(buffer.pool_index.wrapping_sub(1) & 1)?;
                    self.eval_body_blend_pose(&runtime.to, &mut temporary, pool)?;
                    pool.blend(buffer, &temporary, coefficient)?;
                }
                Ok(())
            }
            BodyAnimationReference::TransitionSequence(index) => {
                let store = &self
                    .base
                    .as_ref()
                    .ok_or(E::Missing("Animatable"))?
                    .transition_store;
                let runtime = store
                    .retained_sequence(*index)
                    .map_err(|_| E::Missing("retained Sequence"))?;
                let sequence = store
                    .sequences
                    .get(*index)
                    .ok_or(E::Missing("Sequence slot"))?;
                self.eval_body_blend_pose(
                    if sequence.playing_new_animation {
                        &runtime.to
                    } else {
                        &runtime.from
                    },
                    buffer,
                    pool,
                )
            }
        }
    }
}
