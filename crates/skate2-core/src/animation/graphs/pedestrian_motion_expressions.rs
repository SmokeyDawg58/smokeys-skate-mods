//! Complete EvaluateExpression<1>82922538, with lazy source order.
//! The caller supplies the native per-frame condition cache.
use super::ids::{condition as cond, expression as expr};
use crate::generated_graph_selection::Mask;

/// `HCDelegate_motiongraph_marquee::EvaluateExpression<1>` 0x828ff220. Its
/// `GetStateActivation<2>` 0x82949d68 sustains every state.
pub(crate) fn evaluate_marquee<E>(
    mask: Mask,
    expression: u16,
    c: &mut impl FnMut(u16) -> Result<bool, E>,
) -> Result<bool, E> {
    if mask == Mask::Sustain {
        return Ok(true);
    }
    Ok(match expression {
        0..=5 => c(expression)?,
        6 => !c(0)?,
        _ => true,
    })
}

/// `HCDelegate_Motiongraph_Photographer::EvaluateExpression<1>` 0x828ff388 and `<2>`
/// 0x828ff590, whose jump table reads a condition for expression 7 only.
pub(crate) fn evaluate_photographer<E>(
    mask: Mask,
    expression: u16,
    c: &mut impl FnMut(u16) -> Result<bool, E>,
) -> Result<bool, E> {
    if mask == Mask::Sustain {
        return Ok(match expression {
            7 => c(6)?,
            _ => true,
        });
    }
    Ok(match expression {
        0..=5 => c(expression)?,
        6 => !c(0)?,
        7 => c(6)?,
        8 => c(7)?,
        9 => c(8)?,
        10 => c(9)?,
        _ => true,
    })
}

pub(crate) fn precondition<E>(
    expression: u16,
    c: &mut impl FnMut(u16) -> Result<bool, E>,
) -> Result<bool, E> {
    Ok(match expression {
        expr::HAS_INTENT_COLLISION => c(cond::HAS_INTENT_COLLISION)?,
        expr::FROM_FRONT => c(cond::HAS_INTENT_FROM_FRONT)?,
        expr::LYING_DOWN => c(cond::HAS_INTENT_LYING_DOWN)?,
        expr::WILL_EXPIRE_IN_0_01 => c(cond::WILL_EXPIRE_IN_0_01)?,
        expr::CROUCHED => c(cond::HAS_INTENT_CROUCHED)?,
        expr::ON_THE_GROUND_TO_GET_UP => c(cond::HAS_INTENT_RECOVER)?,
        expr::GET_UP_TO_KNOCKDOWN => c(cond::HAS_INTENT_RECOVERY_COLLISION)?,
        expr::WILL_EXPIRE_ALWAYS_IN_0_01 => c(cond::WILL_EXPIRE_ALWAYS_IN_0_01)?,
        expr::TO_LOCOMOTION => !c(cond::HAS_INTENT_COLLISION)?,
        expr::FROM_BACK => c(cond::HAS_INTENT_FROM_BACK)?,
        expr::FROM_LEFT => c(cond::HAS_INTENT_FROM_LEFT)?,
        expr::FROM_RIGHT => c(cond::HAS_INTENT_FROM_RIGHT)?,
        expr::TAKEDOWN_3 => c(cond::HAS_INTENT_DO_TAKE_DOWN)?,
        expr::ATTACK_CLOTHES_LINE_R_22 => c(cond::HAS_INTENT_CLOTHES_LINE_R)?,
        expr::PLAY_ATTACK_TO_ATTACK_COMPLETE => c(cond::WILL_EXPIRE_IN_0_03)?,
        expr::ATTACK_CLOTHES_LINE_L_23 => c(cond::HAS_INTENT_CLOTHES_LINE_L)?,
        expr::ATTACK_PUSH_R_135_24 => c(cond::HAS_INTENT_PUSH_R_135)?,
        expr::ATTACK_PUSH_L_135_25 => c(cond::HAS_INTENT_PUSH_L_135)?,
        expr::ATTACK_PUSH_R_90_26 => c(cond::HAS_INTENT_PUSH_R_90)?,
        expr::ATTACK_PUSH_L_90_27 => c(cond::HAS_INTENT_PUSH_L_90)?,
        expr::ATTACK_PUSH_FRONT_28 => c(cond::HAS_INTENT_PUSH_FRONT)?,
        expr::ATTACK_PUSH_CATCH_29 => c(cond::HAS_INTENT_PUSH_CATCH)?,
        expr::ATTACK_CHECK_FRONT_30 => c(cond::HAS_INTENT_CHECK_FRONT)?,
        expr::ATTACK_CHECK_RIGHT_31 => c(cond::HAS_INTENT_CHECK_RIGHT)?,
        expr::ATTACK_CHECK_LEFT_32 => c(cond::HAS_INTENT_CHECK_LEFT)?,
        expr::HAS_INTENT_AVOID => c(cond::HAS_INTENT_AVOID)?,
        expr::HAS_INTENT_FORWARD => c(cond::HAS_INTENT_FORWARD)?,
        expr::HAS_INTENT_LEFT => c(cond::HAS_INTENT_LEFT)?,
        expr::HAS_INTENT_RIGHT => c(cond::HAS_INTENT_RIGHT)?,
        expr::HAS_INTENT_BACK => c(cond::HAS_INTENT_BACK)?,
        expr::STILL_SITTING_407_TO_NOT_SITTING_ANYMORE_408 => !c(cond::HAS_PLUGIN)?,
        expr::SIT_SMALL_34 => c(cond::LAST_STATE_LOCOMOTION_SIT_SITTING_CYC)?,
        expr::HAS_INTENT_JUMP => c(cond::HAS_INTENT_JUMP)?,
        expr::STEP_36 => c(cond::HAS_INTENT_STEP)?,
        expr::BRANCH_WINDOW_ENABLED => c(cond::BRANCH_WINDOW_ENABLED)?,
        expr::IN_TURN_BRANCH_WINDOW => c(cond::IN_TURN_BRANCH_WINDOW)?,
        expr::AVOID_COMPLETE_37_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_AVOID)?,
        expr::JUMP_5 => c(cond::HAS_INTENT_CHASE_JUMP)?,
        expr::JUMP_AIR_UP_39_TO_JUMP_AIR_DOWN_40 => c(cond::PEDESTRIAN_JUMPER_IN_DOWNWARD_MOTION)?,
        expr::TO_JUMP_OUT => c(cond::PEDESTRIAN_JUMPER_IS_LANDING)?,
        expr::JUMP_OUT_41_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_CHASE_JUMP_2ND)?,
        expr::TAUNT_6 => c(cond::HAS_INTENT_TAUNT)?,
        expr::TAUNT_COMPLETE_43_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_TAUNT)?,
        expr::DISAPPOINT_7 => c(cond::HAS_INTENT_DISAPPOINT)?,
        expr::DISAPPOINT_COMPLETE_45_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_DISAPPOINT)?,
        expr::CONVERSE_8 => c(cond::HAS_INTENT_CONVERSE)?,
        expr::CYCLE_413_TO_CONVERSE_COMPLETE_47 => c(cond::HAS_INTENT_STOP_CONVERSE)?,
        expr::CONVERSE_COMPLETE_47_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_CONVERSE)?,
        expr::SIT_9 => c(cond::HAS_INTENT_SIT)?,
        expr::SIT_COMPLETE_51_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_SIT_2ND)?,
        expr::USE_ATM_10 => c(cond::HAS_INTENT_USE_ATM)?,
        expr::USE_ATM_COMPLETE_188_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_USE_ATM_2ND)?,
        expr::USE_VENDING_MACHINE_11 => c(cond::HAS_INTENT_USE_VENDING_MACHINE)?,
        expr::USE_VENDING_MACHINE_COMPLETE_192_TO_LOCOMOTION_1 => {
            !c(cond::HAS_INTENT_USE_VENDING_MACHINE_2ND)?
        }
        expr::USE_WATER_FOUNTAIN_12 => c(cond::HAS_INTENT_USE_WATER_FOUNTAIN)?,
        expr::WATER_FOUNTAIN_CYC_194_TO_WATER_FOUNTAIN_OUT_195 => {
            c(cond::HAS_INTENT_FINISH_DRINKING)?
        }
        expr::WATER_FOUNTAIN_COMPLETE_196_TO_LOCOMOTION_1 => {
            !c(cond::HAS_INTENT_USE_WATER_FOUNTAIN_2ND)?
        }
        expr::USE_NEWSPAPER_BOX_13 => c(cond::HAS_INTENT_USE_NEWSPAPER_BOX)?,
        expr::USE_NEWSPAPER_BOX_COMPLETE_198_TO_LOCOMOTION_1 => {
            !c(cond::HAS_INTENT_USE_NEWSPAPER_BOX_2ND)?
        }
        expr::STRAFE_14 => c(cond::HAS_INTENT_STRAFE)?,
        expr::WILL_EXPIRE_ALWAYS_IN_0_03 => c(cond::WILL_EXPIRE_ALWAYS_IN_0_03)?,
        expr::LAST_STATE_LOCOMOTION_CHASE_RUN => c(cond::LAST_STATE_LOCOMOTION_CHASE_RUN)?,
        expr::REVERSE_RIGHT => c(cond::HAS_INTENT_REVERSE_RIGHT)?,
        expr::WILL_EXPIRE_ALWAYS_IN_0_1 => c(cond::WILL_EXPIRE_ALWAYS_IN_0_1)?,
        expr::REVERSE_LEFT => c(cond::HAS_INTENT_REVERSE_LEFT)?,
        expr::TURN_RIGHT_135 => c(cond::HAS_INTENT_TURN_RIGHT_135)?,
        expr::TURN_LEFT_135 => c(cond::HAS_INTENT_TURN_LEFT_135)?,
        expr::TURN_RIGHT_90 => c(cond::HAS_INTENT_TURN_RIGHT_90)?,
        expr::TURN_LEFT_90 => c(cond::HAS_INTENT_TURN_LEFT_90)?,
        expr::TURN_RIGHT_45 => c(cond::HAS_INTENT_TURN_RIGHT_45)?,
        expr::TURN_LEFT_45 => c(cond::HAS_INTENT_TURN_LEFT_45)?,
        expr::MOVE_FORWARDS_222 => c(cond::HAS_INTENT_MOVE_FORWARDS)?,
        expr::MOVE_BACKWARDS_223 => c(cond::HAS_INTENT_MOVE_BACKWARDS)?,
        expr::MOVE_LEFT_45_224 => c(cond::HAS_INTENT_MOVE_LEFT_45)?,
        expr::MOVE_RIGHT_45_225 => c(cond::HAS_INTENT_MOVE_RIGHT_45)?,
        expr::MOVE_LEFT_90_226 => c(cond::HAS_INTENT_MOVE_LEFT_90)?,
        expr::MOVE_RIGHT_90_227 => c(cond::HAS_INTENT_MOVE_RIGHT_90)?,
        expr::MOVE_LEFT_135_228 => c(cond::HAS_INTENT_MOVE_LEFT_135)?,
        expr::MOVE_RIGHT_135_229 => c(cond::HAS_INTENT_MOVE_RIGHT_135)?,
        expr::CHASE_RUN_15 => c(cond::HAS_INTENT_CHASE_RUN)?,
        expr::LAST_STATE_LOCOMOTION_STRAFE => c(cond::LAST_STATE_LOCOMOTION_STRAFE)?,
        expr::SLOW_274 => c(cond::HAS_INTENT_SLOW)?,
        expr::FAST_275 => c(cond::HAS_INTENT_FAST)?,
        expr::SHUFFLE_16 => c(cond::HAS_INTENT_SHUFFLE)?,
        expr::IDLE_17 => c(cond::HAS_INTENT_IDLE)?,
        expr::WALK_TO_IDLE_318 => c(cond::LAST_STATE_LOCOMOTION_WALK)?,
        expr::SHUFFLE_TO_IDLE_319 => c(cond::LAST_STATE_LOCOMOTION_SHUFFLE)?,
        expr::TIRED_18 => c(cond::HAS_INTENT_TIRED)?,
        expr::TIRED_18_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_TIRED)?,
        expr::FROM_RUN_347 => c(cond::LAST_STATE_CHASE_RUN_CHASE_RUN_CYC)?,
        expr::FROM_RUN_347_TO_TIRED_INTO_96 => c(cond::IN_BRANCH_WINDOW_TIRED_BRANCH_INFO)?,
        expr::TIRED_COMPLETE_99_TO_LOCOMOTION_1 => !c(cond::HAS_INTENT_STOP_TIRED)?,
        expr::BRISK_382 => c(cond::HAS_INTENT_BRISK)?,
        expr::TAKEDOWN_COMPLETE_TO_LOCOMOTION => {
            !c(cond::HAS_INTENT_TAKEDOWN_SUCCESSFUL)? && !c(cond::HAS_INTENT_TAKEDOWN_FAILED)?
        }
        expr::KNOCKDOWN_20 => {
            c(cond::HAS_INTENT_RECOVERY_COLLISION)? || c(cond::HAS_INTENT_KNOCKDOWN)?
        }
        expr::ATTACK_COMPLETE_TO_TAKEDOWN_SUCCESSFUL => {
            c(cond::HAS_INTENT_TAKEDOWN_SUCCESSFUL)? && c(cond::WILL_EXPIRE_IN_0_01)?
        }
        expr::ATTACK_COMPLETE_TO_TAKEDOWN_FAILED => {
            c(cond::HAS_INTENT_TAKEDOWN_FAILED)? && c(cond::WILL_EXPIRE_IN_0_01)?
        }
        expr::JUMP_35 => c(cond::HAS_INTENT_JUMP)? || c(cond::LAST_STATE_LOCOMOTION_CHASE_RUN)?,
        expr::DO_STEP_182_TO_JUMP_35 => {
            c(cond::HAS_INTENT_JUMP)? && c(cond::IN_TURN_BRANCH_WINDOW)?
        }
        expr::SITTING_INTO_48 => {
            !c(cond::LAST_STATE_LOCOMOTION_AVOID)? && !c(cond::LAST_STATE_LOCOMOTION_COLLIDING)?
        }
        expr::SITTING_CYC_49_TO_SITTING_OUT_50 => {
            c(cond::HAS_INTENT_STAND_UP)? || !c(cond::HAS_INTENT_SIT_2ND)?
        }
        expr::FROM_MOTION_230 => {
            c(cond::LAST_STATE_LOCOMOTION_STRAFE_STRAFE_MOTION)? && c(cond::BRANCH_WINDOW_ENABLED)?
        }
        expr::TIRED_CYC_97_TO_TIRED_OUT_98 => {
            c(cond::HAS_INTENT_STOP_TIRED)? && c(cond::WILL_EXPIRE_ALWAYS_IN_0_01)?
        }
        expr::WAIT_FOR_BRANCH_554 => {
            c(cond::LAST_STATE_WALK_CYC_DEFAULT)? && c(cond::BRANCH_WINDOW_ENABLED)?
        }
        expr::WAIT_FOR_BRANCH_556 => {
            c(cond::LAST_STATE_WALK_CYC_BRISK)? && c(cond::BRANCH_WINDOW_ENABLED)?
        }
        expr::SIT_JUMP_33 => {
            c(cond::HAS_INTENT_JUMP)?
                && (c(cond::LAST_STATE_LOCOMOTION_SIT_SITTING_CYC)?
                    || c(cond::LAST_STATE_SIT_SMALL)?)
        }
        expr::STRAFE_INTO_56 => {
            c(cond::LAST_STATE_LOCOMOTION_WALK)?
                || c(cond::LAST_STATE_LOCOMOTION_SHUFFLE)?
                || c(cond::LAST_STATE_LOCOMOTION_IDLE)?
        }
        expr::WAIT_FOR_BRANCH_488 => {
            c(cond::BRANCH_WINDOW_ENABLED)?
                && (c(cond::LAST_STATE_CHASE_RUN_CYC_FAST)?
                    || c(cond::LAST_STATE_CHASE_RUN_CYC_DEFAULT)?)
        }
        expr::WAIT_FOR_BRANCH_490 => {
            c(cond::BRANCH_WINDOW_ENABLED)?
                && (c(cond::LAST_STATE_CHASE_RUN_CYC_SLOW)?
                    || c(cond::LAST_STATE_CHASE_RUN_CYC_DEFAULT)?)
        }
        expr::WAIT_FOR_BRANCH_492 => {
            c(cond::BRANCH_WINDOW_ENABLED)?
                && (c(cond::LAST_STATE_CHASE_RUN_CYC_SLOW)?
                    || c(cond::LAST_STATE_CHASE_RUN_CYC_FAST)?)
        }
        expr::IDLE_INTO_85 => {
            c(cond::LAST_STATE_LOCOMOTION_WALK)?
                || c(cond::LAST_STATE_LOCOMOTION_CHASE_RUN)?
                || c(cond::LAST_STATE_LOCOMOTION_SHUFFLE)?
        }
        expr::SHUFFLE_INTO_75 => {
            c(cond::LAST_STATE_LOCOMOTION_WALK)?
                || c(cond::LAST_STATE_LOCOMOTION_IDLE)?
                || c(cond::LAST_STATE_LOCOMOTION_AVOID)?
                || c(cond::LAST_STATE_LOCOMOTION_CHASE_RUN)?
        }
        expr::WALK_INTO_100 => {
            c(cond::LAST_STATE_LOCOMOTION_SHUFFLE)?
                || c(cond::LAST_STATE_LOCOMOTION_IDLE)?
                || c(cond::LAST_STATE_LOCOMOTION_AVOID)?
                || c(cond::LAST_STATE_LOCOMOTION_CHASE_RUN)?
        }
        expr::CHASE_RUN_INTO_65 => {
            c(cond::LAST_STATE_LOCOMOTION_WALK)?
                || c(cond::LAST_STATE_LOCOMOTION_IDLE)?
                || c(cond::LAST_STATE_LOCOMOTION_AVOID)?
                || c(cond::LAST_STATE_LOCOMOTION_SHUFFLE)?
                || c(cond::LAST_STATE_LOCOMOTION_TIRED)?
        }
        _ => true,
    })
}
