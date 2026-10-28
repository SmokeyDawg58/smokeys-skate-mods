# IK initial, external, and blended transforms

Read-only author packet, 2026-09-08. This extends the lifecycle and pre-physics
packets with the actual retained transform producers immediately before contact
and drive-frame solving. No Rust implementation or Cargo run accompanies this
packet. Confidence is **high** from full raw instruction/operand inspection;
independent review and implementation/corpus validation remain separate gates.

## Source identity and audit boundaries

| Native symbol | Half-open byte span | SHA-256 |
| --- | --- | --- |
| `Sk8::Physics::SkeletonIK::CalculateInitialPartTransforms()` | `82b51710..82b51d38` | `bd946b97252de0bd250b99ecdeb685e5ebf4ede5eeaefdff8b0d2ba4867897b0` |
| `Sk8::Physics::SkeletonIK::CalculateExternalIKOffset(uint)` | `82b48d10..82b49534` | `56ae09a3422eac99c4aef223eb4609059a28dd9e2eaa108638fe0f23f8286991` |
| `Sk8::Physics::SkeletonIK::BlendTransforms()` | `82b520d0..82b52e8c` | `1504f1073f2b7f1bb9ee86c5621265f0f2e042ab01c269e8d2853934e50974a9` |
| `Sk8::Physics::PhysicsUtility::ClampVectorWithinMaxLength(Vector3 const&,float)` | `82e49430..82e4957c` | `90d8996ac7ee4b2e035c026b1adb1d6dffc62962f7a76674ac17b872a8327efd` |

Decorated names are respectively
`?CalculateInitialPartTransforms@SkeletonIK@Physics@Sk8@@AAAXXZ`,
`?CalculateExternalIKOffset@SkeletonIK@Physics@Sk8@@AAAXI@Z`, and
`?BlendTransforms@SkeletonIK@Physics@Sk8@@AAAXXZ`.
`SkeletonIK::Update` calls Initial at `59c80`, Blend at `59c88`, then samples
contact enable at `59c94/98`, conditionally calls contact `59ca8`, and calls
DriveFrames `59cb0`. `UpdateWipeoutAdjust` also calls Initial/Blend at
`59d80/59d88`; this packet does not substitute the ordinary caller for wipeout.
External has one code caller, Initial `518a8`.

Raw VMX128 VA extension bits were decoded; misleading printed registers such
as v95 versus actual v63 were not used as semantics. Standard `vmaddfp` uses
its fourth operand as multiplier and third as addend. Scalar BO4/BI24 is
**not less than**, including unordered; BO4/BI25 is **not greater than**,
including unordered. These distinctions determine several gates below.

## Existing owner and actual inputs

The newly integrated `SkeletonIk` aggregate already retains `SkeletonIkState`,
`SkeletonIkDataIn`, its own `SkeletonIkVaultAttributes`, geometry, and the
existing same-Skeleton `SkeletonIkTargets`. Add no competing owner or pose.

| Native IK member | Offset | Existing/logical authority |
| --- | --- | --- |
| `mSkeletonState` | `1c` | same normal owner's previous AnimationRecord |
| DataIn `AnimToWorld`, `WorldToAnim` | `20`, `60` | `SkeletonIkDataIn` typed root matrices |
| DataIn `PhysicsSkateboard` | `a0` | same retained physical skateboard matrix |
| DataIn mapped rows/raw part-order poses | `1c0`, `1c4` | borrowed actual buffers; not read by these three functions |
| on-deck/external blends, modes | `1d4`, `1e4`, `1f4` | `SkeletonIkState` arrays |
| external world/local/delta positions | `220`, `260`, `2a0` | state typed vectors |
| external normal, normal-set, target blend, normal blend | `2e0`, `320`, `32c`, `33c` | state fields |
| retained target transforms | `360` | existing `SkeletonIkTargets` only |
| primary/optional-parent Part IDs | `460`, `470` | existing initialized target mapping |
| primary world/external-world/deck matrices | `480`, `580`, `680` | state matrices |
| prior/current deck-local Part position | `780` | state vector; BlendValues consumes previous value |
| parent animation/world/external-world/deck matrices | `7c0`, `8c0`, `9c0`, `ac0` | state matrices; `7c0` is not written here |
| contact target/current offsets, positions, validity | `bc0`, `bd0`, `be0`, `c20` | preserved by these stages |
| PartOnDeck | `c30` | state four u32 values |

`SkeletonState+10` is previous AnimationRecord volume[0], the deck matrix;
`SkeletonState+40` is that matrix's P. Previous record volume[Part] supplies
the initial animated primary/parent matrices. It is neither current adjusted
mapped0 nor the raw animation bone pose. The target matrix is a different
input, produced by CalculateTargetTransforms and adjustment/AddOffsetTransform.
The target Part IDs are toes 15/19 with parents 16/20, hands 3/7 with no parent.
Actual ancestry for solving remains the separate pre-physics stage.

Use `SkeletonRootMatrix` and `SkeletonRootVector` throughout. These functions
do not require unknown W to become known before proceeding: all geometric
comparisons and dot3 operations use XYZ, while full-four-lane affine/weighted
operations propagate known W when available and retain unknown W otherwise.
Rigid inverses reconstruct basis W=0 and P.W through the actual inverse graph;
they ignore input basis/P W. Never use a generic inverse or impose P.W=1.

## Common affine and numeric graphs

`compose(outer,inner)` means basis column `inner.X * outer.X`, followed by
Y then Z FMAs, per lane. Translation starts with X*outer.X + outer.P in an
FMA, then Y and Z FMAs. Inner W does not contribute. Preserve zero-coefficient
terms; these affect nonfinite values and full W.

Rigid inverse transposes XYZ, inserts basis W=0, computes `negativeP = 0-P`
(not sign-bit XOR), then P with Z multiplication, Y FMA, X FMA. Existing
`skateboard_spawn_rigid_inverse` graph matches. Root's typed equivalent needs
exposure to these stages without requiring every input W.

Existing `reckoning_numeric::try_clamp` is the exact direct clamp dependency:
negative maximum fails only for ordered `<0` (`49460` NOT_LT skips assertion
for NaN). Length executes dot3, reciprocal-square-root estimate, two Newton
refinements, multiply squared length, and source zero selection. If length is
ordered `< 0x37800000` (`SMALL_FLOAT_182` at `82388ab0`), return the complete
original vector. NaN length takes the other branch. Select length if
`maximum-length >= 0`, else maximum, using scalar FSEL. Then read the
reciprocal estimate of length and refine twice. The output is
`refinedReciprocal * (originalVector * selectedLength)`, with the inner
multiply rounded first. Do not replace this with `vector*(maximum/length)` or
an inside-radius early return. A typed adaptation can use zero scratch W for
dot3 only and calculate retained W separately if known.

Existing `try_asin` implements the actual `XMVectorASin` callee
`825e8ce0..825e8db0` (SHA-256
`fcf103a8beda22ab0bba1662d63b8fffe75d197cf21d50a0d1b0adbd81aed69e`).
External rotation calls combined `XMVectorSinCos` at `82584f60`; it must use
the existing combined graph, not standalone sine/cosine DAGs.

## CalculateInitialPartTransforms

1. Unconditionally compute rigid inverse of previous record deck at
   `51760..517a4`, before any attribute/cvar read or retained store.
2. Own `physics_skeletonik` Instance reads key `121e67e2b4096a41`, index zero,
   at `517a8`. Missing pointer alone takes `DefaultDataArea(16)` at `517b8`.
   Full vector load is `517c8`; negative extent uses sign-bit XOR. Existing
   `SkeletonIkAttributeRead::vector` preserves lookup/load timing and malformed
   payload refusal. Do not cache this extent across frames or add deck geometry.
3. Read `kDrawFeetOnDeckIK` at `517f4`. If true, execute the proven BLR debug
   text stub `825187c0` at `51814`; there is no singleton acquisition here.
4. For each entry 0..3, load mode, then clear PartOnDeck at `5188c`.
   Modes 2/3 call the complete External routine before any common gate.
5. Reread mode. Continue the common stage for mode 1, or ordered on-deck blend
   `>0`, or ordered external blend `<1`. Raw `518cc=40980428` means external
   NaN does not satisfy this last alternative. A skipped entry retains all
   common matrices/PartPos, but its OnDeck clear and external writes survive.
6. Assert primary Part != -1 at `518dc`; access actual previous primary row.
   The raw target matrix is loaded before the previous primary row. Checked
   malformed buffer/ID errors belong at those real reads, not a batch precheck.
   Calculate `PartToDeck = inversePreviousDeck * retainedTarget` and
   `PartToWorld = DataIn.AnimToWorld * previousPrimary`.
7. Commit PartPos=PartToDeck.P at `51a14`; deck matrix Z/P/X/Y at
   `51a18/1c/28/30`; world X/Y/Z/P at `51a34/38/3c/40`.
8. Compute per-axis outside bits `(negativeExtent > PartPos) OR
   (PartPos > extent)`. Both comparisons are ordered. The shift
   `vsldoi(zero,outside,12)` yields [0,outsideX,outsideY,outsideZ], discarding W.
   All-zero comparison becomes OnDeck; store at `51a68`. Bounds are inclusive;
   a NaN coordinate/extent creates no outside bit for that comparison.
9. Read the same draw cvar again at `51a70`, separately for each qualifying
   entry. If true, BLR text call `51a94`, acquire actual DebugOverlay singleton
   `51aa4` (assert if absent), then green/red BLR call `51b3c` according to the
   newly stored OnDeck flag. No renderer dependency is present in these stubs.
   A singleton refusal retains all preceding common output stores.
10. Only now read optional parent ID (`51b40`). If absent, advance. Otherwise
    read previous parent row and compute
    `relation = rigidInverse(previousPrimary) * previousParent`;
    `ParentToDeck = PartToDeck * relation`, then
    `ParentToWorld = PartToWorld * relation`.
    Parent deck X/Y/Z/P stores are `51c84/88/9c/a0`; parent world X/Y/Z/P stores
    `51ce4/e8/ec/f0`. No contact or parent-to-animation field changes.

The initial stage itself has no estimate/normalization callbacks, collision
query, or current-pose mutation. Its external subroutine supplies the numerical
services described next. Deck-local W is recoverable from known inverse outer
columns even if target W is unknown; world W may remain unknown.

## CalculateExternalIKOffset

Entry assertion requires index<4. At `48d6c`, call the actual previous-record
primary accessor, including its Part validation, before any output mutation.
Let A be that animated primary matrix, W=`AnimToWorld*A`, and R the literal
I/J/K/P0 matrix. Read old normal blend b from `33c` before its gate.

For ordered b>0 (`48e68` NOT_GT skips zero, negative, and NaN):

- Compute axis=`cross(J,externalNormal)` using the full-four-lane rounded
  cross DAG, then zero-selected length (dot3, rsqrt seed, two refinements).
- If ordered length>bits `3d4ccccd` (0.05), call ASin(length) at `48f28`;
  read reciprocal estimate(length) at `48f2c`; then read `kEXIK_MaxAngle`
  at `48f64`. Preserve this service order. Refine reciprocal twice.
- Scalar clamp is `lower=fsel((-max)-angle,-max,angle)` then
  `bounded=fsel(max-lower,lower,max)`. Angle passed to combined SinCos at
  `48fa4` is rounded `bounded*b`. Axis is full4 multiplied by refined
  reciprocal. No generic min/max or atan2 replacement.
- Rodrigues uses q=1-cos, qx=q*x, qy=q*y, qz=q*z, sx=sin*x, sy=sin*y,
  sz=sin*z. Columns are
  `[fma(qx,x,cos), fma(qx,y,sz), rounded(qx*z)-sy, firstLane]`,
  `[rounded(qy*x)-sz, fma(qy,y,cos), fma(qy,z,sx), firstLane]`,
  `[fma(qz,x,sy), rounded(qz*y)-sx, fma(qz,z,cos), firstLane]`.
  P remains full zero. Raw table `82fd6e90` is
  `00010203 14151617 00010203 00010203`; Z-only inserts overwrite lane Z.
  Thus basis W=X is source-derived, not a homogeneous fill.
- Whether the length gate succeeds or not, if normal-set is false read
  `kTargetNormalBlendSpeed` at `4905c`, store b-speed at `49064`, then store
  `fsel(-(b-speed),0,b-speed)` at `49074`. NaN is retained by FSEL. These
  stores precede the local/world mode branch and subsequent clamp failures.

For mode 3 (ExternalLocal), read target blend t at `490a8`. With d the old
local delta and L the old local target, compute
`blendedDelta = fma(L-A.P,t,rounded(d*(1-t)))`, then step=blendedDelta-d.
Read `kMaxIKLocalDelta` at `49130`, call exact clamp at `49134`. After it
succeeds, store `localDelta = d+clampedStep` at `49160`, then
`localTarget = A.P+blendedDelta` at `49164`. The local target deliberately
uses the UNCLAMPED blendedDelta. Map this newly computed local target through
AnimToWorld and store external world position at `49190`.

Otherwise assert mode==2 at `4919c`. Map external world position through
DataIn.WorldToAnim and store external local position at `491f8`. No local
delta update occurs. A host assertion at this point must retain any preceding
normal-blend decay; do not validate mode before that decay.

Then reload external world position, subtract W.P, read
`kMaxIKOffsetLength` at `49220`, and call clamp `49224`.
Let C=W.P+clampedOffset. Build all three native compositions, including
identity zero-coefficient terms:

1. `pivot = R * translation(signbitXorNegative(W.P))`.
2. `movedPivot = translation(C) * pivot`.
3. `externalPrimary = movedPivot * W`.

The negative translation here is bitwise sign flip, unlike rigid inverse's
0-P. Do not algebraically replace the graph with a position assignment or
rotate-only composition. Publish external primary X/Y/Z/P at
`493ac/b0/b4/bc`. Optional parent ID is read at `493c0`; if present, only then
call previous parent accessor `493d4`. Compute
`relation=rigidInverse(A)*previousParent`, and
`externalParent=externalPrimary*relation`; publish X/Y/Z/P at
`494fc/49504/49514/49518`. Parent accessor failure retains primary output.

There are no Vault, contact, or debug-overlay reads inside External. Its cvars
are sampled lazily at the exact sites above, not eagerly per entry or frame.

| Cvar | Reference / initializer | Constructor default bits | Declared min/max |
| --- | --- | --- | --- |
| `kDrawFeetOnDeckIK` | `8308ed90 / 82f7d918` | false | bool |
| `kTargetNormalBlendSpeed` | `8308e200 / 82f7da68` | `3e4ccccd` (0.2) | 0 / 1 |
| `kMaxIKOffsetLength` | `8308e674 / 82f7dc68` | `3f333333` (0.7) | 0 / 2 |
| `kMaxIKLocalDelta` | `8308ea24 / 82f7dcc8` | `3ccccccd` (0.025) | 0 / 0.5 |
| `kEXIK_MaxAngle` | `8308e0a0 / 82f7dd28` | `3f35c28f` (0.71) | 0 / 2 |

Defaults are evidence for the cvar service, not constants to substitute for
runtime reads. Missing attribute default bytes are zero as independently
recovered in the lifecycle packet; this pass did not decode the mounted
`121e67e2b4096a41` payload anew.

## BlendTransforms

Before visiting any mode, construct one hybrid board B: its basis is
DataIn.PhysicsSkateboard.X/Y/Z, while its position is
`DataIn.AnimToWorld.transformPoint(previousRecord.deck.P)` (`52104..52190`).
PhysicalSkateboard.P is NOT the translation source. Neither current mapped0.P
nor DataIn.skateboard.P is substituted.

Visit entries ascending. Mode 1 performs base blending only if ordered
on-deck blend>0 (`521b8` NOT_GT); then advance. Modes 2/3 perform the same base
blending if ordered blend>0 (`52784` NOT_GT), followed by external blending
regardless of that gate. Disabled/unknown modes skip all output work.

Base blending, first primary then optional parent:

1. Candidate=`B*PartToDeck` (or ParentToDeck). Before normalizing, calculate
   `P=fma(candidate.P,b,rounded(oldWorld.P*(1-b)))`, across all four lanes.
   This is the reverse rounded-addend choice from InterpolateMatrix44Affine's
   weighted-P wrapper. Do not invoke that wrapper for this P.
2. Normalize oldWorld rotation into a temporary with P=full zero, then
   normalize candidate rotation into another temporary with P=full zero.
   BOTH run before SLerp, even if b>=1. Each uses classical Gram-Schmidt
   Z then Y then X. Projection dots use each ORIGINAL candidate column;
   subtractions update its working column. Each projection product is rounded
   before subtraction, and each final squared-length dot/rsqrt seed/two
   refinements precedes the next column. No zero guard. This is the accepted
   `skateboard_spawn_normalize` graph, not SLerp's small-angle normalization.
3. Call raw affine SLerp from normalized old to normalized candidate. The
   final pointer is an OUTPUT residual angle, not an input threshold. Calls
   are `5245c/52738` for mode1 primary/parent and `52a28/52d0c` for external
   modes' base primary/parent.
4. Only after success publish the complete result. Primary mode1 stores P/X/Y/Z
   `5247c/80/84/88`; parent mode1 `52758/6c/70/74`; external-mode parent
   `52d2c/40/44/48`. External-mode base PRIMARY is the exception: Z at `52a40`,
   P at `52a4c`, X at `52a50`, Y at `52a54`. No callback separates those column
   stores. No retained matrix prefix occurs inside normalization or
   SLerp. Parent ID is checked after primary commit and b is reread for parent.

The repeated normalizer loops were checked for actual source VA, original-dot
operand, Z/Y/X traversal, and old-before-candidate order. A fallible numeric
adapter must preserve these callbacks before SLerp's own query services.

External blending at `52d4c` reads external blend e. If ordered e<1:

- Call existing full affine interpolation(primaryWorld, primaryExternal, e)
  at `52d80`; publish primary X/Y/Z/P at `52da0/ac/b8/c0`.
- Reread e at `52dc4`, then interpolate parentWorld to parentExternal at
  `52dd0`; publish parent X/Y/Z/P at `52dd8/e4/ec/f4`.
- Parent interpolation is UNCONDITIONAL, even when optional parent Part is
  absent. A parent failure retains the completed primary interpolation.

Otherwise raw `52d58=409800a4` NOT_LT includes e>=1 AND NaN: copy primary
external X/Y/Z/P (`52e18/28/30/38`), then parent external X/Y/Z/P
(`52e40/48/50/58`). No normalization, SLerp, or numeric service occurs in this
copy path. Negative e uses the full interpolation wrapper, including mandatory
QueryRotate before the lower rotational endpoint and extrapolated weighted P.

No contact, cvar, or attribute service is sampled by BlendTransforms itself.
Contact enable is a later caller boundary, and DriveFrames consumes these
actual retained world matrices after contact's optional mutations.

## Minimal coherent implementation and checks

Extend `SkeletonIkState` with `try_calculate_initial_transforms(...)` and
`try_blend_transforms(...)`, and a private per-entry external helper. Borrow
actual `SkeletonIkDataIn`, previous AnimationRecord rows, and existing
`SkeletonIkTargets`; use the aggregate's existing attributes. Do not accept
caller-supplied substitute world/deck target matrices or create replacement
Part IDs. Root may expose a narrow mutable state view or register child modules
within the owner; all retained fields already exist.

A dedicated cvar trait needs lazy methods for draw-feet-on-deck, actual
debug-overlay availability, normal maximum angle, normal decay speed, local
delta maximum, and world offset maximum. Distinct errors should identify
attribute, cvar/debug, primary/parent record read or native assertion, numeric,
and trigonometry failures. Preserve true-stage validation and ascending entry
prefixes. Native stubs can be explicit no-ops; do not require a renderer.

Expose/extend the already audited affine SLerp and wrapper to typed
`SkeletonRootMatrix`. The XYZ graph and service order remain identical. W is
tracked per column: endpoints preserve it, linear normalization propagates it,
and Rodrigues paths recover known W from the proven W=X basis construction.
QueryRotate's unused center still executes all numeric services. It can use
XYZ scratch only with explicit unknown W propagation; the return must never
publish scratch W as native data. Promote the existing classical normalizer's
numeric graph to a typed helper as necessary rather than creating a second
algorithm. Reuse the exact clamp/ASin/SinCos helpers.

Meaningful implementation tests should cover:

- Previous-record deck and primary rows differing from current rows, target
  transforms, and physical board P; noncanonical and unknown W inputs.
- Four-entry ordering, PartOnDeck clear before External failure, common gate
  NaN combinations, inclusive bounds and ignored W, delayed optional-parent
  read failure, initial and per-entry draw sampling with singleton prefixes.
- Local delta clamp versus intentionally unclamped local target, mode2 local
  restamp before world-clamp failure, normal decay before invalid-mode/late
  failures, ASin/reciprocal/cvar/SinCos service ordering, threshold tie/NaN.
- Frozen Rodrigues off-diagonal FMA/subtraction discriminators and full pivot
  composition versus algebraic simplification, including nonfinite/partial W.
- Base blend P operand-order discriminator, old/candidate normalizer failure
  order, normalization despite blend>=1, primary-before-parent retained writes.
- External NaN direct copy with zero callbacks; negative blend's complete
  query; unconditional parent external interpolation for hands; second-stage
  failure retaining the primary result and both original input buffers.
- Real original Vault extent/default route and a same-owner stage chain from
  current adjusted targets through Initial/Blend into contact or DriveFrames.

There is no newly missing essential math callee. Existing VMX dot/estimate,
denormal, exceptional selection and fused-operation host limits still apply;
this source packet does not claim a measured Xbox numerical trace. Remaining
implementation work is typed math exposure and retained stage integration,
followed by separately recovered contact/drive solving rather than an invented
success at their current capability boundaries.
