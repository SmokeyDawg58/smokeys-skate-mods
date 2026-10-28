# Camera active avoidance prefix and query boundary (2026-09-23)

## Source and ownership

`Iris::Rig::UpdateCollisionAvoidance(float)` is
`?UpdateCollisionAvoidance@Rig@Iris@@AAAXM@Z`, `0x82e46490..82e46ca8`. Its only
code caller is `Iris::Rig::DoUpdate`, `0x82e47c40`, at `0x82e47d78`. The caller
decrements `mCollisionAvoidanceOffTime` first, then (when
`retryWhilstPositioning == false`) runs `UpdateSubjectState`, avoidance,
reference height, interpolated heading, and interpolated elevation in that
order. The distinct `shotEditor` argument does not select the instant-tracker
branch. The Rust gate was corrected in `camera_rig_dynamics.rs`.

The `Rig` layout places the three evaluator pointers at `+0x90`, `+0x94`,
`+0x98`, `mAvoidanceHeading`/`Elevation` at `+0x14c`/`+0x150`, amounts at
`+0x1b4`/`+0x1b8`, strategy at `+0x1c8`, type at `+0x1cc`, state at `+0x1d0`,
and flags at `+0x1d8`. Type values 1..4 select SubjectPath, Inwards, Upwards,
and OppositeHeading (`0x82e46528..82e4654c`).

## Recovered prefix

The routine returns without changing avoidance for type 0 or SubjectPath speed
below `0.5` (`0x82e464b4..82e464d8`). Override flag bit 1 or a positive
post-decrement off timer zeros both amounts (`0x82e464dc..82e464fc`,
`0x82e46c88..82e46c8c`). Otherwise it samples parameter
`mAvoidanceLookahead` at `+0x268`, then calculates the type's heading and
elevation. SubjectPath uses breadcrumb offset 1, i.e. `(last + count - 1) %
count` per `BreadcrumbManager::GetPosition82e3b2c0`, and the anchor tracker's
output position with Y replaced by `mReferenceHeight`. The delta is normalized;
a length no greater than the source epsilon `0x38d1b717` returns before any
angle publication (`0x82e46560..82e46618`). The resulting Euler Y has its
sign bit flipped for heading; Euler X is elevation (`0x82e4664c..82e4667c`).
Inwards uses the subject's last valid ground normal (`0x82e466d8..82e46738`).
Upwards uses the pre-avoidance heading and elevation `0x3fbde44e`. Opposite
Heading normalizes requested heading plus PI and selects the greater of
requested elevation and `0x3e32b8c2` under the native ordered comparison.

The first query boundary follows angle selection. Native computes the desired
camera position, then conditionally updates the desired evaluator when
grounded and `mUpdateCounter % 10 == 0`, or when live
`kbInterleaveAvoidance` is enabled. Tuck-in heading/elevation update at
`% 10 == 5` on ground, or under the same cvar. Desired radius is positioner
collision sphere radius plus `mPositionerAvoidanceCollisionOffset`; tuck-in
radii are `0.02`. Every evaluator retains its prior output between those
phases. `PathEvaluator::Update82e46420` resets collision time to `FLT_MAX`
and valid false, calls `UpdateStaticEvaluator82e3c880`, then
`UpdateDynamicEvaluator82e45f00`, then copies collision time to last-valid
only when valid. `PathEvaluatorOutput` at evaluator `+0x40` has collision time
`+0`, last-valid `+4`, valid `+8`, and was-in-air `+9`. The byte checks in
Rig at `0x82e46a28..82e46a58` read *was-in-air* (byte 1 of the loaded word),
not valid.

## Remaining integration contract

Later September 23 continuation: the World predictor now retains camera-owned
futures in its existing 90-slot queue, and selected SimUpdate2 harvests them
after camera update. `CameraPathEvaluator::try_update_prefix` submits and polls
its static query through that owner. The Rig desired-position branch is now
connected through its first evaluator input and update, using the live
interleave/debug CVars. Its next explicit boundary is the dynamic obstacle
server; the paragraph below records the earlier state at the time of this
prefix trace. See `camera-world-predictor-future-2026-09-23.md` and
`camera-obstacle-cvars-registration-2026-09-23.md`.

`Sk8::Camera::TrajectoryPredictor::StartQuery82e32ac0` submits
`SetupLookAroundPrediction82e4bbc0` to the World predictor with a camera-owned
future and zero gravity. The current Rust `PhysicalPredictionOwner::try_add_query`
only accepts `PhysicalPlayerAssemblyOwner` and player `PredictionTarget`
selectors, so it cannot submit into one of the three camera futures without a
shared predictor/future interface extension. Native dynamic evaluation asks
`PathEvaluatorObstacleServer::FindObstacles82e3a7a0`, which conditionally
enumerates pedestrians and skaters through live cvars; the current Rust
`CameraObstacles` does not yet own these catalogs. Live
`kbInterleaveAvoidance`, `kbShowAvoidanceDebug`, `kbEnableCollisionAvoidance`,
and `kInsignificantSpeed` also need registry links and source defaults.

`camera_rig_dynamics.rs` now stops with `PendingAvoidanceQueries` after the
verified direction prefix. This is an explicit boundary, not a fabricated
no-collision result. No claim of a completed camera frame or first playable
skating test follows from this patch.

## Validation and confidence

IDA decompilation, disassembly, type layout, and xrefs were compared with the
Rust call chain. The SubjectPath direction and retry gate have a focused Rust
regression. Branch/field/order confidence is **high**; exact VMX reciprocal
and Euler exceptional bits are **medium** until a native per-tick trace. The
full collision outcome remains unimplemented, so active avoidance has no parity
claim. `cargo test -p skate2-core
subject_path_uses_reference_height_and_previous_breadcrumb --lib` passed (1
test) after concurrent offboard compilation finished. The first two attempts
were blocked by incomplete concurrent offboard edits, not this camera patch.
