# OffBoardFeetIK ground update prefix

Confidence: exact for wrapper and first-two-call order, member offsets,
comparisons and cvar identity; high for host raw-transform composition.
This is a retained prefix only. No selected OffBoard frame invokes it yet.

`OffBoardFeetIK::UpdateGround` `0x82dd1e00` calls `InitUpdate`
`0x82dcd260` at `0x82dd1e14`, `ConditionLineTestResultsForGround`
`0x82dcd188` at `0x82dd1e1c`, `Update` `0x82dd1b70` at `0x82dd1e20`,
`CalculateGroundCorrectionAndWipeout` `0x82dcd328` at `0x82dd1e28`, and
`CalculateHeightAboveGround` `0x82dd0788` at `0x82dd1e30`; it stores height
at FeetIK `+0x110` at `0x82dd1e34`.

`InitUpdate` copies ProcessedPhysIn `+0x390` to left result `+0xa0`, and
Processed `+0x3c0` to right result `+0xd0`, each as position, normal, surface
ID and valid flag. It then queries `Skeleton::GetAnimPartTransformWorldRaw`
`0x82b45f58` for parts 15 and 19 in that order and stores the translation at
FeetIK `+0x30/+0x40`. Raw means `SkeletonState::GetLastAnimVolumeTransform`
`0x82b3d498` composed with retained animation-to-world. It does not dispatch
through `Skeleton::GetPartTransform` or the physics-history branch. The Rust
source is the already retained animation volume row and root transform.

The Processed foot surfaces are prior line-test results: native Update4 collects
them, then Update1 `FillPhysIn` copies them into Processed before Update3 state
logic. Rust `physical_player_skeleton_line_tests.rs` and
`skeleton_processed_line_surfaces.rs` already own this timing. `InitUpdate`
does not start a query or read the currently running batch.

`ConditionLineTestResultsForGround` captures both original valid flags. Each
valid result contributes `result.position.y - currentAnimationPosition.y`;
invalid results contribute zero. At `0x82dcd20c`, it reads the linked
`kOB_IK_MaxFeetYDist` target. Only if the absolute difference of those two
deltas is strictly greater than the target does it consider invalidations.
It clears left valid only when absolute left delta is strictly greater than
the target **and original right valid**; it clears right valid only when
absolute right delta is strictly greater than the reloaded target **and
original left valid**. Threshold ties leave results unchanged. No surface
position, normal or ID is modified.

The cvar initializer is `0x82f988a0`, pointer table `0x82fd5df0`, reference
`0x830db2c4`: section `Physics_Systems_OffboardIK`, label
`kOB_IK_MaxFeetYDist`, title `Max Y IK Dist`, default float `0x3e99999a`
(0.30000001), declared range 0..1, null callback. Startup overrides remain
live linked values, including out-of-range values per existing cvar behavior.

Host conversion explicitly refuses an unknown fourth surface or animated
position lane rather than fabricating one. The prefix uses the established
affine composition helper; exact Xenon exceptional arithmetic and a complete
OffBoard tick still need a reference trace. `Update` itself calls feet-down,
state, IK target and conditional IK application stages; ground correction and
height follow. The selected pre-solve therefore stays at the explicit pending
boundary. Focused tests cover the four source reads, strict ties, both-result
invalidations, absent opposite results, and linked cvar overrides.

Validation: `cargo test -p skate2-core --lib feet_ik_prefix --quiet` passed
4/4 tests after the concurrently edited test corpus compiled. The two new
prefix source/test files passed a direct `rustfmt --check`. Full workspace
validation and the selected runtime continuation remain the root integration
gate.

## Live frame-59 dispatch (2026-09-24)

The mounted raw-Y replay now completes GroundAnimation103→OffBoard500 entry and
reaches `PhysState_OffBoard::Update` at `0x82da03c8`. IDA recheck of
`OffBoardFeetIK::UpdateGround` `0x82dd1e00` confirms the first two calls are
`InitUpdate` `0x82dd1e14` and `ConditionLineTestResultsForGround` `0x82dd1e1c`;
`Update` `0x82dd1e20` is next. The live pair adapter receives the already
validated physical owner, retrieves the same retained Skeleton, checks the
feet-IK binding against that owner/processed/skeleton, and calls the two
existing leaves in source order. It then stops at typed `PendingUpdate`; no
feet-down state, IK target, correction, height, or later OffBoard update effect
is claimed. IDA `Update` `0x82dd1b70` first conditionally copies processed
`+0xf0` to `mLastGoodPosition` when the bad-correction counter is zero, then
calls `CalculateFeetDownState`, `UpdateState`, `CalculateIKTargets`, and
optionally `ApplyIK` under the live linked enable cvar. Those remain the next
source work. Combined build and replay are parent-owned; the raw-Y expectation
must change only after its next observed boundary.

The first mounted replay reached `InitUpdate` and stopped at an artificial
`UnknownSurfaceLane { left: true }` gate. IDA `InitUpdate` `0x82dcd260`
uses two full-vector loads/stores for each Processed surface (`0x82dcd294`
and `0x82dcd2bc`), followed by full translation-vector copies for raw left
and right toe transforms. It does not validate or scalar-read W. The prior
trajectory line-test result already carries `SkeletonRootVector` with optional
W, which may be unknown for native pointer/address lanes. `SurfaceResult`
position/normal and current raw toe positions now retain those same typed
vectors. `ConditionLineTestResultsForGround` reads Y only, so it continues
without promoting host scratch W. A focused test checks unknown-W propagation
and the strict Y-based invalidation. This change awaits the parent-owned
combined build/replay; the next expected boundary is still observational.

## First `Update` store (2026-09-24)

The mounted raw-Y replay after typed surface propagation reaches frame 59,
physical OffBoard500, and stops at `OffBoardFeetIk(PendingUpdate)` in
`artifacts/raw-y-feetik-typed-surface-2026-09-24.log`. IDA `Update`
`0x82dd1b70` checks `mBadGroundCorrectionCounter` at `0x82dd1b84` and,
when zero, copies the full vector at Processed `+0xf0` into FeetIK
`mLastGoodPosition +0x70` at `0x82dd1ba0`, then sets
`mLastGoodPositionValid` at `0x82dd1ba8`. IDA type layout identifies
Processed `+0xf0` as the translation column of `mEffectiveAnimTransform`
(`+0xc0`), distinct from `mGroundPos +0x1e0`. The existing Processed
ground-position owner already publishes that exact effective transform in
`Update1_ProcessInput` at `0x82e035b0..c8`. The Rust store uses its fourth
column, preserves optional W, and skips both read and store when the bad
correction counter is nonzero. Missing effective animation on the taken branch
is typed. The next native call is `CalculateFeetDownState` at `0x82dd1bb0`,
retained as `PendingCalculateFeetDownState`; no later state or IK effects are
claimed. Focused tests cover the conditional skip and full-vector copy with
unknown W. Combined build and mounted replay remain parent-owned.

## `ClampFootNormal` isolated helper (2026-09-24)

`CalculateFeetDownState` `0x82dcdc90` first calls `ClampFootNormal`
`0x82dcdae8` on the left result normal and stores its full vector at FeetIK
`+0x80`, then calls it on the right result normal and stores at `+0x90`.
The helper starts by copying the input normal, reads linked
`kOBIK_IKMaxYFootNormal` at `0x82dcdb10`, and returns that input unless
`maximum > normal.y`. The raw VMX128 operands shown as high registers in IDA
alias the loaded low registers: `v91` is `v59` (maximum), `v94` is `v62`
(normal), and `v93` is `v61` (the same normal with Y zeroed in stack scratch).
The taken branch measures the XYZ length of the Y-zeroed vector and returns
the original when that length is not ordered-positive. The remaining graph
uses `vrefp` with two Newton refinements for reciprocal horizontal length,
and `vrsqrtefp` with two refinements on `1 - secondMaximum²`. It scales the
Y-zeroed input by inverse length times the square root of that expression,
then overwrites only Y with a **third** linked maximum read. It does not
clamp the cvar or replace a negative square-root argument. The existing
Reckoning numeric/drive-refinement helpers provide the ordinary host policy;
Xenon estimate seeds, exceptional arithmetic and fused rounding are not
claimed bit-exact. IDA initializers `0x82f98f98` and `0x82f98ff8` provide
the two missing linked OffboardIK cvars `kOBIK_IKMaxYFootNormal` (default
`0x3f666666`) and `kYHeightToBeOffGround` (default `0x3db851ec`), now retained
at cvar indices 365 and 366. The clamp helper is isolated with focused
geometric/read-order tests; it is not yet live-wired, and the mounted
boundary remains `PendingCalculateFeetDownState`.

## Reached feet-down prefix (2026-09-24)

The independent IDA review approved the isolated ordinary-finite
`ClampFootNormal` graph and cvar definitions. The reached live adapter now
calls that same helper first for the left result normal, storing FeetIK
`mGroundNormalLeft +0x80`, then for the right result normal, storing
`mGroundNormalRight +0x90` (`0x82dcdcc4..dcfc`). Both fields retain optional W
because native stores each full vector. Next, `GetAnimPartTransformRaw`
`0x82b46040` delegates directly to `SkeletonState::GetLastAnimVolumeTransform`
`0x82b3d498`; only the local translation columns for parts 15, 19, 16 and
20 are used, in that order (`0x82dcdd04..dd60`). These are the existing
`SkeletonAnimationTargets` volume rows, **not** the world-composed positions
used in `InitUpdate`.

`CalculateFeetDownState` reads linked `kYHeightToBeOffGround` at
`0x82dcdd74` for left and `0x82dcddfc` for right. IDA raw VMX alias
decoding shows each result is `height > rawToe.y && height > rawFoot.y`,
with the second comparison short-circuited; strict ties are false. It stores
`mAnimLeftFootDown +0x12c` before the right height read, then
`mAnimRightFootDown +0x12d` at `0x82dcde6c`. The four local positions are
returned as temporary scratch for the following `GetPartTransform` calls.
The next typed boundary is the first such call for part 15 at
`0x82dcde7c`; no physical/history transform, surface validity, or later
foot-down override is claimed. Existing `SkeletonNormalAssembly::
try_query_part_transform` already owns that later attribute-dependent query
and must be reused. Focused tests cover source call order, separate linked
height reads, and strict tie behavior. Combined build and mounted replay
are parent-owned.

## Reached part-transform calls (2026-09-24)

The mounted replay after the raw local prefix reaches frame 59, physical
OffBoard500, and stops at `PendingPartTransform { part: 15 }` in
`artifacts/raw-y-feetdown-prefix-2026-09-24.log`. IDA
`CalculateFeetDownState` `0x82dcde7c..82dcdecc` issues four fresh
`Skeleton::GetPartTransform` calls in part order 15, 19, 16, 20 and keeps
their full translation vectors. Each call reads the plain animation Instance
attribute and selects a typed physics-history transform or world-composed
animation transform. The existing `SkeletonNormalAssembly::
try_query_part_transform` already implements this source branch, with prior
review in `docs/evidence/physical-player-skeleton-line-tests.md` and typed
history W propagation from `docs/evidence/phys-record-lane-knowledge.md`.
The feet-down adapter now reuses that service four times without a second
query implementation, preserving the four earlier raw local translations as
separate stack scratch. Any query failure identifies its part. The next
typed boundary is `PendingLeftFootDownResolution`: native first tests the
left surface valid flag, then projects the queried toe/foot positions onto
the clamped ground normal if valid. No foot-down projection or Processed flag
override is claimed. A focused test checks the four-call order and separation
of raw and part-transform vectors; combined build and replay are parent-owned.

## Reached surface foot-down resolution (2026-09-24)

The mounted raw-Y replay after the four fresh part queries reaches frame 59,
physical OffBoard500, at `PendingLeftFootDownResolution` in
`artifacts/raw-y-feetik-part-transforms-2026-09-24.log`. IDA
`CalculateFeetDownState` `0x82dcded4..82dcdf84` tests each retained result
valid flag in left-then-right order. For an invalid result it copies that
foot's animation foot-down flag without reading the linked height. For a
valid result, it subtracts the surface position from the queried world foot
position, reads the current `kYHeightToBeOffGround`, then subtracts the same
surface position from the queried world toe. `vmsum3fp` projects both XYZ
deltas onto the clamped ground normal; `vmaxfp` selects the larger distance.
Only a strict `largerDistance < height` marks the foot down. Left foot is
stored before the right validity branch. The Rust implementation preserves
this order, uses the established `host_dot3` ordinary-finite policy, and
keeps unknown W typed because the reductions consume XYZ only. VMX max NaN
selection and exact Xenon reduction rounding remain unverified. The next
typed boundary is `PendingProcessedFootFlags`: Processed +8d0 and +8cc bits
can override the two resolved flags, and are not skipped or guessed. Focused
tests cover the larger projected distance, strict tie, invalid result, and
height-read gate. Combined build and replay are parent-owned.

## Processed foot-fix override (2026-09-24)

The mounted raw-Y replay after surface resolution reaches physical OffBoard500
at `PendingProcessedFootFlags` in the parent-owned log. IDA
`CalculateFeetDownState` `0x82dcdf88..82dce004` loads Processed `+0x8d0`
once. PPC `extrwi` positions 22 and 23 select low masks `0x200` and `0x100`.
The source conditionally reloads Processed `+0x8cc` for each reached branch,
using `rlwinm 28,28` (mask `0x8`). In exact branch order, bit9 with clear
`+8cc` bit3 forces left=true/right=false; bit8 with set bit3 forces the same;
bit8 with clear bit3 forces left=false/right=true; bit9 with set bit3 forces
the same. The first taken branch wins, so a host simplification that combines
these into one boolean expression would change behavior when both fix bits
are set or the linked word changes between reads. The Rust leaf reads the
same retained Processed owner on each reached check, preserving typed
missing-word errors. No additional flag owner is created. `CalculateFeetDownState`
then returns; `OffBoardFeetIK::UpdateState` at `0x82dd1bb8` is the next
typed pending boundary. Focused tests cover priority, rereads and no-read
behavior when neither fix bit is set. Combined build and replay are
parent-owned.

## Reached `UpdateState` (2026-09-24)

The mounted raw-Y replay after the Processed foot-fix override reaches frame
59, physical OffBoard500, at `PendingFeetIkUpdateState` in the parent-owned
replay. IDA `OffBoardFeetIK::UpdateState` `0x82dc89a0` first adds the literal
`kUpdateTime_68` at `0x82355824` (bits `0x3c888889`, 1/60 second) to
`mTimeInState`. It reads Processed `+0x8cc` mask `0x40`; a set bit selects
FeetIK state 3 (locomotion) and skips the physical-state read. Otherwise,
Processed current physical state `+0x8e8 == 502` selects state 4 (air).
Otherwise, only with foot locking active, left-only down selects state 0,
right-only down selects state 1, and all other cases select state 2. A state
equal to the retained `mState` causes no more writes or part queries. On a
change, native stores the new state, resets `mTimeInState` to full +0, and
then, for state 0 or 1, issues a fresh `Skeleton::GetPartTransform` for part
15 or 19 and copies its full translation into `mFootPos +0x20`. States 2, 3
and 4 write a full zero vector there. The Rust leaf reuses the existing
attribute-dependent part query; `foot_position` is typed to preserve unknown
W in physical-history transforms. A failed query retains the native state
and timer writes before the typed error. Focused tests cover unchanged state,
ordered reads, fresh left query with unknown W, and locomotion's skipped
physics query/zero position. `CalculateIKTargets` at `0x82dd1bc0` remains
the next typed pending boundary. Combined build and replay are parent-owned.

## Isolated `CalculateNewYPos` scalar leaf (2026-09-24)

The mounted raw-Y replay after `UpdateState` reaches frame 59, physical
OffBoard500, at `PendingCalculateIkTargets`. The state-2 path of IDA
`OffBoardFeetIK::CalculateIKTargets` at `0x82dd0928` can call
`CalculateNewYPos` at `0x82dc8b38` for each valid foot surface. This scalar
leaf subtracts `mYPosOfAnimSpace +0x100` from the animated Y and adds the
ground Y; it reads the three linked cvars in the exact order
`kYMinHeightToBeOffGround` (`0x82dc8b68`, table `0x82fd5e24`, default
`0x3ca3d70a`), `kOBIK_IKMaxYLiftUp` (`0x82dc8b70`, table `0x82fd5e38`,
default `0x3da3d70a`), and `kOBIK_IKMaxYDistUp` (`0x82dc8b74`, table
`0x82fd5e34`, default `0x3ecccccd`). The initializer addresses are
`0x82f98d58`, `0x82f98f38`, and `0x82f98ed8` respectively. The original
uses three `fsel` instructions to bound the height between ground plus
minimum and ground plus minimum plus maximum lift while never lowering the
animated Y. If the upward adjustment strictly exceeds maximum distance,
it returns the original animated Y. The Rust helper follows these ordinary
finite operations and live linked cvar reads, reusing the retained
`OffboardFeetIk.animation_space_y` field. Xenon exceptional `fsel`/FPSCR
behavior remains unverified. Focused tests cover read order, the three
height cases and linked startup overrides. This leaf is isolated; the
enclosing left/right target calculations and blends remain typed pending
and are not yet claimed as live.

## Reached state-2 `CalculateIKTargets` (2026-09-24)

IDA `CalculateIKTargets` `0x82dd0928` loads the current raw animated left
and right positions from +0x30/+0x40 before checking state. The reached
state 2 takes the nonfixed-foot left branch at `0x82dd0aec` and the
nonfixed-foot right branch at `0x82dd0c98`. In each half, a valid surface
first subtracts the positive part of the retained slope adjustment from
animated Y, then calls the already isolated `CalculateNewYPos` with the
surface Y. It changes only the Y lane of that half's current animation
vector. A nonpositive blend-out timer selects weight 1 without reading a
blend cvar. A positive timer is decremented by literal `kUpdateTime_68`
(`0x3c888889`), stored, clamped to zero, then reads the linked
`kOBIK_TimeToBlendOutFixedFootPos` value and computes weight `1-timer/value`.
Its initializer is `0x82f98e78`, table `0x82fd5e30`, default `0x3dcccccd`,
section `Physics_Systems_OffboardIK`, title `Fixed Foot BlendOut Time`.
The full target store blends the retained prior target and adjusted raw
vector, left before right. Rust retains unknown W through this full-vector
store and preserves timer writes if a later cvar read fails. Focused tests
cover surface cvar order, timer-gated blend read, unknown W and partial timer
write. State 0/1 fixed-foot clamping and state 3/4 remain typed pending. The
enclosing `OffBoardFeetIK::Update` next reads `kEnableOffBoardFeetIK` before
the optional `ApplyIK`; that gate is the next live typed boundary.

## Reached `ApplyIK` enable and local foot targets (2026-09-24)

The raw-Y replay after state-2 target calculation reaches frame 59 at
`PendingApplyIkEnable`. IDA `OffBoardFeetIK::Update` `0x82dd1bc4..e0` reads
`kEnableOffBoardFeetIK` and calls `ApplyIK` only if true. Its bool initializer
is `0x82f98800`, table `0x82fd5de8`, default true, section
`Physics_Systems_OffboardIK`, title `Enable FeetIK`. The reached state-2
`ApplyIK` at `0x82dcd96c..82dcd9f0` reads `kOBIK_IKBlendVal` once before
calling `Skeleton::SetLeftFootIKTargetPositionLocal` then
`SetRightFootIKTargetPositionLocal`, both with that blend. The float
initializer is `0x82f98db8`, table `0x82fd5e28`, default `0x3ecccccd`,
title `IK BlendVal`.

IDA's left/right local setters are `0x82b412f0` and `0x82b41348`. Each
reads the same retained `Skeleton::mWorldToAnimTransform` at +0x2dc0 and
transforms XYZ with its four columns in translation/X/Y/Z multiply-add
order; the input W is not a coefficient. They write full local target vectors
to the retained `SkeletonIK` owner at +0x260/+0x270, then set its local-target
bytes at +0x328/+0x329 and blend scalars at +0x32c/+0x330. Existing
`SkeletonRootTransforms.world_to_animation`, `SkeletonIkState` storage, and
the existing `external::transform_point` matrix operation are reused; no
parallel target buffer or transform implementation is introduced. The Rust
adapter calls the two setters in native order on the same retained pair and
keeps unknown W typed. Focused tests cover live linked cvar changes, separate
left/right stores, and a rotated geometric transform. The next typed
boundary is ApplyIK's left/right optional normal checks.

## Reached `ApplyIK` optional foot normals (2026-09-24)

After the two local target setters, IDA `ApplyIK` `0x82dcda0c..82dcdac4`
checks each foot in left-then-right order: the foot-down byte and surface
valid byte must both be set, then that surface's original normal Y must be
strictly greater than `0.7` (`0x3f333333`). Only then does it pass the
retained clamped ground normal to `Skeleton::SetLeft/RightFootIKTargetNormal`
at `0x82b412d0/e0`, both wrappers of the same retained
`SkeletonIK::SetTargetNormal` at `0x82b3da78`.

`SetTargetNormal` reads its prior normal-blend scalar first. Native enters
the clamp on an ordered positive or unordered comparison; the host predicate
is `!(blend <= 0.0)`. That path reads `kMaxNormalAngleDelta` and calls
`PhysicsUtility::ClampVectorWithinAngleRange` with the incoming and prior
normal; it then copies the full returned vector. The angle cvar is newly
registered at index 374 from initializer `0x82f7dac8`, table `0x82fd465c`,
default `0x3dcccccd`, section `Physics_Skeleton_IK`, title
`Max Norm Blend Delta`. The existing source-backed
`reckoning::air::try_clamp_air_angle` is the same PhysicsUtility leaf and is
reused, retaining unknown W. Native then sets the target-normal flag before
reading the already registered `kTargetNormalBlendSpeed` (index 98,
initializer `0x82f7da68`, table `0x82fd4658`, default `0x3e4ccccd`), and
stores `min(1, previousBlend + speed)` via ordinary-finite `fsel`. A failed
speed read retains the earlier flag and any clamped normal. Focused tests
check the first-call skip, conditional angle read and failure prefix, typed
W, live cvar defaults and separate left/right slots. The shared same-pair
adapter reads the right gate only after the optional left setter completes,
matching native failure-prefix order. The next typed boundary is the enclosing
OffBoard.Update continuation after FeetIK.UpdateGround.
