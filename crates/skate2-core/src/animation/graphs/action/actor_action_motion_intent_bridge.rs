//! Actual Actor maps for `CreateMGIntentFromAGIntent` callbacks.
//! Source Begin82cd6290, Update82cd62c8, End82cd4288; generated scheduling
//! must retain one instance per occurrence and overlay its token in Context.
#[path = "actor_action_board_adjust.rs"]
pub(crate) mod board_adjust;
#[path = "actor_action_const_intent.rs"]
pub(crate) mod constant;
#[path = "actor_action_disable_push.rs"]
pub(crate) mod disable_push;
#[path = "actor_action_gesture_trick.rs"]
pub(crate) mod gesture;
#[path = "actor_action_gesture_runtime.rs"]
pub(crate) mod gesture_runtime;
#[path = "actor_action_ground_callbacks.rs"]
pub(crate) mod ground;
#[path = "actor_action_heading_condition.rs"]
mod heading_condition;
use super::{
    ActorConstruction, ActorConstructionError, ActorConstructorPrefix, ActorIdentity, Arc,
};
use crate::{
    actor_graph_construction::{ActorGraphIntentError, ActorGraphIntents},
    generated_action_load::construct::{BehaviourPayload, Filter},
    motion_graph_entity_authority::{BoundMotionGraphContext, MotionGraphInstanceToken},
    play_animation::FastString36,
};

pub(crate) struct TransferIntentInstance {
    token: MotionGraphInstanceToken,
    just_begun: bool,
}

pub(crate) struct JuiceHookInstance {
    token: MotionGraphInstanceToken,
    trick_name: FastString36,
}
impl JuiceHookInstance {
    /// `DataCtor`82cbaaf8, after successful 0x20 allocation82cbab94.
    /// Global initializer82fabfd0 assigns literal "nullString", not empty.
    pub(crate) fn new() -> Self {
        Self {
            token: MotionGraphInstanceToken::issue(),
            trick_name: ActorGraphIntents::native_name(b"nullString"),
        }
    }
    pub(crate) fn token(&self) -> &MotionGraphInstanceToken {
        &self.token
    }
}
impl TransferIntentInstance {
    /// `CreateInstance`82cba730: successful 0xc allocation initializes byte8
    /// true before publication. A null allocation must remain a null slot.
    pub(crate) fn new() -> Self {
        Self {
            token: MotionGraphInstanceToken::issue(),
            just_begun: true,
        }
    }
    pub(crate) fn token(&self) -> &MotionGraphInstanceToken {
        &self.token
    }
}

#[derive(Debug)]
pub(crate) enum ActionMotionIntentError {
    Actor(ActorConstructionError),
    ForeignContext,
    WrongPayload,
    NullInstance,
    MissingAction,
    ForeignAction,
    MissingMotion,
    ForeignMotion,
    MissingAnimation,
    ForeignAnimation,
    UnknownAnimationFlags,
    BoardAdjustState(crate::generated_action_load::BoardAdjustStateError),
    Insert(ActorGraphIntentError),
    Physical(super::physical_mount::ActorPhysicalMountError),
    PhysOut(
        crate::physical_player_phys_out::construct::ConcretePhysOutError<std::convert::Infallible>,
    ),
}
impl std::fmt::Display for ActionMotionIntentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "Action intent Actor: {error}"),
            Self::Insert(error) => write!(f, "Action to Motion insertion: {error:?}"),
            Self::Physical(error) => write!(f, "Action condition physical Actor: {error}"),
            Self::PhysOut(error) => write!(f, "Action condition physical output: {error:?}"),
            other => write!(f, "Action to Motion callback: {other:?}"),
        }
    }
}

impl ActorConstruction {
    fn check_transfer_context(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: &TransferIntentInstance,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_action_instance_context(context, instance.token())
    }

    fn check_action_instance_context(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance_token: &MotionGraphInstanceToken,
    ) -> Result<(), ActionMotionIntentError> {
        if context.entity() != Some(&self.authority.entity_handle())
            || !context
                .instance()
                .is_some_and(|token| token.is_same_issuance(instance_token))
        {
            return Err(ActionMotionIntentError::ForeignContext);
        }
        self.check_action_runtime_context(context)
    }

    pub(crate) fn check_action_runtime_context(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), ActionMotionIntentError> {
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(ActionMotionIntentError::ForeignContext);
        }
        let state = self.prefix().map_err(ActionMotionIntentError::Actor)?;
        check_action(&state)?;
        let wrapper = state
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.action())
            .and_then(|action| action.controller.as_ref())
            .ok_or(ActionMotionIntentError::MissingAction)?;
        if context.controller() != Some(wrapper.controller_identity()) {
            return Err(ActionMotionIntentError::ForeignContext);
        }
        Ok(())
    }

    pub(crate) fn action_runtime_intent_value(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: &FastString36,
    ) -> Result<Option<f32>, ActionMotionIntentError> {
        self.check_action_runtime_context(context)?;
        self.action_intent_value_from_checked_entity(name)
    }

    /// Shared IActionGraph lookup after the caller validates its own graph context.
    pub(crate) fn action_intent_value_from_checked_entity(
        &self,
        name: &FastString36,
    ) -> Result<Option<f32>, ActionMotionIntentError> {
        let state = self.prefix().map_err(ActionMotionIntentError::Actor)?;
        check_action(&state)?;
        Ok(state
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.action())
            .and_then(|action| action.intents.as_ref())
            .ok_or(ActionMotionIntentError::MissingAction)?
            .get(name))
    }

    pub(crate) fn action_runtime_time_since_last_input(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<f32, ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        self.check_action_runtime_context(context)?;
        let allocation = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = allocation
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
        Ok(output
            .try_animation_output()
            .map_err(E::PhysOut)?
            .time_since_last_input)
    }

    pub(crate) fn action_runtime_filtered_state(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<crate::physics::FilteredPhysicsState, ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        self.check_action_runtime_context(context)?;
        let allocation = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = allocation
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
        Ok(output
            .try_filtered_output()
            .map_err(E::PhysOut)?
            .filtered_physics_state)
    }

    pub(crate) fn action_runtime_requests_dismount(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        self.check_action_runtime_context(context)?;
        let allocation = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = allocation
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
        // Activation82c98c70 reads PhysOut+1c State+43.
        Ok(output
            .try_state_output()
            .map_err(E::PhysOut)?
            .physics_requests_dismount)
    }

    /// Both generated Action and Motion conditions call the same native
    /// `IsGrinding` activation. Their graph-context checks remain separate;
    /// this shared read preserves its animation-before-physical order.
    pub(crate) fn read_is_grinding_for_condition(
        &self,
        specific_name: Option<&crate::play_animation::FastString30>,
    ) -> Result<bool, ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        // Activation82caa310 resolves/requires ISkaterAnim before IPhysical,
        // although it reads no animation value in either result branch.
        {
            let state = self.prefix().map_err(E::Actor)?;
            let animation = state
                .animation
                .as_ref()
                .filter(|_| state.animation_published)
                .ok_or(E::MissingAnimation)?
                .lock()
                .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
            if !state.animation_interfaces[2]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, animation.occurrence()))
            {
                return Err(E::ForeignAnimation);
            }
        }
        let allocation = self.cached_physical_allocation().map_err(E::Physical)?;
        {
            let output = allocation
                .owner()
                .phys_out()
                .lock()
                .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
            if !output
                .try_filtered_output()
                .map_err(E::PhysOut)?
                .is_grinding
            {
                return Ok(false);
            }
        }
        let Some(name) = specific_name else {
            return Ok(true);
        };
        // A second actual GetPhysOut read follows the first boolean query.
        let output = allocation
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
        Ok(output
            .try_filtered_output()
            .map_err(E::PhysOut)?
            .skating_grind_name
            == *name)
    }

    pub(crate) fn action_runtime_is_grinding(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        specific_name: Option<&crate::play_animation::FastString30>,
    ) -> Result<bool, ActionMotionIntentError> {
        self.check_action_runtime_context(context)?;
        self.read_is_grinding_for_condition(specific_name)
    }

    /// Begin/End bind the shared no-op825187c0; Update82cd4290 tailcalls
    /// `RemoveIntent`82cc2398. This actual root occurrence runs before transfers.
    pub(crate) fn try_action_juice_hook_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: &mut JuiceHookInstance,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_action_instance_context(context, instance.token())?;
        self.remove_transferred_intent(&instance.trick_name)?;
        // Raw82cc2464 constructs the temporary only after erase2454;
        // six stores248c..24a0 then reset the retained instance name.
        instance.trick_name = ActorGraphIntents::native_name(b"nullString");
        Ok(())
    }

    pub(crate) fn try_action_juice_hook_null_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_action_runtime_context(context)?;
        let state = self.prefix().map_err(ActionMotionIntentError::Actor)?;
        check_motion(&state)?;
        state
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.motion())
            .and_then(|motion| motion.intents.as_ref())
            .ok_or(ActionMotionIntentError::MissingMotion)?;
        // Motion lookup/getmap precede erase's read of null instance+8.
        Err(ActionMotionIntentError::NullInstance)
    }

    pub(crate) fn try_action_motion_intent_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        instance: &mut TransferIntentInstance,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_transfer_context(context, instance)?;
        self.create_transferred_intent(payload)?;
        instance.just_begun = true;
        Ok(())
    }

    /// Checked native null-instance fault occurs after `CreateIntent` in Begin,
    /// not before the actual destination-map side effects.
    pub(crate) fn try_action_motion_intent_null_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_action_runtime_context(context)?;
        self.create_transferred_intent(payload)?;
        Err(ActionMotionIntentError::NullInstance)
    }

    pub(crate) fn try_action_motion_intent_null_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_action_runtime_context(context)?;
        let BehaviourPayload::TransferIntent { on_update, .. } = payload else {
            return Err(ActionMotionIntentError::WrongPayload);
        };
        // onUpdate=false reads instance byte8 at62ec; true skips that read,
        // calls CreateIntent6310 and only faults at the suffix store6318.
        if *on_update {
            self.create_transferred_intent(payload)?;
        }
        Err(ActionMotionIntentError::NullInstance)
    }

    pub(crate) fn try_action_motion_intent_null_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_action_runtime_context(context)?;
        let BehaviourPayload::TransferIntent { motion, .. } = payload else {
            return Err(ActionMotionIntentError::WrongPayload);
        };
        self.remove_transferred_intent(motion)
    }

    pub(crate) fn try_action_motion_intent_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        instance: &mut TransferIntentInstance,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_transfer_context(context, instance)?;
        let BehaviourPayload::TransferIntent {
            motion, on_update, ..
        } = payload
        else {
            return Err(ActionMotionIntentError::WrongPayload);
        };
        if !on_update && !instance.just_begun {
            self.remove_transferred_intent(motion)?;
        }
        if *on_update {
            self.create_transferred_intent(payload)?;
        }
        // Raw82cd6318: the sole suffix follows both conditional calls.
        instance.just_begun = false;
        Ok(())
    }

    pub(crate) fn try_action_motion_intent_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        instance: &TransferIntentInstance,
    ) -> Result<(), ActionMotionIntentError> {
        self.check_transfer_context(context, instance)?;
        let BehaviourPayload::TransferIntent { motion, .. } = payload else {
            return Err(ActionMotionIntentError::WrongPayload);
        };
        // End is a tail branch to RemoveIntent, with no byte8 write.
        self.remove_transferred_intent(motion)
    }

    fn remove_transferred_intent(
        &self,
        name: &FastString36,
    ) -> Result<(), ActionMotionIntentError> {
        let mut state = self.prefix().map_err(ActionMotionIntentError::Actor)?;
        check_motion(&state)?;
        state
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.intents.as_mut())
            .ok_or(ActionMotionIntentError::MissingMotion)?
            .erase(name);
        Ok(())
    }

    fn create_transferred_intent(
        &self,
        payload: &BehaviourPayload,
    ) -> Result<(), ActionMotionIntentError> {
        use ActionMotionIntentError as E;
        let BehaviourPayload::TransferIntent {
            motion,
            action,
            default_value,
            has_default,
            scale,
            first,
            second,
            fakie,
            mirror,
            ..
        } = payload
        else {
            return Err(E::WrongPayload);
        };
        let mut state = self.prefix().map_err(E::Actor)?;
        // Source resolves IActionGraph then IMotionGraph, then gets destination
        // map before source map. Neither source map nor unrelated entries clear.
        check_action(&state)?;
        check_motion(&state)?;
        state
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .and_then(|g| g.intents.as_ref())
            .ok_or(E::MissingMotion)?;
        let source_value = state
            .graphs
            .as_ref()
            .and_then(|g| g.action())
            .and_then(|g| g.intents.as_ref())
            .ok_or(E::MissingAction)?
            .get(action);
        if source_value.is_none() && !has_default {
            state
                .graphs
                .as_mut()
                .and_then(|g| g.motion_mut())
                .and_then(|g| g.intents.as_mut())
                .expect("validated same map")
                .erase(motion);
            return Ok(());
        }
        let mut value = *scale * source_value.unwrap_or(*default_value);
        // ApplyFilters82c98fa0: cached ISkaterAnim must match UCOM occurrence.
        // For a concrete nonnull Actor its cached-null case asserts in source.
        let animation = state
            .animation
            .as_ref()
            .filter(|_| state.animation_published)
            .ok_or(E::MissingAnimation)?;
        {
            let animation = animation
                .lock()
                .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
            if !state.animation_interfaces[2]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, animation.occurrence()))
            {
                return Err(E::ForeignAnimation);
            }
        }
        value = apply_filter(apply_filter(value, *first), *second);
        // GetIsRidingFakie then GetIsMirrored, after each preceding filter.
        for (mask, filter) in [(0x2000_0000, *fakie), (0x4000_0000, *mirror)] {
            let enabled = {
                let animation = animation
                    .lock()
                    .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
                let player = animation.player().ok_or(E::MissingAnimation)?;
                if player.flags_known_mask & mask != mask {
                    return Err(E::UnknownAnimationFlags);
                }
                player.flags_bits & mask != 0
            };
            if enabled {
                value = apply_filter(value, filter);
            }
        }
        state
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.intents.as_mut())
            .expect("validated same map")
            .insert_or_assign(*motion, value)
            .map_err(E::Insert)
    }
}

fn check_action(state: &ActorConstructorPrefix) -> Result<(), ActionMotionIntentError> {
    use ActionMotionIntentError as E;
    let graph = state
        .graphs
        .as_ref()
        .and_then(|g| g.action())
        .filter(|g| state.action_graph_published && g.is_complete())
        .ok_or(E::MissingAction)?;
    if !state.graph_interfaces[0]
        .as_ref()
        .is_some_and(|r| Arc::ptr_eq(r, graph.occurrence()))
    {
        return Err(E::ForeignAction);
    }
    Ok(())
}
fn check_motion(state: &ActorConstructorPrefix) -> Result<(), ActionMotionIntentError> {
    use ActionMotionIntentError as E;
    let graph = state
        .graphs
        .as_ref()
        .and_then(|g| g.motion())
        .filter(|g| state.motion_graph_published && g.is_complete())
        .ok_or(E::MissingMotion)?;
    if !state.graph_interfaces[1]
        .as_ref()
        .is_some_and(|r| Arc::ptr_eq(r, graph.occurrence()))
    {
        return Err(E::ForeignMotion);
    }
    Ok(())
}

/// Raw `ApplyFilter`82c8bfd8: signed `OneMinus`, ordered fsel, single wrap.
#[expect(clippy::neg_multiply, reason = "raw fmuls by minus one, not fneg")]
pub(crate) fn apply_filter(value: f32, filter: Filter) -> f32 {
    use std::f32::consts::{FRAC_PI_2, PI, TAU};
    match filter {
        Filter::None => value,
        Filter::Negate => value * -1.0,
        Filter::Abs => value.abs(),
        Filter::OneMinus => {
            if value >= 0.0 {
                1.0 - value
            } else {
                -1.0 - value
            }
        }
        Filter::Clamp => {
            let low = if -value >= 0.0 { 0.0 } else { value };
            if 1.0 - low >= 0.0 { low } else { 1.0 }
        }
        Filter::AngleFlip => {
            if value >= 0.0 {
                PI - value
            } else {
                -PI - value
            }
        }
        Filter::AngleRot90 => {
            let angle = value - FRAC_PI_2;
            if angle < -PI { angle + TAU } else { angle }
        }
        Filter::AngleRotN90 => {
            let angle = value + FRAC_PI_2;
            if angle > PI { angle - TAU } else { angle }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Filter, apply_filter};
    #[test]
    fn signed_filters_preserve_source_zero_nan_and_single_angle_wrap() {
        assert_eq!(
            apply_filter(-0.0, Filter::OneMinus).to_bits(),
            1.0_f32.to_bits()
        );
        assert_eq!(
            apply_filter(-0.25, Filter::OneMinus).to_bits(),
            (-0.75_f32).to_bits()
        );
        assert_eq!(
            apply_filter(-0.0, Filter::Clamp).to_bits(),
            0.0_f32.to_bits()
        );
        // fsel chooses its negative/unordered operand; the second select
        // therefore returns one for NaN. f32::clamp would preserve NaN.
        assert_eq!(
            apply_filter(f32::NAN, Filter::Clamp).to_bits(),
            1.0_f32.to_bits()
        );
        assert_eq!(
            apply_filter(f32::NEG_INFINITY, Filter::Clamp).to_bits(),
            0.0_f32.to_bits()
        );
        assert_eq!(
            apply_filter(f32::INFINITY, Filter::Clamp).to_bits(),
            1.0_f32.to_bits()
        );
        assert!(apply_filter(20.0, Filter::AngleRotN90) > std::f32::consts::PI);
        assert!(apply_filter(-20.0, Filter::AngleRot90) < -std::f32::consts::PI);
        assert_eq!(
            apply_filter(0.0, Filter::Negate).to_bits(),
            (-0.0_f32).to_bits()
        );
    }
}
