# Board sounds in cutscenes and the other pauses (2026-10-10)

The skateboard sounds (rolling, wheels, board loops) went on in cutscenes while the
simulation was paused. Natively nothing stops or pauses the skater's sounds: the skater's SFX
objects keep running on the last presentation data, and the Nicotine mixer silences their bus
through two MAIN objects, `SFXObj_Pause` (the simulation paused outside a scene) and `SFXObj_NIS`
(the playing scene's type). The port fed `SFXObj_Pause` only the pause menu's screens, so from
`RunBasicNIS` until the scene started (the NIS transition and load), and again from the scene's
purge until `EndNIS`, the board played on at the speed the simulation stopped with. The unlock
overlay, Skate 101, the end-of-challenge screens and Lua's `Pause.PauseSimulation` did not
silence it at all, and the map, a popup or Settings opened from the pause menu dropped the world
instead of fading it.

## Native

### Nothing on the skater side checks the pause

- `AudioPresentation::DoSimSubmit` 0x827fb2f8 submits the skaters (`FillSkaterData`) without a
  pause test; a paused simulation submits nothing. `GameModule::Render` 0x827320f0 skips
  `DataSysMan::StepTime`/`RestoreFrame` while `IsPaused(kPAUSE_FLAG_DATASYSRESTORE)` (the NIS
  state sets that flag), so in a NIS the audio presentation keeps the last restored skater, speed
  and all.
- `CSTATEMGR_Player::UpdateParams` 0x82556c60 attaches and detaches player states by the
  interpolated skaters only (a local index change, `ghosted`, `IsInRange`);
  `SFXCTL_PlayerPhysics::UpdateParams` 0x8254a400 copies the presentation (`mGameSpeed`
  included) without a pause test.
- `SFXObj_SkateBoard::UpdateParams` 0x8254cff8 and `ProcessUpdate` 0x8254d0b0 (rolling grains,
  skids, squeaks, rattle, active sounds, spider cracks, board slide) take every volume from the
  mixer (`GetVolumeFromMixer`), as do the wheels, contacts, rail, tricks and the other PLAYER
  objects.

So the board keeps rolling at the last speed while paused, and the mixer is what silences it. The
voices never stop, so once the mixer lets go the board is heard again at the skater's speed; there
is no restart logic.

### `PauseMan`

- `Push` 0x827381c8 appends the state and takes `kFlagsTable[top]` (0x820d8548, one word per
  state); `Pull` 0x82733df8 removes every entry of the state and takes the new top's word (0 for
  an empty stack). `IsPaused(flag)` 0x824e4778 is `mMask & mFlags & flag != 0`; `IsActive`
  0x82603b30 is its complement.
- `kPAUSE_FLAG_SIM` (1) is set for FRONTEND (0xFFFFFFE5), CHALLENGE (0xFFFFFFE1), NIS
  (0xFFFFFFF1) and TELEPORT (0xFFFFFFF5), clear for INIT and TRANSITIONMANAGER (0) and
  WORLDPREPARE (0xFFFFFFEC).
- `NISManager::PauseSim` 0x828a1070 pushes `kPAUSE_STATE_NIS` (11), from `RunBasicNIS`
  0x828a8d70 (0x828a8fa0), `RunReplayNIS` 0x828a3c70 and the ICE mode message; `ResumeSim`
  0x828a1170 pulls it from `EndNIS` 0x828a3200 (0x828a3434).
- `StateManagerFrontEndBase::UpdateOnEnter` 0x82d663f0 calls `PauseGame` (vtable +0x24,
  `StateManagerFrontEnd::PauseGame` 0x82647910, `Push(kPAUSE_STATE_FRONTEND)`) for a state whose
  `gFEStateInfo` 0x82fded80 row has `mbPause`; `UpdateOnLeave` 0x82d66300 calls `UnpauseGame`
  0x82647758 (`Pull(kPAUSE_STATE_FRONTEND)`). `ExecutePushState` 0x82d694e8 leaves the state
  below (unless kept) before entering the new one; `ExecutePopState` 0x82d672f8 leaves the top and
  enters (or, kept, re-pauses for) the one below.
- `cChallengePauseManager::PauseSimulation` 0x825b12f8 / `UnpauseSimulation` 0x825b12a0 (Lua
  `Pause.PauseSimulation`) push and pull `kPAUSE_STATE_CHALLENGE` (5).
- `StateLoading::TeleportLoadState` 0x8275fdb8 (map travel) pushes `kPAUSE_STATE_TELEPORT` (25)
  and requests the tips load screen on entry and pulls it on exit.

### What the audio system reads

- `SFXObj_Pause::UpdateParams` 0x82532f48:
  - paused: a `PauseMan` and a `Presentation`, `IsPaused(kPAUSE_FLAG_SIM)`, not in a replay, no
    sample capture, `mCurrentSceneInfo.sceneType == SCENE_TYPE_NONE` and not loading;
  - paused raises input 0 when `mCurrentMenu == eCurrentMenu_Pause`, else input 2 from the second
    paused update (`mbIgnoreFirstOther`); `IsExitingToMainMenu` raises input 0;
  - input 1 is `mSk8HUDState == eSk8HUDState_SignUpPoster`;
  - `mbIsPaused` is `IsPaused(SIM) || IsLoading` with a `Presentation`.
- `ReceiveFEMusicStateChangeMsg` 0x82508378 sets `mFEMusicState` (+0xDF0) and `mbInReplay`, runs
  `MusicSystem::FrontEndTrigger(FE_BEGIN)` and sets `mCurrentMenu = (state ==
  eFEMusicState_PauseMenu)`. The constructor 0x8250dda0 leaves `mFEMusicState` at
  `eFEMusicState_Max` and `mCurrentMenu` at `eCurrentMenu_None`; nothing else in the audio
  libraries writes them. `GetCurrentMenu` 0x824e3aa0 (context value 12) returns `mFEMusicState`,
  and `UpdatePresentationData` 0x82504848 counts `mAwayTime` only while it is
  `eFEMusicState_InGame`.
- `StateManagerFrontEnd::ChangeFEMusicState` 0x82656f78 remaps the row's music state (Options to
  PauseMenu in game, MusicPlayer to InGame, ...) and posts `cMsgFEMusicStateChange` only when the
  result differs from its own `mFEMusicState` (9 from the base constructor 0x82d68230) and is not
  `eFEMusicState_Ignore`. A screen with an Ignore row (the map, a popup, Skate 101, the NIS) keeps
  the state below it: the map opened from the pause menu stays `PauseMenu`.
- `UpdateFEAudioState` 0x82d67948 sends the top state's row and `ChangeFEAudioState` 0x82656e80
  its id (`cMsgFEAudioStateChange`, `ReceiveFEAudioStateChangeMsg` 0x824f2360: `mSk8HUDState`).
- `NotifyNISPlaybackStart` 0x824fe540 (`IAudioInterface` +0x74, `Scene::Init`) sets
  `mCurrentSceneInfo`; `NotifyNISPlaybackFinish` 0x824fe558 (+0x78, `Scene::Purge`) clears it.
  `SFXObj_NIS::UpdateParams` 0x82547ab8 raises one input for `GetNISType` 0x82532b58.

The rows the in-game front end reports (`gFEStateInfo`, 20 bytes: `mbPause`, `mbEnableChyron`,
`mbTransition`, `mbDimmer`, `mShowHUD`, `mbHUDStack`, `mFEMusicState`, `mbInputToActionscript`):

| State | Pause | Music |
| --- | --- | --- |
| NIS 2 | no | Ignore |
| StartHub 7 | yes | PauseMenu |
| EndChallengeOptions 8 | yes | InGame |
| UnlockOverlay 10 | yes | InGame |
| CaC 0x13 | yes | CAC |
| ChallengeMap 0x15 | yes | Ignore |
| MoviePlayer 0x1C | yes | Off |
| MusicPlayer 0x1E | yes | MusicPlayer |
| SignUpPoster 0x20 | no | InGame |
| Popup 0x22 | yes, never applied (second pass) | Ignore |
| PressA 0x26 | no | InGame |
| PartyPlayChangeGame 0x27 | yes | Ignore |
| JackedOverlay 0x2A | yes | JackedOverlay |
| Options 0x49 | yes | Options |
| Skate101 0x4F | yes | Ignore |

### `MixMapSK8.mxb`

Dumped with a parser of the port's `nicotine::map` record layout (envelope lengths in frames of
16.67 ms; every event below is a held ASR):

| Input | Event | Level | Attack / release | Buses |
| --- | --- | --- | --- | --- |
| Pause 0 (pause menu) | MAIN 11 | -100 dB | 11 / 5 | PLAYER sub 0 (and sub 10, master 75), COLLISION, TRAFFICCAR, PEDESTRIAN, EMITTER, DYNAMICOBJECT, WHOOSH, NISCHARACTER sub 0, MAIN masters 0, 36, 41 |
| Pause 1 (sign-up poster) | MAIN 53 | -100 dB | 14 / 5 | PEDESTRIAN sub 0, MAIN masters 25, 36 |
| Pause 2 (any other pause) | MAIN 15 | -100 dB, scaled by 1 - security speaking | 2 / 5 | PLAYER, COLLISION, TRAFFICCAR, PEDESTRIAN, DYNAMICOBJECT sub 0 |
| NIS 5 (GOTO, spot and photo intros, a live best trick, film or 1-up) | MAIN 5 | -100 dB | 17 / 23 | PLAYER, COLLISION, TRAFFICCAR, DYNAMICOBJECT sub 0 |
| NIS 0 (STORY, PRO_CHALLENGE_INTRO) | MAIN 50 | -100 dB | 17 / 29 | the same four |
| NIS 4 (RACE_START) | MAIN 60 | -100 dB | 5 / 5 | the same four |
| NIS 10 (tutorial intro, tutorial failure) | MAIN 85 | -100 dB | 17 / 23 | the same four |
| NIS 1 (movie), NIS 9 (loading) | MAIN 4, 10 | -100 dB | 14 / 23, 2 / 5 | the world buses and MAIN sub 0 |

Sub channels sum their inputs' millibels. The replay-type scenes (HIGHLIGHT, PRO_CHALLENGE_OUTRO,
a replayed best trick, film or 1-up) have no PLAYER duck: the replayed skater is meant to be
heard; the photo outro (NIS 2) only bumps the PLAYER bus (+5.11 dB, an unheld 5-frame attack and
release).

So natively, in a GOTO or STORY scene:

1. `RunBasicNIS` pauses the simulation; the second paused audio update raises Pause 2, whose
   2-frame attack silences the board.
2. `Scene::Init` sets the scene type: Pause 2 drops to 0 (release 5 frames) while the NIS duck
   attacks (17 frames). The sum of the two leaves the bus some 6 to 20 dB down for about 0.2 s,
   then silent for the scene.
3. `Scene::Purge` clears the scene type; Pause 2 is raised again until `EndNIS`.
4. `ResumeSim`: Pause 2 releases over 5 frames and the scene's duck over 23, so the board comes
   back within about 0.4 s at the speed the skater has.

The pause menu is Pause 0 (a 183 ms fade); every other pausing screen, the challenge pause and a
NIS's simulation pause outside the scene are Pause 2 (a 33 ms drop).

## Port before

- `GameAudioState::paused` was the reported front-end row's `mbPause`, and
  `frontend_audio_state` reported only the movie player, the editor and the pause menu's screens.
  The NIS's `kPAUSE_STATE_NIS`, the challenge pause and the career's screens never reached
  `SFXObj_Pause`.
- `SFXObj_Pause` tested the requested row's music state (`fe_music_state == 3`) instead of
  `mCurrentMenu`, so the map and a popup (Ignore) and Settings (Options, remapped to PauseMenu in
  game) switched the pause menu's fade to the drop. `GetCurrentMenu` and `mAwayTime` read the same
  raw row.
- Live, before (`nbs_push0`, rolling at 8.19 m/s into `Goto_m_1a_01_goto_slappys_outro_2`): from
  `PlayNis` the board kept `out1` 6550 to 6880 (volume 0.14 to 0.21) for 1.05 s, until the scene
  started and its NIS duck took over.

## Changes

- `crates/skate2-core/src/session/selected_pause_manager.rs`: `try_is_paused` (`IsPaused`
  0x824e4778).
- `crates/skate2-core/src/session/selected_production_input_owner.rs`:
  `try_is_simulation_paused` (`IsPaused(kPAUSE_FLAG_SIM)` over the pause stack).
- `crates/skate2-core/src/session/selected_session_public_facade.rs`: `audio_paused`, the port's
  `PauseMan::IsPaused(kPAUSE_FLAG_SIM)`: the stack (FRONTEND, NIS), the career's challenge pause
  (kept beside the stack) and the reported front-end row's `mbPause` (for screens the port shows
  without a stack push). In the main menu only the reported row counts: the port's main menu
  holds FRONTEND on its stack for its pinned player, which no native main menu row does.
  `frontend_audio_state` falls back to the career's screens.
- `crates/skate2-core/src/session/selected_session_career.rs`:
  `CareerSession::frontend_audio_state`: press A (0x26), the pausing state on top (Unlock 10,
  Skate 101 0x4F, EndChallengeOptions 8, PartyPlayChangeGame 0x27), the sign-up poster (0x20),
  then `FrontEndState_NIS` (2) while it is on the stack. (This build also reported the career's
  popups as the Popup row, 0x22; the second pass removed that.)
- `crates/skate2-core/src/ui/mod.rs`: those rows as `FrontendAudioState` constants.
- `crates/skate2-audio/src/system.rs`: `fe_music_state` (`mFEMusicState`, Max) and
  `current_menu_pause` (`mCurrentMenu`), set by `receive_fe_music_state_change`
  (`ReceiveFEMusicStateChangeMsg`) whenever the music system's `ChangeFEMusicState` posts a new
  state; `SFXObj_Pause`, `mAwayTime` and the contexts' `GetCurrentMenu` read them.
  `SKATE2_TRACE_AUDIO` prints a line whenever the pause state, the pause inputs, the scene, the
  NIS type, the HUD state or the posted music state change.
- `crates/skate2-audio/src/music/mod.rs`: `fe_music_state()` and `fe_state::MAX`.
- `apps/skate2-pc/src/live_skating_client.rs`: `paused` is `audio_paused()`;
  `SKATE2_DIAGNOSTIC_CAREER_PAD=<from>-<to>[:<hex buttons>][;...]` holds buttons (A, a push, by
  default) over career demo ticks.

## Verification

- `cargo build --release -p skate2-pc` builds.
- Career demo (`career_run.sh` with `SKATE2_DIAGNOSTIC_MOVIE_SKIP=120
  SKATE2_DIAGNOSTIC_CAREER_YES=1 SKATE2_TRACE_AUDIO=1 SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES=2600`,
  fresh scratch saves), the `audio pause` and `board slot` trace lines:
  - The start's go-to NIS (`nbs_fix1`, standing skater): `RunBasicNIS` paused, Pause 2 one update
    later with HUD state 2, the scene (type 26, NIS 3), Pause 2 again after the purge, then the
    "New to skate?" popup, which this build reported paused (0x22) until Yes.
  - Rolling into `Goto_m_1a_01_goto_slappys_outro_2` at 8.20 m/s (`nbs_fix1`, pushes from
    `SKATE2_DIAGNOSTIC_CAREER_PAD`): `out1` 6766 on the frame before `PlayNis`, 0 four frames
    later (Pause 2), 3075 and 1958 in the crossover as the scene started, 0 to `EndNIS`, then
    1775, 4172 and 6447 over the next 0.4 s. The same for `outro_3` (6.6 m/s) and `outro_4`
    (5.1 m/s), and in `nbs_fix2` for `outro_2` at 9.7 m/s. `01_Welcome_Back` and
    `02_Pros_Arrival` (STORY, NIS 0) and the highlight replays (HIGHLIGHT, NIS 1: no duck in the
    scene) followed the same pause steps.
  - The pause menu while rolling at 8.7 m/s (`nbs_fix1`): Pause 0 with the posted PauseMenu,
    `out1` 1872 after 0.12 s and 0 after 0.22 s, back on closing. The map from the pause menu
    (`nbs_fix2`): HUD state 0x15 with the music state still PauseMenu and Pause 0 held, then back
    to the hub and closed.
  - The sign-up poster for `m_1a_03` (`nbs_fix2`): HUD state 0x20 and Pause 1, not paused.
  - The unlock overlay at `m_1a_03`'s end (`nbs_fix2`, `nbs_fix3`): paused with HUD state 10,
    Pause 2 one update later, `out1` 278 then 0, silent until `02_Pros_Arrival` took over the
    pause.
- `parity.sh`: PARITY IDENTICAL (6000 lines).
- `cargo test --release -p skate2-audio --lib`: 73 passed, 7 ignored.
- Added, not yet run (the skate2-core test build ran out of memory):
  `selected_pause_manager_tests.rs` `is_paused_reads_the_top_state_entry` (NIS pauses SIM and
  DATASYSRESTORE, FRONTEND only SIM, TRANSITIONMANAGER on top pauses nothing).

## Open

- The popups and the simulation: see the second pass below.
- Map travel: the native pauses with `kPAUSE_STATE_TELEPORT` behind the tips load screen
  (`IsLoading`, NIS 9). The port travels under its fade without a load screen, so its travel is
  paused only while the map is open.
- In the main menu only the movie player is reported (as before); the native main menu's CaC and
  Options rows also pause and set their music states.
- `FrontEndState_PhotoChallenge` (5) is not reported (it does not pause; a photo outro's NIS
  pause covers it).

## Second pass: popups, the pausing screens and the simulation

The first pass's open item had the career's popups pausing the simulation natively. They do not:
popups are not front-end states and pause nothing, except the several-challenges popup, which
pushes the pause itself. The screens whose rows pause (the unlock overlay, Skate 101, the
end-of-challenge options, change game) already pushed `kPAUSE_STATE_FRONTEND` in the port. What
the port lacked was the several-challenges popup's pause and what any pause does to the player's
input.

### Native

- `StateManagerFrontEndBase::OpenPopup` 0x82d6aaa0 adds the popup to `mPopupList` (by
  priority); `StateManagerFrontEnd::UpdatePopupList` 0x82647998 copies the top one into FE state
  34 and calls its `OnEnter` directly, and `ClosePopup` 0x82d68f40 calls `OnLeave` and shows the
  next. Neither goes through `ExecutePushState`/`UpdateOnEnter`, so row 34's `mbPause` is never
  read: no `PauseGame` and no `UpdateFEAudioState`; the HUD and music states stay.
  `FrontEndState_Popup::OnEnter` 0x826583d0 shows the screen and sounds `core_popup`.
- `StateManagerFrontEnd::ReceiveInput` 0x82656af8 gives an open popup the input and nothing else:
  no `CheckForOptions`, so Start does not open the pause menu.
- The progression dialog ("New to skate?": `cSignPostUtil::HandleShowDialogEvent` 0x825ccf78,
  `cMsgProgPopupOpener`, `FrontEndManager::ReceiveProgPopupOpenerMsg` 0x82638748, `OpenPopup`)
  pauses nothing.
- `SignUpHUD::UpdateDPadUp` 0x826bd450, with several challenges under the prompt, opens the
  several-challenges popup (cancellable, up to four titles), pushes `kPAUSE_STATE_FRONTEND`
  itself (0x826bd738) and hides the sign-up HUD; `SignUpHUD::MultipleSignupsCallback` 0x826b5430
  signs the chosen challenge up (`PlayerTriggerSignUp`) or, cancelled, shows the HUD again, and
  pulls the pause (0x826b552c).
- The pausing rows go through `UpdateOnEnter`: the unlock overlay (`ReceiveOpenUnlockOverlayMsg`
  0x826143f8, state 10), Skate 101 (`ReceiveSkate101HUDShowMsg` 0x826285e8, 0x4F), the sponsor
  overlay (`ReceiveOpenSponsorOverlayMsg` 0x82614450, 11), the end-of-challenge options (8) and
  change game (0x27). Press A (0x26, `OnEnter` 0x826423d0) and the sign-up poster (0x20,
  `OnEnter` 0x82647f68) are pushed states whose rows do not pause.

What a pause does, frame by frame (`GameModule::Update` 0x8273edc8):

- Simulation: `FrontEndManager::Update` runs first, so a push or pull made by the front end counts
  in that frame. `GameModule::DoGameUpdate` 0x82730418 generates the sync command only while
  `IsPaused(kPAUSE_FLAG_SIM)` is false, so no simulation tick runs.
- Input: `Sk8AppMan::TimerCallback` 0x82729770 caches SIM input on every timer tick, paused or not
  (`cPadCache::CacheInput` 0x8298cc40 has no pause test), so `GameModule::PreUpdate` 0x82729e54
  sets `mInputUpdate` and `DoGameUpdate` calls `PlayerUI::Update` 0x828af9f8 on paused frames
  too. Its focus needs no debug menu, `CheckForPauseState`, no popup, `mFEState ==
  eFE_STATE_IN_GAME`, no loading tag (`mEnableLoadingTagFlags`), no background fade
  (`mbFadeBackground`) and, in Party Play, the HUD not loading.
  `StateManagerFrontEnd::CheckForPauseState` 0x826477b0 wants the game in game, neither the top
  front-end state's nor the top HUD state's row pausing, no NIS (`NISManager::IsInNIS` 0x8289ecf0,
  set beside `PauseSim` in `RunBasicNIS`/`RunReplayNIS`, cleared in `EndNIS`) and PauseMan's top
  INIT or MANUALCAM. So every pause takes the focus: without simulating, `PlayerUI::Update` only
  clears `mHasFocus`; simulating without the focus (behind a popup), it sends an input without
  bytes, which the receiver decodes as nothing pressed but B (map ID 91, the brake). The first
  focused update sends `PlayerInputGainFocusData` (command 2), and `GameInputSK8::OnCommand`
  0x826df568 clears its four face-button gates: A, B, X and Y held through the pause count only
  after a release.
- Presentation: `GameModule::Render` 0x827320f0 skips `StepTime`/`RestoreFrame` only for
  `kPAUSE_FLAG_DATASYSRESTORE`, which the NIS state sets and FRONTEND (0xFFFFFFE5) and CHALLENGE
  (0xFFFFFFE1) do not; the retained world is drawn behind the screen. The audio is the first
  pass's: Pause 2 from the second paused update, Pause 0 under the pause menu.
- Resume: a front-end pull runs in the front end's update, so the same frame's `DoGameUpdate`
  regains the focus and simulates; `EndNIS` runs in `SimController::Update1`, after
  `DoGameUpdate`, so after a NIS the next frame does.

### Port before

- The several-challenges popup did not pause: the skater rolled on behind it.
- `PlayerUI`'s focus followed only the window: the skater kept the pad behind "New to skate?"
  (the A that answered it also pushed), and Start opened the pause menu over a popup.
- Paused frames returned before `PlayerUI::Update`, so no pause took the focus and none sent
  `PlayerInputGainFocusData`: an A still held after closing a screen or skipping a NIS went on into
  the simulation as a push.
- The first pass reported the career's popups as the Popup row (paused), which silenced the board
  behind "New to skate?"; the pause menu's popups were reported the same way.

### Changes

| File | Change |
| --- | --- |
| `crates/skate2-core/src/challenge/signup.rs` | `dpad_up` returns whether the several-challenges branch ran; `diagnostic_prompt`. |
| `crates/skate2-core/src/challenge/system.rs` | `sign_up_dpad_up`; `diagnostic_sign_up_prompt`. |
| `crates/skate2-core/src/session/selected_session_career.rs` | The D-pad up's push and the choice's or cancel's pull, applied to the pause stack (`try_update_sign_up_choice_pause`, `kPAUSE_STATE_FRONTEND`) right after the career's input, so that frame does not simulate; the popup's `core_popup`; `popup_open`; `frontend_audio_state` reports no popup; `career_sign_up_choice_paused`; `diagnostic_sign_up_prompt`. |
| `crates/skate2-core/src/session/selected_session_public_facade.rs` | The pause menu does not open over a popup; `PlayerUI` has the focus only with no popup open and PauseMan's top INIT or MANUALCAM; `try_tick` clears the focus on every front-end-only frame, so the first simulated frame regains it; the pause menu's popups report nothing to the audio system. |
| `crates/skate2-core/src/ui/mod.rs` | `FrontendAudioState::POPUP` removed. |
| `crates/skate2-core/src/session/selected_player_focus.rs` | `SKATE2_TRACE_CAREER` prints the focus changes. |
| `apps/skate2-pc/src/live_skating_client.rs` | The several-challenges popup's paused frames draw the retained world and the career HUD, as the challenge pause's do; `SKATE2_DIAGNOSTIC_SIGN_UP_PROMPT=<challenge>,<challenge>` at `SKATE2_DIAGNOSTIC_SIGN_UP_PROMPT_TICK` shows the sign-up prompt for those challenges. |

### Verification

- `cargo build --release -p skate2-pc` builds (the last build of these changes, 08:04), with no
  warnings in the changed files; the later runs used the shared 08:10 and 08:16 builds, which
  contain them.
- `parity.sh`: PARITY IDENTICAL (6000 lines) before the paused frames' focus (`nbs_parity2`),
  with it (`nbs_parity3`) and on the 08:04 build (`nbs_parity4`).
- Career demo (`career_run.sh`, scratch saves, `SKATE2_DIAGNOSTIC_MOVIE_SKIP=120
  SKATE2_DIAGNOSTIC_CAREER_YES=1 SKATE2_TRACE_AUDIO=1 SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES=2600`,
  `SKATE2_DIAGNOSTIC_SIGN_UP_PROMPT=m_1a_03_tutorial_fliptricks,m_1a_05_photo_rail` at tick 6450,
  and from `SKATE2_DIAGNOSTIC_CAREER_PAD` the D-pad up at 6480, A at 6620, A for the poster and
  Start, A and B from 8700; `nbs_pop5` ends at tick 7000):
  - "New to skate?" (`nbs_pop3`, `nbs_pop4`): `PlayerUI 0 focus false` as it opens and `true`
    after Yes, with no pause (HUD state 1).
  - The several-challenges popup (`nbs_pop5`): on the D-pad's frame `SignUpChoice`, `pushes
    kPAUSE_STATE_FRONTEND`, `PlayerUI 0 focus false (paused)` and `core_popup`, then Pause 2 (HUD
    state 1). The local board holds 1.654 m/s with `out1` 0 while it is open; on the A's frame
    come `pulls kPAUSE_STATE_FRONTEND` and `PlayerUI 0 focus true`, the board goes on from 1.657
    m/s, and the poster follows (HUD 0x20, Pause 1, not paused). The captures (every 0.3 s) show
    "Multiple Challenges / Sign up" before the press; then the world held under the game camera
    with the minimap, the sign-up HUD gone and the two titles listed; then the poster. In
    `nbs_pop4`, before the redraw, the last gameplay frame stayed up with the prompt and no list.
  - `nbs_pop4`: every NIS (the four go-to outros, `01_Welcome_Back`, the highlight replays,
    `02_Pros_Arrival`), the career start's CaC screen, the unlock overlay and the pause menu print
    `focus false (paused)` as the pause starts and `focus true` (a `PlayerInputGainFocusData`) on
    the first simulated frame. `m_1a_03`'s forced objectives complete on the same ticks as in
    `nbs_pop3` (6680 to 7880), and the run goes on to the unlock overlay (HUD 10, paused),
    `02_Pros_Arrival`, the autosave, the pause menu (HUD 7, Pause 0) and the map (HUD 0x15, music
    still PauseMenu).
- Film (`nbs_film` on the 07:14 build, without the paused frames' focus; `nbs_film2` on the 08:16
  build; `sponsor_me_02` with `SKATE2_DIAGNOSTIC_FILM_WIN=120:B`): the challenge pause (`focus
  false (paused)`), "Nailed It, Nailer!", B, "Are you sure you want to retry this event?" over the
  held world, Yes: `EnterRealTime`, the unpause, `KillNis`, `restarted`, and `focus true` at the
  countdown; the second win, "Money!", A, `complete`, `focus true` and the autosave.
- Photo (`nbs_photo` and `nbs_photo2`, the same builds; the photo recipe): the held shot, the
  photo selection (Continue, Y), the pick, `EnterRealTime`, the unpause, `KillNis`, `complete`,
  the rewards overlay counting to $350 (paused, HUD 10, `focus false (paused)` to `true`) and the
  autosave with cash 350.
- Party Play (`nbs_party`, 08:10 build, `main-menu-freeskate` with
  `SKATE2_DIAGNOSTIC_PARTY_PLAY=17000`, Spot Battle at the megapark and
  `SKATE2_DIAGNOSTIC_PARTY_PLAY_CHANGE=full`; trace only): the intro NIS (`focus false (paused)`,
  then `true`), nine turns whose press-A dialogs (HUD 0x26) neither pause nor take the focus, the
  results (FE state 8 on screen 41, paused, `PlayerUI 2 focus false (paused)`), Change Game (HUD
  0x27, paused), back to the results and in again, then A: `focus true` and the game restarts
  (`reset`, the countdown, the press-A dialog, `challenge_running`).
- Not run (the skate2-core test build does not fit in memory): `challenge/tests.rs`
  `sign_up_at_indicator` now asserts `sign_up_dpad_up`'s result (true only with several
  challenges; the data tests that use it are ignored without `SKATE2_GAME_ROOT`).

### Open

- The rest of `DoGameUpdate`'s focus: the debug menu (not ported), the loading tags
  (`EnableLoadingTag` 0x82614e48: the memcard icon while saving, `ShowMemcardActivityIcon`
  0x8256fc50; the challenge effects' tag after 90 frames of a fade outside a NIS,
  `UpdateChallengeEffectsLoadingTag` 0x82628c48; the NIS loading overlay,
  `NISManager::ReceiveNISLoadingOverlayMsg` 0x828a3550; the load screens), the background fade
  (`EnableBackground` 0x826066c8 sets `mbFadeBackground` whichever way it fades and `UpdateFade`
  0x82614f10 clears it once the fade is out; the port's `FrontendFade` holds the same state but
  does not feed the focus) and Party Play's HUD loading.
  Without them the port's skater keeps the pad during an autosave, a long challenge fade and the
  fade-out after map travel.
- `CheckForPauseState`'s row and NIS tests are PauseMan's top in the port: every pausing row the
  port shows pushes FRONTEND and every NIS pushes `kPAUSE_STATE_NIS`.
- On paused frames the port skips `SessionMarker::Update` and the network's input, which
  `DoGameUpdate` runs before `PlayerUI::Update`.
- Resume frames: the port's pause menu returns one more front-end-only frame after it closes, and
  the port ends a NIS (`EndNIS`) before that frame's `PlayerUI::Update`; so the first simulated
  frame comes one frame later after the pause menu and one frame earlier after a NIS than
  natively.
- The several-challenges popup is drawn as text (`marker_ui.rs`), without the popup screen's frame,
  its A Select legend and its navigation sounds.
- Not ported: the sponsor overlay (11) and the coverage screen (61); the session marker's
  security popup (`PlayerUI::UpdateSessionMarker` 0x828af3f0 opens it and pushes FRONTEND,
  `PlayerUI::DismissAndTeleportCallback` 0x828af178 pulls it) has no security service to raise it
  in the port.

The sponsor overlay, the coverage screens and the several-challenges popup's frame continue in
[sponsor-coverage-screens-2026-10-10.md](sponsor-coverage-screens-2026-10-10.md).
