//! Retained generated Motion evaluation; source Update82955e20.
use super::{
    MotionLoadError,
    conditions::{ConditionPayload, MotionCondition},
    controller::GeneratedMotionController,
};
use crate::reckoning_numeric::ReckoningNumeric;
use crate::{
    actor_construction::{
        ActorConstruction, ActorConstructionError, ActorIdentity,
        physical_mount::ActorPhysicalMountError,
    },
    generated_action_load::{
        ConditionClass, NULL_INDEX,
        construct::{ComparisonOp, ConditionPayload as SharedPayload, FilteredState},
    },
    input_cvar_runtime::InputCvarError,
    motion_graph_entity_authority::BoundMotionGraphContext,
    physical_player_phys_out::construct::ConcretePhysOutError,
    physics::FilteredPhysicsState,
};
use std::convert::Infallible;
#[path = "generated_motion_runtime_graph.rs"]
mod graph;
#[path = "generated_motion_runtime_grind_states.rs"]
mod grind_states;
#[path = "callbacks/actor_motion_runtime_grounded_selection.rs"]
pub(crate) mod grounded_selection;
#[path = "generated_motion_runtime_handplant_states.rs"]
mod handplant_states;
#[path = "generated_motion_runtime_lifecycle.rs"]
mod lifecycle;
#[path = "generated_motion_runtime_moving_states.rs"]
mod moving_states;
#[path = "generated_motion_runtime_selection.rs"]
mod selection;
#[path = "generated_motion_runtime_sustain.rs"]
mod sustain;

#[derive(Debug)]
pub(crate) enum MotionUpdateError {
    Load(MotionLoadError),
    Actor(ActorConstructionError),
    Physical(ActorPhysicalMountError),
    ForcePhysics(Box<crate::simulation_assembly_backend::constraint_session::motion_force_physics::ForcePhysicsCapabilityError>),
    PhysOut(ConcretePhysOutError<Infallible>),
    GrindCondition(crate::actor_construction::action_motion_intent_bridge::ActionMotionIntentError),
    GestureCondition(crate::actor_construction::action_motion_intent_bridge::gesture_runtime::GestureRuntimeError),
    ContactAttribute(skate2_formats::andale::AndaleError),
    HandPlantAttribute(crate::attribute_runtime::AttributeRuntimeError),
    Cvar(InputCvarError),
    ConditionNumeric(crate::reckoning_numeric::ReckoningHostNumericError),
    ScoringNumeric(crate::reckoning_numeric::ReckoningHostNumericError),
    DisablePush(crate::actor_construction::action_motion_intent_bridge::disable_push::DisablePushError<
        crate::actor_construction::action_motion_intent_bridge::disable_push::DisablePushActorError,
        crate::reckoning_numeric::ReckoningHostNumericError>),
    Lock,
    Capacity,
    InvalidState(u16),
    InvalidTransition(u16),
    InvalidInterruptType(u8),
    InvalidCache(u16),
    InvalidActiveSet,
    MissingOwner(&'static str),
    ForeignOwner(&'static str),
    PendingExpression(u16),
    PendingCondition(u16),
    PendingMaskedExpression {
        mask: graph::Mask,
        expression: u16,
    },
    PendingTransitionHook(u16),
    PendingAnimationOperation(&'static str),
    AirLegOverlay(crate::debug_overlay::DebugOverlayError),
    LandingOffBoardAirOverlay(crate::debug_overlay::DebugOverlayError),
    SkitchOverlay(crate::debug_overlay::DebugOverlayError),
    BaseAnimationSpeed(crate::actor_animation_construction::locomotion_update::LocomotionUpdateError),
    AnimationBody(crate::actor_animation_construction::play_anim::PlayBodyAnimationError),
    AnimationPhase(crate::actor_animation_construction::frame_phase::AnimationPhaseError),
    AnimationWrite(crate::actor_animation_construction::gesture_writes::GestureAnimationWriteError),
    Locomotion(Box<crate::actor_animation_construction::locomotion_lifecycle::LocomotionLifecycleError>),
    AnimationPhysicsData(crate::actor_animation_construction::physics_update_data::AnimationPhysicsDataError),
    AnimationChannel(crate::actor_animation_construction::channel_runtime::GestureChannelError),
    PlayAnimation(Box<crate::play_animation_lifecycle::PlayAnimationRuntimeError<Self>>),
    Gesture(crate::actor_construction::character_gesture::CharacterGestureError),
    ActionGraphReset(crate::actor_graph_construction::ActorActionGraphResetError),
    SkaterMotionGraphReset(crate::actor_graph_construction::ActorSkaterMotionGraphResetError),
    SkaterAnimReset(crate::actor_animation_construction::reset_skater::ResetSkaterAnimError),
    ResetToSpawnStance(crate::actor_animation_construction::reset_skater::ResetToSpawnStanceError),
    PendingCallback {
        index: u16,
        address: u32,
    },
    PendingStateUpdate(u32),
    UpdateAndWriteback {
        update: Box<Self>,
        writeback: Box<Self>,
    },
}
impl std::fmt::Display for MotionUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load(e) => write!(f, "Motion load: {e}"),
            Self::Actor(e) => write!(f, "Motion Actor: {e}"),
            Self::Physical(e) => write!(f, "Motion physical: {e}"),
            Self::PhysOut(e) => write!(f, "Motion output: {e:?}"),
            Self::GrindCondition(e) => write!(f, "Motion IsGrinding: {e}"),
            Self::GestureCondition(e) => write!(f, "Motion HasGestureIntent: {e}"),
            Self::HandPlantAttribute(e) => write!(f, "HandPlant animation attribute: {e:?}"),
            Self::ContactAttribute(e) => write!(f, "Motion contact attribute: {e}"),
            Self::ForcePhysics(e) => e.fmt(f),
            Self::DisablePush(e) => write!(f, "Motion DisablePushBrake: {e}"),
            Self::ConditionNumeric(e) => write!(f, "Motion condition numeric: {e}"),
            Self::ScoringNumeric(e) => write!(f, "Motion ScoringGrabs numeric: {e}"),
            Self::Cvar(e) => write!(f, "Motion cvar: {e:?}"),
            Self::Lock => f.write_str("Motion owner lock failed"),
            Self::Capacity => f.write_str("Motion fixed capacity exceeded"),
            Self::InvalidState(i) => write!(f, "invalid Motion state {i}"),
            Self::InvalidTransition(i) => write!(f, "invalid Motion transition {i}"),
            Self::InvalidInterruptType(kind) => write!(f, "invalid Motion interrupt type {kind}"),
            Self::InvalidCache(i) => write!(f, "invalid Motion condition cache {i}"),
            Self::InvalidActiveSet => f.write_str("invalid Motion active behaviour set"),
            Self::MissingOwner(s) => write!(f, "Motion requires {s}"),
            Self::ForeignOwner(s) => write!(f, "Motion owner mismatch: {s}"),
            Self::PendingExpression(i) => write!(f, "Motion expression {i} pending"),
            Self::PendingCondition(i) => write!(f, "Motion condition {i} pending"),
            Self::PendingMaskedExpression { mask, expression } => {
                write!(f, "Motion {mask:?} expression {expression} pending")
            }
            Self::PendingTransitionHook(index) => {
                write!(f, "Motion transition {index} hook pending")
            }
            Self::PendingAnimationOperation(name) => {
                write!(f, "Motion animation operation {name} pending")
            }
            Self::AirLegOverlay(error) => write!(f, "Air leg overlay: {error}"),
            Self::SkitchOverlay(error) => write!(f, "Skitch overlay: {error}"),
            Self::LandingOffBoardAirOverlay(error) => {
                write!(f, "LandingOffBoardAir overlay: {error}")
            }
            Self::BaseAnimationSpeed(error) => write!(f, "Motion base animation speed: {error:?}"),
            Self::AnimationBody(error) => write!(f, "Motion body animation: {error}"),
            Self::AnimationPhase(error) => write!(f, "Motion animation phase: {error}"),
            Self::AnimationWrite(error) => write!(f, "Motion animation attribute: {error}"),
            Self::Locomotion(error) => write!(f, "Motion locomotion: {error:?}"),
            Self::AnimationPhysicsData(error) => {
                write!(f, "Motion animation physics data: {error}")
            }
            Self::AnimationChannel(error) => write!(f, "Motion animation channel: {error}"),
            Self::PlayAnimation(error) => write!(f, "Motion PlayAnimation: {error}"),
            Self::Gesture(error) => write!(f, "Motion gesture: {error}"),
            Self::ActionGraphReset(error) => write!(f, "Motion ActionGraph reset: {error:?}"),
            Self::SkaterMotionGraphReset(error) => {
                write!(f, "Motion SkaterMotionGraph reset: {error:?}")
            }
            Self::SkaterAnimReset(error) => write!(f, "Motion SkaterAnim reset: {error}"),
            Self::ResetToSpawnStance(error) => write!(f, "Motion ResetToSpawnStance: {error}"),
            Self::PendingCallback { index, address } => write!(
                f,
                "Motion occurrence {index} callback {address:08x} pending"
            ),
            Self::PendingStateUpdate(address) => {
                write!(f, "Motion state Update {address:08x} pending")
            }
            Self::UpdateAndWriteback { update, writeback } => {
                write!(f, "Motion update: {update}; context writeback: {writeback}")
            }
        }
    }
}

impl GeneratedMotionController<ActorIdentity> {
    /// `GetNextState`829511b0. Selection does not publish a current state.
    fn select_next(
        &mut self,
        actor: &ActorConstruction,
        context: &BoundMotionGraphContext<ActorIdentity>,
        dt: f32,
    ) -> Result<graph::Selected, MotionUpdateError> {
        self.validate_context(context)
            .map_err(MotionUpdateError::Load)?;
        actor.check_motion_runtime_context(context)?;
        self.condition_active.fill(0);
        self.delta_time = dt;
        self.allow_evaluations = true;
        let graph = graph::Graph {
            states: &super::data::STATES,
            transitions: &super::data::TRANSITIONS,
            indices: &super::data::INDICES,
            current: self.current_state,
        };
        let selected = graph.next(&mut |mask, expression, parent| {
            let condition = &mut |index| self.condition(actor, context, index, parent);
            match mask {
                graph::Mask::Pre => selection::expression(expression, condition),
                graph::Mask::Sustain => sustain::expression(expression, condition),
            }
        })?;
        if self.current_state != selected.state
            && self.current_behaviours.contains(&1932)
            && std::env::var_os("SKATE2_TRACE_PUSH_INTERRUPTION").is_some()
        {
            let conditions: Vec<_> = self
                .condition_results
                .iter()
                .enumerate()
                .filter(|(n, _)| self.condition_active[n / 32] & (1u32 << (n % 32)) != 0)
                .map(|(n, value)| (n, *value))
                .collect();
            eprintln!(
                "push motion transition: {} -> {} sampled={conditions:?}",
                self.current_state, selected.state
            );
        }
        if self.current_state != selected.state
            && (std::env::var_os("SKATE2_TRACE_OFFBOARD_AIR").is_some()
                || std::env::var_os("SKATE2_TRACE_VERT").is_some())
        {
            let name = |state: u16| {
                super::data::STATES
                    .get(usize::from(state))
                    .map_or("?", |s| s.name)
            };
            eprintln!(
                "offboard-air motion: {}{} -> {}{}",
                name(self.current_state),
                self.current_state,
                name(selected.state),
                selected.state
            );
        }
        self.allow_evaluations = false;
        Ok(selected)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "single parent/cache publication envelope around original condition dispatch"
    )]
    fn condition(
        &mut self,
        actor: &ActorConstruction,
        context: &BoundMotionGraphContext<ActorIdentity>,
        index: u16,
        parent: u16,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        let position = usize::from(index);
        let word = position / 32;
        let bit = 1u32 << (position % 32);
        if self.condition_active[word] & bit != 0 {
            return self.condition_results[position].ok_or(E::InvalidCache(index));
        }
        let (condition, cvars) = {
            let statics = self.load.statics.inner.lock().map_err(|_| E::Lock)?;
            (
                statics
                    .conditions
                    .get(position)
                    .and_then(Option::as_ref)
                    .cloned()
                    .ok_or(E::MissingOwner("Motion condition"))?,
                statics.cvars.clone(),
            )
        };
        let base = match condition.as_ref() {
            MotionCondition::Shared(c) => &c.base,
            MotionCondition::Motion { base, .. } => base,
        };
        *base.parent.lock().map_err(|_| E::Lock)? = Some(parent);
        let value = match condition.as_ref() {
            MotionCondition::Motion {
                payload: ConditionPayload::InStateForTime { comparison, state }, ..
            } => {
                // Activation82c94920: resolve the authored name relative to
                // the condition's tagged parent, then sample the retained
                // controller timer. Missing/inactive states yield -1, not zero.
                let target = grounded_selection::substate(&super::data::STATES, parent, state)?;
                crate::generated_action_load::conditions::parent_time_condition(
                    &self.time_in_state, super::data::STATES.len(), Some(target),
                    *comparison, E::InvalidState,
                )?
            }
            MotionCondition::Motion { payload: ConditionPayload::ShouldPlayHandPlantAnim {desired}, .. } => actor.motion_should_play_handplant(context,*desired)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::HasNewHandPlantPos), .. } => actor.motion_new_handplant_position(context)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::WillExpire {
                        time,
                        wait_for_transitions,
                        time_tag,
                    },
                ..
            } => {
                actor.motion_runtime_will_expire(context, *time, *wait_for_transitions, time_tag)?
            }
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Comparison {
                        class: super::conditions::plain::ComparisonCondition::IsLanding,
                        ..
                    },
                ..
            } => {
                actor.check_motion_runtime_context(context)?;
                actor
                    .runtime_is_landing_from_checked_context()
                    .map_err(E::GestureCondition)?
            }
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Comparison {
                        class: super::conditions::plain::ComparisonCondition::IsRunningOut,
                        ..
                    },
                ..
            } => {
                actor.check_motion_runtime_context(context)?;
                actor
                    .runtime_is_running_out_from_checked_context()
                    .map_err(E::GestureCondition)?
            }
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsDoneWipingOut),
                ..
            } => actor.motion_runtime_done_wiping_out(context)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::IsInDebugAnimationsMode,
                    ),
                ..
            } => {
                actor.check_motion_runtime_context(context)?;
                cvars
                    .as_ref()
                    .ok_or(E::MissingOwner("Motion cvars"))?
                    .try_use_debug_animations_mode()
                    .map_err(E::Cvar)?
            }
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::IsRetrievingSkateboard,
                    ),
                ..
            } => actor.motion_runtime_board_operation_flag(context, true)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::IsDroppingSkateboard,
                    ),
                ..
            } => actor.motion_runtime_board_operation_flag(context, false)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::AllowedToTrick),
                ..
            } => actor.motion_runtime_allowed_to_trick(context)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::BreakOutOfPush),
                ..
            } => actor.motion_runtime_break_out_of_push(context)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::OkToDoTrickOnStairs,
                    ),
                ..
            } => actor.motion_runtime_ok_to_trick_on_stairs(context)?,
            MotionCondition::Motion {
                payload: ConditionPayload::CanEnterSlide { right },
                ..
            } => actor.motion_runtime_can_enter_slide(context, *right)?,
            MotionCondition::Motion {
                payload: ConditionPayload::ShouldLeaveSlide { right },
                ..
            } => actor.motion_runtime_should_leave_slide(context, *right)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::CanLandOnBoard), .. } => actor.motion_can_land_on_board_condition(context, cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?)?,
            MotionCondition::Motion { payload: ConditionPayload::IsInJumpAir { foot, .. }, .. } => actor.motion_jump_air_condition(context, *foot, cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?)?,
            MotionCondition::Motion { payload: ConditionPayload::Comparison { class: super::conditions::plain::ComparisonCondition::IsAirMountRequested, .. }, .. } => actor.motion_air_mount_condition(context, cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsPastLocomotionApex), .. } => actor.motion_locomotion_apex_condition(context, cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsRidingSwitch), .. } => actor.motion_riding_switch_condition(context)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsTransitioning), .. } => actor.motion_is_transitioning_condition(context)?,
            MotionCondition::Motion { payload: ConditionPayload::InTimeWindow { start, frame_length }, .. } => actor.motion_in_time_window_condition(context, *start, *frame_length)?,
            MotionCondition::Motion { payload: ConditionPayload::IsLocomotionFootDown { foot, .. }, .. } => actor.motion_locomotion_foot_condition(context, *foot, cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::DoSprintMount), .. } => actor.motion_sprint_mount_condition(context, cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::ShouldPrepareOneFootAirForFootplant), .. } => actor.motion_should_prepare_one_foot_air_for_footplant(context, cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::ApexReached), .. } => actor.motion_runtime_apex_reached(context)?,
            MotionCondition::Motion {
                payload: ConditionPayload::ComVelCompare { comparison, axis },
                ..
            } => actor.motion_runtime_com_velocity_compare(context, *comparison, *axis)?,
            MotionCondition::Motion {
                payload: ConditionPayload::HasIntent { comparison, intent },
                ..
            } => actor
                .motion_runtime_intent_value(context, intent)?
                .is_some_and(|v| {
                    comparison.op == ComparisonOp::Nop
                        || crate::generated_action_load::conditions::compare(*comparison, v)
                }),
            MotionCondition::Motion {
                payload:
                    ConditionPayload::MongoPushFootToFar {
                        contact,
                        right_bone,
                        left_bone,
                    },
                ..
            } => actor.motion_runtime_mongo_push_foot_too_far(
                context,
                *contact,
                *right_bone,
                *left_bone,
                cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?,
            )?,
            MotionCondition::Motion {
                payload: ConditionPayload::HasFilteredMotionGraphIntent { comparison, intent },
                ..
            } => actor
                .motion_runtime_filtered_intent_value(context, intent)?
                .is_some_and(|v| {
                    comparison.op == ComparisonOp::Nop
                        || crate::generated_action_load::conditions::compare(*comparison, v)
                }),
            MotionCondition::Motion {
                payload: ConditionPayload::HasTweak { comparison, x, y },
                ..
            } => {
                // HasTweak::ActivationFunc82cc26f8 queries both filtered
                // intents even when the first is present, then checks map
                // presence before its comparison operation.
                let tweak_x = actor.motion_runtime_filtered_intent_value(context, x)?;
                let tweak_y = actor.motion_runtime_filtered_intent_value(context, y)?;
                if tweak_x.is_none() && tweak_y.is_none() {
                    false
                } else if comparison.op == ComparisonOp::Nop {
                    true
                } else {
                    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
                    let x = tweak_x.unwrap_or(0.0);
                    let y = tweak_y.unwrap_or(0.0);
                    // Native fmuls X*X, then fmadds Y*Y + X². The shared
                    // vector length uses a different dot reduction order.
                    let squared = y.mul_add(y, x * x);
                    let estimate = numeric
                        .reciprocal_sqrt_estimate(squared)
                        .map_err(E::ConditionNumeric)?;
                    let length = crate::reckoning_numeric::length_from_squared_and_estimate(
                        squared, estimate,
                    );
                    crate::generated_action_load::conditions::compare(*comparison, length)
                }
            }
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::LocomotionValid),
                ..
            } => actor.motion_locomotion_valid(
                context,
                cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?,
            )?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::IsLocomotionQuitNotAllowed,
                    ),
                ..
            } => actor.motion_locomotion_quit_not_allowed(
                context,
                cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?,
            )?,
            MotionCondition::Shared(c) => match (&c.class, &c.payload) {
                (ConditionClass::HasAGIntent, SharedPayload::ActionIntent { comparison, intent }) => {
                    //82cc1748: query IActionGraph, not Motion or filtered Motion.
                    actor.check_motion_runtime_context(context)?;
                    actor.action_intent_value_from_checked_entity(intent)
                        .map_err(E::GrindCondition)?
                        .is_some_and(|value| comparison.op == ComparisonOp::Nop
                            || crate::generated_action_load::conditions::compare(*comparison, value))
                }
                (ConditionClass::IsMirrored, SharedPayload::Plain) => {
                    actor.check_motion_runtime_context(context)?;
                    actor
                        .runtime_is_mirrored_from_checked_context()
                        .map_err(E::GestureCondition)?
                }
                (ConditionClass::TimeToLand, SharedPayload::Comparison(comparison)) => {
                    actor.check_motion_runtime_context(context)?;
                    actor.runtime_time_to_land(*comparison)?
                }
                (ConditionClass::OBTimeToLand, SharedPayload::Comparison(comparison)) => {
                    actor.motion_runtime_offboard_time_to_land(context, *comparison)?
                }
                (ConditionClass::IsFootPlanting, SharedPayload::Plain) => {
                    actor.motion_runtime_is_foot_planting(context)?
                }
                (ConditionClass::IsDismounting, SharedPayload::Comparison(_)) => {
                    actor.check_motion_runtime_context(context)?;
                    actor
                        .runtime_is_dismounting_from_checked_context()
                        .map_err(E::GestureCondition)?
                }
                (ConditionClass::InParentStateForTime, SharedPayload::Comparison(comparison)) => {
                    let tagged_parent = *c.base.parent.lock().map_err(|_| E::Lock)?;
                    crate::generated_action_load::conditions::parent_time_condition(
                        &self.time_in_state,
                        super::data::STATES.len(),
                        tagged_parent,
                        *comparison,
                        E::InvalidState,
                    )?
                }
                (
                    ConditionClass::HasAnimAttribute,
                    SharedPayload::AnimationAttribute {
                        comparison,
                        attribute,
                        sequence,
                    },
                ) => match actor.motion_runtime_tree_attribute(context, *attribute)? {
                    None => false,
                    Some(value) => {
                        if *sequence != -1 {
                            return Err(E::PendingAnimationOperation(
                                "tree attribute sequence metadata",
                            ));
                        }
                        if comparison.op == ComparisonOp::Nop {
                            true
                        } else {
                            let scalar = value.scalar_value().ok_or(
                                E::PendingAnimationOperation("tree attribute comparison value"),
                            )?;
                            crate::generated_action_load::conditions::compare(*comparison, scalar)
                        }
                    }
                },
                (ConditionClass::PhysicsWantsRunout, SharedPayload::Plain) => actor
                    .motion_runtime_requests_runout(
                        context,
                        cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?,
                    )?,
                (ConditionClass::PhysFilteredState, SharedPayload::PhysicalState(expected)) => {
                    actor.motion_runtime_filtered_state(context)? == filtered_state(*expected)
                }
                (ConditionClass::IsHandPlanting, SharedPayload::HandPlant { state, direction }) => {
                    actor.motion_runtime_handplant(context, *state, *direction)?
                }
                (
                    ConditionClass::PhysicsSpeedCompare,
                    SharedPayload::Speed {
                        comparison,
                        along_skate_z,
                    },
                ) => actor.motion_runtime_speed_compare(context, *comparison, *along_skate_z)?,
                (ConditionClass::CurrentState, SharedPayload::CurrentState(name)) => {
                    grounded_selection::state_matches(
                        &super::data::STATES,
                        parent,
                        self.current_state,
                        name,
                    )?
                }
                (ConditionClass::DisablePushBrake, SharedPayload::Plain) => {
                    actor.check_motion_runtime_context(context)?;
                    let services = actor.action_runtime_services().map_err(E::Actor)?;
                    actor
                        .runtime_disable_push(
                            services.as_ref().map(|s| &s.pushing),
                            &mut crate::reckoning_numeric::HostReckoningNumeric,
                        )
                        .map_err(E::DisablePush)?
                }

                (ConditionClass::HasGestureIntent, SharedPayload::Gesture(group)) => {
                    actor.check_motion_runtime_context(context)?;
                    let services = actor.action_runtime_services().map_err(E::Actor)?;
                    actor
                        .runtime_has_gesture_from_checked_context(
                            *group,
                            services.as_ref().and_then(|s| s.gesture.as_ref()),
                        )
                        .map_err(E::GestureCondition)?
                }

                (ConditionClass::IsRidingFakie, SharedPayload::Plain) => {
                    actor.motion_runtime_is_riding_fakie(context)?
                }
                (ConditionClass::IsGrinding, SharedPayload::Grinding { has_specific, name }) => {
                    actor.motion_runtime_is_grinding(context, has_specific.then_some(name))?
                }
                (ConditionClass::IsGrabbingObject, SharedPayload::Plain) => {
                    actor.motion_runtime_is_grabbing_object(context)?
                }
                _ => return Err(E::PendingCondition(index)),
            },
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::PhysicsWantsRunout,
                    ),
                ..
            } => actor.motion_runtime_requests_runout(
                context,
                cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?,
            )?,
            MotionCondition::Motion {
                payload: ConditionPayload::IsMovingObject { angle_start, angle_end },
                ..
            } => actor.motion_runtime_is_moving_object(context, *angle_start, *angle_end)?,
            MotionCondition::Motion {
                payload: ConditionPayload::AngleAdjustByAnimationEndLessThan { max_delta_angle },
                ..
            } => actor.motion_moving_object_angle_adjust_condition(context, *max_delta_angle)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsEnteringSkitch),
                ..
            } => {
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                actor.motion_runtime_is_entering_skitch(
                    context,
                    services.as_ref().map(|s| &s.skitching),
                )?
            }
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsSkitching),
                ..
            } => actor.motion_runtime_is_skitching(context)?,
            MotionCondition::Motion { payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsSkitchingWithAbsorb), .. } => {
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                actor.motion_skitch_with_absorb(context, services.as_ref().map(|s| &s.skitching))?
            }
            MotionCondition::Motion { payload: ConditionPayload::SkitchingPosition { desired }, .. } => actor.motion_skitch_position(context, *desired)?,
            MotionCondition::Motion { payload: ConditionPayload::IsSkitchShimmying { direction }, .. } => actor.motion_skitch_shimmying(context, *direction)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsPhysicsWiping),
                ..
            } => actor.motion_runtime_is_physics_wiping(
                context,
                cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?,
            )?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::PhysicsWantsWipeOut,
                    ),
                ..
            } => actor.motion_runtime_wants_wipeout(
                context,
                cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?,
            )?,
            MotionCondition::Motion {
                payload: ConditionPayload::LastState { state },
                ..
            } => grounded_selection::state_matches(
                &super::data::STATES,
                parent,
                self.last_state,
                state,
            )?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::ManualOutTimerIsActive,
                    ),
                ..
            } => actor.motion_runtime_manual_out_timer_is_active(context)?,
            MotionCondition::Motion {
                payload: ConditionPayload::IsProSkater { skater },
                ..
            } => actor.motion_runtime_is_pro_skater(context, skater)?,
            MotionCondition::Motion {
                payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsBumped),
                ..
            } => actor.motion_runtime_is_bumped(
                context,
                cvars.as_ref().ok_or(E::MissingOwner("Motion cvars"))?,
            )?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Comparison {
                        class: super::conditions::plain::ComparisonCondition::SurfaceSlope,
                        comparison,
                    },
                ..
            } => actor.motion_runtime_surface_slope(context, *comparison)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Comparison {
                        class: super::conditions::plain::ComparisonCondition::WipeoutTimeToLand,
                        comparison,
                    },
                ..
            } => actor.motion_runtime_wipeout_time_to_land(context, *comparison)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsBodyFlipping),
                ..
            } => actor.motion_runtime_is_body_flipping(context)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(super::conditions::plain::PlainCondition::IsRidingGoofy),
                ..
            } => actor.motion_runtime_is_riding_goofy(context)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::HasTiltToLargeForPreland,
                    ),
                ..
            } => {
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                actor.motion_has_tilt_too_large(context,crate::actor_construction::motion_runtime_session::prepare_land::PrepareLandRuntime {
                    attributes:services.as_ref().map(|s|&s.prepare_land),body_tilt:services.as_ref().map(|s|&s.body_tilt),cvars:cvars.as_ref(),
                })?
            }
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Comparison {
                        class: super::conditions::plain::ComparisonCondition::TimeToLand,
                        comparison,
                    },
                ..
            } => {
                actor.check_motion_runtime_context(context)?;
                actor.runtime_time_to_land(*comparison)?
            }
            MotionCondition::Motion {
                payload: ConditionPayload::HasLandingType { landing },
                ..
            } => actor.motion_has_landing_type(context, *landing)?,
            MotionCondition::Motion {
                payload: ConditionPayload::Plain(super::conditions::plain::PlainCondition::TrucksOrDeckInContact),
                ..
            } => actor.motion_trucks_or_deck_contact(context)?,
            MotionCondition::Motion {
                payload: ConditionPayload::CurrentGrabType { grab },
                ..
            } => actor.motion_runtime_current_grab_type(context, *grab)?,
            MotionCondition::Motion {
                payload:
                    ConditionPayload::Plain(
                        super::conditions::plain::PlainCondition::IsGrindBluntingBackslash,
                    ),
                ..
            } => actor.motion_runtime_is_grind_blunting_backslash(context)?,
            MotionCondition::Motion { .. } => return Err(E::PendingCondition(index)),
        };
        self.condition_active[word] |= bit;
        self.condition_results[position] = Some(value);
        Ok(value)
    }
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
