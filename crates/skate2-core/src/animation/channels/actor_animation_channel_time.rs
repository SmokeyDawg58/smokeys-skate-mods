//! Actual `ChannelBlend` time advancement, with checked transition boundaries.
use super::super::{ActorAnimationConstruction, AnimationPointerState};
use super::{ChannelBaseReference, GestureChannelError};
use crate::actor_animation_tree::{AnimationTreeError, time::AnimationProperties};

// PPC FSEL chooses its nonnegative operand for either signed zero, and its
// negative operand for unordered. Rust min/max have different NaN semantics.
fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
fn clamp_unit(value: f32) -> f32 {
    let lower = select(-value, 0.0, value);
    select(1.0 - lower, lower, 1.0)
}
fn field(value: Option<f32>) -> Result<f32, GestureChannelError> {
    value.ok_or(GestureChannelError::MissingOwner("channel time field"))
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// `ChannelAnimatable::AddTime82c8a558` and `Animatable::AddTime82592b58`.
    pub(crate) fn try_add_animation_time(&mut self, dt: f32) -> Result<(), GestureChannelError> {
        use GestureChannelError as E;
        self.try_update_channel_transitions()?;
        let initialized = self
            .initialization
            .as_ref()
            .ok_or(E::MissingOwner("Initialize"))?;
        let channels = initialized
            .channels
            .as_ref()
            .ok_or(E::MissingOwner("channels"))?;
        // Inline transitions completed above, before expiry and this frame dt.
        // Both passes use native dictionary traversal order.
        if channels.root.is_some() {
            // Native iterator saves its successor before each erase. No channel
            // insertion occurs here; names retain that order while predicates
            // are sampled only at each channel's turn.
            let names = channels.dictionary.keys().copied().collect::<Vec<_>>();
            for name in names {
                let channels = self
                    .initialization
                    .as_ref()
                    .and_then(|state| state.channels.as_ref())
                    .ok_or(E::MissingOwner("channels"))?;
                let index = *channels.dictionary.get(&name).ok_or(E::InvalidChannel)?;
                let channel = channels.channels.get(index).ok_or(E::InvalidChannel)?;
                let runtime = channel
                    .runtime
                    .as_ref()
                    .ok_or(E::MissingOwner("channel runtime"))?;
                let remove = if runtime.blending_out {
                    field(channel.blend_out_time)?.partial_cmp(&field(channel.blend_out_length)?)
                        != Some(std::cmp::Ordering::Less)
                } else {
                    !runtime.cycle && runtime.expired
                };
                if remove {
                    self.try_remove_channel(&name)?;
                }
            }
        }
        self.try_update_body_transitions()?;
        let channel_root = self
            .initialization
            .as_ref()
            .and_then(|state| state.channels.as_ref())
            .ok_or(E::MissingOwner("channels"))?
            .root;
        let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
        base.is_playing = false;
        base.expired_or_looped = false;
        base.expired_or_looped_by_time = -1.0;
        base.time_to_expire_or_loop = -1.0;
        let current = if let Some(root) = channel_root {
            ChannelBaseReference::Channel(root)
        } else if let Some(root) = &base.body_root {
            ChannelBaseReference::Body(root.clone())
        } else if base.base_animation == AnimationPointerState::Null {
            return Ok(());
        } else {
            return Err(E::MissingOwner("body root"));
        };
        let mut properties = AnimationProperties::default();
        let result = self.add_reference_time(&current, dt, &mut properties);
        let base = self.base.as_mut().ok_or(E::MissingOwner("Animatable"))?;
        base.expired_or_looped = properties.expired_or_looped;
        base.expired_or_looped_by_time = properties.expired_by;
        base.time_to_expire_or_loop = properties.time_to_expire;
        if result.is_ok() {
            properties.is_playing = true;
        }
        base.is_playing = properties.is_playing;
        result
    }

    fn add_reference_time(
        &mut self,
        reference: &ChannelBaseReference,
        dt: f32,
        properties: &mut AnimationProperties,
    ) -> Result<(), GestureChannelError> {
        use GestureChannelError as E;
        match reference {
            ChannelBaseReference::Empty => Err(E::MissingOwner("ChannelBlend base animation")),
            ChannelBaseReference::Channel(index) => self.add_channel_time(*index, dt, properties),
            ChannelBaseReference::Body(root) => self.add_body_reference_time(root, dt, properties),
        }
    }

    /// `ChannelBlend::AddTime82d6d610`: base properties survive; channel uses a local.
    fn add_channel_time(
        &mut self,
        index: usize,
        dt: f32,
        properties: &mut AnimationProperties,
    ) -> Result<(), GestureChannelError> {
        use GestureChannelError as E;
        if dt < 0.0 {
            return Err(E::Tree(AnimationTreeError::NegativeTime));
        }
        let channels = self
            .initialization
            .as_ref()
            .and_then(|i| i.channels.as_ref())
            .ok_or(E::MissingOwner("channels"))?;
        let reference = channels
            .channels
            .get(index)
            .and_then(|c| c.runtime.as_ref())
            .ok_or(E::InvalidChannel)?
            .base
            .clone();
        self.add_reference_time(&reference, dt, properties)?;
        let channels = self
            .initialization
            .as_mut()
            .and_then(|i| i.channels.as_mut())
            .ok_or(E::MissingOwner("channels"))?;
        let channel = channels.channels.get_mut(index).ok_or(E::InvalidChannel)?;
        let in_length = field(channel.blend_in_length)?;
        let in_time = field(channel.blend_in_time)?;
        let mut fade_in = 1.0;
        if in_length > 0.0 && in_time < in_length {
            let fraction = in_time / in_length;
            channel.blend_in_time = Some(in_time + dt);
            fade_in = clamp_unit(fraction);
        }
        let (tree, animation) = super::transition::target(channel)?;
        let pool = &channels.trees.get(tree).ok_or(E::InvalidChannel)?.runtime;
        let length = pool.length(&animation).map_err(E::Tree)?;
        let remaining = length - pool.time(&animation).map_err(E::Tree)?;
        let runtime = channel
            .runtime
            .as_mut()
            .ok_or(E::MissingOwner("channel runtime"))?;
        let remaining = select(remaining, remaining, 0.0);
        let out_length = field(channel.blend_out_length)?;
        let mut fade_out = 1.0;
        let endpoint = runtime.blend_out_last_frame && runtime.expired;
        // Raw bge skips this condition for unordered remaining/length as well.
        let approaching = !runtime.blend_out_last_frame && remaining < out_length;
        if out_length > 0.0
            && (runtime.blend_out_early || (!runtime.cycle && (endpoint || approaching)))
        {
            let out_time = field(channel.blend_out_time)?;
            let fraction = out_time / out_length;
            channel.blend_out_time = Some(out_time + dt);
            fade_out = clamp_unit(1.0 - fraction);
        }
        let resurrect_length = field(channel.resurrect_length)?;
        let resurrect_time = field(channel.resurrect_time)?;
        let mut resurrect = 1.0;
        if resurrect_length > 0.0 && resurrect_time < resurrect_length {
            let fraction = resurrect_time / resurrect_length;
            channel.resurrect_time = Some(resurrect_time + dt);
            resurrect = clamp_unit(fraction);
        }
        let fade = select(fade_in - fade_out, fade_out, fade_in);
        let fade = select(fade - resurrect, resurrect, fade);
        channel.blend_coefficient = Some(clamp_unit(fade * field(channel.influence)?));
        let hold_first = runtime.blend_in_first_frame && field(channel.blend_in_time)? < in_length;
        // Raw ble tests GT false; unordered blend-out time does not hold.
        let hold_last = runtime.blend_out_last_frame && field(channel.blend_out_time)? > 0.0;
        if !hold_first && !hold_last {
            let mut local = AnimationProperties::default();
            super::transition::add_time(channel, &mut channels.trees, dt, &mut local)?;
            channel
                .runtime
                .as_mut()
                .ok_or(E::MissingOwner("channel runtime"))?
                .expired = local.expired_or_looped;
            let flags = channel
                .channel_flags
                .as_mut()
                .ok_or(E::MissingOwner("channel flags"))?;
            *flags = (*flags & !0x0400_0000) | (u32::from(local.expired_or_looped) << 26);
        }
        Ok(())
    }
}
