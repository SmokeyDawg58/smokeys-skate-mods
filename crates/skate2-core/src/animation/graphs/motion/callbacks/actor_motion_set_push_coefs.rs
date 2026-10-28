//! `SetPushCoefs`: Create82cba730, Begin82c95bf8,
//! Update82ca92f0 and Ramp82ca00b0; End825187c0 is a BLR.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::{
        action_motion_intent_bridge::disable_push::PushingMotionAttributes,
        animation_mount::MountedActorAnimation,
    },
    actor_graph_construction::ActorSkaterGraphState,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::FastString30,
};
use std::sync::{Arc, Mutex};

const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889); // kUpdateTime_56 at 0x822f2204
const RAMP_EPSILON: f32 = f32::from_bits(0x3a83_126f); // 82ca03bc

/// The shared native 0x0c instance has one initialized byte at +8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SetPushCoefsInstance {
    pub(crate) first_update: bool,
}
impl SetPushCoefsInstance {
    pub(crate) fn after_allocation(allocated: bool) -> Option<Self> {
        allocated.then_some(Self { first_update: true })
    }
    /// `Begin82c95bf8` writes byte +8 before any owner query.
    pub(crate) fn begin(&mut self) {
        self.first_update = true;
    }
}

pub(crate) trait SetPushCoefsSource {
    type Error;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error>;
    fn push(&mut self, index: usize) -> Result<f32, Self::Error>;
    fn speed(&mut self) -> Result<f32, Self::Error>;
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error>;
    fn dt(&mut self) -> Result<f32, Self::Error>;
    fn set_push(&mut self, index: usize, value: f32) -> Result<(), Self::Error>;
    fn animation_attribute(&mut self, name: &str, value: f32) -> Result<(), Self::Error>;
}

fn fsel_nonnegative(value: f32) -> f32 {
    if value >= 0.0 { value } else { 0.0 }
}

/// Native fneg/fsel/fsubs/fsel clamp at 82ca0460..04b0. NaN selects one.
fn clamp_unit(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

/// Native 82ca03b4..045c: absolute epsilon test, directional fused step,
/// then fsel saturation. All comparisons are unordered-aware by branch shape.
fn ramp_axis(current: f32, target: f32, dt: f32, rate: f32) -> f32 {
    let difference = target - current;
    if difference.abs() <= RAMP_EPSILON {
        return target;
    }
    if difference <= 0.0 {
        let step = (-dt).mul_add(rate, current); // fnmsubs
        if step - target >= 0.0 { step } else { target }
    } else {
        let step = dt.mul_add(rate, current); // fmadds
        if step - target >= 0.0 { target } else { step }
    }
}

pub(crate) fn ramp<S: SetPushCoefsSource>(source: &mut S) -> Result<(), S::Error> {
    source.resolve_interfaces()?;
    // Native GetPushState+0x38 and loads at 82ca02a8..02c4.
    let mut base_third = source.push(4)?;
    let target_first = source.push(5)?;
    let target_second = source.push(6)?;
    let target_third = source.push(7)?;
    let mut base_first = source.push(2)?;
    let mut base_second = source.push(3)?;
    // `bge` after fcmpu skips substitution for positive or unordered values.
    if base_third < 0.0 {
        base_first = target_first;
        base_second = target_second;
        base_third = target_third;
    }
    let speed = fsel_nonnegative(source.speed()?);
    let first_duration = source.graph(0x280, base_third)?;
    let first_rate = 1.0 / (first_duration * UPDATE_TIME);
    let third_duration = source.graph(0x2d0, speed)?;
    let third_rate = 1.0 / (third_duration * UPDATE_TIME);
    let dt = source.dt()?;
    let first = clamp_unit(ramp_axis(base_first, target_first, dt, first_rate));
    let second = clamp_unit(ramp_axis(base_second, target_second, dt, first_rate));
    let third = clamp_unit(ramp_axis(base_third, target_third, dt, third_rate));
    // Native stores all three before calling AddSettableAtt in the same order.
    source.set_push(2, first)?;
    source.set_push(3, second)?;
    source.set_push(4, third)?;
    source.animation_attribute("HStr_Vel_B", first)?;
    source.animation_attribute("LStr_Vel_B", second)?;
    source.animation_attribute("Vel_E", third)?;
    Ok(())
}

pub(crate) fn update<S: SetPushCoefsSource>(
    source: &mut S,
    instance: &mut SetPushCoefsInstance,
    on_first_update_only: bool,
) -> Result<(), S::Error> {
    if !on_first_update_only || instance.first_update {
        ramp(source)?;
    }
    instance.first_update = false;
    Ok(())
}

struct ActorSource<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a PushingMotionAttributes>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    dt: f32,
}
impl ActorSource<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("SetPushCoefs IPhysical"))
    }
    fn attributes(&self) -> Result<&PushingMotionAttributes, Error> {
        let attributes = self
            .attributes
            .ok_or(Error::MissingOwner("SetPushCoefs anim_motion_pushing+13c"))?;
        attributes.check_vault(&self.physical()?.inputs().vault)?;
        Ok(attributes)
    }
    fn with_motion_state<T>(
        &self,
        apply: impl FnOnce(&mut ActorSkaterGraphState) -> T,
    ) -> Result<T, Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.state.as_mut())
            .ok_or(Error::MissingOwner("SetPushCoefs PushState"))?;
        Ok(apply(state))
    }
}
impl SetPushCoefsSource for ActorSource<'_> {
    type Error = Error;
    fn resolve_interfaces(&mut self) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(Error::MissingOwner("SetPushCoefs published IAnimatable"))?;
        let retained = animation.lock().map_err(|_| Error::Lock)?;
        if !actor.animation_interfaces[0]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            || !retained
                .entity()
                .is_same_issuance(&self.actor.authority.entity_handle())
        {
            return Err(Error::ForeignOwner("SetPushCoefs IAnimatable"));
        }
        drop(retained);
        let animation = Arc::clone(animation);
        drop(actor);
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(Error::Physical)?;
        self.physical = Some(Arc::clone(physical.owner()));
        self.animation = Some(animation);
        Ok(())
    }
    fn push(&mut self, index: usize) -> Result<f32, Error> {
        if !matches!(index, 2..=7) {
            return Err(Error::MissingOwner("SetPushCoefs PushState index"));
        }
        self.with_motion_state(|state| state.push[index])
    }
    fn speed(&mut self) -> Result<f32, Error> {
        self.physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_z_speed()
            .map_err(Error::PhysOut)
    }
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Error> {
        self.attributes()?.first_push_graph(offset, input)
    }
    fn dt(&mut self) -> Result<f32, Error> {
        if self.context.controller().is_none() {
            return Err(Error::MissingOwner("SetPushCoefs controller"));
        }
        Ok(self.dt)
    }
    fn set_push(&mut self, index: usize, value: f32) -> Result<(), Error> {
        if !matches!(index, 2..=4) {
            return Err(Error::MissingOwner("SetPushCoefs PushState write index"));
        }
        self.with_motion_state(|state| state.push[index] = value)
    }
    fn animation_attribute(&mut self, name: &str, value: f32) -> Result<(), Error> {
        let animation = self
            .animation
            .as_ref()
            .ok_or(Error::MissingOwner("SetPushCoefs IAnimatable"))?;
        let name = FastString30::encode(name).expect("native SetPushCoefs attribute name");
        animation
            .lock()
            .map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(name, value, true, -1)
            .map_err(Error::AnimationWrite)
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_set_push_coefs(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: &mut SetPushCoefsInstance,
        on_first_update_only: bool,
        attributes: Option<&PushingMotionAttributes>,
        dt: f32,
    ) -> Result<(), Error> {
        update(
            &mut ActorSource {
                actor: self,
                context,
                attributes,
                physical: None,
                animation: None,
                dt,
            },
            instance,
            on_first_update_only,
        )
    }
}

#[cfg(test)]
#[path = "actor_motion_set_push_coefs_tests.rs"]
mod tests;
