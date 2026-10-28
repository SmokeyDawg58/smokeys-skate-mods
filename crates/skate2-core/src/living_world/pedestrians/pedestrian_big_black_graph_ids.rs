//! The `HCDelegate_Big_Black` state graph's ids under names from its own data, written by
//! `tools/generate_graph_ids.py`: conditions by class and attributes, expressions by the
//! states and transitions that evaluate them, states by name and index.
#![allow(dead_code, reason = "the complete graph; code names the ids it reads")]

/// Condition declarations: what each condition index tests.
pub(crate) mod condition {
    /// `NeedsToBeginColliding`.
    pub(crate) const NEEDS_TO_BEGIN_COLLIDING: u16 = 0;
    /// `IsPedestrianColliding`.
    pub(crate) const IS_PEDESTRIAN_COLLIDING: u16 = 1;
    /// `WantsToProtectProtectee`.
    pub(crate) const WANTS_TO_PROTECT_PROTECTEE: u16 = 2;
    /// `IsProtectingProtectee`.
    pub(crate) const IS_PROTECTING_PROTECTEE: u16 = 3;
    /// `HasTakeDownTargetable`.
    pub(crate) const HAS_TAKE_DOWN_TARGETABLE: u16 = 4;
    /// `CanAttemptTakeDownTargetable`.
    pub(crate) const CAN_ATTEMPT_TAKE_DOWN_TARGETABLE: u16 = 5;
    /// `HasMonitoredIntent(intentName = ActiveTakedown)`.
    pub(crate) const HAS_MONITORED_INTENT_ACTIVE_TAKEDOWN: u16 = 6;
    /// `TakeDownAttemptSuccessful`.
    pub(crate) const TAKE_DOWN_ATTEMPT_SUCCESSFUL: u16 = 7;
    /// `TakeDownAttemptFailed`.
    pub(crate) const TAKE_DOWN_ATTEMPT_FAILED: u16 = 8;
    /// `SimpleTimerExpired(timerName = AttemptTakeDownTimeout)`.
    pub(crate) const SIMPLE_TIMER_EXPIRED_ATTEMPT_TAKE_DOWN_TIMEOUT: u16 = 9;
    /// `HasSpecificWantThatNeedsToBeAddressed(want = angrychase)`.
    pub(crate) const HAS_WANT_ANGRY_CHASE: u16 = 10;
    /// `IsProtecteeUnderThreat`.
    pub(crate) const IS_PROTECTEE_UNDER_THREAT: u16 = 11;
    /// `NeedsToInterceptThreat`.
    pub(crate) const NEEDS_TO_INTERCEPT_THREAT: u16 = 12;
    /// `NeedsToBlockThreat`.
    pub(crate) const NEEDS_TO_BLOCK_THREAT: u16 = 13;
    /// `DistanceFromProtectee(greater = 20.0)`.
    pub(crate) const DISTANCE_FROM_PROTECTEE_GREATER_20: u16 = 14;
    /// `DistanceFromProtectee(greater = 10.0)`.
    pub(crate) const DISTANCE_FROM_PROTECTEE_GREATER_10: u16 = 15;
    /// `DistanceFromProtectee(greater = 12.0)`.
    pub(crate) const DISTANCE_FROM_PROTECTEE_GREATER_12: u16 = 16;
    /// `DistanceFromProtectee(greater = 6.0)`.
    pub(crate) const DISTANCE_FROM_PROTECTEE_GREATER_6: u16 = 17;
}

/// Expressions: which states and transitions evaluate each expression index.
pub(crate) mod expression {
    /// State `AttemptTakeDownTargetable7`.
    pub(crate) const ATTEMPT_TAKE_DOWN_TARGETABLE_7: u16 = 0;
    /// `WaitForTakedownDecision12` -> `TakeDownTargetableSuccess14`.
    pub(crate) const WAIT_FOR_TAKEDOWN_DECISION_12_TO_TAKE_DOWN_TARGETABLE_SUCCESS_14: u16 = 1;
    /// State `TauntAfterTakedown17`.
    pub(crate) const TAUNT_AFTER_TAKEDOWN_17: u16 = 2;
    /// State `Protect_Protectee5`.
    pub(crate) const PROTECT_PROTECTEE_5: u16 = 3;
    /// State `Chasing8`.
    pub(crate) const CHASING_8: u16 = 4;
    /// State `Block9`.
    pub(crate) const BLOCK_9: u16 = 5;
    /// State `Colliding2`.
    pub(crate) const COLLIDING_2: u16 = 6;
    /// State `Guard_Protectee3`.
    pub(crate) const GUARD_PROTECTEE_3: u16 = 7;
    /// State `BigBlackTakeDown4`.
    pub(crate) const BIG_BLACK_TAKE_DOWN_4: u16 = 8;
    /// `WaitForTakedownDecision12` -> `TakeDownTargetableFailure13`.
    pub(crate) const WAIT_FOR_TAKEDOWN_DECISION_12_TO_TAKE_DOWN_TARGETABLE_FAILURE_13: u16 = 9;
    /// State `Run15`.
    pub(crate) const RUN_15: u16 = 10;
    /// State `Walk16`.
    pub(crate) const WALK_16: u16 = 11;
}

/// States by name and index, with their path below the root state.
pub(crate) mod state {
    /// `AI`.
    pub(crate) const AI_0: u16 = 0;
    /// `BigBlack`.
    pub(crate) const BIG_BLACK_1: u16 = 1;
    /// `BigBlack/Colliding`.
    pub(crate) const COLLIDING_2: u16 = 2;
    /// `BigBlack/Guard_Protectee`.
    pub(crate) const GUARD_PROTECTEE_3: u16 = 3;
    /// `BigBlack/Guard_Protectee/BigBlackTakeDown`.
    pub(crate) const BIG_BLACK_TAKE_DOWN_4: u16 = 4;
    /// `BigBlack/Guard_Protectee/Protect_Protectee`.
    pub(crate) const PROTECT_PROTECTEE_5: u16 = 5;
    /// `BigBlack/Guard_Protectee/TrackProtectee`.
    pub(crate) const TRACK_PROTECTEE_6: u16 = 6;
    /// `BigBlack/Guard_Protectee/BigBlackTakeDown/AttemptTakeDownTargetable`.
    pub(crate) const ATTEMPT_TAKE_DOWN_TARGETABLE_7: u16 = 7;
    /// `BigBlack/Guard_Protectee/Protect_Protectee/Chasing`.
    pub(crate) const CHASING_8: u16 = 8;
    /// `BigBlack/Guard_Protectee/Protect_Protectee/Block`.
    pub(crate) const BLOCK_9: u16 = 9;
    /// `BigBlack/Guard_Protectee/TrackProtectee/Follow`.
    pub(crate) const FOLLOW_10: u16 = 10;
    /// `BigBlack/Guard_Protectee/TrackProtectee/Watch`.
    pub(crate) const WATCH_11: u16 = 11;
    /// `BigBlack/Guard_Protectee/BigBlackTakeDown/AttemptTakeDownTargetable/WaitForTakedownDecision`.
    pub(crate) const WAIT_FOR_TAKEDOWN_DECISION_12: u16 = 12;
    /// `BigBlack/Guard_Protectee/BigBlackTakeDown/AttemptTakeDownTargetable/TakeDownTargetableFailure`.
    pub(crate) const TAKE_DOWN_TARGETABLE_FAILURE_13: u16 = 13;
    /// `BigBlack/Guard_Protectee/BigBlackTakeDown/AttemptTakeDownTargetable/TakeDownTargetableSuccess`.
    pub(crate) const TAKE_DOWN_TARGETABLE_SUCCESS_14: u16 = 14;
    /// `BigBlack/Guard_Protectee/TrackProtectee/Follow/Run`.
    pub(crate) const RUN_15: u16 = 15;
    /// `BigBlack/Guard_Protectee/TrackProtectee/Follow/Walk`.
    pub(crate) const WALK_16: u16 = 16;
    /// `BigBlack/Guard_Protectee/BigBlackTakeDown/AttemptTakeDownTargetable/TakeDownTargetableSuccess/TauntAfterTakedown`.
    pub(crate) const TAUNT_AFTER_TAKEDOWN_17: u16 = 17;
    /// `BigBlack/Guard_Protectee/BigBlackTakeDown/AttemptTakeDownTargetable/TakeDownTargetableSuccess/NoTaunt`.
    pub(crate) const NO_TAUNT_18: u16 = 18;
}
