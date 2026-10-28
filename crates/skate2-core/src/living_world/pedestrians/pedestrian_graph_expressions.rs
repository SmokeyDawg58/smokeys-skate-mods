//! HCDelegate_Pedestrian EvaluateExpression<1>828fc2d8/<2>828fd248.
//! Native defaults really return true; missing concrete conditions propagate
//! through the callback. Preserve lazy order and independent sustain masks.
use super::ids::{condition as cond, expression as expr};
use crate::generated_graph_selection::Mask;

pub(crate) fn evaluate<E>(
    mask: Mask,
    expression: u16,
    mut c: impl FnMut(u16) -> Result<bool, E>,
) -> Result<bool, E> {
    if mask == Mask::Sustain {
        return Ok(match expression {
            expr::PATROL_ZONE_BOUNDARY_4 => !c(cond::PATROL_ZONE_OWNER_IS_IN_OWN_ZONE)?,
            expr::APPROACH_ZONE_POINT_OF_INTEREST_12 => {
                c(cond::ENTITY_OF_INTEREST_IS_IN_MAINTAIN_INTEREST_RADIUS)?
            }
            expr::ROAD_22 => c(cond::HAS_ROAD_WANDER_TARGET)?,
            expr::USE_CROSS_WALK_40 => c(cond::IS_ENTERING_CROSS_WALK)?,
            expr::WAIT_71 => !c(cond::WALK_SIGN_SAYS_GO)?,
            expr::PATROL_9 => c(cond::IS_PATROLLING_2ND)?,
            expr::PEDESTRIAN_WANT_TO_WARN_25 => c(cond::HAS_WANT_WARN_2ND)?,
            expr::THROW_HAND_PROP_45 => c(cond::HAS_WANT_THROW_HAND_PROP)?,
            expr::GREET_PERP_46 => c(cond::HAS_WANT_GREET)?,
            expr::RETURN_GREET_47 => c(cond::HAS_WANT_RETURN_GREET)?,
            expr::PEDESTRIAN_WANT_TO_START_CONVERSATION_28 => c(cond::HAS_WANT_START_CONVERSATION)?,
            expr::PEDESTRIAN_WANT_TO_TAUNT_29 => c(cond::HAS_WANT_TAUNT)?,
            expr::DO_TAUNT_49 => c(cond::HAS_MONITORED_INTENT_SG_INTENT)?,
            expr::PEDESTRIAN_WANT_TO_TAZE_30 => c(cond::HAS_WANT_TAZE)?,
            expr::PEDESTRIAN_WANT_TO_CHASE_PERP_52 | expr::NEW_CHASEE_35 => {
                c(cond::HAS_WANT_ANGRY_CHASE_2ND)?
            }
            expr::PEDESTRIAN_WANT_TO_CHASE_VICTIM_53 => c(cond::HAS_WANT_JOIN_CHASE_2ND)?,
            expr::PEDESTRIAN_WANT_TO_BECOME_ALERT_32 => c(cond::HAS_WANT_ALERT_TO)?,
            expr::FLEE_33 => c(cond::HAS_WANT_FLEE)?,
            expr::IS_CHASING => c(cond::IS_CHASING_2ND)?,
            expr::HAS_HAND_PROP_83 => c(cond::HAS_HAND_PROP_2ND)?,
            expr::ATTEMPT_TAKE_DOWN_TARGETABLE_87 => c(cond::HAS_MONITORED_INTENT_ACTIVE_TAKEDOWN)?,
            expr::CHASE_END_REASON_AUTO_END_CHASE_89 => c(cond::IS_CHASER_ENDING_CHASE_2ND)?,
            expr::ESCAPED_AUTO_END_CHASE_90 => c(cond::CHASEE_ESCAPED)?,
            expr::NO_CHASER_AUTO_END_CHASE_93 => !c(cond::IS_CHASING_2ND)?,
            expr::WARN_AND_WAIT_94 => c(cond::CANNOT_REACH_CHASE_TARGET)?,
            expr::CHASER_EXHAUSTED_63 => c(cond::NEED_TO_REST)?,
            expr::SEARCH_FOR_CHASEE_96 => !c(cond::CAN_SEE_CHASEE)?,
            expr::WITH_PROTECTOR_98 => c(cond::CHASEE_HAS_PROTECTOR)?,
            expr::BLOCK_CHASEE => c(cond::CHASER_SHOULD_BLOCK)?,
            expr::OBSERVE_COLLISION_42 => {
                c(cond::HAS_WANT_NEARBY_COLLISION_REACTION)? && !c(cond::SIMPLE_TIMER_EXPIRED)?
            }
            expr::OBSERVE_SLAM_43 => {
                c(cond::HAS_WANT_SLAM_REACTION)? && !c(cond::SIMPLE_TIMER_EXPIRED)?
            }
            expr::WATCH_GREET_TARGET_102 => c(cond::DISTANCE_TO_WANT_TARGET_GREET_LESS_5)?,
            expr::WATCH_GREET_TARGET_121 => c(cond::DISTANCE_TO_WANT_TARGET_RETURN_GREET_LESS_5)?,
            expr::DROP_HAND_PROP_50 => {
                c(cond::HAS_HAND_PROP_2ND)? && !c(cond::IS_HOLDING_SPECIFIC_HAND_PROP_TAZR)?
            }
            expr::DO_ALERT_81 => !c(cond::SIMPLE_TIMER_EXPIRED_ALERT_TIMER)?,
            expr::WATCH_ALERT_TARGET_108 => c(cond::DISTANCE_TO_WANT_TARGET_ALERT_TO_LESS_7_5)?,
            expr::INTERCEPT_CHASEE_65 => {
                c(cond::IS_PRIMARY_CHASER)?
                    && !c(cond::CHASERS_ARE_SCARED)?
                    && !c(cond::CHASEE_HAS_PROTECTOR)?
            }
            expr::AGGRESSIVE_CAPTURE_AUTO_END_CHASE_91 => {
                c(cond::CHASEE_CLOSE_ENOUGH_TO_SATISFY_AGGRESSIVE_CHASE)?
                    && c(cond::CHASEE_IS_WIPED_OUT_2ND)?
                    && !c(cond::CHASEE_HAS_PROTECTOR)?
            }
            _ => true,
        });
    }
    Ok(match expression {
        expr::TO_PLUGIN => c(cond::HAS_PLUGIN)?,
        expr::TO_PATROL => c(cond::IS_PATROLLING)?,
        expr::NOT_IS_PEDESTRIAN_COLLIDING => !c(cond::IS_PEDESTRIAN_COLLIDING)?,
        expr::PATROL_ZONE_BOUNDARY_4 => !c(cond::PATROL_ZONE_OWNER_IS_IN_OWN_ZONE)?,
        expr::TO_COLLIDING => c(cond::NEEDS_TO_BEGIN_COLLIDING)?,
        expr::APPROACH_ZONE_POINT_OF_INTEREST_12 => {
            c(cond::ENTITY_OF_INTEREST_IS_IN_MAINTAIN_INTEREST_RADIUS)?
        }
        expr::WAIT_OUT_MAINTAIN_INTEREST_TIMER_13_TO_RETURN_TO_PATROL_ZONE_14 => {
            !c(cond::PATROL_ZONE_MAINTAIN_INTEREST_TIMER_ACTIVE)?
        }
        expr::WANDER_5_TO_RUN_FROM_HONKER_7 => c(cond::IS_BEING_HONKED_AT)?,
        expr::ROAD_22 => c(cond::HAS_ROAD_WANDER_TARGET)?,
        expr::USE_CROSS_WALK_40 => c(cond::IS_ENTERING_CROSS_WALK)?,
        expr::WAIT_71 => !c(cond::WALK_SIGN_SAYS_GO)?,
        expr::TO_WANDER => !c(cond::IS_ON_ROAD)?,
        expr::PLUGIN_8_TO_WANDER_5 => !c(cond::HAS_PLUGIN)?,
        expr::PATROL_9 => c(cond::IS_PATROLLING_2ND)?,
        expr::PATROL_9_TO_WANDER_5 => !c(cond::IS_TARGETING_WAYPOINT)?,
        expr::RESUMING_ACTIVE_CHASE_16 => c(cond::IS_CHASING)?,
        expr::PEDESTRIAN_WANT_TO_WARN_25 => c(cond::HAS_WANT_WARN_2ND)?,
        expr::THROW_HAND_PROP_45 => c(cond::HAS_WANT_THROW_HAND_PROP)?,
        expr::FACING_TARGET_100 => {
            c(cond::IS_FACING_WANT_TARGET_THROW_HAND_PROP_ANGLE_120_DISTANCE_20)?
        }
        expr::GREET_PERP_46 => c(cond::HAS_WANT_GREET)?,
        expr::DO_GREET_75_TO_WATCH_RETURN_GREET_76 => c(cond::SIMPLE_TIMER_EXPIRED_INTEREST_TIMER)?,
        expr::SIMPLE_TIMER_EXPIRED => c(cond::SIMPLE_TIMER_EXPIRED_2ND)?,
        expr::RETURN_GREET_47 => c(cond::HAS_WANT_RETURN_GREET)?,
        expr::PEDESTRIAN_WANT_TO_START_CONVERSATION_28 => c(cond::HAS_WANT_START_CONVERSATION)?,
        expr::PEDESTRIAN_WANT_TO_TAUNT_29 => c(cond::HAS_WANT_TAUNT)?,
        expr::PEDESTRIAN_WANT_TO_TAZE_30 => c(cond::HAS_WANT_TAZE)?,
        expr::TAZE_ENTITY_TO_END_TAZE => c(cond::SIMPLE_TIMER_EXPIRED_TAZER_CYC_TIME)?,
        expr::PEDESTRIAN_WANT_TO_CHASE_PERP_52 => c(cond::HAS_WANT_ANGRY_CHASE_2ND)?,
        expr::PEDESTRIAN_WANT_TO_CHASE_VICTIM_53 => c(cond::HAS_WANT_JOIN_CHASE_2ND)?,
        expr::PEDESTRIAN_WANT_TO_BECOME_ALERT_32 => c(cond::HAS_WANT_ALERT_TO)?,
        expr::FLEE_33 => c(cond::HAS_WANT_FLEE)?,
        expr::HAS_ESCAPED_55 => c(cond::DISTANCE_TO_WANT_TARGET_FLEE_GREATER_15)?,
        expr::IS_CHASING => c(cond::IS_CHASING_2ND)?,
        expr::HAS_HAND_PROP_83 => c(cond::HAS_HAND_PROP_2ND)?,
        expr::CAN_THROW_HAND_PROP_110 => c(cond::CAN_ATTACK_THROW_HAND_PROP)?,
        expr::FACING_TARGET_123 => {
            c(cond::IS_FACING_WANT_TARGET_ANGRY_CHASE_ANGLE_120_DISTANCE_20)?
        }
        expr::HOLD_BEFORE_CHASE_START_58_TO_START_CHASE_FOR_REAL_59 => {
            c(cond::SIMPLE_TIMER_EXPIRED_START_CHASE)?
        }
        expr::FAILED_TO_THROW_HAND_PROP_85 => c(cond::HAS_HAND_PROP)?,
        expr::WAIT_FOR_TAKEDOWN_DECISION_112_TO_TAKE_DOWN_TARGETABLE_SUCCESS_114 => {
            c(cond::TAKE_DOWN_ATTEMPT_SUCCESSFUL)?
        }
        expr::TAUNT_AFTER_TAKEDOWN => c(cond::HAS_WANT_ANGRY_CHASE)?,
        expr::GIVE_UP_DUE_TO_TAKEDOWNS_AUTO_END_CHASE_88 => {
            c(cond::CHASER_SHOULD_GIVE_UP_DUE_TO_TAKEDOWNS)?
        }
        expr::CHASE_END_REASON_AUTO_END_CHASE_89 => c(cond::IS_CHASER_ENDING_CHASE_2ND)?,
        expr::ESCAPED_AUTO_END_CHASE_90 => c(cond::CHASEE_ESCAPED)?,
        expr::NOT_ENTITY_OF_INTEREST_IS_IN_ZONE_BOUNDARY_OR_BUFFER => {
            !c(cond::ENTITY_OF_INTEREST_IS_IN_ZONE_BOUNDARY_OR_BUFFER)?
        }
        expr::NO_CHASER_AUTO_END_CHASE_93 => !c(cond::IS_CHASING_2ND)?,
        expr::WARN_AND_WAIT_94 => c(cond::CANNOT_REACH_CHASE_TARGET)?,
        expr::WARN_AND_WAIT_94_TO_UNREACHABLE_TIMER_EXPIRED_95 => {
            c(cond::SIMPLE_TIMER_EXPIRED_TARGET_UNREACHABLE_TIMER)?
        }
        expr::UNREACHABLE_TIMER_EXPIRED_95 => c(cond::CANNOT_REACH_CHASE_TARGET_2ND)?,
        expr::TAZE_UNREACHABLE_TARGET_117 => c(cond::CAN_TAZE_UNREACHABLE_TARGETS)?,
        expr::END_TAZE_129_TO_GIVE_UP_AFTER_WAIT_118 => {
            !c(cond::IS_END_CHASE_REASON_AGGRESSIVE_CAPTURE)?
        }
        expr::CHASER_EXHAUSTED_63 => c(cond::NEED_TO_REST)?,
        expr::SEARCH_FOR_CHASEE_96 => !c(cond::CAN_SEE_CHASEE)?,
        expr::SEARCH_FOR_CHASEE_96_TO_GIVE_UP_ON_LOST_CHASEE_97 => {
            c(cond::INVESTIGATE_TIME_EXCEEDED)?
        }
        expr::WITH_PROTECTOR_98 => c(cond::CHASEE_HAS_PROTECTOR)?,
        expr::BLOCK_CHASEE => c(cond::CHASER_SHOULD_BLOCK)?,
        expr::AGGRESSIVE_CAPTURE_END_CHASE_67 => c(cond::IS_END_CHASE_REASON_AGGRESSIVE_CAPTURE)?,
        expr::LOST_INTEREST_END_CHASE_69 => c(cond::IS_END_CHASE_REASON_LOST_INTEREST)?,
        expr::WANDER_5_TO_WANDER_ON_ROAD_6 => c(cond::IS_ON_ROAD)? && !c(cond::IS_ON_INTERSECTION)?,
        expr::PLUGIN_8_TO_COLLIDING_3 => {
            c(cond::NEEDS_TO_BEGIN_COLLIDING)? && !c(cond::IS_PLUGIN_COLLISION_OVERRIDE)?
        }
        expr::OBSERVE_COLLISION_42 => c(cond::HAS_WANT_NEARBY_COLLISION_REACTION)?,
        expr::OBSERVE_SLAM_43 => c(cond::HAS_WANT_SLAM_REACTION)?,
        expr::CAN_THROW_HAND_PROP_73 => {
            c(cond::HAS_HAND_PROP)? && c(cond::CAN_ATTACK_THROW_HAND_PROP)?
        }
        expr::WATCH_GREET_TARGET_102 => c(cond::DISTANCE_TO_WANT_TARGET_GREET_LESS_3_5)?,
        expr::WATCH_GREET_TARGET_121 => c(cond::DISTANCE_TO_WANT_TARGET_RETURN_GREET_LESS_3_5)?,
        expr::DROP_HAND_PROP_50 => {
            c(cond::HAS_HAND_PROP_2ND)? && !c(cond::IS_HOLDING_SPECIFIC_HAND_PROP_TAZR)?
        }
        expr::DO_ALERT_81 => c(cond::IS_WANT_TARGET_WITHIN_ALERT_DISTANCE_ALERT_TO)?,
        expr::DO_ALERT_81_TO_PATROL_ZONE_BOUNDARY_4 => {
            !c(cond::ENTITY_OF_INTEREST_IS_IN_ZONE_BOUNDARY_OR_BUFFER)?
                || !c(cond::PATROL_ZONE_OWNER_IS_IN_OWN_ZONE_BOUNDARY_OR_BUFFER)?
        }
        expr::WATCH_ALERT_TARGET_108 => c(cond::DISTANCE_TO_WANT_TARGET_ALERT_TO_LESS_4_5)?,
        expr::NEW_CHASEE_35 => c(cond::HAS_WANT_ANGRY_CHASE_2ND)? && !c(cond::IS_CHASING)?,
        expr::CANCEL_CHASE_57 => {
            !c(cond::CAN_NEW_CHASE_START)? || !c(cond::CAN_CHASEE_ADD_NEW_CHASER)?
        }
        expr::WAIT_FOR_TAKEDOWN_DECISION_112_TO_TAKE_DOWN_TARGETABLE_FAILURE_113 => {
            c(cond::TAKE_DOWN_ATTEMPT_FAILED)?
                || c(cond::SIMPLE_TIMER_EXPIRED_ATTEMPT_TAKE_DOWN_TIMEOUT)?
        }
        expr::INTERCEPT_CHASEE_65 => {
            c(cond::IS_PRIMARY_CHASER)?
                && !c(cond::CHASERS_ARE_SCARED)?
                && !c(cond::CHASEE_HAS_PROTECTOR)?
        }
        expr::PLUGIN_8_TO_PEDESTRIAN_ADDRESS_WANTS_10 => {
            (c(cond::HAS_WANT_WARN)?
                || c(cond::HAS_WANT_ANGRY_CHASE)?
                || c(cond::HAS_WANT_JOIN_CHASE)?)
                && !c(cond::IS_PEDESTRIAN_COLLIDING)?
        }
        expr::DRAW_TAZER_78_TO_TAZE_ENTITY_79 => {
            c(cond::IS_FACING_WANT_TARGET_TAZE_ANGLE_20_DISTANCE_10)?
                && c(cond::HAS_LINE_OF_SIGHT_TO_TAZE_TARGET)?
                && c(cond::HAS_DRAWN_TAZER)?
        }
        expr::AGGRESSIVE_CAPTURE_AUTO_END_CHASE_91 => {
            c(cond::CHASEE_CLOSE_ENOUGH_TO_SATISFY_AGGRESSIVE_CHASE)?
                && c(cond::CHASEE_IS_WIPED_OUT_2ND)?
                && !c(cond::CHASEE_HAS_PROTECTOR)?
        }
        expr::DRAW_TAZER_127_TO_TAZE_ENTITY_128 => {
            c(cond::IS_FACING_WANT_TARGET_ANGRY_CHASE_ANGLE_20_DISTANCE_15)?
                && c(cond::HAS_LINE_OF_SIGHT_TO_TAZE_TARGET)?
                && c(cond::HAS_DRAWN_TAZER)?
        }
        expr::PEDESTRIAN_TAKE_DOWN_CHASEE_60 => {
            c(cond::IS_CHASING)?
                && c(cond::HAS_TAKE_DOWN_TARGETABLE)?
                && c(cond::CAN_ATTEMPT_TAKE_DOWN_TARGETABLE)?
                && !c(cond::IS_CHASER_ENDING_CHASE)?
                && !c(cond::CHASEE_IS_WIPED_OUT)?
        }
        _ => true,
    })
}

/// `HCDelegate_marquee::EvaluateExpression<1>` 0x828ff168. Its `<2>` is inlined in
/// `GetStateActivation<2>` 0x82948438, which reads only expression 2.
pub(crate) fn evaluate_marquee<E>(
    mask: Mask,
    expression: u16,
    mut c: impl FnMut(u16) -> Result<bool, E>,
) -> Result<bool, E> {
    if mask == Mask::Sustain {
        return Ok(match expression {
            2 => c(2)?,
            _ => true,
        });
    }
    Ok(match expression {
        0 => c(0)?,
        1 => !c(1)?,
        2 => c(2)?,
        _ => true,
    })
}

/// `HCDelegate_Photographer::EvaluateExpression<1>` 0x828ff608 and `<2>` 0x828ff728.
pub(crate) fn evaluate_photographer<E>(
    mask: Mask,
    expression: u16,
    mut c: impl FnMut(u16) -> Result<bool, E>,
) -> Result<bool, E> {
    if mask == Mask::Sustain {
        return Ok(match expression {
            2 => c(2)?,
            3 => c(3)?,
            4 => c(4)?,
            _ => true,
        });
    }
    Ok(match expression {
        0 => c(0)?,
        1 => !c(1)?,
        2 => c(2)?,
        3 => c(3)?,
        4 => c(4)?,
        _ => true,
    })
}
