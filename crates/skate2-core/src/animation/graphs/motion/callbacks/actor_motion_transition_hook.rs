//! `OverideNextAnimTransitionHook::OnTransition` 0x82ca6be8 writes the
//! retained MotionGraph override through virtual +0x18 (0x825fdf20).
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::generated_motion_load::hooks::HookPayload;
use std::sync::Arc;

impl ActorConstruction {
    /// `MotionGraph::NextAnimTransOveride` 0x825fdf20 copies the four-word
    /// TransitionParams to MotionGraph+0xa68 in type, length, placement, flag
    /// order. The hook constructor establishes only the flag word's top bits.
    pub(crate) fn motion_apply_transition_hook(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &HookPayload,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let HookPayload::OverrideNextAnimationTransition {
            transition_type,
            blend_length,
            blend_with_current_frame,
            transition_under,
            use_channel_from_weights,
        } = payload
        else {
            return Err(E::MissingOwner("OverrideNextAnimationTransition hook"));
        };
        self.check_motion_runtime_context(context)?;
        let mut actor = self.prefix().map_err(E::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(E::MissingOwner("hook cached IMotionGraph"))?;
        if !actor.graph_interfaces[1]
            .as_ref()
            .is_some_and(|registered| Arc::ptr_eq(registered, graph.occurrence()))
        {
            return Err(E::ForeignOwner("hook IMotionGraph"));
        }
        let destination = actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|graph| graph.transition.as_mut())
            .ok_or(E::MissingOwner("Motion transition override"))?;
        destination.transition_type = *transition_type as u32;
        destination.blend_length = *blend_length;
        destination.placement = u32::from(*transition_under);
        let mask = 0xc000_0000;
        let known_flags = (u32::from(*blend_with_current_frame) << 31)
            | (u32::from(*use_channel_from_weights) << 30);
        destination.flags.bits = (destination.flags.bits & !mask) | known_flags;
        destination.flags.known_mask |= mask;
        Ok(())
    }

    /// `MongoPushToAntic::OnTransition` 0x82cbeb00 resolves the owner's cached
    /// IChannelAnimatable (+0x1c, asserted against UCOM) and calls virtual +0xc,
    /// `ChannelAnimatable::NewChannel` 0x82d7eb98. The authored anim is both
    /// channel and animation name. tChannelParams: priority 0, cycle/mirror
    /// false, speed 1.0, blend-in 0.1 (0x3dcccccd), first-frame false,
    /// blend-out 0.07 (0x3d8f5c29), last-frame false; use-attributes false.
    /// The returned eChannelStatus is discarded.
    pub(crate) fn motion_mongo_push_to_antic(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &HookPayload,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let HookPayload::MongoPushToAntic { channel } = payload else {
            return Err(E::MissingOwner("MongoPushToAntic hook"));
        };
        self.check_motion_runtime_context(context)?;
        let animation = {
            let actor = self.prefix().map_err(E::Actor)?;
            let animation = actor
                .animation
                .as_ref()
                .filter(|_| actor.animation_published)
                .ok_or(E::MissingOwner("hook cached IChannelAnimatable"))?;
            let retained = animation.lock().map_err(|_| E::Lock)?;
            if !actor.animation_interfaces[1]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, retained.occurrence()))
            {
                return Err(E::ForeignOwner("hook IChannelAnimatable"));
            }
            if !retained
                .entity()
                .is_same_issuance(&self.authority.entity_handle())
            {
                return Err(E::ForeignOwner("hook animation entity"));
            }
            drop(retained);
            Arc::clone(animation)
        };
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_new_channel_with_blends(*channel, *channel, 0.1, 0.07, false)
            .map(|_| ())
            .map_err(E::AnimationChannel)
    }
}
