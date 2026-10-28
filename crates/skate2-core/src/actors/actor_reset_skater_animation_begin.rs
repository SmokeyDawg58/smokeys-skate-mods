//! `ResetSkaterAnimation::Begin` 0x82ca4f10 on the retained Actor owners.
//! All three interface lookups/assertions precede the ordered reset calls.
use super::{ActorConstruction, ActorIdentity};
use crate::{
    generated_motion_load::runtime::MotionUpdateError as E,
    motion_graph_entity_authority::BoundMotionGraphContext,
};
use std::sync::Arc;

impl ActorConstruction {
    /// `ResetToGivenStance::Begin` 0x82ca2ea0 resolves the same published
    /// ISkaterAnim and invokes its ResetToSpawnStance vtable slot +0x2c.
    pub(crate) fn try_reset_to_given_stance_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(E::ForeignOwner("ResetToGivenStance IActor"));
        }
        let actor = self.prefix().map_err(E::Actor)?;
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(E::MissingOwner("ResetToGivenStance ISkaterAnim"))?;
        let mut retained = animation.lock().map_err(|_| E::Lock)?;
        if !retained
            .entity()
            .is_same_issuance(&self.authority.entity_handle())
            || !actor.animation_interfaces[2]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
        {
            return Err(E::ForeignOwner("ResetToGivenStance ISkaterAnim"));
        }
        retained
            .try_reset_to_spawn_stance()
            .map_err(E::ResetToSpawnStance)
    }

    pub(crate) fn try_reset_skater_animation_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let mut actor = self.prefix().map_err(E::Actor)?;

        // Source 4f28..50d0 resolves IActionGraph, ISkaterAnim and
        // ISkaterMotionGraph before calling any of their reset functions.
        let graphs = actor
            .graphs
            .as_ref()
            .ok_or(E::MissingOwner("Actor graphs"))?;
        let action = graphs
            .action()
            .filter(|graph| actor.action_graph_published && graph.is_complete())
            .ok_or(E::MissingOwner("ResetSkaterAnimation IActionGraph"))?;
        if !actor.graph_interfaces[0]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, action.occurrence()))
        {
            return Err(E::ForeignOwner("ResetSkaterAnimation IActionGraph"));
        }
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(E::MissingOwner("ResetSkaterAnimation ISkaterAnim"))?;
        let retained = animation.lock().map_err(|_| E::Lock)?;
        if !retained
            .entity()
            .is_same_issuance(&self.authority.entity_handle())
            || !actor.animation_interfaces[2]
                .as_ref()
                .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
        {
            return Err(E::ForeignOwner("ResetSkaterAnimation ISkaterAnim"));
        }
        drop(retained);
        let animation = Arc::clone(animation);
        let motion = graphs
            .motion()
            .filter(|graph| actor.motion_graph_published && graph.is_complete())
            .ok_or(E::MissingOwner("ResetSkaterAnimation ISkaterMotionGraph"))?;
        if !actor.graph_interfaces[2]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, motion.occurrence()))
        {
            return Err(E::ForeignOwner("ResetSkaterAnimation ISkaterMotionGraph"));
        }

        // Source 5114, 5128, 513c: use the already mounted occurrences.
        let graphs = actor.graphs.as_mut().expect("preflight graphs");
        graphs
            .action_mut()
            .expect("preflight IActionGraph")
            .try_reset_live()
            .map_err(E::ActionGraphReset)?;
        graphs
            .motion_mut()
            .expect("preflight ISkaterMotionGraph")
            .try_reset_live()
            .map_err(E::SkaterMotionGraphReset)?;
        drop(actor);
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_reset_skater_anim()
            .map_err(E::SkaterAnimReset)
    }
}
