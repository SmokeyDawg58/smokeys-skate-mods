# In-between frames and render threading — October 8, 2026

Goal: the original's in-between frames, ported 1:1 so the frame rate reflects real
smoothness, then the highest frame rate the test machine allows. Skating physics
had to stay bit-identical, and with restore off the image had to stay
pixel-identical.

This supersedes two open items in `performance-ai-pacing-2026-10-08.md`: the port
now draws frames between ticks, and the renderer is on wgpu 30.

## In-between frames

The original simulates at 60 Hz and draws every frame from each presentation's
last two submissions, blended at the frame's place between them. Each
presentation restores its own data, and the port does the same
(`skate2-core` `rendering/presentation_restore*.rs`):

- Camera: `CameraPresentation::DoSimRestore` 0x827f60e0.
- Skaters: `SkaterPresentation::DoSimRestore` 0x827e6438. ROPA cloth steps once
  per submitted frame and is blended every frame (0x827d24f8; see "Fixes after the
  first playtest" below).
- LivingWorld: `UpdateLivingWorldPresentationEntities` 0x828133e0 (below).
- World animation: `WorldPresentation::Update` 0x827d3af8 takes the restored tick
  and fraction, so trees and water move every frame.

### LivingWorld: pedestrians, vehicles, dynamic objects

`UpdateLivingWorldPresentationEntities` 0x828133e0 keeps a map of the entities it
draws:

- A new entity starts at its newest record, unblended.
- When the previous submission lacks the entity, it slerps the newest record with
  itself and updates with 1.0.
- Otherwise it slerps from the previous record to the newest at the frame's
  percent, and updates with that percent.

A transform is submitted as its QuatFromMatrix candidate (not normalized) and
translation. It is restored through RW `SLerp(Matrix44)` 0x82808ff8:

- The relative rotation is From^T·To over all four rows, so translation enters
  it.
- Below 2° (0x3d0efa35) the rows lerp, and each is normalized with a dot4 and two
  Newton steps.
- Otherwise it rotates about the queried axis (Rodrigues).
- The translation lerps.

Far from the origin the translation keeps nearly every city transform on the
lerp path, as it does natively.

Vehicles:

- Wheel spin and turn are stored in steps: 0x4622f8e0 and 0x4321b390 per turn,
  decoded with 0x38c910a4 and 0x3bcaa525.
- They restore as half-angle quaternions about I and J, slerped with the 5°
  threshold.
- Front wheels are turn ∘ spin.
- Lights come from the newest record.

Dynamic objects slerp their part.

Pedestrians restore their root only. The pose stays per tick, and the palette is
packed again under the restored root each frame.

### The clock: a deliberate deviation

The native `RealTimeController` 0x82993f28 is kept behind
`SKATE2_NATIVE_PRESENT_CLOCK`. By design, the default shows
`now − (tick period + p95 delivery time + 0.5 ms)`, which never freezes or steps
back on uneven PC frame times. The blending math is 1:1 either way.

- `SKATE2_NO_PRESENTATION_RESTORE` draws ticks only. Captures are then
  deterministic, apart from the pedestrian noted under Open.
- `SKATE2_RESTORE_TRACE=<csv>` writes when each restored frame starts and the
  moment it shows.

## Render CPU

Measured on the cruise route: `perf-skate` from `Q_SmokeTest_1300_N900`,
1280×720, 4× MSAA, vsync off, with alternating A/B runs.

The GPU takes about 0.7 ms a frame. The timestamp profile splits it into scene
0.48, overlay 0.10, dynamic objects 0.05 and world shadow 0.04 ms. The frame is
CPU-bound.

| Build | Cruise fps, mean per run |
|---|---|
| Start, wgpu 0.19 | about 250 |
| Bind elision, PostFX readback without a per-frame poll | about 300 |
| wgpu 30.0.1, one scene pass | 315–334 |
| Map runs encoded on threads | 367–379 |
| Render worker pool sized to the work | 439–507 |
| Map job overlapped with the rest of the frame | 604–640 |
| Device poll while waiting on the map job | 648–670 |

| Restore trace | Sized pool | Final |
|---|---|---|
| Restored frames per second | 451 | 665 |
| Frame interval, mean / p95 / max | 2.22 / 2.99 / 8.36 ms | 1.50 / 2.15 / 6.79 ms |
| Frozen or backward steps | 0 | 0 |

### Changes (`apps/skate2-pc`)

1. **wgpu 0.19 → 30.0.1.** Pass encoding moves to `CommandEncoder::finish`, and
   submit drops to about 0.2 ms.
   - API changes:
     - Surface acquire states become `SurfaceUnavailable`.
     - Frames present through `queue.present`.
     - Pipeline layouts take `Option` bind group layouts.
     - Map and poll results are `Result`s.
   - GPU timestamps between passes need `TIMESTAMP_QUERY_INSIDE_ENCODERS`.
2. **Map runs.** `map_draw::draw_runs` cuts the view's selection between objects
   into runs of about equal encoding work. An object's pipeline and material
   binds weigh as much as 4 island draws. `MapRuns::encode` records each run into
   its own pass and command buffer on a render worker. The first run clears the
   targets. Command buffers submit in draw order.
3. **Render workers.** A rayon pool of `SKATE2_MAP_THREADS` + 1 threads. The
   default is available parallelism / 4, clamped to 1..4, which gives 4 here.
   rayon's default 24-thread pool spun idle threads and took CPU from the SIM
   and render threads.
4. **Selection.** `select_visible` culls 256-object chunks across the pool and
   joins them in order.
5. **Overlap.** `render_frame` starts the map job (selection and runs) as soon as
   it has the swapchain image. Meanwhile `record_frame` records the frame's own
   commands on the render thread:
   - Before the map: the shadow maps and the world UI panel.
   - After the map: the objects pass, PostFX, the final blit and overlays.

   One submit sends before, the runs, then after. `GpuMap`'s caches are
   `OnceLock` and `Mutex`, so the job can read the map.
6. **Poll in the wait.** While the map job finishes, the render thread polls the
   device. The submissions the GPU has finished release their resources there,
   not in submit, which drops from 0.33 to 0.21 ms. Map callbacks still fire
   before the next frame records, as they did when submit ran them.

Frame profile columns that changed meaning:

- `map_prepare_ms`: the selection, on a worker.
- `scene_cpu_ms`: the objects pass.
- `pass_end_ms`: closing the objects pass, then waiting for the map's runs.
- `finish_ms`: encoding the render thread's two command buffers.

### Verification

- `parity.sh`: IDENTICAL (6000 lines) after every change.
- Captures with restore off, at 600 ticks:
  - wgpu 0.19 against wgpu 30: 0 pixels differ.
  - Before against after the overlap: identical. Three overlap runs agree byte
    for byte.
- `cargo test -p skate2-pc --release`: 128 pass.

### Measured and rejected

- **More map threads (6, 8).** The wait shrinks, but every command buffer adds
  submit cost, and the render thread's own recording slows. 4 is best here.
- **Bindless materials.** A frame sets about 3,100 material bind groups: one per
  drawn object, with only about 25 pipeline changes. That is about 15,000 texture
  registrations in wgpu-core (`register_init_action`, `merge_single`).
  - Per call, it costs the same on one thread as on four. The cost is cache
    misses on cold texture objects, not lock contention.
  - wgpu-core registers a binding array's every texture on each set, so bindless
    would not reduce this.

## Fixes after the first playtest

**Pedestrian LOD crash.** "pedestrian LOD pose before Sync" ended a session.
- `PedestrianRender::update` now runs every frame. A frame between ticks that
  changed a pedestrian's geometry LOD read the live pose storage, which the SIM
  thread had marked unsynchronized while evaluating the next tick.
- `SelectedPedestrianVisual` now keeps a copy of the synchronized pose of the tick
  that built it. `SwitchLOD` 0x82806528 repacks from that copy on any frame.

**Cloth too fast.** The T-shirt cloth moved far too
fast.
- The step's time was right: the restored time since the last step, 1/60 per
  frame.
- `SolveConstraints` applies its distance and driver passes once per step,
  whatever the dt. Stepping on every drawn frame (about 600 a second) pulled the
  cloth back to the body ten times as often as the original, which drew, and so
  stepped, at most 60 times a second.
- `ClothUpdate` now steps once per submitted frame, at that frame's own
  unblended pose and with `ClothClock { frame, 0 }`. That is the tick path's
  cadence and dt.
- `SkaterRopaRuntime` keeps the hips-relative positions of the last two steps.
  Each drawn frame publishes them blended by the restore's fraction, with
  normals and palette rows from that frame's pose.
- The tick path is step-then-publish at 1, so its output is unchanged.

Verified:
- `parity.sh`: IDENTICAL.
- 130 app tests pass.
- Restore-off captures at 600 ticks match the previous build: two pairs agree,
  and one run shows the known pedestrian-sized patch.
- Two restore-on cruise runs complete with no failure, at 572 and 559 FPS. Frame
  interval maxima are 10.3 and 7.1 ms, with nothing frozen or backwards.

**Skater fade too fast (October 19).** The in-between restore ran `DoSimRestore`'s opacity
approach (0x827e7350..0x827e7390, each rate times `kUpdateTime`) once per drawn frame, so the
camera-proximity fade ran several times the original's speed. It now steps by the frame's own
time; see [grind-transparency-2026-10-19.md](grind-transparency-2026-10-19.md).

## Open

- The map runs (about 0.7 ms) are the critical path. After them come the client
  frame (about 0.25 ms), submit (about 0.2 ms) and present (about 0.1 ms).
- About one run in four, a capture differs from the others in one
  pedestrian-sized patch. This happens with and without these changes.
- NIS scene characters still draw per tick. The world DMO PresManager is not
  drawn by the port.
