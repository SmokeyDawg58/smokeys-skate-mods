//! Additional grounded Pre828ff990/Sustain82906048 cases, preserving lazy reads.
use super::{ActionUpdateError as Error, selection::Mask};
use crate::generated_action_load::ids::{condition as cond, expression as expr};
pub(super) fn expression(
    mask: Mask,
    index: u16,
    mut c: impl FnMut(u16) -> Result<bool, Error>,
) -> Result<bool, Error> {
    Ok(match (mask, index) {
        (Mask::Pre, expr::ACCIDENTAL_DISMOUNT_31) => c(cond::PHYSICS_WANTS_RUNOUT)?,
        (Mask::Sustain, expr::HAS_AG_INTENT_TRICK | expr::ACCIDENTAL_DISMOUNT_31) => true, // native Sustain switch defaults
        (Mask::Pre, expr::IS_LANDING_ON_BOARD_OFFBOARD) => {
            c(cond::IS_LANDING_ON_BOARD)? && c(cond::PHYS_FILTERED_STATE_OFFBOARD)?
        }
        (Mask::Sustain, expr::IS_LANDING_ON_BOARD_OFFBOARD) => {
            c(cond::PHYS_FILTERED_STATE_OFFBOARD)?
        }
        // TransitionToOnBoard: Sustain omits the toggle and exit gates.
        (Mask::Pre, expr::TRANS_TO_ON_BOARD_10) => {
            (c(cond::PHYS_FILTERED_STATE_OFFBOARD)?
                || c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND)?)
                && (c(cond::HAS_AG_INTENT_NEW_TOGGLE_OFF_BOARD_STATE)?
                    || c(cond::HAS_ANIM_ATTRIBUTE_IDLE_MOUNTING)?)
                && !c(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?
                && !c(cond::IS_GRABBING_OBJECT)?
        }
        (Mask::Sustain, expr::TRANS_TO_ON_BOARD_10) => {
            (c(cond::PHYS_FILTERED_STATE_OFFBOARD)?
                || c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND)?)
                && c(cond::HAS_ANIM_ATTRIBUTE_IDLE_MOUNTING)?
        }
        // Running: both masks test filtered physics first, then raw locomotion.
        (_, expr::RUNNING_27) => {
            c(cond::PHYS_FILTERED_STATE_OFFBOARD)? && c(cond::IS_IN_LOCOMOTION_2ND)?
        }
        // OBAir Default: IsDoneAirDismount is queried only while offboard-air.
        (_, expr::DEFAULT_33) => {
            c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND)? && c(cond::IS_DONE_AIR_DISMOUNT)?
        }
        (_, expr::OLLIE_ANTICIPATION_83) => c(cond::HAS_AG_INTENT_ANTIC_ANGLE_ABS_LESS_EQUAL_0_52)?,
        (_, expr::NOLLIE_ANTICIPATION_88) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_ABS_GREATER_EQUAL_2_62)?
        }
        (Mask::Pre, expr::HAS_AG_INTENT_TRICK) => c(cond::HAS_AG_INTENT_TRICK)?,
        (_, expr::LEFT_GRAB) => {
            c(cond::HAS_AG_INTENT_LEFT_GROUND_GRAB)? && !c(cond::HAS_AG_INTENT_RIGHT_GROUND_GRAB)?
        }
        (_, expr::RIGHT_GRAB) => {
            c(cond::HAS_AG_INTENT_RIGHT_GROUND_GRAB)? && !c(cond::HAS_AG_INTENT_LEFT_GROUND_GRAB)?
        }
        (_, expr::DBL_GRAB_80) => {
            c(cond::HAS_AG_INTENT_RIGHT_GROUND_GRAB)? && c(cond::HAS_AG_INTENT_LEFT_GROUND_GRAB)?
        }
        (Mask::Pre, expr::ANTIC_46) => {
            c(cond::HAS_AG_INTENT_ANTIC_MAG_GREATER_0_9)? && c(cond::HAS_AG_INTENT_ANTIC_MAG)?
        }
        (Mask::Sustain, expr::ANTIC_46) => c(cond::HAS_AG_INTENT_ANTIC_MAG)?,
        (_, expr::POP_SHUVIT_ANTICIPATION_84) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_NEG_1_05)?
                && c(cond::HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_NEG_0_52)?
        }
        (_, expr::FS_POP_SHUVIT_ANTICIPATION_85) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_0_52)?
                && c(cond::HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_1_05)?
        }
        (_, expr::STATE_360_POP_SHUVIT_ANTICIPATION_86) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_NEG_1_57)?
                && c(cond::HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_NEG_1_05)?
        }
        (_, expr::FS_360_POP_SHUVIT_ANTICIPATION_87) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_1_05)?
                && c(cond::HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_1_57)?
        }
        (_, expr::N_POP_SHUVIT_ANTICIPATION_89) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_NEG_2_62)?
                && c(cond::HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_NEG_2_09)?
        }
        (_, expr::N_FS_POP_SHUVIT_ANTICIPATION_90) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_2_09)?
                && c(cond::HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_2_62)?
        }
        (_, expr::N_360_POP_SHUVIT_ANTICIPATION_91) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_NEG_2_09)?
                && c(cond::HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_NEG_1_57)?
        }
        (_, expr::N_FS_360_POP_SHUVIT_ANTICIPATION_92) => {
            c(cond::HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_1_57)?
                && c(cond::HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_2_09)?
        }
        (_, expr::PUSHING_48) => {
            !c(cond::DISABLE_PUSH_BRAKE)?
                && (c(cond::HAS_AG_INTENT_RIGHT_PUSH)? || c(cond::HAS_AG_INTENT_LEFT_PUSH)?)
        }
        (_, expr::BRAKE_49) => c(cond::HAS_AG_INTENT_BRAKE)? && !c(cond::DISABLE_PUSH_BRAKE)?,
        (_, expr::HARD_TURN_93) => {
            c(cond::HAS_AG_INTENT_HARD_TURN_ABS_GREATER_0)? && !c(cond::HAS_AG_INTENT_SLIDING)?
        }
        (_, expr::HIPPY_JUMPING_47) => {
            c(cond::HAS_AG_INTENT_RIGHT_PUSH)?
                && c(cond::HAS_AG_INTENT_LEFT_PUSH)?
                && !c(cond::DISABLE_PUSH_BRAKE)?
        }
        (Mask::Pre, expr::KICKTURN_50) => {
            c(cond::PHYSICS_SPEED_AND_SLOPE_COMPARE_LESS_0_1)?
                && c(cond::HAS_AG_INTENT_KICK_TURN_ABS_GREATER_0_3)?
                && ((c(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_4)?
                    && c(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_4_6)?
                    && !c(cond::IS_RIDING_FAKIE)?)
                    || (c(cond::IS_RIDING_FAKIE)?
                        && c(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_1_5)?
                        && c(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_2_1)?))
        }
        (Mask::Sustain, expr::KICKTURN_50) => {
            c(cond::HAS_AG_INTENT_KICK_TURN_ABS_GREATER_0_3)?
                && ((c(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_4_6)? && !c(cond::IS_RIDING_FAKIE)?)
                    || (c(cond::IS_RIDING_FAKIE)? && c(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_2_1)?))
        }
        (Mask::Pre, expr::TELEPORT_OFF_BOARD_7_TO_ON_BOARD_6) => {
            c(cond::IN_PARENT_STATE_FOR_TIME_GREATER_0_5)?
        } //82900798: InParentStateForTime
        (Mask::Pre, expr::JUMPING_ONTO_BOARD_29_TO_ON_BOARD_GENERAL_1) => {
            !c(cond::IS_LANDING_ON_BOARD)?
        } //82900ab4: IsLandingOnBoard, negated
        (
            Mask::Sustain,
            expr::TELEPORT_OFF_BOARD_7_TO_ON_BOARD_6
            | expr::RUNNING_27_TO_DEFAULT_30
            | expr::JUMPING_ONTO_BOARD_29_TO_ON_BOARD_GENERAL_1,
        ) => true, //8290a90c: native Sustain switch defaults
        // 8290290c / 82907e8c: HasAGIntent, else not IsMirrored.
        (_, expr::REGULAR) => c(cond::HAS_AG_INTENT_DONT_MIRROR_TRICK)? || !c(cond::IS_MIRRORED)?,
        // 829033d0 / 82908584: any of three HasAGIntent reads, in native order.
        (_, expr::STALE_MUTE_FINGER_FLIP_SHUV) => {
            c(cond::HAS_AG_INTENT_BS_VARIAL)?
                || c(cond::HAS_AG_INTENT_FS_VARIAL)?
                || c(cond::HAS_AG_INTENT_FINGER_FLIP_SHUV)?
        }
        (Mask::Pre, _) => return Err(Error::PendingExpression(index)),
        (Mask::Sustain, _) => {
            return Err(Error::PendingMaskedExpression {
                mask,
                expression: index,
            });
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_right_grab_never_reads_left_and_antic_sustain_omits_threshold() {
        let mut calls = Vec::new();
        assert!(
            !expression(Mask::Pre, 57, |i| {
                calls.push(i);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(calls, [16]);
        calls.clear();
        assert!(
            expression(Mask::Sustain, 58, |i| {
                calls.push(i);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(calls, [77]);
    }
    #[test]
    fn kickturn_sustain_keeps_repeated_cached_fakie_query_and_omits_pre_only_speed() {
        let mut calls = Vec::new();
        assert!(
            expression(Mask::Sustain, 102, |i| {
                calls.push(i);
                Ok(matches!(i, 99 | 100 | 104))
            })
            .unwrap()
        );
        assert_eq!(calls, [99, 102, 100, 104]);
        calls.clear();
        assert!(
            expression(Mask::Sustain, 102, |i| {
                calls.push(i);
                Ok(true)
            })
            .unwrap()
        );
        assert_eq!(calls, [99, 102, 100, 100, 104]);
    }
    #[test]
    fn running_expression_short_circuits_raw_locomotion_after_filtered_state() {
        for mask in [Mask::Pre, Mask::Sustain] {
            let mut calls = Vec::new();
            assert!(
                !expression(mask, 73, |i| {
                    calls.push(i);
                    Ok(false)
                })
                .unwrap()
            );
            assert_eq!(calls, [134]);
            calls.clear();
            assert!(
                expression(mask, 73, |i| {
                    calls.push(i);
                    Ok(true)
                })
                .unwrap()
            );
            assert_eq!(calls, [134, 141]);
        }
    }
    #[test]
    fn transition_to_onboard_uses_distinct_pre_and_sustain_gates() {
        let mut calls = Vec::new();
        assert!(
            expression(Mask::Pre, 72, |i| {
                calls.push(i);
                Ok(matches!(i, 135 | 136))
            })
            .unwrap()
        );
        assert_eq!(calls, [134, 135, 70, 136, 127, 130]);
        calls.clear();
        assert!(
            expression(Mask::Sustain, 72, |i| {
                calls.push(i);
                Ok(matches!(i, 135 | 136))
            })
            .unwrap()
        );
        assert_eq!(calls, [134, 135, 136]);
        calls.clear();
        assert!(
            !expression(Mask::Pre, 72, |i| {
                calls.push(i);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(calls, [134, 135]);
    }
    #[test]
    fn offboard_air_default_does_not_query_dismount_completion_on_ground() {
        for mask in [Mask::Pre, Mask::Sustain] {
            let mut calls = Vec::new();
            assert!(
                !expression(mask, 75, |i| {
                    calls.push(i);
                    Ok(false)
                })
                .unwrap()
            );
            assert_eq!(calls, [135]);
            calls.clear();
            assert!(
                expression(mask, 75, |i| {
                    calls.push(i);
                    Ok(true)
                })
                .unwrap()
            );
            assert_eq!(calls, [135, 145]);
        }
    }
}
