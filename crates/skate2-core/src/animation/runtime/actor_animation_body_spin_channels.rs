//! `BodySpin`'s actual channel overloads; child of `actor_animation_construction`.
use super::{ActorAnimationConstruction, gesture_writes::GestureAnimationWriteError};
use crate::play_animation::FastString36;

impl<K, W> ActorAnimationConstruction<K, W> {
    /// ChannelAnimatable::EndAllChannelsPrematurely82d74770. The native
    /// dictionary traversal and each channel's zero blend-out-time gate match
    /// the named overload; retain channels until the normal expiry update.
    pub(crate) fn try_end_all_channels_prematurely(
        &mut self,
        length: f32,
        from_current_frame: bool,
    ) -> Result<(), GestureAnimationWriteError> {
        let names = self.initialization.as_ref()
            .and_then(|i| i.channels.as_ref())
            .ok_or(GestureAnimationWriteError::MissingInitialization)?
            .dictionary.keys().copied().collect::<Vec<_>>();
        for name in names {
            self.try_end_channel_with_blend(&name, length, from_current_frame)?;
        }
        Ok(())
    }
    /// `EndChannelPrematurely82d7d648`, `BodySpin` always passes current-frame=true.
    pub(crate) fn try_end_body_spin_channel(
        &mut self,
        name: &FastString36,
        length: f32,
    ) -> Result<(), GestureAnimationWriteError> {
        self.try_end_channel_with_blend(name, length, true)
    }

    /// The same native overload also serves DisallowPumping, which passes
    /// false in r6. Replace the current-frame bit rather than only setting it.
    pub(crate) fn try_end_channel_with_blend(
        &mut self,
        name: &FastString36,
        length: f32,
        from_current_frame: bool,
    ) -> Result<(), GestureAnimationWriteError> {
        use GestureAnimationWriteError as E;
        let channels = self
            .initialization
            .as_mut()
            .and_then(|i| i.channels.as_mut())
            .ok_or(E::MissingInitialization)?;
        let Some(&index) = channels.dictionary.get(name) else {
            return Ok(());
        };
        let channel = channels
            .channels
            .get_mut(index)
            .ok_or(E::InvalidChannelIndex)?;
        if channel
            .blend_out_time
            .ok_or(E::UnknownChannelField("blend out time"))?
            == 0.0
        {
            // Source loads flags, then writes length, then writes flags.
            let flags = channel
                .channel_flags
                .ok_or(E::UnknownChannelField("flags"))?;
            channel.blend_out_length = Some(length);
            channel.channel_flags = Some((flags & !0x4000_0000) | 0x2800_0000
                | if from_current_frame { 0x4000_0000 } else { 0 });
            if let Some(runtime) = &mut channel.runtime {
                runtime.blend_out_last_frame = from_current_frame;
                runtime.blending_out = true;
                runtime.blend_out_early = true;
            }
        }
        Ok(())
    }
    /// `SetChannelInfluence82d7d760`; no clamp or NaN rejection. Missing is false.
    pub(crate) fn try_set_body_spin_influence(
        &mut self,
        name: &FastString36,
        value: f32,
    ) -> Result<bool, GestureAnimationWriteError> {
        use GestureAnimationWriteError as E;
        let channels = self
            .initialization
            .as_mut()
            .and_then(|i| i.channels.as_mut())
            .ok_or(E::MissingInitialization)?;
        let Some(&index) = channels.dictionary.get(name) else {
            return Ok(false);
        };
        channels
            .channels
            .get_mut(index)
            .ok_or(E::InvalidChannelIndex)?
            .influence = Some(value);
        Ok(true)
    }
}
