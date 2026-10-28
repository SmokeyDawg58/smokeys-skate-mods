# Live performance investigation

## Verified result (supersedes intermediate status below)

The KD padding correction is integrated and measured. The source formula is
maxAbs.mul_add(epsilon, fatness), not maxAbs*fatness+epsilon. Camera candidate
units fell from approximately 66,875 to 169 per tick, preserving the native rays.
In astra-kd-padding-after-20260924.stderr.log, median tick cost is 5.7064 ms
(95th percentile 6.3008), pre-solve 1.2551, camera 0.8643, view preparation
0.1589 and present 2.0258. Optional profiling was enabled for these measurements.

The fixed-step host retains elapsed-time debt and performs at most four due
1/60-second steps per redraw. A fresh post-bootstrap epoch prevents startup work
from becoming gameplay debt. With profiling off, 120 scripted completions took
2.012038 seconds, versus 119/60 seconds of scheduled intervals plus final work:
astra-fixed-epoch-wall-20260924.stderr.log. This supports normal simulation speed
for this replay; it is not a claim of complete gameplay or exact hardware parity.
The live build felt dramatically smoother in a hands-on run.

Formats: 214 passed, 1 ignored; transform reuse: 9 passed; uniform batching:
2 passed; scheduler: 2 passed. Subsequent gameplay core gate: 4917 passed,
42 ignored. Original-data facade batch: 4 passed (including four short flip
families). Grab/bail corpus: 5 passed. The extended crouch/push cases and new
offboard dispatch still require their next gate. Gameplay completion is again
the immediate work; historical intermediate investigation follows.

This was prioritized immediately after verified hands-on board turning.
The prior window exited at557 completed frames with Motion.PendingExpression260;
that missing condition is distinct from sustained slow simulation.

## Reproducible baseline

Optimized release, original external assets, no compilation running concurrently.
SKATE2_PROFILE_REDRAW=1, --live-skating --live-ollie-demo --live-completions120.
Log artifacts/astra-performance-ollie-before-20260924.stderr.log completes120.
Discard first25 observations per metric; median milliseconds:

| Stage | Median | 95th percentile |
|---|---:|---:|
| Actor | 0.6591 | 1.0456 |
| Pre-solve | 18.6695 | 19.0956 |
| Solver | 1.5461 | 1.6336 |
| Post-solve | 1.0962 | 1.1699 |
| Camera plus later predictor/breadcrumb | 11.9472 | 12.0676 |
| Entire tick | 36.3577 | 37.1623 |
| View preparation | 2.8425 | 3.0505 |
| Present | 4.6485 | 5.2030 |

Tick metrics95 observations; Present119 includes different redraw offsets.
Separate non-scripted baseline log astra-performance-baseline-20260924.stderr.log
has168 completed frames before GroundPhysicsConstraintsPending; median tick40.06.
Use scripted run for before/after comparison. One1/60 tick per redraw with overdue
slots discarded demonstrably slows game time when computation exceeds budget.

## Integrated changes awaiting performance measurement

- KD line query now borrows immutable serialized node bytes and decodes only
  visited nodes, instead of parsing/allocating the entire tree on every ray.
  Whole allocation bounds are still checked before geometric rejection. Node
  parser has no value-dependent errors; traversal/clip math/error checks unchanged.
  Independent review PASS;375 eager-versus-borrowed cases plus malformed tests.
- Static trajectory queries retain immutable all-cluster/all-unit decoding per
  private map slot. First initialization is at the same post-KD stage; identical
  decode errors are cached. Triangle expansion, ordering, tie handling and cap
  unchanged. Slot removal/re-add resets cache; generic eager API unchanged.
  Independent review PASS;three additional correctness/lifetime tests.
- GPU upload serializers use fixed-size byte arrays instead of temporary Vecs.
  Float byte order, payloads, padding, draw calls and shaders unchanged.
- Opt-in camera profiler separates camera manager, later predictor and breadcrumb.

Formats gate artifacts/astra-collision-perf-tests-20260924.log:212passed1ignored.
Core gate artifacts/astra-core-perf-tests-20260924.log:4890passed42ignored.
First optimized release completed120 Ollie frames, log
artifacts/astra-performance-ollie-after-20260924.stderr.log. Same first25
observation exclusion: pre-solve median4.618ms (p955.033), whole tick23.0471ms
(p9524.0373), camera manager11.4939ms, view2.7886ms, present4.681ms.
This is a substantial pre-solve improvement, but NOT normal-speed completion.
Camera manager remains the dominant measured cost; narrower optional camera
profiling and buffered outer telemetry are integrated for attribution.

Second attribution run artifacts/astra-collision-attribution-20260924.stderr.log
completes120 with opt-in per-unit timing. Final-frame camera drop visits32042
units for10 emitted triangles; positioner visits34833 for0 emitted triangles.
These timers add overhead and are not a final frame-budget measurement.
Map uniform batching reduces final-frame view preparation to0.1877ms; its two
byte/cardinality tests pass. Collision transform reuse differential suite9passes.

Root and both independent reviewers identified the major remaining source bug:
KD constructor827bd514 raw116d602e is standard VMX VA13*VC0+VB12, where13 is
max absolute endpoint,0 is epsilon358637bd and12 is fatness. Existing Rust
reversed epsilon/fatness roles, enormously expanding fat rays at large world
coordinates. Correct formula is maxAbs.mul_add(1e-6,fatness). Minimal correction
and regression packet in preparation; do not claim measured recovery yet.
Physics timestep, tuning, contact math, camera behavior and input graph unchanged.

This also includes an IDA audit of the previous day's pushing, camera and
breadcrumb changes. Pushing feel remains unverified at normal wall-clock speed;
audit actual callback/force/hold-release paths rather than attributing every
symptom to the earlier agent model. Performance remains the immediate priority.

Performance work supersedes new gameplay integration temporarily. Reviewed Landing,
state502 and walking packets remain available unapplied; see root continuation
checkpoint. Do not duplicate those agents' work or merge them into timing tests.
