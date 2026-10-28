# Career: training-area progression, `m_1a_01` to `m_1a_07` (2026-10-09)

Agent D of the 2026-10-09 career port. Addresses are `sk82_na_m.xex` VAs read from the live IDA
database (read-only); data facts are from the career vaults (`ChallengeCore`, `progressioncore`)
through `tools/career`. Nothing here was built or run: the code type-checks
(`cargo check --release -p skate2-core --tests`), and the tests are for the orchestrator to run.

## Progression order (vault data)

The trigger table (`0xf5c8efa024ee3925`: `ChallengeID`, `WinLose`, `StateMachineID`) and the
machines' handler sources give the real order. `m_1a_04` and `m_1a_05` open together; `m_1a_04`
gates nothing.

| Challenge | Machine (`WinLose`) | What it does |
| --- | --- | --- |
| `m_1a_01_goto_slappys` | `Trigger_m_1a_01_goto_slappys` (0) | activates `m_1a_03` |
| `m_1a_02_open_up` | none | progression title `m_1a_02_goto_session_marker`, flag `0x99909de71b39bdf4` set; no machine activates it, so it never runs |
| `m_1a_03_tutorial_fliptricks` | `Trigger_m_1a_03_tutorial_fliptricks` (0) | activates `m_1a_05`, `m_1a_04`, `m_2a_01_tutorial_slappy`; unlocks Reda; "Show Unlocks Screen"; NIS `02_Pros_Arrival`; "pSpawn Pros Arrive" (type 9 → `m_1a_01_postchallenge_playerspawn`); `Terminate` |
| `m_1a_04_tutorial_manuals` | `Trigger_m_1a_04_tutorail_manual` (0) | `Terminate` (the autosave) |
| `m_1a_05_photo_rail` | `Trigger_m_1a_05_photo_rail` (0) | activates `m_1a_06`; "Show Unlocks Screen"; `ChallengeSpeech.PlayProgressionReminder(4004)` (cameraman, 40); `Terminate` |
| `m_1a_06_photo_puzzle` | `Trigger_m_1a_06_photo_puzzle` (0) | activates `m_1a_07`; "Show Unlocks Screen"; NIS `03_Pros_Leave`; "SponsorMeStart" (type 4: character 40, file 2727); "201_Follow_Slappy" (type 5, Skate 101); `Terminate` |
| `m_1a_07_follow_slappy` | `Trigger_m_1a_07_follow_accept` (8, the accept: `CheckStateTriggers` result 5) | `SetupDynamicHullState("slappys", "Unlock")`; `SessionMarker.SetSessionMarker`; `TerminateWithoutSave` |
| | `Trigger_m_1a_07_follow` (0) | activates `m_1b_01_spotfilm_sponsor_me_01` (progression name `sponsor_me_01`) and `hom_tutorial`; "slappys" unlocked; "Unlock Hom" (setting "Hom Available", bool, true); NIS `goto_m_1b_01_goto_school`; `Terminate` |
| | `Trigger_m_1a_07_follow_fail` (4: lose or abandon) | "slappys" locked unless the challenge is complete; `Teleport.TeleportLocalPlayerToLocationName("m_1a_07_follow_slappy_challengelocator01")`; `TerminateWithoutSave` |

Every native these machines call is bound in the progression Lua state.

## Natives and native logic ported

### Objectives manager (`challenge/objectives.rs`)

| Native | Address | Port |
| --- | --- | --- |
| `cChallengeObjectiveGroup::AddPointsObjective` | `0x82d51938` | requirement set; the points objective made once from `global_points_objective`, appended; `ActivateObjectives` |
| zero-requirement points objective in `EvaluateCompletionObjetive` | `0x82d4eb44` | completes without `cMsgObjectiveComplete`, but the pass counts a completion (`OnObjectiveCompleted` runs) |
| `cChallengeObjective::ResetObjectiveData` | `0x82d4cf18` | trigger/sequence volumes, grind data, completed grinds and tricks, session-marker flags, pushes, award flag; `resetCumulativeData` also `mTotalPoints`/`mTotalGrindLength` |
| `cChallengeObjective::ResetObjectiveStatus` | `0x82d4c050` | adds the scoring boundary's `ResetTriggerVolumeCollisionState` (`0x82e22688`, id as float) |
| `cChallengeObjectiveGroup::ResetObjectives` | `0x82d4d4e8` | `ResetObjectiveData(cumulative, false)`, disabled, fresh vars, reactivated |
| `Objectives.ResetObjective` | `0x82d4db98` | `(group, index, resetCumulative = true, resetStatus = true)` |
| `cChallengeObjective::DoPointsValidTest` | `0x82d4b398` | points-valid script in the objective's vars; true adds the points |
| `cChallengeObjectiveGroup::EvaluatePointsValid` | `0x82d4b740` | active objectives, and complete ones among the pass's newly completed (closure `+9`) |
| `OnSequenceComplete` | `0x82d51cf8` | sequence grind length into the total, last points, sequence-complete pass, `EvaluatePointsValid`, group-complete pass, `SequenceCompletePostUpdate` (`0x82d4d038`: mode-4 active objectives reset status) |
| `OnGrindCompleted` / `GrindCompletedMsg` | `0x82d524b0` / `0x82d51140` | grind length, in-sequence total and maximum, `CompletedGrinds` (pathed and local spline IDs, deduplicated, 32), grind-complete pass |
| `PointsAwardedMsg` | `0x82d4cca8` | `EvaluatePointsValid(score, false)` |
| `DoFullCompletionTest` award | `0x82d4d0d8` | `mAwardSequenceCompletePointsImmediately` spent: the skater's `mFinishedSequenceScore` joins `mTotalPoints` |
| `cChallengeObjectiveGroup::GetNumCompletedObjectives` | `0x82d43e10` | `includeOPO`, `includeFractional` |

`Objectives.MakeObjectives` (`0x82d53108`) adds the points objective when `PointRequirement`
(the zeroed default area when absent) is at least 0.

### Objective natives (`challenge/objective_natives.rs`)

`ObjectivePoints.GetSequencePoints` `0x82d46018`, `GetAccumulatedPoints` `0x82d460b0`,
`GetGroupAccumulatedPoints` `0x82d46148`, `IsSequenceCompletePoints` `0x82d461e8`,
`GetGroupPointsRequirement` `0x82d462e8`, `IsSequenceNewlyCompletePoints` `0x82d46250`,
`AwardSequencePointsImmediately` `0x82d46380`; `Grind.DidSpecificGrind` `0x82d464a8`
(`cChallengeObjective::DidSpecificGrind` `0x82d43fe0`: spline ID as float equals the id),
`GetLastGrindLength` `0x82d46538`, `GetTotalGrindLength` `0x82d465a8`,
`GetMaxGrindLengthInSequence` `0x82d46618`, `GetTotalGrindLengthInSequence` `0x82d46688`.

### Challenge natives (`challenge/natives.rs`, `players.rs`, `teleport.rs`, `hud.rs`)

| Native | Address | Port |
| --- | --- | --- |
| `Pause.PauseSimulation` / `UnpauseSimulation` | `0x82888d78` / `0x82888e28` | `Output::ChallengePause` (`cChallengePauseManager` `0x825b12f8`/`0x825b12a0`, `kPAUSE_STATE_CHALLENGE`) |
| `NIS.KillNIS` | `0x8288bd58` | `Output::KillNis` (`NISManager::KillNIS` `0x828a2f88`: `cMsgNISUnloadAll` while `IsInNIS`, `+0x32`) |
| `HUD2.ShowTrickDisplayContext` | `0x8289a420` | "main"'s `TrickDisplayHUDData::SetContextTrickDisplay` (`0x826b2eb0`); default true (ctor `0x826b1b60`, `Reset` `0x826b1bc8`) |
| `HighlightReel.PlayHighlightReel` | `0x82cd7ef8` | `cHighlightGroup::PlayHighlightGroup` (`0x82cd7b68`): a group without pins answers `cMsgHighlightReelComplete` |
| `SessionMarker.Enable/DisableSetSessionMarker` | `0x82dad510` / `0x82db0a88` | the running activity removed from / added to `mSetSessionMarkerOverrides` (`0x82dabee0` / `0x82daefa8`) |
| `SessionMarker.Enable/DisableUseSessionMarker` | `0x82dad5c8` / `0x82db0b40` | `mUseSessionMarkerOverrides` (`0x82dabf78` / `0x82daf060`) |
| `cChallengeTeleportManager::CompleteChallenge` | `0x82dadad8` | the completed activity's overrides go |
| `Teleport.TeleportLocalPlayerToLocationName` | `0x82db0640` | now also bound in the progression state (`cSignPostManager::PerformLuaCBinding` `0x825e4d58`) |
| `Objectives.GetTotalNumObjectives` | `0x82d43388` | points objective included |
| `Objectives.GetAccumulatedPoints` | `0x82d4a008` | `GetAccumulatedPoints` `0x82d43b50` |
| `Objectives.GetCurrentNumCompletedObjectives` | `0x82d49ce8` | `(group, includeOPO = true, includeFractional = true)` |
| `Objectives.GetFractionDenominator` / `SetFractionNumerator` | `0x82d49a38` / `0x82d49ad0` | `0x82d43528` / `0x82d43488` |
| `Objectives.ChangeObjectivePointsRequirement` | `0x82d43408` | group `PointsRequirement` (+0x4C) |
| `Objectives.IsTrackedUniqueTrickGroup` | `0x82d49ea8` | the closure activity's local `UniqueTrickGroup` names a collection |
| `Objectives.GetNumObjectivesRequiresHighlight` | `0x82d49d80` | |
| `Objectives.GetObjectiveFailureHALText` | `0x82d4a140` | |
| `Objectives.GetObjectiveNISPlaybackDefinition` | `0x82d49c18` | `HighlightDefinition` `+0x50` (alternate) or `+0x10` |

### Session (`session/selected_session_career.rs`)

`Output::KillNis` and `Output::ChallengePause` have explicit arms that log them unapplied:
`PauseSimulation` is only called right before replay playback (197 challenges, states
`display_photo`, `killed_it`, `photo`), `NIS.KillNIS` only from the same challenges' end options
after the replay; pushing `kPAUSE_STATE_CHALLENGE` without the replay that ends it would freeze
the player.

## Headless tests (`crates/skate2-core/src/challenge/tests.rs`)

Run with game data: `SKATE2_GAME_ROOT=<game root> cargo test -p skate2-core --release
challenge::tests:: -- --include-ignored`. No test writes a save (`Output::AutoSave` is only an
output).

Harness: `frame()` now gives the system the local skater's frame (player 0) before each update
(the teleport manager and `skate::skater` need it) and answers hull loads after it (the asset
manager waits for `OnComplete`). `sign_up_at_indicator` answers the sign-up choice
(`SignUpHUD::MultipleSignupsCallback`) when other sign-up boxes take the tester in too: none of
these challenges has a `Radius`, so each box is the 40 m default half size, and the training
locators are closer than that (`m_1a_04` and `m_1a_05` are 27 m and 25 m apart on x and z).
Existing tests run the changed harness and should be rerun.

| Mission | Test | Drives |
| --- | --- | --- |
| `m_1a_01` | `first_mission_completes_and_autosaves` (existing) | unchanged |
| `m_1a_03` | `fliptrick_tutorial_completes_and_unlocks` (existing); `after_fliptricks` helper | the helper also asserts the "pSpawn Pros Arrive" teleport |
| `m_1a_04` | `manual_tutorial_completes_and_saves` | sign-up, tail and nose manual 2.5 s, flip trick out of a manual, results, trigger, autosave |
| `m_1a_05` | `photo_rail_runs_to_its_highlight_reel` | sign-up, photographers as players 1/2, countdown, sequence with the spot volume and a 3 m rail grind: objective complete with 600 points, `display_photo`, `ChallengePause`, empty highlight group |
| | `photo_rail_win_unlocks_the_puzzle_and_saves` | `CompleteChallenge(name)` (presentation event 26, `0x825e3098`), trigger, overlay with `m_1a_06`, speech 4004, autosave |
| `m_1a_06` | `photo_puzzle_runs_to_its_highlight_reel` | launch and landing volumes, sequence completes: objective complete with the awarded 800, `display_photo`, `ChallengePause` |
| | `photo_puzzle_win_unlocks_follow_slappy_and_saves` | overlay with `m_1a_07`, `03_Pros_Leave`, speech 2727, Skate 101, autosave |
| `m_1a_07` | `follow_slappy_accept_unlocks_slappys_and_sets_the_marker` | sign-up accept runs the accept machine; challenge starts |
| | `follow_slappy_win_opens_the_next_area_and_saves` | `sponsor_me_01` and `hom_tutorial` available, "Hom Available" 1, `goto_m_1b_01_goto_school`, autosave |
| | `follow_slappy_failure_locks_slappys_and_returns_to_the_locator` | abandon (result 2) + `ProcessChallengeResults`: hull locked, teleport to the locator, no save |

Complete headlessly: `m_1a_01`, `m_1a_03`, `m_1a_04`. `m_1a_05` and `m_1a_06` run to their
highlight reel; their wins and `m_1a_07`'s three trigger machines are driven through the event-26
path. `m_1a_07` itself (heats following Slappy) is not driven: Slappy is an AI skater and the
headless system has no `AiLink`.

## What remains, and why

Photo outro (`m_1a_05`, `m_1a_06`, every photo challenge): `display_photo` waits
`TimeToWaitBeforeReplay`, pauses and calls `PlayHighlightReel`; the pins come from
`ChallengeReplay`/DataMan (`ReplaySystem::request_pin` answers -1), so the group is empty and the
reel ends at once with `cMsgHighlightReelComplete`, which `display_photo` does not handle. The
state waits for `cMsgNISChallengeNotify` from the photo NIS (`PlayHighlight` `0x82cd75e0` →
`NISManager::RunReplayNIS`), whose handler then needs `NIS.GetPhotoNISCount` (`0x8288bc98`) and
`FrontEnd.OpenPhotoSelection` (`0x82896cb0`, the photo selection screen), then `end_options`
(`UnpauseSimulation`, `KillNIS`). Needed: the replay recorder and pins, replay NIS playback, the
photo count and the photo selection screen.

Unbound but not reached in these missions: `Objectives.AddLastUniqueTrickToList` (`0x82d517c8`)
and `HUD2.ABDHUDRefreshFilmTrickList` (`0x8289a088`) only with unique-trick tracking;
`Service.Activate/DeactivateSecurityServiceForChallengeReplay` (`0x8288f7f8`/`0x8288f8a8`) only
for `m_2a_02` and `m_7c_03_photo_cover_sm_3`; `Teleport.TeleportLocalPlayerToSessionMarker`
(`0x82da57d0`) only when an Anywhere Film restarts. `m_1a_07` reaches
`FrontEnd.OpenOnlineContinueRetryDialog` (`0x82896b40`), `NIS.EnterOnlineNISUsingDefinition`
(`0x828958b0`) and `Online.GetSkaterName` (`0x82882f88`) on its online paths only.

Next area: `hom_tutorial` needs `ObjectiveHOM.GetNumBonesGTOEDamageLevel` (`0x82d47dd8`).

## Live wiring

- Objective feed: `MissionSystem::generate_scoring_messages` (`0x82e30368`) builds
  `cMsgSequenceBegin`/`cMsgSequenceComplete`/`cMsgGrindCompleted`/`cMsgPointsAwarded` from the
  PhysOut mission frame (`physical_player_phys_out_mission.rs`) with the field types the handlers
  read; `finished_sequence_score` is in the frame. Trigger volumes reach the objectives as
  `Event::TriggerCollision`; `ResetTriggerVolumeState` clears the session's collision memory.
- Sign-up input: D-pad up/release, the choice, accept and decline are wired in the session.
- AI: the session sets `World::ai_link`, so the photographers and Slappy spawn in game.
- Gaps: `Output::SetSessionMarker` (`cChallengeTeleportManager::SetSessionMarker` `0x82da6e40`)
  and `TeleportPlayer`'s marker move are not applied; `Output::DynamicHullState` is not applied
  (dynamic hulls are not loaded); `selected_session_marker.rs` still answers the challenge rules
  with constants (`main_activity_allows`, `no_overrides`, `message_setting`, `signing_up`), so the
  override lists the `SessionMarker` natives now keep are not read; the port has no
  quit-challenge path, so a photo challenge played live stays in `display_photo`.
