//! `PushCycle::CreateInstance82caf9b0`, Begin82cd6328, Update82cd6590.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_construction::action_motion_intent_bridge::disable_push::PushingMotionAttributes,
    actor_graph_construction::ActorGraphIntents, input_cvar_runtime::InputCvarReferences,
    play_animation::FastString36,
};

const PUSHING_FLAG: u32 = 0x8000_0000;
const TOP_SPEED_KEY: u64 = 0x1a4c_a176_9dcc_b3a8;

/// Original fields at instance +8 and +0xc; refcount/vtable are host-owned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PushCycleInstance {
    pub(crate) stage: u32,
    pub(crate) change_velocity: f32,
}
impl PushCycleInstance {
    pub(crate) fn after_allocation(allocated: bool) -> Option<Self> {
        allocated.then_some(Self {
            stage: 1,
            change_velocity: 0.0,
        })
    }
}

/// Ordered source operations so the callback has one semantics in tests and live play.
pub(crate) trait Source {
    type Error;
    fn resolve_begin(&mut self) -> Result<(), Self::Error>;
    fn resolve_update(&mut self) -> Result<(), Self::Error>;
    fn graph(&mut self, offset: usize, x: f32) -> Result<f32, Self::Error>;
    fn allow_holding(&mut self) -> Result<bool, Self::Error>;
    fn has_intent(&mut self, name: &FastString36) -> Result<bool, Self::Error>;
    fn intent_value(&mut self, name: &FastString36) -> Result<f32, Self::Error>;
    fn pushing_flag(&mut self, enabled: bool) -> Result<(), Self::Error>;
    fn speed(&mut self) -> Result<f32, Self::Error>;
    fn max_holding_acc(&mut self) -> Result<f32, Self::Error>;
    fn top_speed(&mut self) -> Result<f32, Self::Error>;
    fn cap_dv_at_top_speed(&mut self) -> Result<bool, Self::Error>;
    fn push_dv(&mut self) -> Result<f32, Self::Error>;
    fn set_push_dv(&mut self, value: f32) -> Result<(), Self::Error>;
}

fn pushing() -> FastString36 {
    ActorGraphIntents::native_name(b"Pushing")
}
fn new_push() -> FastString36 {
    ActorGraphIntents::native_name(b"NewPush")
}

/// `MapButtonPressToChangeVel82c95328`: duration graph, two fsel clamps,
/// then change-velocity graph. Unordered comparisons follow the PPC fsel path.
fn map_button<S: Source>(source: &mut S, push_time: f32, speed: f32) -> Result<f32, S::Error> {
    let duration = source.graph(0x230, speed)?;
    let ratio = push_time / duration;
    let lower = if -ratio >= 0.0 { 0.0 } else { ratio };
    let clamped = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
    source.graph(0x1e0, clamped)
}

pub(crate) fn begin<S: Source>(
    source: &mut S,
    data: &mut PushCycleInstance,
    name: &FastString36,
) -> Result<(), S::Error> {
    source.resolve_begin()?;
    data.change_velocity = source.graph(0x1e0, 0.0)?;
    if source.allow_holding()? && source.has_intent(&pushing())? && source.has_intent(name)? {
        data.stage = 0;
        source.pushing_flag(true)?;
    } else {
        data.stage = 1;
        source.pushing_flag(false)?;
    }
    Ok(())
}

pub(crate) fn update<S: Source>(
    source: &mut S,
    data: &mut PushCycleInstance,
    name: &FastString36,
) -> Result<(), S::Error> {
    source.resolve_update()?;
    let raw_speed = source.speed()?;
    let speed = if raw_speed >= 0.0 { raw_speed } else { 0.0 };
    if source.allow_holding()? && data.stage == 0 {
        if source.has_intent(&pushing())? {
            let push_time = source.intent_value(&pushing())?;
            data.change_velocity = map_button(source, push_time, speed)?;
            let maximum = source.max_holding_acc()?;
            // `ble` tests !GT, so unordered `fcmpu` also skips this cap.
            if data.change_velocity > maximum {
                data.change_velocity = maximum;
                source.set_push_dv(maximum)?;
            }
            // `bge` tests !LT; unordered values skip the write.
            if source.push_dv()? < data.change_velocity {
                source.set_push_dv(data.change_velocity)?;
            }
        } else {
            data.stage = 1;
            data.change_velocity = source.graph(0x1e0, 0.0)?;
            source.pushing_flag(false)?;
        }
    }
    if data.stage == 1 && source.has_intent(&new_push())? && source.has_intent(name)? {
        data.stage = 2;
    }
    if data.stage == 2 && !source.has_intent(name)? {
        data.stage = 3;
        source.set_push_dv(data.change_velocity)?;
    }
    if data.stage == 2 {
        let top_speed = source.top_speed()?;
        let cap = source.cap_dv_at_top_speed()?;
        // fcmpu/bgt82cd6a40..44 treats unordered as inside the cap limit.
        if !cap || !(speed > top_speed) {
            let push_time = source.intent_value(&pushing())?;
            data.change_velocity = map_button(source, push_time, speed)?;
        }
        // `bge` tests !LT; unordered values skip the write.
        if source.push_dv()? < data.change_velocity {
            source.set_push_dv(data.change_velocity)?;
        }
        source.pushing_flag(true)?;
    }
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
    physical: Option<std::sync::Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>>,
}
impl ActorSource<'_> {
    fn attributes(&self) -> Result<&PushingMotionAttributes, Error> {
        let attributes = self
            .attributes
            .ok_or(Error::MissingOwner("PushCycle Pushing global"))?;
        if let Some(physical) = &self.physical {
            attributes.check_vault(&physical.inputs().vault)?;
        }
        Ok(attributes)
    }
    fn with_state<T>(
        &self,
        f: impl FnOnce(&mut crate::actor_graph_construction::ActorSkaterGraphState) -> T,
    ) -> Result<T, Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|m| m.state.as_mut())
            .ok_or(Error::MissingOwner("PushCycle PushState"))?;
        Ok(f(state))
    }
}
impl Source for ActorSource<'_> {
    type Error = Error;
    fn resolve_begin(&mut self) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)
    }
    fn resolve_update(&mut self) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        self.physical = Some(std::sync::Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(Error::Physical)?
                .owner(),
        ));
        Ok(())
    }
    fn graph(&mut self, offset: usize, x: f32) -> Result<f32, Error> {
        self.attributes()?.first_push_graph(offset, x)
    }
    fn allow_holding(&mut self) -> Result<bool, Error> {
        self.cvars
            .ok_or(Error::MissingOwner("kAllowHolding target"))?
            .try_allow_holding()
            .map_err(Error::Cvar)
    }
    fn has_intent(&mut self, name: &FastString36) -> Result<bool, Error> {
        Ok(self
            .actor
            .motion_runtime_intent_value(self.context, name)?
            .is_some())
    }
    fn intent_value(&mut self, name: &FastString36) -> Result<f32, Error> {
        Ok(self
            .actor
            .motion_runtime_intent_value(self.context, name)?
            .unwrap_or(0.0))
    }
    fn pushing_flag(&mut self, enabled: bool) -> Result<(), Error> {
        self.with_state(|s| {
            s.push_bits.bits = (s.push_bits.bits & !PUSHING_FLAG) | (u32::from(enabled) << 31);
            s.push_bits.known_mask |= PUSHING_FLAG;
        })
    }
    fn speed(&mut self) -> Result<f32, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("PushCycle IPhysical"))?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_motion_z_speed()
            .map_err(Error::PhysOut)
    }
    fn max_holding_acc(&mut self) -> Result<f32, Error> {
        self.attributes()?.max_holding_acc()
    }
    fn top_speed(&mut self) -> Result<f32, Error> {
        self.attributes()?.first_push_scalar(TOP_SPEED_KEY)
    }
    fn cap_dv_at_top_speed(&mut self) -> Result<bool, Error> {
        self.cvars
            .ok_or(Error::MissingOwner("kCapDvAtTopSpeed target"))?
            .try_cap_dv_at_top_speed()
            .map_err(Error::Cvar)
    }
    fn push_dv(&mut self) -> Result<f32, Error> {
        self.with_state(|s| s.push[1])
    }
    fn set_push_dv(&mut self, value: f32) -> Result<(), Error> {
        self.with_state(|s| s.push[1] = value)
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_push_cycle(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        data: &mut PushCycleInstance,
        name: &FastString36,
        attributes: Option<&PushingMotionAttributes>,
        cvars: Option<&InputCvarReferences>,
        phase: Phase,
    ) -> Result<(), Error> {
        let mut source = ActorSource {
            actor: self,
            context,
            attributes,
            cvars,
            physical: None,
        };
        match phase {
            Phase::Begin => begin(&mut source, data, name),
            Phase::Update => update(&mut source, data, name),
        }
    }
}

#[cfg(test)]
#[path = "actor_motion_push_cycle_tests.rs"]
mod tests;
