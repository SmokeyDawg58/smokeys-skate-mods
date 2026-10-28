//! Native pedestrian AI StateUpdate callback order, recovered from IDA.
//! Parent calls are expanded in place; slots address the sorted active instances.
//! Callback implementation/authority belongs to the concrete backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UpdateCallback {
    DrawDebugUpdateMarker,
    EvaluateThreatToProtectee,
    BigBlackEvaluateWhoToTakeDown,
    BigBlackChasing,
    BigBlackBlock,
    TrackProtectee,
    WatchProtectee,
    PedestrianColliding,
    Noop,
    SuggestVelocity,
    WanderOnRoad,
    RunFromHonker,
    Plugin,
    Patrol,
    ApproachZonePointOfInterest,
    ReturnToPatrolZone,
    CheckForTargetTimeout,
    KnowAboutChasee,
    SetAltTargetToChaseePosition,
    CheckForRoadTarget,
    Flee,
    KnowAboutChasers,
    Wander,
    WatchWantTargetWithoutInterruptingLocomotion,
    ChannelWarnWantTarget,
    StopAndFaceWantTarget,
    ChaserEvaluateWhoToTakeDown,
    IgnoreTakedownTargetNavRigVolume,
    TakeDownTargetablePredictions,
    InterceptChasee,
    UpdateBlockPrediction,
    UseCrossWalk,
    ChannelGreetWantTarget,
    DrawTazer,
    AttemptTakeDownTargetable,
    RefreshTargetExplicitWantTarget,
    LostChasee,
    ThrowHandPropAtWantTarget,
    ApproachWantTarget,
    BlockChasee,
    PursueChasee,
    DisableNavigation,
    ProTrackSkater,
    PhotographSkater,
    ReviewingPictures,
}

pub(crate) fn sequence(source: u32) -> Option<&'static [(usize, UpdateCallback)]> {
    use UpdateCallback::*;
    Some(match source {
        0x825187c0 => &[], // verified native blr
        0x828f5f18 => &[(0, DrawDebugUpdateMarker), (1, PedestrianColliding), (2, Noop)], // StateUpdate_Colliding_3
        0x828f5f88 => &[(0, DrawDebugUpdateMarker), (1, SuggestVelocity), (2, Noop)], // StateUpdate_Wander_5
        0x828f5ff8 => &[(0, DrawDebugUpdateMarker), (1, WanderOnRoad), (2, SuggestVelocity)], // StateUpdate_WanderOnRoad_6
        0x828f6068 => &[(0, DrawDebugUpdateMarker), (1, RunFromHonker), (2, Noop)], // StateUpdate_RunFromHonker_7
        0x828f60d8 => &[(0, DrawDebugUpdateMarker), (1, Plugin), (2, Noop)], // StateUpdate_Plugin_8
        0x828f6148 => &[(0, DrawDebugUpdateMarker), (1, Patrol), (2, SuggestVelocity), (3, Noop)], // StateUpdate_Patrol_9
        0x828f61d0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop)], // StateUpdate_ChaseChasee_11
        0x828f6268 => &[(0, DrawDebugUpdateMarker), (1, ApproachZonePointOfInterest), (2, Noop)], // StateUpdate_ApproachZonePointOfInterest_12
        0x828f62d8 => &[(0, DrawDebugUpdateMarker), (1, ApproachZonePointOfInterest), (2, Noop), (3, Noop)], // StateUpdate_WaitOutMaintainInterestTimer_13
        0x828f6360 => &[(0, DrawDebugUpdateMarker), (1, ReturnToPatrolZone), (2, Noop)], // StateUpdate_ReturnToPatrolZone_14
        0x828f63d0 => &[(0, DrawDebugUpdateMarker), (1, SuggestVelocity), (2, Noop), (3, CheckForTargetTimeout)], // StateUpdate_WanderMode_15
        0x828f6458 => &[(0, DrawDebugUpdateMarker)], // StateUpdate_BigBlack_1 (folded shared body)
        0x828f6470 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop)], // StateUpdate_UninterrubtableWants_17
        0x828f64f8 => &[(0, DrawDebugUpdateMarker), (1, Noop)], // StateUpdate_InterrubtableWants_18
        0x828f6558 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop)], // StateUpdate_StartChase_19
        0x828f6560 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop)], // StateUpdate_ActiveChase_20
        0x828f65d8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop)], // StateUpdate_FromChasingEndChase_38
        0x828f6638 => &[(0, DrawDebugUpdateMarker), (1, SuggestVelocity), (2, Noop), (3, CheckForTargetTimeout)], // StateUpdate_Road_22
        0x828f6640 => &[(0, DrawDebugUpdateMarker), (1, SuggestVelocity), (2, Noop), (3, CheckForTargetTimeout), (4, CheckForRoadTarget), (5, Noop)], // StateUpdate_NoRoad_23
        0x828f66a0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop)], // StateUpdate_PedestrianWantToTaze_30
        0x828f66f0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop)], // StateUpdate_ObserveNearbyEvent_24
        0x828f66f8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop)], // StateUpdate_Alert_54
        0x828f6768 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Flee), (4, KnowAboutChasers), (5, Noop), (6, Noop)], // StateUpdate_Flee_33
        0x828f6828 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop)], // StateUpdate_ResumeChase_34
        0x828f6878 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop)], // StateUpdate_PedestrianTakeDownChasee_60
        0x828f6880 => &[(0, DrawDebugUpdateMarker), (1, SuggestVelocity), (2, Noop), (3, CheckForTargetTimeout), (4, Wander)], // StateUpdate_FollowRoad_41
        0x828f68d0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, WatchWantTargetWithoutInterruptingLocomotion), (6, Noop), (7, Noop), (8, Noop)], // StateUpdate_ObserveSlam_43
        0x828f6970 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, WatchWantTargetWithoutInterruptingLocomotion), (7, ChannelWarnWantTarget), (8, Noop), (9, Noop)], // StateUpdate_PedestrianDoWarning_44
        0x828f6a20 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop)], // StateUpdate_ReturnGreet_47
        0x828f6a98 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, StopAndFaceWantTarget), (6, Noop)], // StateUpdate_HasTazer_51
        0x828f6b10 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop)], // StateUpdate_CantThrowHandProp_74
        0x828f6b70 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Flee), (4, KnowAboutChasers), (5, Noop), (6, Noop), (7, Noop)], // StateUpdate_HasEscaped_55
        0x828f6bc0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Flee), (4, KnowAboutChasers), (5, Noop), (6, Noop)], // StateUpdate_Fleeing_56
        0x828f6bc8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop), (8, Noop), (9, Noop), (10, WatchWantTargetWithoutInterruptingLocomotion), (11, Noop)], // StateUpdate_HoldBeforeChaseStart_58
        0x828f6c90 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, SuggestVelocity), (10, StopAndFaceWantTarget)], // StateUpdate_ChaseTargetUnreachable_62
        0x828f6d08 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, SuggestVelocity), (10, Noop), (11, Noop), (12, Noop), (13, Noop), (14, Noop)], // StateUpdate_ChaserExhausted_63
        0x828f6dd0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, IgnoreTakedownTargetNavRigVolume), (10, TakeDownTargetablePredictions), (11, Noop), (12, InterceptChasee)], // StateUpdate_InterceptChasee_65
        0x828f6e70 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop), (10, UpdateBlockPrediction)], // StateUpdate_SecondaryChaser_66
        0x828f6ee8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop), (8, Noop), (9, Noop), (10, Noop), (11, Noop)], // StateUpdate_ReturnToPatrolZoneEndChase_68
        0x828f6fb0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop), (8, Noop), (9, Noop)], // StateUpdate_DefaultEndChase_70
        0x828f7050 => &[(0, DrawDebugUpdateMarker), (1, SuggestVelocity), (2, Noop), (3, CheckForTargetTimeout), (4, SuggestVelocity)], // StateUpdate_Wait_71
        0x828f70a0 => &[(0, DrawDebugUpdateMarker), (1, SuggestVelocity), (2, Noop), (3, CheckForTargetTimeout), (4, UseCrossWalk)], // StateUpdate_Walk_72
        0x828f70f0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, ChannelGreetWantTarget)], // StateUpdate_DoGreet_75
        0x828f7140 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop)], // StateUpdate_WatchGreet_77
        0x828f7148 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, StopAndFaceWantTarget), (6, Noop), (7, Noop), (8, DrawTazer)], // StateUpdate_DrawTazer_78
        0x828f71a8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, StopAndFaceWantTarget), (6, Noop), (7, Noop), (8, Noop), (9, Noop)], // StateUpdate_TazeEntity_79
        0x828f7220 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, StopAndFaceWantTarget), (6, Noop), (7, Noop), (8, Noop)], // StateUpdate_EndTaze_80
        0x828f7280 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop)], // StateUpdate_DoAlert_81
        0x828f7330 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop), (8, Noop), (9, Noop), (10, WatchWantTargetWithoutInterruptingLocomotion), (11, Noop)], // StateUpdate_NotFacingTargetWantTarget_124
        0x828f7338 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop)], // StateUpdate_ChaseAlreadyEnded_39
        0x828f73b0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, IgnoreTakedownTargetNavRigVolume), (9, Noop), (10, SuggestVelocity), (11, Noop), (12, Noop), (13, Noop), (14, AttemptTakeDownTargetable)], // StateUpdate_AttemptTakeDownTargetable_87
        0x828f7478 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop)], // StateUpdate_ReturnToPatrolZoneAutoEndChase_92
        0x828f74d8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown)], // StateUpdate_LostChasee_64
        0x828f7528 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, SuggestVelocity), (10, StopAndFaceWantTarget), (11, Noop), (12, ChannelWarnWantTarget), (13, RefreshTargetExplicitWantTarget)], // StateUpdate_WarnAndWait_94
        0x828f75a0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, SuggestVelocity), (10, StopAndFaceWantTarget)], // StateUpdate_UnreachableTimerExpired_95
        0x828f75a8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, LostChasee), (10, Noop), (11, Noop), (12, Noop)], // StateUpdate_SearchForChasee_96
        0x828f7648 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop), (10, UpdateBlockPrediction), (11, Noop)], // StateUpdate_FormationChaser_119
        0x828f7698 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, WatchWantTargetWithoutInterruptingLocomotion), (6, ThrowHandPropAtWantTarget)], // StateUpdate_FacingTarget_100
        0x828f7710 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, WatchWantTargetWithoutInterruptingLocomotion)], // StateUpdate_NotFacingTargetWantTarget_101
        0x828f7770 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, ChannelGreetWantTarget), (8, StopAndFaceWantTarget)], // StateUpdate_WatchGreetTarget_102
        0x828f77d0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, ChannelGreetWantTarget), (8, ApproachWantTarget)], // StateUpdate_ApproachGreetTarget_103
        0x828f7830 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, StopAndFaceWantTarget), (8, Noop)], // StateUpdate_WaitOutTimer_104
        0x828f7890 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop)], // StateUpdate_WaitOutTimer_106
        0x828f78e0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, StopAndFaceWantTarget)], // StateUpdate_WatchAlertTarget_108
        0x828f7930 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, ApproachWantTarget)], // StateUpdate_ApproachAlertTarget_109
        0x828f7980 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop), (8, Noop), (9, Noop), (10, WatchWantTargetWithoutInterruptingLocomotion), (11, Noop), (12, Noop)], // StateUpdate_CantThrowHandProp_111
        0x828f79d0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, IgnoreTakedownTargetNavRigVolume), (9, Noop), (10, SuggestVelocity), (11, Noop), (12, Noop), (13, Noop), (14, AttemptTakeDownTargetable)], // StateUpdate_WaitForTakedownDecision_112
        0x828f79d8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop), (10, Noop), (11, Noop)], // StateUpdate_TauntAfterTakedown_115
        0x828f7a60 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, SuggestVelocity), (10, StopAndFaceWantTarget), (11, Noop)], // StateUpdate_TazeUnreachableTarget_117
        0x828f7ab0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, SuggestVelocity), (10, StopAndFaceWantTarget), (11, Noop), (12, Noop)], // StateUpdate_GiveUpAfterWait_118
        0x828f7b10 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop), (10, UpdateBlockPrediction), (11, IgnoreTakedownTargetNavRigVolume), (12, TakeDownTargetablePredictions)], // StateUpdate_WithoutProtector_99
        0x828f7b70 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop), (8, StopAndFaceWantTarget)], // StateUpdate_WatchGreetTarget_121
        0x828f7bd0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop), (8, WatchWantTargetWithoutInterruptingLocomotion)], // StateUpdate_ApproachGreetTarget_122
        0x828f7c30 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, Noop), (8, Noop), (9, Noop), (10, WatchWantTargetWithoutInterruptingLocomotion), (11, Noop), (12, ThrowHandPropAtWantTarget)], // StateUpdate_FacingTarget_123
        0x828f7c80 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, IgnoreTakedownTargetNavRigVolume), (9, Noop), (10, SuggestVelocity), (11, Noop), (12, Noop), (13, Noop), (14, AttemptTakeDownTargetable), (15, Noop), (16, Noop), (17, Noop), (18, Noop)], // StateUpdate_TauntAfterTakedown_125
        0x828f7d08 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, IgnoreTakedownTargetNavRigVolume), (9, Noop), (10, SuggestVelocity), (11, Noop), (12, Noop), (13, Noop), (14, AttemptTakeDownTargetable), (15, Noop), (16, Noop)], // StateUpdate_TakeDownTargetableSuccess_114
        0x828f7d68 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, SuggestVelocity), (10, StopAndFaceWantTarget), (11, Noop), (12, Noop), (13, DrawTazer)], // StateUpdate_DrawTazer_127
        0x828f7de0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, SuggestVelocity), (10, StopAndFaceWantTarget), (11, Noop), (12, Noop), (13, Noop), (14, Noop)], // StateUpdate_TazeEntity_128
        0x828f7e68 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop), (10, UpdateBlockPrediction), (11, Noop), (12, BlockChasee), (13, Noop), (14, Noop)], // StateUpdate_BlockChasee_130
        0x828f7ef0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop), (10, UpdateBlockPrediction), (11, Noop), (12, PursueChasee)], // StateUpdate_PursueChasee_131
        0x828f7f50 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop), (10, UpdateBlockPrediction), (11, IgnoreTakedownTargetNavRigVolume), (12, TakeDownTargetablePredictions), (13, BlockChasee), (14, Noop), (15, Noop)], // StateUpdate_BlockChasee_132
        0x828f7ff0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, Noop), (3, Noop), (4, Noop), (5, KnowAboutChasee), (6, SetAltTargetToChaseePosition), (7, Noop), (8, ChaserEvaluateWhoToTakeDown), (9, Noop), (10, UpdateBlockPrediction), (11, IgnoreTakedownTargetNavRigVolume), (12, TakeDownTargetablePredictions), (13, PursueChasee)], // StateUpdate_PursueChasee_133
        0x828f88f0 => &[(0, DisableNavigation), (1, Noop)], // HCDelegate_marquee StateUpdate_Marquee_1
        0x828f8950 => &[(0, DisableNavigation), (1, Noop), (2, ProTrackSkater)], // StateUpdate_TrackingSkater_5
        0x828f8a20 => &[(0, DisableNavigation), (1, Noop), (2, PedestrianColliding)], // HCDelegate_marquee StateUpdate_Colliding_2
        0x828f8a90 => &[(0, DisableNavigation), (1, Noop), (2, PhotographSkater), (3, Noop), (4, Noop)], // HCDelegate_Photographer StateUpdate_PictureTaking_3
        0x828f8b28 => &[(0, DisableNavigation), (1, Noop), (2, ReviewingPictures)], // HCDelegate_Photographer StateUpdate_CheckOutPictures_4
        0x828f8b98 => &[(0, DisableNavigation), (1, Noop), (2, Noop)], // HCDelegate_marquee StateUpdate_HoldAtSpawn_4
        0x828f52e0 => &[(0, DrawDebugUpdateMarker), (1, PedestrianColliding)], // HCDelegate_Big_Black
        0x828f5340 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee)], // HCDelegate_Big_Black
        0x828f53b0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, IgnoreTakedownTargetNavRigVolume)], // HCDelegate_Big_Black
        0x828f5438 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, BigBlackEvaluateWhoToTakeDown), (8, TakeDownTargetablePredictions)], // HCDelegate_Big_Black
        0x828f5520 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, IgnoreTakedownTargetNavRigVolume), (4, IgnoreTakedownTargetNavRigVolume), (5, Noop), (6, SuggestVelocity), (7, Noop), (8, Noop), (9, Noop), (10, AttemptTakeDownTargetable)], // HCDelegate_Big_Black
        0x828f55e8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, BigBlackEvaluateWhoToTakeDown), (8, TakeDownTargetablePredictions), (9, BigBlackChasing), (10, IgnoreTakedownTargetNavRigVolume), (11, Noop)], // HCDelegate_Big_Black
        0x828f5660 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, Noop), (4, Noop), (5, Noop), (6, Noop), (7, BigBlackEvaluateWhoToTakeDown), (8, TakeDownTargetablePredictions), (9, BigBlackBlock), (10, IgnoreTakedownTargetNavRigVolume)], // HCDelegate_Big_Black
        0x828f56c0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, TrackProtectee)], // HCDelegate_Big_Black
        0x828f5748 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, WatchProtectee), (4, SuggestVelocity)], // HCDelegate_Big_Black
        0x828f57e0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, IgnoreTakedownTargetNavRigVolume), (4, IgnoreTakedownTargetNavRigVolume), (5, Noop), (6, SuggestVelocity), (7, Noop), (8, Noop), (9, Noop), (10, AttemptTakeDownTargetable)], // HCDelegate_Big_Black
        0x828f57e8 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, TrackProtectee), (4, SuggestVelocity)], // HCDelegate_Big_Black
        0x828f5838 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, IgnoreTakedownTargetNavRigVolume), (4, IgnoreTakedownTargetNavRigVolume), (5, Noop), (6, SuggestVelocity), (7, Noop), (8, Noop), (9, Noop), (10, AttemptTakeDownTargetable), (11, Noop), (12, Noop), (13, Noop), (14, Noop)], // HCDelegate_Big_Black
        0x828f58c0 => &[(0, DrawDebugUpdateMarker), (1, Noop), (2, EvaluateThreatToProtectee), (3, IgnoreTakedownTargetNavRigVolume), (4, IgnoreTakedownTargetNavRigVolume), (5, Noop), (6, SuggestVelocity), (7, Noop), (8, Noop), (9, Noop), (10, AttemptTakeDownTargetable), (11, Noop), (12, Noop)], // HCDelegate_Big_Black
        _ => return None,
    })
}

/// Preserve context installation even for Noop: native writes mInstance before
/// the folded blr. Unknown source/invalid active slot is never accepted silently.
pub(crate) fn dispatch<E>(source: u32, active: &[u16],
    mut callback: impl FnMut(u16, UpdateCallback) -> Result<(), E>,
) -> Result<(), UpdateDispatchError<E>> {
    let steps = sequence(source).ok_or(UpdateDispatchError::UnknownSource(source))?;
    for &(slot, function) in steps {
        let occurrence = *active.get(slot).ok_or(UpdateDispatchError::MissingSlot(slot))?;
        callback(occurrence, function).map_err(UpdateDispatchError::Callback)?;
    }
    Ok(())
}
#[derive(Debug)]
pub(crate) enum UpdateDispatchError<E> { UnknownSource(u32), MissingSlot(usize), Callback(E) }
