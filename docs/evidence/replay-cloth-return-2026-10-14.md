# Clothing after challenge replay / Replay Editor return

Date: 2026-10-14. PC checkout only. Source correction; not compiled or run.

## Report and ownership defect

A playtest screenshot shows the film/photo results screen (Sick!, Continue,
Retry) with the skater's head, arms and trousers visible but the shirt missing.
Gameplay is still paused at this point.

`SkaterRestore` owns a separate pose palette, but a recorded `SkaterFrame`
retains an `Arc<Mutex<CharacterMaterialAdmission>>` from the gameplay skater.
That material owns the CPU cloth vertex stream and its world-space skin rows.
Restoring the replay therefore changes the shared cloth as well as the replay
palette. Merely switching back to the retained gameplay palette does not put
the clothing back on that pose.

Before this change, `update_replay_view` cleared `replay_clock` when the NIS
stopped controlling replay. The PC's paused-world path then selected the live
palette, with the replay's cloth vertices/world rows still attached to its
materials. No live simulation tick ran to replace them until Continue/Retry.
Replay Editor exit had the same omission before returning to a paused StartHub.
This is a confirmed source defect and is consistent with the screenshot; the
reported run was not reproduced here.

## Original reference

Live IDA was unavailable. IDA 9.3 batch mode read a separate saved debug database,
`world_probe_temp\waypoint_replay_20261014.i64`; the original
database and game data were not modified. Original input SHA-256:
`3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.
Packets include decompiler output and PPC instructions in
`.scratch/replay-cloth-return-20261014/{native,return-native,held-native}.json`.

| Native symbol | Address | Relevant behavior |
| --- | --- | --- |
| `Sk8::GameModule::Render` | `0x827320f0` | Runs scene frame begin, then allowed/unpaused DataSys `StepTime` and `RestoreFrame`, then presentation and frame end. A gameplay tick is not required for this path. |
| `Sk8::DataSysMan::SetTimeController` | `0x8277e688` | Replaces the presentation time source. |
| `Sk8::DataSysMan::RestoreFrame` | `0x8277f040` | Reads the controller's time and restores presentation data. Replay-state notification follows restoration. |
| `Sk8::Presentation::EnterRealTime` | `0x827f1130` | Installs the game time controller. |
| `Sk8::Scene::Scene::Purge` | `0x8286a908` | A non-held replay returns to real time. A held-camera replay instead disconnects its scene from the existing replay controller. |
| `Sk8::Scene::SceneReplayTimeController::Update` | `0x828727e8` | Keeps the last scene elapsed time after that scene pointer is cleared. |
| `Sk8::Scene::SceneReplayTimeController::GetTickCount` | `0x82872720` | Supplies current frame (+0x14), frame fraction (+0x20), rate and direction. Hidden return arguments confuse the decompiler; PPC was inspected. |
| `Sk8::SkaterPresentation::DoSimRestore` | `0x827e6438` | Restores body pose, marks the entity restored, starts skin jobs and later supplies wipeout data. |
| `Sk8::SkaterPresEntity::StartJobs` | `0x827dbb90` | Starts CPU skinning when the local skater's cloth/effects are enabled and ready. |
| `Sk8::SkaterPresEntity::BeginSkinJob` | `0x827d7c48` | Runs cloth from the restored pose. |
| `Sk8::SkaterPresEntity::ClothUpdate` | `0x827d24f8` | Uses the active clock; keeps the original state/15-frame jump restart and elapsed-time behavior. |
| `Sk8::SkaterPresEntity::EndJobs` | `0x827dbd50` | Completes skinning, then packs the matching pose matrices with `m_MatLtoW`. |
| `Sk8::SkaterPresEntity::Update` | `0x827e8568` | A local entity not restored this frame still runs `StartJobs`/`EndJobs`. |

The active-replay cloth reconstruction and frozen-copy cloth-disabled contract
are documented in `career-replay-cloth-continue-ollie-2026-10-10.md`. This change
adds the missing return publication using those existing operations.

## Implementation

- `SelectedPresentationRuntime::restore_game_cloth` skins from its retained
  game palette, saved hips, published root and submitted wipeout header, using
  the last completed game cloth clock. It publishes CPU cloth and matching
  world rows, then republishes the palette. Advancing the palette publication
  also makes the renderer upload the changed CPU vertex stream.
- Reuses `step_restored_cloth`: no fabricated elapsed time, new solver tuning,
  unconditional reset, or simulation tick. The existing original restart and
  zero-delta paths decide how the cloth returns.
- `update_replay_view` reports the transition out of replay. Both career caller
  paths perform the handoff before the next paused visual snapshot.
- Replay Editor exit performs the same handoff before revealing the paused
  world behind StartHub.
- Includes local, AI and party skater runtimes. Unpublished/material-less or
  cloth-disabled skaters are ignored. A game tick that already stepped cloth
  to the same frame is not stepped twice. A character NIS currently borrowing
  a skater keeps ownership of its skin jobs.

## Scope and validation

Confidence: **high** for the retained-palette/shared-cloth mismatch and its
required publication order; no claim of numeric or visual runtime validation.

- Five changed Rust files parsed with rustfmt (`--emit stdout`, `skip_children`
  enabled). The files were not reformatted. This checks syntax only.
- Reviewed the scoped diff against per-file backups and the two career callers,
  Replay Editor return, PC paused visual selection and CPU upload generation.
- No tests added/run, compilation, game launch, save changes or Android edits.
- Runtime follow-up: film and photo results before pressing Continue, skipped
  replays, Retry, Replay Editor exit, then resumed gameplay with clothing.

This patch does not change NIS clock selection. The separate existing
held-camera discrepancy remains: the port currently clears replay on NIS purge
even for a stitched highlight, while native `Scene::Purge` retains its clock
until the next controller takes over. The final highlight uses no hold, so
the reported end-results handoff returns to game time. This correction makes
cloth match the pose the host selects; it is not a certification of every NIS
transition or of the earlier cloth simulation's rate adaptations.

Source/doc backups and a scoped patch are in
`.scratch/replay-cloth-return-20261014/`.
