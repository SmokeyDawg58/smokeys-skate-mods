//! Actual constant intent lifecycle; proposed intent-bridge child.
use super::{ActionMotionIntentError, ActorConstruction, ActorIdentity, check_motion};
use crate::{
    generated_action_load::construct::BehaviourPayload,
    motion_graph_entity_authority::{BoundMotionGraphContext, MotionGraphInstanceToken},
    play_animation::FastString36,
};

pub(crate) struct ConstIntentInstance {
    token: MotionGraphInstanceToken,
    just_begun: bool,
}
impl ConstIntentInstance {
    /// Successful `CreateInstance827126f8`: allocation0xc/alignment16, byte8=false.
    pub(crate) fn new() -> Self {
        Self {
            token: MotionGraphInstanceToken::issue(),
            just_begun: false,
        }
    }
    pub(crate) fn token(&self) -> &MotionGraphInstanceToken {
        &self.token
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ConstIntentError<E> {
    Source(E),
    NullInstance,
}

pub(crate) fn begin<E>(
    instance: Option<&mut ConstIntentInstance>,
    insert: impl FnOnce() -> Result<(), E>,
) -> Result<(), ConstIntentError<E>> {
    insert().map_err(ConstIntentError::Source)?;
    instance.ok_or(ConstIntentError::NullInstance)?.just_begun = true;
    Ok(())
}
pub(crate) fn update<E>(
    instance: Option<&mut ConstIntentInstance>,
    on_update: bool,
    erase: impl FnOnce() -> Result<(), E>,
) -> Result<(), ConstIntentError<E>> {
    // Read byte8 only when onUpdate is false; all paths eventually write it.
    if !on_update
        && !instance
            .as_deref()
            .ok_or(ConstIntentError::NullInstance)?
            .just_begun
    {
        erase().map_err(ConstIntentError::Source)?;
    }
    instance.ok_or(ConstIntentError::NullInstance)?.just_begun = false;
    Ok(())
}
fn translate(error: ConstIntentError<ActionMotionIntentError>) -> ActionMotionIntentError {
    match error {
        ConstIntentError::Source(e) => e,
        ConstIntentError::NullInstance => ActionMotionIntentError::NullInstance,
    }
}
impl ActorConstruction {
    fn check_const_context(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&ConstIntentInstance>,
    ) -> Result<(), ActionMotionIntentError> {
        if let Some(instance) = instance {
            self.check_action_instance_context(context, instance.token())
        } else {
            if context.instance().is_some() {
                return Err(ActionMotionIntentError::ForeignContext);
            }
            self.check_action_runtime_context(context)
        }
    }
    fn insert_const_intent(
        &self,
        name: &FastString36,
        value: f32,
    ) -> Result<(), ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        let mut state = self.prefix().map_err(E::Actor)?;
        check_motion(&state)?;
        state
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.intents.as_mut())
            .ok_or(E::MissingMotion)?
            .insert_or_assign(*name, value)
            .map_err(E::Insert)
    }
    fn erase_const_intent(&self, name: &FastString36) -> Result<(), ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        let mut state = self.prefix().map_err(E::Actor)?;
        // Native RemoveIntent admits a truly absent cached AND UCOM interface.
        if state.graphs.as_ref().and_then(|g| g.motion()).is_none()
            && state.graph_interfaces[1].is_none()
        {
            return Ok(());
        }
        check_motion(&state)?;
        state
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.intents.as_mut())
            .ok_or(E::MissingMotion)?
            .erase(name);
        Ok(())
    }
    pub(crate) fn action_const_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        mut instance: Option<&mut ConstIntentInstance>,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_const_context(context, instance.as_deref())?;
        let BehaviourPayload::ConstIntent { motion, value, .. } = payload else {
            return Err(ActionMotionIntentError::WrongPayload);
        };
        begin(instance.take(), || self.insert_const_intent(motion, *value)).map_err(translate)
    }
    pub(crate) fn action_const_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        mut instance: Option<&mut ConstIntentInstance>,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_const_context(context, instance.as_deref())?;
        let BehaviourPayload::ConstIntent {
            motion, on_update, ..
        } = payload
        else {
            return Err(ActionMotionIntentError::WrongPayload);
        };
        update(instance.take(), *on_update, || {
            self.erase_const_intent(motion)
        })
        .map_err(translate)
    }
    pub(crate) fn action_const_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        instance: Option<&ConstIntentInstance>,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_const_context(context, instance)?;
        let BehaviourPayload::ConstIntent { motion, .. } = payload else {
            return Err(ActionMotionIntentError::WrongPayload);
        };
        self.erase_const_intent(motion)
    }
}
#[cfg(test)]
#[path = "actor_action_const_intent_tests.rs"]
mod tests;
