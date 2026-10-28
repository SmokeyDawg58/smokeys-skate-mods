# Replay cloth, the photo selection's "Y Continue", the first mission's replays (October 10)

Three replay issues from playtesting: the T-shirt (ROPA) cloth in a photo mission's replay
does not behave as in play; the "Y Continue" prompt stays on screen after a photo mission
completes; whether the first mission shows a replay of the ollie over the curb. Native
reference: `sk82_na_m.xex` (debug, Jan 17 2009) in IDA.

## 1. Cloth during replay playback

### Native

- Not recorded. `SkaterPresentation::Data` (0xd0 bytes) holds `localHipsT`,
  `localSkateboardT`, `root`, `peerId`, the character damage, `boardSpeed`, the player id,
  opacity, the pose buffer offset, the teleport/opacity/collide/local flags and `wipingOut`.
  No cloth state is submitted; the replay restores the pose only.
- Re-simulated from the restored skeleton. A replay-controlled scene is no character NIS
  (`SceneBinDirectory::IsSceneWithCharacters` 0x8286ece0 excludes scenes that control the
  replay), so `SkaterPresentation::DoSimRestore` 0x827e6438 runs (0x827e6488): `UpdateBodies`,
  then for each entity `mWasRestoredThisFrame` = 1 and `StartJobs` 0x827dbb90 (0x827e7098) on
  the restored pose: `BeginSkinJob` 0x827d7c48, `ClothUpdate` 0x827d24f8.
- `ClothUpdate`'s clock (0x827d264c..0x827d2684): outside a character NIS, the presentation's
  time controller. During a replay that is `SceneReplayTimeController::GetTickCount`
  0x82872720: `_frame` = `mCurrentReplayFrame` (+0x14), `_delta` = `mFrameUnaccounted` (+0x20).
  The state is `LAST_CLOTH_STATE_GAME`.
- dt = |((fraction − `mPrevDelta`) + (frame − `mPrevFrame`)) / 60| (`fsel` at 0x827d2720),
  each restore. A restart (0x827d26a4..0x827d26dc) comes with a change of state or a jump of
  15 frames or more: the replay's first frame, which is seconds before the last one drawn.
  It puts the particles on the drivers the last skin job left (0x827d27bc..0x827d2810), and that
  frame draws the skin job's vertices.
- `ROPA::Effect::Update` 0x8285ff38 returns no job under 1e-5 s (0x8285ff8c). While the
  scene's time stands (a fixed-frame photo's freeze, the hold under the photo selection), the
  replay's frame and fraction do not move, dt is 0, and the shirt is the skin job's vertices.
- `mFramesInWipeOut` (+0x2c4) is written at 0x827e7294, after `StartJobs`: `PreClothUpdate`
  0x827d2318 reads the previous restore's frames.
- The NIS draws at 30 Hz (`PostFX::Apply` sets `m_b30HzMode` while the ICE post-fx is active),
  so the native replay cloth steps 30 times a second by about 1/30 s.
- Frozen copies. `ICEUserChannelFreeze::UpdateFrameEnd` 0x826cf1a8 calls
  `SceneCharacterManager::InstanceFrozenSkater` 0x82874f90, which builds a new renderable of
  the player's recipe (`GetPlayerRecipeID`): `CreateCharacterRenderable` 0x82871150 with
  `clothDisabled` = 1, a `SkaterPresEntity` (constructor 0x827e5740) with `isClothDisabled` set
  (r9 at 0x82871248) and `IsLocalPlayer` 0 (0x82871244). It has no cloth; its ROPA meshes are
  GPU-skinned on the copy's pose (`i_vtx_params.x` 1).

### Port before

- The replay restore (`PresentationRestorer::restore_recorded`) used the real-time restore's
  cloth: a step only on a new whole frame, at that frame's own pose with fraction 0, published
  blended by the fraction. Through a freeze it kept the simulated shape.
- Frozen copies were drawn with the local skater's materials, so with its CPU cloth: every
  copy's shirt was the live shirt's vertices at the copy's creation, placed by the live
  skater's current world rows. The copies had no shirt where they stood, and the live skater
  carried a stack of them.

### Port now

- `SkaterRestore::restore(.., replayed)`: a replay's restore steps the cloth per restore on the
  restored (blended) pose by the replay's frame and fraction
  (`CharacterMaterialAdmission::step_restored_cloth` →
  `SkaterRopaRuntime::step_restored`: the native `ClothUpdate` of the borrowed path on the game
  clock, the last restore's wipe-out frames), and publishes the step's own positions. A
  restart and a step under 1e-5 s draw the skinned vertices, as natively.
- `PresentationRestorer` gives each restored skater's materials (`RestoredSkater::materials`).
  `update_replay_view` builds each frozen copy's own materials,
  `CharacterMaterialAdmission::try_frozen_copy` (the same recipe and materials, constructed
  non-local with the cloth disabled: `SkaterRopaRuntime::without_effects`). The copies draw as
  scene characters with their own materials (`SelectedSceneCharacterVisual::frozen` and the PC's
  local-materials lock are gone).
- The real-time (gameplay) restore is unchanged.

Deviation kept: the port's paused frames run at 60 Hz, so the replay cloth steps 60 times a
second by 1/60 s (as the cutscene cloth).

### Replay opacity

`DoSimRestore` 0x827e72a4..0x827e72bc picks the opacity target with `Presentation::IsInReplay`
0x827eed90, the time controller's `IsInReplay` (vtable slot 3). For the
`SceneReplayTimeController` that is 0x826c6d80 (`li r3, 1`); for the `RealTimeController` it is
0x82784530 (`li r3, 0`). In a replay, a collidable main-camera-only skater and a non-collidable
skater show opaque (1.0) instead of the header byte or the half. The port had
`IN_REPLAY = false` ("the port has no replay presentation"); `RenderOpacityInputs::in_replay` is
now the restore's `replayed` (false on the completed-tick path).

## 2. "Y Continue" after a photo mission

### Native

- The prompt is `FrontEndState_PhotoChallenge`'s (FE state 5) screen 116,
  `challenge/photoselect`. Its `OnLoaded` reads `PhotoSelect_IsSelectionAvail` 0x826bb310.
  Without a choice (m_1a_05's outro has only fixed-frame shots, count 0) the step buttons hide
  and the title reads `ID_COMMON_CONTINUE` over the Y button (`takePhoto_mc`).
- `OnEnter` 0x8265ff60: `OpenScreen(116)`, `HudManagerShow(0)`, `SetScreenState(116, 0)`.
- `ReceiveInput` 0x82660048, keycode 4 (Y): `SetScreenState` −3 (`takePhoto_mc` `glow`) and −10
  (`fakeoutro`: frames 11–20 fade every layer to alpha 0, then `stop`), the `photo_pick` sound,
  then the career-photo save, or without a unique photo challenge ID (`GetUniquePhotoChallengeID`
  0x825d0328) its callback at once. `cSaveChallengePhotoCallback::OnComplete` 0x8266c180 calls
  `PhotoDone` 0x82660320.
- `PhotoDone`: `SetState(8, -1)`. `ExecuteSetState` 0x82d69310 calls `UpdateOnLeave(5, 8)`
  0x82d66300: `OnLeave` 0x82807098 (only `mEnablePhotoPicking` = 1), then `CloseFEScreen`
  (`DestroyFEScreen` with `mbScreenDestroy`). The screen plays `outro` (frame 21) and
  `OutroComplete` (frame 22) unloads it.
- Each `SetScreenState` reaches the screen's `SetState` once. The prompt goes at the pick; the
  end options, the completion and the teardown never see it.

### Port before

`CareerHud::photo_select_screen` was the one screen-state list `take_career_hud` did not take.
Every HUD hand-off sent the screen all its calls again (0, −3, −10 after the pick). After the
close's `outro`, each update's −10 restarted `fakeoutro` at frame 11 (full alpha), which plays
one frame before the next restart and never reaches `OutroComplete`. The screen stayed at about
full alpha for the rest of the session. Live (`rp_base`): the final capture, after the
mission's autosave, shows "Continue" with the Y button.

### Port now

`take_career_hud` takes `photo_select_screen` like the other screens' lists
(`selected_session_career.rs`).

## 3. The first mission's ollie over the curb

### Native: a replay plays

- `m_1a_01_goto_slappys`'s state graph calls no replay native; the replays are presentation
  events of two objectives.
- `m_1a_01_goto_slappys_ollie` (landing volume `m_1a_01_goto_slappys_ollie_land_vol01`), its
  completion events: 21 HideTutorialCalloutHUD, 4 Wait "1", 19 Replay "3.5"
  `highlight_generic_left`.
- `m_1a_01_open_up_dropin_2`, its completion events: 21, 3 SpeechNoWait CameraMan 2725, 4 Wait
  "1", 19 Replay "4.5" `highlight_generic_front`, 11 EnableSetSessionMarker, 3 SpeechNoWait
  CameraMan 942.
- These are the only Replay events in the database's objectives (activation, completion and
  failure events, and the failure-with-events conditions). Scan: the career db's
  `challenge_objective` records.
- `cChallengePresentationDirector::PlayNextPresentationEvent` 0x82e07f38, case 19
  (0x82e08394..0x82e083e4): `DataMan::GetLastFrameNumberWritten(SubmitRestoreFlag_Sim)`, then
  `RunReplayNIS` (vtable +0x14) with:
  - arg2 as the scene;
  - start = last − 60 × `atol(arg1)` (`mulli`, `subf`, unsigned: "3.5" reads 3 → 180 frames,
    "4.5" → 240);
  - end = last;
  - `TRANSITION_TYPE_NONE` (r7), no camera hold (r8 0), skippable (r9 1), no overrides (r10 0,
    `msDefaultPlaybackOverrides`: `NISPlaybackOverrides::Reset` 0x828a2158, focus player −1).
  The case leaves at `LABEL_61` without playing on: the group waits for
  `OnNISChallengeNotify` 0x82e08848 (notification 1), which the scene's end sends.
- `RunReplayNIS` 0x828a3c70 → `cMsgNISEnterReplay` → `ReceiveNISEnterReplayMsg` 0x8286c0f8 →
  `StartNIS`. `highlight_generic_left` and `_front` (`scene_000.bin`): type 4, replay on,
  length 3, no characters.

### Port before

Event 19 fell to the default arm, logged and played on at once: no replay. Live log
(`rp_base`): `unpresented -1 presentation event 19 3.5 highlight_generic_left` at 93.4 s and
`... 19 4.5 highlight_generic_front` at 115.5 s.

### Port now

`challenge/presentation.rs`: `event::REPLAY` outputs `RunReplayNis` (the recorder's last frame
less 60 × `atol(arg1)`, that frame, no hold, skippable, focus −1) and waits; the session plays it
as it plays a highlight (`NisManager::run_replay_nis`, `update_replay_view`).

## Diagnostics

`SKATE2_DIAGNOSTIC_PHOTO_PUSH`: with `SKATE2_DIAGNOSTIC_PHOTO_SEQUENCE`, A held 12 ticks of
every 30 from the spot teleport to the sequence's end, so the photo replay has the skater
rolling.

## Verification

- Build: `cargo build --release -p skate2-pc` passes (02:47, with every change below); no
  warnings in the changed files. The live runs and parity below used the 02:25 build, which
  had everything but the replay opacity (`in_replay`) and the test edits.
- Parity: `parity.sh rp_parity` printed `PARITY IDENTICAL (6000 lines)`.
- Live, scratch saves, `career_run.sh` with the photo recipe of the task:
  - `rp_base` (before the changes): the final capture after the photo mission's autosave
    still shows the photo selection's "Continue" with the Y button. The log has
    `unpresented -1 presentation event 19 3.5 highlight_generic_left` (93.4 s) and
    `... 19 4.5 highlight_generic_front` (115.5 s).
  - `rp_m1` (first mission, forced objectives, captures every 0.5 s): `RunReplayNis {
    scene: "highlight_generic_left", start: 3049, end: 3229, hold_camera: false,
    skip_enabled: true, .. }` at 97.2 s, after the ollie objective's activation group and its
    1 s wait. The captures show the load transition, the replay of the recorded skater
    through `highlight_generic_left`'s lens, the exit transition, then the next objective's
    NIS. The diagnostic's periodic A (forced objectives) skipped the skippable replay after
    about a second. The drop-in's `highlight_generic_front` (start 3506, end 3746) played at
    120.5 s, before the mission's cleanup and autosave (124.5 s).
  - `rp_photo` (`SKATE2_DIAGNOSTIC_PHOTO_PUSH`, captures every 0.25 s): the skater rolls
    about 15 m during the sequence, and the 8 fixed-frame shots leave 8 copies along the rail,
    each with its own hoodie on its own pose, beside the live replayed skater. The photo
    selection ("Continue", Y) shows over the held last shot; after the pick it is gone and the
    end options ("Nailed It, Nailer!") show; the rewards overlay, the autosave (cash 350) and
    the final capture follow with no prompt on screen.
  - No `career script error` in either run.
- Unit tests (`cargo test --release -j 1 -p skate2-core --lib`, non-incremental, game data):
  36 run, 35 pass: `a_replays_seconds_read_as_atol` (new),
  `a_frozen_copy_is_its_own_entity_without_cloth` (new),
  `photo_rail_replays_its_highlight_and_opens_the_photo_selection`, the `skater_ropa_runtime`,
  `presentation_restore`, `render_opacity` and replay clock tests.
  `first_mission_completes_and_autosaves` failed on its new replay assertion: the scripted
  player completes the twelve objectives within a few frames, so the later groups overflow the
  director's eight-group queue (`StartPresentation` 0x82e07bc0 drops a group when 8 are
  queued) and only the ollie's replay was output. The assertion now requires the ollie's replay
  (180 frames) first and only the two known replays; that edit has not been run (no memory for
  another test build). The test helpers end a replay-controlled scene at once
  (`frame_replaying`), as `frame` ends a basic NIS.

## Open

- Scene characters' SH lighting during a paused NIS: the PC blends it once per simulation tick
  (`SkaterLighting::update_tick`), which a NIS pause stops, so characters created during a NIS
  (the frozen copies here, the story scenes' characters) stay at about the default irradiance
  and look lighter than the live skater. `SkaterPresEntity::Update` 0x827e8568 blends 0.98/0.02
  toward the closest probe each drawn frame (0x827e88a0..0x827e8998), paused or not.
- The replay restore's frame numbers (the recorder's, by `world.tick`) and the real-time
  restore's (submission order) are separate counters. The cloth restarts at a replay's start
  because the two are 15 or more frames apart, as natively the replay's start is; a chance
  match within 15 frames would step instead.
