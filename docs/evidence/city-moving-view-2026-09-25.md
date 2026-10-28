# Moving city visual inventory

Implementation in progress; full native renderer/residency parity is NOT claimed.
Preceding evidence: city-rendering-streaming-2026-09-25.md.

## Source-backed movement and lookup

Pres_LocalPlayerStreamerController::UpdateStreamer8277d3d8 calls GetSkaterRoot
826c71f8, then MoveStreamFocus824d8250 with flags255. IDA type layout confirms
SkaterPresentation.mInterpolatedData at32, DataPostBlend.root at16 and
mHipsTransform at80. Therefore focus uses root translation, not the displayed
root*hips skinning transform. The existing selected presentation runtime now
publishes the same completed root separately; the public visual snapshot carries
it with the live focus-disable and enable-XML cvar values.

The app passes the SAME WorldStreamCatalog to the session and visual consumer.
`city_streaming.rs` borrows owning XSM/XSS/XML snapshots from its registered
Pres/Tex files; no second catalog/metadata parser was introduced. XML parsing and
focus selection reuse StreamXml/CollectionSpace, radius195/70 and the native
Pres/Tex16 capacity. Disabled movement and NaNs do not cancel accepted loading.

The existing move_runtime_presentation_scene/move_runtime_texture_scene routines
now delegate their mutation/load tails to shared selection helpers. The live
consumer passes authored selections to those SAME helpers on one background
worker. Existing CPU scene owners move to the worker, with no full-scene clone;
the prior GPU generation keeps rendering. The completed scene, texture binding
indices, render queue and map GPU objects are replaced together. Character
objects remain after the new map count. Distance-only changes do not reshuffle
index-based resource handles.

The existing proxy_shader_texture_table_draft implementation is now registered
as core shader_texture_table. Its source-reviewed full-ID/low-bit lookup is used
by the live map draw consumer. The existing GPU uploader body was moved into one
shared GpuTextureUploader service; immediate uploads delegate to it. There is
still only one texture decoding/upload implementation.

## Validation so far

Uploader/table release PASS4m15 (`city-texture-owner-release-20260925.log`).
Moving view cargo check PASS (`city-moving-view-check-20260925.log`).
Moving view release/replays pending in `city-moving-view-release-20260925.log`.
New offboard-city-walk diagnostic sends raw Y at32 and left-stick forward
160..3600; it does not teleport the skater or force any gameplay state.

Moving release PASS4m18. Walk/jump540 replay exits0 in8.992s and captures the
skater with board in hand. Live initial selection changes Pres6 ->11 cells,
2085 ->4418 draws. Log/capture: city-moving-walk-jump-20260925.*.
This verifies a complete map-generation replacement preserves character bindings.

The first long-walk3660 replay exits0 in60.990s but is NOT city-travel evidence:
the window lost focus before Y, and the actor stayed on-board braking. A focused
diagnostic confirmed WindowEvent::Focused(false) at tick10 followed immediately
by Action49/Motion1163, later1165. Log: city-walk-focus-90-20260925.log.
Earlier assumptions that this indicated walking/root failure are unsupported.
Explicit raw-input demos now supply focused replay input independently of OS
window activation; real controller mode retains actual window focus. One build
attempt found an ordinary Rust borrow conflict, corrected by sampling the focus
boolean before lending the pad. Rebuild/replay pending in
city-moving-replay-focus-build2-20260925.log. Do not count the failed-focus
long-walk captures as successful locomotion tests.

Focused-replay release PASS1m38. The corrected long-walk enters Action27,
Motion319, physical504 and moves from spawn to the nearby ramp. It then fails
after342 completed ticks with Presentation/Produce/Write/Compression/Numeric
IntegerConversion bits4290772992 (FFC00000, NaN). This is a real walking-route
failure, not a stream-loading success. Log: city-moving-focused-long-walk-20260925.log.

First ranged pose-trace release PASS4m16. Reproduces exactly342 completed ticks;
frames336..342 have no printed nonfinite parts. Log:
city-walking-ramp-pose-trace-20260925.log. The existing trace only flushed after
successful completion, so it discarded the failing343 trace. Diagnostic code now
flushes collected stage records on error and reports nonfinite local matrices,
hand SQTs and face SQTs before compression. Errors remain intact. Current build:
city-walking-ramp-nonfinite-build-20260925.log. Set SKATE2_TRACE_ADJUSTED_POSE=1,
SKATE2_TRACE_ADJUSTED_POSE_FROM=336 and SKATE2_TRACE_ADJUSTED_POSE_THROUGH=345;
run --live-skating --live-controls-demo offboard-city-walk --live-completions350.
Next action: isolate the first bad producer and compare its source flow in IDA.

## Remaining requirements

This connects moving visible inventories, not full native processed-arena
registration. Native PROXY low/global textures, material fallback streams,
first/last-use registration, unlock/world admission and native unload/GC still
need completion. Current inventory loaders retain their documented restrictions.
GPU map objects are rebuilt on membership changes; measure and address update
stalls before claiming smooth streaming. Most material lighting/samplers/effects
remain diagnostic. A live city movement/return route must verify the current
changes before the next packet or any completion claim.

## Walking blend failure isolated

September25 walking/city diagnostic update: release build
city-walking-hand-blend-build-20260925.log PASS1m22; replay
city-walking-hand-blend-trace-20260925.log reproduces tick343 failure.
All three hand/face NBlend streams emit list0/output0, weights[0,1,0,0],
inputs[fixed0,dynamic0,dynamic1,dynamic2]. Inputs are finite BEFORE blending.
The worker overwrites dynamic0 with first fixed input times zero, then reads
that overwritten dynamic0 for weight1, yielding zero scale/quaternion and NaN
on normalization. No behavioral workaround applied. Native NBlend82a26348,
raw266c0..267a4, Schedule82a32800, EnQueueCmd82a2b0d0, and CPU worker
82a2b528 currently appear to preserve that same hazardous alias ordering.
Investigate upstream token/command flow or actual native job selection before
claiming a source-verified correction; do not insert identity NaN fallbacks.
Diagnostic input logging is gated by SKATE2_TRACE_ADJUSTED_POSE. No build or
runtime process remains active. Moving city textures/material work remains
pending; goal ACTIVE, SOLO. See city-moving-view-2026-09-25.md.

The earlier nonfinite diagnostic build also passed (4m10). Its replay
city-walking-ramp-nonfinite-trace-20260925.log first isolated all17 left-hand
rotations; no nonfinite local body matrices were printed. The new NBlend
trace proves finite input poses are destroyed during accumulation. Existing
actor_animation_nblend_tests deliberately test later-alias visibility, so
changing this worker alone would contradict the currently reviewed CPU
reference. Check why native walking avoids this command stream; also review
which worker implementation the native job scheduler actually selects.

### Explicit host alias safety correction (validation in progress)

Fresh IDA checks cover NBlend82a26348 (including raw266c0..267a4),
ACSsqtBlend_job82a2b528 case2, SQTProcessEngine::EnQueueCmd82a2b0d0,
Schedule82a32800, Evaluate82a33018 and EndCommandSet82a330c8. No reorder
or input-preserving stage was found in this original CPU path. Do not describe
the alias fix as exact native instruction parity or an IDA-discovered missing
reorder. Native hand command generation still needs broader fidelity coverage.

The host worker now preserves ONLY the output buffer when a later source
aliases it. The native source selection, coefficient quantization, pass order,
fused accumulation, no hemisphere correction and final normalization remain.
Distinct buffers and safe first-input-only aliases retain existing execution.
This intentionally corrects destructive aliasing rather than substituting a
pose after NaN. Real input NaNs/quaternion cancellation remain observable.
The regression compares the failing [0,1,0,0] aliased case with the same blend
using a distinct destination. Existing alias regression now expects 5 rather
than the destructive native CPU result3.5. This difference is deliberate and
must remain disclosed if asked about exact fidelity.

### Alias correction validation

September25 walking/city verification: NBlend alias-safety release PASS1m46
(city-walking-nblend-safety-build-20260925.log), focused tests PASS9. Long raw
walk PASS3660ticks/61simseconds/60.989wallseconds (city-walking-nblend-safety-runtime-20260925.log).
Old tick343 NaN is gone. Skater moves about35m, then stops progressing against
an apparent obstacle near [624,-54.93,987]. No deep-city route claim. Moving
presentation generation verified at root[600.024,-54.074,995.878], draws4418 ->
3483, 11Pres/4Tex cells; textured1382/alpha144. Separate600tick capture PASS:
artifacts/city-walking-after-blend-20260925.png (visually reviewed).
IMPORTANT: preserving an aliased NBlend input is an explicitly documented
host storage-safety difference from the original CPU worker, not exact native
alias parity; weighted arithmetic retained. See evidence/city-moving-view-2026-09-25.md.
No process remains live. Next: connect existing PROXY processed texture/cache
owners to native low/high fallback and live city rendering; currently absent.
Work SOLO; goal ACTIVE; complete renderer/material/walking parity is unproved.


PROXY next integration inventory (read during build):
- Existing kind0 builder process_decoded_kind0 accepts already-decoded parent OR
  spatial single-Texture arenas; parent-only factory is intentionally narrower.
- Existing shared_loaded_arena draft wraps the original builder session factory
  and preserves full outer-ID physical use counts. Reuse it; do not create a
  second unique-asset cache.
- PresentationDmoArenaRegistrationAdapter is specifically kind2/kind11 and is
  not a general kind0/Environment dispatcher. ArenaRegistrationRefCounter is
  already generic and reusable for separate Presentation first/last-use gates.
- Live session still explicitly reports NativeProxyActivation pending. Its
  shared catalog currently has normal world metadata; register PROXY using
  existing StreamFile/MountedStreamBackend, not the census's manual parser.
- The old proxy_texture_resident_client is only an empty-draw-list diagnostic,
  not the production runtime to mount. Existing ShaderTextureTable is public.

## PROXY texture connection (September25)

Added app proxy_texture_world, promoting the existing kind0 processed builder
and shared physical loaded-arena cache, without another asset decoder. Uses
StreamFile/MountedStreamBackend and same WorldStreamCatalog for PROXY metadata;
stable signed spatial order verified from SyncLoadProxyWorldStream824df648 and
ProxyCollectionOrderCompare824d6888; existing parent planner precedes children.
Retains148 collections and145 unique texture products (shared outer IDs upload
once). Nontexture products remain decoded and are NOT claimed fully activated.
First-use counters coalesce texture publication. Inspection includes retained
PROXY pointers/identities in existing low/high shader table before high textures.
This is partial native integration: parent callbacks, kind2/kind10 activation,
full material bindings, incremental high registration and teardown parity are
still required, not implied by working texture inventory projection.

Check initially caught CollectionId lacking Ord and a shadowed material variable;
corrected. check2 PASS. Release city-proxy-textures-build-20260925.log PASS25.69s.
First runtime stopped at actual unsupported0x28280144 texture. Fresh IDA named
enum renderengine::PixelFormat identifies PIXELFORMAT_R5G6B5. Existing format
parser/decoder already supported it; uploader did not. Added checked original
2-byte texel validation and RGB565 -> nearest RGBA8 UNORM expansion (opaque A,
existing normalized endian). This is a host format conversion, max channel
rounding error0.5/255, not exact packed-format hardware sampling parity.

RGB565 release PASS26.58s. city-proxy-rgb565-runtime-20260925.log PASS600ticks,
9.9889wall seconds. Capture city-proxy-textures-20260925.png exists. Crucially,
diffuse draw counts are UNCHANGED (1189 initial,1360 first live selection,1382
moving selection). Thus these PROXY textures alone do not fix this route's
missing building textures. Investigate BAM_Pres embedded textures discarded by
own_presentation_arena or further required parent/global collections. Added
SKATE2_TRACE_CITY_TEXTURES optional counter at existing load_presentation_collection
before own_presentation_arena. Diagnostic release build is
city-pres-texture-census-build-20260925.log (session50652). No image improvement
or complete city-texture claim from this PROXY connection yet.

### Main missing-texture cause found

Diagnostic city-pres-texture-census-runtime-20260925.log PASS3ticks reports3267
single-texture arena occurrences in nearby BAM_Pres loads. load_presentation_collection
called own_presentation_arena, which retained geometry/materials/instances but
NO Texture objects. This explains why PROXY-only fallback did not increase the
diffuse count. Fresh RegArena827f1430 (decompiled via827f155c) confirms kind0
AddTexture at827f156c separately from material kind2 at1580: Presentation itself
is a texture supplier, not just BAM_Tex.

PresentationArena now retains decoded NativeTexture values by reusing existing
own_texture_arena on actual texture-bearing arenas; original resources are
still decoded only once per existing collection path. Character/test literals
receive empty texture vectors where appropriate. Inspection puts these retained
Pres textures into the existing table after PROXY and before BAM_Tex high
textures, preserving fullGUID/lowbit selection. They move/unload with their
same Presentation collection. This remains the current scene inventory path,
not a claim that all normal-world processed registration callbacks are complete.
check city-pres-low-textures-check-20260925.log PASS10.73s. Release build
city-pres-low-textures-build-20260925.log running, session62878.

RGB565 regression added in existing gpu_renderer tests: primary colors/endian
and exhaustive65536 packed values against max normalized rounding error0.5/255.
It and promoted shared-cache tests still need Cargo execution after current
release compiler finishes. Do not run concurrent compilers.

### Low texture runtime verification

September25 CITY TEXTURE FIX verified on moving600tick route: previously
BAM_Pres texture arenas were discarded by own_presentation_arena. Retained
native textures there and connected to existing low/high table before BAM_Tex.
Textured draws now2079/2085 initial,4403/4418 first live selection,3441/3483
moving selection (previous1189/1360/1382). Alpha-tested draws161/254/299.
Release PASS1m16; city-pres-low-textures-runtime-20260925.log PASS600ticks,
9.994wall seconds; capture city-pres-low-textures-20260925.png reviewed.
PROXY also connected using existing processed/shared owners:148collections,
145unique GPU textures, RGB565 upload support added from IDA enum confirmation.
PROXY alone did NOT improve this route; normal Pres texture retention did.
Full native activation/material lighting remain unproved. Current process:
cargo test -p skate2-pc --bin skate2-pc proxy_shared_loaded_arena, session22909,
log city-proxy-shared-cache-tests-20260925.log. Poll it; do not spawn another
compiler. Then run gpu_renderer RGB565 regression. Next: diagnose remaining
15/42 missing diffuse draws by original material, extend raw-input city route
past spawn obstacle and verify load/unload/foliage there. Goal ACTIVE, SOLO.
See docs/evidence/city-moving-view-2026-09-25.md; NBlend host alias deviation
remains explicitly documented and is not claimed native CPU instruction parity.


### Focused checks completed

September25 latest validation: shared processed-cache tests PASS5, RGB565
upload regression PASS1 (all65536 values). No compiler/runtime remains active.
Latest release includes PROXY145textures and retained BAM_Pres low textures;
600tick moving replay PASS, plus60tick spawn capture PASS
(artifacts/city-textures-spawn-20260925.png, visually reviewed). Large missing
texture gap corrected (4403/4418 first live draws now textured). Still investigate
remaining15/42 diffuse cases, deep-city movement/return and material effects.
Full lighting/shadows/PROXY activation are not complete. Goal ACTIVE, SOLO.

Logs: city-proxy-shared-cache-tests-20260925.log (5 PASS), city-rgb565-tests-20260925.log (1 PASS).


### Parent dependency correction (September25, validation pending)

Read-only `tools/inspect_city_collections.rs` reuses production MountSet and
CollectionMap parsers. BAM_Pres has285 collections:280 refer to parent
`e87238efd7f02b45` (`cPres_Global`,35 assets); five have the native invalid
parent sentinel. BAM_Tex has338 collections, all with invalid parents.
Fresh IDA LoadCollection82d57f58 (queried82d57fdc) confirms ancestor-first
recursive loads; UnloadCollection82d58018 confirms child-first unload requests.
The visual inventory loader omitted these dependencies. It now reuses
CollectionParentManager::plan_load, retains each ancestor once, preserves it
across focus changes, and exposes parent material/mesh/texture inventories
through the same render queue and low/high shader table. Spatial selections
and capacities still contain only the selected cells. This is visual ownership,
not a claim that the full native asynchronous registration callbacks are wired.

Prior missing diffuse GUID census: baseenvironment000053d203e3870a,
baseterrain0000481403e3870a, baseincandescent000000c603e3870a,
treeanimate0000731c03e3870a. New diagnostic messages retain manifest and GUID.
The reverse walking demo `offboard-city-exit` passed1800ticks in29.996wall
seconds but barely moved fromspawn; it is NOT a deep-city streaming test.
Hands-on city travel is planned for the next validated build.
