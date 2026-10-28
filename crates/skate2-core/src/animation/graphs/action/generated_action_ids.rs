//! The Action state graph's ids under names from its own data, written by
//! `tools/generate_graph_ids.py`: conditions by class and attributes, expressions by the
//! states and transitions that evaluate them, states by name and index.
#![allow(dead_code, reason = "the complete graph; code names the ids it reads")]

/// Condition declarations: what each condition index tests.
pub(crate) mod condition {
    /// `HasAGIntent(intent = ForceBrake)`.
    pub(crate) const HAS_AG_INTENT_FORCE_BRAKE: u16 = 0;
    /// `PhysFilteredState(state = teleport)`.
    pub(crate) const PHYS_FILTERED_STATE_TELEPORT: u16 = 1;
    /// `TimeSinceLastInput(greater = 10.0)`.
    pub(crate) const TIME_SINCE_LAST_INPUT_GREATER_10: u16 = 2;
    /// `PhysicsSpeedCompare(greater = 5.0)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_GREATER_5: u16 = 3;
    /// `PhysicsSpeedCompare(less = 0.5)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_LESS_0_5: u16 = 4;
    /// `InParentStateForTime(greater = 10.0)`.
    pub(crate) const IN_PARENT_STATE_FOR_TIME_GREATER_10: u16 = 5;
    /// `IsGrinding`.
    pub(crate) const IS_GRINDING: u16 = 6;
    /// `IsGrinding(grindName = FS_5_O)`.
    pub(crate) const IS_GRINDING_FS_5_O: u16 = 7;
    /// `IsGrinding(grindName = BS_5_O)`.
    pub(crate) const IS_GRINDING_BS_5_O: u16 = 8;
    /// `IsGrinding(grindName = FS_SALAD)`.
    pub(crate) const IS_GRINDING_FS_SALAD: u16 = 9;
    /// `IsGrinding(grindName = BS_SALAD)`.
    pub(crate) const IS_GRINDING_BS_SALAD: u16 = 10;
    /// `IsGrinding(grindName = FS_SMITH)`.
    pub(crate) const IS_GRINDING_FS_SMITH: u16 = 11;
    /// `IsGrinding(grindName = BS_SMITH)`.
    pub(crate) const IS_GRINDING_BS_SMITH: u16 = 12;
    /// `IsGrinding(grindName = FS_FEEBLE)`.
    pub(crate) const IS_GRINDING_FS_FEEBLE: u16 = 13;
    /// `IsGrinding(grindName = BS_FEEBLE)`.
    pub(crate) const IS_GRINDING_BS_FEEBLE: u16 = 14;
    /// `HasAGIntent(intent = LeftGroundGrab)`.
    pub(crate) const HAS_AG_INTENT_LEFT_GROUND_GRAB: u16 = 15;
    /// `HasAGIntent(intent = RightGroundGrab)`.
    pub(crate) const HAS_AG_INTENT_RIGHT_GROUND_GRAB: u16 = 16;
    /// `IsMirrored`.
    pub(crate) const IS_MIRRORED: u16 = 17;
    /// `IsGrinding(grindName = BS_TAILSLIDE)`.
    pub(crate) const IS_GRINDING_BS_TAILSLIDE: u16 = 18;
    /// `IsGrinding(grindName = BS_BLUNT)`.
    pub(crate) const IS_GRINDING_BS_BLUNT: u16 = 19;
    /// `IsGrinding(grindName = FS_TAILSLIDE)`.
    pub(crate) const IS_GRINDING_FS_TAILSLIDE: u16 = 20;
    /// `IsGrinding(grindName = FS_BLUNT)`.
    pub(crate) const IS_GRINDING_FS_BLUNT: u16 = 21;
    /// `IsGrinding(grindName = FS_NOSEGRIND)`.
    pub(crate) const IS_GRINDING_FS_NOSEGRIND: u16 = 22;
    /// `IsGrinding(grindName = BS_NOSEGRIND)`.
    pub(crate) const IS_GRINDING_BS_NOSEGRIND: u16 = 23;
    /// `IsGrinding(grindName = FS_OVERCROOK)`.
    pub(crate) const IS_GRINDING_FS_OVERCROOK: u16 = 24;
    /// `IsGrinding(grindName = BS_OVERCROOK)`.
    pub(crate) const IS_GRINDING_BS_OVERCROOK: u16 = 25;
    /// `IsGrinding(grindName = FS_CROOK)`.
    pub(crate) const IS_GRINDING_FS_CROOK: u16 = 26;
    /// `IsGrinding(grindName = BS_CROOK)`.
    pub(crate) const IS_GRINDING_BS_CROOK: u16 = 27;
    /// `IsGrinding(grindName = FS_OVERWILLY)`.
    pub(crate) const IS_GRINDING_FS_OVERWILLY: u16 = 28;
    /// `IsGrinding(grindName = BS_OVERWILLY)`.
    pub(crate) const IS_GRINDING_BS_OVERWILLY: u16 = 29;
    /// `IsGrinding(grindName = FS_WILLY)`.
    pub(crate) const IS_GRINDING_FS_WILLY: u16 = 30;
    /// `IsGrinding(grindName = BS_WILLY)`.
    pub(crate) const IS_GRINDING_BS_WILLY: u16 = 31;
    /// `IsGrinding(grindName = BS_NOSESLIDE)`.
    pub(crate) const IS_GRINDING_BS_NOSESLIDE: u16 = 32;
    /// `IsGrinding(grindName = BS_NOSEBLUNT)`.
    pub(crate) const IS_GRINDING_BS_NOSEBLUNT: u16 = 33;
    /// `IsGrinding(grindName = FS_NOSESLIDE)`.
    pub(crate) const IS_GRINDING_FS_NOSESLIDE: u16 = 34;
    /// `IsGrinding(grindName = FS_NOSEBLUNT)`.
    pub(crate) const IS_GRINDING_FS_NOSEBLUNT: u16 = 35;
    /// `PhysFilteredState(state = air)`.
    pub(crate) const PHYS_FILTERED_STATE_AIR: u16 = 36;
    /// `IsHandPlanting(state = air)`.
    pub(crate) const IS_HAND_PLANTING_AIR: u16 = 37;
    /// `HasAGIntent(intent = LeftAirGrab)`.
    pub(crate) const HAS_AG_INTENT_LEFT_AIR_GRAB: u16 = 38;
    /// `HasAGIntent(intent = RightAirGrab)`.
    pub(crate) const HAS_AG_INTENT_RIGHT_AIR_GRAB: u16 = 39;
    /// `HasAGIntent(attribute = NewDismount, intent = Dismount)`.
    pub(crate) const HAS_AG_INTENT_NEW_DISMOUNT_DISMOUNT: u16 = 40;
    /// `HasAGIntent(intent = Dismount)`.
    pub(crate) const HAS_AG_INTENT_DISMOUNT: u16 = 41;
    /// `IsMirrored`, declared again: the test of `IS_MIRRORED` under its own index.
    pub(crate) const IS_MIRRORED_2ND: u16 = 42;
    /// `HasAGIntent(intent = NewPush)`.
    pub(crate) const HAS_AG_INTENT_NEW_PUSH: u16 = 43;
    /// `HasAGIntent(intent = RightPush)`.
    pub(crate) const HAS_AG_INTENT_RIGHT_PUSH: u16 = 44;
    /// `HasAGIntent(intent = LeftPush)`.
    pub(crate) const HAS_AG_INTENT_LEFT_PUSH: u16 = 45;
    /// `IsFootPlanting`.
    pub(crate) const IS_FOOT_PLANTING: u16 = 46;
    /// `HasAGIntent(intent = L_F_Kickflip)`.
    pub(crate) const HAS_AG_INTENT_L_F_KICKFLIP: u16 = 47;
    /// `HasAGIntent(intent = L_F_Heelflip)`.
    pub(crate) const HAS_AG_INTENT_L_F_HEELFLIP: u16 = 48;
    /// `HasAGIntent(intent = L_B_Kickflip)`.
    pub(crate) const HAS_AG_INTENT_L_B_KICKFLIP: u16 = 49;
    /// `HasAGIntent(intent = L_B_Heelflip)`.
    pub(crate) const HAS_AG_INTENT_L_B_HEELFLIP: u16 = 50;
    /// `HasAGIntent(intent = FingerFlip)`.
    pub(crate) const HAS_AG_INTENT_FINGER_FLIP: u16 = 51;
    /// `HasAGIntent(intent = FS_Varial)`.
    pub(crate) const HAS_AG_INTENT_FS_VARIAL: u16 = 52;
    /// `HasAGIntent(intent = BS_Varial)`.
    pub(crate) const HAS_AG_INTENT_BS_VARIAL: u16 = 53;
    /// `HasAGIntent(intent = FingerFlipShuv)`.
    pub(crate) const HAS_AG_INTENT_FINGER_FLIP_SHUV: u16 = 54;
    /// `HasAGIntent(intent = FingerFlipVar)`.
    pub(crate) const HAS_AG_INTENT_FINGER_FLIP_VAR: u16 = 55;
    /// `HasAGIntent(abs = true, greaterEqual = 2.355, intent = BoardAdjustAngle)`.
    pub(crate) const HAS_AG_INTENT_BOARD_ADJUST_ANGLE_ABS_GREATER_EQUAL_2_355: u16 = 56;
    /// `CurrentState(state = Down.CrailGrab.FingerFlipShuv)`.
    pub(crate) const CURRENT_STATE_DOWN_CRAIL_GRAB_FINGER_FLIP_SHUV: u16 = 57;
    /// `HasAGIntent(intent = BoardAdjustMag)`.
    pub(crate) const HAS_AG_INTENT_BOARD_ADJUST_MAG: u16 = 58;
    /// `HasAGIntent(intent = L_BS_Shuvit)`.
    pub(crate) const HAS_AG_INTENT_L_BS_SHUVIT: u16 = 59;
    /// `HasAGIntent(intent = L_FS_Shuvit)`.
    pub(crate) const HAS_AG_INTENT_L_FS_SHUVIT: u16 = 60;
    /// `HasAGIntent(intent = BoardAdjustMag)`, declared again: the test of
    /// `HAS_AG_INTENT_BOARD_ADJUST_MAG` under its own index.
    pub(crate) const HAS_AG_INTENT_BOARD_ADJUST_MAG_2ND: u16 = 61;
    /// `HasAGIntent(abs = true, intent = BoardAdjustAngle, lessEqual = 0.785)`.
    pub(crate) const HAS_AG_INTENT_BOARD_ADJUST_ANGLE_ABS_LESS_EQUAL_0_785: u16 = 62;
    /// `CurrentState(state = Up.SeatbeltGrab.FingerFlipShuv)`.
    pub(crate) const CURRENT_STATE_UP_SEATBELT_GRAB_FINGER_FLIP_SHUV: u16 = 63;
    /// `HasAGIntent(intent = RightAirGrab)`, declared again: the test of
    /// `HAS_AG_INTENT_RIGHT_AIR_GRAB` under its own index.
    pub(crate) const HAS_AG_INTENT_RIGHT_AIR_GRAB_2ND: u16 = 64;
    /// `HasAGIntent(intent = LeftAirGrab)`, declared again: the test of
    /// `HAS_AG_INTENT_LEFT_AIR_GRAB` under its own index.
    pub(crate) const HAS_AG_INTENT_LEFT_AIR_GRAB_2ND: u16 = 65;
    /// `HasAGIntent(intent = BoardAdjustAngle, lessEqual = -0.785)`.
    pub(crate) const HAS_AG_INTENT_BOARD_ADJUST_ANGLE_LESS_EQUAL_NEG_0_785: u16 = 66;
    /// `HasAGIntent(greaterEqual = -2.355, intent = BoardAdjustAngle)`.
    pub(crate) const HAS_AG_INTENT_BOARD_ADJUST_ANGLE_GREATER_EQUAL_NEG_2_355: u16 = 67;
    /// `HasAGIntent(intent = BoardAdjustAngle, lessEqual = 2.355)`.
    pub(crate) const HAS_AG_INTENT_BOARD_ADJUST_ANGLE_LESS_EQUAL_2_355: u16 = 68;
    /// `HasAGIntent(greaterEqual = 0.785, intent = BoardAdjustAngle)`.
    pub(crate) const HAS_AG_INTENT_BOARD_ADJUST_ANGLE_GREATER_EQUAL_0_785: u16 = 69;
    /// `HasAGIntent(intent = NewToggleOffBoardState)`.
    pub(crate) const HAS_AG_INTENT_NEW_TOGGLE_OFF_BOARD_STATE: u16 = 70;
    /// `TimeToLand(greater = 0.3)`.
    pub(crate) const TIME_TO_LAND_GREATER_0_3: u16 = 71;
    /// `PhysFilteredState(state = air)`, declared again: the test of `PHYS_FILTERED_STATE_AIR`
    /// under its own index.
    pub(crate) const PHYS_FILTERED_STATE_AIR_2ND: u16 = 72;
    /// `PhysFilteredState(state = offboardair)`.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD_AIR: u16 = 73;
    /// `PhysFilteredState(state = ground)`.
    pub(crate) const PHYS_FILTERED_STATE_GROUND: u16 = 74;
    /// `HasAGIntent(intent = ToggleOffBoardState)`.
    pub(crate) const HAS_AG_INTENT_TOGGLE_OFF_BOARD_STATE: u16 = 75;
    /// `HasAGIntent(greater = 0.9, intent = AnticMag)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_MAG_GREATER_0_9: u16 = 76;
    /// `HasAGIntent(intent = AnticMag)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_MAG: u16 = 77;
    /// `HasAGIntent(abs = true, intent = AnticAngle, lessEqual = 0.52)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_ABS_LESS_EQUAL_0_52: u16 = 78;
    /// `HasAGIntent(greaterEqual = -1.05, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_NEG_1_05: u16 = 79;
    /// `HasAGIntent(intent = AnticAngle, lessEqual = -0.52)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_NEG_0_52: u16 = 80;
    /// `HasAGIntent(greaterEqual = 0.52, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_0_52: u16 = 81;
    /// `HasAGIntent(intent = AnticAngle, lessEqual = 1.05)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_1_05: u16 = 82;
    /// `HasAGIntent(greaterEqual = -1.57, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_NEG_1_57: u16 = 83;
    /// `HasAGIntent(intent = AnticAngle, lessEqual = -1.05)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_NEG_1_05: u16 = 84;
    /// `HasAGIntent(greaterEqual = 1.05, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_1_05: u16 = 85;
    /// `HasAGIntent(intent = AnticAngle, lessEqual = 1.57)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_1_57: u16 = 86;
    /// `HasAGIntent(abs = true, greaterEqual = 2.62, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_ABS_GREATER_EQUAL_2_62: u16 = 87;
    /// `HasAGIntent(greaterEqual = -2.62, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_NEG_2_62: u16 = 88;
    /// `HasAGIntent(intent = AnticAngle, lessEqual = -2.09)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_NEG_2_09: u16 = 89;
    /// `HasAGIntent(greaterEqual = 2.09, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_2_09: u16 = 90;
    /// `HasAGIntent(intent = AnticAngle, lessEqual = 2.62)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_2_62: u16 = 91;
    /// `HasAGIntent(greaterEqual = -2.09, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_NEG_2_09: u16 = 92;
    /// `HasAGIntent(intent = AnticAngle, lessEqual = -1.57)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_NEG_1_57: u16 = 93;
    /// `HasAGIntent(greaterEqual = 1.57, intent = AnticAngle)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_GREATER_EQUAL_1_57: u16 = 94;
    /// `HasAGIntent(intent = AnticAngle, lessEqual = 2.09)`.
    pub(crate) const HAS_AG_INTENT_ANTIC_ANGLE_LESS_EQUAL_2_09: u16 = 95;
    /// `DisablePushBrake`.
    pub(crate) const DISABLE_PUSH_BRAKE: u16 = 96;
    /// `HasAGIntent(intent = Brake)`.
    pub(crate) const HAS_AG_INTENT_BRAKE: u16 = 97;
    /// `PhysicsSpeedAndSlopeCompare(less = 0.1)`.
    pub(crate) const PHYSICS_SPEED_AND_SLOPE_COMPARE_LESS_0_1: u16 = 98;
    /// `HasAGIntent(abs = true, greater = 0.3, intent = KickTurn)`.
    pub(crate) const HAS_AG_INTENT_KICK_TURN_ABS_GREATER_0_3: u16 = 99;
    /// `IsRidingFakie`.
    pub(crate) const IS_RIDING_FAKIE: u16 = 100;
    /// `PhysicsSpeedCompare(abs = true, less = 4.0)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ABS_LESS_4: u16 = 101;
    /// `PhysicsSpeedCompare(abs = true, less = 4.6)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ABS_LESS_4_6: u16 = 102;
    /// `PhysicsSpeedCompare(abs = true, less = 1.5)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ABS_LESS_1_5: u16 = 103;
    /// `PhysicsSpeedCompare(abs = true, less = 2.1)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ABS_LESS_2_1: u16 = 104;
    /// `HasAGIntent(abs = true, greater = 0.0, intent = HardTurn)`.
    pub(crate) const HAS_AG_INTENT_HARD_TURN_ABS_GREATER_0: u16 = 105;
    /// `HasAGIntent(intent = Sliding)`.
    pub(crate) const HAS_AG_INTENT_SLIDING: u16 = 106;
    /// `HasAGIntent(intent = Trick)`.
    pub(crate) const HAS_AG_INTENT_TRICK: u16 = 107;
    /// `HasGestureIntent(group = Square)`.
    pub(crate) const HAS_GESTURE_INTENT_SQUARE: u16 = 108;
    /// `HasAGIntent(intent = HoldPattern)`.
    pub(crate) const HAS_AG_INTENT_HOLD_PATTERN: u16 = 109;
    /// `InParentStateForTime(less = 0.4)`.
    pub(crate) const IN_PARENT_STATE_FOR_TIME_LESS_0_4: u16 = 110;
    /// `HasGestureIntent(group = Tail)`.
    pub(crate) const HAS_GESTURE_INTENT_TAIL: u16 = 111;
    /// `HasGestureIntent(group = Nose)`.
    pub(crate) const HAS_GESTURE_INTENT_NOSE: u16 = 112;
    /// `HasAGIntent(intent = DontMirrorTrick)`.
    pub(crate) const HAS_AG_INTENT_DONT_MIRROR_TRICK: u16 = 113;
    /// `IsGrinding(grindName = BS_TAILSLIDE)`, declared again: the test of
    /// `IS_GRINDING_BS_TAILSLIDE` under its own index.
    pub(crate) const IS_GRINDING_BS_TAILSLIDE_2ND: u16 = 114;
    /// `IsGrinding(grindName = BS_NOSEBLUNT)`, declared again: the test of
    /// `IS_GRINDING_BS_NOSEBLUNT` under its own index.
    pub(crate) const IS_GRINDING_BS_NOSEBLUNT_2ND: u16 = 115;
    /// `IsGrinding(grindName = FS_NOSESLIDE)`, declared again: the test of
    /// `IS_GRINDING_FS_NOSESLIDE` under its own index.
    pub(crate) const IS_GRINDING_FS_NOSESLIDE_2ND: u16 = 116;
    /// `IsGrinding(grindName = FS_BLUNT)`, declared again: the test of `IS_GRINDING_FS_BLUNT` under
    /// its own index.
    pub(crate) const IS_GRINDING_FS_BLUNT_2ND: u16 = 117;
    /// `IsGrinding(grindName = BS_NOSESLIDE)`, declared again: the test of
    /// `IS_GRINDING_BS_NOSESLIDE` under its own index.
    pub(crate) const IS_GRINDING_BS_NOSESLIDE_2ND: u16 = 118;
    /// `IsGrinding(grindName = BS_BLUNT)`, declared again: the test of `IS_GRINDING_BS_BLUNT` under
    /// its own index.
    pub(crate) const IS_GRINDING_BS_BLUNT_2ND: u16 = 119;
    /// `IsGrinding(grindName = FS_TAILSLIDE)`, declared again: the test of
    /// `IS_GRINDING_FS_TAILSLIDE` under its own index.
    pub(crate) const IS_GRINDING_FS_TAILSLIDE_2ND: u16 = 120;
    /// `IsGrinding(grindName = FS_NOSEBLUNT)`, declared again: the test of
    /// `IS_GRINDING_FS_NOSEBLUNT` under its own index.
    pub(crate) const IS_GRINDING_FS_NOSEBLUNT_2ND: u16 = 121;
    /// `HasGestureIntent(group = 90Tail)`.
    pub(crate) const HAS_GESTURE_INTENT_90_TAIL: u16 = 122;
    /// `HasGestureIntent(group = 90Nose)`.
    pub(crate) const HAS_GESTURE_INTENT_90_NOSE: u16 = 123;
    /// `HasGestureIntent(group = N90Tail)`.
    pub(crate) const HAS_GESTURE_INTENT_N_90_TAIL: u16 = 124;
    /// `HasGestureIntent(group = N90Nose)`.
    pub(crate) const HAS_GESTURE_INTENT_N_90_NOSE: u16 = 125;
    /// `PhysFilteredState(state = wipeout)`.
    pub(crate) const PHYS_FILTERED_STATE_WIPEOUT: u16 = 126;
    /// `PhysFilteredState(state = teleport)`, declared again: the test of
    /// `PHYS_FILTERED_STATE_TELEPORT` under its own index.
    pub(crate) const PHYS_FILTERED_STATE_TELEPORT_2ND: u16 = 127;
    /// `PhysicsRequestsDismount`.
    pub(crate) const PHYSICS_REQUESTS_DISMOUNT: u16 = 128;
    /// `InParentStateForTime(greater = 0.5)`.
    pub(crate) const IN_PARENT_STATE_FOR_TIME_GREATER_0_5: u16 = 129;
    /// `IsGrabbingObject`.
    pub(crate) const IS_GRABBING_OBJECT: u16 = 130;
    /// `IsDismounting`.
    pub(crate) const IS_DISMOUNTING: u16 = 131;
    /// `HasAnimAttribute(attribute = IdleDismounting)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_IDLE_DISMOUNTING: u16 = 132;
    /// `DisablePushBrake`, declared again: the test of `DISABLE_PUSH_BRAKE` under its own index.
    pub(crate) const DISABLE_PUSH_BRAKE_2ND: u16 = 133;
    /// `PhysFilteredState(state = offboard)`.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD: u16 = 134;
    /// `PhysFilteredState(state = offboardair)`, declared again: the test of
    /// `PHYS_FILTERED_STATE_OFFBOARD_AIR` under its own index.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND: u16 = 135;
    /// `HasAnimAttribute(attribute = IdleMounting)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_IDLE_MOUNTING: u16 = 136;
    /// `HasAnimAttribute(attribute = IdleMounting)`, declared again: the test of
    /// `HAS_ANIM_ATTRIBUTE_IDLE_MOUNTING` under its own index.
    pub(crate) const HAS_ANIM_ATTRIBUTE_IDLE_MOUNTING_2ND: u16 = 137;
    /// `HasAnimAttribute(attribute = OB_Mounting)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_OB_MOUNTING: u16 = 138;
    /// `IsInLocomotion`.
    pub(crate) const IS_IN_LOCOMOTION: u16 = 139;
    /// `PhysFilteredState(state = offboard)`, declared again: the test of
    /// `PHYS_FILTERED_STATE_OFFBOARD` under its own index.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD_2ND: u16 = 140;
    /// `IsInLocomotion`, declared again: the test of `IS_IN_LOCOMOTION` under its own index.
    pub(crate) const IS_IN_LOCOMOTION_2ND: u16 = 141;
    /// `IsLandingOnBoard`.
    pub(crate) const IS_LANDING_ON_BOARD: u16 = 142;
    /// `OBTimeToLand(lessEqual = 0.03)`.
    pub(crate) const OB_TIME_TO_LAND_LESS_EQUAL_0_03: u16 = 143;
    /// `PhysicsWantsRunout`.
    pub(crate) const PHYSICS_WANTS_RUNOUT: u16 = 144;
    /// `IsDoneAirDismount`.
    pub(crate) const IS_DONE_AIR_DISMOUNT: u16 = 145;
}

/// Expressions: which states and transitions evaluate each expression index.
pub(crate) mod expression {
    /// State `StartBrake14`.
    pub(crate) const START_BRAKE_14: u16 = 0;
    /// `StartBrake14` -> `WaitBrake15`.
    pub(crate) const START_BRAKE_14_TO_WAIT_BRAKE_15: u16 = 1;
    /// `WaitBrake15` -> `OnBoard6`.
    pub(crate) const WAIT_BRAKE_15_TO_ON_BOARD_6: u16 = 2;
    /// State `Grinding16`.
    pub(crate) const GRINDING_16: u16 = 3;
    /// Shared by 12: state `Regular127`, state `Regular129`, state `Goofy160`, state `Goofy162` and
    /// 8 more.
    pub(crate) const NOT_IS_MIRRORED: u16 = 4;
    /// Shared by 18: state `Switch96`, state `Switch98`, state `Switch100`, state `Switch102` and
    /// 14 more.
    pub(crate) const IS_MIRRORED: u16 = 5;
    /// State `InAir17`.
    pub(crate) const IN_AIR_17: u16 = 6;
    /// State `HandPlant37`.
    pub(crate) const HAND_PLANT_37: u16 = 7;
    /// Shared by 2: state `Dismount131`, state `Dismount139`.
    pub(crate) const DISMOUNT: u16 = 8;
    /// Shared by 3: state `NewOneFoot40`, state `NewOneFoot132`, state `NewOneFoot140`.
    pub(crate) const NEW_ONE_FOOT: u16 = 9;
    /// Shared by 4: state `Footplant196`, state `Footplant198`, state `Footplant200`, state
    /// `Footplant202`.
    pub(crate) const FOOTPLANT: u16 = 10;
    /// State `Idle151`.
    pub(crate) const IDLE_151: u16 = 11;
    /// State `Idle155`.
    pub(crate) const IDLE_155: u16 = 12;
    /// Shared by 11: state `UnMirrored26`, state `Regular156`, state `Regular158`, state
    /// `Regular164` and 7 more.
    pub(crate) const ENTER_NOT_MIRRORED: u16 = 13;
    /// Shared by 11: state `Mirrored25`, state `Switch157`, state `Switch159`, state `Switch165`
    /// and 7 more.
    pub(crate) const ENTER_MIRRORED: u16 = 14;
    /// State `OneFootRight41`.
    pub(crate) const ONE_FOOT_RIGHT_41: u16 = 15;
    /// State `OneFootLeft42`.
    pub(crate) const ONE_FOOT_LEFT_42: u16 = 16;
    /// `TransToOffBoardAir44` -> `OnBoardGeneral1`.
    pub(crate) const TRANS_TO_OFF_BOARD_AIR_44_TO_ON_BOARD_GENERAL_1: u16 = 17;
    /// State `OllieAnticipation83`.
    pub(crate) const OLLIE_ANTICIPATION_83: u16 = 18;
    /// State `NollieAnticipation88`.
    pub(crate) const NOLLIE_ANTICIPATION_88: u16 = 19;
    /// Shared by 3: state `Trick20`, state `LateFlips21`, state `TrickOutOfTipSlashGrind22`.
    pub(crate) const HAS_AG_INTENT_TRICK: u16 = 20;
    /// State `L_F_Kickflip55`.
    pub(crate) const L_F_KICKFLIP_55: u16 = 21;
    /// State `L_F_Heelflip56`.
    pub(crate) const L_F_HEELFLIP_56: u16 = 22;
    /// State `L_B_Kickflip57`.
    pub(crate) const L_B_KICKFLIP_57: u16 = 23;
    /// State `L_B_Heelflip58`.
    pub(crate) const L_B_HEELFLIP_58: u16 = 24;
    /// State `L_FS_Shuvit59`.
    pub(crate) const L_FS_SHUVIT_59: u16 = 25;
    /// State `L_BS_Shuvit60`.
    pub(crate) const L_BS_SHUVIT_60: u16 = 26;
    /// State `Wipeout23`.
    pub(crate) const WIPEOUT_23: u16 = 27;
    /// State `Teleport24`.
    pub(crate) const TELEPORT_24: u16 = 28;
    /// State `TeleportOffBoard7`.
    pub(crate) const TELEPORT_OFF_BOARD_7: u16 = 29;
    /// `TeleportOffBoard7` -> `OnBoard6`.
    pub(crate) const TELEPORT_OFF_BOARD_7_TO_ON_BOARD_6: u16 = 30;
    /// State `TransToOffBoard8`.
    pub(crate) const TRANS_TO_OFF_BOARD_8: u16 = 31;
    /// `TransitioningToOnBoard11` -> `OnBoard6`.
    pub(crate) const TRANSITIONING_TO_ON_BOARD_11_TO_ON_BOARD_6: u16 = 32;
    /// `Running27` -> `Default30`.
    pub(crate) const RUNNING_27_TO_DEFAULT_30: u16 = 33;
    /// `JumpingOntoBoard29` -> `OnBoardGeneral1`.
    pub(crate) const JUMPING_ONTO_BOARD_29_TO_ON_BOARD_GENERAL_1: u16 = 34;
    /// State `AccidentalDismount31`.
    pub(crate) const ACCIDENTAL_DISMOUNT_31: u16 = 35;
    /// State `Dismount32`.
    pub(crate) const DISMOUNT_32: u16 = 36;
    /// State `ForceBrake4`.
    pub(crate) const FORCE_BRAKE_4: u16 = 37;
    /// State `NoInputBrake5`.
    pub(crate) const NO_INPUT_BRAKE_5: u16 = 38;
    /// Shared by 6: state `LeftGrab67`, state `LeftGrab81`, state `LeftGrab111`, state
    /// `LeftGrab115` and 2 more.
    pub(crate) const LEFT_GRAB: u16 = 39;
    /// Shared by 6: state `RightGrab68`, state `RightGrab82`, state `RightGrab112`, state
    /// `RightGrab116` and 2 more.
    pub(crate) const RIGHT_GRAB: u16 = 40;
    /// Shared by 5: state `DoubleGrab69`, state `DoubleGrab113`, state `DoubleGrab117`, state
    /// `DoubleGrab121` and 1 more.
    pub(crate) const DOUBLE_GRAB: u16 = 41;
    /// State `DBLGrab71`.
    pub(crate) const DBL_GRAB_71: u16 = 42;
    /// Shared by 2: state `LeftGrab73`, state `StaleGrab148`.
    pub(crate) const LEFT_OR_STALE_GRAB: u16 = 43;
    /// Shared by 2: state `OneFootRight133`, state `OneFootRight141`.
    pub(crate) const GRAB_ONE_FOOT_RIGHT: u16 = 44;
    /// Shared by 2: state `OneFootLeft134`, state `OneFootLeft142`.
    pub(crate) const GRAB_ONE_FOOT_LEFT: u16 = 45;
    /// Shared by 2: state `RightGrab74`, state `MuteGrab147`.
    pub(crate) const RIGHT_OR_MUTE_GRAB: u16 = 46;
    /// State `Up76`.
    pub(crate) const UP_76: u16 = 47;
    /// State `TailGrab149`.
    pub(crate) const TAIL_GRAB_149: u16 = 48;
    /// State `Down77`.
    pub(crate) const DOWN_77: u16 = 49;
    /// State `NoseGrab152`.
    pub(crate) const NOSE_GRAB_152: u16 = 50;
    /// State `RocketAir154`.
    pub(crate) const ROCKET_AIR_154: u16 = 51;
    /// `RocketAir154` -> `NoseGrab152`.
    pub(crate) const ROCKET_AIR_154_TO_NOSE_GRAB_152: u16 = 52;
    /// `Switch157` -> `StaleMute75`.
    pub(crate) const SWITCH_157_TO_STALE_MUTE_75: u16 = 53;
    /// `Regular158` -> `StaleMute75`.
    pub(crate) const REGULAR_158_TO_STALE_MUTE_75: u16 = 54;
    /// State `TransToOffBoardAir44`.
    pub(crate) const TRANS_TO_OFF_BOARD_AIR_44: u16 = 55;
    /// State `OnGround19`.
    pub(crate) const ON_GROUND_19: u16 = 56;
    /// State `DBLGrab80`.
    pub(crate) const DBL_GRAB_80: u16 = 57;
    /// State `Antic46`.
    pub(crate) const ANTIC_46: u16 = 58;
    /// State `PopShuvitAnticipation84`.
    pub(crate) const POP_SHUVIT_ANTICIPATION_84: u16 = 59;
    /// State `FSPopShuvitAnticipation85`.
    pub(crate) const FS_POP_SHUVIT_ANTICIPATION_85: u16 = 60;
    /// State `360PopShuvitAnticipation86`.
    pub(crate) const STATE_360_POP_SHUVIT_ANTICIPATION_86: u16 = 61;
    /// State `FS360PopShuvitAnticipation87`.
    pub(crate) const FS_360_POP_SHUVIT_ANTICIPATION_87: u16 = 62;
    /// State `N_PopShuvitAnticipation89`.
    pub(crate) const N_POP_SHUVIT_ANTICIPATION_89: u16 = 63;
    /// State `N_FSPopShuvitAnticipation90`.
    pub(crate) const N_FS_POP_SHUVIT_ANTICIPATION_90: u16 = 64;
    /// State `N_360PopShuvitAnticipation91`.
    pub(crate) const N_360_POP_SHUVIT_ANTICIPATION_91: u16 = 65;
    /// State `N_FS360PopShuvitAnticipation92`.
    pub(crate) const N_FS_360_POP_SHUVIT_ANTICIPATION_92: u16 = 66;
    /// State `Pushing48`.
    pub(crate) const PUSHING_48: u16 = 67;
    /// State `Brake49`.
    pub(crate) const BRAKE_49: u16 = 68;
    /// State `HardTurn93`.
    pub(crate) const HARD_TURN_93: u16 = 69;
    /// Shared by 6: state `Regular95`, state `Regular97`, state `Regular99`, state `Regular101` and
    /// 2 more.
    pub(crate) const REGULAR: u16 = 70;
    /// State `TransitioningToOffBoard9`.
    pub(crate) const TRANSITIONING_TO_OFF_BOARD_9: u16 = 71;
    /// State `TransToOnBoard10`.
    pub(crate) const TRANS_TO_ON_BOARD_10: u16 = 72;
    /// State `Running27`.
    pub(crate) const RUNNING_27: u16 = 73;
    /// Shared by 2: state `HippyJump28`, state `JumpingOntoBoard29`.
    pub(crate) const IS_LANDING_ON_BOARD_OFFBOARD: u16 = 74;
    /// State `Default33`.
    pub(crate) const DEFAULT_33: u16 = 75;
    /// Shared by 2: state `FingerFlipShuv136`, state `FingerFlipShuv144`.
    pub(crate) const GRAB_FINGER_FLIP_SHUV: u16 = 76;
    /// Shared by 2: state `FingerFlipVar137`, state `FingerFlipVar145`.
    pub(crate) const FINGER_FLIP_VAR: u16 = 77;
    /// Shared by 2: state `FingerFlipShuv205`, state `FingerFlipShuv208`.
    pub(crate) const STALE_MUTE_FINGER_FLIP_SHUV: u16 = 78;
    /// Shared by 2: state `OneFootRight210`, state `OneFootRight217`.
    pub(crate) const BOARD_ADJUST_GRAB_ONE_FOOT_RIGHT: u16 = 79;
    /// Shared by 2: state `OneFootLeft211`, state `OneFootLeft218`.
    pub(crate) const BOARD_ADJUST_GRAB_ONE_FOOT_LEFT: u16 = 80;
    /// Shared by 2: state `NoFootAirWalk212`, state `NoFootAirWalk219`.
    pub(crate) const NO_FOOT_AIR_WALK: u16 = 81;
    /// Shared by 2: state `IdleTailGrab214`, state `IdleNoseGrab221`.
    pub(crate) const IDLE_TAIL_OR_NOSE_GRAB: u16 = 82;
    /// State `SeatbeltGrab150`.
    pub(crate) const SEATBELT_GRAB_150: u16 = 83;
    /// State `CrailGrab153`.
    pub(crate) const CRAIL_GRAB_153: u16 = 84;
    /// State `Left78`.
    pub(crate) const LEFT_78: u16 = 85;
    /// State `Right79`.
    pub(crate) const RIGHT_79: u16 = 86;
    /// State `HippyJumping47`.
    pub(crate) const HIPPY_JUMPING_47: u16 = 87;
    /// State `Default30`.
    pub(crate) const DEFAULT_30: u16 = 88;
    /// State `Sideways64`.
    pub(crate) const SIDEWAYS_64: u16 = 89;
    /// State `Sideways66`.
    pub(crate) const SIDEWAYS_66: u16 = 90;
    /// State `TrickFromSquare52`.
    pub(crate) const TRICK_FROM_SQUARE_52: u16 = 91;
    /// State `TrickFromTail53`.
    pub(crate) const TRICK_FROM_TAIL_53: u16 = 92;
    /// State `TrickFromNose54`.
    pub(crate) const TRICK_FROM_NOSE_54: u16 = 93;
    /// State `TrickFrom90Tail107`.
    pub(crate) const TRICK_FROM_90_TAIL_107: u16 = 94;
    /// State `TrickFrom90Nose108`.
    pub(crate) const TRICK_FROM_90_NOSE_108: u16 = 95;
    /// State `TrickFromN90Tail109`.
    pub(crate) const TRICK_FROM_N_90_TAIL_109: u16 = 96;
    /// State `TrickFromN90Nose110`.
    pub(crate) const TRICK_FROM_N_90_NOSE_110: u16 = 97;
    /// State `TransitioningToOnBoard11`.
    pub(crate) const TRANSITIONING_TO_ON_BOARD_11: u16 = 98;
    /// State `Superman72`.
    pub(crate) const SUPERMAN_72: u16 = 99;
    /// Shared by 6: state `FingerFlip135`, state `FingerFlip143`, state `FingerFlip204`, state
    /// `FingerFlip207` and 2 more.
    pub(crate) const FINGER_FLIP: u16 = 100;
    /// State `Coffin18`.
    pub(crate) const COFFIN_18: u16 = 101;
    /// State `Kickturn50`.
    pub(crate) const KICKTURN_50: u16 = 102;
    /// State `At9061`.
    pub(crate) const AT_90_61: u16 = 103;
    /// Shared by 2: state `FingerFlipShuv216`, state `FingerFlipShuv223`.
    pub(crate) const BOARD_ADJUST_FINGER_FLIP_SHUV: u16 = 104;
    /// State `Straight63`.
    pub(crate) const STRAIGHT_63: u16 = 105;
    /// State `Straight65`.
    pub(crate) const STRAIGHT_65: u16 = 106;
}

/// States by name and index, with their path below the root state.
pub(crate) mod state {
    /// `Action`.
    pub(crate) const ACTION_0: u16 = 0;
    /// `OnBoardGeneral`.
    pub(crate) const ON_BOARD_GENERAL_1: u16 = 1;
    /// `OffBoard`.
    pub(crate) const OFF_BOARD_2: u16 = 2;
    /// `Default`.
    pub(crate) const DEFAULT_3: u16 = 3;
    /// `OnBoardGeneral/ForceBrake`.
    pub(crate) const FORCE_BRAKE_4: u16 = 4;
    /// `OnBoardGeneral/NoInputBrake`.
    pub(crate) const NO_INPUT_BRAKE_5: u16 = 5;
    /// `OnBoardGeneral/OnBoard`.
    pub(crate) const ON_BOARD_6: u16 = 6;
    /// `OnBoardGeneral/TeleportOffBoard`.
    pub(crate) const TELEPORT_OFF_BOARD_7: u16 = 7;
    /// `OnBoardGeneral/TransToOffBoard`.
    pub(crate) const TRANS_TO_OFF_BOARD_8: u16 = 8;
    /// `OnBoardGeneral/TransitioningToOffBoard`.
    pub(crate) const TRANSITIONING_TO_OFF_BOARD_9: u16 = 9;
    /// `OnBoardGeneral/TransToOnBoard`.
    pub(crate) const TRANS_TO_ON_BOARD_10: u16 = 10;
    /// `OnBoardGeneral/TransitioningToOnBoard`.
    pub(crate) const TRANSITIONING_TO_ON_BOARD_11: u16 = 11;
    /// `OffBoard/OBGround`.
    pub(crate) const OB_GROUND_12: u16 = 12;
    /// `OffBoard/OBAir`.
    pub(crate) const OB_AIR_13: u16 = 13;
    /// `OnBoardGeneral/NoInputBrake/StartBrake`.
    pub(crate) const START_BRAKE_14: u16 = 14;
    /// `OnBoardGeneral/NoInputBrake/WaitBrake`.
    pub(crate) const WAIT_BRAKE_15: u16 = 15;
    /// `OnBoardGeneral/OnBoard/Grinding`.
    pub(crate) const GRINDING_16: u16 = 16;
    /// `OnBoardGeneral/OnBoard/InAir`.
    pub(crate) const IN_AIR_17: u16 = 17;
    /// `OnBoardGeneral/OnBoard/Coffin`.
    pub(crate) const COFFIN_18: u16 = 18;
    /// `OnBoardGeneral/OnBoard/OnGround`.
    pub(crate) const ON_GROUND_19: u16 = 19;
    /// `OnBoardGeneral/OnBoard/Trick`.
    pub(crate) const TRICK_20: u16 = 20;
    /// `OnBoardGeneral/OnBoard/LateFlips`.
    pub(crate) const LATE_FLIPS_21: u16 = 21;
    /// `OnBoardGeneral/OnBoard/TrickOutOfTipSlashGrind`.
    pub(crate) const TRICK_OUT_OF_TIP_SLASH_GRIND_22: u16 = 22;
    /// `OnBoardGeneral/OnBoard/Wipeout`.
    pub(crate) const WIPEOUT_23: u16 = 23;
    /// `OnBoardGeneral/OnBoard/Teleport`.
    pub(crate) const TELEPORT_24: u16 = 24;
    /// `OnBoardGeneral/TransitioningToOnBoard/Mirrored`.
    pub(crate) const MIRRORED_25: u16 = 25;
    /// `OnBoardGeneral/TransitioningToOnBoard/UnMirrored`.
    pub(crate) const UN_MIRRORED_26: u16 = 26;
    /// `OffBoard/OBGround/Running`.
    pub(crate) const RUNNING_27: u16 = 27;
    /// `OffBoard/OBGround/HippyJump`.
    pub(crate) const HIPPY_JUMP_28: u16 = 28;
    /// `OffBoard/OBGround/JumpingOntoBoard`.
    pub(crate) const JUMPING_ONTO_BOARD_29: u16 = 29;
    /// `OffBoard/OBGround/Default`.
    pub(crate) const DEFAULT_30: u16 = 30;
    /// `OffBoard/OBAir/AccidentalDismount`.
    pub(crate) const ACCIDENTAL_DISMOUNT_31: u16 = 31;
    /// `OffBoard/OBAir/Dismount`.
    pub(crate) const DISMOUNT_32: u16 = 32;
    /// `OffBoard/OBAir/Default`.
    pub(crate) const DEFAULT_33: u16 = 33;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail`.
    pub(crate) const TAIL_34: u16 = 34;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose`.
    pub(crate) const NOSE_35: u16 = 35;
    /// `OnBoardGeneral/OnBoard/Grinding/Square`.
    pub(crate) const SQUARE_36: u16 = 36;
    /// `OnBoardGeneral/OnBoard/InAir/HandPlant`.
    pub(crate) const HAND_PLANT_37: u16 = 37;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing`.
    pub(crate) const GRABBING_38: u16 = 38;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting`.
    pub(crate) const BOARD_ADJUSTING_39: u16 = 39;
    /// `OnBoardGeneral/OnBoard/InAir/NewOneFoot`.
    pub(crate) const NEW_ONE_FOOT_40: u16 = 40;
    /// `OnBoardGeneral/OnBoard/InAir/OneFootRight`.
    pub(crate) const ONE_FOOT_RIGHT_41: u16 = 41;
    /// `OnBoardGeneral/OnBoard/InAir/OneFootLeft`.
    pub(crate) const ONE_FOOT_LEFT_42: u16 = 42;
    /// `OnBoardGeneral/OnBoard/InAir/Idle`.
    pub(crate) const IDLE_43: u16 = 43;
    /// `OnBoardGeneral/OnBoard/InAir/TransToOffBoardAir`.
    pub(crate) const TRANS_TO_OFF_BOARD_AIR_44: u16 = 44;
    /// `OnBoardGeneral/OnBoard/OnGround/Grabbing`.
    pub(crate) const GRABBING_45: u16 = 45;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic`.
    pub(crate) const ANTIC_46: u16 = 46;
    /// `OnBoardGeneral/OnBoard/OnGround/HippyJumping`.
    pub(crate) const HIPPY_JUMPING_47: u16 = 47;
    /// `OnBoardGeneral/OnBoard/OnGround/Pushing`.
    pub(crate) const PUSHING_48: u16 = 48;
    /// `OnBoardGeneral/OnBoard/OnGround/Brake`.
    pub(crate) const BRAKE_49: u16 = 49;
    /// `OnBoardGeneral/OnBoard/OnGround/Kickturn`.
    pub(crate) const KICKTURN_50: u16 = 50;
    /// `OnBoardGeneral/OnBoard/OnGround/IdleTurning`.
    pub(crate) const IDLE_TURNING_51: u16 = 51;
    /// `OnBoardGeneral/OnBoard/Trick/TrickFromSquare`.
    pub(crate) const TRICK_FROM_SQUARE_52: u16 = 52;
    /// `OnBoardGeneral/OnBoard/Trick/TrickFromTail`.
    pub(crate) const TRICK_FROM_TAIL_53: u16 = 53;
    /// `OnBoardGeneral/OnBoard/Trick/TrickFromNose`.
    pub(crate) const TRICK_FROM_NOSE_54: u16 = 54;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_F_Kickflip`.
    pub(crate) const L_F_KICKFLIP_55: u16 = 55;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_F_Heelflip`.
    pub(crate) const L_F_HEELFLIP_56: u16 = 56;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_B_Kickflip`.
    pub(crate) const L_B_KICKFLIP_57: u16 = 57;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_B_Heelflip`.
    pub(crate) const L_B_HEELFLIP_58: u16 = 58;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_FS_Shuvit`.
    pub(crate) const L_FS_SHUVIT_59: u16 = 59;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_BS_Shuvit`.
    pub(crate) const L_BS_SHUVIT_60: u16 = 60;
    /// `OnBoardGeneral/OnBoard/TrickOutOfTipSlashGrind/At90`.
    pub(crate) const AT_90_61: u16 = 61;
    /// `OnBoardGeneral/OnBoard/TrickOutOfTipSlashGrind/AtN90`.
    pub(crate) const AT_N_90_62: u16 = 62;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight`.
    pub(crate) const STRAIGHT_63: u16 = 63;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways`.
    pub(crate) const SIDEWAYS_64: u16 = 64;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight`.
    pub(crate) const STRAIGHT_65: u16 = 65;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways`.
    pub(crate) const SIDEWAYS_66: u16 = 66;
    /// `OnBoardGeneral/OnBoard/Grinding/Square/LeftGrab`.
    pub(crate) const LEFT_GRAB_67: u16 = 67;
    /// `OnBoardGeneral/OnBoard/Grinding/Square/RightGrab`.
    pub(crate) const RIGHT_GRAB_68: u16 = 68;
    /// `OnBoardGeneral/OnBoard/Grinding/Square/DoubleGrab`.
    pub(crate) const DOUBLE_GRAB_69: u16 = 69;
    /// `OnBoardGeneral/OnBoard/Grinding/Square/Idle`.
    pub(crate) const IDLE_70: u16 = 70;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/DBLGrab`.
    pub(crate) const DBL_GRAB_71: u16 = 71;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/Superman`.
    pub(crate) const SUPERMAN_72: u16 = 72;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab`.
    pub(crate) const LEFT_GRAB_73: u16 = 73;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab`.
    pub(crate) const RIGHT_GRAB_74: u16 = 74;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute`.
    pub(crate) const STALE_MUTE_75: u16 = 75;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up`.
    pub(crate) const UP_76: u16 = 76;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down`.
    pub(crate) const DOWN_77: u16 = 77;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Left`.
    pub(crate) const LEFT_78: u16 = 78;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Right`.
    pub(crate) const RIGHT_79: u16 = 79;
    /// `OnBoardGeneral/OnBoard/OnGround/Grabbing/DBLGrab`.
    pub(crate) const DBL_GRAB_80: u16 = 80;
    /// `OnBoardGeneral/OnBoard/OnGround/Grabbing/LeftGrab`.
    pub(crate) const LEFT_GRAB_81: u16 = 81;
    /// `OnBoardGeneral/OnBoard/OnGround/Grabbing/RightGrab`.
    pub(crate) const RIGHT_GRAB_82: u16 = 82;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/OllieAnticipation`.
    pub(crate) const OLLIE_ANTICIPATION_83: u16 = 83;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/PopShuvitAnticipation`.
    pub(crate) const POP_SHUVIT_ANTICIPATION_84: u16 = 84;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/FSPopShuvitAnticipation`.
    pub(crate) const FS_POP_SHUVIT_ANTICIPATION_85: u16 = 85;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/360PopShuvitAnticipation`.
    pub(crate) const STATE_360_POP_SHUVIT_ANTICIPATION_86: u16 = 86;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/FS360PopShuvitAnticipation`.
    pub(crate) const FS_360_POP_SHUVIT_ANTICIPATION_87: u16 = 87;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/NollieAnticipation`.
    pub(crate) const NOLLIE_ANTICIPATION_88: u16 = 88;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_PopShuvitAnticipation`.
    pub(crate) const N_POP_SHUVIT_ANTICIPATION_89: u16 = 89;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_FSPopShuvitAnticipation`.
    pub(crate) const N_FS_POP_SHUVIT_ANTICIPATION_90: u16 = 90;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_360PopShuvitAnticipation`.
    pub(crate) const N_360_POP_SHUVIT_ANTICIPATION_91: u16 = 91;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_FS360PopShuvitAnticipation`.
    pub(crate) const N_FS_360_POP_SHUVIT_ANTICIPATION_92: u16 = 92;
    /// `OnBoardGeneral/OnBoard/OnGround/IdleTurning/HardTurn`.
    pub(crate) const HARD_TURN_93: u16 = 93;
    /// `OnBoardGeneral/OnBoard/OnGround/IdleTurning/IdleTurning`.
    pub(crate) const IDLE_TURNING_94: u16 = 94;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_F_Kickflip/Regular`.
    pub(crate) const REGULAR_95: u16 = 95;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_F_Kickflip/Switch`.
    pub(crate) const SWITCH_96: u16 = 96;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_F_Heelflip/Regular`.
    pub(crate) const REGULAR_97: u16 = 97;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_F_Heelflip/Switch`.
    pub(crate) const SWITCH_98: u16 = 98;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_B_Kickflip/Regular`.
    pub(crate) const REGULAR_99: u16 = 99;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_B_Kickflip/Switch`.
    pub(crate) const SWITCH_100: u16 = 100;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_B_Heelflip/Regular`.
    pub(crate) const REGULAR_101: u16 = 101;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_B_Heelflip/Switch`.
    pub(crate) const SWITCH_102: u16 = 102;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_FS_Shuvit/Regular`.
    pub(crate) const REGULAR_103: u16 = 103;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_FS_Shuvit/Switch`.
    pub(crate) const SWITCH_104: u16 = 104;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_BS_Shuvit/Regular`.
    pub(crate) const REGULAR_105: u16 = 105;
    /// `OnBoardGeneral/OnBoard/LateFlips/L_BS_Shuvit/Switch`.
    pub(crate) const SWITCH_106: u16 = 106;
    /// `OnBoardGeneral/OnBoard/TrickOutOfTipSlashGrind/At90/TrickFrom90Tail`.
    pub(crate) const TRICK_FROM_90_TAIL_107: u16 = 107;
    /// `OnBoardGeneral/OnBoard/TrickOutOfTipSlashGrind/At90/TrickFrom90Nose`.
    pub(crate) const TRICK_FROM_90_NOSE_108: u16 = 108;
    /// `OnBoardGeneral/OnBoard/TrickOutOfTipSlashGrind/AtN90/TrickFromN90Tail`.
    pub(crate) const TRICK_FROM_N_90_TAIL_109: u16 = 109;
    /// `OnBoardGeneral/OnBoard/TrickOutOfTipSlashGrind/AtN90/TrickFromN90Nose`.
    pub(crate) const TRICK_FROM_N_90_NOSE_110: u16 = 110;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight/LeftGrab`.
    pub(crate) const LEFT_GRAB_111: u16 = 111;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight/RightGrab`.
    pub(crate) const RIGHT_GRAB_112: u16 = 112;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight/DoubleGrab`.
    pub(crate) const DOUBLE_GRAB_113: u16 = 113;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight/Idle`.
    pub(crate) const IDLE_114: u16 = 114;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways/LeftGrab`.
    pub(crate) const LEFT_GRAB_115: u16 = 115;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways/RightGrab`.
    pub(crate) const RIGHT_GRAB_116: u16 = 116;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways/DoubleGrab`.
    pub(crate) const DOUBLE_GRAB_117: u16 = 117;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways/Idle`.
    pub(crate) const IDLE_118: u16 = 118;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight/LeftGrab`.
    pub(crate) const LEFT_GRAB_119: u16 = 119;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight/RightGrab`.
    pub(crate) const RIGHT_GRAB_120: u16 = 120;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight/DoubleGrab`.
    pub(crate) const DOUBLE_GRAB_121: u16 = 121;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight/Idle`.
    pub(crate) const IDLE_122: u16 = 122;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways/LeftGrab`.
    pub(crate) const LEFT_GRAB_123: u16 = 123;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways/RightGrab`.
    pub(crate) const RIGHT_GRAB_124: u16 = 124;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways/DoubleGrab`.
    pub(crate) const DOUBLE_GRAB_125: u16 = 125;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways/Idle`.
    pub(crate) const IDLE_126: u16 = 126;
    /// `OnBoardGeneral/OnBoard/Grinding/Square/LeftGrab/Regular`.
    pub(crate) const REGULAR_127: u16 = 127;
    /// `OnBoardGeneral/OnBoard/Grinding/Square/LeftGrab/Switch`.
    pub(crate) const SWITCH_128: u16 = 128;
    /// `OnBoardGeneral/OnBoard/Grinding/Square/RightGrab/Regular`.
    pub(crate) const REGULAR_129: u16 = 129;
    /// `OnBoardGeneral/OnBoard/Grinding/Square/RightGrab/Switch`.
    pub(crate) const SWITCH_130: u16 = 130;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/Dismount`.
    pub(crate) const DISMOUNT_131: u16 = 131;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/NewOneFoot`.
    pub(crate) const NEW_ONE_FOOT_132: u16 = 132;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/OneFootRight`.
    pub(crate) const ONE_FOOT_RIGHT_133: u16 = 133;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/OneFootLeft`.
    pub(crate) const ONE_FOOT_LEFT_134: u16 = 134;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_135: u16 = 135;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/FingerFlipShuv`.
    pub(crate) const FINGER_FLIP_SHUV_136: u16 = 136;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/FingerFlipVar`.
    pub(crate) const FINGER_FLIP_VAR_137: u16 = 137;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/Idle`.
    pub(crate) const IDLE_138: u16 = 138;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/Dismount`.
    pub(crate) const DISMOUNT_139: u16 = 139;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/NewOneFoot`.
    pub(crate) const NEW_ONE_FOOT_140: u16 = 140;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/OneFootRight`.
    pub(crate) const ONE_FOOT_RIGHT_141: u16 = 141;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/OneFootLeft`.
    pub(crate) const ONE_FOOT_LEFT_142: u16 = 142;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_143: u16 = 143;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/FingerFlipShuv`.
    pub(crate) const FINGER_FLIP_SHUV_144: u16 = 144;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/FingerFlipVar`.
    pub(crate) const FINGER_FLIP_VAR_145: u16 = 145;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/Idle`.
    pub(crate) const IDLE_146: u16 = 146;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute/MuteGrab`.
    pub(crate) const MUTE_GRAB_147: u16 = 147;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute/StaleGrab`.
    pub(crate) const STALE_GRAB_148: u16 = 148;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/TailGrab`.
    pub(crate) const TAIL_GRAB_149: u16 = 149;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/SeatbeltGrab`.
    pub(crate) const SEATBELT_GRAB_150: u16 = 150;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/Idle`.
    pub(crate) const IDLE_151: u16 = 151;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/NoseGrab`.
    pub(crate) const NOSE_GRAB_152: u16 = 152;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/CrailGrab`.
    pub(crate) const CRAIL_GRAB_153: u16 = 153;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/RocketAir`.
    pub(crate) const ROCKET_AIR_154: u16 = 154;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/Idle`.
    pub(crate) const IDLE_155: u16 = 155;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Left/Regular`.
    pub(crate) const REGULAR_156: u16 = 156;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Left/Switch`.
    pub(crate) const SWITCH_157: u16 = 157;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Right/Regular`.
    pub(crate) const REGULAR_158: u16 = 158;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Right/Switch`.
    pub(crate) const SWITCH_159: u16 = 159;
    /// `OnBoardGeneral/OnBoard/OnGround/Grabbing/LeftGrab/Goofy`.
    pub(crate) const GOOFY_160: u16 = 160;
    /// `OnBoardGeneral/OnBoard/OnGround/Grabbing/LeftGrab/Regular`.
    pub(crate) const REGULAR_161: u16 = 161;
    /// `OnBoardGeneral/OnBoard/OnGround/Grabbing/RightGrab/Goofy`.
    pub(crate) const GOOFY_162: u16 = 162;
    /// `OnBoardGeneral/OnBoard/OnGround/Grabbing/RightGrab/Regular`.
    pub(crate) const REGULAR_163: u16 = 163;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/PopShuvitAnticipation/Regular`.
    pub(crate) const REGULAR_164: u16 = 164;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/PopShuvitAnticipation/Switch`.
    pub(crate) const SWITCH_165: u16 = 165;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/FSPopShuvitAnticipation/Regular`.
    pub(crate) const REGULAR_166: u16 = 166;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/FSPopShuvitAnticipation/Switch`.
    pub(crate) const SWITCH_167: u16 = 167;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/360PopShuvitAnticipation/Regular`.
    pub(crate) const REGULAR_168: u16 = 168;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/360PopShuvitAnticipation/Switch`.
    pub(crate) const SWITCH_169: u16 = 169;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/FS360PopShuvitAnticipation/Regular`.
    pub(crate) const REGULAR_170: u16 = 170;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/FS360PopShuvitAnticipation/Switch`.
    pub(crate) const SWITCH_171: u16 = 171;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_PopShuvitAnticipation/Regular`.
    pub(crate) const REGULAR_172: u16 = 172;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_PopShuvitAnticipation/Switch`.
    pub(crate) const SWITCH_173: u16 = 173;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_FSPopShuvitAnticipation/Regular`.
    pub(crate) const REGULAR_174: u16 = 174;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_FSPopShuvitAnticipation/Switch`.
    pub(crate) const SWITCH_175: u16 = 175;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_360PopShuvitAnticipation/Regular`.
    pub(crate) const REGULAR_176: u16 = 176;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_360PopShuvitAnticipation/Switch`.
    pub(crate) const SWITCH_177: u16 = 177;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_FS360PopShuvitAnticipation/Regular`.
    pub(crate) const REGULAR_178: u16 = 178;
    /// `OnBoardGeneral/OnBoard/OnGround/Antic/N_FS360PopShuvitAnticipation/Switch`.
    pub(crate) const SWITCH_179: u16 = 179;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight/LeftGrab/Regular`.
    pub(crate) const REGULAR_180: u16 = 180;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight/LeftGrab/Switch`.
    pub(crate) const SWITCH_181: u16 = 181;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight/RightGrab/Regular`.
    pub(crate) const REGULAR_182: u16 = 182;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Straight/RightGrab/Switch`.
    pub(crate) const SWITCH_183: u16 = 183;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways/LeftGrab/Regular`.
    pub(crate) const REGULAR_184: u16 = 184;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways/LeftGrab/Switch`.
    pub(crate) const SWITCH_185: u16 = 185;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways/RightGrab/Regular`.
    pub(crate) const REGULAR_186: u16 = 186;
    /// `OnBoardGeneral/OnBoard/Grinding/Tail/Sideways/RightGrab/Switch`.
    pub(crate) const SWITCH_187: u16 = 187;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight/LeftGrab/Regular`.
    pub(crate) const REGULAR_188: u16 = 188;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight/LeftGrab/Switch`.
    pub(crate) const SWITCH_189: u16 = 189;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight/RightGrab/Regular`.
    pub(crate) const REGULAR_190: u16 = 190;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Straight/RightGrab/Switch`.
    pub(crate) const SWITCH_191: u16 = 191;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways/LeftGrab/Regular`.
    pub(crate) const REGULAR_192: u16 = 192;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways/LeftGrab/Switch`.
    pub(crate) const SWITCH_193: u16 = 193;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways/RightGrab/Regular`.
    pub(crate) const REGULAR_194: u16 = 194;
    /// `OnBoardGeneral/OnBoard/Grinding/Nose/Sideways/RightGrab/Switch`.
    pub(crate) const SWITCH_195: u16 = 195;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/OneFootRight/Footplant`.
    pub(crate) const FOOTPLANT_196: u16 = 196;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/OneFootRight/Idle`.
    pub(crate) const IDLE_197: u16 = 197;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/OneFootLeft/Footplant`.
    pub(crate) const FOOTPLANT_198: u16 = 198;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/LeftGrab/OneFootLeft/Idle`.
    pub(crate) const IDLE_199: u16 = 199;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/OneFootRight/Footplant`.
    pub(crate) const FOOTPLANT_200: u16 = 200;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/OneFootRight/Idle`.
    pub(crate) const IDLE_201: u16 = 201;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/OneFootLeft/Footplant`.
    pub(crate) const FOOTPLANT_202: u16 = 202;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/RightGrab/OneFootLeft/Idle`.
    pub(crate) const IDLE_203: u16 = 203;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute/MuteGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_204: u16 = 204;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute/MuteGrab/FingerFlipShuv`.
    pub(crate) const FINGER_FLIP_SHUV_205: u16 = 205;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute/MuteGrab/Idle`.
    pub(crate) const IDLE_206: u16 = 206;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute/StaleGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_207: u16 = 207;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute/StaleGrab/FingerFlipShuv`.
    pub(crate) const FINGER_FLIP_SHUV_208: u16 = 208;
    /// `OnBoardGeneral/OnBoard/InAir/Grabbing/StaleMute/StaleGrab/Idle`.
    pub(crate) const IDLE_209: u16 = 209;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/TailGrab/OneFootRight`.
    pub(crate) const ONE_FOOT_RIGHT_210: u16 = 210;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/TailGrab/OneFootLeft`.
    pub(crate) const ONE_FOOT_LEFT_211: u16 = 211;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/TailGrab/NoFootAirWalk`.
    pub(crate) const NO_FOOT_AIR_WALK_212: u16 = 212;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/TailGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_213: u16 = 213;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/TailGrab/IdleTailGrab`.
    pub(crate) const IDLE_TAIL_GRAB_214: u16 = 214;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/SeatbeltGrab/IdleSeatbeltGrab`.
    pub(crate) const IDLE_SEATBELT_GRAB_215: u16 = 215;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Up/SeatbeltGrab/FingerFlipShuv`.
    pub(crate) const FINGER_FLIP_SHUV_216: u16 = 216;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/NoseGrab/OneFootRight`.
    pub(crate) const ONE_FOOT_RIGHT_217: u16 = 217;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/NoseGrab/OneFootLeft`.
    pub(crate) const ONE_FOOT_LEFT_218: u16 = 218;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/NoseGrab/NoFootAirWalk`.
    pub(crate) const NO_FOOT_AIR_WALK_219: u16 = 219;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/NoseGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_220: u16 = 220;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/NoseGrab/IdleNoseGrab`.
    pub(crate) const IDLE_NOSE_GRAB_221: u16 = 221;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/CrailGrab/IdleCrailGrab`.
    pub(crate) const IDLE_CRAIL_GRAB_222: u16 = 222;
    /// `OnBoardGeneral/OnBoard/InAir/BoardAdjusting/Down/CrailGrab/FingerFlipShuv`.
    pub(crate) const FINGER_FLIP_SHUV_223: u16 = 223;
}
