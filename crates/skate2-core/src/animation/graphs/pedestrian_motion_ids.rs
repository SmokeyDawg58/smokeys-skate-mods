//! The pedestrian Motion state graph's ids under names from its own data, written by
//! `tools/generate_graph_ids.py`: conditions by class and attributes, expressions by the
//! states and transitions that evaluate them, states by name and index.
#![allow(dead_code, reason = "the complete graph; code names the ids it reads")]

/// Condition declarations: what each condition index tests.
pub(crate) mod condition {
    /// `HasIntent(intent = Collision)`.
    pub(crate) const HAS_INTENT_COLLISION: u16 = 0;
    /// `HasIntent(intent = RecoveryCollision)`.
    pub(crate) const HAS_INTENT_RECOVERY_COLLISION: u16 = 1;
    /// `HasIntent(intent = Knockdown)`.
    pub(crate) const HAS_INTENT_KNOCKDOWN: u16 = 2;
    /// `HasIntent(intent = FromFront)`.
    pub(crate) const HAS_INTENT_FROM_FRONT: u16 = 3;
    /// `HasIntent(intent = LyingDown)`.
    pub(crate) const HAS_INTENT_LYING_DOWN: u16 = 4;
    /// `WillExpire(InTime = 0.01)`.
    pub(crate) const WILL_EXPIRE_IN_0_01: u16 = 5;
    /// `HasIntent(intent = Crouched)`.
    pub(crate) const HAS_INTENT_CROUCHED: u16 = 6;
    /// `HasIntent(intent = Recover)`.
    pub(crate) const HAS_INTENT_RECOVER: u16 = 7;
    /// `WillExpire(always = true, InTime = 0.01)`.
    pub(crate) const WILL_EXPIRE_ALWAYS_IN_0_01: u16 = 8;
    /// `HasIntent(intent = FromBack)`.
    pub(crate) const HAS_INTENT_FROM_BACK: u16 = 9;
    /// `HasIntent(intent = FromLeft)`.
    pub(crate) const HAS_INTENT_FROM_LEFT: u16 = 10;
    /// `HasIntent(intent = FromRight)`.
    pub(crate) const HAS_INTENT_FROM_RIGHT: u16 = 11;
    /// `HasIntent(intent = DoTakeDown)`.
    pub(crate) const HAS_INTENT_DO_TAKE_DOWN: u16 = 12;
    /// `HasIntent(intent = ClothesLineR)`.
    pub(crate) const HAS_INTENT_CLOTHES_LINE_R: u16 = 13;
    /// `WillExpire(InTime = 0.03)`.
    pub(crate) const WILL_EXPIRE_IN_0_03: u16 = 14;
    /// `HasIntent(intent = TakedownSuccessful)`.
    pub(crate) const HAS_INTENT_TAKEDOWN_SUCCESSFUL: u16 = 15;
    /// `HasIntent(intent = TakedownFailed)`.
    pub(crate) const HAS_INTENT_TAKEDOWN_FAILED: u16 = 16;
    /// `HasIntent(intent = ClothesLineL)`.
    pub(crate) const HAS_INTENT_CLOTHES_LINE_L: u16 = 17;
    /// `HasIntent(intent = PushR135)`.
    pub(crate) const HAS_INTENT_PUSH_R_135: u16 = 18;
    /// `HasIntent(intent = PushL135)`.
    pub(crate) const HAS_INTENT_PUSH_L_135: u16 = 19;
    /// `HasIntent(intent = PushR90)`.
    pub(crate) const HAS_INTENT_PUSH_R_90: u16 = 20;
    /// `HasIntent(intent = PushL90)`.
    pub(crate) const HAS_INTENT_PUSH_L_90: u16 = 21;
    /// `HasIntent(intent = PushFront)`.
    pub(crate) const HAS_INTENT_PUSH_FRONT: u16 = 22;
    /// `HasIntent(intent = PushCatch)`.
    pub(crate) const HAS_INTENT_PUSH_CATCH: u16 = 23;
    /// `HasIntent(intent = CheckFront)`.
    pub(crate) const HAS_INTENT_CHECK_FRONT: u16 = 24;
    /// `HasIntent(intent = CheckRight)`.
    pub(crate) const HAS_INTENT_CHECK_RIGHT: u16 = 25;
    /// `HasIntent(intent = CheckLeft)`.
    pub(crate) const HAS_INTENT_CHECK_LEFT: u16 = 26;
    /// `HasIntent(intent = Avoid)`.
    pub(crate) const HAS_INTENT_AVOID: u16 = 27;
    /// `HasIntent(intent = Jump)`.
    pub(crate) const HAS_INTENT_JUMP: u16 = 28;
    /// `LastState(state = Locomotion.Sit.SittingCyc)`.
    pub(crate) const LAST_STATE_LOCOMOTION_SIT_SITTING_CYC: u16 = 29;
    /// `LastState(state = SitSmall)`.
    pub(crate) const LAST_STATE_SIT_SMALL: u16 = 30;
    /// `HasIntent(intent = Forward)`.
    pub(crate) const HAS_INTENT_FORWARD: u16 = 31;
    /// `HasIntent(intent = Left)`.
    pub(crate) const HAS_INTENT_LEFT: u16 = 32;
    /// `HasIntent(intent = Right)`.
    pub(crate) const HAS_INTENT_RIGHT: u16 = 33;
    /// `HasIntent(intent = Back)`.
    pub(crate) const HAS_INTENT_BACK: u16 = 34;
    /// `HasPlugin`.
    pub(crate) const HAS_PLUGIN: u16 = 35;
    /// `LastState(state = Locomotion.ChaseRun)`.
    pub(crate) const LAST_STATE_LOCOMOTION_CHASE_RUN: u16 = 36;
    /// `HasIntent(intent = Step)`.
    pub(crate) const HAS_INTENT_STEP: u16 = 37;
    /// `BranchWindowEnabled`.
    pub(crate) const BRANCH_WINDOW_ENABLED: u16 = 38;
    /// `InTurnBranchWindow`.
    pub(crate) const IN_TURN_BRANCH_WINDOW: u16 = 39;
    /// `HasIntent(intent = ChaseJump)`.
    pub(crate) const HAS_INTENT_CHASE_JUMP: u16 = 40;
    /// `PedestrianJumperInDownwardMotion`.
    pub(crate) const PEDESTRIAN_JUMPER_IN_DOWNWARD_MOTION: u16 = 41;
    /// `PedestrianJumperIsLanding`.
    pub(crate) const PEDESTRIAN_JUMPER_IS_LANDING: u16 = 42;
    /// `HasIntent(intent = ChaseJump)`, declared again: the test of `HAS_INTENT_CHASE_JUMP` under
    /// its own index.
    pub(crate) const HAS_INTENT_CHASE_JUMP_2ND: u16 = 43;
    /// `HasIntent(intent = Taunt)`.
    pub(crate) const HAS_INTENT_TAUNT: u16 = 44;
    /// `HasIntent(intent = Disappoint)`.
    pub(crate) const HAS_INTENT_DISAPPOINT: u16 = 45;
    /// `HasIntent(intent = Converse)`.
    pub(crate) const HAS_INTENT_CONVERSE: u16 = 46;
    /// `HasIntent(intent = StopConverse)`.
    pub(crate) const HAS_INTENT_STOP_CONVERSE: u16 = 47;
    /// `HasIntent(intent = Sit)`.
    pub(crate) const HAS_INTENT_SIT: u16 = 48;
    /// `LastState(state = Locomotion.Avoid)`.
    pub(crate) const LAST_STATE_LOCOMOTION_AVOID: u16 = 49;
    /// `LastState(state = Locomotion.Colliding)`.
    pub(crate) const LAST_STATE_LOCOMOTION_COLLIDING: u16 = 50;
    /// `HasIntent(intent = StandUp)`.
    pub(crate) const HAS_INTENT_STAND_UP: u16 = 51;
    /// `HasIntent(intent = Sit)`, declared again: the test of `HAS_INTENT_SIT` under its own index.
    pub(crate) const HAS_INTENT_SIT_2ND: u16 = 52;
    /// `HasIntent(intent = UseATM)`.
    pub(crate) const HAS_INTENT_USE_ATM: u16 = 53;
    /// `HasIntent(intent = UseATM)`, declared again: the test of `HAS_INTENT_USE_ATM` under its own
    /// index.
    pub(crate) const HAS_INTENT_USE_ATM_2ND: u16 = 54;
    /// `HasIntent(intent = UseVendingMachine)`.
    pub(crate) const HAS_INTENT_USE_VENDING_MACHINE: u16 = 55;
    /// `HasIntent(intent = UseVendingMachine)`, declared again: the test of
    /// `HAS_INTENT_USE_VENDING_MACHINE` under its own index.
    pub(crate) const HAS_INTENT_USE_VENDING_MACHINE_2ND: u16 = 56;
    /// `HasIntent(intent = UseWaterFountain)`.
    pub(crate) const HAS_INTENT_USE_WATER_FOUNTAIN: u16 = 57;
    /// `HasIntent(intent = FinishDrinking)`.
    pub(crate) const HAS_INTENT_FINISH_DRINKING: u16 = 58;
    /// `HasIntent(intent = UseWaterFountain)`, declared again: the test of
    /// `HAS_INTENT_USE_WATER_FOUNTAIN` under its own index.
    pub(crate) const HAS_INTENT_USE_WATER_FOUNTAIN_2ND: u16 = 59;
    /// `HasIntent(intent = UseNewspaperBox)`.
    pub(crate) const HAS_INTENT_USE_NEWSPAPER_BOX: u16 = 60;
    /// `HasIntent(intent = UseNewspaperBox)`, declared again: the test of
    /// `HAS_INTENT_USE_NEWSPAPER_BOX` under its own index.
    pub(crate) const HAS_INTENT_USE_NEWSPAPER_BOX_2ND: u16 = 61;
    /// `HasIntent(intent = Strafe)`.
    pub(crate) const HAS_INTENT_STRAFE: u16 = 62;
    /// `LastState(state = Locomotion.Walk)`.
    pub(crate) const LAST_STATE_LOCOMOTION_WALK: u16 = 63;
    /// `LastState(state = Locomotion.Shuffle)`.
    pub(crate) const LAST_STATE_LOCOMOTION_SHUFFLE: u16 = 64;
    /// `LastState(state = Locomotion.Idle)`.
    pub(crate) const LAST_STATE_LOCOMOTION_IDLE: u16 = 65;
    /// `WillExpire(always = true, InTime = 0.03)`.
    pub(crate) const WILL_EXPIRE_ALWAYS_IN_0_03: u16 = 66;
    /// `HasIntent(intent = ReverseRight)`.
    pub(crate) const HAS_INTENT_REVERSE_RIGHT: u16 = 67;
    /// `WillExpire(always = true, InTime = 0.1)`.
    pub(crate) const WILL_EXPIRE_ALWAYS_IN_0_1: u16 = 68;
    /// `HasIntent(intent = ReverseLeft)`.
    pub(crate) const HAS_INTENT_REVERSE_LEFT: u16 = 69;
    /// `HasIntent(intent = TurnRight135)`.
    pub(crate) const HAS_INTENT_TURN_RIGHT_135: u16 = 70;
    /// `HasIntent(intent = TurnLeft135)`.
    pub(crate) const HAS_INTENT_TURN_LEFT_135: u16 = 71;
    /// `HasIntent(intent = TurnRight90)`.
    pub(crate) const HAS_INTENT_TURN_RIGHT_90: u16 = 72;
    /// `HasIntent(intent = TurnLeft90)`.
    pub(crate) const HAS_INTENT_TURN_LEFT_90: u16 = 73;
    /// `HasIntent(intent = TurnRight45)`.
    pub(crate) const HAS_INTENT_TURN_RIGHT_45: u16 = 74;
    /// `HasIntent(intent = TurnLeft45)`.
    pub(crate) const HAS_INTENT_TURN_LEFT_45: u16 = 75;
    /// `HasIntent(intent = MoveForwards)`.
    pub(crate) const HAS_INTENT_MOVE_FORWARDS: u16 = 76;
    /// `HasIntent(intent = MoveBackwards)`.
    pub(crate) const HAS_INTENT_MOVE_BACKWARDS: u16 = 77;
    /// `HasIntent(intent = MoveLeft45)`.
    pub(crate) const HAS_INTENT_MOVE_LEFT_45: u16 = 78;
    /// `HasIntent(intent = MoveRight45)`.
    pub(crate) const HAS_INTENT_MOVE_RIGHT_45: u16 = 79;
    /// `HasIntent(intent = MoveLeft90)`.
    pub(crate) const HAS_INTENT_MOVE_LEFT_90: u16 = 80;
    /// `HasIntent(intent = MoveRight90)`.
    pub(crate) const HAS_INTENT_MOVE_RIGHT_90: u16 = 81;
    /// `HasIntent(intent = MoveLeft135)`.
    pub(crate) const HAS_INTENT_MOVE_LEFT_135: u16 = 82;
    /// `HasIntent(intent = MoveRight135)`.
    pub(crate) const HAS_INTENT_MOVE_RIGHT_135: u16 = 83;
    /// `LastState(state = Locomotion.Strafe.StrafeMotion)`.
    pub(crate) const LAST_STATE_LOCOMOTION_STRAFE_STRAFE_MOTION: u16 = 84;
    /// `HasIntent(intent = ChaseRun)`.
    pub(crate) const HAS_INTENT_CHASE_RUN: u16 = 85;
    /// `LastState(state = Locomotion.Strafe)`.
    pub(crate) const LAST_STATE_LOCOMOTION_STRAFE: u16 = 86;
    /// `LastState(state = Locomotion.Tired)`.
    pub(crate) const LAST_STATE_LOCOMOTION_TIRED: u16 = 87;
    /// `HasIntent(intent = Slow)`.
    pub(crate) const HAS_INTENT_SLOW: u16 = 88;
    /// `LastState(state = ChaseRunCyc.Fast)`.
    pub(crate) const LAST_STATE_CHASE_RUN_CYC_FAST: u16 = 89;
    /// `LastState(state = ChaseRunCyc.Default)`.
    pub(crate) const LAST_STATE_CHASE_RUN_CYC_DEFAULT: u16 = 90;
    /// `HasIntent(intent = Fast)`.
    pub(crate) const HAS_INTENT_FAST: u16 = 91;
    /// `LastState(state = ChaseRunCyc.Slow)`.
    pub(crate) const LAST_STATE_CHASE_RUN_CYC_SLOW: u16 = 92;
    /// `HasIntent(intent = Shuffle)`.
    pub(crate) const HAS_INTENT_SHUFFLE: u16 = 93;
    /// `HasIntent(intent = Idle)`.
    pub(crate) const HAS_INTENT_IDLE: u16 = 94;
    /// `HasIntent(intent = Tired)`.
    pub(crate) const HAS_INTENT_TIRED: u16 = 95;
    /// `LastState(state = ChaseRun.ChaseRunCyc)`.
    pub(crate) const LAST_STATE_CHASE_RUN_CHASE_RUN_CYC: u16 = 96;
    /// `InBranchWindow(BranchAnimInfo = TiredBranchInfo)`.
    pub(crate) const IN_BRANCH_WINDOW_TIRED_BRANCH_INFO: u16 = 97;
    /// `HasIntent(intent = StopTired)`.
    pub(crate) const HAS_INTENT_STOP_TIRED: u16 = 98;
    /// `HasIntent(intent = Brisk)`.
    pub(crate) const HAS_INTENT_BRISK: u16 = 99;
    /// `LastState(state = WalkCyc.Default)`.
    pub(crate) const LAST_STATE_WALK_CYC_DEFAULT: u16 = 100;
    /// `LastState(state = WalkCyc.Brisk)`.
    pub(crate) const LAST_STATE_WALK_CYC_BRISK: u16 = 101;
}

/// Expressions: which states and transitions evaluate each expression index.
pub(crate) mod expression {
    /// Shared by 37: state `Collision2`, `AttackClothesLineR22` -> `Collision2`, `PlayAttack116` ->
    /// `Collision2`, `AttackClothesLineL23` -> `Collision2` and 33 more.
    pub(crate) const HAS_INTENT_COLLISION: u16 = 0;
    /// Shared by 2: state `FromFront110`, state `FromFront400`.
    pub(crate) const FROM_FRONT: u16 = 1;
    /// Shared by 4: state `LyingDown558`, state `LyingDown561`, state `LyingDown564`, state
    /// `LyingDown567`.
    pub(crate) const LYING_DOWN: u16 = 2;
    /// Shared by 38: `LyingDown558` -> `OnTheGround385`, `Crouched559` -> `OnTheGround385`,
    /// `FromStand560` -> `OnTheGround385`, `LyingDown561` -> `OnTheGround389` and 34 more.
    pub(crate) const WILL_EXPIRE_IN_0_01: u16 = 3;
    /// Shared by 4: state `Crouched559`, state `Crouched562`, state `Crouched565`, state
    /// `Crouched568`.
    pub(crate) const CROUCHED: u16 = 4;
    /// Shared by 4: `OnTheGround385` -> `GetUp386`, `OnTheGround389` -> `GetUp390`,
    /// `OnTheGround393` -> `GetUp394`, `OnTheGround397` -> `GetUp398`.
    pub(crate) const ON_THE_GROUND_TO_GET_UP: u16 = 5;
    /// Shared by 4: `GetUp386` -> `Knockdown20`, `GetUp390` -> `Knockdown20`, `GetUp394` ->
    /// `Knockdown20`, `GetUp398` -> `Knockdown20`.
    pub(crate) const GET_UP_TO_KNOCKDOWN: u16 = 6;
    /// Shared by 38: `GetUp386` -> `KnockdownComplete387`, `GetUp390` -> `KnockdownComplete391`,
    /// `GetUp394` -> `KnockdownComplete395`, `GetUp398` -> `KnockdownComplete399` and 34 more.
    pub(crate) const WILL_EXPIRE_ALWAYS_IN_0_01: u16 = 7;
    /// Shared by 5: `KnockdownComplete387` -> `Locomotion1`, `KnockdownComplete391` ->
    /// `Locomotion1`, `KnockdownComplete395` -> `Locomotion1`, `KnockdownComplete399` ->
    /// `Locomotion1` and 1 more.
    pub(crate) const TO_LOCOMOTION: u16 = 8;
    /// Shared by 2: state `FromBack111`, state `FromBack401`.
    pub(crate) const FROM_BACK: u16 = 9;
    /// Shared by 2: state `FromLeft112`, state `FromLeft402`.
    pub(crate) const FROM_LEFT: u16 = 10;
    /// Shared by 2: state `FromRight113`, state `FromRight403`.
    pub(crate) const FROM_RIGHT: u16 = 11;
    /// State `Takedown3`.
    pub(crate) const TAKEDOWN_3: u16 = 12;
    /// State `AttackClothesLineR22`.
    pub(crate) const ATTACK_CLOTHES_LINE_R_22: u16 = 13;
    /// Shared by 11: `PlayAttack116` -> `AttackComplete117`, `PlayAttack121` ->
    /// `AttackComplete122`, `PlayAttack126` -> `AttackComplete127`, `PlayAttack131` ->
    /// `AttackComplete132` and 7 more.
    pub(crate) const PLAY_ATTACK_TO_ATTACK_COMPLETE: u16 = 14;
    /// Shared by 11: `TakedownComplete120` -> `Locomotion1`, `TakedownComplete125` ->
    /// `Locomotion1`, `TakedownComplete130` -> `Locomotion1`, `TakedownComplete135` ->
    /// `Locomotion1` and 7 more.
    pub(crate) const TAKEDOWN_COMPLETE_TO_LOCOMOTION: u16 = 15;
    /// State `AttackClothesLineL23`.
    pub(crate) const ATTACK_CLOTHES_LINE_L_23: u16 = 16;
    /// State `AttackPushR13524`.
    pub(crate) const ATTACK_PUSH_R_135_24: u16 = 17;
    /// State `AttackPushL13525`.
    pub(crate) const ATTACK_PUSH_L_135_25: u16 = 18;
    /// State `AttackPushR9026`.
    pub(crate) const ATTACK_PUSH_R_90_26: u16 = 19;
    /// State `AttackPushL9027`.
    pub(crate) const ATTACK_PUSH_L_90_27: u16 = 20;
    /// State `AttackPushFront28`.
    pub(crate) const ATTACK_PUSH_FRONT_28: u16 = 21;
    /// State `AttackPushCatch29`.
    pub(crate) const ATTACK_PUSH_CATCH_29: u16 = 22;
    /// State `AttackCheckFront30`.
    pub(crate) const ATTACK_CHECK_FRONT_30: u16 = 23;
    /// State `AttackCheckRight31`.
    pub(crate) const ATTACK_CHECK_RIGHT_31: u16 = 24;
    /// State `AttackCheckLeft32`.
    pub(crate) const ATTACK_CHECK_LEFT_32: u16 = 25;
    /// Shared by 6: state `Avoid4`, `Converse8` -> `Avoid4`, `Strafe14` -> `Avoid4`, `Shuffle16` ->
    /// `Avoid4` and 2 more.
    pub(crate) const HAS_INTENT_AVOID: u16 = 26;
    /// Shared by 4: state `AvoidForward173`, state `JumpForward177`, state `JumpForward404`, state
    /// `StepForward409`.
    pub(crate) const HAS_INTENT_FORWARD: u16 = 27;
    /// Shared by 4: state `AvoidLeft174`, state `JumpLeft178`, state `JumpLeft405`, state
    /// `StepLeft410`.
    pub(crate) const HAS_INTENT_LEFT: u16 = 28;
    /// Shared by 4: state `AvoidRight175`, state `JumpRight179`, state `JumpRight406`, state
    /// `StepRight411`.
    pub(crate) const HAS_INTENT_RIGHT: u16 = 29;
    /// Shared by 4: state `AvoidBack176`, state `JumpBack180`, state `StepBack412`, state
    /// `JumpBack570`.
    pub(crate) const HAS_INTENT_BACK: u16 = 30;
    /// `StillSitting407` -> `NotSittingAnymore408`.
    pub(crate) const STILL_SITTING_407_TO_NOT_SITTING_ANYMORE_408: u16 = 31;
    /// State `SitSmall34`.
    pub(crate) const SIT_SMALL_34: u16 = 32;
    /// Shared by 2: `SitSmall34` -> `SitJump33`, `WaitForBranch181` -> `Jump35`.
    pub(crate) const HAS_INTENT_JUMP: u16 = 33;
    /// State `Step36`.
    pub(crate) const STEP_36: u16 = 34;
    /// Shared by 103: state `WaitForBranch181`, state `WaitForBranchFlag199`, state
    /// `WaitForBranchFlag210`, state `WaitForBranchFlag213` and 99 more.
    pub(crate) const BRANCH_WINDOW_ENABLED: u16 = 35;
    /// Shared by 111: `WaitForBranch181` -> `DoStep182`, `WaitForVend414` -> `VendHandProp415`,
    /// `WaitForHandProp416` -> `CollectHandProp417`, `WaitForBranchFlag199` -> `PlayTransition200`
    /// and 107 more.
    pub(crate) const IN_TURN_BRANCH_WINDOW: u16 = 36;
    /// `AvoidComplete37` -> `Locomotion1`.
    pub(crate) const AVOID_COMPLETE_37_TO_LOCOMOTION_1: u16 = 37;
    /// State `Jump5`.
    pub(crate) const JUMP_5: u16 = 38;
    /// `JumpAirUp39` -> `JumpAirDown40`.
    pub(crate) const JUMP_AIR_UP_39_TO_JUMP_AIR_DOWN_40: u16 = 39;
    /// Shared by 2: `JumpAirUp39` -> `JumpOut41`, `JumpAirDown40` -> `JumpOut41`.
    pub(crate) const TO_JUMP_OUT: u16 = 40;
    /// `JumpOut41` -> `Locomotion1`.
    pub(crate) const JUMP_OUT_41_TO_LOCOMOTION_1: u16 = 41;
    /// State `Taunt6`.
    pub(crate) const TAUNT_6: u16 = 42;
    /// `TauntComplete43` -> `Locomotion1`.
    pub(crate) const TAUNT_COMPLETE_43_TO_LOCOMOTION_1: u16 = 43;
    /// State `Disappoint7`.
    pub(crate) const DISAPPOINT_7: u16 = 44;
    /// `DisappointComplete45` -> `Locomotion1`.
    pub(crate) const DISAPPOINT_COMPLETE_45_TO_LOCOMOTION_1: u16 = 45;
    /// State `Converse8`.
    pub(crate) const CONVERSE_8: u16 = 46;
    /// `Cycle413` -> `ConverseComplete47`.
    pub(crate) const CYCLE_413_TO_CONVERSE_COMPLETE_47: u16 = 47;
    /// `ConverseComplete47` -> `Locomotion1`.
    pub(crate) const CONVERSE_COMPLETE_47_TO_LOCOMOTION_1: u16 = 48;
    /// State `Sit9`.
    pub(crate) const SIT_9: u16 = 49;
    /// `SitComplete51` -> `Locomotion1`.
    pub(crate) const SIT_COMPLETE_51_TO_LOCOMOTION_1: u16 = 50;
    /// State `UseATM10`.
    pub(crate) const USE_ATM_10: u16 = 51;
    /// `UseATMComplete188` -> `Locomotion1`.
    pub(crate) const USE_ATM_COMPLETE_188_TO_LOCOMOTION_1: u16 = 52;
    /// State `UseVendingMachine11`.
    pub(crate) const USE_VENDING_MACHINE_11: u16 = 53;
    /// `UseVendingMachineComplete192` -> `Locomotion1`.
    pub(crate) const USE_VENDING_MACHINE_COMPLETE_192_TO_LOCOMOTION_1: u16 = 54;
    /// State `UseWaterFountain12`.
    pub(crate) const USE_WATER_FOUNTAIN_12: u16 = 55;
    /// `WaterFountainCyc194` -> `WaterFountainOut195`.
    pub(crate) const WATER_FOUNTAIN_CYC_194_TO_WATER_FOUNTAIN_OUT_195: u16 = 56;
    /// `WaterFountainComplete196` -> `Locomotion1`.
    pub(crate) const WATER_FOUNTAIN_COMPLETE_196_TO_LOCOMOTION_1: u16 = 57;
    /// State `UseNewspaperBox13`.
    pub(crate) const USE_NEWSPAPER_BOX_13: u16 = 58;
    /// `UseNewspaperBoxComplete198` -> `Locomotion1`.
    pub(crate) const USE_NEWSPAPER_BOX_COMPLETE_198_TO_LOCOMOTION_1: u16 = 59;
    /// State `Strafe14`.
    pub(crate) const STRAFE_14: u16 = 60;
    /// Shared by 3: `PlayTransition200` -> `StrafeIdle63`, `PlayTransition435` -> `StrafeIdle63`,
    /// `PlayTransition553` -> `WalkCyc109`.
    pub(crate) const WILL_EXPIRE_ALWAYS_IN_0_03: u16 = 61;
    /// Shared by 2: state `StrafeFromChase57`, state `ChaseRunToIdle320`.
    pub(crate) const LAST_STATE_LOCOMOTION_CHASE_RUN: u16 = 62;
    /// Shared by 10: state `ReverseRight66`, state `ReverseRight76`, state `ReverseRight86`, state
    /// `ReverseRight101` and 6 more.
    pub(crate) const REVERSE_RIGHT: u16 = 63;
    /// Shared by 91: `PlayTransition419` -> `StrafeIdle63`, `PlayTransition421` -> `StrafeIdle63`,
    /// `PlayTransition423` -> `StrafeIdle63`, `PlayTransition425` -> `StrafeIdle63` and 87 more.
    pub(crate) const WILL_EXPIRE_ALWAYS_IN_0_1: u16 = 64;
    /// Shared by 10: state `ReverseLeft67`, state `ReverseLeft77`, state `ReverseLeft87`, state
    /// `ReverseLeft102` and 6 more.
    pub(crate) const REVERSE_LEFT: u16 = 65;
    /// Shared by 10: state `TurnRight13568`, state `TurnRight13578`, state `TurnRight13588`, state
    /// `TurnRight135103` and 6 more.
    pub(crate) const TURN_RIGHT_135: u16 = 66;
    /// Shared by 10: state `TurnLeft13569`, state `TurnLeft13579`, state `TurnLeft13589`, state
    /// `TurnLeft135104` and 6 more.
    pub(crate) const TURN_LEFT_135: u16 = 67;
    /// Shared by 11: state `TurnRight9058`, state `TurnRight9070`, state `TurnRight9080`, state
    /// `TurnRight9090` and 7 more.
    pub(crate) const TURN_RIGHT_90: u16 = 68;
    /// Shared by 11: state `TurnLeft9059`, state `TurnLeft9071`, state `TurnLeft9081`, state
    /// `TurnLeft9091` and 7 more.
    pub(crate) const TURN_LEFT_90: u16 = 69;
    /// Shared by 11: state `TurnRight4560`, state `TurnRight4572`, state `TurnRight4582`, state
    /// `TurnRight4592` and 7 more.
    pub(crate) const TURN_RIGHT_45: u16 = 70;
    /// Shared by 11: state `TurnLeft4561`, state `TurnLeft4573`, state `TurnLeft4583`, state
    /// `TurnLeft4593` and 7 more.
    pub(crate) const TURN_LEFT_45: u16 = 71;
    /// State `MoveForwards222`.
    pub(crate) const MOVE_FORWARDS_222: u16 = 72;
    /// State `MoveBackwards223`.
    pub(crate) const MOVE_BACKWARDS_223: u16 = 73;
    /// State `MoveLeft45224`.
    pub(crate) const MOVE_LEFT_45_224: u16 = 74;
    /// State `MoveRight45225`.
    pub(crate) const MOVE_RIGHT_45_225: u16 = 75;
    /// State `MoveLeft90226`.
    pub(crate) const MOVE_LEFT_90_226: u16 = 76;
    /// State `MoveRight90227`.
    pub(crate) const MOVE_RIGHT_90_227: u16 = 77;
    /// State `MoveLeft135228`.
    pub(crate) const MOVE_LEFT_135_228: u16 = 78;
    /// State `MoveRight135229`.
    pub(crate) const MOVE_RIGHT_135_229: u16 = 79;
    /// State `ChaseRun15`.
    pub(crate) const CHASE_RUN_15: u16 = 80;
    /// Shared by 2: state `ChaseRunFromStrafe64`, state `StrafeToIdle321`.
    pub(crate) const LAST_STATE_LOCOMOTION_STRAFE: u16 = 81;
    /// State `Slow274`.
    pub(crate) const SLOW_274: u16 = 82;
    /// State `Fast275`.
    pub(crate) const FAST_275: u16 = 83;
    /// State `Shuffle16`.
    pub(crate) const SHUFFLE_16: u16 = 84;
    /// State `Idle17`.
    pub(crate) const IDLE_17: u16 = 85;
    /// State `WalkToIdle318`.
    pub(crate) const WALK_TO_IDLE_318: u16 = 86;
    /// State `ShuffleToIdle319`.
    pub(crate) const SHUFFLE_TO_IDLE_319: u16 = 87;
    /// State `Tired18`.
    pub(crate) const TIRED_18: u16 = 88;
    /// `Tired18` -> `Locomotion1`.
    pub(crate) const TIRED_18_TO_LOCOMOTION_1: u16 = 89;
    /// State `FromRun347`.
    pub(crate) const FROM_RUN_347: u16 = 90;
    /// `FromRun347` -> `TiredInto96`.
    pub(crate) const FROM_RUN_347_TO_TIRED_INTO_96: u16 = 91;
    /// `TiredComplete99` -> `Locomotion1`.
    pub(crate) const TIRED_COMPLETE_99_TO_LOCOMOTION_1: u16 = 92;
    /// State `Brisk382`.
    pub(crate) const BRISK_382: u16 = 93;
    /// State `Knockdown20`.
    pub(crate) const KNOCKDOWN_20: u16 = 94;
    /// Shared by 11: `AttackComplete117` -> `TakedownSuccessful118`, `AttackComplete122` ->
    /// `TakedownSuccessful123`, `AttackComplete127` -> `TakedownSuccessful128`, `AttackComplete132`
    /// -> `TakedownSuccessful133` and 7 more.
    pub(crate) const ATTACK_COMPLETE_TO_TAKEDOWN_SUCCESSFUL: u16 = 95;
    /// Shared by 11: `AttackComplete117` -> `TakedownFailed119`, `AttackComplete122` ->
    /// `TakedownFailed124`, `AttackComplete127` -> `TakedownFailed129`, `AttackComplete132` ->
    /// `TakedownFailed134` and 7 more.
    pub(crate) const ATTACK_COMPLETE_TO_TAKEDOWN_FAILED: u16 = 96;
    /// State `Jump35`.
    pub(crate) const JUMP_35: u16 = 97;
    /// `DoStep182` -> `Jump35`.
    pub(crate) const DO_STEP_182_TO_JUMP_35: u16 = 98;
    /// State `SittingInto48`.
    pub(crate) const SITTING_INTO_48: u16 = 99;
    /// `SittingCyc49` -> `SittingOut50`.
    pub(crate) const SITTING_CYC_49_TO_SITTING_OUT_50: u16 = 100;
    /// State `FromMotion230`.
    pub(crate) const FROM_MOTION_230: u16 = 101;
    /// `TiredCyc97` -> `TiredOut98`.
    pub(crate) const TIRED_CYC_97_TO_TIRED_OUT_98: u16 = 102;
    /// State `WaitForBranch554`.
    pub(crate) const WAIT_FOR_BRANCH_554: u16 = 103;
    /// State `WaitForBranch556`.
    pub(crate) const WAIT_FOR_BRANCH_556: u16 = 104;
    /// State `SitJump33`.
    pub(crate) const SIT_JUMP_33: u16 = 105;
    /// State `StrafeInto56`.
    pub(crate) const STRAFE_INTO_56: u16 = 106;
    /// State `WaitForBranch488`.
    pub(crate) const WAIT_FOR_BRANCH_488: u16 = 107;
    /// State `WaitForBranch490`.
    pub(crate) const WAIT_FOR_BRANCH_490: u16 = 108;
    /// State `WaitForBranch492`.
    pub(crate) const WAIT_FOR_BRANCH_492: u16 = 109;
    /// State `IdleInto85`.
    pub(crate) const IDLE_INTO_85: u16 = 110;
    /// State `ShuffleInto75`.
    pub(crate) const SHUFFLE_INTO_75: u16 = 111;
    /// State `WalkInto100`.
    pub(crate) const WALK_INTO_100: u16 = 112;
    /// State `ChaseRunInto65`.
    pub(crate) const CHASE_RUN_INTO_65: u16 = 113;
}

/// States by name and index, with their path below the root state.
pub(crate) mod state {
    /// `Motion`.
    pub(crate) const MOTION_0: u16 = 0;
    /// `Locomotion`.
    pub(crate) const LOCOMOTION_1: u16 = 1;
    /// `Locomotion/Collision`.
    pub(crate) const COLLISION_2: u16 = 2;
    /// `Locomotion/Takedown`.
    pub(crate) const TAKEDOWN_3: u16 = 3;
    /// `Locomotion/Avoid`.
    pub(crate) const AVOID_4: u16 = 4;
    /// `Locomotion/Jump`.
    pub(crate) const JUMP_5: u16 = 5;
    /// `Locomotion/Taunt`.
    pub(crate) const TAUNT_6: u16 = 6;
    /// `Locomotion/Disappoint`.
    pub(crate) const DISAPPOINT_7: u16 = 7;
    /// `Locomotion/Converse`.
    pub(crate) const CONVERSE_8: u16 = 8;
    /// `Locomotion/Sit`.
    pub(crate) const SIT_9: u16 = 9;
    /// `Locomotion/UseATM`.
    pub(crate) const USE_ATM_10: u16 = 10;
    /// `Locomotion/UseVendingMachine`.
    pub(crate) const USE_VENDING_MACHINE_11: u16 = 11;
    /// `Locomotion/UseWaterFountain`.
    pub(crate) const USE_WATER_FOUNTAIN_12: u16 = 12;
    /// `Locomotion/UseNewspaperBox`.
    pub(crate) const USE_NEWSPAPER_BOX_13: u16 = 13;
    /// `Locomotion/Strafe`.
    pub(crate) const STRAFE_14: u16 = 14;
    /// `Locomotion/ChaseRun`.
    pub(crate) const CHASE_RUN_15: u16 = 15;
    /// `Locomotion/Shuffle`.
    pub(crate) const SHUFFLE_16: u16 = 16;
    /// `Locomotion/Idle`.
    pub(crate) const IDLE_17: u16 = 17;
    /// `Locomotion/Tired`.
    pub(crate) const TIRED_18: u16 = 18;
    /// `Locomotion/Walk`.
    pub(crate) const WALK_19: u16 = 19;
    /// `Locomotion/Collision/Knockdown`.
    pub(crate) const KNOCKDOWN_20: u16 = 20;
    /// `Locomotion/Collision/Standing`.
    pub(crate) const STANDING_21: u16 = 21;
    /// `Locomotion/Takedown/AttackClothesLineR`.
    pub(crate) const ATTACK_CLOTHES_LINE_R_22: u16 = 22;
    /// `Locomotion/Takedown/AttackClothesLineL`.
    pub(crate) const ATTACK_CLOTHES_LINE_L_23: u16 = 23;
    /// `Locomotion/Takedown/AttackPushR135`.
    pub(crate) const ATTACK_PUSH_R_135_24: u16 = 24;
    /// `Locomotion/Takedown/AttackPushL135`.
    pub(crate) const ATTACK_PUSH_L_135_25: u16 = 25;
    /// `Locomotion/Takedown/AttackPushR90`.
    pub(crate) const ATTACK_PUSH_R_90_26: u16 = 26;
    /// `Locomotion/Takedown/AttackPushL90`.
    pub(crate) const ATTACK_PUSH_L_90_27: u16 = 27;
    /// `Locomotion/Takedown/AttackPushFront`.
    pub(crate) const ATTACK_PUSH_FRONT_28: u16 = 28;
    /// `Locomotion/Takedown/AttackPushCatch`.
    pub(crate) const ATTACK_PUSH_CATCH_29: u16 = 29;
    /// `Locomotion/Takedown/AttackCheckFront`.
    pub(crate) const ATTACK_CHECK_FRONT_30: u16 = 30;
    /// `Locomotion/Takedown/AttackCheckRight`.
    pub(crate) const ATTACK_CHECK_RIGHT_31: u16 = 31;
    /// `Locomotion/Takedown/AttackCheckLeft`.
    pub(crate) const ATTACK_CHECK_LEFT_32: u16 = 32;
    /// `Locomotion/Avoid/SitJump`.
    pub(crate) const SIT_JUMP_33: u16 = 33;
    /// `Locomotion/Avoid/SitSmall`.
    pub(crate) const SIT_SMALL_34: u16 = 34;
    /// `Locomotion/Avoid/Jump`.
    pub(crate) const JUMP_35: u16 = 35;
    /// `Locomotion/Avoid/Step`.
    pub(crate) const STEP_36: u16 = 36;
    /// `Locomotion/Avoid/AvoidComplete`.
    pub(crate) const AVOID_COMPLETE_37: u16 = 37;
    /// `Locomotion/Jump/JumpInto`.
    pub(crate) const JUMP_INTO_38: u16 = 38;
    /// `Locomotion/Jump/JumpAirUp`.
    pub(crate) const JUMP_AIR_UP_39: u16 = 39;
    /// `Locomotion/Jump/JumpAirDown`.
    pub(crate) const JUMP_AIR_DOWN_40: u16 = 40;
    /// `Locomotion/Jump/JumpOut`.
    pub(crate) const JUMP_OUT_41: u16 = 41;
    /// `Locomotion/Taunt/PlayTaunt`.
    pub(crate) const PLAY_TAUNT_42: u16 = 42;
    /// `Locomotion/Taunt/TauntComplete`.
    pub(crate) const TAUNT_COMPLETE_43: u16 = 43;
    /// `Locomotion/Disappoint/PlayDisappoint`.
    pub(crate) const PLAY_DISAPPOINT_44: u16 = 44;
    /// `Locomotion/Disappoint/DisappointComplete`.
    pub(crate) const DISAPPOINT_COMPLETE_45: u16 = 45;
    /// `Locomotion/Converse/ConverseFromIdle`.
    pub(crate) const CONVERSE_FROM_IDLE_46: u16 = 46;
    /// `Locomotion/Converse/ConverseComplete`.
    pub(crate) const CONVERSE_COMPLETE_47: u16 = 47;
    /// `Locomotion/Sit/SittingInto`.
    pub(crate) const SITTING_INTO_48: u16 = 48;
    /// `Locomotion/Sit/SittingCyc`.
    pub(crate) const SITTING_CYC_49: u16 = 49;
    /// `Locomotion/Sit/SittingOut`.
    pub(crate) const SITTING_OUT_50: u16 = 50;
    /// `Locomotion/Sit/SitComplete`.
    pub(crate) const SIT_COMPLETE_51: u16 = 51;
    /// `Locomotion/UseATM/UsingATM`.
    pub(crate) const USING_ATM_52: u16 = 52;
    /// `Locomotion/UseVendingMachine/UsingVendingMachine`.
    pub(crate) const USING_VENDING_MACHINE_53: u16 = 53;
    /// `Locomotion/UseWaterFountain/UsingWaterFountain`.
    pub(crate) const USING_WATER_FOUNTAIN_54: u16 = 54;
    /// `Locomotion/UseNewspaperBox/UsingNewspaperBox`.
    pub(crate) const USING_NEWSPAPER_BOX_55: u16 = 55;
    /// `Locomotion/Strafe/StrafeInto`.
    pub(crate) const STRAFE_INTO_56: u16 = 56;
    /// `Locomotion/Strafe/StrafeFromChase`.
    pub(crate) const STRAFE_FROM_CHASE_57: u16 = 57;
    /// `Locomotion/Strafe/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_58: u16 = 58;
    /// `Locomotion/Strafe/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_59: u16 = 59;
    /// `Locomotion/Strafe/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_60: u16 = 60;
    /// `Locomotion/Strafe/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_61: u16 = 61;
    /// `Locomotion/Strafe/StrafeMotion`.
    pub(crate) const STRAFE_MOTION_62: u16 = 62;
    /// `Locomotion/Strafe/StrafeIdle`.
    pub(crate) const STRAFE_IDLE_63: u16 = 63;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe`.
    pub(crate) const CHASE_RUN_FROM_STRAFE_64: u16 = 64;
    /// `Locomotion/ChaseRun/ChaseRunInto`.
    pub(crate) const CHASE_RUN_INTO_65: u16 = 65;
    /// `Locomotion/ChaseRun/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_66: u16 = 66;
    /// `Locomotion/ChaseRun/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_67: u16 = 67;
    /// `Locomotion/ChaseRun/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_68: u16 = 68;
    /// `Locomotion/ChaseRun/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_69: u16 = 69;
    /// `Locomotion/ChaseRun/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_70: u16 = 70;
    /// `Locomotion/ChaseRun/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_71: u16 = 71;
    /// `Locomotion/ChaseRun/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_72: u16 = 72;
    /// `Locomotion/ChaseRun/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_73: u16 = 73;
    /// `Locomotion/ChaseRun/ChaseRunCyc`.
    pub(crate) const CHASE_RUN_CYC_74: u16 = 74;
    /// `Locomotion/Shuffle/ShuffleInto`.
    pub(crate) const SHUFFLE_INTO_75: u16 = 75;
    /// `Locomotion/Shuffle/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_76: u16 = 76;
    /// `Locomotion/Shuffle/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_77: u16 = 77;
    /// `Locomotion/Shuffle/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_78: u16 = 78;
    /// `Locomotion/Shuffle/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_79: u16 = 79;
    /// `Locomotion/Shuffle/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_80: u16 = 80;
    /// `Locomotion/Shuffle/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_81: u16 = 81;
    /// `Locomotion/Shuffle/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_82: u16 = 82;
    /// `Locomotion/Shuffle/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_83: u16 = 83;
    /// `Locomotion/Shuffle/ShuffleDefault`.
    pub(crate) const SHUFFLE_DEFAULT_84: u16 = 84;
    /// `Locomotion/Idle/IdleInto`.
    pub(crate) const IDLE_INTO_85: u16 = 85;
    /// `Locomotion/Idle/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_86: u16 = 86;
    /// `Locomotion/Idle/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_87: u16 = 87;
    /// `Locomotion/Idle/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_88: u16 = 88;
    /// `Locomotion/Idle/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_89: u16 = 89;
    /// `Locomotion/Idle/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_90: u16 = 90;
    /// `Locomotion/Idle/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_91: u16 = 91;
    /// `Locomotion/Idle/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_92: u16 = 92;
    /// `Locomotion/Idle/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_93: u16 = 93;
    /// `Locomotion/Idle/IdleDefault`.
    pub(crate) const IDLE_DEFAULT_94: u16 = 94;
    /// `Locomotion/Tired/TiredWaitingForBranch`.
    pub(crate) const TIRED_WAITING_FOR_BRANCH_95: u16 = 95;
    /// `Locomotion/Tired/TiredInto`.
    pub(crate) const TIRED_INTO_96: u16 = 96;
    /// `Locomotion/Tired/TiredCyc`.
    pub(crate) const TIRED_CYC_97: u16 = 97;
    /// `Locomotion/Tired/TiredOut`.
    pub(crate) const TIRED_OUT_98: u16 = 98;
    /// `Locomotion/Tired/TiredComplete`.
    pub(crate) const TIRED_COMPLETE_99: u16 = 99;
    /// `Locomotion/Walk/WalkInto`.
    pub(crate) const WALK_INTO_100: u16 = 100;
    /// `Locomotion/Walk/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_101: u16 = 101;
    /// `Locomotion/Walk/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_102: u16 = 102;
    /// `Locomotion/Walk/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_103: u16 = 103;
    /// `Locomotion/Walk/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_104: u16 = 104;
    /// `Locomotion/Walk/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_105: u16 = 105;
    /// `Locomotion/Walk/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_106: u16 = 106;
    /// `Locomotion/Walk/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_107: u16 = 107;
    /// `Locomotion/Walk/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_108: u16 = 108;
    /// `Locomotion/Walk/WalkCyc`.
    pub(crate) const WALK_CYC_109: u16 = 109;
    /// `Locomotion/Collision/Knockdown/FromFront`.
    pub(crate) const FROM_FRONT_110: u16 = 110;
    /// `Locomotion/Collision/Knockdown/FromBack`.
    pub(crate) const FROM_BACK_111: u16 = 111;
    /// `Locomotion/Collision/Knockdown/FromLeft`.
    pub(crate) const FROM_LEFT_112: u16 = 112;
    /// `Locomotion/Collision/Knockdown/FromRight`.
    pub(crate) const FROM_RIGHT_113: u16 = 113;
    /// `Locomotion/Collision/Standing/Colliding`.
    pub(crate) const COLLIDING_114: u16 = 114;
    /// `Locomotion/Collision/Standing/StandingCollisionComplete`.
    pub(crate) const STANDING_COLLISION_COMPLETE_115: u16 = 115;
    /// `Locomotion/Takedown/AttackClothesLineR/PlayAttack`.
    pub(crate) const PLAY_ATTACK_116: u16 = 116;
    /// `Locomotion/Takedown/AttackClothesLineR/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_117: u16 = 117;
    /// `Locomotion/Takedown/AttackClothesLineR/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_118: u16 = 118;
    /// `Locomotion/Takedown/AttackClothesLineR/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_119: u16 = 119;
    /// `Locomotion/Takedown/AttackClothesLineR/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_120: u16 = 120;
    /// `Locomotion/Takedown/AttackClothesLineL/PlayAttack`.
    pub(crate) const PLAY_ATTACK_121: u16 = 121;
    /// `Locomotion/Takedown/AttackClothesLineL/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_122: u16 = 122;
    /// `Locomotion/Takedown/AttackClothesLineL/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_123: u16 = 123;
    /// `Locomotion/Takedown/AttackClothesLineL/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_124: u16 = 124;
    /// `Locomotion/Takedown/AttackClothesLineL/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_125: u16 = 125;
    /// `Locomotion/Takedown/AttackPushR135/PlayAttack`.
    pub(crate) const PLAY_ATTACK_126: u16 = 126;
    /// `Locomotion/Takedown/AttackPushR135/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_127: u16 = 127;
    /// `Locomotion/Takedown/AttackPushR135/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_128: u16 = 128;
    /// `Locomotion/Takedown/AttackPushR135/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_129: u16 = 129;
    /// `Locomotion/Takedown/AttackPushR135/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_130: u16 = 130;
    /// `Locomotion/Takedown/AttackPushL135/PlayAttack`.
    pub(crate) const PLAY_ATTACK_131: u16 = 131;
    /// `Locomotion/Takedown/AttackPushL135/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_132: u16 = 132;
    /// `Locomotion/Takedown/AttackPushL135/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_133: u16 = 133;
    /// `Locomotion/Takedown/AttackPushL135/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_134: u16 = 134;
    /// `Locomotion/Takedown/AttackPushL135/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_135: u16 = 135;
    /// `Locomotion/Takedown/AttackPushR90/PlayAttack`.
    pub(crate) const PLAY_ATTACK_136: u16 = 136;
    /// `Locomotion/Takedown/AttackPushR90/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_137: u16 = 137;
    /// `Locomotion/Takedown/AttackPushR90/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_138: u16 = 138;
    /// `Locomotion/Takedown/AttackPushR90/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_139: u16 = 139;
    /// `Locomotion/Takedown/AttackPushR90/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_140: u16 = 140;
    /// `Locomotion/Takedown/AttackPushL90/PlayAttack`.
    pub(crate) const PLAY_ATTACK_141: u16 = 141;
    /// `Locomotion/Takedown/AttackPushL90/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_142: u16 = 142;
    /// `Locomotion/Takedown/AttackPushL90/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_143: u16 = 143;
    /// `Locomotion/Takedown/AttackPushL90/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_144: u16 = 144;
    /// `Locomotion/Takedown/AttackPushL90/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_145: u16 = 145;
    /// `Locomotion/Takedown/AttackPushFront/PlayAttack`.
    pub(crate) const PLAY_ATTACK_146: u16 = 146;
    /// `Locomotion/Takedown/AttackPushFront/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_147: u16 = 147;
    /// `Locomotion/Takedown/AttackPushFront/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_148: u16 = 148;
    /// `Locomotion/Takedown/AttackPushFront/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_149: u16 = 149;
    /// `Locomotion/Takedown/AttackPushFront/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_150: u16 = 150;
    /// `Locomotion/Takedown/AttackPushCatch/PlayAttack`.
    pub(crate) const PLAY_ATTACK_151: u16 = 151;
    /// `Locomotion/Takedown/AttackPushCatch/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_152: u16 = 152;
    /// `Locomotion/Takedown/AttackPushCatch/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_153: u16 = 153;
    /// `Locomotion/Takedown/AttackPushCatch/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_154: u16 = 154;
    /// `Locomotion/Takedown/AttackPushCatch/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_155: u16 = 155;
    /// `Locomotion/Takedown/AttackCheckFront/PlayAttack`.
    pub(crate) const PLAY_ATTACK_156: u16 = 156;
    /// `Locomotion/Takedown/AttackCheckFront/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_157: u16 = 157;
    /// `Locomotion/Takedown/AttackCheckFront/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_158: u16 = 158;
    /// `Locomotion/Takedown/AttackCheckFront/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_159: u16 = 159;
    /// `Locomotion/Takedown/AttackCheckFront/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_160: u16 = 160;
    /// `Locomotion/Takedown/AttackCheckRight/PlayAttack`.
    pub(crate) const PLAY_ATTACK_161: u16 = 161;
    /// `Locomotion/Takedown/AttackCheckRight/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_162: u16 = 162;
    /// `Locomotion/Takedown/AttackCheckRight/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_163: u16 = 163;
    /// `Locomotion/Takedown/AttackCheckRight/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_164: u16 = 164;
    /// `Locomotion/Takedown/AttackCheckRight/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_165: u16 = 165;
    /// `Locomotion/Takedown/AttackCheckLeft/PlayAttack`.
    pub(crate) const PLAY_ATTACK_166: u16 = 166;
    /// `Locomotion/Takedown/AttackCheckLeft/AttackComplete`.
    pub(crate) const ATTACK_COMPLETE_167: u16 = 167;
    /// `Locomotion/Takedown/AttackCheckLeft/TakedownSuccessful`.
    pub(crate) const TAKEDOWN_SUCCESSFUL_168: u16 = 168;
    /// `Locomotion/Takedown/AttackCheckLeft/TakedownFailed`.
    pub(crate) const TAKEDOWN_FAILED_169: u16 = 169;
    /// `Locomotion/Takedown/AttackCheckLeft/TakedownComplete`.
    pub(crate) const TAKEDOWN_COMPLETE_170: u16 = 170;
    /// `Locomotion/Avoid/SitJump/LeaveSit`.
    pub(crate) const LEAVE_SIT_171: u16 = 171;
    /// `Locomotion/Avoid/SitJump/ReturnToSit`.
    pub(crate) const RETURN_TO_SIT_172: u16 = 172;
    /// `Locomotion/Avoid/SitSmall/AvoidForward`.
    pub(crate) const AVOID_FORWARD_173: u16 = 173;
    /// `Locomotion/Avoid/SitSmall/AvoidLeft`.
    pub(crate) const AVOID_LEFT_174: u16 = 174;
    /// `Locomotion/Avoid/SitSmall/AvoidRight`.
    pub(crate) const AVOID_RIGHT_175: u16 = 175;
    /// `Locomotion/Avoid/SitSmall/AvoidBack`.
    pub(crate) const AVOID_BACK_176: u16 = 176;
    /// `Locomotion/Avoid/Jump/JumpForward`.
    pub(crate) const JUMP_FORWARD_177: u16 = 177;
    /// `Locomotion/Avoid/Jump/JumpLeft`.
    pub(crate) const JUMP_LEFT_178: u16 = 178;
    /// `Locomotion/Avoid/Jump/JumpRight`.
    pub(crate) const JUMP_RIGHT_179: u16 = 179;
    /// `Locomotion/Avoid/Jump/JumpBack`.
    pub(crate) const JUMP_BACK_180: u16 = 180;
    /// `Locomotion/Avoid/Step/WaitForBranch`.
    pub(crate) const WAIT_FOR_BRANCH_181: u16 = 181;
    /// `Locomotion/Avoid/Step/DoStep`.
    pub(crate) const DO_STEP_182: u16 = 182;
    /// `Locomotion/Converse/ConverseFromIdle/ConverseBase`.
    pub(crate) const CONVERSE_BASE_183: u16 = 183;
    /// `Locomotion/UseATM/UsingATM/InsertCard`.
    pub(crate) const INSERT_CARD_184: u16 = 184;
    /// `Locomotion/UseATM/UsingATM/MakeSelection`.
    pub(crate) const MAKE_SELECTION_185: u16 = 185;
    /// `Locomotion/UseATM/UsingATM/CollectMoney`.
    pub(crate) const COLLECT_MONEY_186: u16 = 186;
    /// `Locomotion/UseATM/UsingATM/CollectCard`.
    pub(crate) const COLLECT_CARD_187: u16 = 187;
    /// `Locomotion/UseATM/UsingATM/UseATMComplete`.
    pub(crate) const USE_ATM_COMPLETE_188: u16 = 188;
    /// `Locomotion/UseVendingMachine/UsingVendingMachine/InsertMoney`.
    pub(crate) const INSERT_MONEY_189: u16 = 189;
    /// `Locomotion/UseVendingMachine/UsingVendingMachine/MakeSelection`.
    pub(crate) const MAKE_SELECTION_190: u16 = 190;
    /// `Locomotion/UseVendingMachine/UsingVendingMachine/CollectGoods`.
    pub(crate) const COLLECT_GOODS_191: u16 = 191;
    /// `Locomotion/UseVendingMachine/UsingVendingMachine/UseVendingMachineComplete`.
    pub(crate) const USE_VENDING_MACHINE_COMPLETE_192: u16 = 192;
    /// `Locomotion/UseWaterFountain/UsingWaterFountain/WaterFountainInto`.
    pub(crate) const WATER_FOUNTAIN_INTO_193: u16 = 193;
    /// `Locomotion/UseWaterFountain/UsingWaterFountain/WaterFountainCyc`.
    pub(crate) const WATER_FOUNTAIN_CYC_194: u16 = 194;
    /// `Locomotion/UseWaterFountain/UsingWaterFountain/WaterFountainOut`.
    pub(crate) const WATER_FOUNTAIN_OUT_195: u16 = 195;
    /// `Locomotion/UseWaterFountain/UsingWaterFountain/WaterFountainComplete`.
    pub(crate) const WATER_FOUNTAIN_COMPLETE_196: u16 = 196;
    /// `Locomotion/UseNewspaperBox/UsingNewspaperBox/CollectGoods`.
    pub(crate) const COLLECT_GOODS_197: u16 = 197;
    /// `Locomotion/UseNewspaperBox/UsingNewspaperBox/UseNewspaperBoxComplete`.
    pub(crate) const USE_NEWSPAPER_BOX_COMPLETE_198: u16 = 198;
    /// `Locomotion/Strafe/StrafeInto/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_199: u16 = 199;
    /// `Locomotion/Strafe/StrafeInto/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_200: u16 = 200;
    /// `Locomotion/Strafe/StrafeFromChase/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_201: u16 = 201;
    /// `Locomotion/Strafe/StrafeFromChase/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_202: u16 = 202;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_203: u16 = 203;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_204: u16 = 204;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_205: u16 = 205;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_206: u16 = 206;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_207: u16 = 207;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_208: u16 = 208;
    /// `Locomotion/Strafe/StrafeFromChase/StrafeFromChaseDefault`.
    pub(crate) const STRAFE_FROM_CHASE_DEFAULT_209: u16 = 209;
    /// `Locomotion/Strafe/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_210: u16 = 210;
    /// `Locomotion/Strafe/TurnRight90/PlayTurn`.
    pub(crate) const PLAY_TURN_211: u16 = 211;
    /// `Locomotion/Strafe/TurnRight90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_212: u16 = 212;
    /// `Locomotion/Strafe/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_213: u16 = 213;
    /// `Locomotion/Strafe/TurnLeft90/PlayTurn`.
    pub(crate) const PLAY_TURN_214: u16 = 214;
    /// `Locomotion/Strafe/TurnLeft90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_215: u16 = 215;
    /// `Locomotion/Strafe/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_216: u16 = 216;
    /// `Locomotion/Strafe/TurnRight45/PlayTurn`.
    pub(crate) const PLAY_TURN_217: u16 = 217;
    /// `Locomotion/Strafe/TurnRight45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_218: u16 = 218;
    /// `Locomotion/Strafe/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_219: u16 = 219;
    /// `Locomotion/Strafe/TurnLeft45/PlayTurn`.
    pub(crate) const PLAY_TURN_220: u16 = 220;
    /// `Locomotion/Strafe/TurnLeft45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_221: u16 = 221;
    /// `Locomotion/Strafe/StrafeMotion/MoveForwards`.
    pub(crate) const MOVE_FORWARDS_222: u16 = 222;
    /// `Locomotion/Strafe/StrafeMotion/MoveBackwards`.
    pub(crate) const MOVE_BACKWARDS_223: u16 = 223;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft45`.
    pub(crate) const MOVE_LEFT_45_224: u16 = 224;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight45`.
    pub(crate) const MOVE_RIGHT_45_225: u16 = 225;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft90`.
    pub(crate) const MOVE_LEFT_90_226: u16 = 226;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight90`.
    pub(crate) const MOVE_RIGHT_90_227: u16 = 227;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft135`.
    pub(crate) const MOVE_LEFT_135_228: u16 = 228;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight135`.
    pub(crate) const MOVE_RIGHT_135_229: u16 = 229;
    /// `Locomotion/Strafe/StrafeIdle/FromMotion`.
    pub(crate) const FROM_MOTION_230: u16 = 230;
    /// `Locomotion/Strafe/StrafeIdle/Default`.
    pub(crate) const DEFAULT_231: u16 = 231;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_232: u16 = 232;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_233: u16 = 233;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_234: u16 = 234;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_235: u16 = 235;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_236: u16 = 236;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_237: u16 = 237;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_238: u16 = 238;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_239: u16 = 239;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ChaseIntoDefault`.
    pub(crate) const CHASE_INTO_DEFAULT_240: u16 = 240;
    /// `Locomotion/ChaseRun/ChaseRunInto/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_241: u16 = 241;
    /// `Locomotion/ChaseRun/ChaseRunInto/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_242: u16 = 242;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_243: u16 = 243;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_244: u16 = 244;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_245: u16 = 245;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_246: u16 = 246;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_247: u16 = 247;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_248: u16 = 248;
    /// `Locomotion/ChaseRun/ChaseRunInto/ChaseIntoDefault`.
    pub(crate) const CHASE_INTO_DEFAULT_249: u16 = 249;
    /// `Locomotion/ChaseRun/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_250: u16 = 250;
    /// `Locomotion/ChaseRun/ReverseRight/PlayTurn`.
    pub(crate) const PLAY_TURN_251: u16 = 251;
    /// `Locomotion/ChaseRun/ReverseRight/TurnComplete`.
    pub(crate) const TURN_COMPLETE_252: u16 = 252;
    /// `Locomotion/ChaseRun/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_253: u16 = 253;
    /// `Locomotion/ChaseRun/ReverseLeft/PlayTurn`.
    pub(crate) const PLAY_TURN_254: u16 = 254;
    /// `Locomotion/ChaseRun/ReverseLeft/TurnComplete`.
    pub(crate) const TURN_COMPLETE_255: u16 = 255;
    /// `Locomotion/ChaseRun/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_256: u16 = 256;
    /// `Locomotion/ChaseRun/TurnRight135/PlayTurn`.
    pub(crate) const PLAY_TURN_257: u16 = 257;
    /// `Locomotion/ChaseRun/TurnRight135/TurnComplete`.
    pub(crate) const TURN_COMPLETE_258: u16 = 258;
    /// `Locomotion/ChaseRun/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_259: u16 = 259;
    /// `Locomotion/ChaseRun/TurnLeft135/PlayTurn`.
    pub(crate) const PLAY_TURN_260: u16 = 260;
    /// `Locomotion/ChaseRun/TurnLeft135/TurnComplete`.
    pub(crate) const TURN_COMPLETE_261: u16 = 261;
    /// `Locomotion/ChaseRun/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_262: u16 = 262;
    /// `Locomotion/ChaseRun/TurnRight90/PlayTurn`.
    pub(crate) const PLAY_TURN_263: u16 = 263;
    /// `Locomotion/ChaseRun/TurnRight90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_264: u16 = 264;
    /// `Locomotion/ChaseRun/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_265: u16 = 265;
    /// `Locomotion/ChaseRun/TurnLeft90/PlayTurn`.
    pub(crate) const PLAY_TURN_266: u16 = 266;
    /// `Locomotion/ChaseRun/TurnLeft90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_267: u16 = 267;
    /// `Locomotion/ChaseRun/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_268: u16 = 268;
    /// `Locomotion/ChaseRun/TurnRight45/PlayTurn`.
    pub(crate) const PLAY_TURN_269: u16 = 269;
    /// `Locomotion/ChaseRun/TurnRight45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_270: u16 = 270;
    /// `Locomotion/ChaseRun/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_271: u16 = 271;
    /// `Locomotion/ChaseRun/TurnLeft45/PlayTurn`.
    pub(crate) const PLAY_TURN_272: u16 = 272;
    /// `Locomotion/ChaseRun/TurnLeft45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_273: u16 = 273;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Slow`.
    pub(crate) const SLOW_274: u16 = 274;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Fast`.
    pub(crate) const FAST_275: u16 = 275;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Default`.
    pub(crate) const DEFAULT_276: u16 = 276;
    /// `Locomotion/Shuffle/ShuffleInto/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_277: u16 = 277;
    /// `Locomotion/Shuffle/ShuffleInto/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_278: u16 = 278;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_279: u16 = 279;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_280: u16 = 280;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_281: u16 = 281;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_282: u16 = 282;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_283: u16 = 283;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_284: u16 = 284;
    /// `Locomotion/Shuffle/ShuffleInto/ShuffleIntoDefault`.
    pub(crate) const SHUFFLE_INTO_DEFAULT_285: u16 = 285;
    /// `Locomotion/Shuffle/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_286: u16 = 286;
    /// `Locomotion/Shuffle/ReverseRight/PlayTurn`.
    pub(crate) const PLAY_TURN_287: u16 = 287;
    /// `Locomotion/Shuffle/ReverseRight/TurnComplete`.
    pub(crate) const TURN_COMPLETE_288: u16 = 288;
    /// `Locomotion/Shuffle/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_289: u16 = 289;
    /// `Locomotion/Shuffle/ReverseLeft/PlayTurn`.
    pub(crate) const PLAY_TURN_290: u16 = 290;
    /// `Locomotion/Shuffle/ReverseLeft/TurnComplete`.
    pub(crate) const TURN_COMPLETE_291: u16 = 291;
    /// `Locomotion/Shuffle/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_292: u16 = 292;
    /// `Locomotion/Shuffle/TurnRight135/PlayTurn`.
    pub(crate) const PLAY_TURN_293: u16 = 293;
    /// `Locomotion/Shuffle/TurnRight135/TurnComplete`.
    pub(crate) const TURN_COMPLETE_294: u16 = 294;
    /// `Locomotion/Shuffle/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_295: u16 = 295;
    /// `Locomotion/Shuffle/TurnLeft135/PlayTurn`.
    pub(crate) const PLAY_TURN_296: u16 = 296;
    /// `Locomotion/Shuffle/TurnLeft135/TurnComplete`.
    pub(crate) const TURN_COMPLETE_297: u16 = 297;
    /// `Locomotion/Shuffle/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_298: u16 = 298;
    /// `Locomotion/Shuffle/TurnRight90/PlayTurn`.
    pub(crate) const PLAY_TURN_299: u16 = 299;
    /// `Locomotion/Shuffle/TurnRight90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_300: u16 = 300;
    /// `Locomotion/Shuffle/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_301: u16 = 301;
    /// `Locomotion/Shuffle/TurnLeft90/PlayTurn`.
    pub(crate) const PLAY_TURN_302: u16 = 302;
    /// `Locomotion/Shuffle/TurnLeft90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_303: u16 = 303;
    /// `Locomotion/Shuffle/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_304: u16 = 304;
    /// `Locomotion/Shuffle/TurnRight45/PlayTurn`.
    pub(crate) const PLAY_TURN_305: u16 = 305;
    /// `Locomotion/Shuffle/TurnRight45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_306: u16 = 306;
    /// `Locomotion/Shuffle/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_307: u16 = 307;
    /// `Locomotion/Shuffle/TurnLeft45/PlayTurn`.
    pub(crate) const PLAY_TURN_308: u16 = 308;
    /// `Locomotion/Shuffle/TurnLeft45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_309: u16 = 309;
    /// `Locomotion/Idle/IdleInto/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_310: u16 = 310;
    /// `Locomotion/Idle/IdleInto/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_311: u16 = 311;
    /// `Locomotion/Idle/IdleInto/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_312: u16 = 312;
    /// `Locomotion/Idle/IdleInto/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_313: u16 = 313;
    /// `Locomotion/Idle/IdleInto/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_314: u16 = 314;
    /// `Locomotion/Idle/IdleInto/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_315: u16 = 315;
    /// `Locomotion/Idle/IdleInto/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_316: u16 = 316;
    /// `Locomotion/Idle/IdleInto/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_317: u16 = 317;
    /// `Locomotion/Idle/IdleInto/WalkToIdle`.
    pub(crate) const WALK_TO_IDLE_318: u16 = 318;
    /// `Locomotion/Idle/IdleInto/ShuffleToIdle`.
    pub(crate) const SHUFFLE_TO_IDLE_319: u16 = 319;
    /// `Locomotion/Idle/IdleInto/ChaseRunToIdle`.
    pub(crate) const CHASE_RUN_TO_IDLE_320: u16 = 320;
    /// `Locomotion/Idle/IdleInto/StrafeToIdle`.
    pub(crate) const STRAFE_TO_IDLE_321: u16 = 321;
    /// `Locomotion/Idle/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_322: u16 = 322;
    /// `Locomotion/Idle/ReverseRight/PlayTurn`.
    pub(crate) const PLAY_TURN_323: u16 = 323;
    /// `Locomotion/Idle/ReverseRight/TurnComplete`.
    pub(crate) const TURN_COMPLETE_324: u16 = 324;
    /// `Locomotion/Idle/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_325: u16 = 325;
    /// `Locomotion/Idle/ReverseLeft/PlayTurn`.
    pub(crate) const PLAY_TURN_326: u16 = 326;
    /// `Locomotion/Idle/ReverseLeft/TurnComplete`.
    pub(crate) const TURN_COMPLETE_327: u16 = 327;
    /// `Locomotion/Idle/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_328: u16 = 328;
    /// `Locomotion/Idle/TurnRight135/PlayTurn`.
    pub(crate) const PLAY_TURN_329: u16 = 329;
    /// `Locomotion/Idle/TurnRight135/TurnComplete`.
    pub(crate) const TURN_COMPLETE_330: u16 = 330;
    /// `Locomotion/Idle/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_331: u16 = 331;
    /// `Locomotion/Idle/TurnLeft135/PlayTurn`.
    pub(crate) const PLAY_TURN_332: u16 = 332;
    /// `Locomotion/Idle/TurnLeft135/TurnComplete`.
    pub(crate) const TURN_COMPLETE_333: u16 = 333;
    /// `Locomotion/Idle/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_334: u16 = 334;
    /// `Locomotion/Idle/TurnRight90/PlayTurn`.
    pub(crate) const PLAY_TURN_335: u16 = 335;
    /// `Locomotion/Idle/TurnRight90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_336: u16 = 336;
    /// `Locomotion/Idle/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_337: u16 = 337;
    /// `Locomotion/Idle/TurnLeft90/PlayTurn`.
    pub(crate) const PLAY_TURN_338: u16 = 338;
    /// `Locomotion/Idle/TurnLeft90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_339: u16 = 339;
    /// `Locomotion/Idle/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_340: u16 = 340;
    /// `Locomotion/Idle/TurnRight45/PlayTurn`.
    pub(crate) const PLAY_TURN_341: u16 = 341;
    /// `Locomotion/Idle/TurnRight45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_342: u16 = 342;
    /// `Locomotion/Idle/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_343: u16 = 343;
    /// `Locomotion/Idle/TurnLeft45/PlayTurn`.
    pub(crate) const PLAY_TURN_344: u16 = 344;
    /// `Locomotion/Idle/TurnLeft45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_345: u16 = 345;
    /// `Locomotion/Idle/IdleDefault/Basic`.
    pub(crate) const BASIC_346: u16 = 346;
    /// `Locomotion/Tired/TiredWaitingForBranch/FromRun`.
    pub(crate) const FROM_RUN_347: u16 = 347;
    /// `Locomotion/Tired/TiredWaitingForBranch/NotFromRun`.
    pub(crate) const NOT_FROM_RUN_348: u16 = 348;
    /// `Locomotion/Walk/WalkInto/ReverseRight`.
    pub(crate) const REVERSE_RIGHT_349: u16 = 349;
    /// `Locomotion/Walk/WalkInto/ReverseLeft`.
    pub(crate) const REVERSE_LEFT_350: u16 = 350;
    /// `Locomotion/Walk/WalkInto/TurnRight135`.
    pub(crate) const TURN_RIGHT_135_351: u16 = 351;
    /// `Locomotion/Walk/WalkInto/TurnLeft135`.
    pub(crate) const TURN_LEFT_135_352: u16 = 352;
    /// `Locomotion/Walk/WalkInto/TurnRight90`.
    pub(crate) const TURN_RIGHT_90_353: u16 = 353;
    /// `Locomotion/Walk/WalkInto/TurnLeft90`.
    pub(crate) const TURN_LEFT_90_354: u16 = 354;
    /// `Locomotion/Walk/WalkInto/TurnRight45`.
    pub(crate) const TURN_RIGHT_45_355: u16 = 355;
    /// `Locomotion/Walk/WalkInto/TurnLeft45`.
    pub(crate) const TURN_LEFT_45_356: u16 = 356;
    /// `Locomotion/Walk/WalkInto/WalkIntoDefault`.
    pub(crate) const WALK_INTO_DEFAULT_357: u16 = 357;
    /// `Locomotion/Walk/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_358: u16 = 358;
    /// `Locomotion/Walk/ReverseRight/PlayTurn`.
    pub(crate) const PLAY_TURN_359: u16 = 359;
    /// `Locomotion/Walk/ReverseRight/TurnComplete`.
    pub(crate) const TURN_COMPLETE_360: u16 = 360;
    /// `Locomotion/Walk/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_361: u16 = 361;
    /// `Locomotion/Walk/ReverseLeft/PlayTurn`.
    pub(crate) const PLAY_TURN_362: u16 = 362;
    /// `Locomotion/Walk/ReverseLeft/TurnComplete`.
    pub(crate) const TURN_COMPLETE_363: u16 = 363;
    /// `Locomotion/Walk/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_364: u16 = 364;
    /// `Locomotion/Walk/TurnRight135/PlayTurn`.
    pub(crate) const PLAY_TURN_365: u16 = 365;
    /// `Locomotion/Walk/TurnRight135/TurnComplete`.
    pub(crate) const TURN_COMPLETE_366: u16 = 366;
    /// `Locomotion/Walk/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_367: u16 = 367;
    /// `Locomotion/Walk/TurnLeft135/PlayTurn`.
    pub(crate) const PLAY_TURN_368: u16 = 368;
    /// `Locomotion/Walk/TurnLeft135/TurnComplete`.
    pub(crate) const TURN_COMPLETE_369: u16 = 369;
    /// `Locomotion/Walk/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_370: u16 = 370;
    /// `Locomotion/Walk/TurnRight90/PlayTurn`.
    pub(crate) const PLAY_TURN_371: u16 = 371;
    /// `Locomotion/Walk/TurnRight90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_372: u16 = 372;
    /// `Locomotion/Walk/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_373: u16 = 373;
    /// `Locomotion/Walk/TurnLeft90/PlayTurn`.
    pub(crate) const PLAY_TURN_374: u16 = 374;
    /// `Locomotion/Walk/TurnLeft90/TurnComplete`.
    pub(crate) const TURN_COMPLETE_375: u16 = 375;
    /// `Locomotion/Walk/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_376: u16 = 376;
    /// `Locomotion/Walk/TurnRight45/PlayTurn`.
    pub(crate) const PLAY_TURN_377: u16 = 377;
    /// `Locomotion/Walk/TurnRight45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_378: u16 = 378;
    /// `Locomotion/Walk/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_379: u16 = 379;
    /// `Locomotion/Walk/TurnLeft45/PlayTurn`.
    pub(crate) const PLAY_TURN_380: u16 = 380;
    /// `Locomotion/Walk/TurnLeft45/TurnComplete`.
    pub(crate) const TURN_COMPLETE_381: u16 = 381;
    /// `Locomotion/Walk/WalkCyc/Brisk`.
    pub(crate) const BRISK_382: u16 = 382;
    /// `Locomotion/Walk/WalkCyc/Default`.
    pub(crate) const DEFAULT_383: u16 = 383;
    /// `Locomotion/Collision/Knockdown/FromFront/Falling`.
    pub(crate) const FALLING_384: u16 = 384;
    /// `Locomotion/Collision/Knockdown/FromFront/OnTheGround`.
    pub(crate) const ON_THE_GROUND_385: u16 = 385;
    /// `Locomotion/Collision/Knockdown/FromFront/GetUp`.
    pub(crate) const GET_UP_386: u16 = 386;
    /// `Locomotion/Collision/Knockdown/FromFront/KnockdownComplete`.
    pub(crate) const KNOCKDOWN_COMPLETE_387: u16 = 387;
    /// `Locomotion/Collision/Knockdown/FromBack/Falling`.
    pub(crate) const FALLING_388: u16 = 388;
    /// `Locomotion/Collision/Knockdown/FromBack/OnTheGround`.
    pub(crate) const ON_THE_GROUND_389: u16 = 389;
    /// `Locomotion/Collision/Knockdown/FromBack/GetUp`.
    pub(crate) const GET_UP_390: u16 = 390;
    /// `Locomotion/Collision/Knockdown/FromBack/KnockdownComplete`.
    pub(crate) const KNOCKDOWN_COMPLETE_391: u16 = 391;
    /// `Locomotion/Collision/Knockdown/FromLeft/Falling`.
    pub(crate) const FALLING_392: u16 = 392;
    /// `Locomotion/Collision/Knockdown/FromLeft/OnTheGround`.
    pub(crate) const ON_THE_GROUND_393: u16 = 393;
    /// `Locomotion/Collision/Knockdown/FromLeft/GetUp`.
    pub(crate) const GET_UP_394: u16 = 394;
    /// `Locomotion/Collision/Knockdown/FromLeft/KnockdownComplete`.
    pub(crate) const KNOCKDOWN_COMPLETE_395: u16 = 395;
    /// `Locomotion/Collision/Knockdown/FromRight/Falling`.
    pub(crate) const FALLING_396: u16 = 396;
    /// `Locomotion/Collision/Knockdown/FromRight/OnTheGround`.
    pub(crate) const ON_THE_GROUND_397: u16 = 397;
    /// `Locomotion/Collision/Knockdown/FromRight/GetUp`.
    pub(crate) const GET_UP_398: u16 = 398;
    /// `Locomotion/Collision/Knockdown/FromRight/KnockdownComplete`.
    pub(crate) const KNOCKDOWN_COMPLETE_399: u16 = 399;
    /// `Locomotion/Collision/Standing/Colliding/FromFront`.
    pub(crate) const FROM_FRONT_400: u16 = 400;
    /// `Locomotion/Collision/Standing/Colliding/FromBack`.
    pub(crate) const FROM_BACK_401: u16 = 401;
    /// `Locomotion/Collision/Standing/Colliding/FromLeft`.
    pub(crate) const FROM_LEFT_402: u16 = 402;
    /// `Locomotion/Collision/Standing/Colliding/FromRight`.
    pub(crate) const FROM_RIGHT_403: u16 = 403;
    /// `Locomotion/Avoid/SitJump/LeaveSit/JumpForward`.
    pub(crate) const JUMP_FORWARD_404: u16 = 404;
    /// `Locomotion/Avoid/SitJump/LeaveSit/JumpLeft`.
    pub(crate) const JUMP_LEFT_405: u16 = 405;
    /// `Locomotion/Avoid/SitJump/LeaveSit/JumpRight`.
    pub(crate) const JUMP_RIGHT_406: u16 = 406;
    /// `Locomotion/Avoid/SitJump/ReturnToSit/StillSitting`.
    pub(crate) const STILL_SITTING_407: u16 = 407;
    /// `Locomotion/Avoid/SitJump/ReturnToSit/NotSittingAnymore`.
    pub(crate) const NOT_SITTING_ANYMORE_408: u16 = 408;
    /// `Locomotion/Avoid/Step/DoStep/StepForward`.
    pub(crate) const STEP_FORWARD_409: u16 = 409;
    /// `Locomotion/Avoid/Step/DoStep/StepLeft`.
    pub(crate) const STEP_LEFT_410: u16 = 410;
    /// `Locomotion/Avoid/Step/DoStep/StepRight`.
    pub(crate) const STEP_RIGHT_411: u16 = 411;
    /// `Locomotion/Avoid/Step/DoStep/StepBack`.
    pub(crate) const STEP_BACK_412: u16 = 412;
    /// `Locomotion/Converse/ConverseFromIdle/ConverseBase/Cycle`.
    pub(crate) const CYCLE_413: u16 = 413;
    /// `Locomotion/UseVendingMachine/UsingVendingMachine/CollectGoods/WaitForVend`.
    pub(crate) const WAIT_FOR_VEND_414: u16 = 414;
    /// `Locomotion/UseVendingMachine/UsingVendingMachine/CollectGoods/VendHandProp`.
    pub(crate) const VEND_HAND_PROP_415: u16 = 415;
    /// `Locomotion/UseNewspaperBox/UsingNewspaperBox/CollectGoods/WaitForHandProp`.
    pub(crate) const WAIT_FOR_HAND_PROP_416: u16 = 416;
    /// `Locomotion/UseNewspaperBox/UsingNewspaperBox/CollectGoods/CollectHandProp`.
    pub(crate) const COLLECT_HAND_PROP_417: u16 = 417;
    /// `Locomotion/Strafe/StrafeFromChase/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_418: u16 = 418;
    /// `Locomotion/Strafe/StrafeFromChase/ReverseRight/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_419: u16 = 419;
    /// `Locomotion/Strafe/StrafeFromChase/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_420: u16 = 420;
    /// `Locomotion/Strafe/StrafeFromChase/ReverseLeft/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_421: u16 = 421;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_422: u16 = 422;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_423: u16 = 423;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_424: u16 = 424;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_425: u16 = 425;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_426: u16 = 426;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_427: u16 = 427;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_428: u16 = 428;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_429: u16 = 429;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_430: u16 = 430;
    /// `Locomotion/Strafe/StrafeFromChase/TurnRight45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_431: u16 = 431;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_432: u16 = 432;
    /// `Locomotion/Strafe/StrafeFromChase/TurnLeft45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_433: u16 = 433;
    /// `Locomotion/Strafe/StrafeFromChase/StrafeFromChaseDefault/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_434: u16 = 434;
    /// `Locomotion/Strafe/StrafeFromChase/StrafeFromChaseDefault/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_435: u16 = 435;
    /// `Locomotion/Strafe/StrafeMotion/MoveForwards/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_436: u16 = 436;
    /// `Locomotion/Strafe/StrafeMotion/MoveForwards/Cycle`.
    pub(crate) const CYCLE_437: u16 = 437;
    /// `Locomotion/Strafe/StrafeMotion/MoveBackwards/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_438: u16 = 438;
    /// `Locomotion/Strafe/StrafeMotion/MoveBackwards/Cycle`.
    pub(crate) const CYCLE_439: u16 = 439;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_440: u16 = 440;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft45/Cycle`.
    pub(crate) const CYCLE_441: u16 = 441;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_442: u16 = 442;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight45/Cycle`.
    pub(crate) const CYCLE_443: u16 = 443;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_444: u16 = 444;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft90/Cycle`.
    pub(crate) const CYCLE_445: u16 = 445;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_446: u16 = 446;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight90/Cycle`.
    pub(crate) const CYCLE_447: u16 = 447;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_448: u16 = 448;
    /// `Locomotion/Strafe/StrafeMotion/MoveLeft135/Cycle`.
    pub(crate) const CYCLE_449: u16 = 449;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_450: u16 = 450;
    /// `Locomotion/Strafe/StrafeMotion/MoveRight135/Cycle`.
    pub(crate) const CYCLE_451: u16 = 451;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_452: u16 = 452;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ReverseRight/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_453: u16 = 453;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_454: u16 = 454;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ReverseLeft/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_455: u16 = 455;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_456: u16 = 456;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_457: u16 = 457;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_458: u16 = 458;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_459: u16 = 459;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_460: u16 = 460;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_461: u16 = 461;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_462: u16 = 462;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_463: u16 = 463;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_464: u16 = 464;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnRight45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_465: u16 = 465;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_466: u16 = 466;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/TurnLeft45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_467: u16 = 467;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ChaseIntoDefault/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_468: u16 = 468;
    /// `Locomotion/ChaseRun/ChaseRunFromStrafe/ChaseIntoDefault/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_469: u16 = 469;
    /// `Locomotion/ChaseRun/ChaseRunInto/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_470: u16 = 470;
    /// `Locomotion/ChaseRun/ChaseRunInto/ReverseRight/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_471: u16 = 471;
    /// `Locomotion/ChaseRun/ChaseRunInto/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_472: u16 = 472;
    /// `Locomotion/ChaseRun/ChaseRunInto/ReverseLeft/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_473: u16 = 473;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_474: u16 = 474;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_475: u16 = 475;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_476: u16 = 476;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_477: u16 = 477;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_478: u16 = 478;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_479: u16 = 479;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_480: u16 = 480;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_481: u16 = 481;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_482: u16 = 482;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnRight45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_483: u16 = 483;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_484: u16 = 484;
    /// `Locomotion/ChaseRun/ChaseRunInto/TurnLeft45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_485: u16 = 485;
    /// `Locomotion/ChaseRun/ChaseRunInto/ChaseIntoDefault/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_486: u16 = 486;
    /// `Locomotion/ChaseRun/ChaseRunInto/ChaseIntoDefault/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_487: u16 = 487;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Slow/WaitForBranch`.
    pub(crate) const WAIT_FOR_BRANCH_488: u16 = 488;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Slow/SlowCyc`.
    pub(crate) const SLOW_CYC_489: u16 = 489;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Fast/WaitForBranch`.
    pub(crate) const WAIT_FOR_BRANCH_490: u16 = 490;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Fast/FastCyc`.
    pub(crate) const FAST_CYC_491: u16 = 491;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Default/WaitForBranch`.
    pub(crate) const WAIT_FOR_BRANCH_492: u16 = 492;
    /// `Locomotion/ChaseRun/ChaseRunCyc/Default/DefaultCyc`.
    pub(crate) const DEFAULT_CYC_493: u16 = 493;
    /// `Locomotion/Shuffle/ShuffleInto/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_494: u16 = 494;
    /// `Locomotion/Shuffle/ShuffleInto/ReverseRight/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_495: u16 = 495;
    /// `Locomotion/Shuffle/ShuffleInto/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_496: u16 = 496;
    /// `Locomotion/Shuffle/ShuffleInto/ReverseLeft/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_497: u16 = 497;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_498: u16 = 498;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_499: u16 = 499;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_500: u16 = 500;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_501: u16 = 501;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_502: u16 = 502;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_503: u16 = 503;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_504: u16 = 504;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_505: u16 = 505;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_506: u16 = 506;
    /// `Locomotion/Shuffle/ShuffleInto/TurnRight45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_507: u16 = 507;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_508: u16 = 508;
    /// `Locomotion/Shuffle/ShuffleInto/TurnLeft45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_509: u16 = 509;
    /// `Locomotion/Shuffle/ShuffleInto/ShuffleIntoDefault/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_510: u16 = 510;
    /// `Locomotion/Shuffle/ShuffleInto/ShuffleIntoDefault/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_511: u16 = 511;
    /// `Locomotion/Idle/IdleInto/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_512: u16 = 512;
    /// `Locomotion/Idle/IdleInto/ReverseRight/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_513: u16 = 513;
    /// `Locomotion/Idle/IdleInto/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_514: u16 = 514;
    /// `Locomotion/Idle/IdleInto/ReverseLeft/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_515: u16 = 515;
    /// `Locomotion/Idle/IdleInto/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_516: u16 = 516;
    /// `Locomotion/Idle/IdleInto/TurnRight135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_517: u16 = 517;
    /// `Locomotion/Idle/IdleInto/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_518: u16 = 518;
    /// `Locomotion/Idle/IdleInto/TurnLeft135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_519: u16 = 519;
    /// `Locomotion/Idle/IdleInto/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_520: u16 = 520;
    /// `Locomotion/Idle/IdleInto/TurnRight90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_521: u16 = 521;
    /// `Locomotion/Idle/IdleInto/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_522: u16 = 522;
    /// `Locomotion/Idle/IdleInto/TurnLeft90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_523: u16 = 523;
    /// `Locomotion/Idle/IdleInto/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_524: u16 = 524;
    /// `Locomotion/Idle/IdleInto/TurnRight45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_525: u16 = 525;
    /// `Locomotion/Idle/IdleInto/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_526: u16 = 526;
    /// `Locomotion/Idle/IdleInto/TurnLeft45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_527: u16 = 527;
    /// `Locomotion/Idle/IdleInto/WalkToIdle/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_528: u16 = 528;
    /// `Locomotion/Idle/IdleInto/WalkToIdle/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_529: u16 = 529;
    /// `Locomotion/Idle/IdleInto/ShuffleToIdle/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_530: u16 = 530;
    /// `Locomotion/Idle/IdleInto/ShuffleToIdle/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_531: u16 = 531;
    /// `Locomotion/Idle/IdleInto/ChaseRunToIdle/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_532: u16 = 532;
    /// `Locomotion/Idle/IdleInto/ChaseRunToIdle/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_533: u16 = 533;
    /// `Locomotion/Idle/IdleInto/StrafeToIdle/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_534: u16 = 534;
    /// `Locomotion/Idle/IdleInto/StrafeToIdle/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_535: u16 = 535;
    /// `Locomotion/Walk/WalkInto/ReverseRight/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_536: u16 = 536;
    /// `Locomotion/Walk/WalkInto/ReverseRight/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_537: u16 = 537;
    /// `Locomotion/Walk/WalkInto/ReverseLeft/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_538: u16 = 538;
    /// `Locomotion/Walk/WalkInto/ReverseLeft/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_539: u16 = 539;
    /// `Locomotion/Walk/WalkInto/TurnRight135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_540: u16 = 540;
    /// `Locomotion/Walk/WalkInto/TurnRight135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_541: u16 = 541;
    /// `Locomotion/Walk/WalkInto/TurnLeft135/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_542: u16 = 542;
    /// `Locomotion/Walk/WalkInto/TurnLeft135/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_543: u16 = 543;
    /// `Locomotion/Walk/WalkInto/TurnRight90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_544: u16 = 544;
    /// `Locomotion/Walk/WalkInto/TurnRight90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_545: u16 = 545;
    /// `Locomotion/Walk/WalkInto/TurnLeft90/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_546: u16 = 546;
    /// `Locomotion/Walk/WalkInto/TurnLeft90/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_547: u16 = 547;
    /// `Locomotion/Walk/WalkInto/TurnRight45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_548: u16 = 548;
    /// `Locomotion/Walk/WalkInto/TurnRight45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_549: u16 = 549;
    /// `Locomotion/Walk/WalkInto/TurnLeft45/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_550: u16 = 550;
    /// `Locomotion/Walk/WalkInto/TurnLeft45/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_551: u16 = 551;
    /// `Locomotion/Walk/WalkInto/WalkIntoDefault/WaitForBranchFlag`.
    pub(crate) const WAIT_FOR_BRANCH_FLAG_552: u16 = 552;
    /// `Locomotion/Walk/WalkInto/WalkIntoDefault/PlayTransition`.
    pub(crate) const PLAY_TRANSITION_553: u16 = 553;
    /// `Locomotion/Walk/WalkCyc/Brisk/WaitForBranch`.
    pub(crate) const WAIT_FOR_BRANCH_554: u16 = 554;
    /// `Locomotion/Walk/WalkCyc/Brisk/BriskCyc`.
    pub(crate) const BRISK_CYC_555: u16 = 555;
    /// `Locomotion/Walk/WalkCyc/Default/WaitForBranch`.
    pub(crate) const WAIT_FOR_BRANCH_556: u16 = 556;
    /// `Locomotion/Walk/WalkCyc/Default/DefaultCyc`.
    pub(crate) const DEFAULT_CYC_557: u16 = 557;
    /// `Locomotion/Collision/Knockdown/FromFront/Falling/LyingDown`.
    pub(crate) const LYING_DOWN_558: u16 = 558;
    /// `Locomotion/Collision/Knockdown/FromFront/Falling/Crouched`.
    pub(crate) const CROUCHED_559: u16 = 559;
    /// `Locomotion/Collision/Knockdown/FromFront/Falling/FromStand`.
    pub(crate) const FROM_STAND_560: u16 = 560;
    /// `Locomotion/Collision/Knockdown/FromBack/Falling/LyingDown`.
    pub(crate) const LYING_DOWN_561: u16 = 561;
    /// `Locomotion/Collision/Knockdown/FromBack/Falling/Crouched`.
    pub(crate) const CROUCHED_562: u16 = 562;
    /// `Locomotion/Collision/Knockdown/FromBack/Falling/FromStand`.
    pub(crate) const FROM_STAND_563: u16 = 563;
    /// `Locomotion/Collision/Knockdown/FromLeft/Falling/LyingDown`.
    pub(crate) const LYING_DOWN_564: u16 = 564;
    /// `Locomotion/Collision/Knockdown/FromLeft/Falling/Crouched`.
    pub(crate) const CROUCHED_565: u16 = 565;
    /// `Locomotion/Collision/Knockdown/FromLeft/Falling/FromStand`.
    pub(crate) const FROM_STAND_566: u16 = 566;
    /// `Locomotion/Collision/Knockdown/FromRight/Falling/LyingDown`.
    pub(crate) const LYING_DOWN_567: u16 = 567;
    /// `Locomotion/Collision/Knockdown/FromRight/Falling/Crouched`.
    pub(crate) const CROUCHED_568: u16 = 568;
    /// `Locomotion/Collision/Knockdown/FromRight/Falling/FromStand`.
    pub(crate) const FROM_STAND_569: u16 = 569;
    /// `Locomotion/Avoid/SitJump/ReturnToSit/StillSitting/JumpBack`.
    pub(crate) const JUMP_BACK_570: u16 = 570;
    /// `Locomotion/Avoid/SitJump/ReturnToSit/NotSittingAnymore/JumpForward`.
    pub(crate) const JUMP_FORWARD_571: u16 = 571;
}
