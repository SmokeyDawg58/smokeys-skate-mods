//! Same retained owners for `SetTurning1413`; child `motion_runtime_session::set_turning`.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    actor_graph_construction::ActorSkaterGraphState,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{
        InputCvarReferences, power_sliding::PowerSlidingAttributes, set_turning::TurningAttributes,
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, Mutex};
#[path = "motion_set_turning.rs"]
pub(crate) mod kernel;
use kernel::{Output, TurningSource, reshape::ReshapeSource};
pub(crate) struct TurningRuntime<'a> {
    pub(crate) attributes: Option<&'a TurningAttributes>,
    pub(crate) sliding_attributes: Option<&'a PowerSlidingAttributes>,
    pub(crate) cvars: Option<&'a InputCvarReferences>,
    pub(crate) overlay: Option<&'a DebugOverlaySlot>,
    pub(crate) dt: f32,
}
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    runtime: TurningRuntime<'a>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    motion: Option<Arc<()>>,
}
impl Source<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("SetTurning IPhysical"))
    }
    fn animation(&self) -> Result<&Arc<Mutex<MountedActorAnimation>>, Error> {
        self.animation
            .as_ref()
            .ok_or(Error::MissingOwner("SetTurning IAnimatable"))
    }
    fn attributes(&self) -> Result<&TurningAttributes, Error> {
        let attributes = self
            .runtime
            .attributes
            .ok_or(Error::MissingOwner("Global anim_carving+174"))?;
        attributes.check_vault(&self.physical()?.inputs().vault)?;
        Ok(attributes)
    }
    fn with_state<T>(
        &self,
        apply: impl FnOnce(&mut ActorSkaterGraphState) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(Error::MissingOwner("SetTurning Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|m| Arc::ptr_eq(m, graph.occurrence()))
        {
            return Err(Error::ForeignOwner("SetTurning captured Motion"));
        }
        apply(
            graph
                .state
                .as_mut()
                .ok_or(Error::MissingOwner("Motion state"))?,
        )
    }
    fn animation_flag(&self, mask: u32) -> Result<bool, Error> {
        let animation = self.animation()?.lock().map_err(|_| Error::Lock)?;
        let player = animation
            .player()
            .ok_or(Error::MissingOwner("SetTurning SkaterAnim player"))?;
        if player.flags_known_mask & mask != mask {
            return Err(Error::MissingOwner("SetTurning animation flag"));
        }
        Ok(player.flags_bits & mask != 0)
    }
    fn named_intent(&self, name: Option<&str>) -> Result<Option<f32>, Error> {
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("SetTurning captured IMotionGraph"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|m| Arc::ptr_eq(m, graph.occurrence()))
        {
            return Err(Error::ForeignOwner("SetTurning captured intents"));
        }
        let intents = graph
            .intents
            .as_ref()
            .ok_or(Error::MissingOwner("Motion intents"))?;
        Ok(name
            .and_then(|name| intents.get(&FastString36::encode(name).expect("source intent name"))))
    }
}
impl ReshapeSource for Source<'_> {
    type Error = Error;
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Error> {
        self.attributes()?.graph(offset, input)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Error> {
        self.attributes()?.scalar(key)
    }
    fn reshape_cvar(&mut self, index: usize) -> Result<bool, Error> {
        self.runtime
            .cvars
            .ok_or(Error::MissingOwner("SetTurning cvars"))?
            .try_turning_reshape(index)
            .map_err(Error::Cvar)
    }
}
impl TurningSource for Source<'_> {
    fn resolve_interfaces(&mut self) -> Result<(), Error> {
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
            .ok_or(Error::MissingOwner("SetTurning published animation"))?;
        let retained = animation.lock().map_err(|_| Error::Lock)?;
        for (index, name) in [(0, "SetTurning IAnimatable"), (2, "SetTurning ISkaterAnim")] {
            if !actor.animation_interfaces[index]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(Error::ForeignOwner(name));
            }
        }
        if !retained
            .entity()
            .is_same_issuance(&self.actor.authority.entity_handle())
        {
            return Err(Error::ForeignOwner("SetTurning animation entity"));
        }
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(Error::MissingOwner("SetTurning cached Motion"))?;
        for (index, name) in [
            (1, "SetTurning IMotionGraph"),
            (2, "SetTurning ISkaterMotionGraph"),
        ] {
            if !actor.graph_interfaces[index]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
            {
                return Err(Error::ForeignOwner(name));
            }
        }
        self.animation = Some(Arc::clone(animation));
        self.motion = Some(Arc::clone(motion.occurrence()));
        Ok(())
    }
    fn riding_fakie(&mut self) -> Result<bool, Error> {
        self.animation_flag(0x2000_0000)
    }
    fn capture_intents(&mut self) -> Result<(), Error> {
        self.named_intent(None).map(|_| ())
    }
    fn output(&mut self, field: Output) -> Result<f32, Error> {
        let output = self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?;
        let animation = output.try_animation_output().map_err(Error::PhysOut)?;
        Ok(match field {
            Output::TargetLean => animation.animation_target_lean,
            Output::Turn => animation.animation_turn,
            Output::Direction => animation.animation_lean_direction,
            Output::Quickness => animation.animation_lean_quickness,
            Output::Holding => animation.animation_holding,
        })
    }
    fn missing_instance(&mut self) -> Error {
        Error::MissingOwner("SetTurning instance")
    }
    fn slide_flags(&mut self, side: usize, required: u32) -> Result<u32, Error> {
        self.with_state(|s| {
            let flag = s.slide_bits[side];
            if flag.known_mask & required != required {
                return Err(Error::MissingOwner("SetTurning sliding flags"));
            }
            Ok(flag.bits)
        })
    }
    fn clear_force(&mut self, side: usize) -> Result<(), Error> {
        self.with_state(|s| {
            s.slide_bits[side].bits &= !0x2000_0000;
            s.slide_bits[side].known_mask |= 0x2000_0000;
            Ok(())
        })
    }
    fn combined(&mut self, required: u32) -> Result<u32, Error> {
        self.with_state(|s| {
            let flag = s.combined_slide_bits;
            if flag.known_mask & required != required {
                return Err(Error::MissingOwner("SetTurning combined slide flags"));
            }
            Ok(flag.bits)
        })
    }
    fn reset_slide_time(&mut self, side: usize) -> Result<(), Error> {
        self.with_state(|s| {
            s.slide_time[side] = 0.0;
            Ok(())
        })
    }
    fn set_slide_fakie(&mut self, fakie: bool) -> Result<(), Error> {
        self.with_state(|s| {
            s.combined_slide_bits.bits =
                (s.combined_slide_bits.bits & !0x8000_0000) | if fakie { 0x8000_0000 } else { 0 };
            s.combined_slide_bits.known_mask |= 0x8000_0000;
            Ok(())
        })
    }
    fn mirrored(&mut self) -> Result<bool, Error> {
        self.animation_flag(0x4000_0000)
    }
    fn debug(&mut self) -> Result<(), Error> {
        if self
            .runtime
            .cvars
            .ok_or(Error::MissingOwner("SetTurning debug cvar"))?
            .try_power_sliding_debug()
            .map_err(Error::Cvar)?
        {
            let _ = self
                .runtime
                .overlay
                .ok_or(Error::MissingOwner("DebugOverlaySlot"))?
                .require()
                .map_err(|e| match e {
                    DebugOverlayError::Poisoned => Error::Lock,
                    _ => Error::MissingOwner("DebugOverlay singleton"),
                })?;
        }
        Ok(())
    }
    fn dt(&mut self) -> Result<f32, Error> {
        Ok(self.runtime.dt)
    }
    fn speed(&mut self) -> Result<f32, Error> {
        self.physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_z_speed()
            .map_err(Error::PhysOut)
    }
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Error> {
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(Error::AnimationWrite)
    }
    fn slide_turn(&mut self) -> Result<f32, Error> {
        let attributes = self
            .runtime
            .sliding_attributes
            .ok_or(Error::MissingOwner("Global anim_motion_power_slide+14c"))?;
        attributes.check_vault(&self.physical()?.inputs().vault)?;
        attributes.scalar(0xbd26_0949_27a0_947a)
    }
    fn intent(&mut self, name: &str) -> Result<Option<f32>, Error> {
        self.named_intent(Some(name))
    }
    fn motion_attribute(&mut self, name: &str, value: f32) -> Result<(), Error> {
        let name = FastString30::encode(name).expect("source attribute name");
        self.with_state(|s| s.try_append_motion_attribute(name, value))
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_set_turning(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut kernel::TurningInstance>,
        names: kernel::Names,
        runtime: TurningRuntime<'_>,
    ) -> Result<(), Error> {
        kernel::update(
            &mut Source {
                actor: self,
                context,
                runtime,
                physical: None,
                animation: None,
                motion: None,
            },
            instance,
            names,
        )
    }
}
