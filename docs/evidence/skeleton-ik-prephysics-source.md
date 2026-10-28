# Active IK pre-physics solve and line-frame dependency

2026-09-08, read-only author packet. This follows the independently
reviewed `skeleton-two-bone-solver.md` and its native Rust kernel. No Rust,
Cargo, IDA mutation or runtime parity claim is included here. The required path
is actual retained IK targets to existing mutable animation-volume rows, then
the same rows to Drives; replacement rotations or remapped raw poses would
bypass the recovered behavior.

## Source identities

All spans are half-open; SHA256 is over raw original database bytes.

| Routine | Span | SHA256 |
|---|---|---|
| `SkeletonIK::DoTwoBoneIk_PrePhysics` | 82b49950..82b4a140 | d5c459749ca24ae26cdbdfcc78ebe68be56a2a9c9b7a1771b1c884594d9b6b5a |
| `PhysicsUtility::ClampVectorAWithinRangeOfVectorB` | 82e496d0..82e49854 | 7d49e2b37876c82fd8c5f991b5b90e4be96eebe209eb6a5863c99679d1431551 |
| `SkeletonIK::CreateMtxFromLineAndNormal` | 82b3e030..82b3e1d8 | 2cc34e0f0d571e8a052cd22895cece462f00f7cae3694002b3e3663b22f93a5d |
| `SkeletonIK::CalculateMatrixFromLineNew` | 82b3e1d8..82b3e384 | 65154ecf300cf83210fc3cab8f6d09e6193fffd6e777c150108bfbaaee7c3671 |
| Immediate caller `SkeletonIK::UpdateDriveFrames` | 82b52e90..82b5321c | 2bf748933dc6c0e003968cde852fe6baeec6799b17cc7b9644e3e0446197f29a |

Decorated symbols:

```text
?DoTwoBoneIk_PrePhysics@SkeletonIK@Physics@Sk8@@AAA_NW4PartIndex@SkeletonDataBase@23@ABVVector3@vpu@math@rw@@MAAV6789@@Z
?ClampVectorAWithinRangeOfVectorB@PhysicsUtility@Physics@Sk8@@YA?AVVector3@vpu@math@rw@@ABV4567@0M@Z
?CreateMtxFromLineAndNormal@SkeletonIK@Physics@Sk8@@AAA_NABVVector3@vpu@math@rw@@00AAVMatrix44Affine@567@@Z
?CalculateMatrixFromLineNew@SkeletonIK@Physics@Sk8@@AAA_NABVVector3@vpu@math@rw@@00000AAVMatrix44Affine@567@@Z
```

Raw VMX128 VA extension bits were decoded independently of IDA's printed
aliases. Scalar branch BO/BI fields were checked for unordered behavior. The
decompiler mislabels several call addresses and pointer arguments; use the raw
addresses and chronology below.

## Actual owner and input authority

`DoTwoBoneIk_PrePhysics` receives this in r3, selected Part in r4, delta pointer
in r5, maxLength float in f1, and **adjust output in r7**. r6 is the floating
argument's integer argument slot, not the output pointer. The sole call is
UpdateDriveFrames at82b53168. Its delta and adjust are separate locals; the
decompiler's apparent same-address alias is incorrect.

The owner reads SkeletonData through IK+18. The actual parent array is
SkeletonData+12d0, 24 signed Part indices. DataIn is IK+20; its part-order raw
animation matrices are DataIn+1a4 = IK+1c4, and mutable mapped volumes are
DataIn+1a0 = IK+1c0. Each row is64 bytes and position is row+30.

This raw animation buffer is the existing `SkeletonAnimationTargets.animation`
array in Part order, not the hierarchy-indexed input pose and not retained IK
targets. `SkeletonData::parents()` is the correct ancestry authority; do not
invent a fixed anatomical chain or derive parent indices from hierarchy IDs.

The source load order is:

1. Assert selected index != -1, then load parent[selected].
2. Assert parent != -1.
3. Load delta, then selected animation P at9a30.
4. Load grandparent = parent[parent] at9a38; there is no additional native
   grandparent-index assertion here.
5. Load parent animation P at9a50, then grandparent animation P at9a64.

A checked host implementation should preserve that true-stage order: an absent
selected row precedes parent-row failure; an invalid grandparent index becomes
an error at the actual grandparent pose access after the parent pose read.
Native indexes beyond the array are invalid memory accesses, not extra game
fallback behavior. Actual typed Part indices exclude that class of host access.

Let A = grandparent P, B = parent P, C = selected P. The immutable intended end
is `requested = C + delta`, all four lanes. A zero scratch middle output is
initialized, then requested is clamped to maxLength around A. The unclamped
requested value remains retained in a local through the solver and later
defines the published adjustment.

## Clamp within range, 82e496d0

The range is an actual f32 argument despite decompiler `double`. The graph is:

1. Assert only ordered `range < 0` at4970c/49710. Raw40980018 branches on NOT_LT,
   so NaN and -0 pass this initial assertion.
2. Load full4 A and B; delta=A-B. Dot3(delta,delta), reciprocal-square-root
   estimate, two accepted Newton refinements, and squared*refined. Select full
   positive zero when squared==0, after all arithmetic/services execute.
3. If ordered `range >= length`, copy the original full4 A unchanged.
4. Otherwise assert ordered `length > 0`. Unordered length fails this assertion.
5. Compute **scalar fdivs `range / length`**, not an estimate/refinement or squared
   range shortcut. Return `FMA(delta, factor, B)` in all four lanes.

On either assertion or a numeric collaborator failure, the output has not been
written. Range NaN with finite positive length reaches the division and yields
the source exceptional vector; range NaN with zero length fails the later
positive-length assertion. The return-A path must retain noncanonical/unknown
W. The clamped path propagates full4 W knowledge through delta and the FMA.

## Initial debug and two lazy global attributes

After clamping and copying the clamped result to the local solver target, read
`kDrawPreFeetIK` at9ab0/9ab8. Reference8308e378 is initialized false by82f7d878
(default r7=0 atd8a0). This is not the later draw read.

If enabled, compute lengths of requested-A and clamped-A. Both dot3 calls run
before both inverse-square-root seeds; each gets the two refinements and zero
selection. Only ordered firstLength > secondLength obtains the actual debug
singleton. Raw40990068 at9b80 is NOT_GT, including unordered. The orange
CLAMPED draw target825187c0 is a proven `blr` stub (4e800020). Preserve the
numeric callbacks and singleton availability check, but do not claim a missing
renderer for this no-op draw.

Next sample the actual global `physics_animation.default` instance, through
GlobalAttributeCollection+cc, twice independently:

| Order | Key64 | Index | Pointer call | Scalar load | Solver argument |
|---|---|---|---|---|---|
| 1 | b76b5101768e856c | 0 | 82b49c2c | 49c44 | minimum degrees, f1 |
| 2 | a175875dec9d6db7 | 0 | 82b49c84 | 49c9c | maximum degrees, f2 |

Each missing pointer invokes DefaultDataArea(4); the previously verified
original default region supplies zero. Present data must be validated as the
actual four-byte scalar, with source inheritance/index semantics. No graph
evaluation or payload8 interpretation occurs here. There is no retained cache
of these two scalar values across calls. The first scalar is loaded before the
second singleton assertion and attribute lookup; a late second read failure
leaves adjust and volume rows unchanged.

No unrelated processed COM, retained COM velocity, timestep or current root
input is consumed by this helper. The caller supplies maxLength from its late
leg/arm cvar samples and delta from its actual retained target transforms.

## Solver, adjustment prefix, and normal

Call the accepted TwoBone kernel at9ce0 with original A/B/C, mutable scratch
middle Bnew, mutable clamped target Cnew, fetched min/max, override=true and
remaining_calls=2. Preserve the kernel's local target mutation/error behavior;
those locals are not yet owner volume writes.

- Invalid2: store a full positive-zero adjust at9d0c; return false. No line
  construction, row writes or final debug read occurs.
- Every other returned outcome: publish `adjust = Cnew - requested` at9d24,
  using the **unclamped** requested end and full4 subtraction.
- Then assert outcome==Reachable0 at9d28..9d3c. Unreachable1 therefore exposes
  the already stored adjustment before a checked host assertion error. Do not
  turn it into silent success or false without that prefix. Native FailAssert
  and DebugBreak can continue under a debugger, which is not a host fallback.

For Reachable, form the normal:

```text
old_normal = cross(A-B, C-B)
new_normal = cross(A-Bnew, Cnew-Bnew)
normal = NormalizeSafe(old_normal + new_normal, full_positive_zero)
```

All cross/adding operations are full4. The normalizer is accepted824b18e0:
dot3, separately rounded subtraction of (bits358637bd)^2, strict positive gate,
then one estimate/two refinements and all4 scaling, or an exact zero fallback.
Unordered takes that fallback. Do not substitute the initial TwoBone plane
normal or a cross of root-to-end vectors.

## Create line frame, 82b3e030

For points a,b and supplied normal n, calculate full4:

```text
line = b-a
raw_side = cross(line, n)
raw_normal = cross(raw_side, line)
Z = NormalizeSafe(raw_normal, zero)   // FIRST call e0cc
X = NormalizeSafe(raw_side, zero)     // SECOND call e0e4
Y = NormalizeSafe(line, zero)         // THIRD call e0fc
x_squared = dot3(X,X)
z_squared = dot3(Z,Z)
y_squared = dot3(Y,Y)
quality = (z_squared*x_squared)*y_squared
```

The final three dots are all unconditional and ordered X,Z,Y; the products are
separately rounded in the shown Z*X, then*Y order. `quality > VECFLOAT_SMALL`
is ordered and strict. VECFLOAT_SMALL slot8307fe60 is initialized at82f76340
from literal821f0c20 bits37800000 (2^-16). NaN quality fails.

On success, store X,Y,Z, then full original a as P; return true. On failure,
store the actual I/J/K vectors and full positive-zero P; return false. Original
I/J/K at8227e8f0/e900/e910 all have W0. The valid frame's basis W is the actual
normalization/cross result, not a canonical zero. Fallback normalization may
recover a known zero W from an unknown input W. Numeric failure before this
final success/fallback publication leaves the caller's output unmodified.

## Matrix mapping between two lines, 82b3e1d8

Construct the old line frame first at e1fc. Only if true construct the new frame
at e220. If either returns false, copy I/J/K/P0 to result and return false.
No second-frame callbacks occur when the first frame fails.

On two successful frames, compose `new_frame OUTER * rigid_inverse(old_frame)
INNER`. This is the accepted rigid inverse graph, unlike the TwoBone cofactor
inverse:

- The word merges transpose old rotation XYZ and set each inverse basis W0.
- Compute negative old P as **0-P**, not sign-bit XOR.
- Inverse P starts Z*negativeP.z, then Y FMA, then X FMA, all four lanes.
- Compose each basis starting X multiply, then Y and Z FMA. Position starts
  X FMA into new P, then Y/Z FMA. Inner W never becomes a coefficient.
- Native result commits Z at e304, Y at e314, X at e31c, then P at e328.

Existing `skateboard_spawn_rigid_inverse` and affine composition have exactly
these graphs. Use typed `SkeletonRootMatrix` knowledge alongside the known XYZ
scratch boundary: inverse W is derived and known even when old W is unknown;
new frame W still governs the result's W. Do not transpose a full fourth row
or use inverse P from the TwoBone cofactor helper.

## Ordered real volume mutations

The pre-physics caller first calculates the LOWER line mapping at49de0:
old B->C, new Bnew->Cnew, the same recovered normal for both. Only on success
calculate the UPPER mapping at49e10: old A->B, new A->Bnew.

If either mapping is false, skip both volume writes. The already published
adjustment survives. Continue to the final debug stage and return false.

On both successful mappings:

1. Read the actual current mapped **grandparent** row at49e40..9e58. Compose
   upper_mapping OUTER * row INNER. Normalize its rotation using accepted
   OrthoNormalizeRotation82b22c78. Commit all four columns back to grandparent
   at49f24/28/2c/30, X/Y/Z/P.
2. Only now read actual current mapped **parent** row at49f44..9f50. Compose
   lower_mapping OUTER * row INNER. Normalize its rotation independently.
   Commit X/Y/Z/P at4a014/18/1c/20.

The accepted normalizer uses classical reverse-axis Z,Y,X Gram-Schmidt; each
projection dots the normalized previous axis against the ORIGINAL current
input column, then separately rounded product/subtraction from its candidate.
Each axis has a dot3 and reciprocal-square-root estimate with two refinements;
there is no zero/finite fallback. P is copied from the composed input. Full4
normalization preserves or propagates unknown W, never canonicalizes it.

No selected Part row is written by this helper. A parent-row lookup or numeric
failure must retain the already committed grandparent row. Do not build and
atomically replace a pair of rows, preflight the entire pose, or re-map raw
animation into a fresh volume buffer before these reads.

## Final debug and true/false return

On every non-Invalid, non-assertion path, including a false line mapping, read
kDrawPreFeetIK again at4a024/28. If enabled, acquire the debug singleton FIRST,
then compute the original delta's zero-selected length (dot3, seed, two
refinements). Both calls at4a0d0/4a10c target the same verified `blr` stub.
No additional singleton acquisition occurs between them. A final cvar,
singleton or numeric failure retains adjust and any completed row writes.

After that stage return true only if both line mappings succeeded and both
actual mapped rows were written. Debug work does not make a false line mapping
true. On a successful source path no mapped pose/root/Part owner is replaced.

## Immediate producer and consumer in UpdateDriveFrames

The source caller's gate and retained-field overview are in the General IK
packet. The relevant concrete bridge is:

- For each active mode1/2/3, transform retained `mIKPartToWorld[i]` into animation
  space with DataIn.WorldToAnim outer. If `mIKPartParentIdx[i]` exists, similarly
  transform mIKParentToWorld and select that parent's row for the two-bone
  solve; otherwise select mIKPartIdx.
- `SkeletonData::GetBoneMtxFromVolMtx`82b3ae60..af50 is called at53104 with the
  selected target volume and Part. The existing same-owner volume_to_bone table
  is available. The requested delta at53144 is resulting bone P minus the
  actual selected part-order animation P.
- The helper receives the selected Part and the per-target maxLength scalar.
  The caller initializes a separate adjust local to positive zero.
- Only on helper true, write the primary target volume row at53194/98/9c/a0,
  preserving the previously computed target basis and adding adjust to its P.
  Then add the same adjust to retained mIKPartToWorld[i].P at531ac. This is
  source behavior even though adjust was calculated in animation coordinates.
- If a retained parent target exists, next write that target volume row at
  531d0/d4/d8/dc and add adjust to mIKParentToWorld[i].P at531e8.

Thus the complete path writes grandparent, parent, then primary target, then
optional target-parent, with retained target P updates interleaved as above.
Target mapping initialized by the existing actual IK owner is [toe15,toe19,
hand3,hand7], optional target parents [foot16,foot20,None,None]. The selected
solver Parts are therefore normally16,20,3,7. Their ancestors must still be
looked up from actual SkeletonData, not hardcoded here.

This immediate caller also selects active modes, reads kDOPrePhysIK and its
leg/arm maxLength values, loops four entries, and uses retained IK matrices.
Those existing owner responsibilities must call the real helper. Supplying
replacement upper/lower matrices would leave this dependency unimplemented.

## Concrete integration proposal

Use a native `skeleton_ik_prephysics` kernel plus a small line-frame helper
module if size warrants. Reuse `skeleton_two_bone`, existing typed root matrices,
actual SkeletonData, and same-owner part-order animation/volume rows.

- Clamp API: typed A/B/range and fallible ReckoningNumeric, with distinct
  NegativeRange and NonpositiveLength assertion errors. Scalar f32 division is
  the actual arithmetic boundary, not a new reciprocal service.
- Line APIs: typed a/b/normal and mutable typed result, returning Result<bool>.
  Preserve false->identity and failure->unwritten behavior and nested-call
  short circuit. Numeric services are the existing dot/estimate primitives.
- PrePhysics API: actual data, selected Part, actual animation and mutable volume
  row authority, delta, maxLength, mutable adjust, lazy attributes, cvars,
  numeric and trig collaborators. Return Result<bool>, with separate pose,
  clamp, attribute, cvar/debug, solver and outcome-assertion errors.
- Attribute trait needs only `minimum_angle_degrees()` and
  `maximum_angle_degrees()`; the actual global Vault adapter owns binding and
  source reads. Cvar trait needs repeated `draw_pre_feet_ik()` and an actual
  `require_debug_overlay()` availability check, without invented draw services.
- The final normalizer's accepted graph should be exposed through a typed
  fallible helper if failure-prefix tests use ReckoningNumeric. The existing
  SkateboardSpawnMathNumeric graph is infallible; do not erase failures by
  swallowing a fallible numeric collaborator in an adapter.

Root owns the necessary animation-row getter/mutable-volume-row seams. The
current arrays are `[Option<AnimationMatrix>;24]`; their actual completed owner
can provide sequential checked access. A complete preflight is unnecessary.
If typed partial output ever reaches that known-W storage boundary, refuse
before its particular store or retain typed knowledge in the existing owner;
do not fill unknown W. The normal live caller's WorldToAnim is a known rigid
inverse. As OUTER transform it recovers known full4 target animation rows even
when retained world target W is unknown; raw part-order poses are also known.
Thus this actual path can produce known-W row writes without inventing lanes.

## Required focused checks and confidence

Tests should cover actual parent lookup and selected/parent/grandparent pose
failure ordering; no raw-hierarchy indexing; range negative/zero/equality/NaN
branches and scalar-division/FMA discrimination; both debug reads and singleton
timing; first/second attribute failures; Invalid zero adjustment; Unreachable
adjust-before-assertion; normal cancellation/fallback; both line-frame false
routes with identity; X/Z/Y final dot order and strict quality threshold; W
knowledge through inverse and mapping; frozen oblique line transforms; actual
grandparent commit before a failed parent read/normalization; final-debug failure
after both writes; and selected-row/world-P caller commits only on true.

An original SkeletonData corpus should verify the actual four selected parent
chains and drive this real helper through the same owner volume rows, then the
existing Drives path. A separate frozen oracle must compare full4 row output
and adjustment, not merely assert that a helper was called.

Confidence **high** for raw control flow, ABI, owners, constants, all immediate
math graphs, W knowledge and store/service ordering; independent review is
requested. Inherited Xenon dot/estimate/denormal/NaN/zero-sign numeric limits
remain explicit. There is no new deterministic reference runtime trace or live
active-IK execution claim. No unidentified immediate math callee blocks this
bounded helper; full owner/stage/caller integration remains required.
