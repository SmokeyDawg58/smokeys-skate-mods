//! `ComputeFirstPushStrength`: Create82ca0c90, Begin82ca0b18,
//! Update82cc54b0, and `MapButtonPressToChangeVel82c95328`.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::action_motion_intent_bridge::disable_push::PushingMotionAttributes,
    actor_graph_construction::ActorGraphIntents, attribute_runtime::AttributeRuntimeData,
    input_cvar_runtime::InputCvarReferences, physical_player_assembly::PhysicalPlayerAssemblyOwner,
};
use std::sync::Arc;

const ACTIVE: u32 = 0x4000_0000;
const TELEPORT: u32 = 0x2000_0000;
const FIRST_UPDATE: u32 = 0x8000_0000;

/// Only the top three native flag bits are read by this callback. The low bits
/// are not established by `CreateInstance` and have no host representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FirstPushInstance {
    pub(crate) elapsed: f32,
    pub(crate) flags: u32,
}
impl FirstPushInstance {
    /// Native allocation is 0x10 bytes, with elapsed+8 and flags+0xc.
    pub(crate) fn after_allocation(allocated: bool) -> Option<Self> {
        allocated.then_some(Self {
            elapsed: 0.0,
            flags: FIRST_UPDATE | ACTIVE,
        })
    }
}

pub(crate) trait FirstPushSource {
    type Error;
    type Attribute;
    fn resolve_begin(&mut self) -> Result<(), Self::Error>;
    fn resolve_update(&mut self) -> Result<(), Self::Error>;
    fn time_since_teleport(&mut self) -> Result<f32, Self::Error>;
    fn capture_teleport_threshold(&mut self) -> Result<Self::Attribute, Self::Error>;
    fn read_teleport_threshold(&mut self, attribute: &Self::Attribute) -> Result<f32, Self::Error>;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn has_pushing_intent(&mut self) -> Result<bool, Self::Error>;
    fn pushing_intent_value(&mut self) -> Result<f32, Self::Error>;
    fn dt(&mut self) -> Result<f32, Self::Error>;
    fn z_speed(&mut self) -> Result<f32, Self::Error>;
    fn cap_dv_at_top_speed(&mut self) -> Result<bool, Self::Error>;
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error>;
    fn current_push_dv(&mut self) -> Result<f32, Self::Error>;
    fn set_push_dv(&mut self, dv: f32) -> Result<(), Self::Error>;
}

pub(crate) fn begin<S: FirstPushSource>(
    source: &mut S,
    instance: &mut FirstPushInstance,
) -> Result<(), S::Error> {
    // The instance writes precede every owner/global lookup.
    instance.elapsed = 0.0;
    instance.flags |= FIRST_UPDATE | ACTIVE;
    source.resolve_begin()?;
    let pointer = source.capture_teleport_threshold()?;
    let time = source.time_since_teleport()?;
    let threshold = source.read_teleport_threshold(&pointer)?;
    let recent_teleport = time < threshold;
    instance.flags = (instance.flags & !TELEPORT) | if recent_teleport { TELEPORT } else { 0 };
    Ok(())
}

/// Native `fsel` clamps: a negative or unordered speed selects zero.
fn nonnegative_speed(speed: f32) -> f32 {
    if speed >= 0.0 { speed } else { 0.0 }
}

/// `fdivs`, `fneg`, then two `fsel` operations. Unordered ratio selects one.
fn clamped_ratio(time: f32, duration: f32) -> f32 {
    let ratio = time / duration;
    let lower = if -ratio >= 0.0 { 0.0 } else { ratio };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

#[expect(
    clippy::assign_op_pattern,
    reason = "native fadds uses delta time as left operand"
)]
pub(crate) fn update<S: FirstPushSource>(
    source: &mut S,
    instance: &mut FirstPushInstance,
) -> Result<(), S::Error> {
    source.resolve_update()?;
    if !source.has_pushing_intent()? {
        instance.flags &= !ACTIVE;
    }
    if instance.flags & (ACTIVE | TELEPORT) != 0 {
        let push_time = if instance.flags & TELEPORT != 0 {
            let old = instance.elapsed;
            instance.elapsed = source.dt()? + instance.elapsed;
            old
        } else {
            // The separate `HasIntent` call happens only on this branch. Its
            // boolean is ignored; an absent entry leaves the output at zero.
            source.pushing_intent_value()?
        };
        let speed = nonnegative_speed(source.z_speed()?);
        // Fresh scalar lookup precedes the live CVar target read.
        let top_speed = source.scalar(0x1a4c_a176_9dcc_b3a8)?;
        let over_top_speed = speed > top_speed;
        let cap = source.cap_dv_at_top_speed()?;
        let push_time = if cap && over_top_speed {
            0.0
        } else {
            push_time
        };
        let duration = source.graph(0x230, speed)?;
        let ratio = clamped_ratio(push_time, duration);
        let dv = source.graph(0x1e0, ratio)?;
        let previous = source.current_push_dv()?;
        // PPC `fcmpu` + `ble` skips an ordered <= result. An unordered
        // comparison reaches the store, unlike Rust's ordinary `>` test.
        if !(dv <= previous) {
            source.set_push_dv(dv)?;
        }
    }
    instance.flags &= !FIRST_UPDATE;
    Ok(())
}

pub(crate) enum Phase {
    Begin,
    Update,
}

struct ActorSource<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a PushingMotionAttributes>,
    cvars: Option<&'a InputCvarReferences>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    dt: f32,
}
impl ActorSource<'_> {
    fn attributes(&self) -> Result<&PushingMotionAttributes, Error> {
        let attributes = self
            .attributes
            .ok_or(Error::MissingOwner("Global anim_motion_pushing+13c"))?;
        if let Some(physical) = &self.physical {
            attributes.check_vault(&physical.inputs().vault)?;
        }
        Ok(attributes)
    }
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("ComputeFirstPushStrength IPhysical"))
    }
    fn with_motion_state<T>(
        &self,
        apply: impl FnOnce(&mut crate::actor_graph_construction::ActorSkaterGraphState) -> T,
    ) -> Result<T, Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.state.as_mut())
            .ok_or(Error::MissingOwner("ComputeFirstPushStrength PushState"))?;
        Ok(apply(state))
    }
}
impl FirstPushSource for ActorSource<'_> {
    type Error = Error;
    type Attribute = Option<AttributeRuntimeData>;
    fn resolve_begin(&mut self) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)
    }
    fn resolve_update(&mut self) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        self.physical = Some(Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(Error::Physical)?
                .owner(),
        ));
        Ok(())
    }
    fn time_since_teleport(&mut self) -> Result<f32, Error> {
        self.with_motion_state(|state| state.time_since_teleport)
    }
    fn capture_teleport_threshold(&mut self) -> Result<Self::Attribute, Error> {
        self.attributes()?
            .first_push_scalar_pointer(0x4c4b_9cdf_5860_f565)
    }
    fn read_teleport_threshold(&mut self, attribute: &Self::Attribute) -> Result<f32, Error> {
        PushingMotionAttributes::first_push_scalar_value(attribute.as_ref())
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Error> {
        self.attributes()?.first_push_scalar(key)
    }
    fn has_pushing_intent(&mut self) -> Result<bool, Error> {
        let name = ActorGraphIntents::native_name(b"Pushing");
        self.actor
            .motion_runtime_intent_value(self.context, &name)
            .map(|value| value.is_some())
    }
    fn pushing_intent_value(&mut self) -> Result<f32, Error> {
        let name = ActorGraphIntents::native_name(b"Pushing");
        self.actor
            .motion_runtime_intent_value(self.context, &name)
            .map(|value| value.unwrap_or(0.0))
    }
    fn dt(&mut self) -> Result<f32, Error> {
        if self.context.controller().is_none() {
            return Err(Error::MissingOwner("ComputeFirstPushStrength controller"));
        }
        Ok(self.dt)
    }
    fn z_speed(&mut self) -> Result<f32, Error> {
        self.physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_z_speed()
            .map_err(Error::PhysOut)
    }
    fn cap_dv_at_top_speed(&mut self) -> Result<bool, Error> {
        self.cvars
            .ok_or(Error::MissingOwner("kCapDvAtTopSpeed target"))?
            .try_cap_dv_at_top_speed()
            .map_err(Error::Cvar)
    }
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Error> {
        self.attributes()?.first_push_graph(offset, input)
    }
    fn current_push_dv(&mut self) -> Result<f32, Error> {
        self.with_motion_state(|state| state.push[1])
    }
    fn set_push_dv(&mut self, dv: f32) -> Result<(), Error> {
        self.with_motion_state(|state| state.push[1] = dv)
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_compute_first_push(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: &mut FirstPushInstance,
        attributes: Option<&PushingMotionAttributes>,
        cvars: Option<&InputCvarReferences>,
        dt: f32,
        phase: Phase,
    ) -> Result<(), Error> {
        let mut source = ActorSource {
            actor: self,
            context,
            attributes,
            cvars,
            physical: None,
            dt,
        };
        match phase {
            Phase::Begin => begin(&mut source, instance),
            Phase::Update => update(&mut source, instance),
        }
    }
}

#[cfg(test)]
#[path = "actor_motion_compute_first_push_tests.rs"]
mod tests;
