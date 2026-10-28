# Lua natives the shipped scripts call (2026-10-10)

Every native a shipped compiled Lua chunk can call, against what the port binds, and the natives
ported to close the gap. Reference: the debug xex `sk82_na_m.xex` (IDA, PDB names).

## Method

A scratch census parsed every compiled Lua 5.1 chunk the vaults ship (challenge state handlers of
each `challenges` collection, `challenge_objective` scripts, the `progression_statemachines`
handlers, and `global.lua`) and followed `GETGLOBAL module` -> `GETTABLE`/`SELF` constant field
chains, through locals and upvalues that alias a module, to (state, module, function, caller)
references. Those were compared with the binding table (`challenge/binding_table.rs`, 771
natives from `Lua::Manager::PerformLuaCBinding` 0x8289b498 and its module binders) and with the
natives each port state registers (`ChallengeSystem::create_challenge_state` in `system.rs`,
`objective_natives::natives()` for the objectives state, `signpost::natives()` for the
progression state). A caller counts as career when a `progression_challenges` record names its
challenge; `challenge_global_data` gives its global type and the debug-only flag.

| | count |
|---|---|
| natives in the binding table | 771 |
| (state, module, function) references | 521 (518 distinct natives) |
| ported (or bound empty) before this work | 363 |
| unported before this work | 158 |
| ported in this work | 132 |
| still unported | 26 (all Create-a-Spot) |
| bound but never referenced by a shipped script | 253 |
| referenced fields the binding table lacks | 3 |

Message exporters were counted too: the one the scripts call, `cMsgEnterRealTime`, is bound.
Three constant keys that equal a native's name without being tied to its module (`SpotFound`,
`DebugText`, `IsMoving`) are script data (`vars.SpotFound`, `vars.DebugText`, a message's
`IsMoving`), not calls.

## Ported in this work, by file

Every native is a 1:1 reading of its IDA function (address after the name); helpers the natives
reach are ported with them and cite their own addresses in the code.

- `challenge/ai.rs`: `AI.FollowTrickPattern` 0x8288d508 (with `ai/trick_pattern.rs`:
  `TrickPatternMatcher::AddTrick` 0x82df70c0, `TrickPatternItem::MatchesTrick` 0x82df7010,
  `CanSubstituteTricks` 0x82df6008, `TrickPattern::DoesContainTrickType` 0x82df6150,
  `Path::MatchesPattern` 0x824b8258, `Path::MatchesSpinCount` 0x824c1e90,
  `PathManager::GetAIPathIDMatchingTrickPattern` 0x824c7f38; `FollowPathBehavior::SetTrickPattern`
  0x824bd750, `FollowPathMove::UpdateActiveController` 0x824b5730 copying the pattern, and
  `PathController::TrickPatternTrickSwap` 0x824b4c48, which every `Start*Trick` of the
  `FollowTrickPattern` swap mode now runs instead of failing), `Skate.StartSkatePath` 0x8288b340.
- `challenge/assets.rs`: `ChallengeAsset.LoadAdditionalChallengeHull` 0x82e12978,
  `UnloadAdditionalChallengeHull` 0x82e10918.
- `challenge/callouts.rs`: `Callouts.ClearCallouts` 0x82dadd28, `ClearTriggers` 0x82da9ed0,
  `RegisterPlayer` 0x82db53a0, `SetPlayerLeading` 0x82da7bc8, `SetupCalloutHUDData` 0x82db1920,
  `StartTimers` 0x82da7a98, `StopTimers` 0x82da7b30, `TriggerCallout` 0x82db1820,
  `UnregisterPlayer` 0x82dac600.
- `challenge/competitors.rs`: `ChallengeProgression.SendDynamicallyChosenAICompetitorCompletedMessages`
  0x828918e0, `SendDynamicallyChosenAIProBeatenMessage` 0x82891990,
  `GetPickAProCompetitorAudioID` 0x82891f48 (`cCompetitorsManager` reset 0x825af2d8 and
  `ShutdownActivity` 0x825af548; the sign-up poster's decline now resets it).
- `challenge/contest.rs`: `ChallengeProgression.GetDemoSponsorName` 0x82891e70,
  `GetNumberOfDemoCompetitors` 0x82891a40, `SelectDemoCompetitors` 0x82891b18, `Entity.GetSpeed`
  0x82886f90, `GameMode.ContestAISkaterScoreCheat` 0x82885f18,
  `ContestDebugAIScoreAdjustmentEnabled` 0x82885f50, `ContestDebugAIScoreAdjustmentRatio`
  0x82885f88, `ContestPlayerScoreCheat` 0x82885ee0, `ContestSpeedRounds` 0x82885ea8.
- `challenge/crowd.rs`: `Crowd.LoadLargeCrowd` 0x828932c0, `UnloadLargeCrowd` 0x828933c0,
  `LivingWorld.ActivateSpectatorWaypoint` 0x82893da8, `DeactivateSpectatorWaypoint` 0x8288e488.
- `challenge/dmo.rs`: `DMO.RunDMOOperations` 0x82892988.
- `challenge/hom.rs`: `HOM.IsHOMReadyToStartReplay` 0x8288ee80, `HOM.GetHOMStartFrame`
  0x8288ed30, `FrontEnd.OpenHOMResults` 0x82896c80, with `ChallengeHOMManager`'s career half
  (`AddActivity` 0x825bfe10, `GameLoad` 0x825b89b8, `CompleteChallenge` 0x825bfec8, the wipeout
  handlers 0x825b3290 and 0x825b6110).
- `challenge/leaderboard_hud.rs`: `HUD2.ABDHUDRefreshFilmTrickList` 0x8289a088
  (`ABDHUDData::SetListRefresh` 0x826b68c8).
- `challenge/mission_audio.rs`: `ChallengeSFX.PlayChallengeSFX` 0x82889fb8,
  `ChallengeSpeech.OneUpFailureNotification` 0x82895338, `OneUpSuccessNotification` 0x82895298,
  `SendContestPointsEarnedMsg` 0x828953f0, `StartThrowdownChallenge` 0x82895108,
  `StartTurnNotification` 0x828951e0.
- `challenge/natives.rs`: `ChallengeSave.GetOtherChallengeSaveData` 0x82890588,
  `SetOtherChallengeSaveData` 0x828906f0, `NIS.GetPhotoNISCountFromHighlightDefinition`
  0x828930a0, `Objectives.AddPointsObjective` 0x82d535a0, `GetObjectiveHighlightDefinition`
  0x82d49b58, `MakeObjectivesForAI` 0x82d53150, `MakeObjectivesForPlayer` 0x82d53198
  (`MakeObjectives_helper` 0x82d52de0 with the `UNIQUE_TRICK_GROUP` filter).
- `challenge/objective_groups.rs`: `Objectives.AddLastUniqueTrickToList` 0x82d517c8,
  `ChangeObjectiveText` 0x82d49f40, `GetObjectiveHALText` 0x82d4a080, `MakeOnlineBonusObjective`
  0x82d533c0.
- `challenge/objective_natives.rs` (objectives state; data in the new
  `challenge/objective_data.rs`, fed by `objectives.rs` handlers `OnAirCompleted` 0x82d52630,
  `OnGesturePerformed` 0x82d527e0, `OnPlayerResult` 0x82d4a358, `OnPlayerOverallResult`
  0x82d4a2e8, and the trick, sequence and grind handlers, and by `MissionFrame`'s new bail, grind,
  gesture, active-trick and big-trick fields filled in `physical_player_phys_out_mission.rs`):
  `Gesture.DidAGesture` 0x82d486f8, `GetNumUniqueGesturesDone` 0x82d48780, `IsGesturing`
  0x82d487f8, `Grind.IsGrinding` 0x82d466f8, `IsGrindingGUID` 0x82d467c8,
  `ObjectiveAchievements.DidAchievement` 0x82d495e8, `ObjectiveAir.GetAirHeight` 0x82d469e8,
  `GetAirTime` 0x82d46908, `GetDegreesOfSpin` 0x82d46ac8, `GetNumBodyFlips` 0x82d46b38,
  `GetNumSkatersAiredOver` 0x82d46bb8, `GetTotalAirTime` 0x82d46978,
  `ObjectiveContextual.DidAchievement` 0x82d49778, `DidAchievementOfGroup` 0x82d496b0,
  `GetNumAchievements` 0x82d498f0, `ObjectiveGameInfo.GetNumSkaters` 0x82d488d8,
  `ObjectiveHOM.DidSpecificBodyTweakThisFrame` 0x82d47fc8, `GetBoneState` 0x82d47bc0,
  `GetBonusScore` 0x82d47a20, `GetDamageScore` 0x82d47af0, `GetMaxAirDrop` 0x82d475f8,
  `GetMaxAirTime` 0x82d476c8, `GetNumBodyTweaksDone` 0x82d47ef8, `GetNumBonesAtDamageLevel`
  0x82d47cb8, `GetNumBonesGTOEDamageLevel` 0x82d47dd8, `GetNumDMOsHit` 0x82d483e8,
  `GetNumPedestriansHit` 0x82d48098, `GetNumVehiclesHit` 0x82d48240, `GetRotationScore`
  0x82d47950, `GetScore` 0x82d47528, `GetWipeoutSpeed` 0x82d47880, `IsLastBailTop10` 0x82d48590,
  `ObjectiveInfo.NumUniqueTrickDone` 0x82d49558, `ObjectiveLocation.IsInLocation` 0x82d499a0,
  `ObjectiveRound.GetOverallRank` 0x82d46d58, `GetRoundRank` 0x82d46cc0, `GetRoundScore`
  0x82d46c28, `ObjectiveTrigger.EnteredVolumesInOrder` 0x82d4e5e8, `Reset.ClearContextualData`
  0x82d44fd8, `ResetAllObjectives` 0x82d4da68, `Tricks.DidSpecificTrickSequence` 0x82d4e6e0,
  `GetNumTricksDoneInTrickGroup` 0x82d4af28, `IsInCoffin` 0x82d47080.
- `challenge/online.rs`: `NIS.EnterOnlineNISUsingDefinition` 0x828958b0,
  `Online.GetLocalPlayerGamerTag` 0x82882fe0, `GetOTSScore` 0x8288aab8, `GetSkaterName`
  0x82882f88, `GetTopFriendName` 0x8288ac48, `GetTopFriendScore` 0x8288ab80, `IsConnectedReady`
  0x82882f30, `OnlineChallengeRoundSyncPoint` 0x8289adb8, `UploadOTSScore` 0x82883040.
- `challenge/results.rs`: `FrontEnd.CloseOnlineContinueRetryDialog` 0x82896c58,
  `OpenOnlineContinueRetryDialog` 0x82896b40, `Results_SetOnlineIsResultsForLastRound`
  0x828971c8, `Results_SetOnlineTimerID` 0x828976b8.
- `challenge/services.rs` (`cServiceManager` constructor defaults 0x8289db20, `GameLoad`
  0x8289c620): `Service.ActivateSecurityServiceForChallengeReplay` 0x8288f7f8,
  `DeactivateSecurityServiceForChallengeReplay` 0x8288f8a8,
  `HasDrainingServiceBeenUnlockedViaProgression` 0x8288f2a8,
  `HasUncappingServiceBeenUnlockedViaProgression` 0x8288f350, `IsLocalPlayerInNoSkateZone`
  0x8288f158, `IsSecurityServiceAvailable` 0x8288f008, `LoadSecurityTimerHUD` 0x8288f3f8,
  `PlayServiceMovie` 0x8288f200, `SetAllowCICAcceptToContinue` 0x8288f558, `SetServiceAvailable`
  0x8288ef28, `SetServiceIntroductionInProgress` 0x8288f600,
  `StartSecurityTimerHUDAndSpawnBigBlack` 0x8288f4a8, `Telemetry.ExecutedService` 0x828947e0.
- `challenge/signpost.rs`: `ChallengeProgression.SendMessage` 0x82890af0,
  `Progression.DidPlayerBetMax` 0x825c7320, `SessionMarker.ClearSessionMarker` 0x82da9350
  (`cMsgClearSessionMarker` 0x523ed4cc on the teleport manager's broadcaster, which
  `PlayerUI::ReceiveClearSessionMarkerMsg` 0x828aa3f0 turns into `SessionMarker::ClearSessionMarker`
  0x828aa550; queued in order with `cMsgSetSessionMarker` in `session_marker_teleport.rs`).
- `challenge/signpost_progress.rs`: `ChallengeProgression.IncrementPointPoolPoints` 0x82890d40.
- `challenge/skate.rs`: `Skate.SetPlayerRank` 0x82883200 (`PlayersBackEnd::SetPlayerRank`
  0x8255a538).
- `challenge/teleport.rs`: `Teleport.TeleportLocalPlayerToSessionMarker` 0x82da57d0.
- `challenge/teleporter.rs` (new): `HUD2.SetupTeleportHUD` 0x8289c148, `HUD2.ShowTeleportHUD`
  0x82896910, `FrontEnd.ShowYesNoPopup` 0x82897840, `ShowGenericAttributePopup` 0x828979c0,
  `ClosePopup` 0x82897be0, `Teleport.ToWorldFileLocation` 0x82da85f0. The sign-up HUD
  (`signup_hud.rs`) now binds the front active group's data as `FixupActiveComponents`
  0x82cedff0 does, so a teleporter's `eSignUpType_Teleport` data shows the overlay's teleport
  state (`RefreshFEScreen` 0x826bd2d0) and its D-pad up sends `cMsgTeleporterSignUp`
  (`UpdateDPadUp` 0x826bd450, `UpdateComponentInput` 0x82cebc38 gating on visibility). The career
  popup takes a script popup's id and, for the generic popup, its options (the app draws them).

### Files

New: `crates/skate2-core/src/challenge/` `mission_audio.rs`, `contest.rs`, `sponsors.rs` (the demo
sponsor reads `contest.rs` uses), `callouts.rs`, `crowd.rs`, `online.rs`, `objective_groups.rs`,
`objective_data.rs`, `services.rs`, `competitors.rs`, `hom.rs`, `teleporter.rs`;
`crates/skate2-core/src/ai/trick_pattern.rs`; this document.

Changed: `challenge/` `ai.rs`, `assets.rs`, `dmo.rs`, `natives.rs`, `results.rs`, `system.rs`,
`mod.rs`, `activity.rs`, `objectives.rs`, `objective_natives.rs`, `mission.rs`,
`leaderboard_hud.rs`, `skate.rs`, `players.rs`, `signpost.rs`, `signpost_progress.rs`,
`signup_poster.rs`, `teleport.rs`, `attrib_lua.rs`, `signup_hud.rs`, `signup.rs`; `ai/` `mod.rs`,
`tricks.rs`, `path_manager.rs`, `controller.rs`, `path_controller.rs`; `session/`
`selected_session_career.rs`, `players_backend_scoring.rs`, `players_backend_create.rs`,
`session_marker_teleport.rs`; `physics/player/output/physical_player_phys_out_mission.rs`;
`apps/skate2-pc/src/` `live_skating_client.rs` (the sweep diagnostic), `marker_ui.rs` and
`marker_ui/progression_popup.rs` (a generic popup's options).

## What the ported natives still miss

The natives run as the IDA functions do; where they hand off to a subsystem the port lacks, the
hand-off is logged or answers as that subsystem's absence would:

- Global activities never run: `cActivityManager::ReevaluateGlobalActivtyBBoxes` (0x82cdb260)
  and `PerformBBoxTests` (0x82cdaf00) are not ported (`activity.rs` notes the global boxes as
  missing). Natively a full global (`challenge_global_data` global type 2: the Hall of Meat spots,
  the go-tos, the teleporters, `service_security`, `ots_discovery_sp_flow_manager`) runs once its
  prerequisites pass (`EvaluateActivitiesForPrereqs` with `IgnorePrereqForGlobalStart`, then
  `cStateGraph::Run` and each sub-controller's `RunGlobalActivity`: the asset manager's load
  0x82e12838 and the DMO manager's `RunDMOOperations` 0x825c3aa8), and a location global (type 1:
  the uncap and drain services) runs while the skater is inside its box. Natives that only those
  challenges call are ported but are reached in the port only through the sweep diagnostic's
  `+name` (below).
- Services: `cServiceManager`'s sign-up and `OnTimerExpired` and the WorldPainter service
  notifications are not ported, so the service challenges' service states are not reached;
  `Output::BigBlack` (the LW Big Black spawn) falls to the career session's catch-all log;
  `IsLocalPlayerInNoSkateZone` answers false (`cFilmManager::IsInNoSkateZone` is set only by the
  WorldPainter 0x82da7838).
- Own the Spot discovery (`cMsgEnterOTSChallengeBoundary`), the phone list's pick-a-pro and
  throwdown starts, and betting (`DidPlayerBetMax` false: only `SetPlayersBet` 0x825cfaf8 sets it).
- `StatsBackEnd::UpdateBailStats` 0x8257d9a0 (`IsLastBailTop10` false), `ChallengeHOMManager::
  Update`, and FE state 9 (Hall of Meat results; `Output::OpenHomResults` is logged).
- `Teleport.ToWorldFileLocation` to an additional world location
  (`m_08_01_teleport_trainingfacility_out`'s `DIST_TrainingFacility_{0:SponsorWorld}`): the port
  loads no additional world (`GameBackEnd::RequiresAdditionalWorldOperation` 0x8255b898 and the
  flow teleport behind the load screen); the session logs it and does not teleport. The stadium,
  Old Town and Matrix exits are in BAM and teleport.
- Popups: the career shows one popup at a time; script popup ids count the script popups only
  (the native `test` counter at 0x830a7c50 numbers every front-end popup).
- `GetAIPathIDMatchingTrickPattern` walks the path map in the port's bucket order (251 buckets,
  the `fixed_hash_map`'s prime); a native map that grew past its fixed buckets would walk
  differently.

## Called but still unported (26)

All belong to Create-a-Spot (`createaspot_setup`, `createaspot_test`; three DMO natives also in
the debug-only `testing_dmo_reset`). They need subsystems the port does not have:
`Sk8::Challenge::ChallengeCASManager` (score, ghost recording, spawn point save and teleport,
the spot's volumes), `Sk8::FE::CreateASpotDataManager` (high scores, names, personal best),
`cChallengeDMOManager`'s serialisation (`SerializeDMOs`, `DeserializeAndSaveState`,
`RestoreFromSavedState`, `ReleaseSerializeBuffer`, `LockDMOs`, `UnlockDMOs`), and the
Create-a-Spot editor and spot info front-end states.

- `CreateASpot.EnableOutOfBounds` 0x828901d0 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.GetCreateASpotBoundaryVolumeID` 0x8288fa40 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.GetCreateASpotScore` 0x8288fc60 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.GetCreateASpotVolumeID` 0x8288f980 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.GetDMOBufferID` 0x82890320 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.GetHighlightDefinition` 0x82890128 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.GetNextHighScorePlayerName` 0x82897ef0 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.GetPersonalBestScore` 0x82897e80 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.GetSetScore` 0x82897e10 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.RequestNextHighScore` 0x82897dc8 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.ResetGhost` 0x82890078 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.SetCreateASpotScore` 0x8288fd08 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.SetDMOBufferID` 0x82890278 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.StartRecordingGhost` 0x8288ff18 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.StopRecordingGhost` 0x8288ffc8 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.StoreCreateASpotSpawnPoint` 0x8288fbb0 (challenge): createaspot_setup [career]; createaspot_test [career]
- `CreateASpot.TeleportToCreateASpotSpawnPoint` 0x8288fb00 (challenge): createaspot_setup [career]; createaspot_test [career]
- `DMO.DeserializeAndSaveState` 0x82892780 (challenge): createaspot_setup [career]; createaspot_test [career]; testing_dmo_reset [career,debug]
- `DMO.LockDMOs` 0x82892ad8 (challenge): createaspot_setup [career]; createaspot_test [career]
- `DMO.ReleaseSerializeBuffer` 0x82892898 (challenge): createaspot_setup [career]; createaspot_test [career]; testing_dmo_reset [career,debug]
- `DMO.RestoreFromSavedState` 0x82892820 (challenge): createaspot_setup [career]; createaspot_test [career]; testing_dmo_reset [career,debug]
- `DMO.SerializeDMOs` 0x82892700 (challenge): createaspot_setup [career]; createaspot_test [career]
- `DMO.UnlockDMOs` 0x82892b40 (challenge): createaspot_setup [career]; createaspot_test [career]
- `FrontEnd.OpenCreateASpotEditor` 0x82896de8 (challenge): createaspot_setup [career]; createaspot_test [career]
- `FrontEnd.OpenCreateASpotSpotInfo` 0x82896e18 (challenge): createaspot_setup [career]; createaspot_test [career]
- `ObjectiveTrigger.GetCreateASpotVolumeID` 0x82d46420 (objectives): createaspot_setup (objective) [career]; createaspot_test (objective) [career]

## Verification

Builds: `cargo build --release -p skate2-pc` with every change in (the warnings left belong to
other modules). Runs: the scratch `career_run.sh` on a fresh scratch save with
`SKATE2_DIAGNOSTIC_CAREER_YES=1 SKATE2_DIAGNOSTIC_MOVIE_SKIP=120` (New Career, Yes, the intro
movie skipped) and `SKATE2_DIAGNOSTIC_CAREER_QUIT_END=<last tick>` (the career demo otherwise
stops after 2400 frames).

The sweep diagnostic (`apps/skate2-pc/src/live_skating_client.rs`):
`SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP=a,b,...` from `SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP_TICK`
(default 3000), one step per `SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP_PERIOD` ticks (default 1200,
at least 240): the running main challenge is abandoned (`ChallengeSystem::diagnostic_abandon`,
the pause menu's abandon), the next challenge is signed up 60 ticks later and its poster,
dialogs and end options take A, and 30 ticks before the next step the main challenge and its
state are logged. `+name` runs a global activity's graph instead (`diagnostic_run_global`, as
`ReevaluateGlobalActivtyBBoxes` and `PerformBBoxTests` run one), `!name` sends a running
teleporter `cMsgTeleporterSignUp`. Script errors show as `career script error` lines.

- `c_2a_01_besttrick_street`: signed up at tick 3660 and ran `initial`, `setup_code`,
  `nis_overview`, `wait_for_assetload`, `nis_with_characters`, `setup_challenge`, `setup_round`,
  `countdown`, `run_round` (three AI skaters on their paths, `ContestPointsEarned`, callouts,
  speech, music), `end_round` and `results_screen` to `OpenEndChallengeOptions { win: true }`
  with no script error. The app then stopped on its own gap: the results screen
  (`apps/skate2-pc/src/marker_ui/end_challenge_movie.rs`, `set_list_data`) translates
  `eResultsLBType_PlaceNameInt` (0) only, and the contest's results are
  `eResultsLBType_BestTrickNonElimInt` (5): `results_lb layout 5 is not translated`.
- Reached their running states with no script error from a native ported here:
  `c_2a_01_jam_branded` (`nis_overview`), `c_1a_01_bowl_series` (`setup_round`),
  `c_1a_01_besttrick_street` (`run_round`), `m_1a_07_follow_slappy` (`run_heat`),
  `m_4b_01_pro_navarrette_1` (`nis_overview`), `m_1a_05_photo_rail` (`nis_overview`),
  `m_4a_01_skate_carroll_koston` (`pres_overview`), `m_7a_01_demo_dyrdek` (`countdown`),
  `m_08_01_photo_team_shot_1` (`spawn_player`), `m_6p_08_film_pool_team_8` and
  `m_2e_03_photo_tm_burnett` (`challenge_running`), `m_5a_02_photo_thrasher` (`countdown`),
  `f_1a_01_ots_pj01_books_sp` and `f_1a_09_ots_ur06_curves` (`view_nis`), the career's own
  `m_1a_01_goto_slappys` (`challenge_running`); as global activities `f_1a_01_hom`
  (`challenge_running`), `m_2a_01_goto` (`wait_for_location`), `c_wf1_neptune_cap`
  (`wait_for_service`), `service_security` (`challenge_running`),
  `ots_discovery_sp_flow_manager` (`wait_trigger`).
- Teleporters, end to end: `c_3b_01_teleport_stadium_ramps` set up its HUD
  (`wait_for_teleport_volume`); its sign-up opened the generic popup (`ShowPopup` id 0, its four
  ramp titles), A chose the first, and `Teleport.ToWorldFileLocation` took the local skater to
  `megaRampTechStart1` in BAM (the challenge end point path, behind the load screen).
  `c_3b_02_teleport_stadium_out`'s sign-up opened the yes/no popup (id 1); A on No went back to
  `wait_for_teleport_volume`.
- Not started, for gating the port lacks (no script of theirs ran): the betting posters
  (`t_1a_01_1up_location`, `r_2_1_dr_ur_01`: `the betting panel is not ported`) and the posters
  after one in the same run (`t_1b_01_skate_alphabet`, `m_6a_01_follow_spillway`,
  `m_6c_01_profilm_soho`, `r_2_04_deathrace_sp_series_1`); the entry fee
  (`c_3a_01_street_gvr`: `the fee-failed popup is not ported`); sign-up story scenes longer than
  a step (`r_1_1_fabio_mem1`'s `06_Meet_Freeway`, `d_1a_01_spotfilm_rob_dyrdek`'s
  `05a_TheRealGame`). `AI.FollowTrickPattern` was therefore not reached live.
- Script errors seen, neither from a native ported here:
  `m_1a_05_photo_rail/complete/stateenter: attempt to index field 'FilmAIID'` (the sweep
  abandoned it during its opening scene, before the state that creates `vars.FilmAIID`), and `AI.SpawnAIAtLocation: AI skater manager: challenge recipe
  None was not requested` in `m_08_01_photo_team_shot_1` (its one `Competitors` entry is zeroed,
  and `AI.SelectCharacters`, ported before this work, selects no character for it).
- No `unported native` error in any run.
- Parity: `NO_SAMPLER=1 sh parity.sh nat_parity` printed `PARITY IDENTICAL (6000 lines)`.

## Unit tests

None added or changed; none run (the skate2-core unit-test target was not built).

## Full lists

### Referenced fields the binding table lacks (3)

A script reaching one of these calls nil; none is on a career path (an online objective, two unused objectives, a test machine).

- objectives `ObjectiveTrigger.E`: objective:ol_fa_svm_06_accumulation_dwmpbowl
- objectives `Tricks.DidSpecificTrickgroup`: objective:<unused 8b5f2393bbc2b5c3>; objective:<unused f81cf5953ea528be>
- progression `Progression.LockCostByName`: machine:Test End State Machine

### Called and ported before this work (363)

By state and module; `(empty)` natives are bound to an empty body as the IDA function is.

- challenge `AI`: ClearBehavior, ClearTeleports, EnterAiBasedChallenge, FilterPaths, FilterReset, GetCharacterPercentageWillDoABDTrick, GetIfSkaterIsRegularStance, GetNumCompetitors, GetPathCount, LoadPaths, LoadRaceHeatPaths, PinAI, RepositionAI, SelectCharacters, SetAIRespawnBehaviour, SkaterDistanceFromLocation, SpawnAIAtLocation, StartLapPaths, StartPath, UnloadPaths, UnpinAI, UnspawnAI
- challenge `Challenge`: AbandonChallenge, CompleteChallenge, GetChallengeType, GetGameMode, GetPlayerName, GetTeleportFadeInProgress, IsChallengeAboutToStartDuringLoad, IsReplayingChallenge, IsSigningup, LoseChallenge, NotifyChallengeCompleteForObjectives, RemoveAllChallengeVisualIndicators (empty), SendPlayerOverallResult, SendPlayerRoundResult, SetAllChallengeVisualIndicators (empty), SetIsGotoSignupAvailable, WinChallenge
- challenge `ChallengeAsset`: ActivateChallengeAssetLoadScreen (empty), DeactivateChallengeAssetLoadScreen (empty), IsChallengeAssetLoadComplete
- challenge `ChallengeManager`: DisableAllTrickTypes, EnableTrickType, GetNumHumanSkaters, GetSkaterCount, ResetTrickTypeScoring, SetManualsAreConnectors, SetPlayer, SetResetScoring, SwitchCamera, SwitchChallengeCamera
- challenge `ChallengeMusic`: PlayMusic, UpdateLapRaceMusic
- challenge `ChallengePresentation`: AddPresentationEvent, AddPresentationGroup, RemoveAllPresentationGroups, StartPresentation
- challenge `ChallengeProgression`: UnlockUnlockableByName, UpdateCashAward
- challenge `ChallengeReplay`: GetLastFrameNumberWritten, ResetReplayBuffer
- challenge `ChallengeSave`: GetSaveData, SetSaveData
- challenge `ChallengeSpeech`: PlayCameramanSpeech, PlayFollowMeWarningSpeech, PlayNPCSpeech, PlayNPCSpeechUsingAttribute, PlayNPCSpeechUsingAttributeAndSkaterIndex, PlayNPCSpeechUsingDynamicCompetitor, PlayNPCSpeechUsingOverrideAttribute, PlaySkaterSpeech
- challenge `ChallengeTimer`: AddCountdownTimer, AddCountupTimer, AddHUDCountdownTimer, GetCountdownTime, GetCountupTime, PauseTimer, RemoveCountdownTimer, RemoveCountupTimer, RemoveHUDCountdownTimer, ResetCountdownTimer, UnpauseTimer
- challenge `DMO`: ResetChallengeDMOs
- challenge `Debug`: DebugTextPermanent, DebugTextTimed, RemoveTextPermanent
- challenge `DynamicHull`: AllowNonEssentialDynamicHulls, FilterNonEssentialDynamicHulls, SetupDynamicHullState, TestForDynamicHullState
- challenge `Effects`: ResetEffect, StartEffect
- challenge `Entity`: DoesSkaterExist, IsPlayerStanceRegular
- challenge `FollowTheLeader`: GetDistance, GetMaxDistance, GetMinDistance, GetRecoveryCharacter, GetRecoverySpeechID, GetRepeatTime, GetTimeBeforeWarningPlayed, GetWarningCharacter, GetWarningFollowee, GetWarningSpeechID, RegisterDistanceCallback, SetActorIDs, SetWarningInfo, StartDistanceUpdates, StopDistanceUpdates, UnregisterDistanceCallback
- challenge `FrontEnd`: OpenContinueRetryDialog, OpenPhotoSelection, OpenPressADialog, Results_SetActiveResultsMenu, Results_SetColTitle, Results_SetMainTitle, Results_SetNumPlayers, Results_SetPlayerData, Results_SetPlayerStatus, Results_SetPlayerTime, Results_SetResultsMenuType, Results_SetSecondLBColTitle, Results_SetSecondLBNumPlayers, Results_SetSecondLBPlayerData, SetEndChallengeOption
- challenge `HOM`: EnableHOMVisuals, GetHOMScore
- challenge `HUD2`: ABDHUDClearList, ABDHUDRefreshCopyTrick, ABDHUDRefreshSetTrick, ABDHUDRefreshTrickList, ABDHUDSetListVisible, CreateGroup, CreatePreFilmSetupHUD, DPOSetFormatType, DPOSetIcon, DPOSetParams, DPOSetStatus, DPOSetTitle, DeleteGroup, DeletePreFilmSetupHUD, GetGroupHash, LeaderboardHUDInit, LeaderboardHUDReset, LeaderboardHUDSetFloat, LeaderboardHUDSetInt, LeaderboardHUDSetPlayerHighlight, LeaderboardHUDSetRank, LeaderboardHUDSetVariantString, LeaderboardHUDSetVariantType, LeaderboardSetTurn, ObjectiveSetModifiedPosition, SetABDGestureVisible, SetActiveGroup, SetupDistanceHUDData, SetupObjectiveHUDData, Show, ShowCentralMessage, ShowChallengeCompleteMessage, ShowChallengeFailedMessage, ShowModifiedCentralMessage, ShowPoolDrainHUD, ShowPreFilmSetupHUD, ShowSecurityHUD, ShowSkateMessage, ShowTrickDisplayContext, ShowUncapHUD, StartCountdown, StartTimer, TutorialInstructionSet
- challenge `HighlightReel`: AddHighlightUsingDefinition, CreateHighlightGroup, DestroyHighlightGroup, PlayHighlightReel, PlayHighlightReelWithSpecifiedOrder, RemoveAllHighlights, RemoveHighlight
- challenge `LivingWorld`: ForceCullLivingWorld
- challenge `Map`: AddMapIcon, AddObjectiveVisualIndicatorToMap, AddRaceCheckPoint, AddRaceFinish, AddRaceGate, RemoveObjectiveVisualIndicatorFromMap, RemoveRaceObject
- challenge `NIS`: ClearPreloadedNIS, EnterNISUsingDefinition, GetPhotoNISCount, KillNIS, PreloadNIS, ResetNISPlayerIndexes, SubmitNISPlayerIndex
- challenge `Objectives`: ChangeObjectivePointsRequirement, DeleteObjectiveGroup, FailAllObjectiveStatus, ForceObjectiveComplete, GetAccumulatedPoints, GetCurrentNumCompletedObjectives, GetFractionDenominator, GetNumObjectivesRequiresHighlight, GetObjectiveFailureHALText, GetObjectiveNISPlaybackDefinition, GetTotalNumObjectives, IsTrackedUniqueTrickGroup, MakeObjectives, ResetObjective, ResetObjectives, SetEvaluationStatus, SetFractionNumerator, SetPersonalBestScore (empty)
- challenge `Online`: DeInitVoting, GetLocalPlayerID, GetNumOnlineSkaters, InitVoting, IsOnline, IsPlayerDisconnected, LeaveGame, OnlineChallengeComplete, StartVoting, StopVoting, SwitchCamera
- challenge `Pause`: DisablePauseMenuAndBackpack, EnablePauseMenuAndBackpack, PauseSimulation, UnpauseSimulation
- challenge `Race`: AddPlayerToRace, FinishPlayer, GetEstimatedTimeToFinish, GetRank, PlayerDisconnected, RegisterPlayerTimer, ResetRaceStats, SetPlayerProgress, StartRaceStats, StopRaceStats
- challenge `Random`: GetRandomNumber, GetRandomNumberRange
- challenge `SessionMarker`: DisableSetSessionMarker, DisableUseSessionMarker, EnableSetSessionMarker, EnableUseSessionMarker, SetSessionMarker
- challenge `Skate`: AddTrickToAlreadyBeenDoneList, ChallengeCompleted, MatchedPattern, ResetAttemptItem, ResetSetItem, ResetSkateTrackers, UpdateAttemptItem
- challenge `Time`: GetVisualGameTick
- challenge `Transition`: EndTransition
- challenge `Trick`: GetCompletedAir180Count, GetCompletedAirDegreesOfSpin, GetCompletedAirGrabTime, GetCompletedAirHeight, GetCompletedAirHorizontalDistance, GetCompletedAirNumFlips, GetCompletedAirTime, GetCompletedGrindDistance, GetCompletedSequenceScore, GetHOMScore, GetNumTrickType, GetTrickIndexFromScorableID, GetTrickName, GetTrickTypeFromScorableID, IsDoingTrick, IsInAirWithValidLaunchTrick, IsInManual, IsInSequence, IsSkaterInAir, IsSkaterInAirOrGrinding
- challenge `Trigger`: ResetTriggerVolumeState
- challenge `VisualIndicators`: AddFollowIndicator, AddIndicator, AddLocationIndicator, AddTriggerIndicator, ChangeIndicatorColour, ClearAllIndicators, CreateGroup, DeleteGroup, RemoveByType, RemoveFollowIndicator, RemoveIndicator, RemoveTriggerIndicator, SetObjectiveVisualIndicator
- objectives `DMOs`: AddDMOVolumeTester, IsDMOInVolume, IsDMOTesterReady, IsGrabbingDMO, ReleaseDMOVolumeTester
- objectives `Grind`: DidSpecificGrind, GetLastGrindLength, GetMaxGrindLengthInSequence, GetTotalGrindLength, GetTotalGrindLengthInSequence
- objectives `Manual`: IsManualing, NoseManualTime, TailManualTime
- objectives `ObjectiveGameInfo`: SetFractionValue
- objectives `ObjectiveHOM`: GetWipeoutDuration
- objectives `ObjectiveInfo`: GetTotalNumCompletedObjectives
- objectives `ObjectiveLocomotion`: GetNumPushes, IsInAir, IsInSequence, IsMoving, IsOffBoard, IsOnBoard, IsStoppped, SequenceCompleted, WentOffBoard
- objectives `ObjectivePoints`: AwardSequencePointsImmediately, GetAccumulatedPoints, GetGroupAccumulatedPoints, GetGroupPointsRequirement, GetSequencePoints, IsSequenceCompletePoints, IsSequenceNewlyCompletePoints
- objectives `ObjectiveSessionMarker`: JustSetSessionMarker, JustUsedSessionMarker
- objectives `ObjectiveTrigger`: EnteredVolume, EnteredVolumeInCurrentSequence, IsInVolume
- objectives `Tricks`: DidSpecificTrick, DidSpecificTrickGroup, DidSpecificTrickGroupThisFrame, DidSpecificTrickThisFrame
- objectives `Wipeout`: IsWipingOut
- progression `Achievement`: GrantAchievement
- progression `Award`: GrantAward
- progression `ChallengeAsset`: PreloadChallengeAssetsByName
- progression `ChallengeSpeech`: PlayProgressionReminder
- progression `DynamicHull`: SetupDynamicHullState, TestForDynamicHullState
- progression `NIS`: IgnoreTeleportToBreadcrumbOnNextNIS
- progression `Progression`: ActivateChallenge, ActivateChallengeByTitle, ActivateGroup, CheckLastCompletedChallengeTitle, CheckMessageSettingString, CheckRanking, DeactivateChallengeByTitle, DeactivateGroup, ExecuteChallengeByTitle, GetMessageSettingBool, IncrementPointPoolPoints, IsChallengeActive, IsChallengeActiveByTitle, IsChallengeComplete, IsChallengeCompleteByTitle, IsUnlockUnlockedByName, PointPoolPoints, RankingEarned, RevertChallengeByTitle, SendMessage, UnlockGestureByName, UnlockSponsorByName, UnlockUnlockableByName
- progression `Service`: DrainPoolViaProgression
- progression `SessionMarker`: SetSessionMarker
- progression `Teleport`: TeleportLocalPlayerToLocationName
- progression `Unlock`: UnlockCharacterForFreeskate, UnlockLocationForFreeskate

### Bound but never referenced by a shipped script (253)

`(ported)` marks the ones the port binds anyway.

- `AI`: CAIBackToAmbient (ported), EnterAmbientSkaterChallenge, ExitAiBasedChallenge (ported), ExitAmbientSkaterChallenge, ForceLocalPlayerAndAIStop, ForceLocalPlayerStop, GetCharacterModel (ported), GetCharacterName (ported), IsLocalPlayerBeingChased, LoadSKATEProPaths (ported), SetAIBreadCrumbLocation, SpawnSkaterRegular, SwitchOffFollowMeRubberBanding (ported), TriggerNegativeGesture (ported), TriggerPositiveGesture (ported)
- `Achievement`: Grant360AchievementBy360Id
- `Callouts`: AddRoundCallouts, AddRoundTriggers, CancelCallout, GetCompletedCallouts (ported)
- `Camera`: ResetGameplayCamera, SetGameplayCamera
- `Challenge`: AddTriggerVolumeToGroup, AddVisualIndicator (ported), CheckIfCheatEnabled, CheckIfTutorialMode, ClearVisualIndicatorsByType (ported), CreateTemporaryTriggerGroup, DestroyTemporaryTriggerGroup, GetFirstNewSpotFound, IsChallenge, IsShopUnlocked, OTSNewABD (ported), PostNewABD, RemoveTriggerVolumeFromGroup, RemoveVisualIndicator (ported), SetLocationVisual (ported), SetRaceGateVisual (ported), SetTriggerVisual (ported), SpotFound
- `ChallengeAsset`: AreChallengeHullsLoaded, IsChallengeAssetUnloadComplete, LoadChallengeAssets (ported), UnloadChallengeAssets (ported)
- `ChallengeManager`: GetSkaterName, SetResetSequence (ported)
- `ChallengeMusic`: TriggerMissionMusic
- `ChallengeProgression`: AddCash, AddCashWithCreditDefinition, AwardCalloutCash, CheckKilledIt, CompleteChallenge, CompleteChallengeByTitle, GetCash, GetChallengeEconomyDataByTagID, GetChallengeEconomyDataByTagName, GetNumberOfSponsorTeamCompetitors, PointPoolPoints, RevertChallenge, RevertChallengeByTitle, SelectSponsorTeamCompetitors, SetCash, SetKilledIt, SetPlayersBet, SetPointPoolPoints, UnlockSponsorByName (ported)
- `ChallengeReplay`: AddFrames, MinusFrames
- `ChallengeSFX`: PlayOTSSFX, PlaySFX
- `ChallengeSpeech`: AskIfCameramanIsSpeaking, PlayAnnouncerSpeech, PlayContextSpeechUsingAttribute, PlayContextedAnnouncerSpeech, PlayNPCProSpeechUsingOverrideAttribute, PlayOwnSpotSpeechUsingAttribute, PlayPhotographerSpeech, StopCameramanSpeech, StopMainCastSpeech
- `CreateASpot`: ActivateCurrentlyLoadedChallenge, CleanupCreateASpotData, DeleteActiveInstance, GenerateThumbnail, LoadCreateASpotData, ResetActiveInstance, SaveActiveInstance (ported), StartGhostPlayback
- `Crowd`: CheckCrowdGraph, GetCrowdValueAtTime, GetPointsGoal, GetTimeLimit
- `Crowds`: SpawnLargeCrowd
- `DMO`: AreDMOOperationsComplete, ClearDMOs, UnwindDMOOperations
- `DMOs`: IsOnDMO, IsOverDMO
- `Debug`: DebugEnableChannel (ported), DebugEnableGraph (ported), DebugText (ported), Print (ported)
- `Entity`: GetPlayer
- `FollowTheLeader`: ResetDistance (ported)
- `FrontEnd`: LocalizeTextWithParam, OpenOTSInfo, OpenSignUpPoster, Results_SetIsNavigatable, Results_SetSecondLBPlayerStatus, Results_SetSecondLBPlayerTime
- `GameMode`: AddGlobalDemoTime (ported), IsDemo
- `HOM`: GetCompletedChallenge
- `HUD2`: FreezeObjectiveHUD, LeaderboardHUDSetBonusAcheived, LeaderboardHUDSetRanks, LeaderboardSetTime, OTSSetBestFriendScore, OTSSetBestPersonalScore, OTSSetBestWorldScore, OnlineObservingSetObservingPlayerName, Refresh, RemoveGroupFromActive, SecondLeaderboardHUDInit, SecondLeaderboardHUDSetFloat, SecondLeaderboardHUDSetInt, SecondLeaderboardHUDSetRank, SecondLeaderboardSetTime, SecondLeaderboardSetTurn, SetObjectiveVariantHalId, ShowCustomCentralMessage, ShowMiniMap, ShowModifiedCustomCentralMessage
- `LivingWorld`: GetIfPlayerBeingChased (ported)
- `Manual`: IsNoseManualing, IsTailManualing, ManualDistance, ManualTime, NoseManualDistance, TailManualDistance
- `Map`: AddChallengeIcon (ported), CheckMapIcons (ported), HideChallengeIcon (ported), RemoveAllChallengeIcons (ported), RemoveMapIcon (ported), ShowChallengeIcon (ported)
- `NIS`: EnterNISReplayUsingDefinition, EnterReplayNIS, IsNISDefinitionValid
- `ObjectiveAir`: GetTotalAirHeight
- `ObjectiveContextual`: GetNumAchievementsOfGroup
- `ObjectiveGameInfo`: GetFractionValue (ported)
- `ObjectiveHOM`: GetNumDMOsHitOfType, GetNumPedestriansHitOfType, GetNumVehiclesHitOfType
- `ObjectiveLocation`: IsInDistrict
- `ObjectiveLocomotion`: GetPumpingAcceleration, GetSpeed (ported), IsOnGround (ported), WentOnBoard (ported)
- `Objectives`: GetPersonalBestScore, MakeSpecificObjectives
- `Online`: RequestTopFriendScore, RequestUserScore
- `PowerSlide`: IsPowerSliding, PowerSlideDistance, PowerSlideTime
- `Progression`: ActivateList, ActiveChallengesCount, CompleteChallenge, CompleteChallengeByTitle, CompletedChallengesCount, CompletedGroupExitsCount, DeactivateChallenge, DeactivateList, ExecuteChallenge, GetActiveChallengeID, GetActiveChallengeName, GetActiveChallengeTitle, GetChallengeEconomyDataByTagID, GetChallengeEconomyDataByTagName, GetCompletedChallengeID, GetCompletedChallengeName, GetCompletedChallengeOrder, GetCompletedChallengeTitle, GetCompletedGroupExitCountForGroupName, GetCompletedGroupExitGroupID, GetCompletedGroupExitName, GetCompletedGroupExitNameForGroupName, GetCompletedGroupExitNumber, GetCompletedGroupExitNumberForGroupName, GetLastCompletedChallengeTitle, GetMessageSettingInt (ported), GetMessageTypeFromMessageID, GetMessageTypeIDFromMessageID, IsGroupActive, IsSponsorUnlockedByName, ReactivateChallengeByTitle, RevertChallenge, SetPointPoolPoints, TeleportToChallenge, TeleportToChallengeByTitle, TerminateStateMachine, TerminateStateMachineID, TriggerStateMachine, TriggerStateMachineID
- `Race`: GetRacerDistance (ported)
- `SequenceConnector`: SetContestRound
- `Service`: IsSecurityServiceRunning, UncapRailsViaProgression (ported)
- `Skate`: CompareScratchPadPatternItem, GetPlayerSpeed (ported), PrepareForExactTrick (ported), PrepareForTrickType (ported), SetAlreadyBeenDoneListItemLimit (ported), SetPlayerOrder, SetPlayerStatus, SetRoundTime, SetScratchPadPatternIndex (ported), SetScratchPadPatternItem (ported), UpdateCurrentPlayerIndex (ported)
- `Stance`: IsFakie, IsGoofy, IsRegular, IsSwitch
- `Transition`: BeginChallengeTransition, BeginTransition
- `Trick`: DeactivatePattern (ported), DidCompletedAirStartSwitched (ported), DisableWipeoutReset, GetAirGrabTime, GetCompletedAirGrabOccured, GetDegreesOfSpin, GetFEBonesBrokenCount, GetGrindDistance, GetHeight, GetLineScore (ported), GetOverallTrickDistance, GetScorableName (ported), GetTime, GetTweak, IsDoingTrickType, IsIndexMatchTrick, IsLineTimerOn, MatchedPattern (ported), SetPattern, SetPatternLastSequence, SetPatternLastSequenceType, SetPatternLastTrick, SetPatternLastTrickType
- `Tricks`: BeganTrick, BeganTrickGroup
- `VisualIndicators`: Enable (ported), RemoveLocationIndicator (ported)

