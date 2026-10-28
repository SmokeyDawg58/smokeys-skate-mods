//! Actual `TemplateHC<Action>::Update`82955c78 and retained lifecycle.
//! Selection reads actual Actor predicates. Unrecovered branches stop at their
//! source call boundary; no selected-state or successful-callback substitute.
use super::{
    BehaviourClass, BehaviourInstanceSlot, BoardAdjustInstance, ConditionClass,
    GeneratedActionAllocation, GeneratedActionController, GeneratedActionInstance,
    GeneratedActionLoadError, NULL_INDEX, ObjectDomain,
    construct::{ComparisonOp, ConditionPayload, FilteredState},
    ids::state as state_id,
};
use crate::{
    actor_construction::{
        ActorConstruction, ActorIdentity,
        action_motion_intent_bridge::{
            ActionMotionIntentError, JuiceHookInstance, TransferIntentInstance,
        },
    },
    dynamic_state_graph_controller::DynamicStateGraphContext,
    motion_graph_entity_authority::{BoundMotionGraphContext, MotionGraphInstanceToken},
    physics::FilteredPhysicsState,
    state_graph_context::StateGraphContextHandle,
};
use std::sync::Arc;
#[path = "generated_action_runtime_antic_dispatch.rs"]
mod antic_dispatch;
#[path = "generated_action_runtime_ground_dispatch.rs"]
mod ground_dispatch;
#[path = "generated_action_runtime_ground_selection.rs"]
mod ground_selection;
#[path = "generated_action_runtime_grounded_transfer_dispatch.rs"]
mod grounded_transfer_dispatch;
#[path = "generated_action_runtime_heading_condition.rs"]
mod heading_condition;
#[path = "generated_action_runtime_selection.rs"]
mod selection;
#[path = "generated_action_runtime_trick_dispatch.rs"]
mod trick_dispatch;
#[path = "generated_action_runtime_trick_selection.rs"]
mod trick_selection;

#[derive(Debug)]
pub(crate) enum ActionUpdateError {
    GestureCondition(crate::actor_construction::action_motion_intent_bridge::gesture_runtime::GestureRuntimeError),
    Gesture(crate::actor_construction::action_motion_intent_bridge::gesture::GestureTrickError<crate::actor_construction::action_motion_intent_bridge::gesture_runtime::GestureRuntimeError>),
    SharedPhysical(crate::generated_motion_load::runtime::MotionUpdateError),
    AnimationAttribute(crate::generated_motion_load::runtime::MotionUpdateError),
    Load(GeneratedActionLoadError),
    Actor(ActionMotionIntentError),
    Ground(crate::actor_construction::action_motion_intent_bridge::ground::runtime::GroundActionRuntimeError),
    DisablePush(crate::actor_construction::action_motion_intent_bridge::disable_push::DisablePushError<
        crate::actor_construction::action_motion_intent_bridge::disable_push::DisablePushActorError,
        crate::reckoning_numeric::ReckoningHostNumericError>),
    Lock,
    InvalidState(u16),
    InvalidActiveSet,
    MissingInstance(u16),
    Capacity,
    NoSelectedState,
    InvalidTransition(u16),
    InvalidInterruptType(u8),
    PendingTransitionHook(u16),
    PendingMaskedExpression {
        mask: selection::Mask,
        expression: u16,
    },
    PendingExpression(u16),
    PendingStateUpdate(u32),
    PendingCallback {
        index: u16,
        address: u32,
    },
}
impl std::fmt::Display for ActionUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GestureCondition(error) => write!(f, "Action gesture condition: {error}"),
            Self::Gesture(error) => write!(f, "Action gesture callback: {error}"),
            Self::SharedPhysical(error) => write!(f, "Action shared physical condition: {error}"),
            Self::AnimationAttribute(error) => write!(f, "Action animation attribute: {error}"),
            Self::Load(error) => write!(f, "Action runtime load: {error}"),
            Self::Actor(error) => write!(f, "Action runtime Actor: {error}"),
            Self::Ground(error) => write!(f, "Action runtime ground: {error}"),
            Self::DisablePush(error) => write!(f, "Action runtime push condition: {error}"),
            Self::InvalidState(index) => write!(f, "invalid Action state {index}"),
            Self::InvalidTransition(index) => write!(f, "invalid Action transition {index}"),
            Self::InvalidInterruptType(kind) => write!(f, "invalid Action interrupt type {kind}"),
            Self::PendingTransitionHook(index) => {
                write!(f, "Action transition {index} hook pending")
            }
            Self::PendingMaskedExpression { mask, expression } => {
                write!(f, "Action {mask:?} expression {expression} pending")
            }
            Self::MissingInstance(index) => {
                write!(f, "null Action instance for occurrence {index}")
            }
            Self::PendingExpression(index) => write!(f, "Action expression {index} pending"),
            Self::PendingStateUpdate(address) => {
                write!(f, "Action state Update {address:08x} pending")
            }
            Self::PendingCallback { index, address } => write!(
                f,
                "Action occurrence {index} callback {address:08x} pending"
            ),
            other => write!(f, "Action runtime: {other:?}"),
        }
    }
}
impl From<GeneratedActionLoadError> for ActionUpdateError {
    fn from(value: GeneratedActionLoadError) -> Self {
        Self::Load(value)
    }
}
impl From<ActionMotionIntentError> for ActionUpdateError {
    fn from(value: ActionMotionIntentError) -> Self {
        Self::Actor(value)
    }
}

impl GeneratedActionController<ActorIdentity> {
    /// Retained `Update82955c78`, including subsequent transition/sustain search.
    pub(crate) fn try_update(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
        dt: f32,
        allocate: &mut impl FnMut(u16, usize) -> GeneratedActionAllocation,
    ) -> Result<(), ActionUpdateError> {
        self.validate_context(context)?;
        actor.check_action_runtime_context(context)?;
        self.condition_active.fill(0);
        self.delta_time = dt;
        self.allow_evaluations = true;
        // A checked predicate failure preserves allowEvaluations=true: source
        // has not returned from GetNextState to the clearing store yet.
        let graph = selection::Graph {
            states: self.states,
            transitions: self.transitions,
            indices: self.transition_indices,
            current: self.current_state,
        };
        let selected = graph.next(&mut |mask, expression, parent| {
            selection::evaluate_expression(mask, expression, |index| {
                self.initial_condition(actor, context, index, parent)
            })
        })?;
        self.allow_evaluations = false;
        if selected.state == NULL_INDEX {
            return Err(ActionUpdateError::NoSelectedState);
        }
        let (exited, entered) = graph.changes(&selected)?;
        for time in self.time_in_state.values_mut() {
            // Keep native dt first, including floating-point NaN propagation.
            let updated_time = dt + *time;
            *time = updated_time;
        }
        self.last_state = self.current_state;
        self.current_state = selected.state;
        self.exit_states(actor, context, &exited)?;
        // All 129 original Action transition declarations have zero hooks.
        // Preserve the actual post-Exit hook boundary for any nonempty table.
        if selected.transition != NULL_INDEX
            && self.transitions[usize::from(selected.transition)]
                .hooks
                .count
                != 0
        {
            return Err(ActionUpdateError::PendingTransitionHook(
                selected.transition,
            ));
        }
        self.enter_states(actor, context, &entered, allocate)?;
        self.update_selected_behaviours(actor, context)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one source condition wrapper preserves parent publication and result caching around dispatch"
    )]
    fn initial_condition(
        &mut self,
        actor: &ActorConstruction,
        context: &BoundMotionGraphContext<ActorIdentity>,
        index: u16,
        parent: u16,
    ) -> Result<bool, ActionUpdateError> {
        let position = usize::from(index);
        let word = position / 32;
        let mask = 1_u32 << (position % 32);
        if self.condition_active[word] & mask != 0 {
            return self.condition_values[position].ok_or(ActionUpdateError::InvalidActiveSet);
        }
        let condition =
            self.statics
                .condition(position)?
                .ok_or(GeneratedActionLoadError::NullObject {
                    domain: ObjectDomain::Condition,
                    index: position,
                })?;
        // Direct wrapper sets the shared condition's actual tagged parent before
        // its ActivationFunc, including on a later checked activation failure.
        *condition
            .base
            .parent
            .lock()
            .map_err(|_| ActionUpdateError::Lock)? = Some(parent);
        let value = match (&condition.class, &condition.payload) {
            (ConditionClass::HasGestureIntent, ConditionPayload::Gesture(group)) => {
                let services = actor
                    .action_runtime_services()
                    .map_err(ActionMotionIntentError::Actor)?;
                actor
                    .action_runtime_has_gesture(
                        context,
                        *group,
                        services.as_ref().and_then(|s| s.gesture.as_ref()),
                    )
                    .map_err(ActionUpdateError::GestureCondition)?
            }
            (ConditionClass::InParentStateForTime, ConditionPayload::Comparison(comparison)) => {
                let tagged_parent = *condition
                    .base
                    .parent
                    .lock()
                    .map_err(|_| ActionUpdateError::Lock)?;
                super::conditions::parent_time_condition(
                    &self.time_in_state,
                    self.states.len(),
                    tagged_parent,
                    *comparison,
                    ActionUpdateError::InvalidState,
                )?
            }
            (ConditionClass::IsLandingOnBoard, ConditionPayload::Plain) => {
                actor.action_runtime_landing_on_board(context)?
            }
            (ConditionClass::IsInLocomotion, ConditionPayload::Plain) => {
                actor.action_runtime_is_in_locomotion(context)?
            }
            (ConditionClass::CurrentState, ConditionPayload::CurrentState(name)) => {
                crate::generated_motion_load::runtime::grounded_selection::state_matches(
                    self.states,
                    parent,
                    self.current_state,
                    name,
                )
                .map_err(ActionUpdateError::SharedPhysical)?
            }
            (ConditionClass::IsMirrored, ConditionPayload::Plain) => actor
                .action_runtime_is_mirrored(context)
                .map_err(ActionUpdateError::GestureCondition)?,
            (ConditionClass::IsRidingFakie, ConditionPayload::Plain) => {
                actor.check_action_runtime_context(context)?;
                actor
                    .runtime_is_riding_fakie_from_checked_context()
                    .map_err(ActionUpdateError::SharedPhysical)?
            }
            (
                ConditionClass::PhysicsSpeedCompare,
                ConditionPayload::Speed {
                    comparison,
                    along_skate_z,
                },
            ) => {
                actor.check_action_runtime_context(context)?;
                actor
                    .runtime_speed_compare_from_checked_context(*comparison, *along_skate_z)
                    .map_err(ActionUpdateError::SharedPhysical)?
            }
            (ConditionClass::IsHandPlanting, ConditionPayload::HandPlant { state, direction }) => {
                actor.check_action_runtime_context(context)?;
                actor
                    .runtime_handplant(*state, *direction)
                    .map_err(ActionUpdateError::SharedPhysical)?
            }
            (ConditionClass::PhysicsWantsRunout, ConditionPayload::Plain) => {
                let physical = actor
                    .cached_physical_allocation()
                    .map_err(ActionMotionIntentError::Physical)?;
                actor
                    .runtime_requests_runout(&physical.owner().inputs().cvars)
                    .map_err(ActionUpdateError::SharedPhysical)?
            }
            (ConditionClass::DisablePushBrake, ConditionPayload::Plain) => {
                let services = actor
                    .action_runtime_services()
                    .map_err(ActionMotionIntentError::Actor)?;
                actor
                    .action_runtime_disable_push(
                        context,
                        services.as_ref().map(|s| &s.pushing),
                        &mut crate::reckoning_numeric::HostReckoningNumeric,
                    )
                    .map_err(ActionUpdateError::DisablePush)?
            }
            (
                ConditionClass::HasAGIntent,
                ConditionPayload::ActionIntent { comparison, intent },
            ) => actor
                .action_runtime_intent_value(context, intent)?
                .is_some_and(|value| {
                    comparison.op == ComparisonOp::Nop
                        || super::conditions::compare(*comparison, value)
                }),
            (ConditionClass::TimeToLand, ConditionPayload::Comparison(comparison)) => {
                actor.check_action_runtime_context(context)?;
                actor
                    .runtime_time_to_land(*comparison)
                    .map_err(ActionUpdateError::SharedPhysical)?
            }
            (ConditionClass::OBTimeToLand, ConditionPayload::Comparison(comparison)) => {
                // OBTimeToLand::ActivationFunc82c9aff0 compares the OffBoard
                // PhysOut float at +0x1c, with no air-trajectory validity gate.
                actor.check_action_runtime_context(context)?;
                let physical = actor.cached_physical_allocation().map_err(|error| {
                    ActionUpdateError::Actor(ActionMotionIntentError::Physical(error))
                })?;
                let output = physical
                    .owner()
                    .phys_out()
                    .lock()
                    .map_err(|_| ActionUpdateError::Lock)?;
                let time = output
                    .try_offboard_output()
                    .map_err(|error| {
                        ActionUpdateError::Actor(ActionMotionIntentError::PhysOut(error))
                    })?
                    .time_to_land;
                super::conditions::compare(*comparison, time)
            }
            (ConditionClass::TimeSinceLastInput, ConditionPayload::Comparison(comparison)) => {
                super::conditions::compare(
                    *comparison,
                    actor.action_runtime_time_since_last_input(context)?,
                )
            }
            (ConditionClass::PhysFilteredState, ConditionPayload::PhysicalState(expected)) => {
                actor.action_runtime_filtered_state(context)? == filtered_state(*expected)
            }
            (ConditionClass::IsGrinding, ConditionPayload::Grinding { has_specific, name }) => {
                actor.action_runtime_is_grinding(context, has_specific.then_some(name))?
            }
            (ConditionClass::IsGrabbingObject, ConditionPayload::Plain) => {
                // IsGrabbingObject::ActivationFunc82c9aef0 resolves this
                // Action-bound IPhysical, then reads PhysOut.OffBoard+0x38.
                // Motion has the same output read, but its context belongs to
                // a different generated controller.
                actor.check_action_runtime_context(context)?;
                let physical = actor.cached_physical_allocation().map_err(|error| {
                    ActionUpdateError::Actor(ActionMotionIntentError::Physical(error))
                })?;
                let output = physical
                    .owner()
                    .phys_out()
                    .lock()
                    .map_err(|_| ActionUpdateError::Lock)?;
                output
                    .try_offboard_output()
                    .map_err(|error| {
                        ActionUpdateError::Actor(ActionMotionIntentError::PhysOut(error))
                    })?
                    .is_grabbing_object
            }
            (ConditionClass::IsFootPlanting, ConditionPayload::Plain) => {
                // IsFootPlanting::ActivationFunc82ca7830 resolves the same
                // Action-bound IPhysical, reads PhysOut.Air +1b0, and only when
                // set compares +c4 with `fcmpu; bge`, so unordered passes.
                actor.check_action_runtime_context(context)?;
                let physical = actor.cached_physical_allocation().map_err(|error| {
                    ActionUpdateError::Actor(ActionMotionIntentError::Physical(error))
                })?;
                let output = physical
                    .owner()
                    .phys_out()
                    .lock()
                    .map_err(|_| ActionUpdateError::Lock)?;
                let air = output.try_air_output().map_err(|error| {
                    ActionUpdateError::Actor(ActionMotionIntentError::PhysOut(error))
                })?;
                air.is_foot_planting
                    && (air.foot_plant_total_absorb_time >= 0.0
                        || air.foot_plant_total_absorb_time.is_nan())
            }
            // 82caefe0 requires IAnimatable and negates attribute presence;
            // it never evaluates the inherited comparison payload.
            (ConditionClass::IsDoneAirDismount, ConditionPayload::Comparison(_)) => {
                actor.check_action_runtime_context(context)?;
                actor
                    .runtime_required_tree_attribute_from_checked_context(
                        crate::play_animation::FastString30::encode("OB_AirDismounting")
                            .expect("source literal fits FastString30"),
                    )
                    .map_err(ActionUpdateError::AnimationAttribute)?
                    .is_none()
            }
            (ConditionClass::IsDismounting, ConditionPayload::Comparison(_)) => actor
                .action_runtime_is_dismounting(context)
                .map_err(ActionUpdateError::GestureCondition)?,
            (
                ConditionClass::HasAnimAttribute,
                ConditionPayload::AnimationAttribute {
                    comparison,
                    attribute,
                    sequence,
                },
            ) => {
                actor.check_action_runtime_context(context)?;
                match actor
                    .runtime_tree_attribute_from_checked_context(*attribute)
                    .map_err(ActionUpdateError::AnimationAttribute)?
                {
                    None => false,
                    Some(value) => {
                        if *sequence != -1 {
                            return Err(ActionUpdateError::PendingCallback {
                                index,
                                address: condition.class.binding().callbacks[1],
                            });
                        }
                        if comparison.op == ComparisonOp::Nop {
                            true
                        } else {
                            let scalar =
                                value
                                    .scalar_value()
                                    .ok_or(ActionUpdateError::PendingCallback {
                                        index,
                                        address: condition.class.binding().callbacks[1],
                                    })?;
                            super::conditions::compare(*comparison, scalar)
                        }
                    }
                }
            }
            (ConditionClass::PhysicsRequestsDismount, ConditionPayload::Plain) => {
                actor.action_runtime_requests_dismount(context)?
            }
            (
                ConditionClass::PhysicsSpeedAndSlopeCompare,
                ConditionPayload::Comparison(comparison),
            ) => heading_condition::evaluate_heading_condition(actor, context, *comparison)?,
            _ => {
                return Err(ActionUpdateError::PendingCallback {
                    index,
                    address: condition.class.binding().callbacks[1],
                });
            }
        };
        self.condition_active[word] |= mask;
        self.condition_values[position] = Some(value);
        Ok(value)
    }

    fn enter_states(
        &mut self,
        actor: &ActorConstruction,
        context: &BoundMotionGraphContext<ActorIdentity>,
        entered: &[u16],
        allocate: &mut impl FnMut(u16, usize) -> GeneratedActionAllocation,
    ) -> Result<(), ActionUpdateError> {
        for &state in entered {
            if !self.time_in_state.contains_key(&state) && self.time_in_state.len() == 8 {
                return Err(ActionUpdateError::Capacity);
            }
            self.time_in_state.insert(state, 0.0);
            let section = self.states[usize::from(state)].behaviours;
            for index in section.index..section.index.saturating_add(section.count) {
                let object = self.statics.behaviour(usize::from(index))?.ok_or(
                    GeneratedActionLoadError::NullObject {
                        domain: ObjectDomain::Behaviour,
                        index: usize::from(index),
                    },
                )?;
                if !object.base.enabled {
                    continue;
                }
                let instance = match object.class {
                    BehaviourClass::CreateTrickIntentFromGesture => match allocate(index, 0x40) {
                        GeneratedActionAllocation::Host => Some(GeneratedActionInstance::Gesture(
                            crate::actor_construction::action_motion_intent_bridge::gesture::GestureTrickInstance::new())),
                        GeneratedActionAllocation::Null => None,
                    },
                    BehaviourClass::BodyFlippingSignal => match allocate(index, 0x2c) {
                        GeneratedActionAllocation::Host => Some(GeneratedActionInstance::BodyFlip(
                            crate::actor_construction::action_motion_intent_bridge::ground::BodyFlippingSignalInstance::new())),
                        GeneratedActionAllocation::Null => None,
                    },
                    BehaviourClass::CreateMGTimeIntentFromAGIntent => match allocate(index, 0xc) {
                        GeneratedActionAllocation::Host => Some(GeneratedActionInstance::TimeIntent(
                            crate::actor_construction::action_motion_intent_bridge::ground::TimeIntentInstance::new())),
                        GeneratedActionAllocation::Null => None,
                    },
                    BehaviourClass::CreateConstMGIntent => match allocate(index, 0xc) {
                        GeneratedActionAllocation::Host => Some(GeneratedActionInstance::ConstIntent(
                            crate::actor_construction::action_motion_intent_bridge::constant::ConstIntentInstance::new(),
                        )),
                        GeneratedActionAllocation::Null => None,
                    },
                    BehaviourClass::CreateMGIntentFromAGIntent => match allocate(index, 0xc) {
                        GeneratedActionAllocation::Host => Some(
                            GeneratedActionInstance::TransferIntent(TransferIntentInstance::new()),
                        ),
                        GeneratedActionAllocation::Null => None,
                    },
                    BehaviourClass::JuiceHook => match allocate(index, 0x20) {
                        GeneratedActionAllocation::Host => {
                            Some(GeneratedActionInstance::JuiceHook(JuiceHookInstance::new()))
                        }
                        GeneratedActionAllocation::Null => None,
                    },
                    BehaviourClass::PrintText2D => None, // actual CreateInstance82bb3da8
                    BehaviourClass::BoardAdjust => match allocate(index, 0x10) {
                        GeneratedActionAllocation::Host => {
                            Some(GeneratedActionInstance::BoardAdjust(BoardAdjustInstance::new()))
                        }
                        GeneratedActionAllocation::Null => None,
                    },
                };
                let position = self
                    .current_behaviours
                    .binary_search(&index)
                    .unwrap_or_else(|p| p);
                if self.current_behaviours.get(position) == Some(&index) {
                    return Err(ActionUpdateError::InvalidActiveSet);
                }
                if self.current_behaviours.len() == 42 {
                    return Err(ActionUpdateError::Capacity);
                }
                for slot in (position..self.current_behaviours.len()).rev() {
                    self.instance_slots[slot + 1] =
                        std::mem::replace(&mut self.instance_slots[slot], empty_slot());
                }
                self.instance_slots[position] = BehaviourInstanceSlot {
                    behaviour: Some(object),
                    instance,
                };
                self.current_behaviours.insert(position, index);
                if self.call_behaviours {
                    let mut callback_context = context.copy_for_behaviour();
                    self.dispatch_callback(
                        actor,
                        &mut callback_context,
                        position,
                        Callback::Begin,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn exit_states(
        &mut self,
        actor: &ActorConstruction,
        context: &BoundMotionGraphContext<ActorIdentity>,
        exited: &[u16],
    ) -> Result<(), ActionUpdateError> {
        for position in (0..self.current_behaviours.len()).rev() {
            let index = self.current_behaviours[position];
            let parent = super::data::BEHAVIOURS[usize::from(index)].parent;
            if parent != NULL_INDEX && exited.contains(&parent) {
                if self.call_behaviours {
                    let mut callback_context = context.copy_for_behaviour();
                    self.dispatch_callback(actor, &mut callback_context, position, Callback::End)?;
                }
                let old_count = self.current_behaviours.len();
                self.current_behaviours.remove(position);
                for slot in position..old_count - 1 {
                    self.instance_slots[slot] =
                        std::mem::replace(&mut self.instance_slots[slot + 1], empty_slot());
                }
                self.instance_slots[old_count - 1] = empty_slot();
            }
        }
        for state in exited {
            if self.time_in_state.remove(state).is_none() {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
        }
        Ok(())
    }

    fn update_selected_behaviours(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), ActionUpdateError> {
        for (position, &index) in self.current_behaviours.iter().enumerate() {
            let static_object = self
                .statics
                .behaviour(usize::from(index))?
                .ok_or(ActionUpdateError::InvalidActiveSet)?;
            if !self.instance_slots[position]
                .behaviour
                .as_ref()
                .is_some_and(|actual| Arc::ptr_eq(actual, &static_object))
            {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
        }
        if !self.call_behaviours {
            return Ok(());
        }
        let address = self.states[usize::from(self.current_state)].update_address;
        if let Some((expected_address, expected)) =
            additional_shared_update_slots(self.current_state)
        {
            if address != expected_address || self.current_behaviours != expected {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..expected.len() {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        if matches!(
            self.current_state,
            state_id::TRICK_FROM_SQUARE_52..=state_id::TRICK_FROM_NOSE_54
        ) {
            return self.update_selected_trick(actor, context);
        }
        if self.current_state == state_id::IDLE_TURNING_94 {
            return self.update_ground_idle(actor, context);
        }
        // AccidentalDismount31 folds Action0 and OffBoard2 once, followed by
        // its OffBoardAir constant-intent occurrence at packed slot38.
        if self.current_state == state_id::ACCIDENTAL_DISMOUNT_31 && address == 0x828e_d528 {
            let expected: Vec<u16> = (0..38).chain(std::iter::once(104)).collect();
            if self.current_behaviours != expected {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..39 {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // StateUpdate_Default_30 0x828ed4b0 calls OffBoard_2 once, then
        // packed slots 38..40. Their retained behaviour occurrences are
        // root0..10, OffBoard11..37, and this state's 101..103.
        if self.current_state == state_id::DEFAULT_30 && address == 0x828e_d4b0 {
            let expected: Vec<u16> = (0..38).chain(101..104).collect();
            if self.current_behaviours != expected {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..41 {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // `StateUpdate_ForceBrake_4` 828ecf80 is also the folded update for
        // TeleportOffBoard7 and both ToOffBoard states8/9. The native thunk
        // updates Action0 once, then the two packed CreateConstMGIntent slots.
        if address == 0x828e_cf80
            && matches!(
                self.current_state,
                state_id::FORCE_BRAKE_4
                    | state_id::TELEPORT_OFF_BOARD_7
                    | state_id::TRANS_TO_OFF_BOARD_8
                    | state_id::TRANSITIONING_TO_OFF_BOARD_9
            )
        {
            let leaf = match self.current_state {
                state_id::FORCE_BRAKE_4 => [39, 40],
                state_id::TELEPORT_OFF_BOARD_7 => [50, 51],
                state_id::TRANS_TO_OFF_BOARD_8 => [52, 53],
                state_id::TRANSITIONING_TO_OFF_BOARD_9 => [54, 55],
                _ => unreachable!(),
            };
            let mut expected = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 0];
            expected[11..].copy_from_slice(&leaf);
            if self.current_behaviours != expected {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..expected.len() {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // HippyJumping47 and Pushing48 share the folded OnGround19 thunk.
        // Its four leaf intent callbacks follow the 31 inherited slots.
        if matches!(
            self.current_state,
            state_id::HIPPY_JUMPING_47 | state_id::PUSHING_48
        ) && address == 0x828e_d6a0
        {
            let leaf = if self.current_state == state_id::HIPPY_JUMPING_47 {
                [120, 121, 122, 123]
            } else {
                [124, 125, 126, 127]
            };
            let mut expected = [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 68, 69, 70, 71,
                72, 73, 74, 75, 76, 77, 78, 79, 0, 0, 0, 0,
            ];
            expected[31..].copy_from_slice(&leaf);
            if self.current_behaviours != expected {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..35 {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // Kickturn50 calls OnGround19 once, then its NewKickTurn constant
        // intent and KickTurn transfer at packed slots31 and32.
        if self.current_state == state_id::KICKTURN_50 && address == 0x828e_d728 {
            if self.current_behaviours
                != [
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 68, 69, 70,
                    71, 72, 73, 74, 75, 76, 77, 78, 79, 129, 130,
                ]
            {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..33 {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // DBLGrab80 calls the OnGround19 thunk once, then its two
        // CreateConstMGIntent leaves at packed slots31 and32.
        if self.current_state == state_id::DBL_GRAB_80 && address == 0x828e_d830 {
            if self.current_behaviours
                != [
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 68, 69, 70,
                    71, 72, 73, 74, 75, 76, 77, 78, 79, 118, 148,
                ]
            {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..33 {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        if address == 0x828e_d930 {
            return self.update_grounded_transfer(actor, context);
        }
        if address == 0x828e_dbc8 {
            return self.update_ground_antic(actor, context);
        }
        // Regular/Goofy grab leaves160..163, shared native828edb50:
        // OnGround ancestry, grab slots31..35, then const/transfer/transfer.
        if address == 0x828e_db50 {
            let expected: &[u16] = match self.current_state {
                state_id::GOOFY_160 => &[
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 68, 69, 70,
                    71, 72, 73, 74, 75, 76, 77, 78, 79, 118, 149, 150, 151, 152, 237, 238, 239,
                ],
                state_id::REGULAR_161 => &[
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 68, 69, 70,
                    71, 72, 73, 74, 75, 76, 77, 78, 79, 118, 149, 150, 151, 152, 240, 241, 242,
                ],
                state_id::GOOFY_162 => &[
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 68, 69, 70,
                    71, 72, 73, 74, 75, 76, 77, 78, 79, 118, 153, 154, 155, 156, 243, 244, 245,
                ],
                state_id::REGULAR_163 => &[
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 68, 69, 70,
                    71, 72, 73, 74, 75, 76, 77, 78, 79, 118, 153, 154, 155, 156, 246, 247, 248,
                ],
                _ => return Err(ActionUpdateError::PendingStateUpdate(address)),
            };
            if self.current_behaviours != expected {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..expected.len() {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // Shared StateUpdate828edc40: 23 inherited slots, then a constant
        // intent. The native descriptor table includes ground and air families.
        if address == 0x828e_dc40 {
            let (air, leaf) = match self.current_state {
                state_id::DOUBLE_GRAB_69 => (false, 138),
                state_id::DOUBLE_GRAB_113 => (false, 202),
                state_id::DOUBLE_GRAB_117 => (false, 203),
                state_id::DOUBLE_GRAB_121 => (false, 204),
                state_id::DOUBLE_GRAB_125 => (false, 205),
                state_id::REGULAR_127..=state_id::SWITCH_130 => {
                    (false, 206 + self.current_state - state_id::REGULAR_127)
                }
                state_id::REGULAR_180..=state_id::SWITCH_195 => {
                    (false, 281 + self.current_state - state_id::REGULAR_180)
                }
                state_id::DBL_GRAB_71 => (true, 139),
                state_id::LEFT_GRAB_73 | state_id::NEW_ONE_FOOT_132 | state_id::IDLE_138 => {
                    (true, 142)
                }
                state_id::RIGHT_GRAB_74 | state_id::NEW_ONE_FOOT_140 | state_id::IDLE_146 => {
                    (true, 143)
                }
                state_id::MUTE_GRAB_147 | state_id::IDLE_206 => (true, 222),
                state_id::STALE_GRAB_148 | state_id::IDLE_209 => (true, 223),
                _ => return Err(ActionUpdateError::PendingStateUpdate(address)),
            };
            let tail = if air {
                [62, 63, 113, 114]
            } else {
                [58, 59, 60, 61]
            };
            let expected: Vec<u16> = (0..11).chain(42..50).chain(tail).chain([leaf]).collect();
            if self.current_behaviours != expected {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..24 {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // Wipeout23 uses the same folded thunk: OnBoard then four intent slots.
        if self.current_state == state_id::WIPEOUT_23 && address == 0x828e_d980 {
            if self.current_behaviours
                != [
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 90, 91, 92,
                    93,
                ]
            {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..23 {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // Switch104 and Up/Down.Idle151/155 share the OnBoard prefix and packed slots0..22.
        // Idle155 is a thunk to Regular158; its tail ends at BoardAdjust147 Update.
        // StateUpdate_Switch_104 828ed980 also serves the LateFlips Regular95..105
        // and Switch96..106 leaves: OnBoard6 prefix, then four
        // CreateMGIntentFromAGIntent updates.
        if (matches!(
            self.current_state,
            state_id::REGULAR_95..=state_id::SWITCH_106
        ) && address == 0x828e_d980)
            || (matches!(self.current_state, state_id::IDLE_151 | state_id::IDLE_155)
                && address == 0x828e_dac0)
        {
            let late_flip_tail = match self.current_state {
                state_id::REGULAR_95 => Some(162),
                state_id::SWITCH_96 => Some(165),
                state_id::REGULAR_97 => Some(168),
                state_id::SWITCH_98 => Some(171),
                state_id::REGULAR_99 => Some(174),
                state_id::SWITCH_100 => Some(177),
                state_id::REGULAR_101 => Some(180),
                state_id::SWITCH_102 => Some(183),
                state_id::REGULAR_103 => Some(186),
                state_id::REGULAR_105 => Some(192),
                state_id::SWITCH_106 => Some(195),
                _ => None,
            };
            let expected = if let Some(first) = late_flip_tail {
                [
                    0,
                    1,
                    2,
                    3,
                    4,
                    5,
                    6,
                    7,
                    8,
                    9,
                    10,
                    42,
                    43,
                    44,
                    45,
                    46,
                    47,
                    48,
                    49,
                    87,
                    first,
                    first + 1,
                    first + 2,
                ]
            } else if self.current_state == state_id::SWITCH_104 {
                [
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 87, 189, 190,
                    191,
                ]
            } else if self.current_state == state_id::IDLE_151 {
                [
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 144,
                    145,
                ]
            } else {
                [
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 146,
                    147,
                ]
            };
            if self.current_behaviours != expected {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..23 {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // Grinding16 and its leaves, At90/AtN90 and their TrickFrom90 children:
        // StateUpdate_Switch_104 828ed980 (OnBoard_6, then four
        // CreateMGIntentFromAGIntent updates), the RightGrab_120 828eda58 tail
        // branch to it, and TrickFrom90Nose_108 828eda08 (Switch_104, then
        // CreateTrickIntentFromGesture). Every sharing state's classes checked.
        let tail: Option<&[u16]> = match (self.current_state, address) {
            (16, 0x828e_d980)
            | (
                34..=36 | 63..=68 | 70 | 111 | 112 | 114..=116 | 118..=120 | 122..=124 | 126,
                0x828e_da58,
            ) => Some(&[58, 59, 60, 61][..]),
            (61, 0x828e_d980) => Some(&[88, 89, 134, 135][..]),
            (62, 0x828e_d980) => Some(&[88, 89, 136, 137][..]),
            (107, 0x828e_da08) => Some(&[88, 89, 134, 135, 198][..]),
            (108, 0x828e_da08) => Some(&[88, 89, 134, 135, 199][..]),
            (109, 0x828e_da08) => Some(&[88, 89, 136, 137, 200][..]),
            (110, 0x828e_da08) => Some(&[88, 89, 136, 137, 201][..]),
            _ => None,
        };
        if let Some(tail) = tail {
            let prefix: [u16; 19] = [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49,
            ];
            if self.current_behaviours.len() != prefix.len() + tail.len()
                || self.current_behaviours[..prefix.len()] != prefix
                || self.current_behaviours[prefix.len()..] != *tail
            {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..self.current_behaviours.len() {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        // Native folded updates: Coffin18 (OnBoard + four leaves),
        // RocketAir154 (Regular158 + one constant), FingerFlip204
        // (Switch104 + two constants). Descriptor aliases share these thunks.
        if let Some(count) = match address {
            0x828e_d168 => Some(23),
            0x828e_dcf0 => Some(24),
            0x828e_da60 => Some(25),
            _ => None,
        } {
            let prefix = [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49,
            ];
            if self.current_behaviours.len() != count || self.current_behaviours[..19] != prefix {
                return Err(ActionUpdateError::InvalidActiveSet);
            }
            for position in 0..count {
                self.dispatch_callback(actor, context, position, Callback::Update)?;
            }
            return Ok(());
        }
        if address != 0x828e_cf30 {
            return Err(ActionUpdateError::PendingStateUpdate(address));
        }
        // StateUpdate_Default3 calls root828ecbc0, then exact packedslot11.
        // Validate generated dispatch layout before assuming a slot mapping.
        if self.current_behaviours != [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 38] {
            return Err(ActionUpdateError::InvalidActiveSet);
        }
        for position in 0..12 {
            self.dispatch_callback(actor, context, position, Callback::Update)?;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "explicit recovered callback and nullable-instance dispatch"
    )]
    fn dispatch_callback(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
        position: usize,
        callback: Callback,
    ) -> Result<(), ActionUpdateError> {
        let index = self.current_behaviours[position];
        let slot = &mut self.instance_slots[position];
        let object = slot
            .behaviour
            .as_ref()
            .ok_or(ActionUpdateError::InvalidActiveSet)?;
        context.set_generated_instance(
            slot.instance
                .as_ref()
                .map(instance_token)
                .map(StateGraphContextHandle::shallow_copy),
        );
        match (object.class, slot.instance.as_mut(), callback) {
            (BehaviourClass::CreateTrickIntentFromGesture, instance, callback) => {
                let instance = match instance {
                    Some(GeneratedActionInstance::Gesture(instance)) => Some(instance),
                    None => None,
                    _ => return Err(ActionUpdateError::InvalidActiveSet),
                };
                match callback {
                    Callback::Begin => {
                        let services = actor
                            .action_runtime_services()
                            .map_err(ActionMotionIntentError::Actor)?;
                        actor.action_gesture_begin(
                            context,
                            &object.payload,
                            instance,
                            services.as_ref().and_then(|s| s.gesture.as_ref()),
                        )
                    }
                    Callback::Update => actor.action_gesture_update(context, instance),
                    Callback::End => actor.action_gesture_end(context, instance.as_deref()),
                }
                .map_err(ActionUpdateError::Gesture)
            }
            (BehaviourClass::BodyFlippingSignal, _, Callback::End)
            | (BehaviourClass::CreateMGTimeIntentFromAGIntent, _, Callback::Begin)
            | (BehaviourClass::PrintText2D, _, _)
            | (BehaviourClass::JuiceHook, _, Callback::Begin | Callback::End) => Ok(()),
            (
                BehaviourClass::BodyFlippingSignal,
                Some(GeneratedActionInstance::BodyFlip(instance)),
                callback,
            ) => {
                let services = actor
                    .action_runtime_services()
                    .map_err(ActionMotionIntentError::Actor)?;
                let attributes = services.as_ref().map(|s| &s.body_flip);
                match callback {
                    Callback::Begin => {
                        actor.try_action_body_flip_begin(context, instance, attributes)
                    }
                    Callback::Update => actor.try_action_body_flip_update(
                        context,
                        instance,
                        attributes,
                        self.delta_time,
                    ),
                    Callback::End => unreachable!(),
                }
                .map_err(ActionUpdateError::Ground)
            }
            (
                BehaviourClass::CreateMGTimeIntentFromAGIntent,
                Some(GeneratedActionInstance::TimeIntent(instance)),
                callback,
            ) => match callback {
                Callback::Update => actor.try_action_time_intent_update(
                    context,
                    &object.payload,
                    instance,
                    self.delta_time,
                ),
                Callback::End => {
                    actor.try_action_time_intent_end(context, &object.payload, Some(instance))
                }
                Callback::Begin => unreachable!(),
            }
            .map_err(ActionUpdateError::Ground),
            (BehaviourClass::CreateMGTimeIntentFromAGIntent, None, Callback::End) => actor
                .try_action_time_intent_end(context, &object.payload, None)
                .map_err(ActionUpdateError::Ground),
            (BehaviourClass::CreateMGTimeIntentFromAGIntent, None, Callback::Update) => actor
                .try_action_time_intent_null_update(context, &object.payload)
                .map_err(ActionUpdateError::Ground),
            (BehaviourClass::BodyFlippingSignal, None, Callback::Begin) => {
                let services = actor
                    .action_runtime_services()
                    .map_err(ActionMotionIntentError::Actor)?;
                actor
                    .try_action_body_flip_null_begin(
                        context,
                        services.as_ref().map(|s| &s.body_flip),
                    )
                    .map_err(ActionUpdateError::Ground)
            }
            (BehaviourClass::BodyFlippingSignal, None, Callback::Update) => {
                let services = actor
                    .action_runtime_services()
                    .map_err(ActionMotionIntentError::Actor)?;
                actor
                    .try_action_body_flip_null_update(
                        context,
                        services.as_ref().map(|s| &s.body_flip),
                        self.delta_time,
                    )
                    .map_err(ActionUpdateError::Ground)
            }
            (BehaviourClass::CreateConstMGIntent, instance, callback) => {
                let instance = match instance {
                    Some(GeneratedActionInstance::ConstIntent(instance)) => Some(instance),
                    None => None,
                    _ => return Err(ActionUpdateError::InvalidActiveSet),
                };
                match callback {
                    Callback::Begin => {
                        actor.action_const_begin(context, &object.payload, instance)?;
                    }
                    Callback::Update => {
                        actor.action_const_update(context, &object.payload, instance)?;
                    }
                    Callback::End => {
                        actor.action_const_end(context, &object.payload, instance.as_deref())?;
                    }
                }
                Ok(())
            }
            (
                BehaviourClass::CreateMGIntentFromAGIntent,
                Some(GeneratedActionInstance::TransferIntent(instance)),
                Callback::Begin,
            ) => Ok(actor.try_action_motion_intent_begin(context, &object.payload, instance)?),
            (
                BehaviourClass::CreateMGIntentFromAGIntent,
                Some(GeneratedActionInstance::TransferIntent(instance)),
                Callback::Update,
            ) => Ok(actor.try_action_motion_intent_update(context, &object.payload, instance)?),
            (
                BehaviourClass::CreateMGIntentFromAGIntent,
                Some(GeneratedActionInstance::TransferIntent(instance)),
                Callback::End,
            ) => Ok(actor.try_action_motion_intent_end(context, &object.payload, instance)?),
            (
                BehaviourClass::JuiceHook,
                Some(GeneratedActionInstance::JuiceHook(instance)),
                Callback::Update,
            ) => Ok(actor.try_action_juice_hook_update(context, instance)?),
            (BehaviourClass::CreateMGIntentFromAGIntent, None, Callback::Begin) => {
                Ok(actor.try_action_motion_intent_null_begin(context, &object.payload)?)
            }
            (BehaviourClass::CreateMGIntentFromAGIntent, None, Callback::Update) => {
                Ok(actor.try_action_motion_intent_null_update(context, &object.payload)?)
            }
            (BehaviourClass::CreateMGIntentFromAGIntent, None, Callback::End) => {
                Ok(actor.try_action_motion_intent_null_end(context, &object.payload)?)
            }
            (BehaviourClass::JuiceHook, None, Callback::Update) => {
                Ok(actor.try_action_juice_hook_null_update(context)?)
            }
            (
                BehaviourClass::BoardAdjust,
                Some(GeneratedActionInstance::BoardAdjust(instance)),
                Callback::Begin,
            ) => {
                instance.begin();
                Ok(())
            }
            (
                BehaviourClass::BoardAdjust,
                Some(GeneratedActionInstance::BoardAdjust(instance)),
                Callback::Update,
            ) if matches!(index, 145 | 147 | 230 | 232 | 234 | 236) => {
                Ok(actor.try_action_board_adjust_update(context, &object.payload, instance)?)
            }
            (
                BehaviourClass::BoardAdjust,
                Some(GeneratedActionInstance::BoardAdjust(_)),
                Callback::Update,
            ) => Err(ActionUpdateError::PendingCallback {
                index,
                address: object.class.binding().callbacks[2],
            }),
            (
                BehaviourClass::BoardAdjust,
                Some(GeneratedActionInstance::BoardAdjust(instance)),
                Callback::End,
            ) => Ok(actor.try_action_board_adjust_end(context, &object.payload, instance)?),
            (_, None, _) => Err(ActionUpdateError::MissingInstance(index)),
            _ => Err(ActionUpdateError::InvalidActiveSet),
        }
    }
}

/// Exact packed occurrence order; no allocation on the Action frame path.
fn additional_shared_update_slots(state: u16) -> Option<(u32, &'static [u16])> {
    match state {
        state_id::HAND_PLANT_37 => Some((
            0x828e_d318,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 108, 109,
                110, 111, 112,
            ],
        )),
        state_id::ONE_FOOT_RIGHT_41 => Some((
            0x828e_d5d8,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 115,
            ],
        )),
        state_id::ONE_FOOT_LEFT_42 => Some((
            0x828e_d5d8,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 116,
            ],
        )),
        state_id::TRANS_TO_OFF_BOARD_AIR_44 => Some((
            0x828e_d5d8,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 117,
            ],
        )),
        state_id::NO_INPUT_BRAKE_5 => Some((0x828e_d0b8, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 41])),
        state_id::TRANS_TO_ON_BOARD_10 => {
            Some((0x828e_d0b8, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 56]))
        }
        state_id::TRANSITIONING_TO_ON_BOARD_11 => {
            Some((0x828e_d0b8, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 57]))
        }
        state_id::START_BRAKE_14 => Some((0x828e_d0b8, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 41])),
        state_id::WAIT_BRAKE_15 => Some((0x828e_d0b8, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 41])),
        state_id::RUNNING_27 => Some((
            0x828e_d4b0,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 96, 97, 98,
            ],
        )),
        // StateUpdate_HippyJump_28 0x828ed448 calls OffBoard_2 once, then packed
        // slots 38 and 39.
        state_id::HIPPY_JUMP_28 => Some((
            0x828e_d448,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 99, 100,
            ],
        )),
        state_id::DISMOUNT_32 => Some((
            0x828e_d528,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 105,
            ],
        )),
        state_id::DEFAULT_33 => Some((
            0x828e_d578,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 106, 107,
            ],
        )),
        state_id::IDLE_146 => Some((
            0x828e_dc40,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 113, 114,
                143,
            ],
        )),
        state_id::REGULAR_156 => Some((
            0x828e_dac8,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 229, 230,
            ],
        )),
        state_id::SWITCH_157 => Some((
            0x828e_dac8,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 231, 232,
            ],
        )),
        state_id::REGULAR_158 => Some((
            0x828e_dac8,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 233, 234,
            ],
        )),
        state_id::SWITCH_159 => Some((
            0x828e_dac8,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 235, 236,
            ],
        )),
        // StateUpdate_Idle_43 828ed108 folds InAir17 and its leaves:
        // OnBoard_6 828ecfe0 (Action_0 then packed11..18), then direct
        // CreateMGIntentFromAGIntent Updates of packed19/20.
        state_id::IN_AIR_17
        | state_id::BOARD_ADJUSTING_39
        | state_id::NEW_ONE_FOOT_40
        | state_id::IDLE_43
        | state_id::LEFT_78
        | state_id::RIGHT_79 => Some((
            0x828e_d108,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63,
            ],
        )),
        // TrickOutOfTipSlashGrind22 shares 828ed108 with its own transfers.
        state_id::TRICK_OUT_OF_TIP_SLASH_GRIND_22 => Some((
            0x828e_d108,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 88, 89,
            ],
        )),
        // StateUpdate_UnMirrored_26 828ed3e8: Action_0, CreateConstMGIntent
        // Update of packed11, direct CreateMGIntentFromAGIntent of packed12.
        state_id::MIRRORED_25 => Some((0x828e_d3e8, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 57, 94])),
        state_id::UN_MIRRORED_26 => {
            Some((0x828e_d3e8, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 57, 95]))
        }
        // StateUpdate_Teleport_24 828ed3e0 is `b StateUpdate_OnBoard_6` 828ecfe0:
        // Action_0 (packed0..10), then direct Updates of packed11..18.
        state_id::TELEPORT_24 => Some((
            0x828e_d3e0,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49,
            ],
        )),
        // StateUpdate_FingerFlip_213 828edc90 folds the Tail/Seatbelt/Nose/Crail
        // leaves: Regular_158 828edac8 (OnBoard_6, packed19/20 transfers,
        // packed21 CreateConstMGIntent, packed22 BoardAdjust), then direct
        // CreateConstMGIntent Updates of packed23 and 24.
        state_id::ONE_FOOT_RIGHT_210 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 144, 145,
                224, 301,
            ],
        )),
        state_id::ONE_FOOT_LEFT_211 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 144, 145,
                224, 302,
            ],
        )),
        state_id::NO_FOOT_AIR_WALK_212 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 144, 145,
                224, 303,
            ],
        )),
        state_id::FINGER_FLIP_213 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 144, 145,
                224, 304,
            ],
        )),
        state_id::FINGER_FLIP_SHUV_216 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 144, 145,
                225, 305,
            ],
        )),
        state_id::ONE_FOOT_RIGHT_217 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 146, 147,
                226, 306,
            ],
        )),
        state_id::ONE_FOOT_LEFT_218 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 146, 147,
                226, 307,
            ],
        )),
        state_id::NO_FOOT_AIR_WALK_219 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 146, 147,
                226, 308,
            ],
        )),
        state_id::FINGER_FLIP_220 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 146, 147,
                226, 309,
            ],
        )),
        state_id::FINGER_FLIP_SHUV_223 => Some((
            0x828e_dc90,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63, 146, 147,
                227, 310,
            ],
        )),
        _ => None,
    }
}

fn empty_slot() -> BehaviourInstanceSlot {
    BehaviourInstanceSlot {
        behaviour: None,
        instance: None,
    }
}
fn instance_token(instance: &GeneratedActionInstance) -> &MotionGraphInstanceToken {
    match instance {
        GeneratedActionInstance::Gesture(instance) => instance.token(),
        GeneratedActionInstance::BodyFlip(instance) => instance.token(),
        GeneratedActionInstance::TimeIntent(instance) => instance.token(),
        GeneratedActionInstance::ConstIntent(instance) => instance.token(),
        GeneratedActionInstance::TransferIntent(instance) => instance.token(),
        GeneratedActionInstance::JuiceHook(instance) => instance.token(),
        GeneratedActionInstance::BoardAdjust(instance) => instance.token(),
    }
}
#[derive(Clone, Copy)]
enum Callback {
    Begin,
    Update,
    End,
}
fn filtered_state(state: FilteredState) -> FilteredPhysicsState {
    match state {
        FilteredState::Ground => FilteredPhysicsState::Ground,
        FilteredState::Air => FilteredPhysicsState::Air,
        FilteredState::Grind => FilteredPhysicsState::Grind,
        FilteredState::Wipeout => FilteredPhysicsState::Wipeout,
        FilteredState::Teleport => FilteredPhysicsState::Teleport,
        FilteredState::Invalid => FilteredPhysicsState::Invalid,
        FilteredState::Offboard => FilteredPhysicsState::OffBoard,
        FilteredState::OffboardAir => FilteredPhysicsState::OffBoardAir,
    }
}

#[cfg(test)]
mod air_idle_dispatch_tests {
    use super::super::data;

    #[test]
    fn idle43_uses_air_parent_transfers_without_ground_or_leaf_callbacks() {
        let idle = &data::STATES[43];
        assert_eq!(idle.update_address, 0x828e_d108);
        assert_eq!(idle.parent, 17);
        assert_eq!(idle.behaviours.count, 0);
        let mut ancestry = Vec::new();
        let mut state = 43;
        while state != u16::MAX {
            ancestry.push(state);
            state = data::STATES[usize::from(state)].parent;
        }
        ancestry.reverse();
        let slots: Vec<_> = ancestry
            .iter()
            .flat_map(|state| {
                let section = data::STATES[usize::from(*state)].behaviours;
                section.index..section.index.saturating_add(section.count)
            })
            .collect();
        assert_eq!(
            slots,
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 43, 44, 45, 46, 47, 48, 49, 62, 63
            ]
        );
        for index in [62, 63] {
            let behaviour = &data::BEHAVIOURS[index];
            assert_eq!(behaviour.parent, 17);
            assert_eq!(behaviour.name, "CreateMGIntentFromAGIntent");
        }
    }

    /// Every alias of the folded 828ed108/828ed3e8 thunks lists its authored
    /// ancestry once, and each packed position holds the class the thunk
    /// direct-calls (OnBoard_6 828ecfe0 for packed11..18 of 828ed108).
    #[test]
    fn folded_air_and_mount_aliases_match_their_native_direct_calls() {
        const AIR: [&str; 10] = [
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "BodyFlippingSignal",
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateMGTimeIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
        ];
        const MOUNT: [&str; 2] = ["CreateConstMGIntent", "CreateMGIntentFromAGIntent"];
        // Regular_158 828edac8 packed11..22, then FingerFlip_213 packed23/24.
        const GRAB: [&str; 14] = [
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "BodyFlippingSignal",
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateMGTimeIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateMGIntentFromAGIntent",
            "CreateConstMGIntent",
            "BoardAdjust",
            "CreateConstMGIntent",
            "CreateConstMGIntent",
        ];
        for (state, address, first, tail) in [
            (17_u16, 0x828e_d108_u32, 11_usize, &AIR[..]),
            (22, 0x828e_d108, 11, &AIR[..]),
            (39, 0x828e_d108, 11, &AIR[..]),
            (40, 0x828e_d108, 11, &AIR[..]),
            (43, 0x828e_d108, 11, &AIR[..]),
            (78, 0x828e_d108, 11, &AIR[..]),
            (79, 0x828e_d108, 11, &AIR[..]),
            (25, 0x828e_d3e8, 11, &MOUNT[..]),
            (26, 0x828e_d3e8, 11, &MOUNT[..]),
            (24, 0x828e_d3e0, 11, &AIR[..8]),
            (210, 0x828e_dc90, 11, &GRAB[..]),
            (211, 0x828e_dc90, 11, &GRAB[..]),
            (212, 0x828e_dc90, 11, &GRAB[..]),
            (213, 0x828e_dc90, 11, &GRAB[..]),
            (216, 0x828e_dc90, 11, &GRAB[..]),
            (217, 0x828e_dc90, 11, &GRAB[..]),
            (218, 0x828e_dc90, 11, &GRAB[..]),
            (219, 0x828e_dc90, 11, &GRAB[..]),
            (220, 0x828e_dc90, 11, &GRAB[..]),
            (223, 0x828e_dc90, 11, &GRAB[..]),
        ] {
            assert_eq!(data::STATES[usize::from(state)].update_address, address);
            let mut ancestry = Vec::new();
            let mut current = state;
            while current != u16::MAX {
                ancestry.push(current);
                current = data::STATES[usize::from(current)].parent;
            }
            ancestry.reverse();
            let slots: Vec<u16> = ancestry
                .iter()
                .flat_map(|state| {
                    let section = data::STATES[usize::from(*state)].behaviours;
                    section.index..section.index.saturating_add(section.count)
                })
                .collect();
            let (table_address, table) = super::additional_shared_update_slots(state).unwrap();
            assert_eq!(table_address, address);
            assert_eq!(slots, table, "state {state}");
            assert_eq!(slots.len(), first + tail.len(), "state {state}");
            for (offset, name) in tail.iter().enumerate() {
                assert_eq!(
                    data::BEHAVIOURS[usize::from(slots[first + offset])].name,
                    *name,
                    "state {state} packed {}",
                    first + offset
                );
            }
        }
    }
}

#[cfg(test)]
mod board_adjust_tests {
    use super::*;
    use crate::generated_action_load::{GeneratedActionFactories, GeneratedActionStatics};

    fn down_controller() -> (
        ActorConstruction,
        BoundMotionGraphContext<ActorIdentity>,
        GeneratedActionController<ActorIdentity>,
    ) {
        let (actor, _route) = crate::actor_construction::tests::actor_with_listener();
        actor.try_construct_graphs().unwrap();
        let context = {
            let prefix = actor.prefix().unwrap();
            prefix
                .graphs()
                .unwrap()
                .action()
                .unwrap()
                .controller
                .as_ref()
                .unwrap()
                .context()
                .copy_for_behaviour()
        };
        let mut factories = GeneratedActionFactories::new();
        factories.register_skater_classes().unwrap();
        let controller = GeneratedActionController::try_load(
            &context,
            &GeneratedActionStatics::new(),
            &factories,
            false,
        )
        .unwrap();
        (actor, context, controller)
    }

    #[test]
    fn done_air_dismount_does_not_treat_missing_animatable_as_finished() {
        let (actor, context, mut controller) = down_controller();
        assert!(matches!(
            controller.initial_condition(&actor, &context, 145, 33),
            Err(ActionUpdateError::AnimationAttribute(
                crate::generated_motion_load::runtime::MotionUpdateError::MissingOwner(
                    "Animatable interface"
                )
            ))
        ));
        assert_eq!(controller.condition_active[145 / 32] & (1 << (145 % 32)), 0);
    }

    #[test]
    fn shared_continuation_slots_match_compiled_ancestry_without_duplicates() {
        use crate::generated_action_load::data;
        for state in [
            5, 10, 11, 14, 15, 27, 28, 32, 33, 41, 42, 44, 146, 156, 157, 158, 159,
        ] {
            let (address, expected) = additional_shared_update_slots(state).unwrap();
            assert_eq!(address, data::STATES[usize::from(state)].update_address);
            let mut ancestry = Vec::new();
            let mut current = state;
            while current != NULL_INDEX {
                ancestry.push(current);
                current = data::STATES[usize::from(current)].parent;
            }
            let declared: Vec<u16> = ancestry
                .into_iter()
                .rev()
                .flat_map(|index| {
                    let section = data::STATES[usize::from(index)].behaviours;
                    section.index..section.index + section.count
                })
                .collect();
            assert_eq!(expected, declared, "state {state}");
            let mut unique = expected.to_vec();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), expected.len(), "state {state}");
            if state >= 156 {
                assert_eq!(
                    data::BEHAVIOURS[usize::from(expected[21])].name,
                    "CreateConstMGIntent"
                );
                assert_eq!(
                    data::BEHAVIOURS[usize::from(expected[22])].name,
                    "BoardAdjust"
                );
            }
        }
        for already_mounted in [30, 31, 138, 151, 155] {
            assert!(additional_shared_update_slots(already_mounted).is_none());
        }
    }

    #[test]
    fn board_adjust_up_down_end_erases_owned_keys_and_preserves_instance() {
        use crate::{
            actor_construction::action_motion_intent_bridge::constant::ConstIntentInstance,
            actor_graph_construction::ActorGraphIntents,
            generated_action_load::construct::BehaviourPayload,
        };
        for (state, occurrence) in [
            (76, 145),
            (77, 147),
            (156, 230),
            (157, 232),
            (158, 234),
            (159, 236),
        ] {
            let (actor, mut context, mut controller) = down_controller();
            let mut allocations = Vec::new();
            controller
                .enter_states(&actor, &context, &[state], &mut |index, size| {
                    allocations.push((index, size));
                    GeneratedActionAllocation::Host
                })
                .unwrap();
            assert_eq!(allocations.last(), Some(&(occurrence, 0x10)));
            let position = controller
                .current_behaviours
                .binary_search(&occurrence)
                .unwrap();
            controller
                .dispatch_callback(&actor, &mut context, position, Callback::Begin)
                .unwrap();
            let Some(GeneratedActionInstance::BoardAdjust(instance)) =
                controller.instance_slots[position].instance.as_mut()
            else {
                panic!("BoardAdjust")
            };
            assert_eq!(instance.last_angle, Some(0.0));
            instance.update_angle(2.0).unwrap();
            instance.update_angle(-2.0).unwrap();
            let expected_state = instance.state;
            let expected_angle = instance.last_angle.map(f32::to_bits);
            let issued = instance.token().shallow_copy();
            let object = controller
                .statics
                .behaviour(usize::from(occurrence))
                .unwrap()
                .unwrap();
            let BehaviourPayload::BoardAdjust {
                magnitude, angle, ..
            } = &object.payload
            else {
                panic!("BoardAdjust payload")
            };
            let other = ActorGraphIntents::native_name(b"UnrelatedIntent");
            for (name, value) in [(*magnitude, 0.75), (*angle, 1.25), (other, -0.5)] {
                let mut constant = ConstIntentInstance::new();
                let mut constant_context = context.copy_for_behaviour();
                constant_context.set_generated_instance(Some(constant.token().shallow_copy()));
                actor
                    .action_const_begin(
                        &constant_context,
                        &BehaviourPayload::ConstIntent {
                            motion: name,
                            value,
                            on_update: true,
                        },
                        Some(&mut constant),
                    )
                    .unwrap();
            }
            controller
                .dispatch_callback(&actor, &mut context, position, Callback::End)
                .unwrap();
            {
                let prefix = actor.prefix().unwrap();
                let intents = prefix
                    .graphs()
                    .unwrap()
                    .motion()
                    .unwrap()
                    .intents
                    .as_ref()
                    .unwrap();
                assert_eq!(intents.get(magnitude), None);
                assert_eq!(intents.get(angle), None);
                assert_eq!(intents.get(&other), Some(-0.5));
            }
            let Some(GeneratedActionInstance::BoardAdjust(instance)) =
                controller.instance_slots[position].instance.as_ref()
            else {
                panic!("BoardAdjust")
            };
            assert_eq!(instance.state, expected_state);
            assert_eq!(instance.last_angle.map(f32::to_bits), expected_angle);
            assert!(issued.is_same_issuance(instance.token()));
            assert!(
                context
                    .instance()
                    .is_some_and(|token| issued.is_same_issuance(token))
            );
        }
    }

    #[test]
    fn board_adjust_147_null_allocation_keeps_slot_and_checks_begin_boundary() {
        let (actor, mut context, mut controller) = down_controller();
        let mut allocations = Vec::new();
        controller
            .enter_states(&actor, &context, &[77], &mut |index, size| {
                allocations.push((index, size));
                if index == 147 {
                    GeneratedActionAllocation::Null
                } else {
                    GeneratedActionAllocation::Host
                }
            })
            .unwrap();
        assert_eq!(allocations.last(), Some(&(147, 0x10)));
        assert_eq!(controller.current_behaviours, [146, 147]);
        let position = controller.current_behaviours.binary_search(&147).unwrap();
        assert!(controller.instance_slots[position].behaviour.is_some());
        assert!(controller.instance_slots[position].instance.is_none());
        assert!(matches!(
            controller.dispatch_callback(&actor, &mut context, position, Callback::Begin),
            Err(ActionUpdateError::MissingInstance(147))
        ));
        assert!(context.instance().is_none());
    }
}
