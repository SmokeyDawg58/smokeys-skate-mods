# Retained IK runtime integration

2026-09-08 update: accepted **47724/092bf0** supplies the actual GroundPool
line-query child, strict3,113 units/six docs/original9 and independent review
PASS. Active IK still stops at UpdateIkContact because default World routing
needs actual DMO/conditional vehicle ownership. See
[World line integration](world-line-runtime-integration.md) for implemented
geometry and the next same-manager source-backed admission dependency.

## Accepted Initial/Blend and remaining pose kernels

2026-09-08, checkpoint **53747/c61f43** supersedes17245 below. Independent
A raw source, production, all new tests, shared integration and
original-corpus review PASS. This extends the actual active skater through
Initial and BlendTransforms. Contact and DriveFrames implementations are
registered and tested but await live World query and processed-input wiring.

`SkeletonNormalAssembly::try_update_grounded` -> current animation -> retained
IK aggregate now runs DataIn/Status/BlendValues, Initial then BlendTransforms.
The same own IK Instance supplies Initial extent, the same existing target
mapping provides hierarchy IDs and Parts, and the same previous animation
record supplies deck/primary/parent matrices. No duplicate pose arrays or
replacement physical owner is introduced. Initial services are lazy forwards
through the same General cvar provider. Source state2be still bypasses all IK;
other states return `UpdateIkContact` before the contact-enable cvar is read.

Initial clears each OnDeck entry before that entry's external/common work.
External normal rotation retains ASin -> reciprocal -> maximum-angle -> SinCos
service order, three full pivot compositions, and primary-before-parent failure
prefix. Inclusive XYZ deck bounds ignore W and preserve source NaN behavior.
Blend's board uses physical-board basis with previous-animation deck world P.
Its base position is candidate*blend fused with separately rounded old*(1-blend),
which differs from the external affine wrapper. Both input rotations normalize
before SLerp even at the upper endpoint; external hand parent interpolation is
unconditional. Source errors retain earlier completed matrices.

Typed shared SLerp uses ONE existing numerical graph and branch metadata to
preserve endpoint W, per-column linear W, and XYZ-derived Rodrigues W. Typed
classical normalization follows original Z/Y/X projections without filling
unknown W. PrePhysics reuses that implementation. Actual animation targets
implement direct Part-order raw position and current mapped-row access; a
write requiring unknown mapped W explicitly refuses at that write, preserving
prior commits. The existing retained global animation provider lazily reads
minimum/maximum angles, independently of the own IK Instance.

Contact preserves captured versus late processed flags and hierarchy bone IDs,
only two feet, ordered comparisons, separate retained push flags, unconditional
parent position offset and contact-state clear before query. Hits, misses and
service failures remain distinct. Independent ucrt fmaf arithmetic reproduced
both signed-sideways start/end snapshots, all16 words. Source end=start+down
is preserved rather than reassociating the segment around the projected point.
DriveFrames performs enable -> leg and arm limits once before mode tests,
uses actual volume-to-bone transforms, runs the real prephysics solver and
writes ancestors before primary row/world P and optional parent row/world P.
The same animation-space adjustment is added directly to retained world P.
These are reviewed leaf capabilities, not a completed World/query invocation.

New87 tests: typed affine8, Initial10, External14, Blend6, PrePhysics13,
actual row2/global attribute2, Contact19, DriveFrames13. Source packets:
`skeleton-ik-initial-transforms-source.md`, `skeleton-ik-prephysics-source.md`,
`skeleton-ik-contact-source.md`, plus the prior numerical packets below.

Workspace61157 passed **3,052 units** (2,824core+187formats+38app+3symbols)
and six docs. Core9/app1 remain intentionally ignored in ordinary workspace
checks. Focused65960/89a5d9 passed135 IK tests; typed affine17 and prephysics13
also passed2696/4d8f36. Original91882/d2fd8e passedall9. InitialClippy61157
and captured33826/55ee3a failed only lint issues: docs/complexity/length,
finite-fixture float comparisons and checked small-index conversion, literal
separators and borrowed input conventions. No runtime arithmetic graph changed.
Final all-target Clippy53747/c61f43 passed-Dwarnings; fmt/diff passed before
original91882. Documentation-only final fmt/diff follows.

The same original grounded corpus now independently verifies all initial
PartToDeck matrices, PartPos, OnDeck, parent relationships, unblended full world
matrices and blended full-position lanes on first ordinary and fourth1f9 calls.
It uses original Vault extent and previous records. Blended ROTATION columns
are not independently compared in this corpus helper; separately reviewed
numerical tests cover that arithmetic. No extra solve or invented query result.
Full Actor scheduling, native per-tick traces and complete controls remain open.

Next actual World line query needs original mesh traversal/triangle numerics
and real ground/DMO/conditional-vehicle owner publication. Default contact and
prephysics cannot be disabled to bypass it. See the line-query dispatch,
world-line-geometry and world-line-owners packets, whose source-review status
is separate from this accepted Rust checkpoint. The full playable goal remains
ACTIVE through Actor/input/history/combos and portable window/GPU/camera.

## Previous accepted constructor/data/status/blends checkpoint


2026-09-08, checkpoint **17245/0458af**. Independent source,
implementation, shared-owner and focused/corpus review PASS. This closes the
constructor and first two active IK stages; it does not close the six-stage
active IK path or the full playable goal.

## Actual producer-to-consumer path

The paired factory now constructs IK at `SkeletonIk` phase after the actual
Skeleton candidate is created and before `SkeletonRouting`. Native Skeleton
allocates its IK heap object at82b57420, calls its constructor at5743c and
stores the pointer at57448. The Rust same-Skeleton owner retains the existing
`SkeletonIkTargets` and one additional `SkeletonIk` containing missing state,
geometry, numeric DataIn and the actual own physics_skeletonik Instance.

Pool/world construction validates actual backend, board member/kind, selected
manager and shared PhysicalPlayer owner before borrowing the existing admitted
board constructor's retained global Deck provider. It does not reconstruct a
board or bind a replacement Deck Instance. IK own Instance binding, cleared
numeric DataIn and full Reset precede every geometry read. The aggregate is
retained before those reads, so a late failure preserves completed scalars and
reset state and refuses replay. Geometry has no invented values before stores.

TeleportReset now resets all represented IK histories and seven solved-matrix
arrays as well as the SAME original target transforms. It preserves Instance,
DataIn, geometry and target mapping. The old target-only component constructor
still supports isolated fixtures. The complete factory always constructs IK
before admission, registration, teleport reset and later target Init.

Active GeneralUpdate now builds DataIn from actual roots, adjusted hips,
previous AnimationRecord, general physics skateboard and collision normal.
State1f9 uses animation-to-world * previous record deck for both skateboard
copies and derives its rigid inverse; ordinary states copy the distinct root
and physical-board values. Collision XYZ and its independently unknown W are
copied after local branch math. Skeleton hips-offset+3d70 is a separate retained
constructor-zero field, preserved by reset, not post-record hook-to-hips+3d80.
No later hips-offset writer has been recovered; indirect writes remain open.

The numerical DataIn assignment precedes Status and four lazy BlendValues
reads on the own Instance. Actual row/assembly/Island relationships remain in
the same enclosing normal owner and selected world; no duplicate pose arrays,
invented handles or early line-query availability check are introduced. These
are numeric DataIn fields, not a claim to have implemented a native pointer
layout or the later World query service.

The active result now names `CalculateInitialIkTransforms`. State2be continues
to skip IK entirely and completes the existing retained drives/force/trajectory
suffix. Initial, External, BlendTransforms, contact and DriveFrames are next.
Both source contact and pre-physics cvars remain enabled by default; no default
is disabled to manufacture a successful active path.

## Reviewed numerical dependencies

- TwoBone wrapper/core829b1e30/829b1240: full cofactor inverse, angle graph,
  recursive tail behavior, source candidate/store order and typed W. 22 tests.
- XMVectorTan82581520: full reduction/polynomial/reciprocal graph, raw bounds
  masks, saturating vector conversion and callback ordering. 10 tests.
- Retained own IK Vault provider: lazy keyed index0 reads, correct missing-only
  zero fallback and malformed-present failures. Seven tests.
- Constructor geometry82b55178..55388: four actual Deck reads, separate half
  products, new extra-length plus retained half-length, rounded degree product,
  fsel bounds and tangent. Six tests.
- Reset51370, Status3db20 and BlendValues51d38: full arrays/modes, source
  independent transitions, prior hand positions, four-read prefix and asymmetric
  X/Z/Y NaN selections. 17 tests.
- Range clamp82e496d0 and line frames82b3e030/82b3e1d8: full callback/store
  prefixes, quality gates, inverse W recovery and original FMA graphs. 15 tests.
- Three aggregate tests prove constructor partial geometry with reset target
  authority, reset preserving nonidentity DataIn/geometry/Instance, and a late
  malformed blend preserving DataIn/status before any blend store.

The accepted standalone cosine DAG was moved unchanged to reckoning_sin_cos
for reuse by the solver; its existing 11 normal-displacement tests pass. Source
packets: lifecycle, tangent, blend-values, two-bone, general-ik and prephysics.
Independent reviewer recomputed frozen full-lane line-frame and clamp values
with a separate Python/ucrt fmaf graph, and checked the solver FMA discriminator.
Host seed/dot, denormal, NaN-payload, signed-zero and packed-W limits remain;
this is not measured per-tick Xbox trace parity.

## Validation

Workspace8533 passed **2,965 units** (2,737 core, 187 formats, 38 app, three
symbols), plus six docs. Core9/app1 are deliberately ignored in ordinary tests.
Its initial Clippy failure was only comment formatting, naming, two literal
separators and explicit complexity/length lint annotations. No numerical graph
or service ordering changed. Final all-target Clippy17245/0458af passed with
warnings denied. Focused IK45, aggregate3, line-frame15, TwoBone22 and prior
cosine11 all pass. Original10643/0bc438 passed all nine mounted core checks.

The same original factory now verifies geometry from actual original Deck
words and an independent bounded tangent oracle, both after admission and on
a candidate stopped at later visual-Island routing. The same solved grounded
owners verify ordinary DataIn and original own-Instance foot-blend step, then
state2be's preservation of that DataIn. A fourth component call uses actual
state1f9 and independently composes previous-record deck and inverse (irrelevant
W replaced by a sentinel in the inverse oracle). It verifies known/unknown W,
bit28 blend decay and preservation of raw/mapped rows, body state and drives.
There is no additional solve and no claim that later IK stages ran.

Native Actor, original input/history/combos, state execution and portable
window/GPU/camera remain required. Keep the persistent full playable goal ACTIVE.
