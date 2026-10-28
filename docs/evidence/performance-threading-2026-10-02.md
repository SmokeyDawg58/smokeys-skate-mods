# Performance: SIM thread, async loads, caches — October 2, 2026

Parts of the city ran at 55–80 fps with multi-frame stutters on the test machine
(500–800 fps elsewhere). Goal: maximum optimization without changing skating
physics. Scripted, uncapped profiling sessions ran with an isolated save root.

## Measurement

`SKATE2_FRAME_PROFILE` CSVs, `SKATE2_NO_VSYNC=1`, 2400 completed ticks:

- Pool: `SKATE2_DIAGNOSTIC_SPAWN_LOCATION=F_UR_PermissionPool`
  `--live-controls-demo perf-skate` (the skater rolls in the bowl and crosses
  a terrain collection boundary repeatedly).
- Cruise: `Q_SmokeTest_1300_N900` with the same input; reaches the old 72 fps
  spot near (1345, −767).

`perf-skate` is a benchmark-only raw pad sequence: hold A to push, then one
second of A every four. No steering or tricks.

New profiler columns: `tick_terrain_release/focus`, `tick_session_marker`,
`tick_queued_frame`, `view_*` (city streaming, map frame, character, DMO,
vehicles, pedestrians, marker UI), `terrain_poll/admit/finish/depart/request`,
and render `objects/pass_end/post/finish/submit/present`.

## Findings

1. Sim and render ran serially on one thread: ~7.5–8.6 ms per tick plus
   ~3–5 ms render recording per frame in heavy spots.
2. Car/ped recipe loads ran synchronously inside `RecipeMan::Update`
   (8–40 ms every ~2 s, one load per 120-tick countdown).
3. Every pedestrian mesh creation (spawn or LOD switch) re-read and re-parsed
   the whole `skater.vlt`/`skater.bin` Vault from `db.big` and re-uploaded its
   textures (5–7 ms view spikes about once a second).
4. Terrain collection admission decoded every arena of a collection inline on
   the tick (`drain_prior_wake`, 15–41 ms per event).
5. The PostFX colour cube re-blended and re-uploaded 32³ texels on every
   redraw of an unchanged presentation update during a 6 s LUT blend.
6. GPU time is ~0.6–1.0 ms per frame (Vulkan, NVIDIA); the render thread is
   CPU-bound in wgpu recording/submission.

## Changes

- `live_skating_client.rs`: `LiveSkatingWindow` runs gameplay ticks on a SIM
  thread (the original also split SIM and PRES). A tick runs with the session
  locked; the window thread consumes it (snapshot, `view.update`, acks,
  diagnostics) before the next may start, so the order nav sync → tick →
  view update → next nav sync is exactly the serial loop's. Boot, frontend
  modes (pause, map, editor, main menu, display settings), completion bounds
  and ≥2-tick catch-up run inline exactly as before. `SKATE2_SERIAL_SIM=1` or
  `SKATE2_LEAN_TRACE` keep everything on the window thread. `frame()` is split
  into `stage_tick`/`run_tick`/`complete_tick`/`finish_redraw`.
- `SelectedSessionTerrainSlot` trait object and `try_select_terrain` effects
  gained `Send` bounds (the only non-`Send` session member).
- Ambient recipes: `begin_ambient` (registry hit or request), worker
  `AmbientLoadRequest::prepare` (read/parse/scene/collision/splines/finalize;
  pure), `finish_ambient` (publish, retain, assets). Completion is delivered
  `AMBIENT_LOAD_UPDATES` (8) RecipeMan updates after the request, waiting for
  the worker only if it is late — deterministic in simulation time. The
  native loader is asynchronous too; no native latency is claimed.
- `load_material_attribute_bindings` caches resolved bindings per (mount,
  key); resolution is a pure function of the Vault. Pedestrian textures are
  cached by asset ID (name-checked).
- Resident collection driver: `CollectionAssetBuilder::prefetch` (default
  `None`). The Simulation arena builder starts each submitted job's pure
  decode/collision/plan build on a worker when the pump submits it; the drain
  after the next wake completes it exactly as before (same tick, FIFO order,
  identity-checked, inline fallback). Error latching is unchanged.
- PostFX cube upload skipped when (current, target, blend) equals the last
  upload: identical texels.

Tried and reverted: a map render bundle recorded from an enlarged frustum
(fewer re-records but ~6% lower average fps, no worst-frame gain).

## Results (uncapped)

| Route | Before p10/median fps | After p10/median fps |
|---|---|---|
| Pool | 128 / 189 | ~397 / ~449 |
| Cruise | 72 / 157 | ~383 / ~445 |

Worst frame per second (cruise): median 17.0 → 5.6 ms, p90 20.9 → 9.3 ms.
Recipe tick cost 26–40 ms → ~0.02 ms. Terrain admission 15–41 ms per event →
~5 ms spread over ticks. Pedestrian view spikes 5–7 ms → <0.7 ms.

## Parity

Per-tick `SKATE2_TRACE_SELECTED_STATES` (board position/speeds, Action/
Motion/physical state IDs, offboard/manual flags), pool route, 1500 ticks:
byte-identical (6000/6000 lines) between the original serial path and every
step (threaded; + async recipes; + concurrent decode; + prefetch).
Living-world population timing already varied run to run before these
changes (render-side city streaming feeds presentation navigation).

Scripted flows `main-menu-gameplay`, `main-menu-roundtrip`, `map-teleport`,
`edit-skater-roundtrip` exit 0 with their own assertions (isolated save root).

## Remaining

- Render thread: occasional 9–12 ms frames from wgpu 0.19 device maintenance
  (resource triage after map-bundle re-records, HUD vertex buffers, streaming
  retirement) and map bundle re-recording (~2 ms). A newer wgpu would cut
  per-draw/tracking cost substantially; not started.
- New pedestrian/vehicle models still create their GPU objects on the render
  thread (3–5 ms when a new model first appears).
- Gameplay presents a new image only per 60 Hz tick; frames above 60 fps repeat
  it. Smooth high-refresh motion would need presentation interpolation.
