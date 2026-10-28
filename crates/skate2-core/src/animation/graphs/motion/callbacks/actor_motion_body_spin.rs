//! Same retained Actor interfaces for `BodySpin30`. Child `motion_runtime_session`.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    physical_player_assembly::PhysicalPlayerAssemblyOwner, play_animation::FastString36,
};
use std::sync::{Arc, Mutex};
#[path = "motion_body_spin.rs"]
pub(crate) mod kernel;
use kernel::{BodySpinSource, SpinChannel, SpinTuning};
pub(crate) trait BodySpinTuning {
    fn enable_landing_on_board(&mut self) -> Result<bool, MotionUpdateError>;
    fn input_graph(&mut self, magnitude: f32) -> Result<f32, MotionUpdateError>;
    fn scalar(&mut self, field: SpinTuning) -> Result<f32, MotionUpdateError>;
}
fn channel_name(channel: SpinChannel) -> FastString36 {
    FastString36::encode(match channel {
        SpinChannel::Front => "IA_BODYSPIN_OLLIE_FS_0_N",
        SpinChannel::Back => "IA_BODYSPIN_OLLIE_BS_0_N",
    })
    .expect("source static FastString")
}
struct Source<'a, T> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    tuning: &'a mut T,
    preland: super::prepare_land::PrepareLandRuntime<'a>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    motion: Option<Arc<()>>,
}
impl<T> Source<'_, T> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, MotionUpdateError> {
        self.physical
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("BodySpin IPhysical"))
    }
    fn animation(&self) -> Result<&Arc<Mutex<MountedActorAnimation>>, MotionUpdateError> {
        self.animation
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner(
                "BodySpin IChannelAnimatable",
            ))
    }
    fn with_state<R>(
        &self,
        apply: impl FnOnce(
            &crate::actor_graph_construction::ActorSkaterGraphState,
        ) -> Result<R, MotionUpdateError>,
    ) -> Result<R, MotionUpdateError> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(E::MissingOwner("BodySpin Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, graph.occurrence()))
        {
            return Err(E::ForeignOwner("BodySpin captured Motion"));
        }
        apply(
            graph
                .state
                .as_ref()
                .ok_or(E::MissingOwner("BodySpin Motion state"))?,
        )
    }
}
impl<T: BodySpinTuning> BodySpinSource for Source<'_, T> {
    type Error = MotionUpdateError;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        self.physical = Some(Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(E::Physical)?
                .owner(),
        ));
        let actor = self.actor.prefix().map_err(E::Actor)?;
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(E::MissingOwner("BodySpin published animation"))?;
        let retained = animation.lock().map_err(|_| E::Lock)?;
        for (index, name) in [
            (0, "IAnimatable"),
            (1, "IChannelAnimatable"),
            (2, "ISkaterAnim"),
        ] {
            if !actor.animation_interfaces[index]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            {
                return Err(E::ForeignOwner(name));
            }
        }
        if !retained
            .entity()
            .is_same_issuance(&self.actor.authority.entity_handle())
        {
            return Err(E::ForeignOwner("BodySpin animation entity"));
        }
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(E::MissingOwner("BodySpin cached Motion"))?;
        for (index, name) in [(2, "ISkaterMotionGraph"), (1, "IMotionGraph")] {
            if !actor.graph_interfaces[index]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
            {
                return Err(E::ForeignOwner(name));
            }
        }
        if self.context.entity() != Some(&self.actor.authority.entity_handle()) {
            return Err(E::ForeignOwner("BodySpin IActor"));
        }
        self.motion = Some(Arc::clone(motion.occurrence()));
        self.animation = Some(Arc::clone(animation));
        Ok(())
    }
    fn enable_landing_on_board(&mut self) -> Result<bool, Self::Error> {
        self.tuning.enable_landing_on_board()
    }
    fn physical_state(&mut self) -> Result<i32, Self::Error> {
        Ok(self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_state_output()
            .map_err(MotionUpdateError::PhysOut)?
            .physics_state)
    }
    fn filtered_state(&mut self) -> Result<u32, Self::Error> {
        Ok(self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_filtered_output()
            .map_err(MotionUpdateError::PhysOut)?
            .filtered_physics_state as u32)
    }
    fn doing_trick(&mut self) -> Result<bool, Self::Error> {
        self.with_state(|s| {
            if s.flags.known_mask & 0x0800_0000 == 0 {
                return Err(MotionUpdateError::MissingOwner("BodySpin doing-trick flag"));
            }
            Ok(s.flags.bits & 0x0800_0000 != 0)
        })
    }
    fn body_spin_intent(&mut self) -> Result<Option<f32>, Self::Error> {
        self.actor.motion_runtime_intent_value(
            self.context,
            &FastString36::encode("BodySpin").expect("static intent"),
        )
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        let animation = self
            .animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?;
        let player = animation.player().ok_or(MotionUpdateError::MissingOwner(
            "BodySpin SkaterAnim player",
        ))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(MotionUpdateError::MissingOwner("BodySpin mirrored flag"));
        }
        Ok(player.flags_bits & 0x4000_0000 != 0)
    }
    fn input_graph(&mut self, magnitude: f32) -> Result<f32, Self::Error> {
        self.tuning.input_graph(magnitude)
    }
    fn time_to_prepare_land(&mut self) -> Result<bool, Self::Error> {
        self.actor.motion_time_to_prepare_land(
            self.context,
            super::prepare_land::PrepareLandRuntime {
                attributes: self.preland.attributes,
                body_tilt: self.preland.body_tilt,
                cvars: self.preland.cvars,
            },
        )
    }
    fn grab_busy(&mut self) -> Result<[i32; 2], Self::Error> {
        self.with_state(|s| Ok(s.grab_busy))
    }
    fn missing_instance(&mut self) -> Self::Error {
        MotionUpdateError::MissingOwner("BodySpin instance")
    }
    fn tuning(&mut self, field: SpinTuning) -> Result<f32, Self::Error> {
        self.tuning.scalar(field)
    }
    fn has_channel(&mut self, channel: SpinChannel) -> Result<bool, Self::Error> {
        let animation = self
            .animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?;
        let channels = animation
            .initialization()
            .and_then(|i| i.channels.as_ref())
            .ok_or(MotionUpdateError::MissingOwner(
                "BodySpin channel dictionary",
            ))?;
        Ok(channels.dictionary.contains_key(&channel_name(channel)))
    }
    fn new_channel(&mut self, channel: SpinChannel) -> Result<(), Self::Error> {
        let name = channel_name(channel);
        self.animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_new_channel_with_blends(name, name, 0.2, 0.3, false)
            .map(|_| ())
            .map_err(MotionUpdateError::AnimationChannel)
    }
    fn end_channel(&mut self, channel: SpinChannel, length: f32) -> Result<(), Self::Error> {
        self.animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_end_body_spin_channel(&channel_name(channel), length)
            .map_err(MotionUpdateError::AnimationWrite)
    }
    fn influence(&mut self, channel: SpinChannel, value: f32) -> Result<(), Self::Error> {
        self.animation()?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_set_body_spin_influence(&channel_name(channel), value)
            .map(|_| ())
            .map_err(MotionUpdateError::AnimationWrite)
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_body_spin<T: BodySpinTuning>(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut kernel::BodySpinInstance>,
        tuning: &mut T,
        preland: super::prepare_land::PrepareLandRuntime<'_>,
    ) -> Result<(), MotionUpdateError> {
        kernel::update(
            &mut Source {
                actor: self,
                context,
                tuning,
                preland,
                physical: None,
                animation: None,
                motion: None,
            },
            instance,
        )
    }
}
