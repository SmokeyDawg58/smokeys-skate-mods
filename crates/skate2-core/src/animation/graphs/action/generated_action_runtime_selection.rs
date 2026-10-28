//! Action expression callbacks over shared native TemplateHC selection.
use super::ActionUpdateError as Error;
use crate::generated_action_load::ids::{condition as cond, expression as expr};
#[cfg(test)]
use crate::generated_action_load::{NULL_INDEX, StateDeclaration, TransitionDeclaration};
pub(crate) use crate::generated_graph_selection::Mask;
pub(super) use crate::generated_graph_selection::Selected;
pub(super) type Graph<'a> = crate::generated_graph_selection::Graph<'a, 8>;

/// Independently read `EvaluateExpression<1>828ff990` and `<2>82906048`.
/// Only the audited expression domain is admitted; Sustain is not a reuse
/// of Pre: 27/28 become true and 29/31 test only NOT `DisablePushBrake96`.
pub(super) fn evaluate_expression(
    mask: Mask,
    expression: u16,
    mut c: impl FnMut(u16) -> Result<bool, Error>,
) -> Result<bool, Error> {
    Ok(match (mask, expression) {
        (Mask::Pre, expr::BOARD_ADJUST_GRAB_ONE_FOOT_RIGHT) => {
            (c(cond::HAS_AG_INTENT_RIGHT_PUSH)?
                && !c(cond::IS_MIRRORED_2ND)?
                && !c(cond::HAS_AG_INTENT_LEFT_PUSH)?)
                || (c(cond::HAS_AG_INTENT_LEFT_PUSH)?
                    && c(cond::IS_MIRRORED_2ND)?
                    && !c(cond::HAS_AG_INTENT_RIGHT_PUSH)?)
        } //829034f4
        (Mask::Sustain, expr::BOARD_ADJUST_GRAB_ONE_FOOT_RIGHT) => {
            (c(cond::HAS_AG_INTENT_RIGHT_PUSH)? && !c(cond::HAS_AG_INTENT_LEFT_PUSH)?)
                || (c(cond::HAS_AG_INTENT_LEFT_PUSH)? && !c(cond::HAS_AG_INTENT_RIGHT_PUSH)?)
        } //829086a8: no stance check
        (Mask::Pre, expr::BOARD_ADJUST_GRAB_ONE_FOOT_LEFT) => {
            (c(cond::HAS_AG_INTENT_LEFT_PUSH)?
                && !c(cond::IS_MIRRORED_2ND)?
                && !c(cond::HAS_AG_INTENT_RIGHT_PUSH)?)
                || (c(cond::HAS_AG_INTENT_RIGHT_PUSH)?
                    && c(cond::IS_MIRRORED_2ND)?
                    && !c(cond::HAS_AG_INTENT_LEFT_PUSH)?)
        } //8290371c
        (Mask::Sustain, expr::BOARD_ADJUST_GRAB_ONE_FOOT_LEFT) => {
            (c(cond::HAS_AG_INTENT_LEFT_PUSH)? && !c(cond::HAS_AG_INTENT_RIGHT_PUSH)?)
                || (c(cond::HAS_AG_INTENT_RIGHT_PUSH)? && !c(cond::HAS_AG_INTENT_LEFT_PUSH)?)
        } //82908820: no stance check
        (Mask::Pre | Mask::Sustain, expr::REGULAR) => {
            c(cond::HAS_AG_INTENT_DONT_MIRROR_TRICK)? || !c(cond::IS_MIRRORED)?
        } //8290290c /82907e8c: DontMirrorTrick, else NOT IsMirrored
        (Mask::Pre | Mask::Sustain, expr::STALE_MUTE_FINGER_FLIP_SHUV) => {
            c(cond::HAS_AG_INTENT_BS_VARIAL)?
                || c(cond::HAS_AG_INTENT_FS_VARIAL)?
                || c(cond::HAS_AG_INTENT_FINGER_FLIP_SHUV)?
        } //829033d0 /82908584: BS_Varial, FS_Varial, FingerFlipShuv
        // Tail/Nose grab NoFootAirWalk212/219 and IdleTailGrab214/IdleNoseGrab221.
        // Both masks read Dismount first (Pre82903944/82903a64, Sustain82908998/82908ab8).
        (_, expr::NO_FOOT_AIR_WALK) => {
            c(cond::HAS_AG_INTENT_DISMOUNT)?
                || (c(cond::HAS_AG_INTENT_RIGHT_PUSH)? && c(cond::HAS_AG_INTENT_LEFT_PUSH)?)
        }
        (_, expr::IDLE_TAIL_OR_NOSE_GRAB) => {
            !c(cond::HAS_AG_INTENT_DISMOUNT)?
                && !c(cond::HAS_AG_INTENT_LEFT_PUSH)?
                && !c(cond::HAS_AG_INTENT_RIGHT_PUSH)?
        }
        (_, expr::GRINDING_16) => c(cond::IS_GRINDING)?,
        (_, expr::NOT_IS_MIRRORED) => !c(cond::IS_MIRRORED)?, // Pre828ffce8 / Sustain8290628c: NOT IsMirrored
        (_, expr::IS_MIRRORED) => c(cond::IS_MIRRORED)?, // Pre828ffd5c / Sustain82906300: IsMirrored
        (Mask::Pre, expr::SUPERMAN_72) => {
            c(cond::HAS_AG_INTENT_NEW_DISMOUNT_DISMOUNT)?
                && c(cond::HAS_AG_INTENT_DISMOUNT)?
                && c(cond::PHYS_FILTERED_STATE_AIR)?
                && c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)?
                && c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)?
        } //82904b10
        (Mask::Sustain, expr::SUPERMAN_72) => {
            c(cond::HAS_AG_INTENT_DISMOUNT)?
                && c(cond::PHYS_FILTERED_STATE_AIR)?
                && c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)?
                && c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)?
        } //829098c8 omits entry intent40
        (_, expr::L_F_KICKFLIP_55) => c(cond::HAS_AG_INTENT_L_F_KICKFLIP)?, // Pre8290037c / Sustain8290665c
        (_, expr::L_F_HEELFLIP_56) => c(cond::HAS_AG_INTENT_L_F_HEELFLIP)?, // Pre829003dc / Sustain829066bc
        (_, expr::L_B_KICKFLIP_57) => c(cond::HAS_AG_INTENT_L_B_KICKFLIP)?, // Pre8290043c / Sustain8290671c
        (_, expr::L_B_HEELFLIP_58) => c(cond::HAS_AG_INTENT_L_B_HEELFLIP)?, // Pre8290049c / Sustain8290677c
        (_, expr::L_FS_SHUVIT_59) => c(cond::HAS_AG_INTENT_L_FS_SHUVIT)?, // Pre829004fc / Sustain829067dc
        (_, expr::L_BS_SHUVIT_60) => c(cond::HAS_AG_INTENT_L_BS_SHUVIT)?, // Pre8290055c / Sustain8290683c
        (_, expr::IN_AIR_17) => c(cond::PHYS_FILTERED_STATE_AIR)?,
        (_, expr::HAND_PLANT_37) => c(cond::IS_HAND_PLANTING_AIR)?,
        (_, expr::DISMOUNT) => c(cond::HAS_AG_INTENT_DISMOUNT)?, // Pre828ffe78 / Sustain8290641c: HasAGIntent(Dismount)
        (Mask::Pre, expr::NEW_ONE_FOOT) => c(cond::HAS_AG_INTENT_NEW_PUSH)?,
        (Mask::Pre, expr::FOOTPLANT) => c(cond::IS_FOOT_PLANTING)?, //828fff38: IsFootPlanting
        (Mask::Pre, expr::IDLE_151 | expr::IDLE_155) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG_2ND)?
        } //828fff98
        (Mask::Sustain, expr::IDLE_151 | expr::IDLE_155) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG_2ND)?
        } //8290647c, independently audited
        (Mask::Pre, expr::ENTER_NOT_MIRRORED) => !c(cond::IS_MIRRORED_2ND)?, //828ffff8: NOT IsMirrored
        (Mask::Pre, expr::ENTER_MIRRORED) => c(cond::IS_MIRRORED_2ND)?,      //82900070: IsMirrored
        (Mask::Sustain, expr::ENTER_NOT_MIRRORED | expr::ENTER_MIRRORED) => true, //8290a90c: unconditional return
        // NewOneFoot40 transitions80/81 target the inactive OneFootRight41/
        // OneFootLeft42; each mask reads its own cached push intent
        // (Pre829000d0/82900130, Sustain829064dc/8290653c).
        (_, expr::ONE_FOOT_RIGHT_41) => c(cond::HAS_AG_INTENT_RIGHT_PUSH)?,
        (_, expr::ONE_FOOT_LEFT_42) => c(cond::HAS_AG_INTENT_LEFT_PUSH)?,
        // Pre828ff990 transition predicates; transitions always use Pre.
        (Mask::Pre, expr::RUNNING_27_TO_DEFAULT_30) => !c(cond::IS_IN_LOCOMOTION)?,
        (Mask::Pre, expr::SWITCH_157_TO_STALE_MUTE_75) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_LESS_EQUAL_NEG_0_785)?
                && c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_GREATER_EQUAL_NEG_2_355)?
        }
        (Mask::Pre, expr::REGULAR_158_TO_STALE_MUTE_75) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_LESS_EQUAL_2_355)?
                && c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_GREATER_EQUAL_0_785)?
        }
        (Mask::Pre, expr::WIPEOUT_23) => c(cond::PHYS_FILTERED_STATE_WIPEOUT)?,
        (Mask::Pre, expr::TELEPORT_24) => c(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?,
        (
            Mask::Sustain,
            expr::NEW_ONE_FOOT
            | expr::FOOTPLANT
            | expr::WIPEOUT_23
            | expr::TELEPORT_24
            | expr::UP_76
            | expr::DOWN_77
            | expr::TRANS_TO_OFF_BOARD_AIR_44,
        ) => true,
        (Mask::Pre, expr::TELEPORT_OFF_BOARD_7) => {
            c(cond::PHYSICS_REQUESTS_DISMOUNT)?
                && !c(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?
                && !c(cond::DISABLE_PUSH_BRAKE)?
        }
        (Mask::Pre, expr::TRANS_TO_OFF_BOARD_8) => {
            c(cond::HAS_AG_INTENT_NEW_TOGGLE_OFF_BOARD_STATE)?
                && !c(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?
                && !c(cond::DISABLE_PUSH_BRAKE)?
                && !c(cond::IS_GRABBING_OBJECT)?
        }
        (Mask::Sustain, expr::TELEPORT_OFF_BOARD_7 | expr::TRANS_TO_OFF_BOARD_8) => {
            !c(cond::DISABLE_PUSH_BRAKE)?
        }
        (_, expr::DISMOUNT_32) => c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND)?, // Pre828ff990/Sustain82906048: PhysFilteredState(offboardair)
        // Default off-board state. Both native masks test filtered OffBoard
        // first; OffBoardAir only reads OBTimeToLand when it is true.
        (_, expr::DEFAULT_30) => {
            c(cond::PHYS_FILTERED_STATE_OFFBOARD)?
                || (c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND)?
                    && c(cond::OB_TIME_TO_LAND_LESS_EQUAL_0_03)?)
        }
        // State9 TransitioningToOffBoard. Native Pre case0x47 checks the
        // animation gates first, then physical exclusions; Sustain only the
        // two animation gates (828ff990 / 82906048).
        (Mask::Pre, expr::TRANSITIONING_TO_OFF_BOARD_9) => {
            (c(cond::IS_DISMOUNTING)? || c(cond::HAS_ANIM_ATTRIBUTE_IDLE_DISMOUNTING)?)
                && !c(cond::PHYS_FILTERED_STATE_AIR_2ND)?
                && !c(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?
                && !c(cond::DISABLE_PUSH_BRAKE_2ND)?
        }
        (Mask::Sustain, expr::TRANSITIONING_TO_OFF_BOARD_9) => {
            c(cond::IS_DISMOUNTING)? || c(cond::HAS_ANIM_ATTRIBUTE_IDLE_DISMOUNTING)?
        }
        (_, expr::FORCE_BRAKE_4) => {
            c(cond::HAS_AG_INTENT_FORCE_BRAKE)? && !c(cond::PHYS_FILTERED_STATE_TELEPORT)?
        }
        (_, expr::NO_INPUT_BRAKE_5) => {
            c(cond::TIME_SINCE_LAST_INPUT_GREATER_10)? && !c(cond::PHYS_FILTERED_STATE_TELEPORT)?
        }
        (_, expr::DBL_GRAB_71) => {
            c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? && c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)?
        }
        (Mask::Pre, expr::LEFT_OR_STALE_GRAB) => {
            (c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? && !c(cond::IS_MIRRORED_2ND)?)
                || (c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? && c(cond::IS_MIRRORED_2ND)?)
        }
        (Mask::Sustain, expr::LEFT_OR_STALE_GRAB) => {
            c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? || c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)?
        }
        // OneFootRight133/141 and OneFootLeft134/142 swap Right/LeftPush by
        // stance (Pre82901218/8290136c); Sustain82906ef4/82906fc0 drop it.
        (Mask::Pre, expr::GRAB_ONE_FOOT_RIGHT) => {
            (c(cond::HAS_AG_INTENT_RIGHT_PUSH)? && !c(cond::IS_MIRRORED_2ND)?)
                || (c(cond::HAS_AG_INTENT_LEFT_PUSH)? && c(cond::IS_MIRRORED_2ND)?)
        }
        (Mask::Sustain, expr::GRAB_ONE_FOOT_RIGHT) => {
            c(cond::HAS_AG_INTENT_RIGHT_PUSH)? || c(cond::HAS_AG_INTENT_LEFT_PUSH)?
        }
        (Mask::Pre, expr::GRAB_ONE_FOOT_LEFT) => {
            (c(cond::HAS_AG_INTENT_LEFT_PUSH)? && !c(cond::IS_MIRRORED_2ND)?)
                || (c(cond::HAS_AG_INTENT_RIGHT_PUSH)? && c(cond::IS_MIRRORED_2ND)?)
        }
        (Mask::Sustain, expr::GRAB_ONE_FOOT_LEFT) => {
            c(cond::HAS_AG_INTENT_LEFT_PUSH)? || c(cond::HAS_AG_INTENT_RIGHT_PUSH)?
        }
        (Mask::Pre, expr::RIGHT_OR_MUTE_GRAB) => {
            (c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? && !c(cond::IS_MIRRORED_2ND)?)
                || (c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? && c(cond::IS_MIRRORED_2ND)?)
        }
        (Mask::Sustain, expr::RIGHT_OR_MUTE_GRAB) => {
            c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? || c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)?
        }
        (Mask::Pre, expr::UP_76) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_ABS_GREATER_EQUAL_2_355)?
                || c(cond::CURRENT_STATE_DOWN_CRAIL_GRAB_FINGER_FLIP_SHUV)?
        }
        (Mask::Pre, expr::TAIL_GRAB_149) => {
            (c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG)?
                || c(cond::CURRENT_STATE_DOWN_CRAIL_GRAB_FINGER_FLIP_SHUV)?)
                && ((c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? && !c(cond::IS_MIRRORED_2ND)?)
                    || (c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? && c(cond::IS_MIRRORED_2ND)?))
        }
        (Mask::Sustain, expr::TAIL_GRAB_149) => {
            c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? || c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)?
        }
        (Mask::Pre, expr::DOWN_77) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_ABS_LESS_EQUAL_0_785)?
                || c(cond::CURRENT_STATE_UP_SEATBELT_GRAB_FINGER_FLIP_SHUV)?
        }
        (Mask::Pre, expr::NOSE_GRAB_152) => {
            (c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG)?
                || c(cond::CURRENT_STATE_UP_SEATBELT_GRAB_FINGER_FLIP_SHUV)?)
                && ((c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? && !c(cond::IS_MIRRORED_2ND)?)
                    || (c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? && c(cond::IS_MIRRORED_2ND)?))
        } //829018a4
        (Mask::Pre, expr::ROCKET_AIR_154) => {
            c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? && c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)?
        } //8290199c
        (Mask::Sustain, expr::NOSE_GRAB_152) => {
            c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? || c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)?
        } //829071ec: NoseGrab152 keeps only the grab pair
        (Mask::Sustain, expr::ROCKET_AIR_154) => {
            c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? && c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)?
        } //829072b8: RocketAir154, same pair as Pre
        (Mask::Pre, expr::ROCKET_AIR_154_TO_NOSE_GRAB_152) => {
            !(c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB_2ND)?
                && c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB_2ND)?)
        } //82901a68: NOT both RightAirGrab and LeftAirGrab
        (
            Mask::Sustain,
            expr::ROCKET_AIR_154_TO_NOSE_GRAB_152
            | expr::SWITCH_157_TO_STALE_MUTE_75
            | expr::REGULAR_158_TO_STALE_MUTE_75,
        ) => true, //82906080 table ->8290a90c: native default, no reads
        (Mask::Pre, expr::TRANS_TO_OFF_BOARD_AIR_44) => {
            c(cond::HAS_AG_INTENT_NEW_TOGGLE_OFF_BOARD_STATE)? && c(cond::TIME_TO_LAND_GREATER_0_3)?
        } //82901ccc
        (Mask::Pre, expr::GRAB_FINGER_FLIP_SHUV) => {
            c(cond::HAS_AG_INTENT_FINGER_FLIP_SHUV)?
                || (c(cond::HAS_AG_INTENT_FS_VARIAL)? && !c(cond::IS_MIRRORED_2ND)?)
                || (c(cond::IS_MIRRORED_2ND)? && c(cond::HAS_AG_INTENT_BS_VARIAL)?)
        } //82903030: FingerFlipShuv
        (Mask::Sustain, expr::GRAB_FINGER_FLIP_SHUV) => {
            c(cond::HAS_AG_INTENT_FINGER_FLIP_SHUV)?
                || c(cond::HAS_AG_INTENT_FS_VARIAL)?
                || c(cond::HAS_AG_INTENT_BS_VARIAL)?
        } //8290833c
        (Mask::Pre, expr::FINGER_FLIP_VAR) => {
            c(cond::HAS_AG_INTENT_FINGER_FLIP_VAR)?
                || (c(cond::HAS_AG_INTENT_BS_VARIAL)? && !c(cond::IS_MIRRORED_2ND)?)
                || (c(cond::IS_MIRRORED_2ND)? && c(cond::HAS_AG_INTENT_FS_VARIAL)?)
        } //82903200: FingerFlipVar
        (Mask::Sustain, expr::FINGER_FLIP_VAR) => {
            c(cond::HAS_AG_INTENT_FINGER_FLIP_VAR)?
                || c(cond::HAS_AG_INTENT_BS_VARIAL)?
                || c(cond::HAS_AG_INTENT_FS_VARIAL)?
        } //82908460
        (_, expr::ON_GROUND_19) => {
            c(cond::PHYS_FILTERED_STATE_GROUND)? && !c(cond::HAS_AG_INTENT_TOGGLE_OFF_BOARD_STATE)?
        }
        (Mask::Pre, expr::CRAIL_GRAB_153) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG)?
                && ((c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? && !c(cond::IS_MIRRORED_2ND)?)
                    || (c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? && c(cond::IS_MIRRORED_2ND)?))
        } //82903c2c
        (Mask::Pre, expr::SEATBELT_GRAB_150) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG)?
                && ((c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? && !c(cond::IS_MIRRORED_2ND)?)
                    || (c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? && c(cond::IS_MIRRORED_2ND)?))
        } //82903b88 ->82901514
        (Mask::Sustain, expr::CRAIL_GRAB_153) => {
            c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? || c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)?
        } //82908ca8: independent Sustain
        (Mask::Sustain, expr::SEATBELT_GRAB_150) => {
            c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)? || c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)?
        } //82908bdc
        (Mask::Pre, expr::LEFT_78) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG_2ND)?
                && c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_LESS_EQUAL_NEG_0_785)?
                && c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_GREATER_EQUAL_NEG_2_355)?
        }
        (Mask::Pre, expr::RIGHT_79) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG_2ND)?
                && c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_LESS_EQUAL_2_355)?
                && c(cond::HAS_AG_INTENT_BOARD_ADJUST_ANGLE_GREATER_EQUAL_0_785)?
        }
        (_, expr::COFFIN_18) => {
            c(cond::HAS_AG_INTENT_LEFT_PUSH)?
                && c(cond::HAS_AG_INTENT_RIGHT_PUSH)?
                && ((c(cond::PHYS_FILTERED_STATE_GROUND)?
                    && c(cond::HAS_AG_INTENT_LEFT_GROUND_GRAB)?
                    && c(cond::HAS_AG_INTENT_RIGHT_GROUND_GRAB)?)
                    || (c(cond::PHYS_FILTERED_STATE_AIR)?
                        && c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)?
                        && c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)?))
        } //Pre82904eb8 / Sustain82909c18: same ordered Coffin predicates
        (_, expr::FINGER_FLIP) => {
            c(cond::HAS_AG_INTENT_L_F_KICKFLIP)?
                || c(cond::HAS_AG_INTENT_L_F_HEELFLIP)?
                || c(cond::HAS_AG_INTENT_L_B_KICKFLIP)?
                || c(cond::HAS_AG_INTENT_L_B_HEELFLIP)?
                || c(cond::HAS_AG_INTENT_FINGER_FLIP)?
        } // Pre82904ce4 / Sustain82909a44: four flip intents, then FingerFlip
        (_, expr::BOARD_ADJUST_FINGER_FLIP_SHUV) => {
            c(cond::HAS_AG_INTENT_L_BS_SHUVIT)?
                || c(cond::HAS_AG_INTENT_BS_VARIAL)?
                || c(cond::HAS_AG_INTENT_FINGER_FLIP_SHUV)?
                || c(cond::HAS_AG_INTENT_L_FS_SHUVIT)?
                || c(cond::HAS_AG_INTENT_FS_VARIAL)?
                || c(cond::HAS_AG_INTENT_FINGER_FLIP_VAR)?
        } //Pre829057ec / Sustain8290a0bc
        (Mask::Sustain, expr::LEFT_78 | expr::RIGHT_79) => {
            c(cond::HAS_AG_INTENT_BOARD_ADJUST_MAG_2ND)?
        }
        // TransitioningToOnBoard11: IdleMounting, OB_Mounting, locomotion,
        // then NOT PhysFilteredState(teleport), lazily (Pre82904998).
        (Mask::Pre, expr::TRANSITIONING_TO_ON_BOARD_11) => {
            c(cond::HAS_ANIM_ATTRIBUTE_IDLE_MOUNTING_2ND)?
                || c(cond::HAS_ANIM_ATTRIBUTE_OB_MOUNTING)?
                || c(cond::IS_IN_LOCOMOTION)?
                || !c(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?
        }
        // Transition84 guard: neither filtered air nor offboard-air (Pre82900190).
        (Mask::Pre, expr::TRANS_TO_OFF_BOARD_AIR_44_TO_ON_BOARD_GENERAL_1) => {
            !c(cond::PHYS_FILTERED_STATE_AIR_2ND)? && !c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR)?
        }
        // Transition107 TransitioningToOnBoard11 -> OnBoard6 (Pre82900970):
        // NOT HasAnimAttribute(OB_Mounting), then NOT PhysFilteredState(offboard).
        (Mask::Pre, expr::TRANSITIONING_TO_ON_BOARD_11_TO_ON_BOARD_6) => {
            !c(cond::HAS_ANIM_ATTRIBUTE_OB_MOUNTING)? && !c(cond::PHYS_FILTERED_STATE_OFFBOARD_2ND)?
        }
        (
            Mask::Sustain,
            expr::TRANS_TO_OFF_BOARD_AIR_44_TO_ON_BOARD_GENERAL_1
            | expr::TRANSITIONING_TO_ON_BOARD_11_TO_ON_BOARD_6
            | expr::TRANSITIONING_TO_ON_BOARD_11,
        ) => true, //8290a90c: native Sustain default, no reads
        (Mask::Pre, expr::START_BRAKE_14) => c(cond::PHYSICS_SPEED_COMPARE_GREATER_5)?, //828ffb78: StartBrake14, PhysicsSpeedCompare(>5)
        (Mask::Pre, expr::START_BRAKE_14_TO_WAIT_BRAKE_15) => {
            c(cond::PHYSICS_SPEED_COMPARE_LESS_0_5)?
        } //828ffbd4: StartBrake -> WaitBrake, speed threshold
        (Mask::Pre, expr::WAIT_BRAKE_15_TO_ON_BOARD_6) => {
            c(cond::IN_PARENT_STATE_FOR_TIME_GREATER_10)?
        } //828ffc30: WaitBrake -> OnBoard, parent elapsed time
        (
            Mask::Sustain,
            expr::START_BRAKE_14
            | expr::START_BRAKE_14_TO_WAIT_BRAKE_15
            | expr::WAIT_BRAKE_15_TO_ON_BOARD_6,
        ) => true, //82906084 table ->8290a90c: no condition reads
        // Grinding16 / TrickOutOfTipSlashGrind22 children; both masks share
        // each case's reads (Pre / Sustain addresses per arm).
        (_, expr::DOUBLE_GRAB) => {
            c(cond::HAS_AG_INTENT_LEFT_GROUND_GRAB)? && c(cond::HAS_AG_INTENT_RIGHT_GROUND_GRAB)?
        } //82900f0c / 82906c94: DoubleGrab
        (_, expr::SIDEWAYS_64) => {
            c(cond::IS_GRINDING_BS_TAILSLIDE)?
                || c(cond::IS_GRINDING_BS_BLUNT)?
                || c(cond::IS_GRINDING_FS_TAILSLIDE)?
                || c(cond::IS_GRINDING_FS_BLUNT)?
        } //8290415c / 82908fb8
        (_, expr::SIDEWAYS_66) => {
            c(cond::IS_GRINDING_BS_NOSESLIDE)?
                || c(cond::IS_GRINDING_BS_NOSEBLUNT)?
                || c(cond::IS_GRINDING_FS_NOSESLIDE)?
                || c(cond::IS_GRINDING_FS_NOSEBLUNT)?
        } //829042cc / 82909128
        // TrickFrom90Tail/90Nose/N90Tail/N90Nose: the gesture group, else
        // HoldPattern and (InParentStateForTime(<0.4) or filtered air).
        (_, expr::TRICK_FROM_90_TAIL_107) => {
            c(cond::HAS_GESTURE_INTENT_90_TAIL)?
                || (c(cond::HAS_AG_INTENT_HOLD_PATTERN)?
                    && (c(cond::IN_PARENT_STATE_FOR_TIME_LESS_0_4)?
                        || c(cond::PHYS_FILTERED_STATE_AIR)?))
        } //82904708 / 82909564
        (_, expr::TRICK_FROM_90_NOSE_108) => {
            c(cond::HAS_GESTURE_INTENT_90_NOSE)?
                || (c(cond::HAS_AG_INTENT_HOLD_PATTERN)?
                    && (c(cond::IN_PARENT_STATE_FOR_TIME_LESS_0_4)?
                        || c(cond::PHYS_FILTERED_STATE_AIR)?))
        } //829047ac / 82909608
        (_, expr::TRICK_FROM_N_90_TAIL_109) => {
            c(cond::HAS_GESTURE_INTENT_N_90_TAIL)?
                || (c(cond::HAS_AG_INTENT_HOLD_PATTERN)?
                    && (c(cond::IN_PARENT_STATE_FOR_TIME_LESS_0_4)?
                        || c(cond::PHYS_FILTERED_STATE_AIR)?))
        } //82904850 / 829096ac
        (_, expr::TRICK_FROM_N_90_NOSE_110) => {
            c(cond::HAS_GESTURE_INTENT_N_90_NOSE)?
                || (c(cond::HAS_AG_INTENT_HOLD_PATTERN)?
                    && (c(cond::IN_PARENT_STATE_FOR_TIME_LESS_0_4)?
                        || c(cond::PHYS_FILTERED_STATE_AIR)?))
        } //829048f4 / 82909750
        // At9061 (Pre82905464): IsMirrored selects the backside or frontside
        // blunt/slide set (the second IsMirrored read is the cached one);
        // Sustain 8290a90c is the native default.
        (Mask::Pre, expr::AT_90_61) => {
            (c(cond::IS_MIRRORED_2ND)?
                && (c(cond::IS_GRINDING_BS_NOSESLIDE_2ND)?
                    || c(cond::IS_GRINDING_BS_BLUNT_2ND)?
                    || c(cond::IS_GRINDING_FS_TAILSLIDE_2ND)?
                    || c(cond::IS_GRINDING_FS_NOSEBLUNT_2ND)?))
                || (!c(cond::IS_MIRRORED_2ND)?
                    && (c(cond::IS_GRINDING_BS_TAILSLIDE_2ND)?
                        || c(cond::IS_GRINDING_BS_NOSEBLUNT_2ND)?
                        || c(cond::IS_GRINDING_FS_NOSESLIDE_2ND)?
                        || c(cond::IS_GRINDING_FS_BLUNT_2ND)?))
        }
        (Mask::Sustain, expr::AT_90_61) => true,
        // Straight63 (82905a18 / 8290a2e8) and Straight65 (82905cd8 / 8290a5a8).
        (_, expr::STRAIGHT_63) => {
            c(cond::IS_GRINDING_FS_5_O)?
                || c(cond::IS_GRINDING_BS_5_O)?
                || c(cond::IS_GRINDING_FS_SALAD)?
                || c(cond::IS_GRINDING_BS_SALAD)?
                || c(cond::IS_GRINDING_FS_SMITH)?
                || c(cond::IS_GRINDING_BS_SMITH)?
                || c(cond::IS_GRINDING_FS_FEEBLE)?
                || c(cond::IS_GRINDING_BS_FEEBLE)?
        }
        (_, expr::STRAIGHT_65) => {
            c(cond::IS_GRINDING_FS_NOSEGRIND)?
                || c(cond::IS_GRINDING_BS_NOSEGRIND)?
                || c(cond::IS_GRINDING_FS_OVERCROOK)?
                || c(cond::IS_GRINDING_BS_OVERCROOK)?
                || c(cond::IS_GRINDING_FS_CROOK)?
                || c(cond::IS_GRINDING_BS_CROOK)?
                || c(cond::IS_GRINDING_FS_OVERWILLY)?
                || c(cond::IS_GRINDING_BS_OVERWILLY)?
                || c(cond::IS_GRINDING_FS_WILLY)?
                || c(cond::IS_GRINDING_BS_WILLY)?
        }
        (_, expr::TRICK_FROM_SQUARE_52 | expr::TRICK_FROM_TAIL_53 | expr::TRICK_FROM_NOSE_54) => {
            return super::trick_selection::expression(mask, expression, c);
        }
        _ => return super::ground_selection::expression(mask, expression, c),
    })
}

#[cfg(test)]
#[path = "generated_action_runtime_selection_tests.rs"]
mod tests;
