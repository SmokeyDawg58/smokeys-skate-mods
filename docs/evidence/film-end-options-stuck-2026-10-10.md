# Stuck on a film mission's end screen (2026-10-10)

A playtest completed what looked like the career's second film mission. After the highlight
reel the small end-of-challenge screen came up ("Blaow!", A Continue, B Retry) over a white
radial bloom, and no button did anything. All addresses are `sk82_na_m.xex` (Jan 17 2009) VAs,
read in IDA (read-only).

## The film missions in career order

The film challenges are `ChallengeType` 7 (Film), 39 (AnywhereFilm), 40 (SpotFilm), 41
(FriendFilm), 46 (PickaProFilm) and 48 (TeamFilm) in `challenge_global_data`. The career's first
ones are the five sponsor-me tapes, all SpotFilm: `m_1b_01_spotfilm_sponsor_me_01` (record
`sponsor_me_01`) is activated with `hom_tutorial` when `m_1a_07_follow_slappy` is won, then the
progression activates `m_1b_02_spotfilm_sponsor_me_02` (`sponsor_me_02`), `m_1b_03`, `m_1b_04` and
`m_1b_05` (progressioncore Lua). The second film mission is `sponsor_me_02`: one objective
(`sponsor_me_02_4`: two airs over a dumpster, contextual achievements), no film timer,
`KilledItScore` 1200, `TimeToWaitBeforeReplay` 2 s. Every film (and every other spot template
challenge) ends through the same states, so all of them were affected.

## Native

### The film template's end

`sponsor_me_02`'s state graph (the career db's `challenge_states` and their Lua):

- `killed_it` on a win: `ChallengeTimer.AddCountdownTimer(TimeToWaitBeforeReplay)`; its expiry
  runs `Pause.PauseSimulation()` (`cChallengePauseManager::PauseSimulation` 0x825b12f8, which
  pushes `kPAUSE_STATE_CHALLENGE`) and enters `highlight_reel`. A loss waits 1.5 s and enters
  `end_options` without the pause.
- `highlight_reel`: `HighlightReel.PlayHighlightReel(group, false, true)`;
  `highlightreelcomplete` enters `end_options`.
- `end_options`: the results title and options (Continue_Retry, Retry_Quit), then
  `FrontEnd.OpenContinueRetryDialog(win)` (0x82896a58: FE state 8's `mWinState`, the throwdown
  type and the calling challenge, `PushState(8)`).
- `hudcontinue`: `cMsgEnterRealTime`, `Pause.UnpauseSimulation()`,
  `Pause.EnablePauseMenuAndBackpack()`, `Challenge.WinChallenge` or `LoseChallenge`,
  `NIS.KillNIS()`, `complete`. `hudretry`: `cMsgEnterRealTime`, `Pause.UnpauseSimulation()`,
  `NIS.KillNIS()`, `AI.PinAI`, `restarted`.

So a won film holds the simulation paused from its reel until the end screen's Continue or Retry,
and only those two handlers lift the pause.

### The front end runs through every pause

- `GameModule::Update` 0x8273edc8 calls `FrontEndManager::Update` every update (0x8273ee8c, or
  0x8273f02c with `kRunFrontEndDuringPhysics`), whatever `PauseMan` holds: the screens' APT
  movies, `StateManagerFrontEndBase::Update` and the input.
- `StateManagerFrontEnd::ReceiveInput` 0x82656af8: while a popup is open, FE state 34 takes the
  input (once its screen is ready and allows input); otherwise the state on top does.
- `FrontEndState_EndChallengeOptions`: `OnEnter` 0x82658880 opens screen 42 for a win without
  several players (`hudComponents/endchallenge_small`), 45 for such a loss. `ReceiveInput`
  0x8267c798 maps A, B, X and Y to the leaderboard's options and `OptionSelect` 0x82676da0:
  Continue pops the state, and once its outro is done `OnOutroDone` 0x826590c8 sends
  `cMsgHUDContinue`; Retry opens the `eSk8FEPopup_RematchConfirmation` popup, whose yes
  (`ConfirmRematchPopupCallback` 0x82658db0) restarts the challenge, sends `cMsgHUDRetry` and pops
  the state.
- `GameModule::Render` 0x827320f0 renders the world every frame; it skips the frame restore only
  for `IsPaused(kPAUSE_FLAG_DATASYSRESTORE)` (0x10, at 0x82732370). `kFlagsTable` 0x820d8548 gives
  `kPAUSE_STATE_CHALLENGE` (5) 0xFFFFFFE1, without that flag (the NIS state, 11, is 0xFFFFFFF1), so
  under the challenge pause the presentation keeps restoring the last simulated frame.

### What is behind the end screen

- The replay scene ends through `SceneManagerImpl::StopNIS` 0x8286e438: it pauses the scene and
  begins the NIS load-screen transition, a 1 s bloom to white, whose end calls
  `StopNISTransitionInCallback` 0x8286e300: `EndTransition` starts the bloom back from white (1 s)
  and `StopDestroyUnloadNIS` 0x8286dfe0 purges the scene and sends the completion messages in the
  same call. `Scene::Purge` 0x8286a908 clears `mICEPostfxActive` and,
  for a replay-controlling scene without the camera hold (the reel's last highlight),
  calls `Presentation::EnterRealTime` 0x827f1130. `NISManager::EndNIS` 0x828a3200 sends
  `cMsgNISComplete`, whose reel completion opens the end screen. The screen therefore comes up as
  the white starts to fade, and over the next second the paused world appears: the last simulated
  frame (the player pinned at the spot) through the game camera and the game post-fx.
- The challenge fade: `VDWriter::WriteData` 0x827ff8c8 writes fade info only while a challenge
  effect runs, and none runs from the countdown's fade-in to the end screen (`killed_it` starts
  none), so `VisualFX_Animation` 0x82847610 takes no fade.
- The success and failure effects: the `VDWriter` (constructor 0x827feb98) takes
  `cMsgObjectiveComplete` (`ReceiveObjectiveCompleteMsg` 0x827fc728), `cMsgChallengeSuccess`
  (0x827fc738, from `cSignPostManager::WinChallenge` 0x825e0ea0) and `cMsgChallengeFailed`
  (0x827fc748) from the challenge broadcaster. `_PrepareChallengeSuccessData` 0x827fc820 writes
  success info for 1 s after an objective completes and 3 s after `WinChallenge`: the larger of
  two graphs of the challenge-objectives visual treatment (class 0xF332E7155560FC55, collection
  `default` 0xD7EDBD362D7D2152, attributes 0x4F36D46878B3A902 and 0x3470740ACC176507: durations 1
  and 3 at +8, sixteen x at +0x10, sixteen y at +0x50). Every y in the shipped data is 0, so the
  value is 0, and `VisualFX_Animation`'s success branch (0x82847e7c, `bChallengeSuccessInfo` at
  `VDReaderData` +0x3190) blends the success grade by its `functions_success` graph (post_fx
  0x62F60C220B469D2C, attribute 0xED113C540D32B72D: x 0, 1, 5, 10; y 0, 0, 1, 1) at 0, which is
  0. The failure side is the same: `_PrepareChallengeFailedData` 0x827fc9c0 (3 s after
  `cMsgChallengeFailed`, attribute 0x978A5A9DCE1CE26E, all y 0) and the failure branch
  (0x82848378, `bChallengeFailedInfo` at +0x3198) through `function_failure_effect_vs_time`
  (post_fx 0x4064BFB54F7CE275, attribute 0x68375B2CEF46A03D, the same points). The frame behind
  the end screen is `TimeToWaitBeforeReplay` (2 s) after the objective
  completed, past that 1 s, and `WinChallenge` comes with Continue, after the screen. Neither
  effect applies behind this screen.
- `cMsgResultScreenEffect`, which `OnEnter` sends to the presentation for a win (0x8265893c), has
  no native handler: its Hermes handler is only bound for Lua (`BindAllMessageHandlersToLua`
  0x828e97e8) and no other code compares its kind 0xBF6ABED7.

The full white is not native: behind the end screen the original shows the bloom fading back to
the paused world within a second.

## Reproduction (before the fix)

`career_run.sh fw_b 1 260` with `SKATE2_DIAGNOSTIC_MOVIE_SKIP=120`,
`SKATE2_DIAGNOSTIC_CAREER_YES=1`, `SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES=4200` (to
`_END=7900`, `m_1a_01`), `SKATE2_DIAGNOSTIC_CHALLENGE=sponsor_me_02` at tick 8000 and the new
`SKATE2_DIAGNOSTIC_FILM_WIN=120:A`; captures every second from 150 s. The trace:

```
career graph sponsor_me_02: enter killed_it                                        (9102)
career output: ChallengePause { pause: true }
career graph sponsor_me_02: enter highlight_reel
career output: RunReplayNis { scene: "highlight_generic_front", start: 4820, end: 4999, .. }
career graph sponsor_me_02: enter end_options
career output: OpenEndChallengeOptions(EndChallengeOptions { win: true, .. screen: 42 })
diagnostic: film state Some("end_options") nis_paused false overlay true end_ready true (9535)
diagnostic: film end options A at 9595
diagnostic: film state Some("end_options") nis_paused false overlay false end_ready false (9596)
```

and nothing after it: the challenge stayed in `end_options` for the rest of the run. The captures
show "Film Challenge Complete", the reel's bloom to white, the reel, the bloom again, then
"Nailed It, Nailer!" with A Continue and B Retry over a full white radial bloom, frozen from the
A on (the reported screenshot, with another of the 19 `ID_CHALLENGE_COMPLETE_COMMENT` lines).

## Cause

1. Stuck after A. The port's front end is not one per-frame update: the PC client updates the
   marker UI (the APT movies) only on a gameplay tick, while a NIS holds the simulation
   (`career_nis_paused`), while a pausing front-end state is open (`career_overlay_open`), under
   the pause menu or a movie. After the reel the NIS has ended and only
   `kPAUSE_STATE_CHALLENGE` holds the simulation. FE state 8 was open, so its screen came up and
   took A; A popped the state, and from then on no branch updated the front end. The screen's
   outro never ran, `OnOutroDone`'s `cMsgHUDContinue` was never sent, and the challenge pause,
   which only `hudcontinue` or `hudretry` lifts, held for good. Later presses found no state.
2. The white. The same branches drew the world: none redrew it while the challenge pause held
   without a NIS (the pausing-state branch draws only the UI over the retained frame). The last
   world frame drawn was the reel's last, at the peak of its bloom to white, and it stayed.
3. B (Retry), on every end screen. The Retry and Quit confirmations are popups over FE state 8,
   and the port gave a popup input only in `career_input`, which no frame reaches while a
   pausing state is open (that path calls `overlay_input` and returns) or while a NIS or the
   challenge pause holds. The confirmation never took input; its A went to FE state 8's A option
   under it.

None of the recent changes listed for the task caused it: the screen manager's open blocking and
intro gating do not cover FE state 8 (its intro completed, `end_ready` above), the replay restores
worked (the reel played and completed), the post-fx challenge fade was not involved (no fade ran)
and the pause/audio reporting does not route input. The film win path had simply never been run
live (`career-replay-photo-film-2026-10-09.md`, section 3).

## Changes

| File | Change |
| --- | --- |
| `crates/skate2-core/src/session/selected_session_career.rs` | `CareerSession::popup_input`: FE state 34's input before the state on top (`StateManagerFrontEnd::ReceiveInput`), taken by `career_input` and at the start of `overlay_input`, so the popups over FE state 8 answer in every path. `career_simulation_paused`: a NIS or the challenge pause holds the simulation. |
| `apps/skate2-pc/src/live_skating_client.rs` | `finish_redraw`'s NIS branch covers `career_simulation_paused`: the world is drawn (the restored world, the game camera and post-fx once the scene is purged) and the career front end updated and acknowledged while the challenge pause holds. |
| `crates/skate2-core/src/challenge/objectives.rs`, `challenge/system.rs`, `session/selected_session_career.rs`, `apps/skate2-pc/src/live_skating_client.rs` | Diagnostics: `objectives::diagnostic_complete` (an active objective completed as its completion script would, with its `RequiresHighlight`), `ChallengeSystem::diagnostic_complete_active_objectives`, the session's `diagnostic_complete_objectives` and `career_main_challenge_state`, and `SKATE2_DIAGNOSTIC_FILM_WIN=<ticks\|lose>[:A\|:B]`. |

## Verification

- `cargo build --release -p skate2-pc`: builds; no warnings in the changed files.
- `fw_c` (the reproduction's recipe after the fix, captures every 0.5 s): the reel, its bloom
  to white, the end screen ("Nailed It, Nailer!") coming up over the white, the paused world
  showing through within the next capture (the player on the board at the spot, the bloom
  fading), A at 9595, and seven ticks later `EnterRealTime`, `ChallengePause { pause: false }`,
  `KillNis`, `complete` and the autosave (165.8 s); play goes on at the spot.
- `fw_d` (`SKATE2_DIAGNOSTIC_FILM_WIN=120:B`, captures every 0.25 s): B at 9595 opens the "Are
  you sure you want to retry this event?" popup over the end screen; D-pad down and A (9656) give
  `EnterRealTime`, `ChallengePause { pause: false }`, `KillNis` and `restarted`; the challenge
  sets up again (white teleport fade, countdown 3, 2, 1, Go!), runs, is won again (10047), plays
  its reel and opens the end screen ("Money!") over the fading white; A (10571) continues to
  `complete` and the autosave (179.0 s).
- `fp_photo` (the photo recipe: `m_1a_03` and `m_1a_05_photo_rail` with
  `SKATE2_DIAGNOSTIC_PHOTO_SEQUENCE`, `_PHOTO_SPOT` and `_PHOTO_PICK`): the photo template also
  pauses (`ChallengePause` at `display_photo`); the photo selection's RT and Y, then the end
  screen's A (13770) give `EnterRealTime`, the unpause, `KillNis`, `complete`, the unlock
  overlay's A twice and the autosave with the photo's $50 (cash 350), as before.
- `fl_loss` (`SKATE2_DIAGNOSTIC_CHALLENGE=sponsor_me_01`, `SKATE2_DIAGNOSTIC_FILM_WIN=lose:A`):
  an ordinary end screen, without the challenge pause: the objective fails, "Challenge Failed!",
  screen 45 ("Next time!", A Retry, B Quit) under the FRONTEND pause only; A opens the retry
  confirmation, which now takes D-pad down and A, and the challenge restarts; four losses and
  retries in a row.
- `parity.sh fw_parity`: PARITY IDENTICAL (6000 lines).
- No unit tests were added or changed: the career session's input needs the game data to
  construct, and the core test target does not build in the memory available; the live runs above
  cover the paths.

## Follow-up 1: `cMsgChallengeFailed`

### Native

- `cSignPostManager::LoseChallenge` 0x825e0f90 and `AbandonChallenge` 0x825e1028: outside a
  replay (`sGameInfo.isReplayingChallenge`), `mChallengeResults[key]` = 1 (lose) or 2 (abandon),
  `DeductChallengeLosses` 0x825de1e0 (takes back the bet of a betting challenge), then a 0x20-byte
  message of kind 0xCFCC4109, `cMsgChallengeFailed` (the activity key at +0x10, the
  `eFinishResult` at +0x18). The call symbol, `SendMsg<cMsgTerminateStateMachine>`, is a folded
  instantiation; the kind is written at 0x825e0ff4 and 0x825e108c. No `cMsgTerminateStateMachine`
  is sent.
- Its receivers (every `Handler::Create<cMsgChallengeFailed, ...>` and every use of the kind):
  - `AudioSystem::ChallengeFailedHandler` 0x82505740, on the challenge broadcaster
    (`AddDelayedMessageHandlers` 0x82508858): the challenge's `ChallengeType` (its
    `challenge_global_data`, attribute 0x02B828CCDB4785E4, through `GetChallenge`): a lost Demo (2),
    Best Trick (8) or Jam (15) sends `cMsgPlayCameraManCsisEvent` (0x2B009311) with
    `T_405_BstLose`, an abandoned film (7, 39, 40, 41, 46, 48) with `T_801_FilmChalQuit`, the
    parameters zeroed. `CameraManCsisEventHandler` 0x824ff7a0 plays it through
    `PlayCameraManSpeech` 0x824fc3a0 (an unset filmer becomes the challenge's).
  - `VDWriter::ReceiveChallengeFailedMsg` 0x827fc748 (constructor 0x827feae8): the failure time
    starts at 0 (-1 idle), as `ReceiveObjectiveCompleteMsg` 0x827fc728 and
    `ReceiveChallengeSuccessMsg` 0x827fc738 start theirs. Each simulation submit (`DoSimSubmit`
    0x82800120, `WriteData` 0x827ff8c8) evaluates the running times on the challenge-objectives
    treatment (`_PrepareChallengeSuccessData` 0x827fc820: the larger of the objective's 1 s and
    the win's 3 s graph; `_PrepareChallengeFailedData` 0x827fc9c0: 3 s), steps them by
    `kUpdateTime` and writes an info unless its value is negative. `VDReader::ReadData` 0x827fdd78
    restores them with the frame (`bChallengeSuccessInfo` +0x3190, `bChallengeFailedInfo`
    +0x3198), and `VisualFX_Animation` 0x82847610 takes, in order, tazer, Hall of Meat, success
    (0x82847e7c), failure (0x82848378), fade, none. The success and failure branches blend the
    grades, `m_NISNearDist`/`m_NISFarDist` and the bloom threshold and saturation toward the
    success (or failure) post-fx by `functions_success` (`function_failure_effect_vs_time`) at the
    info's value and leave `m_bloomMultiply` at (1, 1, 0, 0). Every value of the shipped treatment
    is 0 and both functions are 0 there, so nothing blends: for 1 s after each objective completes
    and 3 s after `WinChallenge`, `LoseChallenge` or `AbandonChallenge` the bloom's fine, medium
    and coarse intensities are (1, 1, 0) instead of the location's (the default bloom's are 1, 2,
    3), and no challenge fade shows.
  - Lua: `cMsgChallengeFailed_BuildMessageTable` 0x828c08c8 (`ActivityKey` as a float, `Result`
    nil), bound by `BindAllMessageHandlersToLua` 0x828e97e8. No shipped script handles it: none of
    the 627 `challenge_messagehandlers` has kind 0xCFCC4109 (nor `cMsgChallengeSuccess`'s), and
    the progression machines take only their script types.
  - No HUD component and not the progression manager: `cSignPostManager` registers no handler of
    it.
- The scripts: the film template's `hudcontinue` after a loss calls `LoseChallenge` (screen 45's B,
    Quit: `OptionSelect` 0x82676da0 maps Retry_Quit to Quit, which pops the state for
    `cMsgHUDContinue`); its global `abandonchallenge` handler calls `AbandonChallenge`, plays
    `AudioPlayerQuitChallenge` and enters `complete`. `cMsgAbandonChallenge` comes from the
    challenge hub's Quit Challenge (`FrontEndState_StartHub::QuitChallengeCallback` 0x8265e868,
    `IsQuitting` false), Quit Game during a main challenge (`FrontEndManager::ExitToMainMenu`
    0x82628990, `IsQuitting` true), `GameModule::BackToSP`, `ChallengeCASManager::AbandonSetup` and
    the online flows.

### The port before

`signpost::end_challenge_unwon` sent `TerminateStateMachine` of the challenge's key: no state
machine has a challenge's key (233 machine collections, 804 challenges, none shared), so it did
nothing, the Lua post office routed `cMsgTerminateStateMachine` instead, and neither the
cameraman's line nor the Visual Director's failure info ran. The success info was missing too.

### Changes

| File | Change |
| --- | --- |
| `crates/skate2-core/src/challenge/signpost.rs` | `end_challenge_unwon` sends `Event::ChallengeFailed` (key, result). |
| `crates/skate2-core/src/challenge/system.rs` | `Event::ChallengeFailed` and its Lua message; `Output::PlayCameraManCsisEvent`; `World::vd_results`; the flush delivers to `vd_results` and `audio_handlers` before the Lua post office; `ChallengeSystem::submit_result_info`. |
| `crates/skate2-core/src/challenge/audio_handlers.rs` (new) | `ChallengeFailedHandler`. |
| `crates/skate2-core/src/challenge/vd_results.rs` (new) | The `VDWriter`'s objective, success and failure times and their submit. |
| `crates/skate2-core/src/session/selected_session_career.rs` | `CareerHud::result_info`, written once per simulation update (`try_update_career`, before the HUD is collected); `PlayCameraManCsisEvent` to the audio interface. |
| `crates/skate2-core/src/nis/audio.rs`, `nis/manager_tests.rs` | `AudioInterface::play_camera_man_csis_event`, and the test double's. |
| `crates/skate2-audio/src/system.rs`, `speech/mission.rs` | `AudioMessage::PlayCameraManCsisEvent`, handled in group A by `camera_man_csis_event_msg` (`PlayCameraManSpeech` with unset parameters). |
| `apps/skate2-pc/src/game_audio.rs` | The game's audio interface posts it (`SKATE2_TRACE_SPEECH` traces it). |
| `apps/skate2-pc/src/live_skating_view.rs` | `challenge_result`: the success or failure branch ahead of the fade (bloom intensities 1, 1, 0, the grades written); the retained frame carries the tick's info. |
| `apps/skate2-pc/src/live_skating_client.rs` | `SKATE2_DIAGNOSTIC_FILM_WIN=quit`: `cMsgAbandonChallenge` 60 ticks into `challenge_running`. |

### Verification

- `cargo build --release -p skate2-pc`: no warnings in the changed files.
- `fg_retry` (`sponsor_me_02`, `FILM_WIN=120:B`): win, reel, B, the retry popup's yes, the
  countdown, the second win, A: `EnterRealTime`, the unpause, `KillNis`, `complete`, autosave.
- `fg_quit` (`FILM_WIN=quit`): at 9011 `AbandonChallenge` runs from the global handler:
  `PlayNpcSpeech` 725 (`AudioPlayerQuitChallenge`), then `PlayCameraManCsisEvent { function:
  "T_801_FilmChalQuit" }` and `speech: play camera man CSIS event T_801_FilmChalQuit` (the native
  order: the script's speech reaches the audio broadcaster before the flush delivers
  `cMsgChallengeFailed`), `complete`, the challenge hull unloaded, the AI released, autosave.
- `fg_lose` (`sponsor_me_01`, `FILM_WIN=lose:B`): "Challenge Failed!", screen 45, B (Quit) at
  9492; after the outro `EnterRealTime`, the unpause, `LoseChallenge` (a Spot Film: no
  cameraman's line), `KillNis`, `complete` and the autosave `ProcessChallengeResults` makes when
  no state machine takes the result; skating goes on.
- `parity.sh fg_parity` on the final build: PARITY IDENTICAL (6000 lines).
- Not run live: `T_405_BstLose` (a lost Demo, Best Trick or Jam).

## Follow-up 2: "Go!" before the countdown

### Reproduction

A temporary trace of the countdown HUD's screen states (removed again) on `fg_retry`, `fg_quit`
and `fg_lose` shows the same at every countdown, the first and the retry's: state 2 with the
value 4, then 3, 2, 1, 0. `fg_retry`'s captures every 0.04 s from 160 s show the white teleport
fade (54..76), the world (77..83) and "3" from 84; `fg_quit`'s first countdown the same (23, then
"3" at 24). No capture caught the GO: it lasts under 0.04 s. `fw_d`'s capture every 0.25 s had
caught it once.

### Cause

`AddHUDCountdownTimer(PrefilmTime)` (0x82db3c20, `cChallengeTimerManager::AddTimer` 0x82db3a50)
stores `(int)(3.0f * 60.0f)` = 180 ticks, and `GetTime` 0x82da79f0 returns `(float)ticks *
kUpdateTime` (0x3C888889): 180 ticks read 3.0000002 s. `CountdownHUD::StartCountdown` 0x826bc660
takes the ceiling, 4, and sets screen state 2; `Countdown_GetCountdownTime` 0x826b8bd8 gives
"#4", and hudcountdown's `ShowCountdown` shows only "#3", "#2" and "#1" as numbers: anything
else goes to `mcNumber`'s language label, GO (with the count sound). One update later the timer
has 179 ticks (2.98 s) and the count shows 3. The movie is not stale and does not restart from
its last frame: GO is a state of its own, the trace's first.

### Native

The original does the same. The timer is created by the challenge's `teleporttransitionfinished`
handler during `Simulation::PostUpdate`'s flush, and the HUD reads it before its first
decrement: `GameModule::Update` 0x8273edc8 runs `FrontEndManager::Update` (with
`HUDManager2::Update` 0x82cee4f0) between `SimController::Update1` and `Update2`
(`kRunFrontEndDuringPhysics`, default 1), and `Sk8ChallengeSystem::Update` runs in `SimUpdate2`
0x8289f980 (`kRunChallengeUpdateDuringPhysics`, default 0), after it. So the native's first count
is 4 too, and its GO shows for one update. The port shows it one update earlier than the native,
in the update of the flush, because it updates the HUD components after the flush.

No change: the countdown starts as the native's does.

## Follow-up 3: the start hub during a challenge

### Native

- `FrontEndState_StartHub::OnIntroDone` 0x826536e0 picks the hub: a main challenge of type 36
  (`CreateASpot_Setup`) gives `Create_A_Spot_Setup_WithScore`, of type 35 (`CreateASpot`)
  `Create_A_Spot`; then Party Play gives `PartyPlay`; online `Online_Challenge` (with a main
  challenge) or `Online_FreeSkate`; free skate, or a mode other than single player,
  `Offline_FreeSkate`; single player `SinglePlayer`, or `SinglePlayer_Challenge` while a main
  challenge runs, the only hub that calls `SetScreenState(1)` (every other one 0).
  `Create_A_Spot_Setup` is never picked.
- Their rows (`m_eMenuOptions` 0x8208e230..0x8208e2fc, the counts from each vtable's
  `GetNumOptions`, each list led by the debug Debug row, which the port leaves out as before) and
  `IsItemEnabled`:

  | Hub | Rows | `IsItemEnabled` |
  | --- | --- | --- |
  | `SinglePlayer_Challenge` (0x8208e26c) | Restart Challenge, Quit Challenge, Replay, Profile, Trick Guide, Settings, Quit Game | 0x8264c560: Restart Challenge unless the main challenge's `ChallengeType` is 9 (Own the Spot); Quit Challenge with its `challenge_local_data` 0x23F394473183C613 (set for every challenge but `m_1a_01_goto_slappys` and the two without local data, `mcz_mongocorp_hq` and `m_6p_thrasher_photo_pool_04`); Replay with `mbReplayAvailable`; the rest always |
  | `Create_A_Spot` (0x8208e28c) | Spot Info, Replay, Trick Guide, Settings | 0x826446a8 |
  | `Create_A_Spot_Setup` (0x8208e2a0) | Quit Create-a-Spot, Replay, Trick Guide, Settings, Quit Game | 0x82644710 |
  | `Create_A_Spot_Setup_WithScore` (0x8208e2b8) | Spot Info, Replay, Trick Guide, Settings | 0x82644770 |
  | `Online_FreeSkate` (0x8208e2e4) | Save Replay, Settings, Quit Game | 0x8265e928 (Save Replay only in an online free skate) |
  | `Online_Challenge` (0x8208e2f4) | Settings, Quit Game | 0x826447d0 |

  `SinglePlayer`, `Offline_FreeSkate` and `PartyPlay` are as the port had them. Every hub but
  `SinglePlayer` labels its rows with the base `GetOptionText` 0x82640f80.
- `IStartHubState::OnMenuSelect` 0x82674d08: Restart Challenge opens `Options_RetryChallenge`
  (204, "ID_OPTIONS_MENU_RETRY_CHALLENGE"; `RetrySelectedCallback` 0x8265ea20:
  `RestartMainChallenge(0, false)` and the pop), Quit Challenge `QuitChallengeConfirmation` (154,
  "ID_QUIT_CHALLENGE_MESSAGE"; `QuitChallengeCallback` 0x8265e868: `cMsgAbandonChallenge` with
  `IsQuitting` false on the challenge broadcaster and the pop), Quit Game
  `QuitGameConfirmation` (152, "ID_QUIT_GAME_MESSAGE") with a main challenge, else
  `QuitGameConfirmationNoDataLoss` (153) (`QuitPopupCallback` 0x82671678). The four templates
  (`gPopupTemplate` 0x82fdbc58) are "ID_COMMON_WARNING_TITLE", No/Yes, default No, not
  cancellable, and each callback acts on Yes only. Spot Info pushes FE state 50 (0x82675080, also
  `Create_A_Spot`'s); Quit Create-a-Spot cancels the editor's challenge (0x82675008).
- `Sk8ChallengeSystem::ReceiveAbandonChallengeMessage` 0x825a2de0, registered by the system's
  constructor 0x825a7630 (`InitWorkspace`), before the subsystems' handlers and the Lua post
  office: a main challenge abandoned without `IsQuitting`, of a type other than 35 and 36, is
  abandoned (`cSignPostManager::AbandonChallenge`, result 2, `cMsgChallengeFailed`) and the results
  processed (`ProcessChallengeResults` 0x825e10c0). The scripts' global `abandonchallenge`
  handlers run after it; the film template's calls `Challenge.AbandonChallenge` 0x82888710, the
  same `cSignPostManager::AbandonChallenge`, so Quit Challenge sends `cMsgChallengeFailed` twice.
- Quit Game (`QuitPopupCallback`, offline): without a main challenge, in hub mode 0 or 1
  (`mCurrentStartHubMode`: the free-skate or single-player hub) on a saved profile, the hub's screen
  is hidden and the profile saved (`SaveLoadManager::RequestSave(eBeMemcardSave_Profile)`), and the
  save's `OnComplete` 0x82671960 or `OnFail` 0x8266b268 quits. With a main challenge,
  `cChallengeTeleportManager::GetChallengeEndPoint` 0x82da2f08 (with the global data's
  0x89B3580572F41DA3 set, the first of `EndLocation`, `MapStartLocation` and `Location` the global
  data, then the challenge, has) gives the spawn location (`GetSpawnLocation(name,
  GetLocalPlayerID())`; a name the location manager lacks gives the identity at (0, 0.5, -5 (index
  + 1)), 0x82a8dd34) that `cSignPostManager::QuittingOverride` 0x825c4688 makes the progression
  orientation, which `SaveProgressionOrientation` 0x825c7448 then writes instead of asking the
  breadcrumbs. Three challenges have an end point: `c_2a_01_besttrick_street` and
  `c_3a_01_jam_mega` (`EndLocation`, their `signup_locator1`) and `m_7c_03_photo_cover_sm_3`
  (`MapStartLocation`, `challengelocator01`). Then `DoQuit` 0x8266b1a0: `EnableBackground` with
  `FadeToExit` 0x8265e7f0 as its callback, which pops the hub and calls
  `FrontEndManager::ExitToMainMenu` 0x82628990: with a main challenge `cMsgAbandonChallenge` with
  `IsQuitting` true (0x82628a40), `KillNIS` while a NIS runs, and `Sk8ChallengeSystem::QuittingToFE`
  0x825a2bf0 (`cActivityManager::IsQuittingToFE`: `ReevaluatePrereqActivities` 0x82cdbaa8 does
  nothing until the next `GameLoad`); `UpdateExitToMainMenu` 0x82628bc0 enters the front end once
  the hub is gone. FE state 8's Quit (`QuitPopupCallback` 0x82658ed8) calls `QuittingToFE` and
  `EnterFE` itself, without `ExitToMainMenu`.
- `Sk8ChallengeSystem::InitiateAutoSave` 0x825a66c0, behind every autosave the progression asks
  for, sends `cMsgInitAutosave` only in a career game (game mode flags 4).
- The screen: `SetScreenState(1)` runs starthubmenu's `SetState(1)`, `SetMode(1)`:
  `mcChallengeInfo.SetMode(1)`, then `mHub.gotoAndPlay("tall")` (frame 17 removes the short
  backing at depth 1 and shows the tall one at depth 20). challenge_info's `SetMode(1)` (0x4a3c)
  turns every part off, then sets the header "ID_PAUSEMENU_PAUSE" at (`mTitleOffsetX`,
  `mTitleOffsetY` + 7), both 0; the description from `Mission.GetDescription` 0x8260e4e8 (the main
  challenge's 0x6B0BD2BAE508B742 through `FormatStringCustom`), shown with
  `InChallengeIsDescriptionEnabled` 0x8260fd40; `InitObjectives` 0x5399 when
  `IsObjectivesEnabled(false)` 0x82625850, both reading FE state 32's `mKey`, the challenge the
  sign-up poster last showed (`SignUp_GetNumObjectives` 0x826b7ca8, `SignUp_GetObjectiveTitle`
  0x826b7f10); the picker's rows; the wallet (`Cash_GetFormattedCashString`, right of the
  auto-sized label); A Select, B Back. `UpdatePositions` 0x3ec4: shown objectives hide the
  description; from the header down, 10 apart, the description 5 lower (16 per line plus 15), the
  objective rows (26 per line under their title) and the picker; the buttons at 577 and the wallet
  10 above them.
- The music system's handler of the message, `AudioSystem::QuitChallenge` 0x824ff478, sets the
  challenge music's `CHALLENGE_OVER` trigger.

### The port before

The hub was always the game mode's (`SinglePlayer` in a career), with the single-player
`IsItemEnabled` and the pause layout, so a challenge could be neither restarted nor quit from it.
Quit Game called `request_main_menu`: no profile save, no `QuittingOverride`, no `ExitToMainMenu`.
Nothing native received `cMsgAbandonChallenge`, and only `ReceiveTerminateStateMachineMessage`'s
autosave had `InitiateAutoSave`'s career check.

### Changes

| File | Change |
| --- | --- |
| `crates/skate2-core/src/ui/pause_menu.rs` | Options 9, 10, 12 and 13; every hub's rows and `IsItemEnabled` (`PauseMenuSnapshot::item_enabled`); the `StartHub` variants and `for_challenge` (`OnIntroDone`); `MainChallenge`; `screen_state`; `HubPopup::QuitChallenge`. The controller picks the hub as it opens and re-enters, reports Quit Challenge's yes (`take_abandon_confirmed`) and keeps `mbDoingQuit` (`do_quit`, `take_doing_quit`). |
| `crates/skate2-core/src/challenge/start_hub.rs` (new) | The main challenge as the hub reads it, `cMsgAbandonChallenge`, `ExitToMainMenu`'s challenge half, `ReceiveAbandonChallengeMessage`, `GetChallengeEndPoint`, `QuittingOverride` and the in-challenge screen's natives. |
| `crates/skate2-core/src/challenge/system.rs`, `mod.rs` | The receiver first in the flush; the poster's view keeps its challenge as `mKey`. |
| `crates/skate2-core/src/challenge/signup.rs`, `signup_poster.rs` | `SignUpManager::poster` (FE state 32's `mKey`); the poster natives' helpers shared with the hub. |
| `crates/skate2-core/src/challenge/activity.rs` | `is_quitting_to_fe`: set by `QuittingToFE`, cleared by `GameLoad`, read by `ReevaluatePrereqActivities`. |
| `crates/skate2-core/src/challenge/signpost_progress.rs` | `use_override_because_quitting`, which keeps the save off the breadcrumbs. |
| `crates/skate2-core/src/challenge/signpost.rs` | `initiate_auto_save`: the career check for every autosave (`ProcessChallengeResults`, message type 57, a terminated machine): none in Party Play (flags 16) or free skate (2). |
| `crates/skate2-core/src/session/selected_session_career_start_hub.rs` (new) | The session's Quit Challenge, Quit Game (`QuitPopupCallback`: the save or `QuittingOverride`, then `DoQuit`) and `FadeToExit` (`ExitToMainMenu`; the abandon is delivered by a post-update without simulation before the unload). |
| `crates/skate2-core/src/session/selected_session_career.rs` | Declares it; `CareerHud::start_hub`, collected with the HUD while a main challenge runs; the old `main_challenge` helper removed. |
| `crates/skate2-core/src/session/selected_session_public_facade.rs` | The hub's main challenge, Quit Game through `start_hub_quit`, Quit Challenge's abandon, and `FadeToExit` before the unload when the hub's quit faded out. |
| `apps/skate2-pc/src/marker_ui/pause_movie.rs` | `ChallengeLayout` and the challenge mode (`mHub` at `tall`, `SetMode(1)`, `InitObjectives`, `UpdatePositions`); the picker's rows shared by both modes. |
| `apps/skate2-pc/src/marker_ui.rs` | The rows' enabling from the hub's `IsItemEnabled`; the challenge layout for a screen in state 1. |
| `apps/skate2-pc/src/marker_ui/edit_skater_movie.rs` | The popup body breaks at '^' (`SkateAptString` 0x82a70b28), which "ID_QUIT_CHALLENGE_MESSAGE" has. |
| `apps/skate2-pc/src/live_skating_client.rs` | `QuitChallengeCallback`'s address in the `FILM_WIN` comment (0x8265e868). |

### Verification

All runs `career_run.sh` (scratch saves) up to `m_1a_01`, `sponsor_me_02` signed up at 8000
(running at 8950), `SKATE2_DIAGNOSTIC_FILM_WIN=99999` for the challenge states, the hub driven
with `SKATE2_DIAGNOSTIC_CAREER_PAD`, captures every 0.5 s.

- `cargo build --release -p skate2-pc`: builds; no warnings in the changed files.
- `hb_restart` (Start at 9100, A, down, A): the hub in the tall layout: "Pause", "Objectives:"
  with "Air over both Dumpsters" (the poster's challenge has objectives, so no description),
  Restart Challenge (selected), Quit Challenge, Replay Editor (grey), skate.Profile: Career, Trick
  Guide, Game Settings, Quit Game, "Total Cash: $300", A Select, B Back. "Are you sure you wish to
  restart the current challenge?" on No; Yes (9280): `restarted`, `setup`, the countdown at 9282,
  `challenge_running` at 9552.
- `hb_quitch` (down to Quit Challenge, A, down, A at 9340): the popup on No; Yes:
  `start hub abandons the main challenge`, `AutoSave` (the receiver's `ProcessChallengeResults`),
  `PlayNpcSpeech` 725 (the script's `AudioPlayerQuitChallenge`), `PlayCameraManCsisEvent
  T_801_FilmChalQuit` twice (the receiver's and the script's `AbandonChallenge`), `complete`, the
  autosave; skating goes on.
- `hb_quitgame` (Quit Challenge's popup answered No, then up twice to Quit Game, Yes at 9580): "Are
  you sure you want to quit the current challenge? / All unsaved progress will be lost." on two
  lines, the hub back on Quit Challenge; "Are you sure you want to quit? All unsaved progress may
  be lost." (152); after the fade `start hub exits to the main menu (abandon sent: true)`, the
  script's 725 and `T_801_FilmChalQuit` once (`IsQuitting`: no receiver), `complete`, the autosave
  (its orientation from the breadcrumbs), then the main menu.
- `hb_endpoint` (`c_2a_01_besttrick_street`, `SKATE2_DIAGNOSTIC_CAREER_QUIT=9300`): `start hub quit
  overrides the orientation with c_2a_01_besttrick_street_signup_locator1 at [982.5623, 262.0036,
  -1458.91]`, then the exit with the abandon.
- `hb_freeroam` (`SKATE2_DIAGNOSTIC_CAREER_QUIT=7500`, after `m_1a_01`): the single-player hub as
  before (pause layout, Challenge Map first, "Are you sure you want to quit?"); Yes: `start hub
  quit saves the profile`, `career: autosaved`, then the exit without abandon and the main menu.
- `hb_party` (S.K.A.T.E., `SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT=3300`): the Party Play hub as before
  (Restart Challenge, Replay Editor grey, Trick Guide, Game Settings, Quit Game, the 152 popup);
  Yes: the exit with the abandon, the game's graph to `complete`, no autosave, the main menu, then
  free skate.
- `parity.sh hb_parity2`: PARITY IDENTICAL (6000 lines). (`hb_parity` stopped at start-up on an
  allocation failure while the machine had 1.6 GB of commit left.)
- Unit test `pause_menu::quit_tests::challenge_hub_follows_the_main_challenge` added (the hub
  variants, their first rows, Quit Challenge's popup and yes, Party Play's Quit Game popup); not
  run (no test build).

## Follow-up 4: the start hub's open items

### Native

- `StartHubState_SinglePlayer::GetOptionText` 0x8264c788 labels Challenge Map and Slappy's Blog
  from `m_pStartHubOptionDefinitionsNew` 0x82fdfc8c ("ID_START_HUB_CHALLENGE_MAP_NEW", "Challenge
  Map (%d)"; "ID_START_HUB_SLAPPYS_BLOG_NEW", "Slappy's Blog (%d)") while
  `cSignPostManager::GetNewChallengeCount` 0x825c7dd0 or `GetNewBLOGCount` 0x825c7e38 is above 0;
  the other hubs keep the base `GetOptionText` 0x82640f80. challenge_info's `SetMode(0)` shows the
  wallet (`EnableCash`) outside Party Play and free skate (0x49a4), `SetMode(1)` always;
  `UpdatePositions` 0x3ec4 sets it 10 above the buttons (577).
- challenge_info's constructor hides its screen; only `OnIntroDone` 0x826536e0's
  `SetScreenState` lays it out, so the hub's intro plays over the empty backing.
- `QuitPopupCallback` 0x82671678 (yes, offline, no main challenge,
  `MemcardManager::mbIsUsingSavedProfile`, hub mode 0 or 1): input `eInputState_Block`,
  `ShowFEScreen(false)` (`ScreenManagerBase::ShowScreen` 0x82d65ca8 clears the screen's
  `_visible`), then `SaveLoadManager::RequestSave(eBeMemcardSave_Profile)` with the hub as the
  callback; `OnComplete` 0x82671960 and `OnFail` 0x8266b268 call `DoQuit` 0x8266b1a0 (the fade)
  in modes 0 and 1. Mode 0 is the main menu's free skate (`Offline_FreeSkate`), which saves the
  profile the same way. The port has no unsaved-profile mode, so it always saves.
- `ProcessChallengeResults` 0x825e10c0 ends, unless `UseOverrideBecauseQuiting`, with
  `TeleportLocalPlayerToChallengeEndPoint` 0x82da30b8 for the first result's challenge, after
  `InitiateAutoSave` (0x825e1394, then 0x825e13c8): `GetChallengeEndPoint`, then
  `cTeleportManager::TeleportLocalPlayer(location, onboard false, large radius false)` 0x828a92a8
  (0x828a9f40): without a load screen (`TeleportRequiresLoadScreen` 0x828a9420)
  `TeleportPlayer` 0x828a9888 sends `cMsgTeleport`; with one, the teleport flow behind it and
  `cMsgClearSessionMarker`.
- `AudioSystem::QuitChallenge` 0x824ff478, the `cMsgAbandonChallenge` handler
  `AddDelayedMessageHandlers` 0x82508858 puts on the challenge broadcaster: with the music on, the
  environment the default mode (`mDefaultMode`, +0x504) and no chase playing, `mRaceKey` returns
  to `default`, and a target challenge state takes `CHALLENGE_OVER` (11) with ID 0 unless the
  current state already is that (`MusicChallengeData::Equals` 0x824e4120). Only races set that
  state. `ChallengeMusic.PlayMusic` reaches `IAudioInterface::TriggerMissionMusic` (+0x34,
  0x82504f28), which posts `cMsgTriggerMissionMusic` (the countdown first caches the crowd pods);
  `TriggerMissionMusicImp` 0x824fee80 maps the trigger to an `eChallengeTrigger` and, by the
  challenge's `ChallengeType` (19 without a key), plays the contest music for Demos, Best Tricks,
  Mega Ramps, Street Skating and `m_3a_03_besttrick_danny` (`MusicSystem::ChallengeTrigger`
  0x824f76e8 with the challenge's `MusicPlaylist`, the countdown also starting the crowds and
  moving the emitters' contest state) or calls `RaceTrigger` 0x824f7548 for Point-to-Point, Death
  and Spot races: `mRaceKey` becomes the challenge's `AudioRaceTuning`, and a countdown or start
  (later triggers only on a challenge state) becomes the target challenge state.
  `CheckStateReady` 0x8250d1b0 loads the game track and runs `UpdateChallengeState` 0x82507140: a
  new trigger starts the race music (`StartRaceMusic` 0x82502b50: the `aud_race` collection, its
  start intensity, the stereo map, a stream reset, event 0x23BE42D), plays its event
  (three-quarters, straight, win, loss or abort, short jump) or stops the game track (end of round,
  over); while racing a wipeout plays 0x2D6D1FF; `CalculateRaceIntensity` 0x824f7b48 waits the
  tuning's delay in 1/60 steps per update, then takes the rank's intensity in a ranked race or
  follows the board speed (km/h over the top speed, through the speed curve, times 127) by the rise
  and fall steps, 0 off the board while racing, and its integer goes to `PATH_control`. From a win,
  loss or over on, `raceFinishCounter` counts the updates; past the `aud_music` tweak
  0x59BE3547112AAC36 (250) with the stream stopped, the controller returns to 0 and
  `MusicTrigger(ALL_RESET)` puts the environment back. `RestartChallenge` 0x824ff320 (+0x84, from
  `ChallengeAudioManager::RestartChallenge` 0x82da1c48 in `RestartMainChallenge`) resets the
  emitters' contest state while a contest is loaded (`mContestId`), otherwise in a race calls
  `RaceTrigger(CHALLENGE_OVER, 0)` and sets `mbChallengeLoading`.
- Create-a-Spot: a sign-up of a `CreateASpot` (35) challenge opens the Create-a-Spot poster
  (`OpenCASPoster` 0x82667530: FE state 50, `FrontEndState_CreateASpotInfoPopup`); the editor, the
  spot data (`CreateASpotDataManager`), FE states 47 to 53 and the spots' saves (save type 3,
  `mCASpotFileList`) run the challenge. The hubs' Spot Info pushes FE state 50 (0x82675080) and
  Quit Create-a-Spot cancels the editor's challenge (`CancelChallenge` 0x82663080's
  `CAS_CancelEditorConfirm`, 0x82675008).

### The port before

The hub labelled every row with the base text and showed no wallet; it was configured before its
intro; Quit Game saved in the background while the hub stayed drawn; the main menu's free skate
quit without a save; `ProcessChallengeResults` teleported nowhere; nothing delivered
`TriggerMissionMusic` or `cMsgAbandonChallenge` to the audio system, which had no challenge music
state (`CheckStateReady` loaded the game track and stopped there).

### Changes

| File | Change |
| --- | --- |
| `crates/skate2-core/src/challenge/start_hub.rs` | `StartHubView`: the cash, the new challenge and BLOG counts and the in-challenge screen's view. |
| `crates/skate2-core/src/ui/pause_menu.rs` | `option_label` (`GetOptionText`), `shows_cash`; the save's blocked input and hidden screen (`hide_for_save`, `saving`, `save_finished`). |
| `crates/skate2-core/src/session/selected_session_career_start_hub.rs` | The quit's save before `DoQuit` in both hubs (the career's, or the free skate's profile save), and `teleport_to_challenge_end_point`. |
| `crates/skate2-core/src/session/selected_session_career.rs` | The hub's view each HUD collect; the end point, `TriggerMissionMusic`, `QuitChallengeMusic` and `RestartChallengeAudio` outputs to the session and the audio system; the local race ranking; `diagnostic_trigger_mission_music`. |
| `crates/skate2-core/src/session/selected_session_public_facade.rs` | `audio_local_race_ranking`. |
| `crates/skate2-core/src/challenge/signpost.rs`, `system.rs` | The end point output at the end of `ProcessChallengeResults`; the audio outputs; `RestartMainChallenge` sends the audio manager's restart. |
| `crates/skate2-core/src/challenge/audio_handlers.rs`, `mod.rs` | The abandon handler; `mission_music` (`ChallengeType`, `AudioRaceTuning`); the module is public. |
| `crates/skate2-core/src/nis/audio.rs`, `nis/manager_tests.rs` | `IAudioInterface`'s `TriggerMissionMusic`, `RestartChallenge` and `QuitChallenge`; the test double records them. |
| `crates/skate2-audio/src/music/mod.rs` | `eChallengeTrigger`, `RaceTrigger`, `QuitChallenge`'s music half, the race restart, `UpdateChallengeState`, `StartRaceMusic`, `CalculateRaceIntensity`, `MusicChallengeData::Equals` and `CheckStateReady`'s challenge state; `SKATE2_TRACE_AUDIO` lines for the race music. |
| `crates/skate2-audio/src/system.rs` | `MissionMusic`; `cMsgTriggerMissionMusic` and `TriggerMissionMusicImp` (races); `quit_challenge`, `restart_challenge`; `WorldAudioState::local_race_ranking`; the music's local board speed and ranking. |
| `apps/skate2-pc/src/game_audio.rs`, `live_skating_client.rs` | The interface's three calls; `local_race_ranking` in the audio world input; `SKATE2_DIAGNOSTIC_MISSION_MUSIC=<tick>:<trigger>:<challenge>`. |
| `apps/skate2-pc/src/marker_ui/pause_movie.rs`, `marker_ui.rs`, `marker_ui/screen_manager.rs` | `HubRow` (label and count), `HubLayout` (wallet, challenge layout); the screen laid out on the intro's `PauseIntroComplete`; no hub drawn while hidden. |
| `apps/skate2-pc/src/render_window.rs` | `SKATE2_DIAGNOSTIC_CAPTURE_COUNT`: the periodic captures stop after that many. |

### Verification

`cargo build --release -p skate2-pc` builds (no warnings in the changed files). Runs on scratch
saves; captures deleted after reading.

- `hc_free` (career, `SKATE2_DIAGNOSTIC_CAREER_QUIT` after `m_1a_01`): the intro over the empty
  backing, then "Challenge Map (1)", "Slappy's Blog (1)", "Total Cash: $300"; Quit Game's yes: the
  hub gone, the save (129.399), `career: autosaved` (129.448), the save written (129.449), the fade,
  the exit (130.065), the main menu.
- `hc_end` (`c_2a_01_besttrick_street` at 8000, the hub at 9100): the tall layout appears only
  after the intro (backing, then "Pause", the description, Restart Challenge, Quit Challenge,
  "Total Cash: $100"); Quit Challenge's yes: `start hub abandons the main challenge` (158.010),
  `challenge end point c_2a_01_besttrick_street_signup_locator1 at [982.5623, 262.0036,
  -1458.91]` (158.031) and the autosave at [982.5855, 261.84775, -1458.911]; the captures show the
  skater there.
- `hc_fs`, `hc_fs2` (main menu free skate, `SKATE2_DIAGNOSTIC_FREESKATE_QUIT`): the free-skate hub
  (Replay Editor grey, Challenge Map without a count, Trick Guide, Game Settings, Quit Game, no
  wallet); yes: `start hub quit saves the profile`, `free skate: profile saved`, `start hub quit's
  save is written` (46.276 to 46.309 in `hc_fs`; 4.3 s in `hc_fs2`, which captured every 0.05 s),
  the hub hidden throughout, then the fade, the main menu and a second free skate.
- `hc_party` (S.K.A.T.E., `SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT`): the intro, then the Party Play rows
  without a wallet; Quit Game's yes: the exit with the abandon (59.864), no save, the main menu.
- `hc_race2` (`r_1_1_fabio_mem1`, a Death Race with the `default` tuning, after
  `SKATE2_DIAGNOSTIC_CASH=5000`; the poster wants the minimum bet): `run_heat`'s
  `trigger mission music 1 (type 19, race tuning 0xd7edbd362d7d2152)`, `music race trigger 15 ->
  2`, `music race start: aud_race 0xd7edbd362d7d2152, active track 0x12000001`, intensity 40 (all
  at 173.229). Half a second later the game stops: `controlled skating test failed: leaderboard
  HUD time layout is not translated` (the race leaderboard's time layout, see Open).
- `hc_rq3` (`c_2a_01_besttrick_street`, which now runs its rounds, with
  `SKATE2_DIAGNOSTIC_MISSION_MUSIC=8700:1:r_1_1_fabio_mem1`; `SKATE2_AUDIO_DUMP`, as `hc_rq` had no
  output device): the race music from 146.092 (intensity 40); the contest's own triggers (type 8:
  0, 1 and the abort's 5) leave it; Quit Challenge's yes (157.981) → `quit challenge` → `music
  quit challenge: current mode 6, trigger 2` → `music race trigger 2 -> 11, node 2095` (the stop);
  after the end point's load screen the intensity falls by the tuning's 1.7 to 0; `music race reset
  after 251 updates` (165.165). The end point teleport ran behind the load screen (the skater was
  at the contest, 250 m away); the autosave, requested first, kept the contest position. The dump:
  -32 dB before, -26 dB with the race music, -39 dB after the stop.
- `parity.sh hc_parity` (items 1 to 5) and `hc_parity2` (item 6): PARITY IDENTICAL (6000 lines).
- No unit test added. The `marker_ui::screen_manager` test now builds `HubRow`s and the NIS test
  double implements the three new interface calls; neither run (no test build).

## Open

- The port updates the HUD components after the challenge flush, the native before the challenge
  update (Follow-up 2): every HUD component's screen state comes one simulation update earlier.
- `challenge_result` applies what the shipped treatment gives (no blend). Treatment values above
  0 would blend the grades, the NIS distances and the bloom toward the success or failure post-fx
  (`m_NEWcolour_matrix_success`/`_failure`, `m_NEWbloom_success`/`_failure`), which the PC does not
  load.
- `DeductChallengeLosses` (betting) is not ported, as before.
- Create-a-Spot cannot be reached, so its hubs' Spot Info and Quit Create-a-Spot stay unported: the
  session ignores `OpenCreateASpotPoster` (FE state 50, `FrontEndState_CreateASpotInfoPopup`), and
  `CreateASpotDataManager`, FE states 47 to 53, the challenge system's Create-a-Spot side and the
  spots' saves are absent, so no `CreateASpot` or `CreateASpot_Setup` challenge starts. The online
  hubs are never picked.
- The contest music is not ported: `MusicSystem::ChallengeTrigger` (`ENV_CONTEST` with the
  challenge's `MusicPlaylist`), the countdown's crowds and emitter contest state, and `mContestId`
  belong with `PlayMissionSFXImp`'s contest load and unload and the speakers' multi-speaker submix.
  `RestartChallenge`'s race branch is ported but not seen live.
- Races stop at `run_heat`: the leaderboard HUD's time layout (`LeaderboardSetTime`) is not
  translated, which ends the game (`hud_leaderboard_movie.rs`).
- FE state 8's Quit leaves `IsQuittingToFE` unset (no challenge update runs before the unload).
