//! Same retained Actor owners for Wipeout Begin/Update; child `motion_runtime_session`.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    actor_graph_construction::ActorSkaterGraphState,
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::MotionLoadError,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, Mutex};
#[path = "motion_wipeout.rs"]
pub(crate) mod kernel;

/// Global+170 uses plain Instance ctor828b3234, with no generated layout.
pub(crate) struct WipeoutAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl WipeoutAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            vault: Arc::clone(vault),
            instance: vault.instance(0x3505_a2ac_5dde_a557, 0xd7ed_bd36_2d7d_2152),
        }
    }
}
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a WipeoutAttributes>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    motion: Option<Arc<()>>,
}
impl Source<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("Wipeout IPhysical"))
    }
    fn animation(&self) -> Result<&Arc<Mutex<MountedActorAnimation>>, Error> {
        self.animation
            .as_ref()
            .ok_or(Error::MissingOwner("Wipeout ISkaterAnim"))
    }
    fn with_state<T>(
        &self,
        apply: impl FnOnce(&mut ActorSkaterGraphState) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let motion = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(Error::MissingOwner("Wipeout captured Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("Wipeout captured Motion"));
        }
        apply(
            motion
                .state
                .as_mut()
                .ok_or(Error::MissingOwner("Motion state"))?,
        )
    }
    fn intents(&self, name: Option<&str>) -> Result<Option<f32>, Error> {
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("Wipeout captured intents"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("Wipeout captured intents"));
        }
        let intents = motion
            .intents
            .as_ref()
            .ok_or(Error::MissingOwner("Motion intents"))?;
        Ok(
            name.and_then(|value| {
                intents.get(&FastString36::encode(value).expect("source intent"))
            }),
        )
    }
}
impl kernel::Source for Source<'_> {
    type Error = Error;
    fn missing_instance(&mut self) -> Error {
        Error::MissingOwner("Wipeout instance")
    }
    fn resolve(&mut self, update: bool) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        self.physical = Some(Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(Error::Physical)?
                .owner(),
        ));
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(Error::MissingOwner("Wipeout published animation"))?;
        let retained = animation.lock().map_err(|_| Error::Lock)?;
        // Begin: physical then ISkaterAnim. Update: physical, ISkaterAnim,
        // IAnimatable, IMotionGraph, ISkaterMotionGraph in native order.
        if !retained
            .entity()
            .is_same_issuance(&self.actor.authority.entity_handle())
        {
            return Err(Error::ForeignOwner("Wipeout animation entity"));
        }
        if !actor.animation_interfaces[2]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
        {
            return Err(Error::ForeignOwner("Wipeout ISkaterAnim"));
        }
        self.animation = Some(Arc::clone(animation));
        if !update {
            return Ok(());
        }
        if !actor.animation_interfaces[0]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
        {
            return Err(Error::ForeignOwner("Wipeout IAnimatable"));
        }
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(Error::MissingOwner("Wipeout published Motion"))?;
        for (index, name) in [
            (1, "Wipeout IMotionGraph"),
            (2, "Wipeout ISkaterMotionGraph"),
        ] {
            if !actor.graph_interfaces[index]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
            {
                return Err(Error::ForeignOwner(name));
            }
        }
        self.motion = Some(Arc::clone(motion.occurrence()));
        Ok(())
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Error> {
        let attributes = self
            .attributes
            .ok_or(Error::MissingOwner("Global anim_wipeout+170"))?;
        if !Arc::ptr_eq(&attributes.vault, &self.physical()?.inputs().vault) {
            return Err(Error::ForeignOwner("Wipeout global Vault"));
        }
        attributes
            .instance
            .try_scalar_word(key, 0)
            .map(|word| f32::from_bits(word.unwrap_or(0)))
            .map_err(|error| Error::Load(MotionLoadError::AttributeRuntime(error)))
    }
    fn lean(&mut self) -> Result<f32, Error> {
        Ok(self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_skeleton_output()
            .map_err(Error::PhysOut)?
            .skeleton_lean)
    }
    fn twist(&mut self) -> Result<f32, Error> {
        Ok(self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_skeleton_output()
            .map_err(Error::PhysOut)?
            .skeleton_twist)
    }
    fn mirrored(&mut self) -> Result<bool, Error> {
        let animation = self.animation()?.lock().map_err(|_| Error::Lock)?;
        let player = animation
            .player()
            .ok_or(Error::MissingOwner("Wipeout SkaterAnim player"))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(Error::MissingOwner("Wipeout mirrored flag"));
        }
        Ok(player.flags_bits & 0x4000_0000 != 0)
    }
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Error> {
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(Error::AnimationWrite)
    }
    fn capture_intents(&mut self) -> Result<(), Error> {
        self.intents(None).map(|_| ())
    }
    fn gesture(&mut self, x: bool) -> Result<Option<f32>, Error> {
        self.intents(Some(if x {
            "WipeoutGestureX"
        } else {
            "WipeoutGestureY"
        }))
    }
    fn controlled(&mut self) -> Result<(), Error> {
        self.with_state(|state| {
            state.try_append_motion_attribute(
                FastString30::encode("ControlledWipeout").expect("source name"),
                1.0,
            )
        })
    }
    fn publish_gesture(&mut self, value: [f32; 2]) -> Result<(), Error> {
        self.with_state(|state| {
            state.wipeout = value;
            Ok(())
        })
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_wipeout(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut kernel::WipeoutInstance>,
        names: kernel::Names,
        attributes: Option<&WipeoutAttributes>,
        begin: bool,
    ) -> Result<(), Error> {
        let mut source = Source {
            actor: self,
            context,
            attributes,
            physical: None,
            animation: None,
            motion: None,
        };
        if begin {
            kernel::begin(&mut source, instance)
        } else {
            kernel::update(&mut source, instance, names)
        }
    }
}
