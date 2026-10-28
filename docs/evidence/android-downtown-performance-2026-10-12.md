# Android downtown performance audit - 2026-10-12

## Status

Build19 compiled successfully in 653.54 seconds with the normal Android profile
and one compiler job. APK v2/v3 signing and 16 KiB alignment checks passed. It is
installed on the test phone with `adb install -r` (Success), preserving saves and
settings, and the launcher opened the game. The launcher still selects 540p.
Six production files are updated identically in the Android and PC source trees;
the new PC executable has not been compiled by this audit. No tests were added
or run. A build19 downtown capture is pending.

In playtesting, build18 made a large improvement when AI skaters appeared,
but downtown with NPCs and traffic still has large transient frame drops. The
prior PC sources compiled after a restart; the
previous PC compiler OOM is no longer the active task.

Work is solo. Changes target allocation, immutable-data ownership and GPU
resource lifetime. No tick is discarded, and no NPC count, solver iteration,
contact cap, collision result, animation sampling rate or physics arithmetic is
reduced. This is source/reference review, not a deterministic native trace proof.

## Captured evidence

Directory in the Android tree: `.scratch/performance-audit-20261012/`.

- `build18-downtown01-cpu.data`: 45-second simpleperf recording, 99 Hz CPU cycles,
  DWARF call stacks, process 15717. 10,921 samples, 167,539,218,733 sampled cycles.
- `build18-downtown01.perfetto-trace`: 120.029 seconds of scheduler/frequency data.
- `build18-downtown01-flushed.csv`: fresh frame timings flushed when the game closed;
  516 rows, elapsed time ends at 263.233 seconds. This was compared against
  the earlier file to confirm that it was newly flushed.
- `build18-previous-session.csv` is OLD and must not be used as this run's data.
- `build18-downtown01.png`: active downtown skating, 111 FPS at the screenshot
  instant, NPCs and touch controls visible. One image is not a sustained result.
- `build18-downtown01-state.txt`: thread IDs, uptime, thermal service and logcat.
- `build18-downtown01-{flat,sim,render,animation,audio}.txt`: symbol reports.
- `build18-downtown01-analysis.{sql,txt}`: scheduling queries and results.
- `ida-downtown-reference.json`, `ida-pedestrian-lod-reference.json`: IDA output.

Device: an Android 14 phone. 540p selected; actual Vulkan
surface is 1147x540. Earlier runs used other resolutions, including 2293x1080,
so their FPS is not a controlled comparison. Worker CPU-placement probe is OFF.

### Frame behavior

Approximate arithmetic means of the one-second report windows during world
play (not frame-weighted averages):

| Frame elapsed window | FPS | SIM ticks/s | Mean/max simulation debt | Mean render recording | Mean command finish |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100-150 s | 48.67 | 59.96 | 153 / 530 ms | 10.82 ms | 5.57 ms |
| 150-200 s | 42.21 | 59.74 | 128 / 356 ms | 12.16 ms | 6.42 ms |
| 200-250 s | 44.28 | 59.94 | 776 / 1186 ms | 11.30 ms | 6.20 ms |

Individual worst frames reach about 90-105 ms and often complete four ticks.
For example, at 102.937 s: 105.329 ms redraw, 54.431 ms total tick work,
23.235 ms render recording and 11.823 ms command finish. At 232.483 s:
98.603 ms redraw, 55.469 ms tick work, four ticks, 786.973 ms debt.
These timings overlap in places; do not add all columns or treat multi-tick
frame totals as per-tick costs. Simulation is much closer to 60 ticks/s than in
earlier failing runs, but transient accumulated debt and long frames remain.
GPU scene means in those windows are about 8-8.5 ms. A late downtown sample
contains 1867 visible map objects and 1917 map draw calls.

### CPU and scheduling

Whole-process sampled-cycle inclusive shares (parents overlap children):
run_tick 30.23%, render_frame 22.90%, command encoder finish 16.49%,
wgpu encode_render_pass 15.14%, audio render_stereo 11.86%, animation worker
loop 7.12%, VBR SQT sampler 4.49%, MeshCluster::parse 0.93%.

SIM-only report: contact-buffer construction 1.50% self / 1.58% inclusive;
PackedFastString<5>::encode 0.94% self / 1.92% inclusive; AndaleError destruction
0.98% self. Allocator/free/memcpy costs also remain. Animation workers now show
no sampled AndaleError destructor cost; sampling itself is 38.24% self / 50.60%
inclusive of the animation-worker samples. The earlier build18 removal was
therefore exercised in this run, although this does not isolate an FPS gain.

Render-thread report: texture-init action registration 7.74% self / 11.79%
inclusive, BindGroup::try_raw 6.09% self, texture usage merge 4.77% self,
texture-init check 4.00% self. Driver draw-indexed is 7.73% inclusive.
The expensive render finish is mostly real command/resource processing; it is
not explained by a permanent simulation loop or a new frame limiter.

Across the 120-second trace, SIM ran 79.57 CPU-seconds and render 61.70,
virtually all on faster cores near 2.2 GHz. Each animation worker ran about
15 seconds, about 94% on efficiency cores near 1.46 GHz, and spent about
20.8 seconds runnable but waiting. Render workers also mostly used efficiency
cores. Audio ran 26.40 seconds; Java main plus RenderThread ran about 28 seconds.
No affinity or clock changes were made.

Trace caveat: Perfetto reports 374 `trace_sorter_negative_timestamp_dropped`
events and the wakeup-dependency query was empty. Aggregate scheduling durations
are available; do not claim exact worker-to-parent wake relationships from this
trace. A thermal-service status of zero alone does not prove no throttling.

## Implemented changes and native comparison

### Contact-buffer capacity: initialize accepted contacts only

File: `crates/skate2-core/src/physics/solver/contacts/simulation_contact_append.rs`.
Native: `EA::Physics::GlobalBuffer<rw::physics::Contact,EA::Physics::ExternalMemory>::Proxy::Store`
at 0x827cb5c8, accepted-prefix memcpy at 0x827cb668.

Previously each producer reserved its maximum contact capacity, then wrote
`None` throughout that capacity before any collision was admitted. The private
storage now reserves a dense `Vec<T>` without initializing unused elements and
extends only the accepted prefix. T remains Copy; capacity is established
before the producer authority exists, so Store does not allocate.

The reservation plan is unchanged: reserve before copying, signed remaining
capacity, positive-partial overshoot, and nonpositive clamp are preserved.
Temp-buffer membership, capacity, poison checks and flush ordering are unchanged.
The returned record vector still reserves exactly the used size and retains its
existing allocation failure path; large maximum scratch capacity does not escape
into retained frame results. Contact values, iteration and solver order are
unchanged. Confidence: high; native Store and all modified source operations
reviewed. No deterministic solver trace comparison was run.

### FastString success path: avoid temporary error destruction

File: `crates/skate2-formats/src/andale.rs`.
Native: `FastString<6>::Assign` at 0x824b1a78.

`PackedFastString::encode` now constructs the overflow and invalid-character
errors lazily. The hot per-character success path previously eagerly created an
AndaleError with nontrivial enum drop glue. The case/digit mapping, six characters
per packed word, base-38 weights and wrapping accumulation remain unchanged.
This follows the same compiler-overhead correction as build18's VBR sampler.
Confidence: high for source equivalence. The old linked FastString<5> encoder
contained 27 static AndaleError destructor call sites. Both linked build19
FastString<5>/<6> encoders now contain zero. The <5> function is 1864 bytes versus
2904 previously. Full disassemblies and a manifest are saved; this is compiler
output evidence, not an isolated runtime performance measurement.

### Pedestrians: retain previously encountered LOD GPU objects

File: `apps/skate2-pc/src/pedestrian_render.rs`.
Native: base `PresentationEntity::SwitchLOD` 0x827efcf0, full-hand/face
`SwitchLOD` 0x82806528 and `cPresPedestrian::InitSkinning` 0x828063b8.

The original retains model instances in `m_vLODMdlInstances[currentLOD]` and
rebinds the selected LOD, then remaps bones. Our renderer used to retain only
the immediately selected LOD. Crossing a boundary and returning therefore
recreated its mesh buffers, object constants and material bindings.

The host now retains inactive LOD objects for still-live entity identities
(entity ID, recipe ID and palette Arc identity), removes them when the entity
leaves, and reuses the matching LOD/part/mesh object on return. Selection,
remapping, opacity, material updates, every pose upload, draw order, hidden
flags and shadow admission still execute in the same places. Cached inactive
objects are not submitted. This retains more GPU memory per live pedestrian
when several LODs have been encountered; the cache is bounded by its loaded
entity/LOD mesh set. Confidence: high for resource lifetime equivalence;
stall improvement has not yet been measured.

### Pedestrians: share immutable bone-name snapshots

Files: `crates/skate2-core/src/animation/face/actor_face_playback.rs` and
`crates/skate2-core/src/session/living_world/selected_living_world_pedestrian_presentation.rs`.
Native: the same InitSkinning retains `AnimTypeData->mHir` for RemapBones;
`UpdateBoneTransforms` 0x827efb78 copies pose transforms, not hierarchy names.

Every pedestrian visual snapshot previously cloned the hierarchy's complete
bone-name vector and each string. FaceDatabase now lazily owns an Arc of the
immutable CharacterHierarchy. Registration and removal invalidate it alongside
the existing hierarchy cache. Snapshots still validate and lock the same
resident database, so unloaded/missing database errors are preserved. Matrices
and pose publication are still fresh per tick. Confidence: high for ownership
and invalidation; no update cadence was removed.

### Character palettes: reuse packed-row capacity

File: `crates/skate2-core/src/rendering/character_palette_upload.rs`.
Native: `PackAndMultiplyMatricesForUpload` 0x828221e0 retains its destination
matrix storage; X multiply, Y/Z/W fused additions and transpose were checked.

The two packing entry points now clear and refill each part's existing row
vector. Every matrix is recalculated with the same function and iteration order.
Generation and publication gates are unchanged, and LOD changes still rebuild
the skinning/remap state. An existing private fixture helper delegates to the
same implementation under cfg(test); no tests were added or executed.
Confidence: high for arithmetic/order preservation.

## Remaining candidates, not claims of completed fixes

- Wgpu render-pass encoding and texture/resource tracking are a substantial
  measured cost. The existing map bundles already retain unchanged ordered
  chunks and suppress repeated bindings within a chunk. About 1900 map draws
  are present in one downtown frame; optimizing this must preserve original
  culling, visible-island order, materials, shadows and multipass ordering.
- MeshCluster parsing uses a one-cluster memo per assembly/mesh producer. It
  decodes vertices/normals and copies unit bytes again across producers. This
  is measured CPU work; broader immutable decode reuse needs ownership and
  native traversal/error-order review before a change. The separate line-query
  cursor already decodes selected vertices lazily and is not the same path.
- On a material binding cache miss, the host reads/parses the full skater
  Vault again. Pedestrian mesh creation requests one missing key at a time.
  This is source-confirmed repeated work but not a large sampled hotspot in
  this capture. Any caching must retain runtime Vault inheritance/lifetime.
- AttributeRuntimeInstance::resolved_node performs repeated class/collection
  tree lookups. It is sampled, but resolving parent/null/layout semantics is
  required; replacing it with a frozen value table would be unjustified.
- Audio decoder/mixer work remains substantial. No codec math/voice reduction
  was made. TouchControlsView invalidates on input; it does not run a perpetual
  onDraw invalidation loop. That particular UI-loop suspicion was ruled out.
- Native higher-level actor phase jobs remain a separate known scheduling
  difference, documented in the October11 audit. They cannot be parallelized
  safely by bypassing shared-world/RNG dependencies.

## Build and next comparison

`build19.log` records the normal `python tools/build_android.py` build, one job.
Original files and readable diffs are saved in both trees' scratch directories
(backups in both, diff files in Android). Existing unrelated changes were kept.
The build completed successfully; `build19.apk`, `build19-manifest.json`,
`build19-faststring-disassembly.txt` and `build19-install.log` preserve the result.
The library extracted from the APK matches the compiled library exactly.

- APK SHA256: `4270ecff774d82cc428d6880f8077d3adae6b4bc6aae519d9cf49ae63c947f1f`
- Library SHA256: `1975a984dfb50348533064137522a80e453a789c5e29dc0c39481e97b2e47d29`

Next: capture another busy downtown run at the same 1147x540 setting, distinguish
frame stalls from accumulating simulation debt, and inspect sustained NPC/traffic
behavior. Build19 is not yet performance-validated, and this is not proof that
all stalls are fixed. PC source is mirrored; no PC build is pending here.


## Build19 live result and worker-placement comparison

Build19 was captured in downtown after playtesting showed improvement with
remaining drops. Artifacts: `build19-downtown01-cpu.data` (45 seconds),
`build19-downtown01.perfetto-trace` (120 seconds), and the fresh
`build19-downtown01-flushed.csv` (792 rows, ends 401.838 seconds), under the
Android `.scratch/performance-audit-20261012` directory. Actual surface is
1147x540 throughout, matching the 540p setting.

Active windows at host elapsed 200-400 seconds average 58.92 FPS and 60.03
simulation ticks/sec. Mean per-window maximum debt is 104.34 ms, maximum
441.83 ms. Build18's earlier 100-250 second windows averaged 45.05 FPS and
59.88 ticks/sec, debt mean 352.79 ms/max 1186.13 ms. Routes and visible map
counts differ, so these are observations, not an isolated speedup estimate.
Individual build19 90-99 ms frames still commonly complete four simulation
ticks. Tick/core durations on such frames aggregate those ticks and overlap
other measurements; they are not single-tick costs to sum together.

Build19 CPU samples no longer show contact-buffer construction or AndaleError
drop above the 0.10% report threshold. FastString<5> is 0.54% of SIM samples
inclusive versus build18's 1.92%. MeshCluster::parse remains 3.15% of SIM
samples. Animation workers spend approximately 20 seconds each runnable but
waiting in the 120-second trace, with approximately 14.5 seconds each running;
91% of their execution is on the efficiency cores. The trace has 259 negative
timestamp drops. It does not establish exact wakeup dependency durations.

### Controlled scheduling experiment, unchanged build19 APK

The existing optional `worker-cpu-probe` was set to `animation` for 40 seconds
between two 40-second normal periods, then removed. All three worker setaffinity
calls succeeded with mask f8; removal restored ff successfully. SIM, window,
render workers, gameplay logic, clock rate, resolution, and thermal policy were
not changed. The first capture (`build19-workers01-*`) included travel through
different scenery, so its FPS difference is not a controlled scene comparison.
It nevertheless confirms the intended worker placement and reduced runnable
waiting. This first trace has 256 negative timestamp drops.

The skater was then left stationary for `build19-workers02-*`. Captured
start, middle, and end screenshots show the same view. CSV positions remain
x155.608-155.610, y-35.455, z429.726-429.777 across selected windows, with
3069-3078 mean visible map objects. Traffic remains live and changes naturally.
No controller inputs were injected. This 120.012-second scheduler trace has
no reported lost, overwritten, or dropped events. The experiment was restored
before the app was closed; the fresh CSV has 356 rows and ends
at host elapsed 180.903 seconds.

| Selected host windows | Normal before, 48-73 s | Faster animation workers, 88-113 s | Normal after, 128-153 s |
| --- | ---: | ---: | ---: |
| FPS, arithmetic mean of window rates | 48.66 | 86.68 | 53.09 |
| Simulation ticks/sec | 59.97 | 60.00 | 60.11 |
| Mean window p95 redraw, ms | 51.53 | 17.02 | 47.99 |
| Maximum simulation debt, ms | 132.28 | 46.51 | 126.95 |
| Simulation ready wait per rendered frame, ms | 6.22 | 0.24 | 3.65 |
| Mean ticks per rendered frame | 1.378 | 0.696 | 1.181 |
| Mean vehicle visuals | 6.96 | 13.20 | 13.83 |

CSV windows deliberately exclude both switches and their settling period;
its host Instant epoch is approximately the process start, not device boot.
Perfetto independently uses device BOOTTIME windows 370467-370497,
370507-370537, and 370547-370577 seconds. Each animation worker executes
99.6-99.8% on efficiency cores around 1.17 GHz normally, versus 0% on efficiency
cores around 2.18 GHz under the probe. Runnable wait falls from roughly
14.5-16.7% of wall time to 3.5%, and execution falls from roughly 13% to 6.8%.
SIM/window continue on faster cores. This is strong evidence of a significant
Android worker-scheduling bottleneck. It is not a promise of sustained 87 FPS
elsewhere: even the improved phase has a 78.74 ms worst frame.

### Build20 implementation, compiling

**Android only:** `apps/skate2-pc/src/android_cpu_probe.rs` now defaults to the
measured animation-worker placement while active, only when the allowed CPU
mask contains at least five CPUs above the minimum CPU capacity. Five covers
the current three animation workers plus SIM and window without restricting
them to a smaller set. This conservative host guard is not a native constant
or proof for other devices. Unknown/homogeneous/smaller topologies keep the OS
policy. No hardcoded CPU IDs, frequency forcing, or thermal override. The
existing diagnostic file still permits animation/render/all probes; any other
present value (for example `off`) requests the original masks. Focus loss,
suspend and shutdown use the existing restore path, and reentry records the
then-current original mask. Registration and restoration are serialized with
thread lifetime. Only the Android host file changes this policy; PC affinity
is untouched. AppMan MainLoop 0x82989ae0 and RenderLoop 0x82989fc0 were rechecked;
this is explicitly host scheduling, not claimed original-console affinity.

**Both source trees:** selected static ground collision clusters now retain
successful endian-decoded vertices/normals/raw unit bytes for the lifetime of
their StaticGroundSlot. Native AssemblyGroundCollisionJob 0x827cdb98,
BoxTest<AssemblyTriangleIntersectionTester,1> 0x827cd960 (resident cluster
lookup 0x827cdb64), and RipTriangles 0x827cd7c8 were checked live in IDA.
The native ripper accesses retained cluster bytes rather than rebuilding owned
vertex/normal/unit vectors for every object and tick.

The formats LeafClusterCache is bound by its owner to one immutable mesh.
StaticGroundSlot retains an Arc<SimulationStaticArenaPlan>; that plan exposes
only immutable asset references. No mutable bytes escape that owner. Cache
keys are serialized cluster offsets within this mesh, not global addresses or
reusable entity IDs. Slot removal (including native swap-with-last ordering)
drops its cache; a new slot starts empty. Cache locks cover only lookup/insertion,
never triangle callbacks or physics. Concurrent misses may parse identical
immutable bytes twice and publish one result. Poisoned cache locks fall back
to the original checked parse, and failed parses are not retained.

Only a first-visited successful cluster is retained. Zero-unit gates, cluster
index/table checks, capped KD range gathering before ripping, unit validation
and early exit, per-unit decompression/transform math, triangle admission,
twenty-triangle batches, final flush, contact reservation, solver iteration and
all tick cadence remain unchanged. The all-triangle background cache is not
substituted: it validates a broader unit set and would alter failure ordering.
The memory tradeoff is one decoded copy per visited cluster in each live static
slot, released with that slot; no global growing cache or predecode-all pass.
Confidence high for arithmetic/order/ownership equivalence by source and IDA
review; no deterministic reference gameplay trace comparison or tests were run.

Files: `collision.rs`, `static_ground_pool.rs`,
`simulation_assembly_ground_job.rs`, `simulation_assembly_ground_triangle_rip.rs`.
Four shared files match byte-for-byte in Android and PC. Original files and
readable diffs are saved as `*.before-worker-cache` and `*.worker-cache.diff`;
IDA excerpts in `ida-worker-cache-reference.json`. Build20 uses the normal
one-job `python tools/build_android.py`; build/install outcome follows below.
PC source only, no PC rebuild. No tests added/run.


Additional lifetime review during build20: live IDA GroundPool::AddVolume
0x827c84e0 publishes the resident mesh pointer through ClusteredMeshData::Reset;
RemoveVolume 0x827c8e60 synchronizes queries, selects the first matching volume,
and fills its hole from the last mesh. Those operations are unchanged by the
cache. Decompilations are in `ida-ground-lifetime.json`. The animation queue
was also reviewed for unbounded retained jobs: it has 512 slots, refuses reuse
of occupied slots, and removes completed selected submissions; this audit did
not identify an unbounded completed-job backlog there. Dependency extensions
are cleared by DormantAnimationJob reinitialization. No queue-policy edit made.


## Build20 build and installation result

Normal Android compilation and APK packaging completed successfully in 620.91
seconds; `build20.log` records the run. Signature v2/v3 verification and 16 KiB
zip alignment passed. The APK's native library is byte-identical to the newly
compiled library. `adb -s <device> install -r` returned Success and retained app
data. The launcher still shows the 540p setting. Game startup was
requested. Diagnostic worker-cpu-probe is absent, selecting automatic policy.

- APK SHA256: `95478c8d17ce061c19d28d8bafa8038ee210a0b732c37e104fe0ddb7a678071e`
- Native library SHA256: `f13c713cfaec5534392b8095ee56e2acea6068fd730315733aefca3fb66f287e`
- Artifacts: `build20.apk`, `build20-manifest.json`, `build20-install.log`.

The measured scheduling gain above used build19's temporary probe. The default
policy and collision cache are now compiled and installed as build20, but no
build20 populated-world performance capture has yet been made. In particular,
the collision-cache improvement has not been separately timed. PC receives the
four shared collision files only; no PC executable was built. No tests added
or executed, and no deterministic native physics trace comparison was run.


Build20 startup confirmed: fresh game process PID32098 automatically applied
policy4 to animation workers32248/32249/32250 with maskf8, result0 for all three,
without a diagnostic marker. `build20-boot.txt` records this and
`build20-boot.png` shows the stationary downtown world with HUD96FPS. A hands-on
run then showed 80-90FPS at that stationary downtown spot. This is
live startup confirmation, not a sustained moving-world timing capture or
an isolated measurement of the collision cache. No compiler/capture remains
running from this work; the game is open. All gameplay arithmetic/order review
limits above remain applicable.


## PC release build after build20

The PC build followed. The normal release command from
`build-release.bat`, `cargo build --release -p skate2-pc -j 1 --timings`,
completed with exit 0 at 2026-10-12 00:14:21 UTC (03:14:21 Helsinki).
Wrapper wall time was 797.056 seconds; Cargo reported 13m 16s. Existing warnings
were emitted; there were no compile errors. The newly written executable is
`target\release\skate2-pc.exe`, 42,428,928 bytes, AMD64 PE.
Its modification timestamp is later than both the previous executable and the
start of this build. SHA256:
`82958b5a8301e5ca48ad3d3d3aeee44c7102cf2394178ef6b06d3d4b64bd3727`.

This compiles the shared optimizations already mirrored into the PC tree,
including the visited-cluster decode cache. The preceding read-only comparison
found 2,588 Rust crate files identical; the two remaining crate differences are
Android ARM64 audio helpers. Android worker CPU placement remains platform-only.
No production source changes, tests, PC gameplay launch or PC performance
measurements were made as part of this build task. This supersedes the older
PC source-only/unbuilt statements in this document. Android build20 is unchanged.
PC build log and manifest: `.scratch/performance-audit-20261012/pc-build20-release.log`
and `pc-build20-release.json`. Launch with the existing `play-quiet.bat`.
