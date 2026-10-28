//! The `HCDelegate_Pedestrian` state graph's ids under names from its own data, written by
//! `tools/generate_graph_ids.py`: conditions by class and attributes, expressions by the
//! states and transitions that evaluate them, states by name and index.
#![allow(dead_code, reason = "the complete graph; code names the ids it reads")]

/// Condition declarations: what each condition index tests.
pub(crate) mod condition {
    /// `HasPlugin`.
    pub(crate) const HAS_PLUGIN: u16 = 0;
    /// `IsPatrolling`.
    pub(crate) const IS_PATROLLING: u16 = 1;
    /// `IsPedestrianColliding`.
    pub(crate) const IS_PEDESTRIAN_COLLIDING: u16 = 2;
    /// `PatrolZoneOwnerIsInOwnZone`.
    pub(crate) const PATROL_ZONE_OWNER_IS_IN_OWN_ZONE: u16 = 3;
    /// `NeedsToBeginColliding`.
    pub(crate) const NEEDS_TO_BEGIN_COLLIDING: u16 = 4;
    /// `EntityOfInterestIsInMaintainInterestRadius`.
    pub(crate) const ENTITY_OF_INTEREST_IS_IN_MAINTAIN_INTEREST_RADIUS: u16 = 5;
    /// `PatrolZoneMaintainInterestTimerActive`.
    pub(crate) const PATROL_ZONE_MAINTAIN_INTEREST_TIMER_ACTIVE: u16 = 6;
    /// `IsOnRoad`.
    pub(crate) const IS_ON_ROAD: u16 = 7;
    /// `IsOnIntersection`.
    pub(crate) const IS_ON_INTERSECTION: u16 = 8;
    /// `IsBeingHonkedAt`.
    pub(crate) const IS_BEING_HONKED_AT: u16 = 9;
    /// `HasRoadWanderTarget`.
    pub(crate) const HAS_ROAD_WANDER_TARGET: u16 = 10;
    /// `IsEnteringCrossWalk`.
    pub(crate) const IS_ENTERING_CROSS_WALK: u16 = 11;
    /// `WalkSignSaysGo`.
    pub(crate) const WALK_SIGN_SAYS_GO: u16 = 12;
    /// `IsPluginCollisionOverride`.
    pub(crate) const IS_PLUGIN_COLLISION_OVERRIDE: u16 = 13;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = warn)`.
    pub(crate) const HAS_WANT_WARN: u16 = 14;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = angrychase)`.
    pub(crate) const HAS_WANT_ANGRY_CHASE: u16 = 15;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = joinchase)`.
    pub(crate) const HAS_WANT_JOIN_CHASE: u16 = 16;
    /// `IsPatrolling`, declared again: the test of `IS_PATROLLING` under its own index.
    pub(crate) const IS_PATROLLING_2ND: u16 = 17;
    /// `IsTargetingWaypoint`.
    pub(crate) const IS_TARGETING_WAYPOINT: u16 = 18;
    /// `IsChasing`.
    pub(crate) const IS_CHASING: u16 = 19;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = nearbycollisionreaction)`.
    pub(crate) const HAS_WANT_NEARBY_COLLISION_REACTION: u16 = 20;
    /// `SimpleTimerExpired`.
    pub(crate) const SIMPLE_TIMER_EXPIRED: u16 = 21;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = slamreaction)`.
    pub(crate) const HAS_WANT_SLAM_REACTION: u16 = 22;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = warn)`, declared again: the test of
    /// `HAS_WANT_WARN` under its own index.
    pub(crate) const HAS_WANT_WARN_2ND: u16 = 23;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = throwhandprop)`.
    pub(crate) const HAS_WANT_THROW_HAND_PROP: u16 = 24;
    /// `HasHandProp`.
    pub(crate) const HAS_HAND_PROP: u16 = 25;
    /// `CanAttackThrowHandProp`.
    pub(crate) const CAN_ATTACK_THROW_HAND_PROP: u16 = 26;
    /// `IsFacingWantTarget(angle = 120, distance = 20, want = throwhandprop)`.
    pub(crate) const IS_FACING_WANT_TARGET_THROW_HAND_PROP_ANGLE_120_DISTANCE_20: u16 = 27;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = greet)`.
    pub(crate) const HAS_WANT_GREET: u16 = 28;
    /// `DistanceToWantTarget(less = 3.5, want = greet)`.
    pub(crate) const DISTANCE_TO_WANT_TARGET_GREET_LESS_3_5: u16 = 29;
    /// `DistanceToWantTarget(less = 5.0, want = greet)`.
    pub(crate) const DISTANCE_TO_WANT_TARGET_GREET_LESS_5: u16 = 30;
    /// `SimpleTimerExpired(timerName = InterestTimer)`.
    pub(crate) const SIMPLE_TIMER_EXPIRED_INTEREST_TIMER: u16 = 31;
    /// `SimpleTimerExpired`, declared again: the test of `SIMPLE_TIMER_EXPIRED` under its own
    /// index.
    pub(crate) const SIMPLE_TIMER_EXPIRED_2ND: u16 = 32;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = returngreet)`.
    pub(crate) const HAS_WANT_RETURN_GREET: u16 = 33;
    /// `DistanceToWantTarget(less = 3.5, want = returngreet)`.
    pub(crate) const DISTANCE_TO_WANT_TARGET_RETURN_GREET_LESS_3_5: u16 = 34;
    /// `DistanceToWantTarget(less = 5.0, want = returngreet)`.
    pub(crate) const DISTANCE_TO_WANT_TARGET_RETURN_GREET_LESS_5: u16 = 35;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = startconversation)`.
    pub(crate) const HAS_WANT_START_CONVERSATION: u16 = 36;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = taunt)`.
    pub(crate) const HAS_WANT_TAUNT: u16 = 37;
    /// `HasMonitoredIntent(intentName = SGIntent)`.
    pub(crate) const HAS_MONITORED_INTENT_SG_INTENT: u16 = 38;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = taze)`.
    pub(crate) const HAS_WANT_TAZE: u16 = 39;
    /// `HasHandProp`, declared again: the test of `HAS_HAND_PROP` under its own index.
    pub(crate) const HAS_HAND_PROP_2ND: u16 = 40;
    /// `IsHoldingSpecificHandProp(handprop = tazr)`.
    pub(crate) const IS_HOLDING_SPECIFIC_HAND_PROP_TAZR: u16 = 41;
    /// `IsFacingWantTarget(angle = 20.0, distance = 10.0, want = taze)`.
    pub(crate) const IS_FACING_WANT_TARGET_TAZE_ANGLE_20_DISTANCE_10: u16 = 42;
    /// `HasLineOfSightToTazeTarget`.
    pub(crate) const HAS_LINE_OF_SIGHT_TO_TAZE_TARGET: u16 = 43;
    /// `HasDrawnTazer`.
    pub(crate) const HAS_DRAWN_TAZER: u16 = 44;
    /// `SimpleTimerExpired(timerName = TazerCycTime)`.
    pub(crate) const SIMPLE_TIMER_EXPIRED_TAZER_CYC_TIME: u16 = 45;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = angrychase)`, declared again: the test of
    /// `HAS_WANT_ANGRY_CHASE` under its own index.
    pub(crate) const HAS_WANT_ANGRY_CHASE_2ND: u16 = 46;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = joinchase)`, declared again: the test of
    /// `HAS_WANT_JOIN_CHASE` under its own index.
    pub(crate) const HAS_WANT_JOIN_CHASE_2ND: u16 = 47;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = alertto)`.
    pub(crate) const HAS_WANT_ALERT_TO: u16 = 48;
    /// `IsWantTargetWithinAlertDistance(want = alertto)`.
    pub(crate) const IS_WANT_TARGET_WITHIN_ALERT_DISTANCE_ALERT_TO: u16 = 49;
    /// `SimpleTimerExpired(timerName = AlertTimer)`.
    pub(crate) const SIMPLE_TIMER_EXPIRED_ALERT_TIMER: u16 = 50;
    /// `EntityOfInterestIsInZoneBoundaryOrBuffer`.
    pub(crate) const ENTITY_OF_INTEREST_IS_IN_ZONE_BOUNDARY_OR_BUFFER: u16 = 51;
    /// `PatrolZoneOwnerIsInOwnZoneBoundaryOrBuffer`.
    pub(crate) const PATROL_ZONE_OWNER_IS_IN_OWN_ZONE_BOUNDARY_OR_BUFFER: u16 = 52;
    /// `DistanceToWantTarget(less = 4.5, want = alertto)`.
    pub(crate) const DISTANCE_TO_WANT_TARGET_ALERT_TO_LESS_4_5: u16 = 53;
    /// `DistanceToWantTarget(less = 7.5, want = alertto)`.
    pub(crate) const DISTANCE_TO_WANT_TARGET_ALERT_TO_LESS_7_5: u16 = 54;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = flee)`.
    pub(crate) const HAS_WANT_FLEE: u16 = 55;
    /// `DistanceToWantTarget(greater = 15.0, want = flee)`.
    pub(crate) const DISTANCE_TO_WANT_TARGET_FLEE_GREATER_15: u16 = 56;
    /// `IsChasing`, declared again: the test of `IS_CHASING` under its own index.
    pub(crate) const IS_CHASING_2ND: u16 = 57;
    /// `CanNewChaseStart`.
    pub(crate) const CAN_NEW_CHASE_START: u16 = 58;
    /// `CanChaseeAddNewChaser`.
    pub(crate) const CAN_CHASEE_ADD_NEW_CHASER: u16 = 59;
    /// `IsFacingWantTarget(angle = 120, distance = 20, want = angrychase)`.
    pub(crate) const IS_FACING_WANT_TARGET_ANGRY_CHASE_ANGLE_120_DISTANCE_20: u16 = 60;
    /// `SimpleTimerExpired(timerName = StartChase)`.
    pub(crate) const SIMPLE_TIMER_EXPIRED_START_CHASE: u16 = 61;
    /// `HasTakeDownTargetable`.
    pub(crate) const HAS_TAKE_DOWN_TARGETABLE: u16 = 62;
    /// `CanAttemptTakeDownTargetable`.
    pub(crate) const CAN_ATTEMPT_TAKE_DOWN_TARGETABLE: u16 = 63;
    /// `IsChaserEndingChase`.
    pub(crate) const IS_CHASER_ENDING_CHASE: u16 = 64;
    /// `ChaseeIsWipedOut`.
    pub(crate) const CHASEE_IS_WIPED_OUT: u16 = 65;
    /// `HasMonitoredIntent(intentName = ActiveTakedown)`.
    pub(crate) const HAS_MONITORED_INTENT_ACTIVE_TAKEDOWN: u16 = 66;
    /// `TakeDownAttemptSuccessful`.
    pub(crate) const TAKE_DOWN_ATTEMPT_SUCCESSFUL: u16 = 67;
    /// `TakeDownAttemptFailed`.
    pub(crate) const TAKE_DOWN_ATTEMPT_FAILED: u16 = 68;
    /// `SimpleTimerExpired(timerName = AttemptTakeDownTimeout)`.
    pub(crate) const SIMPLE_TIMER_EXPIRED_ATTEMPT_TAKE_DOWN_TIMEOUT: u16 = 69;
    /// `ChaserShouldGiveUpDueToTakedowns`.
    pub(crate) const CHASER_SHOULD_GIVE_UP_DUE_TO_TAKEDOWNS: u16 = 70;
    /// `IsChaserEndingChase`, declared again: the test of `IS_CHASER_ENDING_CHASE` under its own
    /// index.
    pub(crate) const IS_CHASER_ENDING_CHASE_2ND: u16 = 71;
    /// `ChaseeEscaped`.
    pub(crate) const CHASEE_ESCAPED: u16 = 72;
    /// `ChaseeHasProtector`.
    pub(crate) const CHASEE_HAS_PROTECTOR: u16 = 73;
    /// `ChaseeCloseEnoughToSatisfyAggressiveChase`.
    pub(crate) const CHASEE_CLOSE_ENOUGH_TO_SATISFY_AGGRESSIVE_CHASE: u16 = 74;
    /// `ChaseeIsWipedOut`, declared again: the test of `CHASEE_IS_WIPED_OUT` under its own index.
    pub(crate) const CHASEE_IS_WIPED_OUT_2ND: u16 = 75;
    /// `CannotReachChaseTarget`.
    pub(crate) const CANNOT_REACH_CHASE_TARGET: u16 = 76;
    /// `SimpleTimerExpired(timerName = TargetUnreachableTimer)`.
    pub(crate) const SIMPLE_TIMER_EXPIRED_TARGET_UNREACHABLE_TIMER: u16 = 77;
    /// `CannotReachChaseTarget`, declared again: the test of `CANNOT_REACH_CHASE_TARGET` under its
    /// own index.
    pub(crate) const CANNOT_REACH_CHASE_TARGET_2ND: u16 = 78;
    /// `CanTazeUnreachableTargets`.
    pub(crate) const CAN_TAZE_UNREACHABLE_TARGETS: u16 = 79;
    /// `IsFacingWantTarget(angle = 20.0, distance = 15.0, want = angrychase)`.
    pub(crate) const IS_FACING_WANT_TARGET_ANGRY_CHASE_ANGLE_20_DISTANCE_15: u16 = 80;
    /// `IsEndChaseReason(reason = aggressivecapture)`.
    pub(crate) const IS_END_CHASE_REASON_AGGRESSIVE_CAPTURE: u16 = 81;
    /// `NeedToRest`.
    pub(crate) const NEED_TO_REST: u16 = 82;
    /// `CanSeeChasee`.
    pub(crate) const CAN_SEE_CHASEE: u16 = 83;
    /// `InvestigateTimeExceeded`.
    pub(crate) const INVESTIGATE_TIME_EXCEEDED: u16 = 84;
    /// `IsPrimaryChaser`.
    pub(crate) const IS_PRIMARY_CHASER: u16 = 85;
    /// `ChasersAreScared`.
    pub(crate) const CHASERS_ARE_SCARED: u16 = 86;
    /// `ChaserShouldBlock`.
    pub(crate) const CHASER_SHOULD_BLOCK: u16 = 87;
    /// `IsEndChaseReason(reason = lostinterest)`.
    pub(crate) const IS_END_CHASE_REASON_LOST_INTEREST: u16 = 88;
}

/// Expressions: which states and transitions evaluate each expression index.
pub(crate) mod expression {
    /// Shared by 3: `InitialState2` -> `Plugin8`, `Wander5` -> `Plugin8`, `Patrol9` -> `Plugin8`.
    pub(crate) const TO_PLUGIN: u16 = 0;
    /// Shared by 2: `InitialState2` -> `Patrol9`, `Wander5` -> `Patrol9`.
    pub(crate) const TO_PATROL: u16 = 1;
    /// Shared by 5: `Colliding3` -> `Wander5`, `PatrolZoneBoundary4` -> `PedestrianAddressWants10`,
    /// `Wander5` -> `PedestrianAddressWants10`, `WanderOnRoad6` -> `PedestrianAddressWants10` and 1
    /// more.
    pub(crate) const NOT_IS_PEDESTRIAN_COLLIDING: u16 = 2;
    /// State `PatrolZoneBoundary4`.
    pub(crate) const PATROL_ZONE_BOUNDARY_4: u16 = 3;
    /// Shared by 7: `PatrolZoneBoundary4` -> `Colliding3`, `Wander5` -> `Colliding3`,
    /// `WanderOnRoad6` -> `Colliding3`, `Patrol9` -> `Colliding3` and 3 more.
    pub(crate) const TO_COLLIDING: u16 = 4;
    /// State `ApproachZonePointOfInterest12`.
    pub(crate) const APPROACH_ZONE_POINT_OF_INTEREST_12: u16 = 5;
    /// `WaitOutMaintainInterestTimer13` -> `ReturnToPatrolZone14`.
    pub(crate) const WAIT_OUT_MAINTAIN_INTEREST_TIMER_13_TO_RETURN_TO_PATROL_ZONE_14: u16 = 6;
    /// `Wander5` -> `RunFromHonker7`.
    pub(crate) const WANDER_5_TO_RUN_FROM_HONKER_7: u16 = 7;
    /// State `Road22`.
    pub(crate) const ROAD_22: u16 = 8;
    /// State `UseCrossWalk40`.
    pub(crate) const USE_CROSS_WALK_40: u16 = 9;
    /// State `Wait71`.
    pub(crate) const WAIT_71: u16 = 10;
    /// Shared by 2: `WanderOnRoad6` -> `Wander5`, `RunFromHonker7` -> `Wander5`.
    pub(crate) const TO_WANDER: u16 = 11;
    /// `Plugin8` -> `Wander5`.
    pub(crate) const PLUGIN_8_TO_WANDER_5: u16 = 12;
    /// State `Patrol9`.
    pub(crate) const PATROL_9: u16 = 13;
    /// `Patrol9` -> `Wander5`.
    pub(crate) const PATROL_9_TO_WANDER_5: u16 = 14;
    /// State `ResumingActiveChase16`.
    pub(crate) const RESUMING_ACTIVE_CHASE_16: u16 = 15;
    /// State `PedestrianWantToWarn25`.
    pub(crate) const PEDESTRIAN_WANT_TO_WARN_25: u16 = 16;
    /// State `ThrowHandProp45`.
    pub(crate) const THROW_HAND_PROP_45: u16 = 17;
    /// State `FacingTarget100`.
    pub(crate) const FACING_TARGET_100: u16 = 18;
    /// State `GreetPerp46`.
    pub(crate) const GREET_PERP_46: u16 = 19;
    /// `DoGreet75` -> `WatchReturnGreet76`.
    pub(crate) const DO_GREET_75_TO_WATCH_RETURN_GREET_76: u16 = 20;
    /// Shared by 4: `WaitOutTimer104` -> `ExitGreet105`, `WaitOutTimer106` -> `DoGreet107`,
    /// `DrawTazer78` -> `EndTaze80`, `DrawTazer127` -> `EndTaze129`.
    pub(crate) const SIMPLE_TIMER_EXPIRED: u16 = 21;
    /// State `ReturnGreet47`.
    pub(crate) const RETURN_GREET_47: u16 = 22;
    /// State `PedestrianWantToStartConversation28`.
    pub(crate) const PEDESTRIAN_WANT_TO_START_CONVERSATION_28: u16 = 23;
    /// State `PedestrianWantToTaunt29`.
    pub(crate) const PEDESTRIAN_WANT_TO_TAUNT_29: u16 = 24;
    /// State `DoTaunt49`.
    pub(crate) const DO_TAUNT_49: u16 = 25;
    /// State `PedestrianWantToTaze30`.
    pub(crate) const PEDESTRIAN_WANT_TO_TAZE_30: u16 = 26;
    /// Shared by 2: `TazeEntity79` -> `EndTaze80`, `TazeEntity128` -> `EndTaze129`.
    pub(crate) const TAZE_ENTITY_TO_END_TAZE: u16 = 27;
    /// State `PedestrianWantToChasePerp52`.
    pub(crate) const PEDESTRIAN_WANT_TO_CHASE_PERP_52: u16 = 28;
    /// State `PedestrianWantToChaseVictim53`.
    pub(crate) const PEDESTRIAN_WANT_TO_CHASE_VICTIM_53: u16 = 29;
    /// State `PedestrianWantToBecomeAlert32`.
    pub(crate) const PEDESTRIAN_WANT_TO_BECOME_ALERT_32: u16 = 30;
    /// State `Flee33`.
    pub(crate) const FLEE_33: u16 = 31;
    /// State `HasEscaped55`.
    pub(crate) const HAS_ESCAPED_55: u16 = 32;
    /// Shared by 2: state `ResumeChase34`, state `FromChasingEndChase38`.
    pub(crate) const IS_CHASING: u16 = 33;
    /// State `HasHandProp83`.
    pub(crate) const HAS_HAND_PROP_83: u16 = 34;
    /// State `CanThrowHandProp110`.
    pub(crate) const CAN_THROW_HAND_PROP_110: u16 = 35;
    /// State `FacingTarget123`.
    pub(crate) const FACING_TARGET_123: u16 = 36;
    /// `HoldBeforeChaseStart58` -> `StartChaseForReal59`.
    pub(crate) const HOLD_BEFORE_CHASE_START_58_TO_START_CHASE_FOR_REAL_59: u16 = 37;
    /// State `FailedToThrowHandProp85`.
    pub(crate) const FAILED_TO_THROW_HAND_PROP_85: u16 = 38;
    /// State `AttemptTakeDownTargetable87`.
    pub(crate) const ATTEMPT_TAKE_DOWN_TARGETABLE_87: u16 = 39;
    /// `WaitForTakedownDecision112` -> `TakeDownTargetableSuccess114`.
    pub(crate) const WAIT_FOR_TAKEDOWN_DECISION_112_TO_TAKE_DOWN_TARGETABLE_SUCCESS_114: u16 = 40;
    /// Shared by 2: state `TauntAfterTakedown115`, state `TauntAfterTakedown125`.
    pub(crate) const TAUNT_AFTER_TAKEDOWN: u16 = 41;
    /// State `GiveUpDueToTakedownsAutoEndChase88`.
    pub(crate) const GIVE_UP_DUE_TO_TAKEDOWNS_AUTO_END_CHASE_88: u16 = 42;
    /// State `ChaseEndReasonAutoEndChase89`.
    pub(crate) const CHASE_END_REASON_AUTO_END_CHASE_89: u16 = 43;
    /// State `EscapedAutoEndChase90`.
    pub(crate) const ESCAPED_AUTO_END_CHASE_90: u16 = 44;
    /// Shared by 2: state `ReturnToPatrolZoneEndChase68`, state `ReturnToPatrolZoneAutoEndChase92`.
    pub(crate) const NOT_ENTITY_OF_INTEREST_IS_IN_ZONE_BOUNDARY_OR_BUFFER: u16 = 45;
    /// State `NoChaserAutoEndChase93`.
    pub(crate) const NO_CHASER_AUTO_END_CHASE_93: u16 = 46;
    /// State `WarnAndWait94`.
    pub(crate) const WARN_AND_WAIT_94: u16 = 47;
    /// `WarnAndWait94` -> `UnreachableTimerExpired95`.
    pub(crate) const WARN_AND_WAIT_94_TO_UNREACHABLE_TIMER_EXPIRED_95: u16 = 48;
    /// State `UnreachableTimerExpired95`.
    pub(crate) const UNREACHABLE_TIMER_EXPIRED_95: u16 = 49;
    /// State `TazeUnreachableTarget117`.
    pub(crate) const TAZE_UNREACHABLE_TARGET_117: u16 = 50;
    /// `EndTaze129` -> `GiveUpAfterWait118`.
    pub(crate) const END_TAZE_129_TO_GIVE_UP_AFTER_WAIT_118: u16 = 51;
    /// State `ChaserExhausted63`.
    pub(crate) const CHASER_EXHAUSTED_63: u16 = 52;
    /// State `SearchForChasee96`.
    pub(crate) const SEARCH_FOR_CHASEE_96: u16 = 53;
    /// `SearchForChasee96` -> `GiveUpOnLostChasee97`.
    pub(crate) const SEARCH_FOR_CHASEE_96_TO_GIVE_UP_ON_LOST_CHASEE_97: u16 = 54;
    /// State `WithProtector98`.
    pub(crate) const WITH_PROTECTOR_98: u16 = 55;
    /// Shared by 2: state `BlockChasee130`, state `BlockChasee132`.
    pub(crate) const BLOCK_CHASEE: u16 = 56;
    /// State `AggressiveCaptureEndChase67`.
    pub(crate) const AGGRESSIVE_CAPTURE_END_CHASE_67: u16 = 57;
    /// State `LostInterestEndChase69`.
    pub(crate) const LOST_INTEREST_END_CHASE_69: u16 = 58;
    /// `Wander5` -> `WanderOnRoad6`.
    pub(crate) const WANDER_5_TO_WANDER_ON_ROAD_6: u16 = 59;
    /// `Plugin8` -> `Colliding3`.
    pub(crate) const PLUGIN_8_TO_COLLIDING_3: u16 = 60;
    /// State `ObserveCollision42`.
    pub(crate) const OBSERVE_COLLISION_42: u16 = 61;
    /// State `ObserveSlam43`.
    pub(crate) const OBSERVE_SLAM_43: u16 = 62;
    /// State `CanThrowHandProp73`.
    pub(crate) const CAN_THROW_HAND_PROP_73: u16 = 63;
    /// State `WatchGreetTarget102`.
    pub(crate) const WATCH_GREET_TARGET_102: u16 = 64;
    /// State `WatchGreetTarget121`.
    pub(crate) const WATCH_GREET_TARGET_121: u16 = 65;
    /// State `DropHandProp50`.
    pub(crate) const DROP_HAND_PROP_50: u16 = 66;
    /// State `DoAlert81`.
    pub(crate) const DO_ALERT_81: u16 = 67;
    /// `DoAlert81` -> `PatrolZoneBoundary4`.
    pub(crate) const DO_ALERT_81_TO_PATROL_ZONE_BOUNDARY_4: u16 = 68;
    /// State `WatchAlertTarget108`.
    pub(crate) const WATCH_ALERT_TARGET_108: u16 = 69;
    /// State `NewChasee35`.
    pub(crate) const NEW_CHASEE_35: u16 = 70;
    /// State `CancelChase57`.
    pub(crate) const CANCEL_CHASE_57: u16 = 71;
    /// `WaitForTakedownDecision112` -> `TakeDownTargetableFailure113`.
    pub(crate) const WAIT_FOR_TAKEDOWN_DECISION_112_TO_TAKE_DOWN_TARGETABLE_FAILURE_113: u16 = 72;
    /// State `InterceptChasee65`.
    pub(crate) const INTERCEPT_CHASEE_65: u16 = 73;
    /// `Plugin8` -> `PedestrianAddressWants10`.
    pub(crate) const PLUGIN_8_TO_PEDESTRIAN_ADDRESS_WANTS_10: u16 = 74;
    /// `DrawTazer78` -> `TazeEntity79`.
    pub(crate) const DRAW_TAZER_78_TO_TAZE_ENTITY_79: u16 = 75;
    /// State `AggressiveCaptureAutoEndChase91`.
    pub(crate) const AGGRESSIVE_CAPTURE_AUTO_END_CHASE_91: u16 = 76;
    /// `DrawTazer127` -> `TazeEntity128`.
    pub(crate) const DRAW_TAZER_127_TO_TAZE_ENTITY_128: u16 = 77;
    /// State `PedestrianTakeDownChasee60`.
    pub(crate) const PEDESTRIAN_TAKE_DOWN_CHASEE_60: u16 = 78;
}

/// States by name and index, with their path below the root state.
pub(crate) mod state {
    /// `AI`.
    pub(crate) const AI_0: u16 = 0;
    /// `Pedestrian`.
    pub(crate) const PEDESTRIAN_1: u16 = 1;
    /// `Pedestrian/InitialState`.
    pub(crate) const INITIAL_STATE_2: u16 = 2;
    /// `Pedestrian/Colliding`.
    pub(crate) const COLLIDING_3: u16 = 3;
    /// `Pedestrian/PatrolZoneBoundary`.
    pub(crate) const PATROL_ZONE_BOUNDARY_4: u16 = 4;
    /// `Pedestrian/Wander`.
    pub(crate) const WANDER_5: u16 = 5;
    /// `Pedestrian/WanderOnRoad`.
    pub(crate) const WANDER_ON_ROAD_6: u16 = 6;
    /// `Pedestrian/RunFromHonker`.
    pub(crate) const RUN_FROM_HONKER_7: u16 = 7;
    /// `Pedestrian/Plugin`.
    pub(crate) const PLUGIN_8: u16 = 8;
    /// `Pedestrian/Patrol`.
    pub(crate) const PATROL_9: u16 = 9;
    /// `Pedestrian/PedestrianAddressWants`.
    pub(crate) const PEDESTRIAN_ADDRESS_WANTS_10: u16 = 10;
    /// `Pedestrian/ChaseChasee`.
    pub(crate) const CHASE_CHASEE_11: u16 = 11;
    /// `Pedestrian/PatrolZoneBoundary/ApproachZonePointOfInterest`.
    pub(crate) const APPROACH_ZONE_POINT_OF_INTEREST_12: u16 = 12;
    /// `Pedestrian/PatrolZoneBoundary/WaitOutMaintainInterestTimer`.
    pub(crate) const WAIT_OUT_MAINTAIN_INTEREST_TIMER_13: u16 = 13;
    /// `Pedestrian/PatrolZoneBoundary/ReturnToPatrolZone`.
    pub(crate) const RETURN_TO_PATROL_ZONE_14: u16 = 14;
    /// `Pedestrian/Wander/WanderMode`.
    pub(crate) const WANDER_MODE_15: u16 = 15;
    /// `Pedestrian/PedestrianAddressWants/ResumingActiveChase`.
    pub(crate) const RESUMING_ACTIVE_CHASE_16: u16 = 16;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants`.
    pub(crate) const UNINTERRUBTABLE_WANTS_17: u16 = 17;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants`.
    pub(crate) const INTERRUBTABLE_WANTS_18: u16 = 18;
    /// `Pedestrian/ChaseChasee/StartChase`.
    pub(crate) const START_CHASE_19: u16 = 19;
    /// `Pedestrian/ChaseChasee/ActiveChase`.
    pub(crate) const ACTIVE_CHASE_20: u16 = 20;
    /// `Pedestrian/ChaseChasee/EndChase`.
    pub(crate) const END_CHASE_21: u16 = 21;
    /// `Pedestrian/Wander/WanderMode/Road`.
    pub(crate) const ROAD_22: u16 = 22;
    /// `Pedestrian/Wander/WanderMode/NoRoad`.
    pub(crate) const NO_ROAD_23: u16 = 23;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/ObserveNearbyEvent`.
    pub(crate) const OBSERVE_NEARBY_EVENT_24: u16 = 24;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToWarn`.
    pub(crate) const PEDESTRIAN_WANT_TO_WARN_25: u16 = 25;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToThrowHandProp`.
    pub(crate) const PEDESTRIAN_WANT_TO_THROW_HAND_PROP_26: u16 = 26;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet`.
    pub(crate) const PEDESTRIAN_WANT_TO_GREET_27: u16 = 27;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToStartConversation`.
    pub(crate) const PEDESTRIAN_WANT_TO_START_CONVERSATION_28: u16 = 28;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToTaunt`.
    pub(crate) const PEDESTRIAN_WANT_TO_TAUNT_29: u16 = 29;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToTaze`.
    pub(crate) const PEDESTRIAN_WANT_TO_TAZE_30: u16 = 30;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToChase`.
    pub(crate) const PEDESTRIAN_WANT_TO_CHASE_31: u16 = 31;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/PedestrianWantToBecomeAlert`.
    pub(crate) const PEDESTRIAN_WANT_TO_BECOME_ALERT_32: u16 = 32;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/Flee`.
    pub(crate) const FLEE_33: u16 = 33;
    /// `Pedestrian/ChaseChasee/StartChase/ResumeChase`.
    pub(crate) const RESUME_CHASE_34: u16 = 34;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee`.
    pub(crate) const NEW_CHASEE_35: u16 = 35;
    /// `Pedestrian/ChaseChasee/ActiveChase/NonInterruptibleChase`.
    pub(crate) const NON_INTERRUPTIBLE_CHASE_36: u16 = 36;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase`.
    pub(crate) const INTERRUPTIBLE_CHASE_37: u16 = 37;
    /// `Pedestrian/ChaseChasee/EndChase/FromChasingEndChase`.
    pub(crate) const FROM_CHASING_END_CHASE_38: u16 = 38;
    /// `Pedestrian/ChaseChasee/EndChase/ChaseAlreadyEnded`.
    pub(crate) const CHASE_ALREADY_ENDED_39: u16 = 39;
    /// `Pedestrian/Wander/WanderMode/Road/UseCrossWalk`.
    pub(crate) const USE_CROSS_WALK_40: u16 = 40;
    /// `Pedestrian/Wander/WanderMode/Road/FollowRoad`.
    pub(crate) const FOLLOW_ROAD_41: u16 = 41;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/ObserveNearbyEvent/ObserveCollision`.
    pub(crate) const OBSERVE_COLLISION_42: u16 = 42;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/ObserveNearbyEvent/ObserveSlam`.
    pub(crate) const OBSERVE_SLAM_43: u16 = 43;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToWarn/PedestrianDoWarning`.
    pub(crate) const PEDESTRIAN_DO_WARNING_44: u16 = 44;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToThrowHandProp/ThrowHandProp`.
    pub(crate) const THROW_HAND_PROP_45: u16 = 45;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/GreetPerp`.
    pub(crate) const GREET_PERP_46: u16 = 46;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/ReturnGreet`.
    pub(crate) const RETURN_GREET_47: u16 = 47;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToStartConversation/PedestrianDoStartConversation`.
    pub(crate) const PEDESTRIAN_DO_START_CONVERSATION_48: u16 = 48;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToTaunt/DoTaunt`.
    pub(crate) const DO_TAUNT_49: u16 = 49;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToTaze/DropHandProp`.
    pub(crate) const DROP_HAND_PROP_50: u16 = 50;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToTaze/HasTazer`.
    pub(crate) const HAS_TAZER_51: u16 = 51;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToChase/PedestrianWantToChasePerp`.
    pub(crate) const PEDESTRIAN_WANT_TO_CHASE_PERP_52: u16 = 52;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToChase/PedestrianWantToChaseVictim`.
    pub(crate) const PEDESTRIAN_WANT_TO_CHASE_VICTIM_53: u16 = 53;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/PedestrianWantToBecomeAlert/Alert`.
    pub(crate) const ALERT_54: u16 = 54;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/Flee/HasEscaped`.
    pub(crate) const HAS_ESCAPED_55: u16 = 55;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/Flee/Fleeing`.
    pub(crate) const FLEEING_56: u16 = 56;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/CancelChase`.
    pub(crate) const CANCEL_CHASE_57: u16 = 57;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/HoldBeforeChaseStart`.
    pub(crate) const HOLD_BEFORE_CHASE_START_58: u16 = 58;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/StartChaseForReal`.
    pub(crate) const START_CHASE_FOR_REAL_59: u16 = 59;
    /// `Pedestrian/ChaseChasee/ActiveChase/NonInterruptibleChase/PedestrianTakeDownChasee`.
    pub(crate) const PEDESTRIAN_TAKE_DOWN_CHASEE_60: u16 = 60;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase`.
    pub(crate) const AUTO_END_CHASE_61: u16 = 61;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaseTargetUnreachable`.
    pub(crate) const CHASE_TARGET_UNREACHABLE_62: u16 = 62;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaserExhausted`.
    pub(crate) const CHASER_EXHAUSTED_63: u16 = 63;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/LostChasee`.
    pub(crate) const LOST_CHASEE_64: u16 = 64;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/InterceptChasee`.
    pub(crate) const INTERCEPT_CHASEE_65: u16 = 65;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser`.
    pub(crate) const SECONDARY_CHASER_66: u16 = 66;
    /// `Pedestrian/ChaseChasee/EndChase/FromChasingEndChase/AggressiveCaptureEndChase`.
    pub(crate) const AGGRESSIVE_CAPTURE_END_CHASE_67: u16 = 67;
    /// `Pedestrian/ChaseChasee/EndChase/FromChasingEndChase/ReturnToPatrolZoneEndChase`.
    pub(crate) const RETURN_TO_PATROL_ZONE_END_CHASE_68: u16 = 68;
    /// `Pedestrian/ChaseChasee/EndChase/FromChasingEndChase/LostInterestEndChase`.
    pub(crate) const LOST_INTEREST_END_CHASE_69: u16 = 69;
    /// `Pedestrian/ChaseChasee/EndChase/FromChasingEndChase/DefaultEndChase`.
    pub(crate) const DEFAULT_END_CHASE_70: u16 = 70;
    /// `Pedestrian/Wander/WanderMode/Road/UseCrossWalk/Wait`.
    pub(crate) const WAIT_71: u16 = 71;
    /// `Pedestrian/Wander/WanderMode/Road/UseCrossWalk/Walk`.
    pub(crate) const WALK_72: u16 = 72;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToThrowHandProp/ThrowHandProp/CanThrowHandProp`.
    pub(crate) const CAN_THROW_HAND_PROP_73: u16 = 73;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToThrowHandProp/ThrowHandProp/CantThrowHandProp`.
    pub(crate) const CANT_THROW_HAND_PROP_74: u16 = 74;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/GreetPerp/DoGreet`.
    pub(crate) const DO_GREET_75: u16 = 75;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/GreetPerp/WatchReturnGreet`.
    pub(crate) const WATCH_RETURN_GREET_76: u16 = 76;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/ReturnGreet/WatchGreet`.
    pub(crate) const WATCH_GREET_77: u16 = 77;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToTaze/HasTazer/DrawTazer`.
    pub(crate) const DRAW_TAZER_78: u16 = 78;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToTaze/HasTazer/TazeEntity`.
    pub(crate) const TAZE_ENTITY_79: u16 = 79;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToTaze/HasTazer/EndTaze`.
    pub(crate) const END_TAZE_80: u16 = 80;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/PedestrianWantToBecomeAlert/Alert/DoAlert`.
    pub(crate) const DO_ALERT_81: u16 = 81;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/PedestrianWantToBecomeAlert/Alert/AutoEndAlert`.
    pub(crate) const AUTO_END_ALERT_82: u16 = 82;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/HoldBeforeChaseStart/HasHandProp`.
    pub(crate) const HAS_HAND_PROP_83: u16 = 83;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/HoldBeforeChaseStart/NoHandProp`.
    pub(crate) const NO_HAND_PROP_84: u16 = 84;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/StartChaseForReal/FailedToThrowHandProp`.
    pub(crate) const FAILED_TO_THROW_HAND_PROP_85: u16 = 85;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/StartChaseForReal/NoHandProp`.
    pub(crate) const NO_HAND_PROP_86: u16 = 86;
    /// `Pedestrian/ChaseChasee/ActiveChase/NonInterruptibleChase/PedestrianTakeDownChasee/AttemptTakeDownTargetable`.
    pub(crate) const ATTEMPT_TAKE_DOWN_TARGETABLE_87: u16 = 87;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase/GiveUpDueToTakedownsAutoEndChase`.
    pub(crate) const GIVE_UP_DUE_TO_TAKEDOWNS_AUTO_END_CHASE_88: u16 = 88;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase/ChaseEndReasonAutoEndChase`.
    pub(crate) const CHASE_END_REASON_AUTO_END_CHASE_89: u16 = 89;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase/EscapedAutoEndChase`.
    pub(crate) const ESCAPED_AUTO_END_CHASE_90: u16 = 90;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase/AggressiveCaptureAutoEndChase`.
    pub(crate) const AGGRESSIVE_CAPTURE_AUTO_END_CHASE_91: u16 = 91;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase/ReturnToPatrolZoneAutoEndChase`.
    pub(crate) const RETURN_TO_PATROL_ZONE_AUTO_END_CHASE_92: u16 = 92;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase/NoChaserAutoEndChase`.
    pub(crate) const NO_CHASER_AUTO_END_CHASE_93: u16 = 93;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaseTargetUnreachable/WarnAndWait`.
    pub(crate) const WARN_AND_WAIT_94: u16 = 94;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaseTargetUnreachable/UnreachableTimerExpired`.
    pub(crate) const UNREACHABLE_TIMER_EXPIRED_95: u16 = 95;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/LostChasee/SearchForChasee`.
    pub(crate) const SEARCH_FOR_CHASEE_96: u16 = 96;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/LostChasee/GiveUpOnLostChasee`.
    pub(crate) const GIVE_UP_ON_LOST_CHASEE_97: u16 = 97;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser/WithProtector`.
    pub(crate) const WITH_PROTECTOR_98: u16 = 98;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser/WithoutProtector`.
    pub(crate) const WITHOUT_PROTECTOR_99: u16 = 99;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToThrowHandProp/ThrowHandProp/CanThrowHandProp/FacingTarget`.
    pub(crate) const FACING_TARGET_100: u16 = 100;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToThrowHandProp/ThrowHandProp/CanThrowHandProp/NotFacingTargetWantTarget`.
    pub(crate) const NOT_FACING_TARGET_WANT_TARGET_101: u16 = 101;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/GreetPerp/DoGreet/WatchGreetTarget`.
    pub(crate) const WATCH_GREET_TARGET_102: u16 = 102;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/GreetPerp/DoGreet/ApproachGreetTarget`.
    pub(crate) const APPROACH_GREET_TARGET_103: u16 = 103;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/GreetPerp/WatchReturnGreet/WaitOutTimer`.
    pub(crate) const WAIT_OUT_TIMER_104: u16 = 104;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/GreetPerp/WatchReturnGreet/ExitGreet`.
    pub(crate) const EXIT_GREET_105: u16 = 105;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/ReturnGreet/WatchGreet/WaitOutTimer`.
    pub(crate) const WAIT_OUT_TIMER_106: u16 = 106;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/ReturnGreet/WatchGreet/DoGreet`.
    pub(crate) const DO_GREET_107: u16 = 107;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/PedestrianWantToBecomeAlert/Alert/DoAlert/WatchAlertTarget`.
    pub(crate) const WATCH_ALERT_TARGET_108: u16 = 108;
    /// `Pedestrian/PedestrianAddressWants/InterrubtableWants/PedestrianWantToBecomeAlert/Alert/DoAlert/ApproachAlertTarget`.
    pub(crate) const APPROACH_ALERT_TARGET_109: u16 = 109;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/HoldBeforeChaseStart/HasHandProp/CanThrowHandProp`.
    pub(crate) const CAN_THROW_HAND_PROP_110: u16 = 110;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/HoldBeforeChaseStart/HasHandProp/CantThrowHandProp`.
    pub(crate) const CANT_THROW_HAND_PROP_111: u16 = 111;
    /// `Pedestrian/ChaseChasee/ActiveChase/NonInterruptibleChase/PedestrianTakeDownChasee/AttemptTakeDownTargetable/WaitForTakedownDecision`.
    pub(crate) const WAIT_FOR_TAKEDOWN_DECISION_112: u16 = 112;
    /// `Pedestrian/ChaseChasee/ActiveChase/NonInterruptibleChase/PedestrianTakeDownChasee/AttemptTakeDownTargetable/TakeDownTargetableFailure`.
    pub(crate) const TAKE_DOWN_TARGETABLE_FAILURE_113: u16 = 113;
    /// `Pedestrian/ChaseChasee/ActiveChase/NonInterruptibleChase/PedestrianTakeDownChasee/AttemptTakeDownTargetable/TakeDownTargetableSuccess`.
    pub(crate) const TAKE_DOWN_TARGETABLE_SUCCESS_114: u16 = 114;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase/AggressiveCaptureAutoEndChase/TauntAfterTakedown`.
    pub(crate) const TAUNT_AFTER_TAKEDOWN_115: u16 = 115;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/AutoEndChase/AggressiveCaptureAutoEndChase/NoTaunt`.
    pub(crate) const NO_TAUNT_116: u16 = 116;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaseTargetUnreachable/UnreachableTimerExpired/TazeUnreachableTarget`.
    pub(crate) const TAZE_UNREACHABLE_TARGET_117: u16 = 117;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaseTargetUnreachable/UnreachableTimerExpired/GiveUpAfterWait`.
    pub(crate) const GIVE_UP_AFTER_WAIT_118: u16 = 118;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser/WithProtector/FormationChaser`.
    pub(crate) const FORMATION_CHASER_119: u16 = 119;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser/WithoutProtector/FormationChaser`.
    pub(crate) const FORMATION_CHASER_120: u16 = 120;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/ReturnGreet/WatchGreet/WaitOutTimer/WatchGreetTarget`.
    pub(crate) const WATCH_GREET_TARGET_121: u16 = 121;
    /// `Pedestrian/PedestrianAddressWants/UninterrubtableWants/PedestrianWantToGreet/ReturnGreet/WatchGreet/WaitOutTimer/ApproachGreetTarget`.
    pub(crate) const APPROACH_GREET_TARGET_122: u16 = 122;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/HoldBeforeChaseStart/HasHandProp/CanThrowHandProp/FacingTarget`.
    pub(crate) const FACING_TARGET_123: u16 = 123;
    /// `Pedestrian/ChaseChasee/StartChase/NewChasee/HoldBeforeChaseStart/HasHandProp/CanThrowHandProp/NotFacingTargetWantTarget`.
    pub(crate) const NOT_FACING_TARGET_WANT_TARGET_124: u16 = 124;
    /// `Pedestrian/ChaseChasee/ActiveChase/NonInterruptibleChase/PedestrianTakeDownChasee/AttemptTakeDownTargetable/TakeDownTargetableSuccess/TauntAfterTakedown`.
    pub(crate) const TAUNT_AFTER_TAKEDOWN_125: u16 = 125;
    /// `Pedestrian/ChaseChasee/ActiveChase/NonInterruptibleChase/PedestrianTakeDownChasee/AttemptTakeDownTargetable/TakeDownTargetableSuccess/NoTaunt`.
    pub(crate) const NO_TAUNT_126: u16 = 126;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaseTargetUnreachable/UnreachableTimerExpired/TazeUnreachableTarget/DrawTazer`.
    pub(crate) const DRAW_TAZER_127: u16 = 127;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaseTargetUnreachable/UnreachableTimerExpired/TazeUnreachableTarget/TazeEntity`.
    pub(crate) const TAZE_ENTITY_128: u16 = 128;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/ChaseTargetUnreachable/UnreachableTimerExpired/TazeUnreachableTarget/EndTaze`.
    pub(crate) const END_TAZE_129: u16 = 129;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser/WithProtector/FormationChaser/BlockChasee`.
    pub(crate) const BLOCK_CHASEE_130: u16 = 130;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser/WithProtector/FormationChaser/PursueChasee`.
    pub(crate) const PURSUE_CHASEE_131: u16 = 131;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser/WithoutProtector/FormationChaser/BlockChasee`.
    pub(crate) const BLOCK_CHASEE_132: u16 = 132;
    /// `Pedestrian/ChaseChasee/ActiveChase/InterruptibleChase/SecondaryChaser/WithoutProtector/FormationChaser/PursueChasee`.
    pub(crate) const PURSUE_CHASEE_133: u16 = 133;
}
