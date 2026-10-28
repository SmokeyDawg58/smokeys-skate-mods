//! Actual Actor/Vault adapter for `action_motion_intent_bridge::ground`.
use super::super::{
    ActionMotionIntentError, ActorConstruction, ActorIdentity, check_action, check_motion,
};
use super::{BodyFlippingSignalInstance, GroundActionServices, TimeIntentInstance};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_action_load::construct::BehaviourPayload,
    motion_graph_entity_authority::BoundMotionGraphContext,
    physics::FilteredPhysicsState,
    play_animation::FastString36,
};
use std::sync::Arc;

/// Actual selected GlobalAttributeCollection+128 allocation/binding, not all globals.
pub(crate) struct BodyFlipAttributes(AttributeRuntimeInstance);
impl BodyFlipAttributes {
    /// Global ctor828b2cec allocates10;828b2d10 binds collection3fc308e69aea6385.
    /// Generated `anim_motion828b1590` establishes class90c3decf0105631e/layout5c0.
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0x3fc3_08e6_9aea_6385);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self(instance))
    }
    fn read(&self, key: u64) -> Result<f32, GroundActionRuntimeError> {
        let data = self
            .0
            .try_get_data(key, 0)
            .map_err(GroundActionRuntimeError::Attribute)?;
        let Some(data) = data else { return Ok(0.0) }; //actual DefaultDataArea4
        let bytes = data
            .bytes()
            .try_into()
            .map_err(|_| GroundActionRuntimeError::AttributeSize(data.bytes().len()))?;
        Ok(f32::from_be_bytes(bytes))
    }
}
#[derive(Debug)]
pub(crate) enum GroundActionRuntimeError {
    Intent(ActionMotionIntentError),
    Attribute(AttributeRuntimeError),
    AttributeSize(usize),
    MissingGlobalBodyFlip,
}
impl std::fmt::Display for GroundActionRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Intent(e) => write!(f, "ground Action intent: {e}"),
            Self::Attribute(e) => write!(f, "ground Action attribute: {e}"),
            Self::AttributeSize(size) => write!(f, "ground Action scalar has {size} bytes"),
            Self::MissingGlobalBodyFlip => write!(f, "missing BodyFlip global"),
        }
    }
}
struct Services<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a BodyFlipAttributes>,
    dt: f32,
}
impl GroundActionServices for Services<'_> {
    type Error = GroundActionRuntimeError;
    fn action_intent_exists(&mut self, name: &FastString36) -> Result<bool, Self::Error> {
        self.actor
            .action_runtime_intent_value(self.context, name)
            .map(|v| v.is_some())
            .map_err(Self::Error::Intent)
    }
    fn require_motion(&mut self) -> Result<(), Self::Error> {
        let state = self
            .actor
            .prefix()
            .map_err(|e| Self::Error::Intent(ActionMotionIntentError::Actor(e)))?;
        check_motion(&state).map_err(Self::Error::Intent)
    }
    fn filtered_state(&mut self) -> Result<FilteredPhysicsState, Self::Error> {
        self.actor
            .action_runtime_filtered_state(self.context)
            .map_err(Self::Error::Intent)
    }
    fn delta_time(&mut self) -> Result<f32, Self::Error> {
        Ok(self.dt)
    }
    fn body_flip_extension(&mut self) -> Result<f32, Self::Error> {
        self.attributes
            .ok_or(Self::Error::MissingGlobalBodyFlip)?
            .read(0x48cf_d7fd_67dc_55a2)
    }
    fn takeoff_extension(&mut self) -> Result<f32, Self::Error> {
        self.attributes
            .ok_or(Self::Error::MissingGlobalBodyFlip)?
            .read(0x0307_546c_31db_9472)
    }
    fn insert_motion(&mut self, name: &FastString36, value: f32) -> Result<(), Self::Error> {
        let mut state = self
            .actor
            .prefix()
            .map_err(|e| Self::Error::Intent(ActionMotionIntentError::Actor(e)))?;
        check_motion(&state).map_err(Self::Error::Intent)?;
        state
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.intents.as_mut())
            .ok_or(Self::Error::Intent(ActionMotionIntentError::MissingMotion))?
            .insert_or_assign(*name, value)
            .map_err(|e| Self::Error::Intent(ActionMotionIntentError::Insert(e)))
    }
    fn erase_motion(&mut self, name: &FastString36) -> Result<(), Self::Error> {
        self.actor
            .remove_transferred_intent(name)
            .map_err(Self::Error::Intent)
    }
}
impl ActorConstruction {
    pub(crate) fn try_action_body_flip_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: &mut BodyFlippingSignalInstance,
        attributes: Option<&BodyFlipAttributes>,
    ) -> Result<(), GroundActionRuntimeError> {
        self.check_action_instance_context(context, instance.token())
            .map_err(GroundActionRuntimeError::Intent)?;
        instance.begin(&mut Services {
            actor: self,
            context,
            attributes,
            dt: 0.0,
        })
    }
    /// dt is the locked SAME generated controller's `delta_time`, not a new clock.
    pub(crate) fn try_action_body_flip_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: &mut BodyFlippingSignalInstance,
        attributes: Option<&BodyFlipAttributes>,
        dt: f32,
    ) -> Result<(), GroundActionRuntimeError> {
        self.check_action_instance_context(context, instance.token())
            .map_err(GroundActionRuntimeError::Intent)?;
        instance.update(&mut Services {
            actor: self,
            context,
            attributes,
            dt,
        })
    }
    pub(crate) fn try_action_time_intent_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        instance: &mut TimeIntentInstance,
        dt: f32,
    ) -> Result<(), GroundActionRuntimeError> {
        self.check_action_instance_context(context, instance.token())
            .map_err(GroundActionRuntimeError::Intent)?;
        let BehaviourPayload::TimeIntent { action, motion } = payload else {
            return Err(GroundActionRuntimeError::Intent(
                ActionMotionIntentError::WrongPayload,
            ));
        };
        // Native resolves Action before Motion, gets destination before source.
        {
            let state = self
                .prefix()
                .map_err(|e| GroundActionRuntimeError::Intent(ActionMotionIntentError::Actor(e)))?;
            check_action(&state).map_err(GroundActionRuntimeError::Intent)?;
        }
        instance.update(
            action,
            motion,
            &mut Services {
                actor: self,
                context,
                attributes: None,
                dt,
            },
        )
    }
    pub(crate) fn try_action_time_intent_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        instance: Option<&TimeIntentInstance>,
    ) -> Result<(), GroundActionRuntimeError> {
        if let Some(instance) = instance {
            self.check_action_instance_context(context, instance.token())
                .map_err(GroundActionRuntimeError::Intent)?;
        } else {
            self.check_action_null_ground_context(context)?;
        }
        let BehaviourPayload::TimeIntent { motion, .. } = payload else {
            return Err(GroundActionRuntimeError::Intent(
                ActionMotionIntentError::WrongPayload,
            ));
        };
        let mut services = Services {
            actor: self,
            context,
            attributes: None,
            dt: 0.0,
        };
        // End82cc2028 never reads the instance, including on allocation NULL.
        services.require_motion()?;
        services.erase_motion(motion)
    }
    fn check_action_null_ground_context(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), GroundActionRuntimeError> {
        if context.instance().is_some() {
            return Err(GroundActionRuntimeError::Intent(
                ActionMotionIntentError::ForeignContext,
            ));
        }
        self.check_action_runtime_context(context)
            .map_err(GroundActionRuntimeError::Intent)
    }
    pub(crate) fn try_action_time_intent_null_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
    ) -> Result<(), GroundActionRuntimeError> {
        self.check_action_null_ground_context(context)?;
        let BehaviourPayload::TimeIntent { action, .. } = payload else {
            return Err(GroundActionRuntimeError::Intent(
                ActionMotionIntentError::WrongPayload,
            ));
        };
        {
            let state = self
                .prefix()
                .map_err(|e| GroundActionRuntimeError::Intent(ActionMotionIntentError::Actor(e)))?;
            check_action(&state).map_err(GroundActionRuntimeError::Intent)?;
            check_motion(&state).map_err(GroundActionRuntimeError::Intent)?;
        }
        self.action_runtime_intent_value(context, action)
            .map_err(GroundActionRuntimeError::Intent)?;
        // Both native paths fault on timer access before inserting or erasing.
        Err(GroundActionRuntimeError::Intent(
            ActionMotionIntentError::NullInstance,
        ))
    }
    pub(crate) fn try_action_body_flip_null_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attributes: Option<&BodyFlipAttributes>,
    ) -> Result<(), GroundActionRuntimeError> {
        self.check_action_null_ground_context(context)?;
        attributes
            .ok_or(GroundActionRuntimeError::MissingGlobalBodyFlip)?
            .read(0x48cf_d7fd_67dc_55a2)?;
        // Raw48b8 faults after the first attribute read, before second lookup.
        Err(GroundActionRuntimeError::Intent(
            ActionMotionIntentError::NullInstance,
        ))
    }
    pub(crate) fn try_action_body_flip_null_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attributes: Option<&BodyFlipAttributes>,
        dt: f32,
    ) -> Result<(), GroundActionRuntimeError> {
        self.check_action_null_ground_context(context)?;
        let mut services = Services {
            actor: self,
            context,
            attributes,
            dt,
        };
        let name = crate::actor_graph_construction::ActorGraphIntents::native_name;
        if !services.action_intent_exists(&name(b"FrontFlip"))?
            && !services.action_intent_exists(&name(b"BackFlip"))?
        {
            services.body_flip_extension()?;
            services.delta_time()?;
        }
        // Front2234/Back22a4 fault on reset; absent2300 on the late timer read.
        // Neither path reaches TakeOffWindow or a Motion-map mutation.
        Err(GroundActionRuntimeError::Intent(
            ActionMotionIntentError::NullInstance,
        ))
    }
}
