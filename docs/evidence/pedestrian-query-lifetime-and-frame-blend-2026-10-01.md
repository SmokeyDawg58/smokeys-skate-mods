# Pedestrian query lifetime and animation interpolation

A playtest hit ground-query lost retained entity at tick2960 and visibly choppy
walking/avoidance. Two concrete source mismatches found; no tuning changed.

## Retained query targets after census detach

RequestCull8271a5e8 calls OnCullRequested, detaches from managers, and inserts
the same object pointer into mCullMap. Destruction is deferred. Existing Rust
already implements this lifetime with pedestrian_culls, but query receive
searched only EntityMan40. ReceiveGroundLineTestBatch82e72850 directly uses
mpNavRig (82e72918), not an active-roster lookup. Volume and Jump hooks likewise
retain the rig. Resolve these IDs against the live roster OR existing retained
culls. Do not recreate entities or skip missing targets. Positioning/taze keep
the original nullable active-ID lookup, so they do not acquire this fallback.
No destruction timing, simulation membership, or query-buffer indices changed.

## Explicit frame interpolation

SetUpSimLODAnimation82e6ea28 passes FETCHBLEND to both low-detail and full-skeleton
paths. Raw82e6ea80 loads r8=1; raw82e6eac8 loads r7=1. Previous Rust passed2,
which is USEDEFAULT, not a two-frame count. Both pedestrian ACS configurations
use fetch_mode0, so this resolved to noninterpolated sample pairs. The shared
BufferedFetchTiming::encode only sets blend_frames when resolved mode==1.
Changed both call sites to1. Existing SIM recording/PRES replay preserves this
flag and fractional timing; no duplicate interpolator or artificial animation
speed change. Normal simulation graph pooling remains as native BuildAnimTree
82e6e878: it can alternate graph decisions while AddTime advances every frame.

Opt-in SKATE2_TRACE_PEDESTRIANS reports sampled clip frame pairs every300 streams
for runtime verification. Normal play remains silent. Build/runtime pending.


Focused fetch regression PASS: artifacts/pedestrian-fetch-blend-regression.log.
The small rustc test harness imports the actual production timing module and
checks mode2/default0 versus mode1/default0 at a half animation frame: unchanged
decoded clocks, old pair[0,0], corrected pair[0,1] and fractional blend. This is
not a reimplemented decoder. Runtime trace will additionally check the actual
pedestrian call sites feeding SIM/PRES, rather than only the encoding helper.


First build PASS6m54s. Runtime confirmed actual clip fetches interpolated=true,
adjacent frame pairs and fractional weights. It then reproduced missing ground
owner at1164 despite the pending-cull lookup. Census.Update82702780 drains the
cull maps before the next Update2 query receive; RequestCull retention is only
part of the lifetime. Native Pedestrian destructor frees NavRig at82e94380.
Its raw pointer hookup cannot be represented by a live Rust entity after that
allocation ends. At that exact destructor boundary, retire only matching
rig-targeted ground/volume/jump delivery flags in both buffers. Keep all slots
and native query synchronization/indexing. Positioning/taze remain unchanged.
Unknown missing live owners still error; this is not blanket missing-ID skipping.

This is an explicit host lifetime safeguard for stale native raw-pointer
hookups, not evidence that the original executable performs the same cleanup.
No freed-storage writes or effects on a subsequently reused allocation are
claimed as deterministic game behavior. Live rig results still use the exact
retained owner; culling timing, pool capacity and spawn policy remain unchanged.
Second build/runtime pending.


Final release build PASS7m07s: artifacts/pedestrian-rig-retirement-build.log.
Runtime PASS4800 completed ticks, exit0: pedestrian-rig-retirement-run.log.
276 sampled live clip fetches report interpolated=true, zero false; they include
adjacent frame pairs and fractional weights. Population changes and close
approaches continued beyond the earlier failures. Screenshot captured at4800.
Two actual local Actor contacts were classified Ignore; no Light/Full response
was captured, so the broader physical-interaction goal remains unverified.
Final close-camera screenshot contains large translucent polygon surfaces; do
not treat that frame as proof of correct near-camera presentation or contact.
A hands-on comparison of walking/avoidance smoothness remains valuable.
