# Races and end-of-challenge results, 2026-10-10

## Continuation on 2026-10-11

Race names, dynamic AI NIS lending, race win/quit, GVR screen43, and contest
Next/Previous are implemented. Race and three-round contest/GVR runs passed;
final GVR columns/Quit/teardown passed on release build6. HoM screen46 was
completed in the earlier HoM task.
See [continuation evidence](resume-three-2026-10-11.md) for changes, native
references, run results and remaining scope limits. The original handoff below
is historical.

## Resume here

### Done and verified (career run `race_c`, r_1_1_fabio_mem1, `SKATE2_DIAGNOSTIC_RACE=lose`)
- Leaderboard HUD times: `LeaderboardHUD` time layout no longer aborts; the race HUD shows
  `07.35 / +00.54 / +01.20 / +02.98` with ranks 1-4 (capture checked).
- Race tracking: `Race.StartRaceStats` resolves its start through the session's
  `cLocationManager` (`world.host.location_orientation`), so the race manager tracks and the
  times/positions update.
- NIS playback overrides: the race start NIS (`RaceStarts_2_Slap`) plays at the definition's
  location override (capture of Cuz at the race start); the "unported playback overrides" script
  error is gone.
- Lost race: FE state 8 opens screen 44 (`challengefailed`), end options become ready, the
  diagnostic picks retry and the heat restarts (`setup_heat` -> `run_heat`), per the log's
  `diagnostic: race state` lines.

### Written, compiled, not verified on screen
- AI player names (`UpdatePlayerName` 0x82563a88 AI branch): `challenge/ai.rs`
  `ai_player_names` + `PlayersBackendSettings::try_set_ai_names`. In `race_c` the AI rows were
  still blank: either `CareerHost::localized` -> `career::localized` (selected_session_career.rs,
  searches only the Global and FrontEnd categories) misses `ID_COMMON_*` names, or the names are
  set before something else. First step: log `ai_player_names`' result.
- `marker_ui/results_list.rs`: every `results_lb` layout 0-11 (columns, titles, team pairing,
  GVR totals). c_2a_01 (layout 5) and a Death Race win (layout 3) not seen yet. Right-aligned
  autosize (GVR layouts 10/11) still returns "results right auto-size is not translated".
- `marker_ui/challenge_failed_movie.rs` (screen 44 visuals): opened in race_c, not inspected.
- NIS: restore streamer (`useStreamPositionOnReturn`, outro of r_1_1_fabio_mem1) and a
  "dynamic" record lending the local player (`nis/characters.rs` `instance_characters`).
- Won race (screen 41 + outro NIS + rewards), quit, other race types: not run.

### Half-done / open
- NIS "dynamic" records of AI players are left out (`instance_characters`); native
  `SceneCharacterManager::InstanceCharacter(int)` 0x828769f0 borrows
  `GetSkaterPresEntityFromPlayerId`. Needs: `SceneCharacter.player`, publish_borrowed on the AI
  skater's presentation runtime (selected_session_career.rs ~2115 and public_facade ~2327),
  skipping that skater in `presentation_submission`, borrowed hips/bounds in
  `ai_skater_visuals`.
- Screen 43 `EndChallengeOptions_GVR` (hudComponents/endchallenge_gvr, entry 0x82fdb608,
  placement 4) is untranslated: `marker_ui.rs` `update_end_options` errors for 43. Disassembly in
  the agent scratch `apt/endchallenge_gvr.txt` is not in the repo.
- Screen 46 `HOMResults` (hudComponents/hom_results, 0x82fdb638, placement 0) untranslated.

### Next steps, in order
1. Rebuild (the last build failed only in another agent's poster-betting code,
   selected_session_career.rs ~2589-2649). Then diagnose blank AI names (step above).
2. Win run: `SKATE2_DIAGNOSTIC_RACE=win` (teleports to the next gate every 90 ticks) -> screen 41
   Death Race layout 3, outro NIS `RaceEnd_TiredSkaters` with restore streamer, continue.
3. Quit run: `SKATE2_DIAGNOSTIC_RACE=quit` (cMsgAbandonChallenge at 120 ticks of run_heat).
4. c_2a_01 results (layout 5) via a local `sweep_natives.sh` or
   `SKATE2_DIAGNOSTIC_CHALLENGE=c_2a_01_besttrick_street`.
5. GVR autosize (right alignment) in `results_list.rs` `auto_size`, then screen 43.
6. AI lending in NIS (above). 7. `NO_SAMPLER=1 sh perf/parity.sh <name>` (sim-touching: race.rs,
   ai.rs, NIS manager) must print "PARITY IDENTICAL (6000 lines)" - not yet run.

### Run command used
```
sh career_run.sh race_c 1 300 SKATE2_DIAGNOSTIC_MOVIE_SKIP=120 SKATE2_DIAGNOSTIC_CAREER_YES=1 \
  SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES=4200 SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES_END=7900 \
  SKATE2_DIAGNOSTIC_CASH=5000 SKATE2_DIAGNOSTIC_CASH_TICK=7950 \
  SKATE2_DIAGNOSTIC_CHALLENGE=r_1_1_fabio_mem1 SKATE2_DIAGNOSTIC_CHALLENGE_TICK=8000 \
  SKATE2_DIAGNOSTIC_RACE=lose SKATE2_DIAGNOSTIC_CAREER_QUIT_END=16000 \
  SKATE2_DIAGNOSTIC_CAPTURE_EVERY=5 SKATE2_DIAGNOSTIC_CAPTURE_FROM=160 SKATE2_DIAGNOSTIC_WINDOW_SIZE=1280x720
```
Timeline: heat NIS at tick ~9600, `run_heat` ~10270, `end_heat` ~12970, results ~13270.
`SKATE2_DIAGNOSTIC_RACE` (live_skating_client.rs) logs every race state change.

### Known errors
- AI names blank (above). GVR right autosize error. Screens 43/46 untranslated.
- Unit tests were not run (none built); test call sites of `instance_characters` and the
  `EndOptionsView` literals in marker_ui/tests.rs were updated by hand.

## Native evidence
- Time text: `LocalizerManager::FormatTime`/`BuildTimeFormatString` with `ID_COMMON_TIME_FORMAT`
  ("hh:mm:ss.ff"), flags 1/2/4/16; leaderboard split of seconds as `0x826b97ac`
  (`hud_leaderboard_movie.rs` `split_time`), leader without sign, others `+`.
- `Race.StartRaceStats` 0x828942e8; `cLocationManager::GetLocationPosition` 0x82a8e330.
- `PlayersBackEnd::UpdatePlayerName` 0x82563a88: AI types 2..4; `CheckGenericPlayerName`
  0x825b2518 (no main challenge, empty or `###GENERIC###` short HAL -> "DefaultName" into the
  short name only); else `Snprintf` of `FormatString(GetCharacterShortHAL)` 0x825b2670 (layout
  word 6) and `GetCharacterLongHAL` 0x825b2490 (word 7), prefix skipped.
- End screens: template entries 0x82fdb5e8 (41), 0x82fdb5f8 (42), 0x82fdb608 (43 GVR, placement
  4), 0x82fdb618 (44 `challengefailed`, placement 0), 0x82fdb628 (45), 0x82fdb638 (46
  HOMResults), 0x82fdb648 (47). `challengefailed` class: InitAction char 27 at 0x244c; root
  actions 0x23f8 (construct), 0x287c (IntroComplete), 0x28b4 (OutroComplete); its `mStroke` and
  `mTableBacking` are absent from the movie (AS writes to undefined).
- `results_lb` layouts 0-11 from `controls/results_lb` AS; AVM1 `new Array` element 0 = last
  push (`AptActionInterpreter::_createObject` 0x82a136b0); text `_x/_y` setter adds 2
  (`AptCIH::objectMemberSet` 0x82a16bb4).
- NIS: `EnterNISUsingDefinition` 0x828956e0, `EnterOnlineNISUsingDefinition` 0x828958b0 (restore
  false), `RunBasicNISDefinition` 0x828a8a70 (breadcrumb teleport twice, overrides: location from
  definition +0x10/+0x20 when +0x6, stream position `GetRestorePosition` 0x828a1418 = local
  actor position, player indexes = `mActorIDs`; `RandomPlaybackSpeed`/`RandomCameraTrack` unused),
  `RunBasicNIS` 0x828a8d70, `StartPendingNIS` 0x828a2d30 (`cMsgNISEnterBasic` with overrides),
  `UpdateLoadState` 0x82875bc0 (state 1 `useLocationValues` frame, normal (0,1,0)),
  `StopNISTransitionInCallback` 0x8286e300 / `UpdateNIS` 0x82876608
  (`mWaitStreamerBeforeTransitionOut`), `AddEntities` 0x82877838 (dynamic record i borrows
  `playerIndexes[i]`).

## Files changed (this task)
apps/skate2-pc/src: marker_ui.rs, marker_ui/{localizer.rs, hud_leaderboard_movie.rs,
results_list.rs (new), challenge_failed_movie.rs (new), end_challenge_movie.rs, assets.rs,
clips.rs, tests.rs}, live_skating_client.rs (SKATE2_DIAGNOSTIC_RACE).
crates/skate2-core/src: challenge/{race.rs, results.rs, ai.rs, host.rs, natives.rs, online.rs,
system.rs}, nis/{manager.rs, characters.rs}, session/{selected_session_career.rs,
selected_session_scene_characters.rs, players_backend_create.rs}.
