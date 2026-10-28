# Retained processed toolkit producer source packet

2026-09-08. Read-only original IDA raw/PDB audit and
comparison to existing native owners. No implementation or Cargo execution
for this packet. Confidence **exact** for raw writes/branches/offsets and
**high** for the meaningful arithmetic mapping. Native VMX estimates,
reduction order, denormals and exceptional payloads retain existing explicit
host boundaries. This does not establish an executing Actor frame.

## Source and actual placement

`?PrepareToolkitInput@Skateboard@Physics@Sk8@@QAAXXZ`,
`82b26910..82b26df0`, original bytes SHA256
`7e2bf85211c060935bcde2c34727eaa2d406658304debc8a7ef354659f6b2003`.
Only direct caller is `PhysicalPlayerHiLOD::Update1_ProcessInput` at
`82e036cc` (function `82e01f98..82e03be4`). Processed reset is `82e0200c`,
ProcessAnimOut `82e02134`, SurfacePhysics update `82e02268`. Prior PhysOut
copies precede toolkit. After toolkit come FillPhysIn_SkeletonLineTestResults
`82e036d4`, then Skeleton::ProcessData `82e03718`. Thus the later animation
attribute/collision writes are not inputs to the earlier toolkit call.

Current `skeleton_processed_contact.rs::SkeletonProcessedContact` already
owns contact ID and the source mass/deck/effective prefix. Its
`try_sample_toolkit_prefix` calls actual mass, physical deck Part6 transform,
then an independent physical deck transform inside the effective calculation,
then samples flip flags. **Extend this retained owner**, rather than adding a
second constructor/snapshot path which rereads mass or transforms. Its
`ready(flags)` currently means the bounded Contact prefix; it must not imply
that all six-force inputs or the remaining toolkit have executed.

## Prefix and inverse/axis continuation

Skateboard retained processed pointer is +110. Physical body is the actual
SkateboardBody/Assembly Part6. PDB processed matrix/field layout:

| Offset | Field |
|---|---|
| +40 | mDeckTransform |
| +80 | mEffectiveTransform |
| +100 | mInverseEffectiveTransform |
| +140/+150/+160 | mBodyXAxis/mBodyYAxis/mBodyZAxis |
| +170/+180 | mBodyZDirection/mBodyYDirection |
| +1b0/+1c0 | mZAxisVelocity/mZAxisVelocityDir |
| +240 | mJumpNormal |
| +944/+948/+94c | mZSpeed/mAbsBodySpeed/mFlippedControlsScalar |
| +970 | mSkateboardMass |

Mass commits at `2692c`; Part6 GetTransform `26948`; deck full4 columns commit
`26960/26968/26970/26978`. GetEffectiveTransform calls at `26984`, whose
independent body/late flag behavior is already covered by the Contact prefix.
Effective full4 columns commit `269b4/269c0/269d0/269e0`.

`269e8..26a4c` computes a **rigid transpose inverse**, not a general inverse
or reorthogonalization: transpose effective XYZ axes, generated basis W=+0,
translation negation uses arithmetic `+0-position` (not bit-negation), then
negativeP.z * transposedZ, fused negativeP.y * transposedY + prior, fused
negativeP.x * transposedX + prior, all four result lanes. Stores inverse Z
at `26a38`, Y `26a3c`, X `26a40`, translation `26a4c`. Existing
`skateboard_spawn_math::skateboard_spawn_rigid_inverse` has this graph. A
typed wrapper can use unobserved input W scratch because transposition and
translation coefficients consume XYZ only; output inverse basis W is proven
zero, while translation W must still execute zero-axis products/FMA and can
be exceptional for exceptional position. Do not fake the physical deck W.

**Axes come from physical deck, not effective transform.** Read deck Y+50
and store processed bodyY at `26a50/26a54`, read deck X+40 and store bodyX
`26a58/26a5c`, read deck Z+60 and store bodyZ `26a60/26a64`. This preserves
Y/X/Z source order. Late flags word+8c4 is read at `26a68`; mask00100000
selects scalar -1 if set, otherwise +1. Store scalar+94c at `26a88`.
BodyX full4 multiply scalar commits `26af8`; read retained scalar again
`26b00`; bodyZ full4 multiply scalar commits `26b24`. These are multiplications
by +/-1, not sign-bit edits. BodyY is not flipped here.

## Direction normalization and exact partial writes

1. Copy flipped bodyZ to scratch, replace **only Y with +0** at `26b30`;
   preserve X/Z/W. Dot3(flat,flat) at `26b3c` occurs BEFORE either bodyZDirection
   store. A numeric failure here retains already written axes/scalar but the
   previous direction. Store original full flipped Z to direction+170 at
   `26b40`, then flattened full4 at `26b44`.
2. `26b48..26b58` compares replicated squared length ordered-greater than
   bits3727c5ac (approximately 1e-5). Only true executes another dot3 of the
   flattened vector `26b64`, rsqrt estimate `26b70`, TWO original fused
   refinements, then full4 normalization store `26ba0`. Equality, less and
   unordered leave the flattened vector; they do not select a zero fallback.
   This is **not** the existing Math::NormalizeSafe epsilon graph. Reuse the
   unsafe finish `normalize_from_squared_and_estimate` after the exact guard
   and second dot. On second-dot/estimate failure the flattened store survives.
3. Read completed bodyZDirection at `26ba4`, then original physical bodyY at
   `26bac`; dot3(bodyY,bodyZDirection) `26bb4`. Product direction*projection
   `26bf8`, separate bodyY-product `26bfc`, store raw bodyYDirection+180
   `26c04`. Then dot3(raw,raw) `26c08`, rsqrt estimate `26c0c`, TWO refinements
   and unconditional full4 multiplication/store `26c3c`. No epsilon test,
   zero fallback or finite guard. Degenerate bodyY may yield NaNs. A numeric
   failure after the raw store retains raw bodyYDirection.

Reuse the shared numeric primitive/finish DAGs, retaining typed W. Unknown
input W remains unknown through normalization; setting Y to +0 does not
establish W. No duplicate numerical graph is necessary.

## Retained speed and sign-derived outputs

Read processed mZSpeed+944 at `26c40`, multiply full **bodyZAxis**, not flattened
bodyZDirection, by it, store mZAxisVelocity+1b0 at `26c64`.
Read mZSpeed AGAIN at `26c68`, compare with +0. Raw branch `4099000c` at
`26c70` is BO4/BI25, GT-not-set selecting -1, including unordered. Ordered
positive selects +1. Read full bodyZAxis at `26c8c`, compute source speed*sign
`26c94`, store mAbsBodySpeed+948 `26c9c`; multiply bodyZAxis*sign and store
mZAxisVelocityDir+1c0 at `26cc8`.

| zSpeed | sign | absBodySpeed ordinary source result |
|---|---|---|
| negative | -1 | positive |
| -0 | -1 | +0 |
| +0 | -1 | -0 |
| positive | +1 | unchanged |
| NaN | -1 | NaN with source multiplication semantics |

Do not use `abs`, signum, a single cached zSpeed load, or normalized flattened
Z for these outputs. The speed is earlier published motion, not a fresh
body-velocity dot recomputed by toolkit.

## Board jump-normal tail and cvar dependencies

Although the six force leaves do not directly consume processed JumpNormal,
source toolkit continues through this retained board write and debug gate.
Skateboard+ c0 is mJumpNormal, separate from processed+240.

Capture `kGroundMaxCosAngleDelta` target at `26ccc` before reads/dot: load
processed groundNormal+1d0 `26cd0`, retained board jumpNormal `26cd4`,
dot3(oldJump,currentGround) `26cd8`; read captured cvar float target+d8
`26ce0`. Raw `41990088` at `26cec` BO12/BI25 branches only ordered GT to
unblended current ground. Otherwise, including unordered, capture/read
`kGroundProportionalBlend` target/value `26d0c/26d24`. Compute 1-blend
separately; currentGround*blend `26d5c`, then
oldJump.mul_add(1-blend,currentProduct) `26d60`. NormalizeSafe at `26d68`
uses actual full4 zero fallback created before the call, retaining its
existing conditional numeric graph. Result replaces current-ground candidate.
Store board jumpNormal full4 at `26d78`.

Capture `kDrawJump` bool target at `26d80`, read target+cd `26d84`. If true,
require actual DebugOverlay singleton at `83059210`; missing performs source
assert/break and a second existence check on resumed execution. There is no
actual drawing call in this compiled tail. Only afterward read board
jumpNormal again `26de0` and store processed jumpNormal+240 `26de4`.
A failed cvar/normalization call leaves prior direction/speed writes; an
unavailable requested overlay leaves the new **board** jump normal but old
**processed** jump normal. No fake default board jump normal is authorized:
its actual construction/retained state remains required for this tail.

| Cvar | Ref object | Target capture/value sites | Initializer / original default |
|---|---|---|---|
| kGroundMaxCosAngleDelta | 8309d994, target+28 | 26ccc/26ce0 | 82f7b318..b378; bits3f6f5c29 (.935), range[0,1] |
| kGroundProportionalBlend | 8309d920, target+28 | 26d0c/26d24, conditional | 82f7b378..b3d8; bits3dcccccd (.1), range[0,1] |
| kDrawJump | 8309d970, target+20 | 26d80/26d84 | 82f7b228..b274; false |

All belong to Physics_Skateboard. Runtime reads remain live staged target
reads, not hoisted constants; initializer defaults are evidence only.

## Six-force inputs: actual provenance and missing producer bridges

| Force input | Actual producer before use | Source evidence / remaining boundary |
|---|---|---|
| physical mass, deck right, bodyYDirection, bodyZDirection, absBodySpeed, ZAxisVelocityDir | actual toolkit prefix/continuation above | Extend existing retained Contact producer |
| velocity+190 | prior PhysOut_SkateboardMotion.mDeckVelocity+50 | Update1 copy load/store82e02638/63c |
| surfaceVelocity+1a0 | prior motion.mSkateboardSurfaceVelocity+60 | 82e02674/678 |
| zSpeed+944 | prior motion.mSkateboardZSpeed+a8 | 82e026dc/6e0 |
| surfaceSpeed+96c | prior motion.mSkateboardSurfaceSpeed+a4 | 82e02710/714 |
| processed groundNormal+1d0 | prior PhysOut_Ground.mLastValidGroundUpVector+60 | 82e02a24/2a28 |
| processed upVector+220 | prior PhysOut_SystemReckoning.mUpVector+50 | 82e029b0/29b4 |
| actual Reckoning ground+1f0 and up+1b0 | retained board Reckoning, its own update phase | Do not substitute above processed/PhysOut fields |
| timeDelta+93c | raw AnimOutPhysIn.mTimeDelta+288c | ProcessAnimOut82e00060/68 |
| wheelHardness+9c8 | raw AnimOutPhysIn.mWheelHardness+2a18 | ProcessAnimOut82e00108/110; not prior motion hardness |
| toolkit flip flag8c4 mask100000 | raw AnimOutPhysIn.mBoardIsFlipped+2880 | ProcessAnimOut82e00050/58; toolkit late read26a68 |
| stateTimer+974 for slide | retained PhysicalPlayer state timer+14c | Update1 82e023d0/d8 |
| balance+9a0 | ordered animation scalar attributes; BalanceAtt value bit-negated | ProcessAnimAttributes82b5a9f8/9fc/aa00 |
| footBrake+9a8 | ordered animation scalar attributes, raw scalar copy | 82b5aaf0/aaf4 |
| push/foot/manual flags and other input scalars | current animation attribute/state processing | Existing named-attribute interpreter; no direct keyboard substitution |
| selected surface+904 | SurfacePhysics::Update/actual prior collision enum dispatch | separate skater-processed-surface-source.md and retained token owner |
| collision flag8c8 mask40000 | actual SkeletonBody(+1930).mSkeletonCollision(+a0).mCompliantJustCollided(+fbe) | **later** Skeleton::ProcessData82b5f294/f298/f29c |
| collision hookToHips+2e0 | retained Skeleton.mHookToHips+3d80, only if actual collision.mIsHookToHipsValid+fc6 | **later** 82b5f2a8/f2b0/f2bc/f2c0; invalid leaves prior/reset field |

Each prior PhysOut getter asserts its owner is not locked for write (+4c)
before reading the corresponding subrecord; copied vectors retain all four
lanes. These are not replaceable with a freshly queried ground hit or current
body observation. Prior Skateboard::FillPhysOut publishes motion deck
velocity at `82b2f788`, then its distinct surface velocity/speed sequence;
zSpeed publication uses effective Z dot published velocity `82b3016c..301a0`.
Those full publication phases are separate from this bounded toolkit audit.

`Skeleton::ProcessData` `82b5ecc8..82b5f348` calls ProcessAnimAttributes at
`82b5ed44`, after toolkit in Update1. It later updates orientation flags at
`82b5f288` as well as collision flag/hook above. Do not collapse all phases
into one early immutable animation snapshot: toolkit uses the earlier
ProcessAnimOut flip flag; consumers may observe later current processed flags.
The actual input-history/controller/animation path must feed named animation
attributes; this packet establishes no direct key-to-force rule.

## Suggested bounded integration and validation

Add optional retained inverse/body-axis/body-direction/speed fields to the
existing processed Contact owner, with separate continuation readiness. Begin
from its already committed physical deck/effective matrices; preserve source
late flags and repeated zSpeed reads through staged services. Keep board jump
normal in its actual board owner; a tail service can commit it separately
before the overlay gate. Attach prior motion/ground/system-reckoning copies
and later animation/collision writers to the same processed cycle, rather
than constructing a second force-only owner with convenient values.

Focused validation should exercise: nonidentity physical deck versus effective
flip, X/Z +/-1 full-lane multiply, first-dot failure before direction writes,
second-dot failure after flattened write, tiny/equal/NaN squared Z branches,
unconditional degenerate Y normalization, +0/-0/NaN zSpeed table, live repeated
reads, and board-versus-processed jump-normal failure prefix. The current
original body/force corpora can supply real admitted Part6 state, but prior
PhysOut publication and raw attribute/controller scheduling must be identified
explicitly before claiming complete six-force live inputs.

Additional original fingerprints:
ProcessAnimOut82e00030..82e00130 SHA256
`47b05bda97fddb1ca398359dd829490421a2361c3b7e3d991404c2ccf1a4b620`;
Skeleton::ProcessData82b5ecc8..82b5f348 SHA256
`d6e8b6b51091283dfbd4be0df3c86e8e8487b569a3317aeb00bff755a0dca983`.
No source executable or original data was copied into this repository.


## Authorized retained-owner continuation implementation

`SkeletonProcessedContact` is extended through the separately owned child file
`skeleton_processed_toolkit.rs`, with `SkeletonProcessedToolkitState` retained
as the parent `toolkit` field. Root owns parent registration, initialization,
and invalidation at reset/start-of-prefix. The child method
`try_continue_toolkit_directions(input,numeric)` consumes already committed
physical deck/effective matrices after prefix readiness, and never rereads
the body/mass. Input services provide source-stage current flags and two
separate prior-motion zSpeed reads. State getters expose retained partial
fields; `directions_complete` becomes true only after the source26cc8 store.
It does not imply the jump-normal tail or a complete toolkit/frame.

Inverse basis stores are retained Z/Y/X/P; direction failures preserve the
exact earlier raw/flattened fields. Shared rigid inverse and unsafe
normalization finish perform the numerical graphs. Invalidation changes
readiness only, without inventing reset writes to numerical fields whose
native Reset behavior is not part of this continuation. Ten focused tests
cover physical/effective distinction, repeated-read stages, partial failures,
threshold/unordered branch, unconditional degenerate Y normalization,
signed-zero/NaN speed, unknown W and readiness invalidation. Author performed
rustfmt only; root owns compilation/tests and independent reviewer audits
source fidelity. Board jump-normal tail has a separately gated continuation described below.

## Retained board jump-normal constructor proof

Root and independent reviewer separately verified the raw outer
Skateboard constructor. `lis` at82b3898c/38994 and signed `lfs` displacements
at389a0/389a8 resolve actual data addresses8220a90c/8220a908, containing words
00000000/3f800000 respectively. Stack stores38a3c/38a40/38a44 write XYZ0/1/0;
38a48 writes r30=0 to stack+bc (pointer389d0). Pointer38a8c selects stack+b0;
full vector load38ae8 and store38af0 with r17=c0 (38a70) establish the actual
retained board+c0 initial vector [0,1,0,0]. This precedes queue initialization
and own physicsdeck binding. Rust retains this value in the existing bounded
outer-board runtime; the toolkit board trait reads/writes that same owner.
These are source constructor constants, not substituted Vault tuning.


### Actual board jump-normal construction and tail continuation

Root recovery and independent reviewer confirmed actual Skateboard constructor
full4 jumpNormal+c0 initialization: lfs constants resolve to8220a90c raw00000000
and8220a908 raw3f800000; local XYZ stores82b38a3c/40/44 and W zero store38a48,
then load38ae8 and board+c0 store38af0 establish `[0,1,0,0]`. Root owns this
field in the actual `SkateboardForceRuntime`; no toolkit-owned substitute
normal is introduced.

The same child now provides `try_continue_toolkit_jump_normal` after
`directions_complete`. Separate typed traits read actual processed ground,
actual retained board normal and staged live cvar targets. Max-cos capture
precedes input/board reads and dot; max value follows dot. Conditional blend
uses separate current product followed by fused old product and shared typed
NormalizeSafe with actual zero fallback. Board setter precedes draw capture,
bool read and conditional overlay requirement. Final board getter follows
the gate; only its result commits processed jump normal and marks
`jump_tail_complete`. Failed calls retain earlier board/processed prefixes.
No cvar initializer values enter the runtime helper as constants.

Eight additional tests (18 toolkit tests total) cover read/capture order,
unblended branch, unavailable directions, numeric failure before board write,
draw/overlay failure after board write, authoritative final board reread,
unordered comparison, zero fallback versus unknown W, and fused cancellation
observed at the normalization dot. Root owns compilation/runtime binding;
the author ran rustfmt only. Completing these bounded toolkit continuations
still does not establish the external prior-PhysOut and animation schedule.

Validation update: accepted18370/1eaaa4, workspace3241 units/sixdocs,
original73431/58c467 eleven PASS, strict Clippy/fmt/diff PASS. Independent
A review passes the integrated bounded owners/corpus. The empty
contact test uses a separate fresh construction, preserving the preexisting
positive-contact fixture. Actual Actor/PhysOut scheduling remains open.
