//! `CreateAttribute` uses actual shared Motion/Animation owners; child `motion_runtime_session`.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    generated_motion_load::construct::CreateAttribute, play_animation::FastString30,
};
use std::sync::{Arc, Mutex};
#[path = "motion_create_attribute.rs"]
pub(crate) mod kernel;
use kernel::CreateAttributeSource;
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    motion: Option<Arc<()>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
}
impl CreateAttributeSource for Source<'_> {
    type Error = Error;
    fn resolve_skater_motion(&mut self) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(Error::MissingOwner(
                "CreateAttribute cached ISkaterMotionGraph",
            ))?;
        if !actor.graph_interfaces[2]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("CreateAttribute ISkaterMotionGraph"));
        }
        self.motion = Some(Arc::clone(motion.occurrence()));
        Ok(())
    }
    fn append_motion(&mut self, name: FastString30, value: f32) -> Result<(), Error> {
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let motion = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(Error::MissingOwner("CreateAttribute captured Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("CreateAttribute captured Motion"));
        }
        motion
            .state
            .as_mut()
            .ok_or(Error::MissingOwner("Motion state"))?
            .try_append_motion_attribute(name, value)
    }
    fn resolve_animatable(&mut self) -> Result<bool, Error> {
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        if let Some(animation) = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
        {
            let retained = animation.lock().map_err(|_| Error::Lock)?;
            if !retained
                .entity()
                .is_same_issuance(&self.actor.authority.entity_handle())
                || !actor.animation_interfaces[0]
                    .as_ref()
                    .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(Error::ForeignOwner("CreateAttribute IAnimatable"));
            }
            self.animation = Some(Arc::clone(animation));
            Ok(true)
        } else if actor.animation_interfaces[0].is_some() {
            Err(Error::ForeignOwner("CreateAttribute IAnimatable"))
        } else {
            Ok(false)
        }
    }
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Error> {
        self.animation
            .as_ref()
            .ok_or(Error::MissingOwner("CreateAttribute captured IAnimatable"))?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(Error::AnimationWrite)
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_create_attribute(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &CreateAttribute,
        phase: kernel::Phase,
    ) -> Result<(), Error> {
        kernel::run(
            &mut Source {
                actor: self,
                context,
                motion: None,
                animation: None,
            },
            payload,
            phase,
        )
    }
}
