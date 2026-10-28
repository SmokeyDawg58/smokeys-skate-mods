# Android worker scheduling, animation allocations and GPU uploads

Scope: fix the sustained gameplay drops and simulation slowdown, while
preserving physics. Continued solo from `android-hectic-performance-2026-10-11.md`.

## Measured cause and scope

The preceding 30-second trace places native render and SIM on fast cores, but
six dependent animation/render workers spend 97.7-98.7% of their running time
on efficiency cores. Each animation worker spends another approximately four
seconds runnable and waiting. The live ADPF session contains only render/SIM.
The final gameplay section averages 52.25 simulation updates/second and retains
5.34 seconds of debt; an earlier interval presents 14.9 frames/second while
batching 3.2 updates/frame. These are distinct measured problems; high peak FPS
does not establish adequate sustained simulation throughput.

## Implemented

1. A platform-neutral worker lifetime hook in `skate2-core::host_thread`.
   Animation workers register before entering the existing job loop and leave
   before thread exit, including unwinding. The guard cannot move between
   threads. No installed hook means no OS effect (PC).
2. The Android boundary installs that hook before workers start. SIM and Rayon
   render workers register as well. Java records the actual calling OS TIDs;
   the render thread creates a hint session covering itself and every live
   registered worker. Membership changes close/recreate the session, retaining
   API 31 support. Thread registration and session creation share one monitor,
   so an exiting worker cannot disappear during creation with its TID.
   Pause/focus loss closes hints; live membership survives resume. No per-frame
   thread enumeration, thread-array allocation or per-job JNI calls were added.
3. Clip fetching/replay retains the already immutable parsed database Arc and
   borrows its hierarchy. It previously deeply cloned every hierarchy vector
   for every clip fetch. Existing owner validation, lock ordering, errors,
   time packing/decoding, part traversal and numerical expressions remain.

The shared core changes are mirrored in this repository. Android-specific
session registration remains in the Android experiment. No physics dt, solver,
collision, animation sampling, job queue order or logical job affinity changed.
No CPU affinity/frequency forcing or thermal setting changed.

## Source evidence

- Live IDA `Sk8::Job::cJobManager::Init` **0x826e3490**: three LOCAL/ANY workers,
  processor hints 1,3,5, queue capacities 512/128, calling thread may run deferred
  jobs. All retained by the implementation; these are not host CPU indices.
- Live IDA `Andale::FetchSys::BatchFetch` **0x82d737d0** reads retained `mSkel`
  at **0x82d7386c**, **0x82d739b4**, **0x82d73ad0** and uses its parts/bones
  directly. No per-fetch hierarchy copy. The port's parsed database is already
  an `Arc<AndaleDatabase>` and is read-only after registration; registration
  mutations use `Arc::make_mut` and content replacement retains old snapshots.
- Android [PerformanceHintManager](https://developer.android.com/reference/android/os/PerformanceHintManager)
  groups related threads in a session. Its existing API 31 creation/close/report
  interface suffices; no API 34-only `setThreads` dependency is introduced.

Confidence: high for removing the redundant immutable copy without changing
values; host scheduling is an Android adaptation, not a recovered console
algorithm. Performance improvement must be measured on the device.

## Build15: compiled, installed, still falls behind

Normal Android build passed in 577.1 seconds; signed/aligned and installed.
APK SHA256: `e5d56c34161ccd4fcb0e6ca6fb484875c8a88906c2c505640c371c31bed08cc5`.
The live service accepted all eight TIDs: render, SIM, three animation workers
and three render workers. JNI lifetime notifications rename workers to Java
`Thread-N` names; their registered native TIDs remain correct.

The initial short run dropped to 17 FPS. After resume,
the final approximately 60 seconds of the fresh flushed CSV average 50.91 FPS
and 57.64 SIM updates/second. Debt grows from about 3.33 to 5.67 seconds.
The active 60-second trace still places animation workers 85.4-85.8% on small
cores, approximately 1667 MHz runtime-weighted. Each has about 8.1 seconds
running and 10.8 seconds runnable but awaiting CPU. This trace reports 366
dropped negative timestamps, so prefer the clean build16 trace below for the
latest placement evidence. `build15-sched.perfetto-trace` is BACKGROUND/IDLE;
the active capture is `build15-sched-active.perfetto-trace`.

The 25-second CPU sample still shows SIM hierarchy copies (2.33% inclusive),
clip GUID lookup by BTreeMap iteration (2.13% self), and significant renderer
buffer allocation/command encoding. These are sampled CPU costs, not elapsed
frame timings and not additive when inclusive call stacks overlap.

## Build16: reusable GPU uploads

Shared PC/Android renderer changes:

- `FrameUploads` retains a wgpu StagingBelt and one copy encoder. Palette,
  character material, exposure and fog writes preserve byte contents, offsets,
  destination buffers and submission order. A copy command buffer precedes all
  shadow and scene draw command buffers in the same queue submission.
- `finish_and_recall_on_submit` reclaims staging asynchronously after GPU use;
  no extra GPU wait. An unsuccessful surface acquisition/encoding leaves writes
  available for retry. The 64 KiB chunk is a host allocation policy.
- Fixed-size material/fog encoding now uses stack arrays with the same f32 bits.
  CPU cloth geometry and general object uniforms initially keep Queue writes.
- Native `cScope::DispatchParamsHelper` **0x82e14aa0**, constant dispatch at
  **0x82e14b64**, rechecked live. This preserves the port's constant values and
  ordering; wgpu staging is a host renderer adaptation.
- API semantics checked in the installed wgpu30.0.1 source and its
  [StagingBelt documentation](https://docs.rs/wgpu/30.0.1/wgpu/util/struct.StagingBelt.html).

Normal build passed in 172.6 seconds; signing/alignment passed; update installed.
APK SHA256: `0b95ca550266927833ba71d51c5c6d2a8eb79ecdd38b33dadb316769729dd53c`.
The captured skater, clothing, board, world and touch overlay render. No GPU
validation/runtime failure was observed in this run. This is not exhaustive
visual parity validation.

The run returned to the world and left the skater AFK. The native buffer stays
**2293x1080**. The 120.026-second scheduler trace has no inspected lost,
overwritten or dropped event counters. Runtime-weighted placement:

| Thread | CPU time | Small-core share | Frequency |
| --- | ---: | ---: | ---: |
| SIM 10124 | 80.25 s | 0.00% | 2125 MHz |
| Render 10069 | 68.96 s | 0.65% | 2130 MHz |
| Animation 10268/10269/10270 | 16.39-16.51 s each | 98.66-98.77% | 1193-1195 MHz |
| Render workers 10140/10141/10144 | 9.66-11.24 s each | 99.50-99.76% | 1172-1180 MHz |

Each animation worker also spends about 20.1 seconds runnable and waiting.
Whole-device scheduler occupancy over the same trace is52.4-52.9% on CPUs0-2,
32.1-41.8% on CPUs3-6 and9.1% on CPU7. Thus the observed worker placement is not
explained by every performance core being continuously occupied. These average
loads do not establish simultaneous per-frame availability; the switchable
placement comparison is still required.
ADPF has all eight TIDs, session allowed, target11,111,111 ns. A separate live
foreground read confirms all inspected threads in `/top-app`, affinity0-7,
priority110. They are eligible for fast cores; ordinary placement still uses
small cores for the bursty workers. Current Thermal HAL reports status0 and
all inspected cooling-device levels0; cached 89.9 readings are not current
HAL readings and must not be cited as evidence of this run's throttling.

Fresh CSV `build16-profile-flushed.csv`:342 mean windows, ends347.026 s.
The final approximately120 s average49.61 FPS and53.83 SIM updates/second;
debt13.38 ->25.68 s. Seven sub30-FPS windows average22.68 FPS and53.18 updates/s,
with2.355 updates/draw. Catch-up changes presentation frequency but the core
still fails to sustain60 updates/s. The upload change is **not a complete fix**.
CPU samples still show renderer command-encoder finish52.07% inclusive,
Queue::write_buffer10.15%, SIM hierarchy clone2.03%, GUID lookup2.04% self.
Different workloads/clock placement prevent a controlled speedup claim from
comparing these percentages with build15.

**Capture pitfall:** immediately pulling after KEYCODE_HOME returned the old
build15 file byte-for-byte (`build16-profile-active.csv`,235426 bytes). Wait
for the asynchronous background/focus handling to finish, then pull again.
`build16-profile-flushed.csv` (508485 bytes) is the actual build16 run.
The game was resumed via the launcher intent after capturing. The phone then
disconnected from ADB; build16 remains its last installed APK.

## Build17: additional shared changes and placement diagnostic

Source changes are mirrored in both trees:

1. FaceDatabase retains an immutable hierarchy Arc, created once at first
   command-stream use after part-name mapping. The cache is invalidated on
   content registration/removal; old queued streams retain their snapshot.
   Hand/face, matrix, SIM pedestrian and PRES pedestrian streams clone the Arc
   instead of the hierarchy vectors/strings, including marquee/regular streams.
   Consumers still perform the same SQT, matrix, operand and extraction work.
   Existing fixture constructions are adapted to the Arc type; no tests added.
2. GUID clip lookup uses a prebuilt sorted ordinal vector. The vector is derived
   from the same surviving BTreeMap entries (including duplicate replacement)
   before assigning the unchanged GUIDs. Removal clears it; missing-owner and
   ordinal errors retain their order. The existing ordinal cap is unchanged.
3. Character object uniforms use the same reusable upload stream. Their only
   update entrypoint is GpuRenderer::update_object; the dynamic-object writer
   uses a different object type/allocation. Character ownership is assigned at
   construction and never cleared. No mixed later Queue writes to these buffers.

Additional live IDA evidence:

- `DataBaseManager::GetClipHeader` **0x82d6e878**, direct `mClips.mpBegin[guid>>5]`
  at **0x82d6e8e8**.
- `DataBase::SetDBContent` **0x82d7f7e8**, surviving sorted clip iteration and GUID
  assignment **0x82d7f93c..990**; `UpdateDatabase` **0x82d76628** for part mapping.
- `Actor::SetUpBlendPoseStream` **0x825f8398** uses retained playback ACS/fetch
  owners and emits ordered commands; there is no per-stream hierarchy copy.
- `PoseEngine::PoseWithMatMult` **0x82a2add0** publishes the existing parent
  vector pointer **0x82a2aef4..af24**, with original bone count and trajectory.
- Pedestrian `EvalBlendPose` **0x828078f0** rechecked; frame selection/token
  decoding/command order are unchanged.

Android-only `android_cpu_probe.rs` is an **opt-in diagnostic**, inactive by
default. External `worker-cpu-probe` accepts `animation`, `render`, or `all`.
Once per second the window thread checks that file; selected live workers get
an affinity subset derived from allowed CPUs with capacity above the minimum.
It neither pins a numbered CPU nor changes clock/thermal settings. Unknown or
homogeneous topology leaves affinity unchanged. SIM/window are excluded from
the experiment. Original worker masks are restored on focus loss or disabling
the marker. A registration mutex prevents TID reuse during affinity operations.
The two worker pools originate on unmodified SIM/window threads. Native job
affinities, dependencies, worker counts, dt, arithmetic and input remain intact.
NDK28.2 Bionic sched.h signatures/layout checked locally.

Build17 first compile found three additional compact pedestrian constructors
requiring the hierarchy ownership type; they were updated. Retry is compiling.
No new tests or unit suites were run. No build17 runtime result yet. PC source
has all shared changes, but its last executable is the earlier release with
SHA256 `055672682b948be2f1f3721f4031aa4ce9c5f3e73e0773982a239a5b6ad3ec6e`.
The placement experiment must be measured before enabling a default policy.

Confidence: high for preserving values/order in the immutable data/index and
upload changes; no deterministic native per-tick trace comparison was run.
The sustained performance issue remains open.

Backups/build log: `.scratch/android-worker-performance-20261011/` in each tree
(Android APK backup in the Android tree only).

## AI skater / animation audit: build18

Work moved to a source-level bottleneck that grows with extra
skaters. The three animation-worker TIDs10268/10269/10270 in the saved build16
25-second CPU capture yield1,042 samples (6,212,258,076 sampled cycle events).
Aggregated report: `build16-cpu-animation-flat.txt`. These shares are for the
animation workers, not the whole process, and inclusive rows overlap:

| Function | Inclusive share | Self share |
| --- | ---: | ---: |
| execute_fetch | 77.02% | 5.31% |
| sample_vbr_sqt_pair | 62.67% | 1.61% |
| sample_vbr_sqt_cached | 59.28% | 24.17% |
| AndaleError drop glue | 14.88% | 14.88% |
| allocator free | 9.35% | 1.64% |
| allocator malloc entry | 8.10% | 1.03% |

The current main tick runs the same actor animation pipeline for local, party
and AI skaters. AI decision work itself is0.82% inclusive in the SIM CPU sample;
AISkaterManager costs around0.014ms per displayed frame in representative
stationary windows. This points to the added actors' animation/physics workload
rather than proving the AI path planner itself is the dominant CPU cost.
No before/after actor-count capture was available in this audit.

### Demonstrated decoder waste

1. `Option::ok_or(AndaleError::...)` constructs and drops a drop-bearing error
   enum even when a checked read succeeds. The installed optimized ARM64
   sampler contains15 static calls to AndaleError drop glue; three execute per
   bone per channel and up to three per present compressed coefficient.
   They materialize the enum on the stack, call its destructor and reload state.
   This is visible in `build17-vbr-before-range.asm` (sampler0x1c2c190..1c2cfa0),
   not just a source-level guess. The earlier `build17-vbr-before.asm` contains
   only a missing-symbol warning/header and is not usable evidence.
2. Each runtime block decode copied quantization selectors, significance bits
   and coefficient bytes into three owned vectors, despite immutable data
   already retained by the clip. The public inspection API still returns owned
   buffers; runtime now borrows the same checked slices. Ordinary and overlapping
   zero/256 significance streams retain their original offsets and validation.
3. Each sampled frame allocated a channel-layout vector and up to three channel
   output vectors. Layout validation now compares the iterator, collecting an
   identical error payload only on failure. A256-float local scratch replaces
   channel allocations. Its bound follows the packed part header's1..64 bones
   (Vector_ExtractPackedNVBR0x82d71608) and validated3/4/3 component layout. Channel counts are checked against that
   bone count before slicing scratch. Only the written prefix is consumed.

For an interpolated pair in one block with all three channels animated, the
sampler previously made14 allocations:4 block/coefficients plus5 per frame.
It now makes3: one coefficient vector and two returned SQT vectors. Counts
exclude outer fetch/output/blend allocations and may be lower for empty channels.
There is no persistent decompressed-pose cache or skipped sampling.

`cargo rustc --profile android --target aarch64-linux-android -p skate2-formats
--lib -j1 -- --emit=asm` PASS28.88s. The new sampler has0 AndaleError drop calls
in generated ARM64 (`build18-vbr-format-only.s`). No tests were added or run.
The full APK compilation subsequently passed (see below). This code-generation
inspection is not a measured runtime speedup or a native numerical trace comparison.

### Clip payload lookup

FaceDatabase now builds a record-to-payload vector once during content
registration. It retains the first payload for each record offset, matching
previous iter().find semantics, independently of duplicate-name map winners.
Registration/removal clears the vector. Part/GUID writes never reorder payloads.
Clip initialization, single/all attribute queries, fetch planning and worker
fetch execution use the index. A worker takes the immutable parsed Arc and its
index under the same database lock, then decodes outside that lock as before.
Missing-content/record/payload errors retain the existing variants and order.

### Live IDA evidence and parity boundary

- Vector_ExtractPackedNVBR0x82d71268: InitPerAnim0x82d7156c, one/two-frame
  dispatch0x82d718f8/0x82d71638, original trajectory branch0x82d719b0 onward.
- VBRDecompressor::Vector_UnPackFrameBlockAndDecompressOneFrame0x82edbd18
  reads retained compressed data; DecompressFrameBlock0x82edb790 uses scratch,
  selected IDCT row0x82edb838 and per-channel weights0x82edba88/90.
- DecompressTwoConsecFrames0x82edc8d8 uses per-thread work buffers and shared
  coefficients. Final constant/animated selection0x82edd2e8..3dc visits channels
  and bones in source order, with both output frames from the same input.
- Clip::GetAttribute0x82d77c98 (header/count/attributes0x82d77ccc..cd0) and
  GetAttributes0x82d80808 directly dereference retained headers. The port's
  whole-database scans were representation overhead.
- DataBaseManager::GetClipHeader0x82d6e878, indexed header0x82d6e8e8/8f8.

Confidence: exact for the observed host destructor calls and eliminated
allocations/scans; high for preserving immutable inputs, validation and existing
FP graph. Samples, quantized blend coefficient, channel/bone iteration, negative
zero, IDCT grouping and FMA graph remain unchanged. Existing Xenon reduction/
estimate numerical limitations remain. No physics/timestep/actor-count changes.

The seven edited production files and one adapted existing fixture are byte-identical
between Android and PC. The zero-count fixture calls now pass a borrowed block
and the existing per-call cache argument; no cases were added or executed. During
comparison, PC was found to lack the earlier Android VBR table caching,
selected-row reconstruction, paired block reuse and bulk bit reads. Those
previously documented, source-backed changes are now included in PC as well.
The PC executable has not been rebuilt in this audit.

### Further architecture finding (not changed in build18)

SkaterManager.UpdateAnimation1/20x825ffc70/0x825fa310 dispatch higher-level
Actor phase jobs. StartAnimUpdatePhase10x825ff988 establishes Phase2 depends on
Phase1 and Phase3 depends onPhase2 (0x825ffc10/20). Initializers0x82f56ae8 and
0x82f564d0 default both kUseJobsForSkaterAnimUpdate and kRunAsJobTree to true;
network/cvar gates still apply. The port's selected tick runs those Actor phases
serially, while its ACS fetch/blend work already runs on workers. That is a
confirmed remaining scheduling boundary, not proof of its timing contribution
or permission to reorder shared world/RNG/graph writes. Preserve native
ownership/dependencies before changing it. The decoder fixes do not alter it.

### Build/install state correction

Build17 retry completed565.6s and installed successfully before this audit:
SHA256 daa1b582035c78ccaf40fd124c6b92f15cde95aeb833dcd06e572b26fb720d44.
APK, retry and install logs are saved. It has no fresh world capture. ADB again
reports no device during this audit; reconnect requested asynchronously.
The optional worker-cpu-probe remains absent/off. Build18 subsequently completed;
see final status below. Sustained performance remains unverified.

### Build18 final status

Full `python tools/build_android.py` completed successfully in605.55 seconds,
exit0. Native ARM64 compilation, javac/D8, v2/v3 signing and16KiB alignment
passed. APK: `android-apk/skate2-android-arm64.apk` (16,224,735 bytes), backed up
as `.scratch/android-worker-performance-20261011/build18.apk`.

- APK SHA256: `40ed6cb1acebd2804d771ab78f26afc40e6ac383fd57c46bee20ef48f5c79086`.
- Native library SHA256: `a386f8bb40270f1a0ec19b2ec145577145e367fb661cf3bb4bbc0f33672dde21`.
- Final linked sampler0x1c33c64..1c34a5c: **0 AndaleError drop calls**, versus15
  static sites in build17; all static call sites47 ->23. Saved disassembly:
  `build18-vbr-linked.asm`. This confirms the actual packaged binary fix.
- ADB still lists no devices. Build18 is **not installed or profiled**. Build17
  is the last confirmed installed version. Reconnect question remains pending.
- No compiler/capture process remains running from this audit. No tests added
  or run. Existing fixture call signatures adapted only. PC executable unchanged.
- Next measurement: install build18 preserving storage; capture a sustained busy
  Party Play turn with AI present. Compare simulation ticks/s, debt slope and
  decoder/wait cost, alongside FPS. Do not claim the complete15-FPS issue solved.


### Build18 installation

The phone was reconnected for installation. ADB reports the device
authorized. APK SHA256 rechecked against the build18 hash above.
`adb -s <device> install -r` completed with `Success`, preserving app storage.
Exported `org.skate2.android/.LauncherActivity` opened successfully.
Log: `.scratch/android-worker-performance-20261011/build18-install.log`.
No new compile, tests or gameplay capture in this installation step.
Sustained performance with AI skaters is still unmeasured for build18.
