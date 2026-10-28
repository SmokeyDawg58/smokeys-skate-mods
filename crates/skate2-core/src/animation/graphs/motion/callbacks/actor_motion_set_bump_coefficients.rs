//! Same Actor/global attributes for SetBumpCoefficients.Begin82cb1600.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError, attribute_error,
};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::FastString30,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};
#[path = "motion_set_bump_coefficients.rs"]
mod kernel;
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    names: [FastString30; 2],
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
}
impl kernel::BumpCoefficientSource for Source<'_> {
    type Error = MotionUpdateError;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        self.actor.check_motion_runtime_context(self.context)?;
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
            .ok_or(E::MissingOwner("bump IAnimatable"))?;
        let retained = animation.lock().map_err(|_| E::Lock)?;
        for index in [0, 2] {
            if !retained
                .entity()
                .is_same_issuance(&self.actor.authority.entity_handle())
                || !actor.animation_interfaces[index]
                    .as_ref()
                    .is_some_and(|r| Arc::ptr_eq(r, retained.occurrence()))
            {
                return Err(E::ForeignOwner("bump animation interface"));
            }
        }
        self.animation = Some(Arc::clone(animation));
        Ok(())
    }
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.physical
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("bump physical"))?
            .phys_out()
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_motion_bump_acceleration()
            .map_err(MotionUpdateError::PhysOut)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        use MotionUpdateError as E;
        let services = self.actor.action_runtime_services().map_err(E::Actor)?;
        let attributes = &services
            .as_ref()
            .ok_or(E::MissingOwner("global anim_motion_bumps+158"))?
            .bumps;
        let physical = self
            .physical
            .as_ref()
            .ok_or(E::MissingOwner("bump physical"))?;
        if !Arc::ptr_eq(&attributes.vault, &physical.inputs().vault) {
            return Err(E::ForeignOwner("global anim_motion_bumps Vault"));
        }
        let Some(data) = attributes
            .instance
            .try_get_data(key, 0)
            .map_err(attribute_error)?
        else {
            return Ok(0.);
        };
        let word = data.bytes().try_into().map_err(|_| {
            attribute_error(
                crate::attribute_runtime::AttributeRuntimeError::ScalarSize {
                    size: data.bytes().len(),
                },
            )
        })?;
        Ok(f32::from_be_bytes(word))
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        use MotionUpdateError as E;
        let animation = self
            .animation
            .as_ref()
            .ok_or(E::MissingOwner("bump ISkaterAnim"))?
            .lock()
            .map_err(|_| E::Lock)?;
        let player = animation
            .player()
            .ok_or(E::MissingOwner("bump SkaterAnim player"))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(E::MissingOwner("bump mirrored flag"));
        }
        Ok(player.flags_bits & 0x4000_0000 != 0)
    }
    fn set_attribute(&mut self, index: usize, value: f32) -> Result<(), Self::Error> {
        self.animation
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("bump IAnimatable"))?
            .lock()
            .map_err(|_| MotionUpdateError::Lock)?
            .try_add_gesture_attribute(self.names[index], value, false, -1)
            .map_err(MotionUpdateError::AnimationWrite)
    }
}
struct Numeric;
impl ReckoningNumeric for Numeric {
    type Error = MotionUpdateError;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .dot3(a, b)
            .map_err(MotionUpdateError::ConditionNumeric)
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(v)
            .map_err(MotionUpdateError::ConditionNumeric)
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .reciprocal_estimate(v)
            .map_err(MotionUpdateError::ConditionNumeric)
    }
    fn truncate_i32(&mut self, v: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric
            .truncate_i32(v)
            .map_err(MotionUpdateError::ConditionNumeric)
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_set_bump_coefficients(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        x: FastString30,
        y: FastString30,
    ) -> Result<(), MotionUpdateError> {
        kernel::begin(
            &mut Source {
                actor: self,
                context,
                names: [x, y],
                physical: None,
                animation: None,
            },
            &mut Numeric,
        )
    }
}
