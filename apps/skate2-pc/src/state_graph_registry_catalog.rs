//! Exact linked `StateGraph` factory-name catalog for the matched debug build.
//!
//! These keys prove registry membership and canonical spelling only. Concrete
//! constructors, `Process` methods, activation behavior, and `AddParam` semantics
//! are not inferred from membership.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateGraphFactoryKind {
    Condition,
    Behaviour,
    Hook,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateGraphFactoryCatalogCounts {
    pub conditions: usize,
    pub behaviours: usize,
    pub hooks: usize,
}

pub const CONDITION_REGISTRATION_ROWS_SHA256: &str =
    "07C8137269E4F699F2917EF17218C4D0ACABA8C7B5B2D44B918CEEE1144A9E55";
pub const BEHAVIOUR_REGISTRATION_ROWS_SHA256: &str =
    "5F0694852342C455CB40AA388FC29CC5CCFA1B31981EA92A00383C3A56097594";
pub const HOOK_REGISTRATION_ROWS_SHA256: &str =
    "78965096B4214C2B4039B353ED642BAEC0D813472A239BBE222D20F33A384C7C";

// These key-only digests are over the catalog order, with every key followed
// by one LF byte. Unlike the registration-row provenance hashes above, they
// can be recomputed from this module and therefore detect any key or ordering
// drift in the embedded 587-name catalog.
const CONDITION_KEYS_SHA256: &str =
    "12C1D26162BB559BF9FCAE08AF1F884A283C1788618D85E76AB8B6EAB178A829";
const BEHAVIOUR_KEYS_SHA256: &str =
    "79120677F8072021A64A1D90656138933A6315DA00471CE05B1C2F2D69EADF82";
const HOOK_KEYS_SHA256: &str = "896DA97BA649EBCFBCF895A9548670855E271D60F7A55720EFA6CF80A5B20498";

const CONDITION_NAMES: &[&[u8]] = &[
    b"IsPluginCollisionOverride",
    b"IsPluginAvoidOverride",
    b"MonitoredIntentComplete",
    b"IsLocomotionTransitioning",
    b"IsLocomotionTurning",
    b"InTurnBranchWindow",
    b"BranchWindowEnabled",
    b"CanPlayPedestrianChannel",
    b"IsPedestrianChannelComplete",
    b"NeedsToBeginColliding",
    b"IsPedestrianColliding",
    b"IsPhotographerTakingPictures",
    b"IsPhotographerReviewingPictures",
    b"IsFacingInterimTarget",
    b"MaxRecognizersExceeded",
    b"CanRecognizePlayer",
    b"IsBeingHonkedAt",
    b"AtGatherDistance",
    b"GatherTimedOut",
    b"IsGathering",
    b"InGatherArea",
    b"IsPatrolling",
    b"IsEntityCategory",
    b"TakeDownAttemptSuccessful",
    b"TakeDownAttemptFailed",
    b"CanAttemptTakeDown",
    b"SpectatorIsBored",
    b"HasWaypointLocked",
    b"EntityTypesAreSame",
    b"IsOnRoad",
    b"IsOnIntersection",
    b"TimedRangeRandom",
    b"SourceEntitySharesPlugin",
    b"SimpleRandom",
    b"DistanceFromProp",
    b"LookAtTimeOut",
    b"SkaterInFrontalCone",
    b"IsExhaustedChasingSkater",
    b"IsRested",
    b"MinTimeAfterWarn",
    b"IsStopped",
    b"OutsideOfGroupRadius",
    b"InSkaterRadius",
    b"PredictedToCrossSkaterPath",
    b"MemberOfGroup",
    b"HasPlugin",
    b"CanSeeSkater",
    b"CanHearSkater",
    b"SkaterVelocityExceeds",
    b"IsFacingSkater",
    b"TargetConverseTimeout",
    b"IsFacingWaypointOrientation",
    b"IsLocomotionTurnComplete",
    b"DistanceFromWaypointXZ",
    b"IsAtWaypoint",
    b"IsTargetingWaypoint",
    b"IsBeingChased",
    b"WantsToWarn",
    b"WantsToChase",
    b"CanNewChaseStart",
    b"IsChasing",
    b"CanChaseeAddNewChaser",
    b"CanSeeChasee",
    b"DistanceFromChasee",
    b"ChaseeCloseEnoughToSatisfyAggressiveChase",
    b"NeedToRest",
    b"HasTakeDownTargetable",
    b"CanAttemptTakeDownTargetable",
    b"ChaseeIsWipedOut",
    b"AttemptTakeDownTimeout",
    b"TakeDownSuccessTimeout",
    b"InvestigateTimeExceeded",
    b"IsChaserEndingChase",
    b"IsEndChaseReason",
    b"ChaseeEscaped",
    b"ChaserAtFlankPosition",
    b"ChaserShouldBlock",
    b"IsPrimaryChaser",
    b"CannotReachChaseTarget",
    b"CanTazeUnreachableTargets",
    b"ChaseeHasProtector",
    b"ChasersAreScared",
    b"PathingFailureLastSeek",
    b"SkaterIsOnNonZeroMeshCell",
    b"IsNearCrossWalk",
    b"IsEnteringCrossWalk",
    b"HasCrossedCrossWalk",
    b"WalkSignSaysGo",
    b"HasLeadPedestrian",
    b"WanderWaitTimer",
    b"HasArrivedAtTarget",
    b"HasMonitoredIntent",
    b"HasRoadWanderTarget",
    b"WantsToBecomeAlert",
    b"AlertComplete",
    b"HasWantThatNeedsToBeAddressed",
    b"HasSpecificWantThatNeedsToBeAddressed",
    b"PedestrianIsInConversation",
    b"ConversationIsComplete",
    b"ConversationThisParticipantIsSpeaker",
    b"SimpleTimerExpired",
    b"HasHandProp",
    b"IsFacingWantTarget",
    b"IsHoldingSpecificHandProp",
    b"GoingToStandBackUp",
    b"HasDrawnTazer",
    b"CanAttackThrowHandProp",
    b"IsWantTargetWithinAlertDistance",
    b"DistanceToWantTarget",
    b"HasLineOfSightToTazeTarget",
    b"ChaserShouldGiveUpDueToTakedowns",
    b"InBranchWindow",
    b"InTransitionInfoWindow",
    b"PedestrianJumperIsLanding",
    b"PedestrianJumperInDownwardMotion",
    b"WantsToProtectProtectee",
    b"IsProtectingProtectee",
    b"IsProtecteeUnderThreat",
    b"IsProtecteeBeingChased",
    b"NeedsToBlockThreat",
    b"NeedsToInterceptThreat",
    b"DistanceFromProtectee",
    b"PatrolZoneOwnerIsInOwnZone",
    b"PatrolZoneOwnerIsInOwnZoneBoundaryOrBuffer",
    b"EntityOfInterestIsInZone",
    b"EntityOfInterestIsInZoneBufferRegion",
    b"EntityOfInterestIsInZoneBoundaryOrBuffer",
    b"PatrolZoneMaintainInterestTimerActive",
    b"EntityOfInterestIsInMaintainInterestRadius",
    b"HasAGIntent",
    b"AGIntentHasMagnitude",
    b"AGIntentInAngle",
    b"HasGestureIntent",
    b"TimeSinceLastInput",
    b"PhysicsRequestsDismount",
    b"IsLandingOnBoard",
    b"IsInLocomotion",
    b"HasAnimAttribute",
    b"LocomotionValid",
    b"IsGrinding",
    b"PhysFilteredState",
    b"IsPhysicsWiping",
    b"PhysicsWantsWipeOut",
    b"PhysicsWantsRunout",
    b"TrucksOrDeckInContact",
    b"SurfaceSlope",
    b"SkateSlope",
    b"DisablePushBrake",
    b"PhysicsSpeedAndSlopeCompare",
    b"PhysicsSpeedCompare",
    b"ComVelCompare",
    b"InParentStateForTime",
    b"InStateForTime",
    b"LastState",
    b"CurrentState",
    b"IsRidingSwitch",
    b"IsMirrored",
    b"IsRidingFakie",
    b"IsRollingBackwards",
    b"RandomCond",
    b"IsHandPlanting",
    b"IsAISkater",
    b"IsInDebugAnimationsMode",
    b"IsDismounting",
    b"IsGrabbingObject",
    b"OBTimeToLand",
    b"HasIntent",
    b"HasFilteredMotionGraphIntent",
    b"WillExpire",
    b"InTimeWindow",
    b"IsTransitioning",
    b"CurrentGrabType",
    b"UseNewLocomotion",
    b"BreakOutOfPush",
    b"SlideAngle",
    b"IsDoneWipingOut",
    b"WipeoutTimeToLand",
    b"HasTweak",
    b"IsRidingGoofy",
    b"IsHoldingSkateboard",
    b"IsRetrievingSkateboard",
    b"IsDroppingSkateboard",
    b"IsMovingObject",
    b"ApexReached",
    b"CanLandOnBoard",
    b"EnableAnimTrickWipeOuts",
    b"IsTimeToPrepareLand",
    b"HasLandingType",
    b"HasTiltToLargeForPreland",
    b"IsBumped",
    b"CanEnterSlide",
    b"ShouldLeaveSlide",
    b"TimeToLand",
    b"IsBodyFlipping",
    b"IsProSkater",
    b"FacingForwards",
    b"AllowedToTrick",
    b"OkToDoTrickOnStairs",
    b"TimeSinceKickturn",
    b"MongoPushFootToFar",
    b"IsLanding",
    b"IsRunningOut",
    b"IsDoneAirDismount",
    b"IsAirMountRequested",
    b"IsInJumpAir",
    b"IsLocomotionQuitNotAllowed",
    b"IsLocomotionFootDown",
    b"IsPastLocomotionApex",
    b"DoSprintMount",
    b"ManualOutTimerIsActive",
    b"AngleAdjustByAnimationEndLessThan",
    b"IsSkitching",
    b"IsSkitchingWithAbsorb",
    b"IsEnteringSkitch",
    b"SkitchingPosition",
    b"IsSkitchShimmying",
    b"ShouldPrepareOneFootAirForFootplant",
    b"IsFootPlanting",
    b"ShouldPlayHandPlantAnim",
    b"HasNewHandPlantPos",
    b"IsGrindBluntingBackslash",
    b"IsRequiredToPassIntersection",
    b"IsRequiredToBeImpatient",
    b"IsRequiredToChangeLane",
    b"IsRequiredToPullOver",
    b"IsRequiredToPullOut",
    b"IsImpatienceComplete",
    b"IsLaneChangePassageComplete",
    b"IsIntersectionPassageComplete",
    b"IsRequiredToRegisterToIntersection",
    b"IsRequiredToUnregisterFromIntersection",
    b"IsRequiredToStopAlarming",
    b"SkaterIsOnRoad",
    b"IsPlayerSkateboardOnGround",
    b"IsWipingOut",
    b"IsFrontside",
    b"IsInManual",
    b"SkaterIsGrinding",
    b"SkaterWasGrinding",
    b"IsGrindForwards",
    b"IsGrindType5050",
    b"IsGrindTypeBoardslide",
    b"IsGrindType5O",
    b"IsNoseslide",
    b"IsTailslide",
    b"IsReentry",
    b"IsUpcomingGrindValid",
    b"IsPreparingToJump",
    b"IsGrindLedgeLeft",
    b"IsGrindLedgeRight",
    b"TurningCentredTime",
    b"TurningTime",
    b"CameraSubjectSpeed",
    b"CameraSubjectSpeedY",
    b"CameraSubjectAcceleration",
    b"HeightRelativeToTakeoff",
    b"PredictedAirTime",
    b"PredictedLandingHeight",
    b"PredictedMaxHeight",
    b"PredictedHeightDistanceRatio",
    b"FrontsideAngle",
    b"InclineAlignmentAngle",
    b"InclineAngle",
    b"InclineAngleAbs",
    b"GrindInclineAngle",
    b"TransferAngle",
    b"LaunchInclineAngle",
    b"LandingInclineAngle",
    b"TrajectoryProgressWarped",
    b"UpcomingGrindAngle",
    b"SkaterIsUnderThreat",
    b"PreviousHeading",
    b"IsCameraTypeActive",
    b"IsInRaceChallenge",
    b"IsInWorldPainterRegion",
    b"IsInVolume",
    b"IsOffboard",
    b"IsAirOffboard",
    b"SkaterIsSkitching",
    b"SkaterIsMovingObject",
    b"SkaterIsPerformingFootPlant",
    b"SkaterIsPerformingHandPlant",
    b"SkaterIsPerformingBoneless",
    b"SkaterIsPerformingHippyJump",
    b"SkaterIsRunningOut",
    b"IsPreviousShot",
];

const BEHAVIOUR_NAMES: &[&[u8]] = &[
    b"OverridePluginCollision",
    b"OverridePluginAvoid",
    b"PedestrianColliding",
    b"ReviewingPictures",
    b"PhotographSkater",
    b"PedestrianIdle",
    b"DisableNavRigOverrides",
    b"FaceInterimTarget",
    b"TargetEdgeOfPathBoundary",
    b"WaitForCrossWalk",
    b"DisableNavigation",
    b"NoRoadWander",
    b"UseCrossWalk",
    b"RegisterRecognizer",
    b"WanderFollow",
    b"WanderOnRoad",
    b"ScanForLeadPedestrian",
    b"RunFromHonker",
    b"DoGather",
    b"CheckForRoadTarget",
    b"KeepMonitorIntentInQueue",
    b"DrawTazer",
    b"TazeWantTarget",
    b"EndTaze",
    b"RegisterTazer",
    b"UnregisterTazerOnEnd",
    b"CheckForTargetTimeout",
    b"IgnoreTakedownTargetNavRigVolume",
    b"StartChase",
    b"NewChasee",
    b"ResumeChase",
    b"InterceptChasee",
    b"RestFromChase",
    b"LostChasee",
    b"ChaserEvaluateWhoToTakeDown",
    b"PassiveNearChasee",
    b"ChaserEndChase",
    b"ChaserGroupEndChase",
    b"EndChase",
    b"AllowPedestrianJumping",
    b"EmotionalResponseToAggressiveChaseEnd",
    b"EmotionalResponseToGivingWarning",
    b"SuppressMoodAboutChasee",
    b"MoodResetAboutChasee",
    b"KnowAboutChasers",
    b"KnowAboutChasee",
    b"StartInvestigateTimer",
    b"EndInvestigateTimer",
    b"GiveUpBeingPrimaryChaser",
    b"PursueChasee",
    b"BlockChasee",
    b"StandAndWatchWantTarget",
    b"ClearExhaustionTimer",
    b"SetAltTargetToChaseePosition",
    b"UpdateBlockPrediction",
    b"Flee",
    b"TellChaseeToFlee",
    b"TakeDownTargetablePredictions",
    b"AttemptTakeDownTargetable",
    b"TakeDownTargetableSuccess",
    b"TakeDownTargetableFailure",
    b"TakedownTargetableClearOnEnd",
    b"DisableAllMoods",
    b"PlayerRecognition",
    b"Patrol",
    b"UnlockWaypoint",
    b"LockAndTargetWayPoint",
    b"TurnToFaceWaypointOrientation",
    b"LockFirstWaypoint",
    b"TargetWaypoint",
    b"Spectate",
    b"ChannelWarnWantTarget",
    b"AlertToWantTarget",
    b"TargetProp",
    b"ExitPlugin",
    b"SuggestVelocity",
    b"LookAt",
    b"Plugin",
    b"DisableCollisionsWithBehaviourSource",
    b"Wander",
    b"WanderWait",
    b"SetStandardMotionIntent",
    b"StopChannel",
    b"IgnoreCollision",
    b"StandAndWatchSkater",
    b"TurnToFaceSkater",
    b"CreateSimpleMonitoredIntent",
    b"IncrementMonitoredPacketStage",
    b"SetSimpleTimer",
    b"KeepSimpleTimerActive",
    b"SetExplicitTurnDirectionToWaypointOrientation",
    b"ProTrackSkater",
    b"ProWaitingForSkater",
    b"DisableHeavyCollision",
    b"IgnoreStandingCollisions",
    b"DisableCollisionSliding",
    b"LockToCurrentPosition",
    b"LockToWaypointPosition",
    b"OverrideAnimData",
    b"SuggestMusicState",
    b"SendSpeechEvent",
    b"KnowAboutWantTarget",
    b"UnsetWant",
    b"UnsetWantOnEnd",
    b"ChannelGreetWantTarget",
    b"SetIsReactingToMoodEventFlagOnBegin",
    b"UnsetIsReactingToMoodEventFlagOnEnd",
    b"ActivateRelatedWant",
    b"ChangeNavModifierSetting",
    b"OwnPluginObject",
    b"DrawDebugUpdateMarker",
    b"LockClosestWaypoint",
    b"PedestrianInConversation",
    b"ConversationSpeak",
    b"ConversationListenToSpeaker",
    b"ConversationSignalInPosition",
    b"ConversationBaseIdleAnimation",
    b"SpawnConversationArea",
    b"TakedownTauntVictim",
    b"SendChaseStateMessage",
    b"ThrowHandPropAtTrashBin",
    b"ThrowHandPropAtWantTarget",
    b"WatchWantTargetWithoutInterruptingLocomotion",
    b"StopAndFaceWantTarget",
    b"InvalidateNavTargetsOnEnd",
    b"SetSitTimer",
    b"DropHandProp",
    b"SetAlertTimer",
    b"SetTargetUnreachableTimer",
    b"ApproachWantTarget",
    b"SetWaitingToReactFlagOnBegin",
    b"ClearWaitingToReactFlagOnBegin",
    b"JackSkaterIfAppropriate",
    b"RefreshTargetExplicitWantTarget",
    b"DisallowHandPropActions",
    b"IgnoreCollisionsWithWantTarget",
    b"UnsetWantAndSuppressMoodForATime",
    b"PlayRemappedAnimation",
    b"MirrorBlock",
    b"RegisterClipBlock",
    b"RegisterTransitionInfo",
    b"MajorIntentComplete",
    b"IntentStageComplete",
    b"LocomotionTurnComplete",
    b"LocomotionTransitionStart",
    b"LocomotionTransitionComplete",
    b"LocomotionTransition",
    b"SetAttemptingTakedown",
    b"TriggerPedestrianJump",
    b"PlayPedestrianTimeScaledJumpAnimation",
    b"ChanceToDropHandPropOnLargeAvoid",
    b"SpawnInteractionBasedHandProp",
    b"ProtectProtectee",
    b"TrackProtectee",
    b"WatchProtectee",
    b"BigBlackChasing",
    b"BigBlackBlock",
    b"EvaluateThreatToProtectee",
    b"BigBlackEvaluateWhoToTakeDown",
    b"ApproachZonePointOfInterest",
    b"ReturnToPatrolZone",
    b"SetPatrolZoneMaintainInterestTimerOnBegin",
    b"CreateMGIntentFromAGIntent",
    b"CreateConstMGIntent",
    b"CreateMGTimeIntentFromAGIntent",
    b"CreateTrickIntentFromGesture",
    b"ManualSignal",
    b"BodyFlippingSignal",
    b"BoardAdjust",
    b"JuiceHook",
    b"PrintText2D",
    b"ResetSkaterAnimation",
    b"UpdateTimeSinceTeleport",
    b"PrintTimeInState",
    b"PrintAnimAttribute",
    b"PrintFilteredIntent",
    b"PrintFacingDirection",
    b"PrintOverlay",
    b"PrintHeightOfCom",
    b"DebugPush",
    b"OverideNextAnimTransition",
    b"PlayAnimation",
    b"ForcePhysics",
    b"CreateAttribute",
    b"AttachIntent",
    b"AttachFilteredIntent",
    b"SetDistComToBoard",
    b"SetSpeed",
    b"SetTurning",
    b"KickTurnSteering",
    b"SetBumpCoefficients",
    b"ApplyingBodyTilt",
    b"SettingBodyTilt",
    b"StoreLandingData",
    b"SetLandingData",
    b"ChooseRandomSketchyLanding",
    b"ControlAirLegExtension",
    b"SetTrickAttr",
    b"ClearTrickAttr",
    b"SetTrickHeight",
    b"JumpInto",
    b"SetDeckPitchAndYaw",
    b"FingerFlipOut",
    b"SetGrabType",
    b"InitPush",
    b"ComputeFirstPushStrength",
    b"ComputeRepushDeadline",
    b"PushCycle",
    b"SetPushCoefs",
    b"PushOut",
    b"ComputeTargetCoefsFromSpeedAndStrength",
    b"Crouching",
    b"Pumping",
    b"DisallowPumping",
    b"GrindControlFade",
    b"CreateGrindAttributes",
    b"ControlGrindCrouch",
    b"InitMovingObjects",
    b"MovingObject",
    b"Locomotion",
    b"PlayWipeoutRecoverAnim",
    b"Wipeout",
    b"ToggleBoard",
    b"CharacterGesture",
    b"EndGesture",
    b"HandBusy",
    b"BodySpin",
    b"IsDoingTrick",
    b"FakieHeadChannel",
    b"IsAnticipating",
    b"IsLanding",
    b"IsManualing",
    b"UpdateTimeSinceKickturn",
    b"ResetTimeSinceKickturn",
    b"PowerSliding",
    b"CreateSlide",
    b"PowerSlideDecel",
    b"PowerSlideSpin",
    b"PowerSlideManualAtt",
    b"InCandidateSlidingState",
    b"IsPowerSliding",
    b"TweakProject",
    b"SetManualAngle",
    b"FilterMotionGraphIntent",
    b"UpdateRidingFakie",
    b"UpdateIsWeightOnNose",
    b"ResetToGivenStance",
    b"SlowMotionController",
    b"Rumble",
    b"DisableTricks",
    b"Dismount",
    b"RunoutFront",
    b"RunoutBack",
    b"OBAir",
    b"AirDismounting",
    b"LandOnBoard",
    b"StandingJumpDescent",
    b"StandingJumpLandingCOG",
    b"LandingOffBoardAir",
    b"SteppingBoardNoBoard",
    b"SetManualOutTimer",
    b"UpdateManualOutTimer",
    b"SkitchingBehaviour",
    b"EnterSkitchingBehaviour",
    b"SkitchShimmyingBehaviour",
    b"EndShimmy",
    b"FootPlantAbsorb",
    b"SetHandPlantAnticLength",
    b"HippyFlip",
    b"EndHippyFlip",
    b"HippyJumpAntic",
    b"ScoringTrick",
    b"ScoringGrabs",
    b"ScoringHandPlants",
    b"SetScoreAugmentation",
    b"ContinueFollowingLane",
    b"ContinueImpatience",
    b"ContinueChangingLane",
    b"ContinuePullingOver",
    b"ContinueStayingParked",
    b"ContinuePullingOut",
    b"ContinuePassingIntersection",
    b"CameraChooseShot",
];

const HOOK_NAMES: &[&[u8]] = &[
    b"ToBaseLocomotionState",
    b"ToTransitionInfoBaseLocomotionState",
    b"MyHook",
    b"GrabSlide",
    b"OverideNextAnimTransitionHook",
    b"MongoPushToAntic",
    b"InitializeLocomotion",
    b"ToImpatience",
    b"ToChangingLane",
    b"ToPullingOver",
    b"ToStayingParked",
    b"ToPullingOut",
    b"ToPassingIntersection",
    b"FromChangingLaneToFollowingLane",
    b"FromPassingIntersectionToFollowingLane",
    b"RegisterToIntersection",
    b"UnregisterFromIntersection",
    b"StopAlarming",
];

#[must_use]
pub const fn counts() -> StateGraphFactoryCatalogCounts {
    StateGraphFactoryCatalogCounts {
        conditions: CONDITION_NAMES.len(),
        behaviours: BEHAVIOUR_NAMES.len(),
        hooks: HOOK_NAMES.len(),
    }
}

/// Returns the canonical registered spelling using the recovered ASCII
/// case-insensitive lookup domain.
#[must_use]
pub fn resolve(kind: StateGraphFactoryKind, query: &[u8]) -> Option<&'static [u8]> {
    registered_names(kind)
        .iter()
        .copied()
        .find(|candidate| ascii_eq_ignore_case(candidate, query))
}

#[must_use]
pub fn registered_names(kind: StateGraphFactoryKind) -> &'static [&'static [u8]] {
    match kind {
        StateGraphFactoryKind::Condition => CONDITION_NAMES,
        StateGraphFactoryKind::Behaviour => BEHAVIOUR_NAMES,
        StateGraphFactoryKind::Hook => HOOK_NAMES,
    }
}

/// Returns whether every embedded factory key and its order match the frozen
/// registration TSV projection.
#[must_use]
pub fn key_catalog_matches_frozen() -> bool {
    [
        (StateGraphFactoryKind::Condition, CONDITION_KEYS_SHA256),
        (StateGraphFactoryKind::Behaviour, BEHAVIOUR_KEYS_SHA256),
        (StateGraphFactoryKind::Hook, HOOK_KEYS_SHA256),
    ]
    .into_iter()
    .all(|(kind, expected)| catalog_keys_sha256(kind) == expected)
}

fn catalog_keys_sha256(kind: StateGraphFactoryKind) -> String {
    let mut canonical = Vec::new();
    for name in registered_names(kind) {
        canonical.extend_from_slice(name);
        canonical.push(b'\n');
    }
    sha256_hex(&canonical)
}

/// Small dependency-free SHA-256 helper shared by mounted corpus oracles.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut result = String::with_capacity(64);
    for byte in sha256(bytes) {
        write!(result, "{byte:02X}").expect("writing into a String cannot fail");
    }
    result
}

#[allow(clippy::many_single_char_names, clippy::too_many_lines)]
fn sha256(bytes: &[u8]) -> [u8; 32] {
    const INITIAL: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const ROUND: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let bit_length = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());

    let mut hash = INITIAL;
    for chunk in padded.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (word, source) in words[..16].iter_mut().zip(chunk.chunks_exact(4)) {
            *word = u32::from_be_bytes(source.try_into().expect("four-byte SHA-256 word"));
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = hash;
        for (&word, &round) in words.iter().zip(&ROUND) {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temporary1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(round)
                .wrapping_add(word);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temporary2 = sum0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temporary1);
            d = c;
            c = b;
            b = a;
            a = temporary1.wrapping_add(temporary2);
        }
        for (slot, value) in hash.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }

    let mut result = [0_u8; 32];
    for (output, word) in result.chunks_exact_mut(4).zip(hash) {
        output.copy_from_slice(&word.to_be_bytes());
    }
    result
}

fn ascii_eq_ignore_case(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_linked_registry_sizes_are_frozen() {
        assert_eq!(
            counts(),
            StateGraphFactoryCatalogCounts {
                conditions: 286,
                behaviours: 283,
                hooks: 18,
            }
        );
    }

    #[test]
    fn lookups_preserve_canonical_spelling_and_registry_domains() {
        assert_eq!(
            resolve(StateGraphFactoryKind::Condition, b"hASiNTENT"),
            Some(b"HasIntent".as_slice())
        );
        assert_eq!(
            resolve(StateGraphFactoryKind::Behaviour, b"pLAYaNIMATION"),
            Some(b"PlayAnimation".as_slice())
        );
        assert_eq!(
            resolve(
                StateGraphFactoryKind::Hook,
                b"OVERIDENEXTANIMTRANSITIONHOOK"
            ),
            Some(b"OverideNextAnimTransitionHook".as_slice())
        );
        assert_eq!(
            resolve(
                StateGraphFactoryKind::Hook,
                b"OverrideNextAnimTransitionHook"
            ),
            None
        );
        assert_eq!(
            resolve(StateGraphFactoryKind::Condition, b"DistanceFromWaypoint"),
            None
        );
        assert_eq!(
            resolve(StateGraphFactoryKind::Condition, b"distancefromwaypointxz"),
            Some(b"DistanceFromWaypointXZ".as_slice())
        );
        assert_eq!(
            resolve(StateGraphFactoryKind::Condition, b"IsLanding"),
            Some(b"IsLanding".as_slice())
        );
        assert_eq!(
            resolve(StateGraphFactoryKind::Behaviour, b"IsLanding"),
            Some(b"IsLanding".as_slice())
        );
        assert_eq!(resolve(StateGraphFactoryKind::Hook, b"IsLanding"), None);
    }

    #[test]
    fn each_registry_has_no_ascii_case_equivalent_duplicate() {
        for kind in [
            StateGraphFactoryKind::Condition,
            StateGraphFactoryKind::Behaviour,
            StateGraphFactoryKind::Hook,
        ] {
            let names = registered_names(kind);
            for (index, name) in names.iter().enumerate() {
                assert!(
                    names[index + 1..]
                        .iter()
                        .all(|other| !ascii_eq_ignore_case(name, other)),
                    "duplicate in {kind:?}: {:?}",
                    String::from_utf8_lossy(name)
                );
            }
        }
    }

    #[test]
    fn exact_key_catalog_and_sha256_implementation_are_frozen() {
        assert_eq!(
            sha256_hex(b"abc"),
            "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        );
        assert!(key_catalog_matches_frozen());
    }
}
