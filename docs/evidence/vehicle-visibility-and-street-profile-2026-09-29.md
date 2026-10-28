# Vehicle visibility and measured street slowdown

## Follow-up:70FPS, moving-view reproduction

`artifacts/frame-profile-19215-7113.csv` (19:03:32) confirms80.01FPS for the
slowest full second, p95 redraw16.5465ms, mean tick4.1104ms, record5.6862ms,
submit2.5125ms. Within record, map preparation costs2.3486ms. The earlier
stationary capture understated rebuild work while camera visibility changes.

Added an explicitly selected raw-input diagnostic `--live-controls-demo
offboard-street-look`: press Y, then hold right stick sideways, reverse later.
No game-state overrides or physics changes. On the same authored street spawn,
2400-tick baseline (`frame-profile-turn-whole-20260929.csv`) reproduced65.11FPS
in one full-second window. A512-object command-cache experiment failed to give
consistent gains and was REMOVED (`frame-profile-turn-ranges-20260929.csv`).
Both64/512 command-cache experiments are absent from the resulting code.

Next measured candidate: shared immutable GPU mesh buffers. The existing
GpuResourceBuilder upload path now appends unchanged20-byte position/UV records,
unchanged32-bit indices and unchanged36-byte lighting records into bounded
pages. Adjust GPU draw startIndex by the index allocation offset and signed
baseVertex by the vertex allocation offset. Every original island, bound,
material, draw order, triangle and FP shader operation is unchanged. No shader
on this path reads builtin vertex_index. IDA RenderShadowMesh82824440 rechecked:
the per-island base/start/count semantics are exactly what the relocation
preserves. This is native-PC resource allocation, not reconstructed game tuning.

One allocator, shared by existing builder clones/workers behind a short upload
mutex. Published ranges are never overwritten; full pages leave the allocator
and last mesh/cache/command references release them. At most one unfinished page
is retained by the allocator. Page allocation units65536 vertices/262144 indices
are host choices (~4.5MiB with lighting); larger meshes use the existing dedicated
buffer path. Streams load once, not repacked/copied per map generation. Unloading
can release storage at page granularity when its last mesh retires. No original
asset decoder, culling decision, collision, simulation timing or draw is removed.

GPU tests34passed,1ignored (`artifacts/test-mesh-arena-20260929.log`): actual byte
readback verifies append preservation, lazy lighting aligned with position slots,
oversized fallback and retired-page lifetime. Existing effect/shadow adapter test
now draws a nonzero allocation using an original negative base vertex. Release
and controlled baseline/candidate runtime comparisons pending. Set optional
SKATE2_SEPARATE_MESH_BUFFERS to retain old storage for an otherwise matched run.

IMPORTANT first arena runtime exposed a character integration error: Character
create_character_object reused upload_mesh admission then replaced only the
vertex buffer with its own skinned stream, retaining page-relative draw bases.
The missing skater seen in playtesting was real. Corrected with a dedicated storage
option inside the SAME upload/admission routine; only character replacement
chooses it. Searched other vertex-buffer replacement assignments: this is the
only relevant consumer. Added regression for dedicated local signed draw bases
after prior arena uploads. Final34GPU tests pass,1ignored. Do not cite the first
arena run's360FPS as a validated final result: its skater was missing. Rebuilt
visible-character replay pending. Old-storage confirmation median123.87FPS
(`frame-profile-turn-separate-confirm-20260929.csv`,windows5..39s).

Corrected release PASSED (`artifacts/build-mesh-arena-final-20260929.log`),
followed by2400 camera-turn ticks without errors. Final screenshot
`artifacts/turn-arena-final-20260929.png` visibly includes skater, board and shadow.
Compare mean windows5s<elapsed<39s in `frame-profile-turn-*-20260929.csv`:

| Metric | separate-confirm | arena-final (character corrected) |
|---|---:|---:|
| Median FPS |123.87|182.34|
| Lowest full-second window FPS |61.15|135.75|
| Median main-map prepare ms |1.195|0.460|
| Median recording ms |3.783|1.771|
| Median submit/present ms |2.104|1.015|

Both use the same authored street spawn/raw input and1280x720, uncapped, no GPU
readbacks. Traffic scheduling and redraw cadence vary, so these are controlled
practical runs, not bit-identical frame workloads or universal FPS promises.
Both separate storage runs reproduce the60s; corrected arena stays above135
full-second FPS in the compared windows. No claim that isolated frame hitches
are eliminated. Scoped rustfmt passes; full-core tests retain unrelated baseline
compile failures documented earlier. Additional1800-tick movement check passed:
`frame-profile-walk-arena-final-20260929.csv`, displacement~78m from886.306,-391.913
to827.987,-338.919. Observed map generation5985->5947 objects; final screenshot
`walk-arena-final-20260929.png` includes visible skater/board and intact city.
This short path is not a comprehensive long-city unload/reload test. No game or
compiler remains active. Release executable is ready for a hands-on comparison.

## Follow-up: the 109 FPS capture

Latest playtest capture `artifacts/frame-profile-13823-21750.csv` (18:35:54) confirms
remaining rendering cost. At40.114s:108.63FPS, tick2.272ms/redraw,
record4.8258ms, scene3.2977ms, world-shadow1.0730ms, submit/present1.9584ms.
Position877.251,48.134,-379.374;3827 map islands,2842 world-shadow islands.
The capture moves through these windows; do not call this a stationary benchmark.

Earlier controlled captures used default Fifo presentation: the~165FPS ceiling
can be the monitor refresh limit. `play-performance.bat` now requests uncapped
presentation with SKATE2_NO_VSYNC, preserving the fixed1/60 simulation. Normal
play launchers are unchanged. Uncapped whole-bundle baseline:
`artifacts/frame-profile-agent-street-uncapped-20260929.csv`,20..59s median
152.98FPS, tick1.369ms, record3.414ms, scene2.247ms, world-shadow0.804ms.

Tested finer64-object command bundles, then REMOVED that experiment after it
failed to improve the controlled street capture. CPU-only chunks capture median
134.87FPS, scene2.5665ms; no reason to retain this added cache complexity versus
the152.98FPS whole-bundle baseline. GPU-instrumented chunks capture112.05FPS;
instrumentation also perturbs performance, so do not compare its FPS directly.
Captures: `frame-profile-agent-street-chunks-{cpu,gpu}-20260929.csv`.

The GPU-instrumented capture gives median main-scene0.9325ms, world-shadow0.1981ms,
dynamic-shadow0.0953ms. This supports a CPU-side bottleneck, not GPU fill cost.
Optional hardware timestamps now require BOTH SKATE2_FRAME_PROFILE and
SKATE2_GPU_PROFILE, and sample one in16 frames to reduce perturbation. Four
bounded readback slots, Maintain::Poll only; busy slots skip measurements.
Completed measurements aggregate separately into window means with explicit
sample count, NOT attached to the current CPU worst frame. Zero samples yields
blank timing fields. Normal play and play-performance.bat avoid GPU readbacks.

Added one bounded world-shadow command bundle per immutable map generation.
Exact ordered caster selection, camera binding identity, pipeline identity and
depth format are the cache key. Light rows still upload every frame; depth and
stencil still clear and redraw, with the same coverage reference and resolve.
Rechecked IDA RenderShadowMesh82824440: selected-island order and each indexed
draw's base/start/count are unchanged. This is host command reuse, not skipped
shadow rendering or a claimed original command-buffer implementation.

33 renderer tests pass,1ignored (`artifacts/test-gpu-world-commands-20260929.log`).
The actual-adapter test checks shadow cache reuse across row changes, invalidation
on reordered islands, and submits the depth/stencil bundle. Optional CSV now
separately reports main-map command preparation within scene CPU time. Release
rebuild PASSED (`artifacts/build-world-commands-20260929.log`). A3600-tick street
run completed without errors; screenshot `artifacts/street-shadow-cache-20260929.png`
shows the skater/shadows and upright full-size traffic. In mean windows20..59s,
`artifacts/frame-profile-agent-street-shadow-cache-20260929.csv` medians:
164.99FPS, tick1.3053ms/redraw, record2.8938ms, scene1.9982ms,
world-shadow0.4454ms, submit/present1.7068ms, main-map prepare0.1202ms.
This is~8%FPS improvement versus the uncapped whole-bundle baseline; shadow CPU
falls~45%. Main draw/submission CPU remains substantial. Not a500FPS claim and
not the exact larger-window/viewpoint measurement. Follow up there.

Sampled GPU readback validation also completed1080 ticks without errors:
`artifacts/frame-profile-agent-gpu-sampled-validation-20260929.csv`. Populated
GPU timings and~13 samples/second confirm asynchronous1/16 sampling. This short
run is a profiler validation, not a replacement for the matched60-second FPS
comparison. Scoped rustfmt checks pass. No game/compiler remains running.

## Subsequent measured fixes (September29)

The release street runs now cover 5400 completed ticks. Compare mean windows
40s < elapsed < 89s (49 windows per capture, excluding startup/shutdown):

| Median metric | street-fixed (trajectory fix) | street-cache (plus topology cache) |
|---|---:|---:|
| Render FPS | 53.12 | 155.72 |
| Tick CPU ms/redraw | 12.318 | 1.272 |
| Solver preparation ms/redraw | 3.469 | 0.050 |
| Whole solver/collision stage ms/redraw | 7.077 | 0.499 |
| Command recording ms/redraw | 5.103 | 3.781 |

Files: artifacts/frame-profile-agent-street-{fixed,cache}-20260929.csv.
These are completed matched-location runs, not a universal FPS claim. Mean
timings are per redraw (including any catch-up ticks), not per simulation tick.

### Repeated body-pool validation

`SimulationRigidBodyPool::validate_integrity` previously allocated a capacity-
sized bitmap and traversed all four rings on almost every ordinary body read.
This is safe-host structural validation, not original physics arithmetic.
Retain a successful validation in OnceLock until `links_mut` invalidates it.
All production Add/Remove/Activate mutations use this gateway; state spy-bit
set/clear preserves the validated allowed-state predicate. Pool authority,
slot IDs and capacity are immutable. Per-access handle/occurrence validation
still runs. Numeric fields, iterations, order and timestep are unchanged.
Test-only direct corruptions now explicitly invalidate the structural cache.

The standalone actual public-pool probe (artifacts/body-pool-read-probe-20260929.rs)
measured 100000 reads in a1024-slot pool:356.420ms before,0.872ms after. Both
executables passed removal, full-capacity reuse and stale-observation checks.
The live street comparison above establishes the practical frame benefit.

### Second vehicle error: serialized byte versus shader lane

The trajectory fix made vehicles full-sized, exposing a chassis rotating with
its wheel. Read actual recipes through the SAME mounted character decoder:
fourdoor_sedan_01 body mesh has palette[1,2,3,4,7]. First four entries remap to
wheels; entry4 remaps to Chassis. Its vertex groups are [0,0,0,0] through
[4,0,0,0]; group4 has1936 vertices spanning the body bounds. Glass uses its own
one-entry Chassis palette. Original inverse bind/chassis pose are identity.

The original vehicle UPDB source reads `In.BoneIndices.w`, but this is AFTER
the D3D declaration swizzle. IDA D3D::PatchVertexShaderToMatchVertexDeclaration
82c43ec4..3f04 (raw PPC checked) maps Type001a2286 to packed swizzle0530,
lanes(3,2,1,0). Thus shader W selects SERIALIZED BYTE0. Our format decoder
deliberately retains the raw bytes, while the vehicle uploader wrongly chose
byte3, selecting wheel0 for the whole body. Added one shared vehicle_bone_index
helper used by upload AND palette-range validation. All body/glass/light and
shadow draws use that same uploaded index. No corrective transform or pose
substitution. Regression evaluates the original swizzle expression.
Rebuilt runtime validation of this SECOND fix remains pending.

Additional actual-asset regression: the existing mounted scene and production
VehicleAnimation code passed Chassis independence from wheel spin/turn for all
17 distinct VehiclesDB names with matching mounted recipes (18 records include
a duplicate). It compares every Chassis mesh-palette row at spin0/turn0 versus
spin1.2/turn0.3. See artifacts/vehicle-mesh-probe-20260929.{rs,txt}. This covers
asset setup/palettes; the shader-index correction still needs the rebuilt live
visual check.

### Remaining rendering work

The cached-physics run still spends median2.817ms/redraw recording the main
scene. Added a bounded one-entry per-map render-bundle cache keyed by the EXACT
ordered object/island selection, camera binding identity, format and samples.
Uniform contents update every frame. New map generations own new caches;
retirement drops the old bundle and its retained resources. Commands keep the
same ranges, base vertices, instance IDs, pipelines and ordering. This is host
GPU submission optimization, not a native culling/quality change. GPU adapter
regression and runtime measurement pending. First adapter test exposed an old
expected count10 although its setup now supplies11 draws (three diagnostics,
seven effects, sky); corrected that expectation after counting the setup.

`cargo test --release -p skate2-pc -j1 gpu_renderer:: -- --nocapture` PASSED:
33 passed,1 ignored,67 filtered. Includes actual GPU adapter draw validation,
exact selection cache reuse/invalidation and the native declaration swizzle
regression. Log artifacts/test-gpu-bundles-vehicle-20260929.log.

## Completed rebuilt runtime check

Release build PASSED (existing warnings), followed by5400 completed street ticks
without runtime errors. `artifacts/vehicle-probe-street-bundle-20260929.png`
visually shows upright, full-size cars at the intersection and further down the
road. The earlier vertical chassis is gone. The optional probe still publishes
actual live-car positions; no collision removal or forced opacity was used.

Same49 mean windows (40..89s) in
`artifacts/frame-profile-agent-street-bundle-20260929.csv`:
median164.77FPS, tick1.184ms/redraw, record2.864ms, main-scene1.821ms,
world-shadow0.663ms, submit/present1.202ms, acquire0.714ms. Compared with the
physics-cache-only run this cuts main recording by~1ms, but total FPS improves
only~6%; rendering/driver/GPU waits still matter. No claim of500FPS everywhere.
These are1280x720 controlled captures; the exact larger-window viewpoint
and hands-on collision alignment still require the next test.

A controller run launched with normal Boneyard spawn and explicit optional
CSV `artifacts/frame-profile-user-vehicle-fixed-20260929.csv`, driving back to the
slow street. Escape saves the capture. Goal
remains active pending those findings and any necessary follow-up.

## Confirmed vehicle animation setup mismatch

Cars collided but appeared invisible. Optional live CSV probes establish
that cars reach the renderer with palettes at their simulation positions; the
street capture reports first-bone axis lengths of 0.01. Index admission checks
passed for encountered meshes. A small palette is not by itself proof of a bug;
the following original call-chain check establishes the missed setup:

- `InitVehicleAnimSys` 82810cc4 calls `AnimCommandSystemManager::NewACS`.
- `NewACS` 824d632c..634c builds the ACS Skeleton. Raw PPC at **824d6340 is
  `li r5,0`**, supplying `trajNode=0` to `Skeleton` 82a2fb98. This is unconditional,
  independent of the FetchSys trajectory flag or a bone named `trajectory`.
- `PoseEngine::PoseWithMatMult` 82a2aef0/af14 copies `mTrajNode` to the job.
- `Anim_Tools::PoseWithMatMult` 82a23944..54 checks each parent against -1 and
  that trajectory node; either match copies local as global instead of inheriting
  the parent's transform.
- Vehicle `InitVehicleAnimDataMap` 82810ab0 separately sets
  FetchSys::mHasTrajectoryBone=false. This does NOT change the ACS Skeleton node.

Our vehicle constructor used `AnimationSkeleton::from_andale` without supplying
the actual ACS trajectory node. Hence Vehicle_Root's authored 0.01 scale
propagated into Chassis and the wheel descendants. Other existing presentation
paths already explicitly set node0 (`actor_animation_matrix_program.rs`,
`actor_animation_bind_pose.rs`, `actor_animation_body_dispatch.rs`).

Fix: set the vehicle skeleton's trajectory node to Some(0), using the SAME
existing hierarchy evaluator and palette path. Preserve the RAW root scale and
all original bind data. No scale compensation, forced opacity or collision
suppression. Root still retains its scale; its children are evaluated as roots.

Standalone compilation of the actual `animation.rs` test module against the
built skate2-formats rlib passed all10 tests, including a new root-scale/chassis/
wheel regression. This avoids unrelated pre-existing core test-target errors;
it is not a full workspace test pass. Release rebuild and visual verification
of the fix are pending at this checkpoint.

## Reproduction and performance evidence

`SKATE2_DIAGNOSTIC_SPAWN_LOCATION=m_2c_03_goto_find_pool_locator` selects an
original BAM_Sim.loc transform at (886.305847,48.137413,-391.913422), about15m
from the captured slow spot. It uses the SAME selected bootstrap,
collision collection and presentation seed. Default remains F_SH_Boneyard.
This is a host diagnostic spawn choice, not a replacement native SpawnManager.

Completed bounded2400-tick street run:
`artifacts/frame-profile-agent-street-20260929.csv` and
`artifacts/vehicle-probe-street-20260929.png`. Stationary street windows reach
65.78 FPS: tick8.99ms/redraw, solver4.27–5.19ms, pre-solve1.15–1.40ms,
post-solve0.82–0.98ms; command recording about4.6–5.2ms. Vehicle-ground timing is
near zero in this run (cars are distant). Last partial26.3FPS window includes
final image capture/shutdown and must not be represented as sustained gameplay.
The earlier playtest capture is slower still and remains relevant.

Extended the SAME in-memory profiler with collision/solver substage timers for
the next run. No solver iterations, timestep, precision, collisions, actor
behavior or draw quality changed. The performance root cause is NOT yet fixed.
