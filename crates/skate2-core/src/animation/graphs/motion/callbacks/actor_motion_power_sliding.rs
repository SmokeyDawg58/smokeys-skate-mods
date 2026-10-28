//! Proposed child `actor_construction::motion_runtime_session::power_sliding`.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_graph_construction::ActorSkaterGraphState,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarReferences, power_sliding::PowerSlidingAttributes},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physics::FilteredPhysicsState,
    play_animation::FastString36,
    reckoning_numeric::{host_dot3, length_from_squared_and_estimate},
    simulation_drive_jacobian::refine_drive_reciprocal,
};
use std::sync::Arc;
#[path = "motion_power_sliding.rs"]
pub(crate) mod kernel;
use kernel::{Side, SlidingSource};

struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a PowerSlidingAttributes>,
    cvars: Option<&'a InputCvarReferences>,
    overlay: Option<&'a DebugOverlaySlot>,
    dt: f32,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    motion: Option<Arc<()>>,
}
impl Source<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("PowerSliding IPhysical"))
    }
    fn attributes(&self) -> Result<&PowerSlidingAttributes, Error> {
        let attrs = self
            .attributes
            .ok_or(Error::MissingOwner("Global anim_motion_power_slide+14c"))?;
        attrs.check_vault(&self.physical()?.inputs().vault)?;
        Ok(attrs)
    }
    fn with_state<T>(
        &self,
        apply: impl FnOnce(&mut ActorSkaterGraphState) -> T,
    ) -> Result<T, Error> {
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(Error::MissingOwner("PowerSliding Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|m| Arc::ptr_eq(m, graph.occurrence()))
        {
            return Err(Error::ForeignOwner("PowerSliding captured Motion"));
        }
        Ok(apply(
            graph
                .state
                .as_mut()
                .ok_or(Error::MissingOwner("Motion state"))?,
        ))
    }
    fn named_intent(&self, name: &str) -> Result<Option<f32>, Error> {
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("PowerSliding captured IMotionGraph"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|m| Arc::ptr_eq(m, graph.occurrence()))
        {
            return Err(Error::ForeignOwner("PowerSliding captured intents"));
        }
        Ok(graph
            .intents
            .as_ref()
            .ok_or(Error::MissingOwner("Motion intents"))?
            .get(&FastString36::encode(name).expect("native intent name")))
    }
}
impl SlidingSource for Source<'_> {
    type Error = Error;
    fn resolve_interfaces_and_sliding_state(&mut self) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        self.physical = Some(Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(Error::Physical)?
                .owner(),
        ));
        // Native owner order: IPhysical, ISkaterAnim, IMotionGraph, ISkaterMotionGraph.
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(Error::MissingOwner("PowerSliding cached ISkaterAnim"))?;
        let animation = animation.lock().map_err(|_| Error::Lock)?;
        if !actor.animation_interfaces[2]
            .as_ref()
            .is_some_and(|p| Arc::ptr_eq(p, animation.occurrence()))
        {
            return Err(Error::ForeignOwner("PowerSliding ISkaterAnim"));
        }
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("PowerSliding IMotionGraph"))?;
        if !actor.graph_interfaces[1]
            .as_ref()
            .is_some_and(|p| Arc::ptr_eq(p, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("PowerSliding IMotionGraph"));
        }
        if !actor.motion_graph_published {
            return Err(Error::MissingOwner(
                "PowerSliding cached ISkaterMotionGraph",
            ));
        }
        if !actor.graph_interfaces[2]
            .as_ref()
            .is_some_and(|p| Arc::ptr_eq(p, motion.occurrence()))
        {
            return Err(Error::ForeignOwner("PowerSliding ISkaterMotionGraph"));
        }
        motion
            .state
            .as_ref()
            .ok_or(Error::MissingOwner("PowerSliding sliding state"))?;
        self.motion = Some(Arc::clone(motion.occurrence()));
        Ok(())
    }
    fn intent(&mut self, side: Side) -> Result<Option<f32>, Error> {
        self.named_intent(match side {
            Side::Left => "LeftSlide",
            Side::Right => "RightSlide",
        })
    }
    fn is_ground(&mut self) -> Result<bool, Error> {
        Ok(self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_filtered_output()
            .map_err(Error::PhysOut)?
            .filtered_physics_state
            == FilteredPhysicsState::Ground)
    }
    fn missing_instance(&mut self) -> Error {
        Error::MissingOwner("PowerSliding instance")
    }
    fn deck_speed(&mut self) -> Result<f32, Error> {
        self.physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_deck_speed()
            .map_err(Error::PhysOut)
    }
    fn speed_limit(&mut self) -> Result<f32, Error> {
        self.attributes()?.scalar(0x1cb9_5aad_d7b6_5137)
    }
    fn start_intent(&mut self, side: Side) -> Result<bool, Error> {
        Ok(self
            .named_intent(match side {
                Side::Left => "LeftSlideStart",
                Side::Right => "RightSlideStart",
            })?
            .is_some())
    }
    fn write_start(&mut self, side: Side, value: bool) -> Result<(), Error> {
        self.with_state(|s| {
            let flag = &mut s.slide_bits[side.index()];
            flag.bits = (flag.bits & !0x8000_0000) | if value { 0x8000_0000 } else { 0 };
            flag.known_mask |= 0x8000_0000;
        })
    }
    fn debug(&mut self, calls: usize) -> Result<(), Error> {
        if self
            .cvars
            .ok_or(Error::MissingOwner("PowerSliding cvars"))?
            .try_power_sliding_debug()
            .map_err(Error::Cvar)?
        {
            let overlay = self
                .overlay
                .ok_or(Error::MissingOwner("DebugOverlaySlot"))?;
            for _ in 0..calls {
                // Native Get then BLR DrawText; each Get is still observable.
                let _ = overlay.require().map_err(|e| match e {
                    DebugOverlayError::Poisoned => Error::Lock,
                    _ => Error::MissingOwner("DebugOverlay singleton"),
                })?;
            }
        }
        Ok(())
    }
    fn heading_cosine(&mut self) -> Result<f32, Error> {
        // GetGroundHeadingCosine825f4ea8 repeats actual IPhysical resolution.
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(Error::Physical)?;
        let velocity = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_deck_velocity()
            .map_err(Error::PhysOut)?;
        let velocity = [velocity.xyz[0], velocity.xyz[1], velocity.xyz[2], 0.0];
        let squared = host_dot3(velocity, velocity);
        // Raw4fa8 BO4/BI25 !GT =>1, including NaN. Source constant3a83126f.
        if !matches!(
            squared.partial_cmp(&f32::from_bits(0x3a83_126f)),
            Some(std::cmp::Ordering::Greater)
        ) {
            return Ok(1.0);
        }
        let transform = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_ground_heading_transform()
            .map_err(Error::PhysOut)?;
        let flipped = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_skateboard_flipped()
            .map_err(Error::PhysOut)?;
        let forward = transform.columns()[2];
        let direction = [forward.xyz[0], forward.xyz[1], forward.xyz[2], 0.0].map(|v| {
            if flipped {
                f32::from_bits(v.to_bits() ^ 0x8000_0000)
            } else {
                v
            }
        });
        // Existing host seed/reduction boundary, original two rsqrt refinements,
        // length multiplication, two reciprocal refinements and final dot.
        let length = length_from_squared_and_estimate(squared, 1.0 / squared.sqrt());
        let inverse_length = refine_drive_reciprocal(length, 1.0 / length);
        Ok(host_dot3(velocity.map(|v| v * inverse_length), direction))
    }
    fn graph(&mut self, offset: usize, value: f32) -> Result<f32, Error> {
        self.attributes()?.graph(offset, value)
    }
    fn dt(&mut self) -> Result<f32, Error> {
        Ok(self.dt)
    }
    fn minimum_time(&mut self) -> Result<f32, Error> {
        self.attributes()?.scalar(0x09ef_22b1_2364_eb7f)
    }
    fn increment_time(&mut self, side: Side, dt: f32) -> Result<f32, Error> {
        self.with_state(|s| {
            let value = dt + s.slide_time[side.index()];
            s.slide_time[side.index()] = value;
            value
        })
    }
    fn write_end(&mut self, side: Side, value: bool) -> Result<(), Error> {
        self.with_state(|s| {
            let flag = &mut s.slide_bits[side.index()];
            flag.bits = (flag.bits & !0x4000_0000) | if value { 0x4000_0000 } else { 0 };
            flag.known_mask |= 0x4000_0000;
        })
    }
}
impl ActorConstruction {
    /// PowerSlideSpin.Begin82cb2350 uses the existing heading-cosine getter.
    pub(crate) fn motion_slide_spin_begin(&self, context: &BoundMotionGraphContext<ActorIdentity>, attributes: &PowerSlidingAttributes, cvars: &InputCvarReferences, overlay: &DebugOverlaySlot) -> Result<(), Error> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(0)?;
        self.with_grounded_motion_state(context, |_| Ok(()))?;
        let mut source = Source { actor: self, context, attributes: Some(attributes), cvars: Some(cvars), overlay: Some(overlay), dt: 0.0, physical: None, motion: None };
        let cosine = source.heading_cosine()?;
        let value = attributes.slide_graph(0x50, cosine)?;
        source.debug(1)?;
        animation.lock().map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(crate::play_animation::FastString30::encode("slidespin").expect("source"), -value, false, -1)
            .map_err(Error::AnimationWrite)
    }

    /// CreateSlide.Begin82ca3160: capture riding-fakie and reset both timers.
    pub(crate) fn motion_create_slide_begin(&self, context: &BoundMotionGraphContext<ActorIdentity>) -> Result<(), Error> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(2)?;
        let guard = animation.lock().map_err(|_| Error::Lock)?;
        let player = guard.player().ok_or(Error::MissingOwner("CreateSlide player"))?;
        if player.flags_known_mask & 0x2000_0000 == 0 {
            return Err(Error::MissingOwner("CreateSlide riding fakie"));
        }
        let fakie = player.flags_bits & 0x2000_0000 != 0;
        drop(guard);
        self.with_grounded_motion_state(context, |state| {
            state.combined_slide_bits.bits = (state.combined_slide_bits.bits & !0x8000_0000)
                | if fakie { 0x8000_0000 } else { 0 };
            state.combined_slide_bits.known_mask |= 0x8000_0000;
            state.slide_time = [0.0; 2];
            Ok(())
        })
    }

    /// CreateSlide.Update82cc7e10: source clamp, stance reflection, MG outputs.
    pub(crate) fn motion_create_slide_update(&self, context: &BoundMotionGraphContext<ActorIdentity>, right: bool, attributes: &PowerSlidingAttributes) -> Result<(), Error> {
        let minimum = attributes.scalar(0x3150_3aab_02ee_76dd)?;
        let minimum_time = attributes.scalar(0x09ef_22b1_2364_eb7f)?;
        let turn_amount = attributes.scalar(0xbd26_0949_27a0_947a)?;
        let (fakie, times) = self.with_grounded_motion_state(context, |state| {
            if state.combined_slide_bits.known_mask & 0x8000_0000 == 0 {
                return Err(Error::MissingOwner("CreateSlide from fakie"));
            }
            Ok((state.combined_slide_bits.bits & 0x8000_0000 != 0, state.slide_time))
        })?;
        let selected_right = right ^ fakie;
        let name = FastString36::encode(if selected_right { "RightSlide" } else { "LeftSlide" }).expect("source intent");
        let input = self.motion_runtime_intent_value(context, &name)?.unwrap_or(0.0);
        let mut slide = if times[usize::from(selected_right)] > minimum_time { input }
            else if selected_right { if minimum - input >= 0.0 { minimum } else { input } }
            else { if -minimum - input >= 0.0 { input } else { -minimum } };
        let mut turn = if selected_right { turn_amount } else { -turn_amount };
        if fakie {
            let negative = -slide;
            let magnitude = negative.abs();
            // Native BLE skips the fold; unordered follows the subtraction.
            let folded = if magnitude <= 0.5 { magnitude } else { 1.0 - magnitude };
            slide = if negative >= 0.0 { folded } else { -folded };
            turn = -turn;
        }
        self.with_grounded_motion_state(context, |state| {
            state.try_append_motion_attribute(crate::play_animation::FastString30::encode("slide").expect("source"), slide)?;
            state.try_append_motion_attribute(crate::play_animation::FastString30::encode("turn").expect("source"), turn)
        })
    }

    /// PowerSlideDecel.Update82cb1e00 uses ground-local horizontal deck speed.
    pub(crate) fn motion_slide_decel_update(&self, context: &BoundMotionGraphContext<ActorIdentity>, value: &mut f32, attributes: &PowerSlidingAttributes, cvars: &InputCvarReferences, overlay: &DebugOverlaySlot) -> Result<(), Error> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(Error::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        self.lifecycle_animation(2)?;
        self.with_grounded_motion_state(context, |_| Ok(()))?;
        let output = physical.owner().phys_out().lock().map_err(|_| Error::Lock)?;
        let velocity = output.try_motion_deck_velocity().map_err(Error::PhysOut)?;
        let transform = output.try_motion_ground_heading_transform().map_err(Error::PhysOut)?;
        drop(output);
        let axes = transform.columns();
        let local = |axis: usize| {
            let first = axes[axis].xyz[0] * velocity.xyz[0];
            let second = axes[axis].xyz[1].mul_add(velocity.xyz[1], first);
            axes[axis].xyz[2].mul_add(velocity.xyz[2], second)
        };
        let x = local(0);
        let z = local(2);
        let squared = host_dot3([x, 0.0, z, 0.0], [x, 0.0, z, 0.0]);
        let speed = length_from_squared_and_estimate(squared, 1.0 / squared.sqrt());
        let target = attributes.slide_graph(0, speed)?;
        let blend = attributes.scalar(0x2dc2_d46d_fba6_12cd)?;
        *value = (1.0 - blend).mul_add(*value, blend * target);
        if cvars.try_power_sliding_debug().map_err(Error::Cvar)? {
            overlay.require().map_err(|_| Error::MissingOwner("slide DebugOverlay"))?;
        }
        animation.lock().map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(crate::play_animation::FastString30::encode("decel").expect("source"), *value, false, -1)
            .map_err(Error::AnimationWrite)
    }

    pub(crate) fn motion_runtime_power_sliding(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut kernel::SlidingInstance>,
        attributes: Option<&PowerSlidingAttributes>,
        cvars: Option<&InputCvarReferences>,
        overlay: Option<&DebugOverlaySlot>,
        dt: f32,
    ) -> Result<(), Error> {
        kernel::update(
            &mut Source {
                actor: self,
                context,
                attributes,
                cvars,
                overlay,
                dt,
                physical: None,
                motion: None,
            },
            instance,
        )
    }
}
