//! Native transition pruning and recursive time advancement on retained owners.
use super::{
    ActorAnimationConstruction, AnimationTransitionHandle, blend_anim::BodyAnimationReference,
    channel_runtime::GestureChannelError as E, play_anim::PlayBodyAnimationError,
};
use crate::actor_animation_tree::{AnimationTreeError, time::AnimationProperties};

fn producer_error(error: PlayBodyAnimationError) -> E {
    match error {
        PlayBodyAnimationError::MissingOwner(owner)
        | PlayBodyAnimationError::UnsupportedDispatch(owner) => E::MissingOwner(owner),
        PlayBodyAnimationError::InvalidStore => E::InvalidChannel,
        PlayBodyAnimationError::Lock => E::Lock,
        PlayBodyAnimationError::Tree(error) => E::Tree(error),
        PlayBodyAnimationError::BindPose(error) => error,
    }
}

fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

/// `GetBlend82d6e090`: this does not assert positive length; callers differ.
pub(crate) fn transition_blend_weight(time: f32, length: f32) -> f32 {
    let ratio = time / length;
    let lower = select(-ratio, 0.0, ratio);
    let value = select(1.0 - lower, lower, 1.0);
    // Raw e0c8 is true-LT, so unordered length continues into smoothstep.
    if length < f32::from_bits(0x3d4c_cccd) {
        return value;
    }
    let square = value * value;
    if value > 1.0 {
        return value;
    }
    if value < 0.0 {
        return value;
    }
    let cube = square * value;
    let twice_cube = cube * 2.0;
    square.mul_add(3.0, -twice_cube)
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// `UpdateTransitions82d7aef0`, before property resets and this frame's dt.
    pub(crate) fn try_update_body_transitions(&mut self) -> Result<(), E> {
        loop {
            let store = &self
                .base
                .as_ref()
                .ok_or(E::MissingOwner("Animatable"))?
                .transition_store;
            let mut expired = None;
            // GetAnExpiredTransition82d71fc8 searches oldest first on EVERY pass.
            for entry in &store.transition_order.slots[..store.transition_order.len] {
                let handle = entry.ok_or(E::InvalidChannel)?;
                let complete = match handle {
                    AnimationTransitionHandle::Blend(index) => {
                        store.retained_blend(index).map_err(producer_error)?;
                        let blend = store.blends.get(index).ok_or(E::InvalidChannel)?;
                        // Raw TransitionComplete e080=41990008: strict GT, NaN false.
                        blend.time > blend.length
                    }
                    AnimationTransitionHandle::Sequence(index) => {
                        store.retained_sequence(index).map_err(producer_error)?;
                        store
                            .sequences
                            .get(index)
                            .ok_or(E::InvalidChannel)?
                            .playing_new_animation
                    }
                };
                if complete {
                    expired = Some(handle);
                    break;
                }
            }
            let Some(index) = expired else {
                return Ok(());
            };
            self.try_force_transition_release(index)
                .map_err(producer_error)?;
        }
    }

    pub(crate) fn try_transition_blend_weight(&self, index: usize) -> Result<f32, E> {
        let store = &self
            .base
            .as_ref()
            .ok_or(E::MissingOwner("Animatable"))?
            .transition_store;
        store.retained_blend(index).map_err(producer_error)?;
        let blend = store.blends.get(index).ok_or(E::InvalidChannel)?;
        Ok(transition_blend_weight(blend.time, blend.length))
    }

    /// Both native transition kinds forward GetLength to their TO child.
    fn body_reference_length(&self, reference: &BodyAnimationReference) -> Result<f32, E> {
        match reference {
            BodyAnimationReference::Tree(root) => self
                .initialization
                .as_ref()
                .and_then(|state| state.tree_pools.get(root.pool_index))
                .ok_or(E::InvalidChannel)?
                .runtime
                .length(&root.node)
                .map_err(E::Tree),
            BodyAnimationReference::TransitionBlend(index) => {
                let to = &self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .retained_blend(*index)
                    .map_err(producer_error)?
                    .to;
                self.body_reference_length(to)
            }
            BodyAnimationReference::TransitionSequence(index) => {
                let to = &self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .retained_sequence(*index)
                    .map_err(producer_error)?
                    .to;
                self.body_reference_length(to)
            }
        }
    }

    /// `TransitionBlend::AddTime82d6df98`; FROM properties are local, TO survive.
    pub(crate) fn add_body_reference_time(
        &mut self,
        reference: &BodyAnimationReference,
        dt: f32,
        properties: &mut AnimationProperties,
    ) -> Result<(), E> {
        match reference {
            BodyAnimationReference::Tree(root) => self
                .initialization
                .as_mut()
                .ok_or(E::MissingOwner("Initialize"))?
                .tree_pools
                .get_mut(root.pool_index)
                .ok_or(E::InvalidChannel)?
                .runtime
                .add_time(&root.node, dt, properties)
                .map_err(E::Tree),
            BodyAnimationReference::TransitionBlend(index) => {
                // dfc8 BO4/BI24 skips assert for !LT, including unordered dt.
                if dt < 0.0 {
                    return Err(E::Tree(AnimationTreeError::NegativeTime));
                }
                let runtime = self
                    .base
                    .as_ref()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .retained_blend(*index)
                    .map_err(producer_error)?
                    .clone();
                if runtime.flags & 0x2000_0000 == 0 {
                    let mut local = AnimationProperties::default();
                    self.add_body_reference_time(&runtime.from, dt, &mut local)?;
                }
                self.add_body_reference_time(&runtime.to, dt, properties)?;
                // e048/e050 load and commit the clock only after both child calls.
                let blend = self
                    .base
                    .as_mut()
                    .ok_or(E::MissingOwner("Animatable"))?
                    .transition_store
                    .blends
                    .get_mut(*index)
                    .ok_or(E::InvalidChannel)?;
                blend.time += dt;
                Ok(())
            }
            BodyAnimationReference::TransitionSequence(index) => {
                if dt < 0.0 {
                    return Err(E::Tree(AnimationTreeError::NegativeTime));
                }
                let (runtime, playing_new) = {
                    let store = &self
                        .base
                        .as_ref()
                        .ok_or(E::MissingOwner("Animatable"))?
                        .transition_store;
                    (
                        store
                            .retained_sequence(*index)
                            .map_err(producer_error)?
                            .clone(),
                        store
                            .sequences
                            .get(*index)
                            .ok_or(E::InvalidChannel)?
                            .playing_new_animation,
                    )
                };
                if playing_new {
                    return self.add_body_reference_time(&runtime.to, dt, properties);
                }
                let mut local = AnimationProperties::default();
                self.add_body_reference_time(&runtime.from, dt, &mut local)?;
                if local.expired_or_looped {
                    self.base
                        .as_mut()
                        .ok_or(E::MissingOwner("Animatable"))?
                        .transition_store
                        .sequences
                        .get_mut(*index)
                        .ok_or(E::InvalidChannel)?
                        .playing_new_animation = true;
                    if !(local.expired_by > 0.0) {
                        return Err(E::InvalidChannel);
                    }
                    if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                        eprintln!(
                            "known air: sequence switch expired_by={} ({:#010x})",
                            local.expired_by,
                            local.expired_by.to_bits()
                        );
                    }
                    self.add_body_reference_time(&runtime.to, local.expired_by, properties)
                } else {
                    if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some()
                        && local.time_to_expire < 0.05
                    {
                        eprintln!(
                            "known air: sequence from time_to_expire={} ({:#010x})",
                            local.time_to_expire,
                            local.time_to_expire.to_bits()
                        );
                    }
                    properties.expired_or_looped = false;
                    properties.time_to_expire =
                        self.body_reference_length(&runtime.to)? + local.time_to_expire;
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::transition_blend_weight;

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact binary fractions and FSEL endpoints"
    )]
    fn short_length_linear_threshold_smooth_and_unordered_fsel() {
        let threshold = f32::from_bits(0x3d4c_cccd);
        let below = f32::from_bits(threshold.to_bits() - 1);
        assert_eq!(transition_blend_weight(below * 0.25, below), 0.25);
        assert_eq!(
            transition_blend_weight(threshold * 0.25, threshold),
            0.15625
        );
        assert_eq!(transition_blend_weight(f32::NAN, 1.0), 1.0);
        assert_eq!(transition_blend_weight(0.0, f32::NAN), 1.0);
        assert_eq!(
            transition_blend_weight(-1.0, 1.0).to_bits(),
            0.0_f32.to_bits()
        );
        assert_eq!(
            transition_blend_weight(-0.0, 1.0).to_bits(),
            0.0_f32.to_bits()
        );
        assert_eq!(transition_blend_weight(2.0, 1.0), 1.0);
    }
}
