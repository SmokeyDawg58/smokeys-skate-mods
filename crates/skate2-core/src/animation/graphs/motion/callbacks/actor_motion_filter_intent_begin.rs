//! FilterMotionGraphIntent::Begin82cd5e30, after the caller's Data writes.
use super::{ActorConstruction, ActorIdentity};
use crate::{
    generated_motion_load::runtime::MotionUpdateError,
    motion_graph_entity_authority::{BoundMotionGraphContext, MotionGraphInstanceToken},
    play_animation::FastString36,
};

impl ActorConstruction {
    pub(crate) fn motion_filter_intent_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        token: &MotionGraphInstanceToken,
        output_intent: FastString36,
        starting_value: f32,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        // Native reads Context.mInstance before resolving ISkaterMotionGraph.
        if !context
            .instance()
            .is_some_and(|current| current.is_same_issuance(token))
        {
            return Err(E::ForeignOwner("FilterMotionGraphIntent instance"));
        }
        self.check_motion_runtime_context(context)?;
        let mut actor = self.prefix().map_err(E::Actor)?;
        let motion = actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .ok_or(E::MissingOwner("Skater Motion graph"))?;
        let intents = motion
            .filtered_intents
            .as_mut()
            .ok_or(E::MissingOwner("Filtered Motion intents"))?;
        intents
            .insert_or_assign(output_intent, starting_value)
            .map_err(|_| E::Capacity)
    }

    /// FilterMotionGraphIntent::End82cc7388 removes the published output key.
    pub(crate) fn motion_filter_intent_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        output_intent: FastString36,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let mut actor = self.prefix().map_err(E::Actor)?;
        let motion = actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .ok_or(E::MissingOwner("Skater Motion graph"))?;
        let intents = motion
            .filtered_intents
            .as_mut()
            .ok_or(E::MissingOwner("Filtered Motion intents"))?;
        intents.erase(&output_intent);
        Ok(())
    }
}
