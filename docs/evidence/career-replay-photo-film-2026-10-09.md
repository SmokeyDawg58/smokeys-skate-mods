# Career replay, photo and film missions, saves, frame pacing — October 9, 2026

Scope: the replay system for the photo and film missions, saves and
unlocks, the objective list after a challenge, a frame-rate counter option, the 120→60 fps
drop after a mission, the same skater for both Party Play players, and the pause menu's
Debug Menu row. Native reference: `sk82_na_m.xex` (debug, Jan 17 2009) in IDA.

## 1. Replay recorder and replay-controlled NIS

`DataSystem::DataMan` keeps the recorded frames; the port keeps the same index
(`challenge/replay.rs`) and the presentations themselves (`PresentationSubmission`, one per
simulation tick) in the session.

- `DataSysMan` ctor `0x8277f600`: `numframes` 5400, `desiredsize` 12 MiB, `pinratio` 0.6.
  The port's frames are uncompressed, so the frame count stands in for the size.
- Frame numbers: scripts pin from `SequenceBeginTime = Time.GetVisualGameTick()`
  (`Simulation::mTick`) to `ChallengeReplay.GetLastFrameNumberWritten`, so a recorded frame
  takes its tick's number (`store_frame(tick)`; a number not past the newest is not stored).
- `PinnedFrameManager`: four slots; `RequestPin` `0x829935b8` fails when no slot is free, when
  the closest frame is not before `to`, or past 60%; `FrameStored` `0x82993820` extends pins
  waiting for future frames. `GetLastFrameNumberWritten` `0x82992138` reads 0 when empty.
- `cHighlightGroup`: `PlayHighlightGroup` `0x82cd7b68` → `PlayHighlight(0)` `0x82cd75e0` →
  `NISManager::RunReplayNIS(scene, pinStart, pinEnd, 0, stitch, skip 1, overrides)`
  `0x828a3c70`; `HighlightUpdate` (in `Simulation::PostUpdate`, after the broadcaster flush)
  moves FrameWait → Playing; `OnNisCompleteMsg` `0x82cd79f0` plays the next highlight or
  sends `cMsgHighlightReelComplete`.
- `RunReplayNIS` pauses the simulation, pushes `PauseMan` 7 and sends `cMsgNISEnterReplay`
  (`ReceiveNISEnterReplayMsg` `0x8286c0f8`).
- `Scene::Init`: length `max((end - start) / 60, 1.5)`; `SceneReplayTimeController`
  (`Update` `0x828727e8`): scrub = start + sceneTime × 60 clamped to end, whole-frame
  SkipForward/SkipBackward, the fraction kept (`session/selected_session_replay.rs`).
- The PC view draws the restored recorded skaters, pedestrians, vehicles and DMOs under the
  scene's camera (`PresentationRestorer::restore_recorded`).
- Memory: a full buffer (5400 frames) held about 150-200 MB more than an empty one in a career
  run (working set ~1.0 GB -> ~1.16 GB, then flat), about 30 KB a frame; the native keeps a
  12 MB compressed buffer.
- `cMsgEnterRealTime` (exporter `0x828cc6a0`, `PresThreadMsgMan::ReceiveEnterRealTimeMsg`
  `0x8277bd58` → `Presentation::EnterRealTime` `0x827f1130`): the restore leaves the replay's
  time. It was the first `cMsg*` exporter the port reconstructs; every end-options Continue of
  a photo, film or Own-the-Spot challenge calls it.

## 2. Photo missions

- `NIS.GetPhotoNISCount(definition)` `0x8288bc98` → `NISManager::GetPhotoCount` `0x828a0e70`:
  the freeze channel (34) intervals whose command (element 141) is `PhotoShot` (4).
  `photo_killed_m_1a_05_photo_rail` only has `PhotoShot_FixedFrame` (5) shots, so its count is
  0 and the selection shows only Continue — native.
- `FrontEnd.OpenPhotoSelection` `0x82896cb0` → `FrontEndState_PhotoChallenge` (FE state 5,
  screen 116, `challenge/photoselect.swf`, placement `g3dHUDRight`): RT/LT change the photo
  (`cMsgNISPhotoSelect` sets the scene time to the shot's), Y accepts, then `PhotoDone`
  `0x82660320` (FE state 8 + `cMsgNISComplete`).
- `ICEUserChannelFreeze::FireEvent` `0x826d1b50` command 5 (`PhotoShot_FixedFrame`), with
  time running and the local skater in camera 5's view (`Frustum::TestSphere_VMX`
  `0x8282c208` on `GetBoundsCenter` = hips, `m_BR` = half the local bounds' diagonal,
  `GetMinPixelSize` 19; `GetPixelSize` `0x828268a8` = `m_H` r/(d−r), `m_H` from
  `CalculateH` `0x82826830`): the freeze for `frames/30` s, the bloom pulse (`s_fBloom` −0.6,
  up 0.15 s, back 0.1 s), `Take_FixedPhoto` (85) to the front end (the fixedphoto
  treatment), and `mInstanceFrozenSkater`.
- `ICEUserChannelFreeze::UpdateFrameEnd` `0x826cf1a8` → `InstanceFrozenSkater`
  `0x82874f90`: a copy of the local skater in its saved pose stays in the scene. The port
  keeps the restored local skater's palette, hips and bounds and draws the copy as a scene
  character with the local skater's materials until the replay scene ends.

## 3. Film missions

Photo and film share the spot template. A film's `challenge_cleanup` goes to `killed_it`
(pause menu off, the player pinned); a win waits `TimeToWaitBeforeReplay`, pauses and plays
`highlight_reel` (`HighlightReel.PlayHighlightReel(group, false, true)`), a loss waits 1.5 s;
both reach `end_options`, whose Continue sends `cMsgEnterRealTime`, unpauses and kills the NIS.
`sponsor_me_01` (the first main-game mission) has one objective group; its replay scenes are
`killed_/replay_m_1a_07_spotfilm_sponsor_me_1_01/02` (replay-controlled). The headless test
`sponsor_me_film_runs_out_of_film_and_continues` runs the loss to the end. The win was not
driven headlessly: its objective conditions are runtime-compiled scripts inherited from a
template record and need their trick events; forcing objectives completes them without points
and reads as a loss. Not yet played live.

## 4. Saves and unlocks

- Every career autosave writes `UNLOCKABLES`, `DYNAMICOBJECTS` (hull states) and `SignPost`
  (progression) with `SKATER_PROFILE`, read back and validated before the replace.
- The file work now runs on a "career save" thread, as the original's storage device writes
  in the background: `edit_skater_profile.rs` serializes every SKATER.P access and every
  access made after a hand-over (`HandedSave`) waits for that save, so nothing reads a stale
  file or lands between its renames. The autosave tick used to take 45–64 ms.
- Freeskate locations: `fe_location_group` 'freeskate' (27) filtered by `IsLocationUnlocked`
  and `IsValidLocation` — 1:1. Default unlocks are group `0x251427BAFAE99434` (21). Locked
  until the career: clubhouse, stadium, gvr, trainingfacility, svpeak, svdam
  (`UnlockLocationForFreeskate`). "Fun Track" (`funpark`) is unlocked with the mega park at
  "FMV - Unlock Danny Mega Park" but is in no location group, so the original never lists it.
- Not ported: the dynamic hulls' geometry (the gates themselves: `PJ03_SlappysGate`,
  `SVM_01_TunnelLock`, the training facility, `megapark_extension_gates`, GVR, caps, pool
  water). Their states are tracked and saved; the hull files are not loaded or drawn.

## 5. 120 → 60 fps after a mission

Cause, from `SKATE2_PRESENT_TRACE`/`SKATE2_RESTORE_TRACE` (photo run, autosave at 171.4 s):
for ~0.75 s after the save the restore drew one frame per tick (moments 6398.000,
6399.000, …) instead of ~12. The scheduled restore clock shows `now − (period + delivery)`,
`delivery` being the 95th percentile of recent tick latencies, and keeps three frames. A
reset (every NIS, results screen or pause) empties the latency window, and the first ticks
after it were a stall late (the autosave's 37 ms). With a few samples, those were the
percentile: the delay passed a period, the moment fell behind the tracked frames and stayed
on whole ticks, so frames between ticks repeated the last one and were not presented —
60 fps at a 120 cap until the samples aged out.

Fixes: a tick later than its own period (`sim_rate`) is a catch-up, not delivery spread, and
no longer enters the window (`presentation_restore.rs`, test
`a_stall_as_ticks_resume_does_not_hold_the_clock_back`); the autosave's file work moved off
the simulation thread (section 4).

Verified live (uncapped runs, `SKATE2_RESTORE_TRACE`): before the fix the 0.75 s after the
autosave drew 15 restored frames per 250 ms (one per tick); after it, 191 in the first 250 ms
after the resume, and the worst tick latency at the save fell from 37 ms to 20 ms.

## 6. Animation graph expression coverage (crash)

A live photo run stopped with `PendingMaskedExpression { mask: Sustain, expression: 86 }`
during `m_1a_05`'s `challenge_running`: the board motion graph reached a Sustain expression
the port had not listed. The evaluators were completed against their native jump tables:

- Motion Sustain (`EvaluateExpression<2>` `0x8291e218`, table `0x8291e254`, 447 cases): 29
  remaining default cases (86 among them) return true like the native default `0x829221c8`
  (`li r3, 1`); 16 cases with bodies were added (228, 272, 310, 314, 316, 317, 320, 321, 326,
  327, 332, 333, 357, 358, 412, 418).
- Motion Pre (`EvaluateExpression<1>` `0x8290aa50`, table `0x8290aa90`): 13 cases added (89,
  211–214, 228, 229, 295, 343, 388, 412, 424, 425).
- Action graph (`0x828ff990` Pre, `0x82906048` Sustain, 107 cases): Pre 30, 34, 70, 78;
  Sustain 70, 78 and the defaults 30, 33, 34.

Every case keeps the native read order and stops at the first deciding read.

Two more crashes surfaced in live photo runs with the player in the shot:

- AI respawn: `PathController::ShouldRespawnOnPath` (`0x824bf1c0`) also returns true when
  `IsInPathRespawnChallengeType` (`0x824bf108`) does, i.e. when the challenge AI manager has
  no `mRespawnLocation` (only races set one, `AI.SetAIRespawnBehaviour(true, ...)`). The port
  lacked that test, so a photo challenge's AI skater that wiped out for too long asked
  `TeleportToSpawnLocator` for a respawn point that does not exist and stopped the game.
- APT textures: the marker UI resolved a movie's `<owner>_textures\N.texture` by suffix, and
  `nonfixedphoto_textures\1.texture` also ends with `fixedphoto_textures\1.texture`; the fixed
  photo treatment (FE event 85) found two textures and stopped the game. Suffixes now match
  whole trailing path components.

## 7. Smaller items

- Objective list after "challenge complete" (task 3): native. The completed objective stays
  listed for `kObjectiveNotificationWindow` (1 s), then the HUD shows the title only until the
  template's `show_results` (3 s) deletes the group.
- Video Settings: Frame Rate Counter row (Off by default, saved with the display settings) —
  a PC option.
- Party Play: both players may choose the same skater (a PC change).
- Pause menu: the debug build's "Debug Menu" row is gone.

## Diagnostics

`SKATE2_DIAGNOSTIC_CHALLENGE2`(+`_TICK`), `SKATE2_DIAGNOSTIC_PHOTO_SEQUENCE=<tick>` (posts
m_1a_05's sequence messages), `SKATE2_DIAGNOSTIC_PHOTO_PICK=1` (RT, Y, then A on the end
options), `SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES_END`, `SKATE2_TRACE_WAKE`.
