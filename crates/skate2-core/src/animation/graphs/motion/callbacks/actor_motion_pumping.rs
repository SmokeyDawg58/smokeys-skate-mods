//! Proposed `actor_construction::motion_runtime_session::pumping` child.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    input_cvar_runtime::{InputCvarReferences, motion_pumping::PumpingAttributes},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::FastString36,
};
use std::sync::{Arc, Mutex};
#[path = "motion_pumping.rs"]
pub(crate) mod kernel;
use kernel::{PumpingChannel, PumpingSource};
fn channel(index: usize) -> Result<FastString36, Error> {
    let name = ["PUMP0", "PUMP1", "PUMP2", "PUMP3", "PUMP4"]
        .get(index)
        .ok_or(Error::MissingOwner("Pumping native channel index"))?;
    Ok(FastString36::encode(name).expect("source name"))
}
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a PumpingAttributes>,
    cvars: Option<&'a InputCvarReferences>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    motion: Option<Arc<()>>,
}
impl Source<'_> {
    fn animation(&self) -> Result<&Arc<Mutex<MountedActorAnimation>>, Error> {
        self.animation
            .as_ref()
            .ok_or(Error::MissingOwner("Pumping captured IChannelAnimatable"))
    }
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("Pumping IPhysical"))
    }
    fn attributes(&self) -> Result<&PumpingAttributes, Error> {
        let value = self
            .attributes
            .ok_or(Error::MissingOwner("Global anim_motion_anim_pump+134"))?;
        value.check_vault(&self.physical()?.inputs().vault)?;
        Ok(value)
    }
}
impl PumpingSource for Source<'_> {
    type Error = Error;
    fn resolve(&mut self, update: bool) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        // Source resolves channel first, even on the early AllowPumping return.
        {
            let actor = self.actor.prefix().map_err(Error::Actor)?;
            let animation = actor
                .animation
                .as_ref()
                .filter(|_| actor.animation_published)
                .ok_or(Error::MissingOwner("Pumping IChannelAnimatable"))?;
            let retained = animation.lock().map_err(|_| Error::Lock)?;
            if !retained
                .entity()
                .is_same_issuance(&self.actor.authority.entity_handle())
                || !actor.animation_interfaces[1]
                    .as_ref()
                    .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(Error::ForeignOwner("Pumping IChannelAnimatable"));
            }
            self.animation = Some(Arc::clone(animation));
        }
        if !update {
            return Ok(());
        }
        self.physical = Some(Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(Error::Physical)?
                .owner(),
        ));
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(Error::MissingOwner("Pumping ISkaterMotionGraph"))?;
        if !actor.graph_interfaces[2]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, graph.occurrence()))
        {
            return Err(Error::ForeignOwner("Pumping ISkaterMotionGraph"));
        }
        self.motion = Some(Arc::clone(graph.occurrence()));
        Ok(())
    }
    fn allowed(&mut self) -> Result<bool, Error> {
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("Pumping Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, graph.occurrence()))
        {
            return Err(Error::ForeignOwner("Pumping captured Motion"));
        }
        let state = graph
            .state
            .as_ref()
            .ok_or(Error::MissingOwner("Pumping Motion state"))?;
        if state.flags.known_mask & 0x0100_0000 == 0 {
            return Err(Error::MissingOwner("AllowPumping flag"));
        }
        Ok(state.flags.bits & 0x0100_0000 != 0)
    }
    fn missing_instance(&mut self) -> Error {
        Error::MissingOwner("Pumping instance")
    }
    fn ground_acceleration(&mut self) -> Result<f32, Error> {
        Ok(self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_ground_output()
            .map_err(Error::PhysOut)?
            .pumping_acceleration)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Error> {
        self.attributes()?.scalar(key)
    }
    fn graph(&mut self, input: f32) -> Result<f32, Error> {
        self.attributes()?.graph(input)
    }
    fn has_channel(&mut self, index: usize) -> Result<bool, Error> {
        let name = channel(index)?;
        let animation = self.animation()?.lock().map_err(|_| Error::Lock)?;
        let channels = animation
            .initialization()
            .and_then(|i| i.channels.as_ref())
            .ok_or(Error::MissingOwner("Pumping channel dictionary"))?;
        Ok(channels.dictionary.contains_key(&name))
    }
    fn enabled(&mut self) -> Result<bool, Error> {
        self.cvars
            .ok_or(Error::MissingOwner("Pumping live cvar238"))?
            .try_motion_pumping_enabled()
            .map_err(Error::Cvar)
    }
    fn new_channel(&mut self, request: PumpingChannel) -> Result<(), Error> {
        let name = channel(request.index)?;
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_new_channel_with_blends(
                name,
                FastString36::encode("B_PUMP").expect("source name"),
                request.blend_in,
                request.blend_out,
                true,
            )
            .map(|_| ())
            .map_err(Error::AnimationChannel)
    }
    fn influence(&mut self, index: usize, value: f32) -> Result<(), Error> {
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_set_body_spin_influence(&channel(index)?, value)
            .map(|_| ())
            .map_err(Error::AnimationWrite)
    }
    fn end_channel(&mut self, index: usize) -> Result<(), Error> {
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_end_gesture_channel(&channel(index)?)
            .map_err(Error::AnimationWrite)
    }
}
impl ActorConstruction {
    /// DisallowPumping Begin82c9f460 ends the five authored PUMP channels
    /// with a 0.1s blend, then SetAllowPumping825fe028(false). End82c9f608
    /// restores that flag; the native Update is empty.
    pub(crate) fn motion_runtime_disallow_pumping(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        begin: bool,
    ) -> Result<(), Error> {
        if begin {
            let mut source = Source {
                actor: self, context, attributes: None, cvars: None,
                animation: None, physical: None, motion: None,
            };
            source.resolve(false)?;
            self.with_grounded_motion_state(context, |_| Ok(()))?;
            let mut animation = source.animation()?.lock().map_err(|_| Error::Lock)?;
            for index in 0..5 {
                animation.try_end_channel_with_blend(&channel(index)?, 0.1, false)
                    .map_err(Error::AnimationWrite)?;
            }
        }
        self.with_grounded_motion_state(context, |state| {
            state.flags.bits = (state.flags.bits & !0x0100_0000)
                | if begin { 0 } else { 0x0100_0000 };
            state.flags.known_mask |= 0x0100_0000;
            Ok(())
        })
    }

    pub(crate) fn motion_runtime_pumping(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attributes: Option<&PumpingAttributes>,
        cvars: Option<&InputCvarReferences>,
        instance: Option<&mut kernel::PumpingInstance>,
        end: bool,
    ) -> Result<(), Error> {
        let mut source = Source {
            actor: self,
            context,
            attributes,
            cvars,
            animation: None,
            physical: None,
            motion: None,
        };
        if end {
            kernel::end(&mut source)
        } else {
            kernel::update(&mut source, instance)
        }
    }
}
