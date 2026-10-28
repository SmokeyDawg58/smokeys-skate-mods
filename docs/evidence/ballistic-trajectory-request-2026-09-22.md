# Ballistic trajectory request integration draft, 2026-09-22

Status: review artifact, not registered or runtime accepted. The author has run
no Cargo commands. Root integration and independent review remain required.

Artifacts:

- `artifacts/ballistic-trajectory-request-2026-09-22.patch`: registered-file changes
  for root review/application only.
- `artifacts/build-ballistic-trajectory-request-2026-09-22.py`: reproducible patch
  construction with exact replacement assertions; reads registered files and
  formats proposed contents through rustfmt stdin, never edits registered files.
- `crates/skate2-core/src/physics/physics_trajectory_numeric.rs`: shared source numeric
  leaves, referenced by the proposed existing physics module.
- `crates/skate2-core/src/physics/collision/ballistic_trajectory_request_tests.rs`: twelve unregistered
  tests; patch adds its module declaration to the world trajectory child module.

## Same owner and lifecycle

`collision_query_batches::trajectory::BallisticTrajectoryInput` carries typed
`position`, `velocity`, `gravity` (`SkeletonRootVector`), plus f32 `duration`,
`radius`, `maximum_error_start`, `maximum_error_end`.

The existing `TrajectoryTest` storage becomes tagged `TrajectoryInput` with
Skeleton and Ballistic cases. The existing family pools, counts, capacity,
result storage, retained lease, incomplete-job marker, and state machine are
reused. No predictor, service, future, or world owner is introduced.

The world adds `try_acquire_trajectory(dmos,label)`; the old skeleton acquisition
delegates to it. It adds
`try_add_ballistic_trajectory<N>(&batch,input)->Result<bool,WorldTrajectoryError<N>>`.
The signatures of existing begin/end/sync/get/release are unchanged. End dispatches
the actual Ground provider or checks the actual constructor-owned DMO/Vehicle
collection membership before executing the walk against that verified empty
collection. It does not publish a completed future or make results readable:
batch state remains Running until later Sync.

The old low-level skeleton-only End remains available to existing tests/callers.
It explicitly fails if given a Ballistic entry and leaves the job incomplete.

## Source evidence and reuse

Fresh IDA inspections for this task:

- `ClusteredMeshTrajectoryBatch::AddTest827bdcc0`: validates XYZ of position,
  velocity and gravity before capacity; then initializes the native result and
  stores p0/v0/g/T/radius/errorStart/errorEnd. Unknown W is not rejected or assigned.
- `LocalClusteredMeshTrajectoryTest827c2270`: builds the existing walk functor
  with ground-normal calculation true (at `827c22e0`).
- `WalkTrajectory827c1e58`: negative gravity Y selects adaptive steps from the
  two authored error radii; otherwise stepStart/stepEnd are duration. Loop uses
  strict duration > time, advances segmentStart only after a submitted miss,
  and does not clamp the end sample to duration.
- `Get_dTWithErrorR827bf868`: source reciprocal estimate and two refinements,
  multiplied by -8*errorRadius, followed by reciprocal-square-root estimate and
  two refinements with zero selection. The revised patch executes that graph
  through the actual caller's fallible numeric service.
- `CalculateTimeOfCollision827bce08`: the linear fallback floors only the frame
  at `827bd064/68`; collision time uses the unrounded ratio times dt at
  `827bd078..84`. The patch fixes the legacy helper's prior floor-before-time bug.
- `TrajectoryWalkFunctor827c1c78`: ground-normal landing velocity is the fused
  g*(collisionFrame*dt)+v0 expression at `827c1d88..1da8`; patch restores this FMA.

The patch reuses `physics::predict_ballistic_trajectory`, the existing geometry
trait, `StaticGroundPool` trajectory sweep/triangle providers, and the existing
ground-normal kernel. It does not duplicate the ballistic walk.

After independent review blocked a first draft that bypassed the provided numeric
service, the existing walk was refactored into one shared numeric-aware algorithm.
The production world job calls `predict_ballistic_trajectory_with_numeric`, which
propagates separate Geometry/Numeric errors. The prior public XYZ entry point
delegates to that same algorithm using an explicit compatibility-only infallible
host adapter. Public timestep/collision-time helpers also delegate to the shared
leaves; no second walk or collision-time search is introduced.
Square-root reconstruction reuses the already present
`reckoning_numeric::length_from_squared_and_estimate` after the caller's estimate.
The reused linear collision-time helper is corrected to test distance before
attempting velocity dot/length, preserving the native early return at `827bcf54`.
The numeric-aware walk returns an optional hit separately from its time validity:
Local copies even a negative-time hit payload; only later GetResult filters it.

Independent raw audit additionally confirms `CalculateTimeOfCollision` loads
literal 2.0 then performs reciprocal estimate and two refinements before multiplying
gravity Y, before deciding the linear/nonlinear branch. Nonlinear search starts
minimum distance at bits `7f7fffff` (source `821043a4`). GE(minimum,distance) replaces
minimum; otherwise GT(distance,minimum) steps back and stops; unordered preserves
minimum and advances. There is no extra positive-distance gate. Final frame uses
refined reciprocal(dt), multiplication, ceil, and fallible integer conversion.
The new helper follows these operations and preserves failure before subsequent
geometry calls/stores. Step interpolation likewise invokes the caller's duration
reciprocal estimate and refinements after each submitted miss, lazily.

Unsupported legacy assertions on radius, positive/nonvertical gravity and sample
time are absent from the live route. The source gravity-down debug invariant is
retained only within Get_dT, selected by negative gravity Y.
Fresh `_Assert_GravityPointingDown827bd218` confirms exact X==0, Z==0 and
0>=Y tests; the helper retains those comparisons (Y<=0), while the walker
invokes it only for Y<0.

Three additional corrections are needed before that legacy walk can be reused:

1. Chord threshold is `((errorStart*radius)*errorStart)*radius`, not radius².
   Raw VMX128 VA decoding is required: `827c1f78` multiplies v51 (errorStart)
   and v52 (radius); the following product uses v50 and v51, then v40 and v52.
   IDA prints incorrect high VA register names in these instructions.
2. Step interpolation forms `(1-interpolation)*startStep` then
   `endStep.mul_add(interpolation, product)`; it is not the differently rounded
   `(endStep-startStep).mul_add(interpolation,startStep)`.
3. Linear collision time retains the fractional ratio as described above.

These are narrow amendments to the existing kernel, not new live tuning.

## Typed lanes and numerical limits

Every request retains optional W lanes in its actual batch entry until job
execution. The conversion to the existing XYZ geometry kernel happens only at
the provider leaf. Successful geometry exposes XYZ only, so result position,
normal and groundNormal have `w: None`. No-hit results retain the source
constructor's known lanes. Request W is never reconstructed from XYZ.

**This draft does not establish Xenon seed or reduction bit parity.** The shared
walk uses caller-owned `ReckoningNumeric` for dot reduction, reciprocal estimate,
reciprocal-square-root estimate, and integer conversion, preserving the source
refinement graph and fallibility. The established host numeric implementation still
supplies host estimates and separately rounded host reductions. The same existing
ground-normal geometry leaf is reused by both skeleton and ballistic routes;
its host normalization/reduction remains explicit preexisting debt, accepted by
the independent reviewer as a separately bounded geometry leaf pending root
acceptance. Skeleton requests retain their selected source path unchanged.

No queue capacity, batch capacity, seven-candidate count, async scheduling,
launch tuning, AI, scoring, or private Launch completion is invented here.

## Validation and root integration

Performed: rustfmt parse/format of the unregistered tests and all proposed
registered contents via stdin; `git apply --check` passes. No compilation or
runtime tests have been run by the author.

Twelve proposed tests distinguish ballistic gravity/duration from skeleton
g=0,T=1; preserve start-error cutoff (including zero-error short chords);
check fractional linear collision time and unknown hit W; retain mixed requests
with their original typed lanes; reject gravity NaN before a full-capacity return;
and prevent failed jobs or unsynchronized results from becoming successful misses.
Additional numeric tests stop on initial reciprocal failure, fail at interpolation
only after the first submitted segment, fail conversion before the ground-normal
geometry query, and distinguish actual source admission from legacy eager panics.
They also prove a coincident hit skips velocity numeric calls and a computed
negative-time hit retains position/normal/surface and unknown W until GetResult.

Root: inspect/apply the patch, run the twelve `ballistic_request_tests` plus existing
trajectory/skeleton tests and required gates, and route actual Predictor AddQuery
through this same batch. Future readiness belongs to the later actual predictor
harvest, after Sync/Get/Release. Independent reviewer should inspect both the
source corrections and the explicit numerical debt before live activation.

