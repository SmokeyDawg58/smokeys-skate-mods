//! Same Actor allocations for `SettingBodyTilt`; tuning remains a late capability.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    physical_player_assembly::PhysicalPlayerAssemblyOwner, physics::FilteredPhysicsState,
    play_animation::FastString30,
};
use std::sync::{Arc, Mutex};
#[path = "motion_setting_body_tilt.rs"]
pub(crate) mod kernel;

/// Actual shared `Global`.`anim_motion_body_tilt` and live cvar bindings must supply
/// this capability. Calls are separate to retain native lookup/failure order.
pub(crate) trait BodyTiltTuning {
    fn evaluate_spin_graph(&mut self, absolute_speed: f32) -> Result<f32, MotionUpdateError>;
    fn clamp_acceleration(&mut self, air: bool) -> Result<f32, MotionUpdateError>;
    fn clamp_velocity(&mut self, air: bool) -> Result<f32, MotionUpdateError>;
    fn use_tilt(&mut self) -> Result<bool, MotionUpdateError>;
    fn set_tilt(&mut self) -> Result<bool, MotionUpdateError>;
    fn override_x(&mut self) -> Result<f32, MotionUpdateError>;
}
struct Source<'a, T> {
    actor: &'a ActorConstruction,
    tuning: &'a mut T,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    animatable: Option<Arc<Mutex<MountedActorAnimation>>>,
    skater_anim: Option<Arc<Mutex<MountedActorAnimation>>>,
    motion: Option<Arc<()>>,
}
impl<T> Source<'_, T> {
    fn capture_animation(
        &self,
        index: usize,
    ) -> Result<Option<Arc<Mutex<MountedActorAnimation>>>, MotionUpdateError> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published);
        if let Some(animation) = animation {
            let retained = animation.lock().map_err(|_| E::Lock)?;
            if !retained
                .entity()
                .is_same_issuance(&self.actor.authority.entity_handle())
                || !actor.animation_interfaces[index]
                    .as_ref()
                    .is_some_and(|r| Arc::ptr_eq(r, retained.occurrence()))
            {
                return Err(E::ForeignOwner("SettingBodyTilt animation interface"));
            }
        } else if actor.animation_interfaces[index].is_some() {
            return Err(E::ForeignOwner("SettingBodyTilt animation interface"));
        }
        Ok(animation.map(Arc::clone))
    }
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, MotionUpdateError> {
        self.physical
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("SettingBodyTilt IPhysical"))
    }
}
impl<T: BodyTiltTuning> kernel::TiltSource for Source<'_, T> {
    type Error = MotionUpdateError;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        self.physical = Some(Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(E::Physical)?
                .owner(),
        ));
        self.animatable = self.capture_animation(0)?;
        self.skater_anim = self.capture_animation(2)?;
        if self.skater_anim.is_none() {
            return Err(E::MissingOwner("SettingBodyTilt cached ISkaterAnim"));
        }
        {
            let actor = self.actor.prefix().map_err(E::Actor)?;
            let graph = actor
                .graphs
                .as_ref()
                .and_then(|g| g.motion())
                .filter(|_| actor.motion_graph_published)
                .ok_or(E::MissingOwner("SettingBodyTilt cached ISkaterMotionGraph"))?;
            if !actor.graph_interfaces[2]
                .as_ref()
                .is_some_and(|r| Arc::ptr_eq(r, graph.occurrence()))
            {
                return Err(E::ForeignOwner("SettingBodyTilt ISkaterMotionGraph"));
            }
            self.motion = Some(Arc::clone(graph.occurrence()));
        }
        if self.animatable.is_none() {
            return Err(E::MissingOwner("SettingBodyTilt IAnimatable"));
        }
        Ok(())
    }
    fn applying(&mut self) -> Result<bool, Self::Error> {
        use MotionUpdateError as E;
        let actor = self.actor.prefix().map_err(E::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(E::MissingOwner("SettingBodyTilt Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|r| Arc::ptr_eq(r, graph.occurrence()))
        {
            return Err(E::ForeignOwner("SettingBodyTilt captured Motion"));
        }
        let flags = graph
            .state
            .as_ref()
            .ok_or(E::MissingOwner("SettingBodyTilt state"))?
            .flags;
        if flags.known_mask & 0x8000_0000 == 0 {
            return Err(E::MissingOwner("ApplyingBodyTilt bit"));
        }
        Ok(flags.bits & 0x8000_0000 != 0)
    }
    fn missing_instance(&mut self) -> Self::Error {
        MotionUpdateError::MissingOwner("SettingBodyTilt instance")
    }
    fn capture_animation_output(&mut self) -> Result<(), Self::Error> {
        // The actual allocation is already retained. Source captures the section
        // pointer here, but reads its first lane only after the mirror callback.
        let _ = self.physical()?;
        Ok(())
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        use MotionUpdateError as E;
        let animation = self
            .skater_anim
            .as_ref()
            .ok_or(E::MissingOwner("SettingBodyTilt SkaterAnim"))?
            .lock()
            .map_err(|_| E::Lock)?;
        let player = animation
            .player()
            .ok_or(E::MissingOwner("SkaterAnim player"))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(E::MissingOwner("mirror flag initialization"));
        }
        Ok(player.flags_bits & 0x4000_0000 != 0)
    }
    fn animation_tilt_x(&mut self) -> Result<f32, Self::Error> {
        let out = self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?;
        Ok(out
            .try_animation_output()
            .map_err(MotionUpdateError::PhysOut)?
            .anim_tilt
            .xyz[0])
    }
    fn body_spin_speed(&mut self) -> Result<f32, Self::Error> {
        let out = self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?;
        Ok(out
            .try_air_output()
            .map_err(MotionUpdateError::PhysOut)?
            .body_spin_speed)
    }
    fn filtered_is_air(&mut self) -> Result<bool, Self::Error> {
        let out = self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?;
        Ok(out
            .try_filtered_output()
            .map_err(MotionUpdateError::PhysOut)?
            .filtered_physics_state
            == FilteredPhysicsState::Air)
    }
    fn evaluate_spin_graph(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.tuning.evaluate_spin_graph(v)
    }
    fn clamp_acceleration(&mut self, air: bool) -> Result<f32, Self::Error> {
        self.tuning.clamp_acceleration(air)
    }
    fn clamp_velocity(&mut self, air: bool) -> Result<f32, Self::Error> {
        self.tuning.clamp_velocity(air)
    }
    fn use_tilt(&mut self) -> Result<bool, Self::Error> {
        self.tuning.use_tilt()
    }
    fn set_tilt(&mut self) -> Result<bool, Self::Error> {
        self.tuning.set_tilt()
    }
    fn override_x(&mut self) -> Result<f32, Self::Error> {
        self.tuning.override_x()
    }
    fn publish(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        self.animatable
            .as_ref()
            .ok_or(E::MissingOwner("SettingBodyTilt IAnimatable"))?
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(E::AnimationWrite)
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_setting_body_tilt<T: BodyTiltTuning>(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut kernel::TiltInstance>,
        name: FastString30,
        tuning: &mut T,
    ) -> Result<(), MotionUpdateError> {
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(MotionUpdateError::ForeignOwner("SettingBodyTilt entity"));
        }
        kernel::update(
            &mut Source {
                actor: self,
                tuning,
                physical: None,
                animatable: None,
                skater_anim: None,
                motion: None,
            },
            instance,
            name,
        )
    }
}
