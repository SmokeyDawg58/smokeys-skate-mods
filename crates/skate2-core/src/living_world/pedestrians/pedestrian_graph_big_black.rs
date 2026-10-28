//! HCDelegate_Big_Black expressions 828fbeb0 (acquire) / 828fc1c0 (sustain).
use super::big_black_ids::{condition as cond, expression as expr};
use crate::{
    generated_action_load::runtime::ActionUpdateError as Error, generated_graph_selection::Mask,
};
pub(crate) fn evaluate(
    mask: Mask,
    expression: u16,
    c: &mut dyn FnMut(u16) -> Result<bool, Error>,
) -> Result<bool, Error> {
    Ok(match mask {
        Mask::Pre => match expression {
            expr::WAIT_FOR_TAKEDOWN_DECISION_12_TO_TAKE_DOWN_TARGETABLE_SUCCESS_14 => {
                c(cond::TAKE_DOWN_ATTEMPT_SUCCESSFUL)?
            }
            expr::TAUNT_AFTER_TAKEDOWN_17 => c(cond::HAS_WANT_ANGRY_CHASE)?,
            expr::PROTECT_PROTECTEE_5 => c(cond::IS_PROTECTEE_UNDER_THREAT)?,
            expr::CHASING_8 => c(cond::NEEDS_TO_INTERCEPT_THREAT)?,
            expr::BLOCK_9 => c(cond::NEEDS_TO_BLOCK_THREAT)?,
            expr::COLLIDING_2 => {
                c(cond::NEEDS_TO_BEGIN_COLLIDING)? || c(cond::IS_PEDESTRIAN_COLLIDING)?
            }
            expr::GUARD_PROTECTEE_3 => {
                c(cond::WANTS_TO_PROTECT_PROTECTEE)? || c(cond::IS_PROTECTING_PROTECTEE)?
            }
            expr::BIG_BLACK_TAKE_DOWN_4 => {
                c(cond::HAS_TAKE_DOWN_TARGETABLE)? && c(cond::CAN_ATTEMPT_TAKE_DOWN_TARGETABLE)?
            }
            expr::WAIT_FOR_TAKEDOWN_DECISION_12_TO_TAKE_DOWN_TARGETABLE_FAILURE_13 => {
                c(cond::TAKE_DOWN_ATTEMPT_FAILED)?
                    || c(cond::SIMPLE_TIMER_EXPIRED_ATTEMPT_TAKE_DOWN_TIMEOUT)?
            }
            expr::RUN_15 => {
                c(cond::DISTANCE_FROM_PROTECTEE_GREATER_20)?
                    && c(cond::DISTANCE_FROM_PROTECTEE_GREATER_10)?
            }
            expr::WALK_16 => {
                c(cond::DISTANCE_FROM_PROTECTEE_GREATER_12)?
                    && c(cond::DISTANCE_FROM_PROTECTEE_GREATER_6)?
            }
            _ => true,
        },
        Mask::Sustain => match expression {
            expr::ATTEMPT_TAKE_DOWN_TARGETABLE_7 => c(cond::HAS_MONITORED_INTENT_ACTIVE_TAKEDOWN)?,
            expr::PROTECT_PROTECTEE_5 => c(cond::IS_PROTECTEE_UNDER_THREAT)?,
            expr::CHASING_8 => c(cond::NEEDS_TO_INTERCEPT_THREAT)?,
            expr::BLOCK_9 => c(cond::NEEDS_TO_BLOCK_THREAT)?,
            expr::COLLIDING_2 => c(cond::IS_PEDESTRIAN_COLLIDING)?,
            expr::GUARD_PROTECTEE_3 => c(cond::IS_PROTECTING_PROTECTEE)?,
            expr::BIG_BLACK_TAKE_DOWN_4 => c(cond::HAS_TAKE_DOWN_TARGETABLE)?,
            expr::RUN_15 => c(cond::DISTANCE_FROM_PROTECTEE_GREATER_10)?,
            expr::WALK_16 => c(cond::DISTANCE_FROM_PROTECTEE_GREATER_6)?,
            _ => true,
        },
    })
}
