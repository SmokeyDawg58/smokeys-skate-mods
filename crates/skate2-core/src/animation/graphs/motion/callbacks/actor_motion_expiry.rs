//! Same-Actor adapter for the existing `WillExpire` activation algorithm.
use super::{
    ActorConstruction, ActorIdentity, MotionUpdateError, play_animation::PlayAnimationActor,
};
use crate::{
    motion_graph_conditions::{
        AnimProperty, WillExpireActivationError, WillExpireCollaborator, WillExpireCondition,
    },
    motion_graph_entity_authority::BoundMotionGraphContext,
    play_animation_lifecycle::{
        PlayAnimationInterfaceKind, PlayAnimationInterfaceResolution, PlayAnimationLifecycleBackend,
    },
};
use std::sync::{Arc, Mutex};
type Animation = super::super::animation_mount::MountedActorAnimation;

struct AnimationOwner(Arc<Mutex<Animation>>);
impl PartialEq for AnimationOwner {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
struct ExpiryActor<'a>(PlayAnimationActor<'a>);
impl WillExpireCollaborator for ExpiryActor<'_> {
    type Error = MotionUpdateError;
    type AnimInfo = ();
    type Animatable = AnimationOwner;
    fn find_anim_info(&mut self) -> Result<Option<()>, Self::Error> {
        // Nonempty tags require the actual optional IAnimInfo lookup, not a
        // fabricated missing interface. The admitted original condition19 has
        // only InTime=0.1 and never enters this branch.
        Err(MotionUpdateError::PendingAnimationOperation(
            "WillExpire IAnimInfo tag lookup",
        ))
    }
    fn resolve_special_tag(&mut self, (): &(), _: &[u8]) -> Result<f32, Self::Error> {
        Err(MotionUpdateError::PendingAnimationOperation(
            "WillExpire special tag",
        ))
    }
    fn cached_animatable(&mut self) -> Result<Option<AnimationOwner>, Self::Error> {
        self.0.animation().map(|owner| Some(AnimationOwner(owner)))
    }
    fn find_animatable(&mut self) -> Result<Option<AnimationOwner>, Self::Error> {
        if self
            .0
            .resolve_interface(PlayAnimationInterfaceKind::Animatable)?
            != PlayAnimationInterfaceResolution::Resolved
        {
            return Err(MotionUpdateError::ForeignOwner("IAnimatable"));
        }
        self.0.animation().map(|owner| Some(AnimationOwner(owner)))
    }
    fn transitioning(&mut self, owner: &AnimationOwner) -> Result<bool, Self::Error> {
        let animation = owner.0.lock().map_err(|_| MotionUpdateError::Lock)?;
        animation.try_is_transitioning().map_err(|error| match error {
            crate::actor_animation_construction::play_anim::PlayBodyAnimationError::MissingOwner(owner) =>
                MotionUpdateError::MissingOwner(owner),
            _ => MotionUpdateError::ForeignOwner("base animation in transition list"),
        })
    }
    fn anim_property(
        &mut self,
        owner: &AnimationOwner,
    ) -> Result<Option<AnimProperty>, Self::Error> {
        let animation = owner.0.lock().map_err(|_| MotionUpdateError::Lock)?;
        let base = animation
            .base()
            .ok_or(MotionUpdateError::MissingOwner("Animatable"))?;
        // GetAnimProp82592bf8 returns the actual retained AddTime properties.
        Ok(Some(AnimProperty {
            is_playing_raw: u8::from(base.is_playing),
            has_expired_or_looped_raw: u8::from(base.expired_or_looped),
            has_expired_or_looped_by_time: base.expired_or_looped_by_time,
            time_to_expire_or_loop: base.time_to_expire_or_loop,
        }))
    }
}
impl ActorConstruction {
    /// `WillExpire82c9bc78` on the generated condition's retained payload.
    pub(crate) fn motion_runtime_will_expire(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        time: f32,
        wait: bool,
        tag: &str,
    ) -> Result<bool, MotionUpdateError> {
        self.check_motion_runtime_context(context)?;
        WillExpireCondition::activate_values(
            time,
            tag.as_bytes(),
            u8::from(wait),
            &mut ExpiryActor(PlayAnimationActor {
                actor: self,
                context,
            }),
        )
        .map_err(|error| match error {
            WillExpireActivationError::Backend(error) => error,
            WillExpireActivationError::AnimatableIdentityMismatch => {
                MotionUpdateError::ForeignOwner("IAnimatable")
            }
            WillExpireActivationError::MissingAnimatable => {
                MotionUpdateError::MissingOwner("IAnimatable")
            }
            WillExpireActivationError::MissingAnimProperty => {
                MotionUpdateError::MissingOwner("AnimProperty")
            }
        })
    }

    /// `IsTransitioning::ActivationFunc`82c9bf08: the Actor's cached
    /// IAnimatable (asserted), then `Animatable.Transitioning`82d720f0.
    pub(crate) fn motion_is_transitioning_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        self.check_motion_runtime_context(context)?;
        let mut actor = ExpiryActor(PlayAnimationActor {
            actor: self,
            context,
        });
        let owner = actor
            .find_animatable()?
            .ok_or(MotionUpdateError::MissingOwner("IAnimatable"))?;
        actor.transitioning(&owner)
    }

    /// `InTimeWindow::ActivationFunc`82c9bdd0: a transitioning Animatable is
    /// false before GetBaseAnim (+0x28); otherwise base GetTime (+0x18) in
    /// [StartTime, WindowFrameLength + StartTime] by `blt`/`ble`.
    pub(crate) fn motion_in_time_window_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        start: f32,
        frame_length: f32,
    ) -> Result<bool, MotionUpdateError> {
        self.check_motion_runtime_context(context)?;
        let mut actor = ExpiryActor(PlayAnimationActor {
            actor: self,
            context,
        });
        let owner = actor
            .find_animatable()?
            .ok_or(MotionUpdateError::MissingOwner("IAnimatable"))?;
        if actor.transitioning(&owner)? {
            return Ok(false);
        }
        let animation = owner.0.lock().map_err(|_| MotionUpdateError::Lock)?;
        let root = animation
            .base()
            .ok_or(MotionUpdateError::MissingOwner("Animatable"))?
            .body_root
            .clone()
            .ok_or(MotionUpdateError::MissingOwner("base animation"))?;
        let time = animation
            .body_reference_clock(&root, true)
            .map_err(MotionUpdateError::AnimationBody)?;
        Ok(in_time_window(time, start, frame_length))
    }
}

/// 82c9bebc..bed8: an unordered time fails the upper `ble`.
fn in_time_window(time: f32, start: f32, frame_length: f32) -> bool {
    if time < start {
        false
    } else {
        time <= frame_length + start
    }
}

#[cfg(test)]
mod tests {
    use super::in_time_window;

    #[test]
    fn window_is_inclusive_and_unordered_time_is_outside() {
        assert!(in_time_window(0.0, 0.0, 0.3));
        assert!(in_time_window(0.3, 0.0, 0.3));
        assert!(!in_time_window(0.31, 0.0, 0.3));
        assert!(!in_time_window(-0.01, 0.0, 0.3));
        assert!(!in_time_window(f32::NAN, 0.0, 0.3));
    }
}
