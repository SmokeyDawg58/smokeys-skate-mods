# Location-dependent frame-rate investigation

## Captured slow run and ongoing diagnosis

A playtest captured `artifacts/frame-profile-29973-12902.csv`, showing sustained
50–65 FPS while stationary facing downhill, plus invisible collidable cars.
The slowest one-second window averages 23.18 FPS: tick work 31.03 ms/redraw,
recording 6.15 ms/redraw. A separate worst redraw takes 127.46 ms, including
94.54 ms in surface acquisition. These are CPU durations, not GPU timestamps;
catch-up redraws can contain multiple simulation ticks. Earlier renderer-only
changes have not established or resolved the primary cause.

Added optional in-memory core stage totals to the SAME frame capture: queued,
actor, pre-solve, solver, post-solve, camera, prediction, breadcrumb,
presentation; nested recipe/LivingWorld/vehicle-ground timings identify costs
within their enclosing stages and must not be summed again. Vehicle probes
record published visual/draw counts and the nearest car's opacity, world
position and first palette origin. No gameplay/simulation-rate change.

Compilation and targeted runtime diagnostics
ran from September 29. This supersedes the
older no-build restriction for this investigation. Release build is in
progress; no claim of a visibility/performance fix yet.

Additional source checks: UpdateAnimationPoses82809610, skinned Update828121d8,
UpdateBoneTransforms827efb78, vehicle InitSkinning82806668, BindConstants8280b8f8
and PoseWithMatMult82a23528. Original vehicle_defaultVS UPDB uses BoneIndices.w
and three palette rows per bone, matching the current host convention. These
checks alone do not establish that live palette contents or vertex fetches are
correct. Continue from measured runtime data; do not suppress car collision or
force opacity as a substitute for locating the failure.

The sections below describe the earlier first-pass work before this capture.

Playtests show up to about 1000 FPS, but about 65 FPS in some locations, on the
test machine. Whether low FPS persists after streaming settles is
still being tested. No slow-location timing sample is available yet.
No claim that the changes below solve the full drop or guarantee 500 FPS.

## Read-only findings

- City CPU preparation and GPU map building already run on workers in
  `city_streaming.rs` and `render_inspection.rs`; old GPU-map retirement also
  runs on a worker. Do not duplicate that earlier work.
- `GpuMap::draw` traversed/classified every retained map object and candidate
  island on every redraw, including redraws between completed 60Hz camera ticks.
- `world_shadow_draws` similarly repeated source world-shadow culling, fitted
  bounds and draw-list construction for an identical projection/geometry.
- Vehicle renderer evicted meshes by the currently selected `(recipe, LOD)`
  pairs every update. Once the last car switched away from a LOD, switching back
  could force another mesh decode/upload on the window thread.
- Actual new vehicle resources/animation setup still happen in visual update.
  DMO resource admission and shared wgpu worker/foreground resource contention
  also remain possible transient costs; not proven by measurements.
- Map culling currently explicitly omits original anti-frustum occluders and
  minimum-pixel/area rejection. Thus geometry inside the camera frustum but
  hidden by buildings can still be submitted. This is a substantial candidate
  for dense-view draw/GPU cost, not yet established as this report's cause.
- Dynamic constants, shadows and GPU draws still execute per redraw. Geometry
  selection caching does not eliminate their upload/submission/GPU costs.

## Implemented host optimizations

`gpu_renderer/map_draw.rs` now retains an ordered flat list of visible islands
per immutable map generation. It uses exactly the previous classifications,
bounds, draw order and draw ranges. Exact clip-matrix inequality invalidates it;
new chunk generations construct empty caches. No per-object allocation in the
cached list. Cached world-shadow selection uses the complete existing projection
and settings as its invalidation input, and retains the original fitted rows.
Failed selections are not cached. Lighting/fog uniforms continue updating.

`vehicle_render.rs` keeps already uploaded LOD meshes while that recipe is used
by current visuals. The same authored distance thresholds, selected LOD,
animation, materials and physics remain in place. Eviction occurs when the
recipe leaves the visual set (or all vehicles leave). This increases retained
GPU memory by the visited LODs of those recipes, bounded by those recipes/LODs;
it is not an indefinitely growing history cache.

These are host reuse changes, not new translations of gameplay or native culling
math. Existing source-derived operations and their address annotations remain
unchanged; no new native parity claim or missing occluder implementation.

## Optional measurement build

Compile using the existing `build-release.bat`, then run `play-performance.bat`.
It uses normal quiet controller gameplay and sets `SKATE2_FRAME_PROFILE` to a
unique `artifacts/frame-profile-*.csv`. Spend 10–15 seconds still in a slow area,
rotate the camera without changing position, then exit normally with Escape.

The capture keeps one-second aggregates in memory (last 1800 windows, roughly
30 minutes). Nothing is written to disk or console during play by this capture;
the report is written on normal shutdown, including handled runtime errors.
Forced process termination cannot save it. Normal `play-quiet.bat` does not
enable capture, allocate its samples, or create a report.

CSV has a mean and worst-redraw row per window, window presentation FPS and
95th-percentile redraw duration. It records host time for client, fixed ticks,
visual snapshot, visual update, surface acquisition, total command recording,
dynamic uploads, dynamic/world shadow recording, scene recording and submission.
Position, tick count and map/shadow/dynamic draw counts describe the worst frame
of that window (including on the mean row, explicitly `worst_*` columns).

These are CPU durations, not GPU timestamps. `tick/snapshot/view` are subsets of
client; shadow/scene/upload durations are subsets of record. Do not add them
again. Acquire and submit/present may include GPU/driver backpressure; a high
wait alone is not proof of a particular GPU shader bottleneck. Window FPS also
includes time between redraw callbacks, whereas redraw duration does not.

## Validation

Scoped rustfmt and static call-site/initializer inspection. Existing GPU map
regression extended to compare cached versus uncached selection and verify
camera/projection invalidation independently of fog. Test UNRUN. No Cargo,
compiler, GPU test or runtime launch this turn.
The new report is required before attributing the full 1000-to-65 FPS drop.
