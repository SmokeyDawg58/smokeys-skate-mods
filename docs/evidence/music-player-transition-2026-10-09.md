# Music player transition (2026-10-09)

Opening the music player from the in-game Settings showed the world behind it brighten: the
Options screen's dimmer faded out while the music player's own dimmer faded in, and the dimmer was
cut off half way when the Options outro ended. The music player also appeared on the very frame
of the press, over Options rows that stay up for their whole outro, and the letterbox bars were
cut off mid-slide. Closing played the same in reverse. The native keeps one dimmer on through
both transitions and builds the next screen through the screen manager's load path first.

A follow-up the same day did the same for the rest of that code: the Options exit to the StartHub
(which cut the dimmer and the bars when the Options rows went), Options' own open from the
StartHub and the main menu, the StartHub's close and reopen around the states pushed over it, the
dimmer of the challenge map, Party Play's change game and the jacked overlay, the music player's
`OnLoaded`, and `RequestExit`'s Party Play save. It also found that the seven frames of the first
pass are only the screen manager's best case (`mBlockScreenOpen`, below).

## Where the native opens the music player

- `FrontEndState_Options::OnMenuSelect` (`0x826871a0`) is the only `PushState(30)`: the
  `eSk8FEGameOption_MusicPlayer` row plays `core_a_button` and pushes FE state 30 with
  `sFEStateTransition { mSubState 35, mbKeepPrevious 0, mbKeepPreviousVisible 0 }`
  (`0x82687228..0x82687248`: `li r4,0x1E`, the struct's word `0x23`). The row is in the in-game
  list only (`gGameOptionsScreens` `0x82fdfa70`); the StartHub reaches Options through
  `IStartHubState::OnMenuSelect` (`0x82674d08`, `eSk8FEStartHubOption_GameSettings` ->
  `PushState(73)`). The front-end main menu's Options has no Music Player row.
- B on the music player (`FrontEndState_MusicPlayer::ReceiveInput` `0x8266aa00`, keycode 3, not
  editing) sets `mbAllowInput = 0` and calls `PopState()` with no sound.

## Native sequence

`gFEStateInfo` (`0x82fded80`, 20 bytes a state; no `mbHUDStack` among these):

| State | mbPause | mbEnableChyron | mbTransition | mbDimmer | mShowHUD | mFEMusicState |
| --- | --- | --- | --- | --- | --- | --- |
| 7 StartHub | 1 | 0 | 0 | 0 | HideExceptOnline | PauseMenu |
| 8 EndChallengeOptions | 1 | 0 | 0 | 0 | Hide | 4 |
| 21 ChallengeMap | 1 | 1 | 0 | 1 | Hide | 9 |
| 22 MainMenu | 0 | 1 | 0 | 0 | Hide | 0 |
| 30 MusicPlayer | 1 | 1 | 0 | 1 | Hide | MusicPlayer |
| 39 PartyPlayChangeGame | 1 | 0 | 0 | 1 | Hide | 9 |
| 42 JackedOverlay | 1 | 0 | 0 | 1 | Hide | 8 |
| 73 Options | 1 | 1 | 0 | 1 | Hide | Options |

Push (`StateManagerFrontEndBase::PushState(int, sFEStateTransition)` `0x82d6a250` queues it;
`ExecutePushState` `0x82d694e8` runs it in `ExecuteActions`):

1. `UpdateOnLeave(73, 30)` (`0x82d66300`): `FrontEndState_Options::OnLeave` (`0x8266af70`) sends
   `SetScreenState(id, 2)` outside the front end (options.apt `SetState(2)`:
   `LetterBox.EnableLetterBoxInfo(false)`, `LetterBox.EnableLetterBox(false)`) and
   `CloseScreen(false)`; with `mbTransition` 0 the state then calls `CloseFEScreen`
   (`0x826572c0`) -> `ScreenManagerBase::CloseScreen` (`0x82d65c20`) -> screenmanager.apt
   `CloseScreen`: input off, `PlayClip(id, "outro")`. `UnpauseGame`. A state pushed with
   `mbKeepPrevious` gets `Show(mbKeepPreviousVisible)` (`0x82d661e8`) instead.
2. `UpdateFEAudioState(30)`, push, `SetInfo`, `UpdateOnEnter(30)` (`0x82d663f0`: HUD hidden,
   `PauseGame`, `UpdateChyron(1)`, then `FrontEndState_MusicPlayer::OnEnter` `0x826438c8`:
   `mbIsDirty = 0`, `OpenScreen(35)` = `mScreenState = RequestLoad` (`0x82d67000`)).
3. `SetState(35)` (the base's empty slot), then `EnableDimmer(gFEStateInfo[30].mbDimmer)`.

Pop (`PopState()` `0x82d69e50`; `ExecutePopState` `0x82d672f8`): `UpdateOnLeave(30, 73)` (the
music player's `OnLeave` is `CloseScreen(false)`, folded at `0x82641910`; `CloseFEScreen` plays
its outro), `EnableDimmer(gFEStateInfo[73].mbDimmer)`, `UpdateFEAudioState(73)`, and, Options
having been left (`mbKeepPrevious` 0), `UpdateOnEnter(73)`: `FrontEndState_Options::OnEnter`
(`0x8265c468`) clears `mIsDirty` and `OpenScreen(120)` again; a kept state gets `Show(true)`. The
last state's pop sends `EnableDimmer(0)`.

### The dimmer

`StateManagerFrontEnd::EnableDimmer` (`0x82656a30`) -> `FrontEndManager::EnableDimmer`
(`0x82614db8`) -> `eASCall_EnableDimmer`, the AS function `_global.ScreenManager_ActivateDimmer`
(call table `0x82fdb194`). screenmanager.apt `ActivateDimmer(enable)` returns when
`mbDimmerEnabled == enable`, else plays `gDimmer`'s `intro` or `outro` (`gDimmer` is
mainmisc.apt's `createEmptyMovieClip("gDimmer", 1)` loading `main/dimmer.swf`). States 73 and 30
both have `mbDimmer` 1, so the push and the pop leave the dimmer stopped at its intro's last frame
(29, alpha `0xe5`): no fade, no `FE_SOUND_CORE_FADE`.

The screens' own `_global.ScreenManager.EnableDimmer(...)` calls (options.apt frame 1,
musicplayertest frames 1 and 15, challengemap frames 1 and 35, partyplaychangegame frames 1 and
78, jacked_overlay frames 1 and 40) call a method nothing defines: screenmanager.apt's prototype
has `ActivateDimmer` only, no movie in `screens.big` assigns `EnableDimmer` (22 movies call it),
and the string `EnableDimmer` is not in the xex, so no native can be registered under that name.
The calls do nothing; the state on top drives the one dimmer.

`AptRenderingIntegration::Render2D` (`0x82631a98`) draws `_level8` first (mainmisc.swf, the
dimmer), then `_level7` to `_level2` with the 3D camera (the HUD levels, and `_level2`,
main3dfocal.swf's `g3dFocalContainer`, which holds `3D_Focus_Left` screens such as the StartHub),
then `_level0` (main.swf: `gScreenManager`'s 2D screens at depth 12, `gLetterBox` 14, `gChyron`
15), then `_level9`. The dimmer is under every APT layer: the StartHub fades over it, and Options,
the music player, the map and the change game are over the StartHub.

### The screen manager

`FrontEndManager::Update` (`0x82640130`) runs `AptIntegration::Update` (`0x826014c0`: `AptUpdate`,
then `AptLoadingIntegration::CheckAndCompleteAsyncLoad` `0x82a728b0`) before the controllers'
input, `StateManagerFrontEndBase::Update` (`0x82d68370`) and `ExecuteActions`. A state's
`OpenScreen` only sets `RequestLoad`; the next frame's `StateManagerFrontEndBase::Update` calls
`UpdateFEScreen` (`0x82657248`) -> `ScreenManagerBase::OpenScreen` (`0x82d6a840`) -> AS
`OpenScreen` -> `ReadyScreen` (`0x000b77`), which queues the screen and sets
`mScreen.onEnterFrame = Update`. (`mbBlockLoad`, which the StartHub's B sets until its
`OnOutroDone` 0x8265e628, holds `RequestLoad` there.)

screenmanager.apt `Update` (`0x000f64`) runs `ProcessDestroyedScreens` (`0x000fa0`: one
`RemoveScreen`, and true), and `ProcessOpenScreens` (`0x001029`) only when that removed nothing:

1. `mBlockScreenOpen > 0`: count it down and return.
2. A screen queued and none waiting removal: `mBlockScreenOpen = 8`, `CreateScreen` the first.
3. Nothing queued and nothing to remove: `mScreen.onEnterFrame = undefined`.

Step 1 returns before step 3, and only `ReadyScreen` and `DestroyScreen` (`0x00110d`, from every
`OutroComplete`) set `onEnterFrame` again. So the 8 a `CreateScreen` leaves stays until the next
`OpenScreen` or `DestroyScreen`: a screen opened after another was created, with none destroyed
since, waits 8 ticks more, and each destroy costs the `Update` that removes it. The APT tick
(`tick` `0x82a21c10`: display lists, `RunActions`, inputs, `AptLinker::Update`) queues the screen
manager's `onEnterFrame` before its screens' frame actions (`AptCIH::tick` `0x82a1b580` queues a
clip's events before its children's; the levels tick in depth order), so an `OutroComplete` is
removed by the next tick's `Update`, and the screen shows its last outro frame until then.

`CreateScreen` `loadMovie`s the screen. Only the controls, the StartHub and the online hub are
preloaded (`FrontEndManager::PreloadTextures` `0x82610730`): they link in the tick that creates
them. Options and the music player are read from `screens.big`: `CheckAndCompleteAsyncLoad` moves
the load one step a tick (`CheckLoadingOperation` `0x82a720d8`: `.apt`, `.const`, `.geo`, the
`.rx2` arena of `LoadAsyncArena` `0x82a70800`, then `CompleteLoad` `0x82a72030`), and the fifth
tick after `CreateScreen`'s `AptLinker::Update` (`0x82a21228`) links the movie and runs its
actions: the constructor's `RegisterScreen` -> `ScreenManager.OnLoaded` ->
`InitialStandardScreenReady` -> `PlayClip("intro")`, then `Game.OnScreenLoaded` -> the state's
`OnLoaded`. `ReadyScreen` gives each screen `mScreenArrayDepth` and increments it, and
`CreateScreen` makes the screen's clip at that depth in `gScreenManager`, so a screen opened later
draws over one opened before. With every read done within its tick, a screen comes 7 ticks after
its state's push when the screen manager was idle, and 8 more when a creation's block is still
standing; disc reads only add to it.

`StateManagerFrontEnd::ReceiveInput` (`0x82656af8`) hands a state its input only while
`IsScreenReady` (`0x82d661a8`, `mScreenState == Ready`) and `mbAllowInput`: the screen's
`IntroComplete` -> `OnScreenIntroCallBack` (`0x82d66fe0`) -> `OnScreenReady` (`0x82d65f98`) sets
both.

### The StartHub

`FrontEndState_StartHub` (state 7) opens screen 11, starthub/starthubmenu.swf, a
`3D_Focus_Left` screen (`gScreenTemplate` `0x82fdb358`). Game Settings is `PushState(73)` and
Challenge Map `PushState(21)` (`IStartHubState::OnMenuSelect` `0x82674d08`), both with
`mbKeepPrevious` 0: the push's `UpdateOnLeave(7)` runs `OnLeave` (`0x8264c4d8`,
`CloseScreen(false)`) and `CloseFEScreen`, the hub's outro. The pop's `UpdateOnEnter(7)` runs
`OnEnter` (`0x8265e4d0`, `OpenScreen(11)`) again, a new screen: its `OnIntroDone` (`0x826536e0`)
picks the hub mode and `SetScreenState(0 or 1)`, so its picker starts on the first enabled row,
and it takes no input before that `IntroComplete`. Start or B in the hub
(`ReceiveInput` `0x8265e778`) plays `pause_menu_press`, sets `mbBlockLoad` and pops.

### The main menu

`FrontEndState_MainMenu::OnMenuSelectGame` (`0x82681338`, case 5 at `0x826815e8`) pushes Options
with `sFEStateTransition { mSubState -1, mbKeepPrevious 1, mbKeepPreviousVisible 0 }`: the
push's `Show(false)` (`ShowFEScreen` -> screenmanager.apt `ShowScreen`, `_visible = false`) hides
the main menu at once, with no outro; the pop's `Show(true)` shows it at once (`OnScreenReady`,
`OnGainFocus`), under Options' outro, taking input. Options' `OnLeave` sends `SetScreenState(2)`
only outside the front end, so the main menu's bars stay up through the push and the pop.

### The movies

| Movie | Timeline (0-based frames) |
| --- | --- |
| options.apt root | 1 `intro` (undefined `EnableDimmer(true)`); 4 stop, `EnableLetterBoxInfo(true)`, `IntroComplete`; 5 `outro`; 20 stop, `OutroComplete`. Frames 1-20 place nothing: the rows (`attachMovie`d by `CreateOptions`) stay as they are through the outro. |
| musicplayertest root | 0 `mMusicPlayer` at alpha 0; 1 `intro`: undefined `EnableDimmer(true)`, `LetterBox.EnableLetterBox(false)`; alpha `26 4c 3b 2a 19 3e 60 7f 9c b5 cc e0 f1` (1-13); 14 `ff`, stop, `IntroComplete`; 15 `outro` (undefined `EnableDimmer(false)`); `a6 4c 66 7f 99 88 77 66 55 43 32 21 10` (16-28); 29 `00`, stop, `OutroComplete`. |
| starthubmenu root | 1 `intro`: alpha `26 4c 33 19 59 91 bf e3` (1-8); 9 `ff`, stop, `IntroComplete`; 10 `outro`, `FE_SOUND_PAUSE_MENU`; `a6 4c 66 7f 99 66 32` (11-17); 18 `00`, stop, `OutroComplete`. |
| main/dimmer.apt | 1 `intro`: visible, `FE_SOUND_CORE_FADE`, alpha `07`..`e5` to 29 (stop); 30 `outro`: `FE_SOUND_CORE_FADE`, `dd`..`07` to 58; 59 stop, hidden. |
| main/letterbox.apt `mcLetterbox` | `niceintro` 1-58 (1 `FE_SOUND_LETTERBOX_IN`, 58 `EnableLetterBoxInfo(true)`); `niceoutro` 59-86 (59 `FE_SOUND_LETTERBOX_OUT`, bars slide 92 px off, 86 stop, `OutroDone` hides the letterbox). `mInfo`: `infointro` 1 (`FE_SOUND_PAGETITLE_FLICKER`), `infooutro` 25-39. |

letterbox.apt: `EnableLetterBox(true)` -> `PlayIntro` plays `niceintro` only while hidden;
`EnableLetterBox(false)` -> `PlayOutro` -> `EnableLetterBoxInfo(false)` and `gotoAndPlay
("niceoutro")` every time, so the music player's frame 1 replays the bars' slide from frame 59
(bars back in, a second `FE_SOUND_LETTERBOX_OUT`) whenever the screen links before the first slide
ended (27 frames). main.apt makes the layers in depth order `gHUD` 11, `gScreenManager` 12 (the 2D
screens), `gScreenTransitionManager` 13, `gLetterBox` 14, `gChyron` 15, so the letterbox draws
over both screens.

### Frame by frame

Frame 0 is the press, reads finish within their tick, and the screen manager is idle before the
pause.

| Frame | Native |
| --- | --- |
| Start (P) | `PushState(7)`: the hub's `RequestLoad`; `EnableDimmer(0)` changes nothing |
| P+2 | the hub created and linked (preloaded): intro frame 1; `mBlockScreenOpen` 8 |
| P+10 | the hub's `IntroComplete`: input |
| Game Settings (T) | `core_a_button`; the hub's outro (`PAUSE_MENU`); the dimmer's intro (`FADE`); Options' `RequestLoad` |
| T+2..T+8 | the block counts 8 down to 1 |
| T+8 | the hub at frame 18 (alpha 0): `OutroComplete` |
| T+9 | the hub removed (that `Update` opens nothing) |
| T+11 | Options created (`mBlockScreenOpen` 8) |
| T+16 | Options linked: intro, bars in (`LETTERBOX_IN`) |
| T+19 | Options' `IntroComplete`: title (`PAGETITLE_FLICKER`), input |
| Music Player (M) | `core_a_button`; title out, bars out (`LETTERBOX_OUT`), Options' outro; the dimmer unchanged; the music player's `RequestLoad` |
| M+2..M+9 | Options' creation left the block: 8 down to 0 |
| M+10 | the music player created |
| M+15 | linked: intro frame 1 (alpha `26`), `OnLoaded`; frame 1 replays the bars' outro (`LETTERBOX_OUT`); Options at frame 20: `OutroComplete` |
| M+16 | the Options rows removed |
| M+28 | the music player's `IntroComplete`: input |
| M+42 | bars gone |
| B (N) | the music player's outro (frame 15, still `ff`); Options' `RequestLoad` |
| N+2 | Options created (its destroy at M+15 let the block run out) |
| N+7 | Options linked: intro, bars and title in |
| N+10 | Options' `IntroComplete`: input |
| N+14 | the music player at frame 29 (alpha 0): `OutroComplete`, removed at N+15 |
| B (X) | title out, bars out (`LETTERBOX_OUT`), Options' outro; the dimmer's outro (`FADE`); the hub's `RequestLoad` |
| X+2 | a new hub linked (the music player's destroy let the block run out): intro, first row |
| X+10 | the hub's `IntroComplete`: input |
| X+15 | Options at frame 20: `OutroComplete`; the rows removed at X+16 |
| X+27 | bars hidden |
| X+29 | dimmer hidden |

Without the music player in between, the exit finds the block Options' creation left: the hub
links at X+10 and takes input at X+18. A second Music Player from the same Options links at M+7,
as the first pass had it. From the main menu (`T`): the main menu hides at once, the dimmer fades
in, Options links at T+7 when the screen manager is idle (8 later if the main menu's own screen
was the last created with nothing destroyed since), the bars only retitle; its B shows the main
menu at once under Options' outro and the dimmer's.

## RequestExit and OnLoaded

`FrontEndState_Options::RequestExit` (`0x82681700`) folds the music player's `mbIsDirty` into
`mIsDirty` and saves (`RequestSave(eBeMemcardSave_Profile)`) when `mIsDirty && (!GetMainChallenge()
|| sGameInfo.gameMode == GM_PartyPlay)`: Party Play, whose games run as the main challenge, saves
too. It is the profile save the port already makes on the exit
(`EditSkaterProfileStore::save_audio_profile`).

`FrontEndState_MusicPlayer::OnEnter` (`0x826438c8`) only clears `mbIsDirty` and opens the screen.
`OnLoaded` (`0x8266a968`), from `Game.OnScreenLoaded` as the screen links, reads the profile's
music mode: Music Player sets the player screen, World the ambient one, and Off runs
`UpdateMusicMode(1)` (the world's music, `mbIsDirty`) before the ambient one.

## Port

- `marker_ui/screen_manager.rs` (new): `ScreenManager`, screenmanager.apt's queues for the
  StartHub's, Options' and the music player's screens: `request` (`OnEnter`'s `RequestLoad`),
  `update_states` (the state update's `ReadyScreen`), `tick` (`Update`'s
  `ProcessDestroyedScreens`/`ProcessOpenScreens` with `mBlockScreenOpen`, the 5-tick load, the
  preloaded StartHub), `destroy` (`OutroComplete`), `cancel` and `track`. `Dimmer`: the one
  `gDimmer` with `ActivateDimmer`'s `mbDimmerEnabled`. `ScreenEvents`: the StartHub's, Options'
  and the music player's `IntroComplete`, and the music player's link.
- `marker_ui.rs`: `MarkerUi` owns the screen manager and the dimmer. Each APT step runs the screen
  manager's tick before the movies, reports their `OutroComplete`s, links the screens the tick
  linked after the movies' actions, then the state update. `sync_dimmer` is
  `StateManagerFrontEnd::EnableDimmer` for the state on top (Options or the music player, the
  map, the change game, the jacked overlay), after every state change. Options and the music
  player draw in their `ReadyScreen` depth order (`ScreenManager::over`), so the screen opened
  later is on top: the music player over Options' outro on the push, the new Options over the
  music player's outro on the pop. The StartHub is open while the pause is up as the hub with
  neither Options nor the map over it, so their push closes it and their pop opens a new one.
  `update_pause` applies the pause and the map before the step, as `ExecuteActions` precedes the
  next tick. Draw order: the dimmer first, then the HUD, the StartHub, the change game, the map,
  Options and the music player, the letterboxes and the chyron, whatever screen is up; over the
  main menu the menu is hidden while Options is on the stack and the overlay is the dimmer,
  Options, the main menu's letterbox and the chyron.
- `marker_ui/settings_movie.rs`, `music_player_movie.rs`, `pause_movie.rs`: each screen waits for
  `link` after its state's `OnEnter` (`pending`), plays its outro on `OnLeave` (or drops a screen
  not yet linked), and reports `OutroComplete`. Options no longer owns a dimmer, nor resets the
  in-game letterbox (one `gLetterBox`); the StartHub's link builds the hub on the core's row.
  `SCREEN_LOAD_FRAMES` is gone.
- `marker_ui/map_movie.rs`, `party_play_change_movie.rs`, `jacked_movie.rs`: their screens'
  `EnableDimmer` calls do nothing; the map's dimmer clip is gone.
- `ui/pause_menu.rs`: `ready` (`IsScreenReady`), `acknowledge_intro`, `awaiting_intro`, `reenter`
  (the pop's `OnEnter`: no input before the intro, the first enabled row). The session hands the
  hub no input while it awaits its intro, and re-enters it when Options or the map pops.
- `ui/music_player.rs`: `open` is `OnEnter` with the screen `OnLoaded` will show; `loaded` runs
  `OnLoaded`'s `UpdateMusicMode(1)` for an Off profile when the screen links.
- `session/selected_session_music_player.rs`: `acknowledge_screens` (the StartHub's, Options' and
  the music player's intros, the music player's link); `party_play_mode`
  (`sGameInfo.gameMode == GM_PartyPlay`); `request_options_exit` saves when dirty and no main
  challenge runs or the game is Party Play. `selected_session_public_facade.rs` passes the mode,
  the hub's input gate and the re-entries.
- `live_skating_view.rs`, `live_skating_client.rs`: `take_screen_events` to `acknowledge_screens`.
  Diagnostics: `SKATE2_DIAGNOSTIC_MUSIC_PLAYER=exit` (Options, B to the StartHub every 240 ticks,
  Game Settings again) and `=map` (the same with the map);
  `SKATE2_DIAGNOSTIC_PARTY_PLAY_SETTINGS=<tick>` (in Party Play: Start, Game Settings, Audio
  Settings, SFX to 80%, B, B; logs the scratch profile's SFX).

## Verification

- `cargo build --release -p skate2-pc`: clean; no warnings in the files changed.
- Core: `pause_menu.rs`, `options.rs` and `music_player.rs` tests through a scratch crate that
  pulls the files in with `#[path]` (the core lib test target still crashes rustc): 9 pass, among
  them the hub's input gate (`quit_confirmation_defaults_to_no_and_requires_yes`) and `OnLoaded`
  waiting for the screen (`opens_on_the_saved_mode_and_switches_tabs`, an Off profile).
- `cargo test --release -p skate2-pc marker_ui:: -- --include-ignored` with `SKATE2_GAME_ROOT`:
  41 pass. New: `screen_manager::tests::a_destroyed_screen_unblocks_the_next_open` and
  `start_hub_options_and_music_player_follow_the_screen_manager`, which steps the real movies
  through the table above: the hub links on tick 2 and completes its intro on tick 10; Game
  Settings' hub outro ends on tick 8 (`FE_SOUND_PAUSE_MENU`, the dimmer's `FADE` at the push) and
  Options links on tick 16; the music player links on tick 15 with its loaded event, over Options'
  last frame, the rows go on tick 16, the bars replay their outro, the dimmer never moves; B links
  Options on tick 7, drawn over the music player's outro; B again links a new hub on tick 2 while
  the rows stay to tick 15, the bars to tick 26 and the dimmer to tick 28.
  `display_preferences_original_options_layout_and_lifecycle` and
  `original_party_play_end_screens_build` (no dimmer calls any more) pass.
- Live (`SKATE2_DIAGNOSTIC_MUSIC_PLAYER`, the main-menu-freeskate demo, 960x540 captures every
  0.05 s, a region's mean brightness): `exit` (Game Settings, B, three times): on A the hub fades
  out over the darkening world and Options comes up with its bars; on B the dimmed world
  brightens over about 0.45 s (14.3 to 42.1) and the bars slide off over 0.35 s while the new hub
  fades in under the Options rows, which go a few frames later; the hub comes back on its first
  enabled row. `transition`: the world stays at 14.3 through every push and pop; the music player
  flickers in over the last of the Options rows with the bars back in and sliding out again, and
  on B the new Options comes up over its outro. `map`: the dimmer fades out and in again with the
  map's pop and push, the hub reappearing under the map's outro. The front-end
  `display-settings` demo: A on Options hides the main menu at once, the dimmer fades in over the
  wall and Options comes up a few frames later under the main menu's retitled bars; B shows the
  main menu at once on New Career under the Options rows and the fading dimmer; its main-menu
  save check (tick 900) passes. (Its in-game half fails at tick 1500 with paused false, apart
  from this change: its Start at 1170 lands in the Free Skate option screen the main menu's
  Freeskate now opens, and its hub script expects a Debug row.)
- Live, Party Play (`SKATE2_DIAGNOSTIC_PARTY_PLAY=2760`,
  `SKATE2_DIAGNOSTIC_PARTY_PLAY_SETTINGS=1950`: three players, S.K.A.T.E. at the snakes spot
  running as the main challenge; Start, Game Settings, Audio Settings, SFX to 80%, B, B): the
  scratch profile holds SFX 0.8 after the exit (`Party Play settings 2700: ... saved sfx
  0.79999995`), which the old `!main_challenge` condition would not have saved.
- Live, Party Play's change game (`SKATE2_DIAGNOSTIC_PARTY_PLAY_GAME=spot`,
  `SKATE2_DIAGNOSTIC_PARTY_PLAY_CHANGE=full`, captures every 0.25 s): the results (state 8) over
  the undimmed world (mean 33.9); X opens Change Event and the world dims (13.9); B back to the
  results brightens it again and the second X dims it again, with partway values on the fades;
  A restarts the game.
- `parity.sh` with the copied build: PARITY IDENTICAL (6000 lines).

## Open

- The challenge map's, the change game's and the jacked overlay's own screens (14, 65, 68) still
  appear on their push: their load path is not modelled, so their creation does not count toward
  `mBlockScreenOpen` either (the hub's reopening after the map finds the block run out, where the
  native's map creation would leave it standing: X+2 instead of X+10). Nor do the main menu's
  screen, the HUD and popups: the model starts idle, so the first pause of a game and the first
  Options from the main menu assume no creation is still blocking.
- The ±1 tick of the hub's removal rests on the levels ticking in depth order (`_level0`, with the
  screen manager, before `_level2`).
- The port's saves are synchronous: `RequestExit`'s `RequestSave` is asynchronous natively
  (`OnComplete` 0x82681930, then `EndScreen`'s `PopState(73)`), so the native's exit waits for the
  write.
- The native has one `gLetterBox`; the port keeps separate letterboxes for Options in game, the
  map, the main menu and the editor.
- The music mode in the live runs was World, so the Off -> World `OnLoaded` switch is covered by
  the core test only.
- `SCREEN_LOAD_FRAMES` (the first pass) is gone; reads slower than a tick, which the model does not
  have, only lengthen the native's path.
