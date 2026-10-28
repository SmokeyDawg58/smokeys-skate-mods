//! Captured actual interfaces for `AttachIntent` and `IsProSkater`.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, Mutex};
#[path = "motion_attach_intent.rs"]
mod kernel;

struct Source<'a> {
    actor: &'a ActorConstruction,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    motion: Option<Arc<()>>,
    skater_motion: Option<Arc<()>>,
}

impl kernel::AttachIntentSource for Source<'_> {
    type Error = MotionUpdateError;
    fn resolve_animatable(&mut self) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        if let Some(animation) = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
        {
            let retained = animation.lock().map_err(|_| E::Lock)?;
            if !retained
                .entity()
                .is_same_issuance(&self.actor.authority.entity_handle())
                || !actor.animation_interfaces[0]
                    .as_ref()
                    .is_some_and(|registered| Arc::ptr_eq(registered, retained.occurrence()))
            {
                return Err(E::ForeignOwner("AttachIntent IAnimatable"));
            }
            self.animation = Some(Arc::clone(animation));
        } else if actor.animation_interfaces[0].is_some() {
            return Err(E::ForeignOwner("AttachIntent IAnimatable"));
        }
        Ok(())
    }
    fn resolve_motion(&mut self) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        if let Some(motion) = actor.graphs.as_ref().and_then(|g| g.motion()) {
            if !actor.graph_interfaces[1]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, motion.occurrence()))
            {
                return Err(E::ForeignOwner("AttachIntent IMotionGraph"));
            }
            self.motion = Some(Arc::clone(motion.occurrence()));
        } else if actor.graph_interfaces[1].is_some() {
            return Err(E::ForeignOwner("AttachIntent IMotionGraph"));
        }
        Ok(())
    }
    fn resolve_skater_motion(&mut self) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(E::MissingOwner("AttachIntent cached ISkaterMotionGraph"))?;
        if !actor.graph_interfaces[2]
            .as_ref()
            .is_some_and(|registered| Arc::ptr_eq(registered, motion.occurrence()))
        {
            return Err(E::ForeignOwner("AttachIntent ISkaterMotionGraph"));
        }
        self.skater_motion = Some(Arc::clone(motion.occurrence()));
        Ok(())
    }
    fn require_interfaces(&mut self) -> Result<(), Self::Error> {
        if self.animation.is_none() || self.motion.is_none() || self.skater_motion.is_none() {
            return Err(MotionUpdateError::MissingOwner("AttachIntent interfaces"));
        }
        Ok(())
    }
    fn intent(&mut self, name: &FastString36) -> Result<Option<f32>, Self::Error> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(E::MissingOwner("AttachIntent captured IMotionGraph"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|captured| Arc::ptr_eq(captured, motion.occurrence()))
        {
            return Err(E::ForeignOwner("AttachIntent captured IMotionGraph"));
        }
        Ok(motion
            .intents
            .as_ref()
            .ok_or(E::MissingOwner("Motion intents"))?
            .get(name))
    }
    fn append_motion_attribute(
        &mut self,
        name: FastString30,
        value: f32,
    ) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        let mut actor = self.actor.prefix().map_err(E::Actor)?;
        let motion = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(E::MissingOwner("AttachIntent captured ISkaterMotionGraph"))?;
        if !self
            .skater_motion
            .as_ref()
            .is_some_and(|captured| Arc::ptr_eq(captured, motion.occurrence()))
        {
            return Err(E::ForeignOwner("AttachIntent captured ISkaterMotionGraph"));
        }
        motion
            .state
            .as_mut()
            .ok_or(E::MissingOwner("Motion state"))?
            .try_append_motion_attribute(name, value)
    }
    fn settable_attribute(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        self.animation
            .as_ref()
            .ok_or(E::MissingOwner("AttachIntent captured IAnimatable"))?
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(E::AnimationWrite)
    }
}

impl ActorConstruction {
    pub(crate) fn motion_runtime_attach_intent(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        intent: &FastString36,
        attribute: FastString30,
        settable: bool,
    ) -> Result<(), MotionUpdateError> {
        // Generated controller already validates its context before callbacks.
        // Validate entity identity without prefetching later Motion interfaces.
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(MotionUpdateError::ForeignOwner("AttachIntent entity"));
        }
        kernel::update(
            &mut Source {
                actor: self,
                animation: None,
                motion: None,
                skater_motion: None,
            },
            intent,
            attribute,
            settable,
        )
    }

    /// `IsProSkater`82caac80: same `ISkaterAnim`, virtual+30, packed name comparison.
    pub(crate) fn motion_runtime_is_pro_skater(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        expected: &FastString30,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let source = super::play_animation::PlayAnimationActor {
            actor: self,
            context,
        };
        let owner = source.animation()?;
        let actor = self.prefix().map_err(E::Actor)?;
        let animation = owner.lock().map_err(|_| E::Lock)?;
        if !actor.animation_interfaces[2]
            .as_ref()
            .is_some_and(|registered| Arc::ptr_eq(registered, animation.occurrence()))
        {
            return Err(E::ForeignOwner("IsProSkater ISkaterAnim"));
        }
        Ok(animation
            .player()
            .ok_or(E::MissingOwner("SkaterAnim player"))?
            .pro_skater
            == *expected)
    }
}
