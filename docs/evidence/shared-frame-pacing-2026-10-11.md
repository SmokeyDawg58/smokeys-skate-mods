October11 ANDROID DROPS / PIPELINE FOLLOW-UP (in progress).
- Supersedes older build5/source-only status below. Normal Android builds9 and10
  succeeded after RPCS3 closed; both installed. PC source remains uncompiled.
- Build9 still trapped at four ticks/draw; slow capture ~48-58 ticks/second.
- Build10 in BOTH trees uses measured 1..4 catch-up budgeting and releases SIM
  before view.update_frame. All elapsed debt and per-tick completion retained.
  Five focused scheduler checks passed; not full native physics trace proof.
- Build10 still drops: fresh capture slow windows average16.3FPS/50.7ticks/s;
  simulation debt grows. The scheduling change alone did not solve the issue.
- Perfetto active-turn trace: main65% on efficiency CPUs; SIM performance-core
  runtime at ~1.1-1.2GHz. Thermal HAL current cooling devices report0.
- Android-only ADPF build11 compiled (2m53s), signed/aligned and installed.
  Android confirms an allowed session for render+SIM; runtime capture underway.
  Report real redraw
  duration/deadline with native render+SIM TIDs, close on pause/focus loss.
  Supported HAL previously had no sessions. Performance effect UNVERIFIED.
- Captures/backups: Android .scratch/pipeline-recovery-20261011. Reference
  AppMan MainLoop82989ae0 / RenderLoop82989fc0 rechecked live. Host scheduling
  policy is not an original-console algorithm. No physics/core edits this pass.

# Shared frame pacing, Lua GC and replay bookkeeping ? 2026-10-11

## Current status

Implemented in both this repository and the Android port.
Physics parity is preserved in both copies.
No successful full build or gameplay run for this
candidate. The installed Android APK is still build5. Do not claim a measured
FPS improvement or complete native physics equivalence for these changes.

## Changes

- Challenge event flushes use the existing allocation-triggered host collector
  instead of forcing three full heap sweeps after every flush. Script dispatch
  ordering is unchanged. Native `cLuaStateManager::CreateState` 0x82aae998 calls
  `lua_gc(L, 0, 0)` at 0x82aaeadc; `lua_gc` 0x829994f8 confirms STOP. The original
  arena's allocation/reset machinery differs from this host arena. The existing
  8192-allocation host threshold is retained, not presented as a native value.
- Table traversal only emits arena-owned table/function/userdata references;
  it avoids cloning scalar/string keys that marking immediately discarded.
  `traversetable` 0x8299cf60 tag-checks collectable values. Host strings remain
  Arc-owned. Native weak-table/finalizer/arena parity is not established here.
- Top-level Lua call results remain explicit roots during automatic collection.
  Previously the returned Rust Vec was off the Lua stack when GC ran. Native
  `luaD_call` 0x8299c398 checks collection at 0x8299c424 while native return values
  are still stack-rooted. A focused test reproduces the old freed-object panic
  and passes after the correction for returned tables, userdata and functions.
- Replay payload retention merges sorted frame keys and the sorted tracker in
  one pass, replacing repeated membership searches. Pins, gaps, resets, erase,
  audio/visual payloads and ascending destruction order remain as before.
  Native references: InternalSubmit 0x82997968 and EraseAllButLastNFrames
  0x82994368. The existing host uncompressed 5400-frame budget is unchanged;
  it is not the native compressed-byte allocator.
- Threaded catch-up now requests every extra tick from SIM and waits through
  the existing mutex/condition-variable handoff. Every completed tick still
  receives complete_tick and navigation updates before the next tick starts.
  Retains the four-tick redraw budget and two-period catch-up trigger; no
  elapsed slots are discarded and the serial/frontend path is unchanged.
  Timeout rechecks detect mutex poisoning if the worker panics before notify.
- Idle waits require a worker and eligible threaded gameplay. Frontend and
  explicitly serial operation no longer wait for a worker unable to run.
- Optional CSV profiling reports ticks/frame, worker ticks/frame, simulation
  ticks/second, maximum observed debt, mutex wait and SIM-ready wait. PC retains
  its existing output fields; Android retains its additional surface dimensions.

### Important correction to the earlier Android draft

The unbuilt Android-only draft disabled catch-up in consume_tick entirely.
That could cap simulation progress to one tick per rendered frame, slowing the
simulation when rendering is below the timer rate. It was replaced before any
build/install by the bounded worker handoff in BOTH copies. Earlier notes saying
Android consumes just one worker tick per redraw are superseded.

Moving work onto SIM does not remove the cost of overdue work: bounded catch-up
can still make a displayed frame expensive, and the window must still complete
each tick. This is not a fully independent buffered simulation/render design.
The GC and replay fixes reduce actual work; scheduling performance still needs
measurement. Inspect simulation rate and debt alongside FPS before claiming gains.

Native AppMan::MainLoop 0x82989ae0 performs Update at 0x82989ed8 separately from
RenderLoop 0x82989fc0 / Render at 0x8298a110. MainLoop's locking depends on
IsThreadSafe; GameModule::IsThreadSafe 0x8272a000 reads kbEnableMT. This supports
thread ownership, not a claim that the host mailbox reproduces native OS timing.

## Parity safeguards and verification

- Backups and source manifests: `.scratch/shared-frame-pacing-20261011` in each
  tree. PC changed exactly eight Rust files. Android's preceding GC/replay/
  diagnostic changes were already present; this pass only changed its scheduler.
- All five shared core files are byte-identical between copies; final scheduler
  handoff code is identical too. Platform entrypoints and Android lifecycle
  handling are retained separately.
- Compared stage_tick, run_tick, complete_tick, finish_redraw and run_sim_thread
  against each tree's pre-edit backup: all five implementations are unchanged.
  No physics solver, contact/integration math, RNG, AI update rate, input polling,
  fixed 1/60 physics dt or slow-motion timer arithmetic was changed by this pass.
  Thread scheduling and wall-time input sampling are not claimed identical.
- Rustfmt emit-to-stdout parsed all eight candidate Rust files in both copies;
  it did not rewrite source. This is syntax validation, not whole-app type checking.
- Standalone Windows rustc harnesses compile each tree's actual Lua and replay
  modules using the existing PC formats rlib. Each passed 23/23 tests, including
  the external game-data test: 627 ChallengeCore + 910 progressioncore chunks,
  with the expected 627/6 handler globals. These are host-side checks of both
  source copies, not ARM64 device execution or a full engine test suite.
- New focused checks: returned Lua references at the allocation threshold;
  live/deleted table, function and userdata keys/values; optimized replay
  retention versus the previous membership oracle over 12000 frame numbers
  with pins, gaps, release, erase and reset; elapsed-slot conservation for
  normal/slow-motion periods and stalls up to 600 ticks.
- Replay-retention and scheduler predicates are extracted verbatim from the
  production source into the small harness. The scheduler check validates debt
  arithmetic, not the entire threaded UI/physics integration.
- Pre-change Lua harness: 21 passed, 1 failed, 1 original-data test ignored.
  Failure is the new returned-value-root regression (`live Lua object` panic).
  Candidate PC and Android-source harnesses: 23 passed, 0 failed/ignored.
- PC cargo check --release -p skate2-pc -j1 failed because Windows committed
  memory ran out (2097152-byte allocation failure, 0xc0000409). It did not finish
  application type checking. Prior Android build8 also failed for memory.
  No compiler is left running; no further builds launched.
- No new full-game state trace or on-device performance capture for this candidate.
  Earlier 6000-line physics traces from other changes are not evidence for this
  candidate. The source audit and focused checks support unchanged physics
  semantics but do not prove all original-game numerical parity.

## Files

Shared core changes: challenge/system.rs, challenge/lua/{mod,table,vm}.rs,
session/selected_session_replay.rs under crates/skate2-core/src.
App changes: live_skating_client.rs, render_window.rs, frame_profile.rs under
apps/skate2-pc/src. No changes to saves, assets, resolution, physics coefficients
or compiler floating-point flags in this pass.

## Resume after compilation

1. Confirm both platform builds succeed. PC check log and focused harnesses/logs
   are under the PC `.scratch/shared-frame-pacing-20261011` directory.
2. Run the established isolated-save 1500-tick pool route and compare its 6000
   board/state/offboard/manual lines against the saved port baseline; repeat
   serial/threaded. This remains a port regression, not a native trace proof.
3. On the test phone compare active Party Play and city captures, not just turn prompts.
   Inspect ticks/second and debt at low FPS, plus GC memory, replay/HoM, pause,
   foreground/background and controller input. Controller had been disconnected.
4. Confirm backlog recovers after transient stalls. Do not erase accumulated
   time, lower simulation frequency or change physics to improve the FPS counter.


## PC release built, October11

Normal `cargo build --release -p skate2-pc -j1 --timings` completed successfully
in12m43s,exit0, using the existing release profile. Core435warnings and app18warnings;
no compiler errors. New executable SHA256 `055672682B948BE2F1F3721F4031AA4CE9C5F3E73E0773982A239A5B6AD3EC6E`.
The prior executable was backed up in `.scratch/pipeline-recovery-20261011/`;
full build output is `pc-release-build-20261011.log` in that same directory.
Current shared Lua/replay/HUD/scheduling changes, including the SIM-dominated
batch guard, are in this release. No further game-source edits or runtime tests
were performed this turn; the build is ready for a hands-on test.
Android14 is installed and its investigation remains paused.
The animation index/hierarchy-copy investigation created backups only: the five
PC animation source files still exactly match those backups.
