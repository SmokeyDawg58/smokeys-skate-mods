//! Proposed `actor_construction::motion_runtime_session::crouching` child.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    attribute_runtime::AttributeRuntimeData,
    input_cvar_runtime::{InputCvarReferences, crouching::CrouchingAttributes},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, Mutex};
#[path = "motion_crouching.rs"]
pub(crate) mod kernel;
use kernel::{Collection, CrouchingSource, PhysicsValue};
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a CrouchingAttributes>,
    cvars: Option<&'a InputCvarReferences>,
    dt: f32,
    name: FastString30,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    motion: Option<Arc<()>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
}
impl Source<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("Crouching IPhysical"))
    }
    fn attributes(&self) -> Result<&CrouchingAttributes, Error> {
        let value = self
            .attributes
            .ok_or(Error::MissingOwner("Crouching retained globals"))?;
        value.check_vault(&self.physical()?.inputs().vault)?;
        Ok(value)
    }
    fn cvars(&self) -> Result<&InputCvarReferences, Error> {
        self.cvars
            .ok_or(Error::MissingOwner("Crouching live cvars"))
    }
    fn check_motion(&self, identity: &Arc<()>) -> Result<(), Error> {
        if self
            .motion
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, identity))
        {
            Ok(())
        } else {
            Err(Error::ForeignOwner("Crouching captured Motion"))
        }
    }
}
impl CrouchingSource for Source<'_> {
    type Error = Error;
    type Scalar = Option<AttributeRuntimeData>;
    type Skeleton = Arc<PhysicalPlayerAssemblyOwner>;
    fn resolve(&mut self, update: bool) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        self.physical = Some(Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(Error::Physical)?
                .owner(),
        ));
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        if update {
            let animation = actor
                .animation
                .as_ref()
                .filter(|_| actor.animation_published)
                .ok_or(Error::MissingOwner("Crouching IAnimatable"))?;
            let retained = animation.lock().map_err(|_| Error::Lock)?;
            if !retained
                .entity()
                .is_same_issuance(&self.actor.authority.entity_handle())
                || !actor.animation_interfaces[0]
                    .as_ref()
                    .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(Error::ForeignOwner("Crouching IAnimatable"));
            }
            self.animation = Some(Arc::clone(animation));
        }
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("Crouching IMotionGraph"))?;
        if !actor.graph_interfaces[1]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("Crouching IMotionGraph"));
        }
        if update {
            if !actor.motion_graph_published {
                return Err(Error::MissingOwner("Crouching ISkaterMotionGraph"));
            }
            if !actor.graph_interfaces[2]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
            {
                return Err(Error::ForeignOwner("Crouching ISkaterMotionGraph"));
            }
        }
        self.motion = Some(Arc::clone(motion.occurrence()));
        Ok(())
    }
    fn scalar_capture(&mut self, c: Collection, key: u64) -> Result<Self::Scalar, Error> {
        self.attributes()?
            .capture(matches!(c, Collection::AutoPump), key)
    }
    fn scalar_read(&mut self, value: &Self::Scalar) -> Result<f32, Error> {
        CrouchingAttributes::read(value.as_ref())
    }
    fn graph(&mut self, c: Collection, offset: usize, input: f32) -> Result<f32, Error> {
        self.attributes()?
            .graph(matches!(c, Collection::AutoPump), offset, input)
    }
    fn physics(&mut self, field: PhysicsValue) -> Result<f32, Error> {
        let out = self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?;
        Ok(match field {
            PhysicsValue::CentreOfMassToDeck => {
                out.try_animation_output()
                    .map_err(Error::PhysOut)?
                    .centre_of_mass_to_deck
            }
            PhysicsValue::SurfaceSpeed => out.try_motion_surface_speed().map_err(Error::PhysOut)?,
            PhysicsValue::VelocityY => {
                out.try_motion_deck_velocity().map_err(Error::PhysOut)?.xyz[1]
            }
            PhysicsValue::Absorption => {
                out.try_skeleton_output()
                    .map_err(Error::PhysOut)?
                    .absorption_compression
            }
            PhysicsValue::GroundNormal => {
                out.try_skeleton_output()
                    .map_err(Error::PhysOut)?
                    .ground_normal_compression
            }
            PhysicsValue::MinimumCrouch => {
                out.try_skeleton_output()
                    .map_err(Error::PhysOut)?
                    .ground_normal_min_crouch
            }
            PhysicsValue::PumpPotential => {
                out.try_motion_pump_potential().map_err(Error::PhysOut)?
            }
        })
    }
    fn skeleton_capture(&mut self) -> Result<Self::Skeleton, Error> {
        let owner = Arc::clone(self.physical()?);
        owner
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_skeleton_output()
            .map_err(Error::PhysOut)?;
        Ok(owner)
    }
    fn deck_compression(&mut self, owner: &Self::Skeleton) -> Result<f32, Error> {
        if !Arc::ptr_eq(owner, self.physical()?) {
            return Err(Error::ForeignOwner("Crouching captured Skeleton owner"));
        }
        Ok(owner
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_skeleton_output()
            .map_err(Error::PhysOut)?
            .deck_angle_compression)
    }
    fn intent(&mut self, name: &str) -> Result<Option<f32>, Error> {
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("Crouching Motion intents"))?;
        self.check_motion(graph.occurrence())?;
        Ok(graph
            .intents
            .as_ref()
            .ok_or(Error::MissingOwner("Crouching Motion intents"))?
            .get(&FastString36::encode(name).expect("source name")))
    }
    fn power_sliding(&mut self) -> Result<bool, Error> {
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("Crouching Motion"))?;
        self.check_motion(graph.occurrence())?;
        let state = graph
            .state
            .as_ref()
            .ok_or(Error::MissingOwner("Crouching Motion state"))?;
        if state.flags.known_mask & 0x0400_0000 == 0 {
            return Err(Error::MissingOwner("Crouching IsPowerSliding flag"));
        }
        Ok(state.flags.bits & 0x0400_0000 != 0)
    }
    fn use_auto_pump(&mut self) -> Result<bool, Error> {
        self.cvars()?.try_crouching_auto_pump().map_err(Error::Cvar)
    }
    fn pumping_time(&mut self) -> Result<f32, Error> {
        self.cvars()?
            .try_crouching_pumping_time()
            .map_err(Error::Cvar)
    }
    fn dt(&mut self) -> Result<f32, Error> {
        Ok(self.dt)
    }
    fn publish_motion(&mut self, name: &str) -> Result<(), Error> {
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(Error::MissingOwner("Crouching Motion"))?;
        self.check_motion(graph.occurrence())?;
        graph
            .state
            .as_mut()
            .ok_or(Error::MissingOwner("Crouching Motion state"))?
            .try_append_motion_attribute(FastString30::encode(name).expect("source name"), 1.0)
    }
    fn publish_animation(&mut self, value: f32) -> Result<(), Error> {
        self.animation
            .as_ref()
            .ok_or(Error::MissingOwner("Crouching captured IAnimatable"))?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(self.name, value, false, -1)
            .map_err(Error::AnimationWrite)
    }
}
impl ActorConstruction {
    /// Allocation is controller-owned. False allocation must not sample233.
    pub(crate) fn motion_runtime_crouching_create(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        cvars: Option<&InputCvarReferences>,
        allocated: bool,
    ) -> Result<Option<kernel::CrouchingInstance>, Error> {
        let mut source = Source {
            actor: self,
            context,
            attributes: None,
            cvars,
            dt: 0.0,
            name: FastString30::encode("").expect("empty"),
            physical: None,
            motion: None,
            animation: None,
        };
        kernel::create(&mut source, allocated)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn motion_runtime_crouching(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attributes: Option<&CrouchingAttributes>,
        cvars: Option<&InputCvarReferences>,
        name: FastString30,
        dt: f32,
        instance: &mut kernel::CrouchingInstance,
        begin: bool,
    ) -> Result<(), Error> {
        let mut source = Source {
            actor: self,
            context,
            attributes,
            cvars,
            dt,
            name,
            physical: None,
            motion: None,
            animation: None,
        };
        if begin {
            kernel::begin(&mut source, instance)
        } else {
            kernel::update(&mut source, instance)
        }
    }
}
