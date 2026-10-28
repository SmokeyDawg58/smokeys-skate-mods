//! `PlayWipeoutRecoverAnim::Begin` 0x82cbc4c0 on retained Actor owners.

use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::{
    play_animation::FastString36,
    play_animation_lifecycle::{
        AnimationDispatch, PlayAnimationLifecycleBackend, PlayAnimationRuntimeError,
    },
};
use std::sync::Arc;

use super::play_animation::PlayAnimationActor;

impl ActorConstruction {
    /// `PlayWipeoutRecoverAnim::Begin` 0x82cbc4c0. The transition length is
    /// the behaviour object's `transTime`, rather than a Motion override.
    pub(crate) fn motion_runtime_play_wipeout_recover_anim(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        transition_length: f32,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;

        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(E::ForeignOwner("PlayWipeoutRecoverAnim IActor"));
        }

        // Native Begin resolves all four interfaces before asking IPhysical
        // for output. Keep the individual checks in its lookup order.
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = {
            let actor = self.prefix().map_err(E::Actor)?;
            let owner = actor
                .animation
                .as_ref()
                .filter(|_| actor.animation_published)
                .ok_or(E::MissingOwner("PlayWipeoutRecoverAnim ISkaterAnim"))?;
            let retained = owner.lock().map_err(|_| E::Lock)?;
            if !retained
                .entity()
                .is_same_issuance(&self.authority.entity_handle())
                || !actor.animation_interfaces[2]
                    .as_ref()
                    .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(E::ForeignOwner("PlayWipeoutRecoverAnim ISkaterAnim"));
            }
            if !actor.animation_interfaces[0]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(E::ForeignOwner("PlayWipeoutRecoverAnim IAnimatable"));
            }
            let motion = actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .filter(|_| actor.motion_graph_published)
                .ok_or(E::MissingOwner("PlayWipeoutRecoverAnim IMotionGraph"))?;
            if !actor.graph_interfaces[1]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
            {
                return Err(E::ForeignOwner("PlayWipeoutRecoverAnim IMotionGraph"));
            }
            Arc::clone(owner)
        };
        self.check_motion_runtime_context(context)?;

        // These are two source IPhysical::GetOutput calls, not one shared
        // snapshot. The second observes the twist independently.
        let lean = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::PhysOut)?
            .skeleton_lean;
        let twist = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::PhysOut)?
            .skeleton_twist;
        let mirrored = {
            let retained = animation.lock().map_err(|_| E::Lock)?;
            let player = retained
                .player()
                .ok_or(E::MissingOwner("PlayWipeoutRecoverAnim SkaterAnim player"))?;
            if player.flags_known_mask & 0x4000_0000 == 0 {
                return Err(E::MissingOwner("PlayWipeoutRecoverAnim mirrored flag"));
            }
            player.flags_bits & 0x4000_0000 != 0
        };
        let signed_twist = if mirrored { -twist } else { twist };

        let selected = self
            .wipeout_recovery_graph()?
            .select(lean, signed_twist)
            .ok_or(E::MissingOwner("SkaterManager recovery graph point"))?;
        let mut backend = PlayAnimationActor {
            actor: self,
            context,
        };
        let blend = backend.dispatch(AnimationDispatch::Blend {
            name: selected,
            blend_length_bits: transition_length.to_bits(),
            playback_speed_bits: 1.0f32.to_bits(),
            time_bits: 0.0f32.to_bits(),
            blend_with_current_frame: false,
            placement_raw: 0,
        });
        // Source clears the next override before inspecting the blend result.
        // Keep this call even if the host dispatch itself reports an error.
        let cleared = backend.clear_transition_override();
        let blended = blend?;
        cleared?;
        if blended == 0 {
            let fallback = backend.dispatch(AnimationDispatch::Play {
                name: FastString36::encode("KeepDefaultAnim")
                    .expect("source fallback animation name"),
                playback_speed_bits: 1.0f32.to_bits(),
                time_bits: 0.0f32.to_bits(),
            })?;
            if fallback == 0 {
                return Err(E::PlayAnimation(Box::new(
                    PlayAnimationRuntimeError::FallbackDispatchFailed,
                )));
            }
        }
        Ok(())
    }
}
