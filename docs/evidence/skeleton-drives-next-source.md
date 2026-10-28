# Skeleton drives, shared hooks and full retained-pose Set

## 2026-09-08 accepted current-target/drive feedback (checkpoint60267)

This supersedes the historical NEXT Update wording later in this document.
Confidence **high** for the reviewed source graph and controlled integration;
full gameplay targets/GeneralUpdate and native numeric traces remain open.

`skeleton_animation_targets.rs` implements ProcessData's mapped-pose loop,
keeping raw animation and volume rows on the existing SkeletonNormalAssembly.
Unwritten rows are None. Each actual boneIds lookup/copy precedes composition
through the same retained boneToVolume, with full4 source arithmetic and no
normalization. A short subsequent pose retains the updated prefix and old
suffix but cannot report completed mapping. These are explicitly CONTROLLED
UNADJUSTED current targets, not the final IK/adjusted buffer or physics history.

`skeleton_drives_update.rs` implements the source Update graph below directly
on existing SkeletonDrives/bone/hook payloads. Inverses1..23, updates1..22;
current global attributes, partial mask, strict strength<1 (NaN normal), source
FMA/product ordering, flags/strengths and lazy per-drive services are preserved.
Child frame stores precede parent numeric callbacks. Soft damping failure leaves
completed frames/row strengths but no dynamics changes. Source normal-follow
ignores argument strengths and reads captured layout offsets8,12,4,0. No payload
lock spans a callback. Native concurrent observers and exceptional-result/FP
status behavior remain outside the explicit host numeric/locking policy.
Inverse scratch0 is unwritten; a source read of it is a typed error, not identity.

`skeleton_drive_attributes.rs` now implements current update lookups separately
from constructor physics_skeleton_drives lookups. The EXACT provider used by
paired construction, including its selected Local/Root cached owners, moves into
SkeletonDriveRuntime immediately after successful drive admission. It is not
rebound to fresh slots. Every normal drive captures its selected retained layout
once, then loads its four scalars. Genuine missing global values get Default4/1;
found wrong-width/malformed values and a real nonnull collection with null cached
layout are errors. Immutable original graph/global replacement limits unchanged.

World method try_update_skeleton_drives_from_controlled_animation_pose validates
current completed frame/feedback/list, then SAME-owner collision completion and
exact frame. Retained runtime rejects replay, missing selected attributes, wrong
data/authority and incomplete admission/release. No-lock session/backend/pool
preflight verifies exact live occurrences, endpoints and Arc identity before
source writes. Native null hooks remain allowed and distinct from released
cross entries; bone admission cannot Complete after a null because its original
unconditional spy write is already a typed stop. Partial target/drive progress
is retained. Geometry/spies are not invalidated by frame/dynamics-only changes;
the NEXT synchronous normalize/Build/solve reads those exact payload allocations.
No already-copied prepared batch is silently refreshed.

Original positive-contact corpus now supplies original current pose with an
explicit small bone7 XY offset (+1/64,+1/32), preserving W. Independent literal
24-bone mapping and full4 equation check every mapped row; current target is
proven different from constructor pose. Actual collision strength0 drives all
44 eligible bones to soft mode, with exact parameters derived from original
Vault scalars/body coefficients and controlled current damping0.75. Per-drive
44 damping reads, SAME frames/dynamics Arcs, all49 Drive occurrences/endpoints,
unchanged bodies, Hook0 angular/Hook1 and replay-before-cvar behavior checked.
The NEXT real50-iteration terrain step still has28 joints/49 drives. Test-only
capture of its actual prepared Horus records proves each bone's linear
coefficient row2.W changed hard1->soft0; row3.W matches original soft Build
equation under audited host reciprocal refinement. Horus preserves these W
fields. Full Drive snapshot closes identity/ABA/order, not row12's slot ID alone.
Next frame without new collision response refuses before cvars/payload writes.
Existing teardown and all earlier collision oracles remain intact.

Strict60267/d09ffe EXIT0:fmt/check,2421core+187formats+38app+3symbols=2649unit,
6doc, all-target Clippy-Dwarnings, ALL7original core tests and diff-check.
New18unit:9 drive update,3 mapped target,3 provider,3 live guard. Independent
source/implementation/test/corpus review PASS. Tests include literal partial
mask, rational fused-result b22aaaa8 vs unfused0, overflow multiplication-order
discriminator, parent failure after child commit, late damping/poison prefix,
inactive-row parent validation, permuted bone mapping, old/new/null cached
layout capture and actual occurrence/endpoint/Arc validation without locks.
Earlier focused53805 passed actual next solve and board regression, then one
post-format too-many-lines lint; narrow structural expectation fixed before60267.
Prior DMO hash/positions and RobPark board trajectories are unchanged. No app
ignored probe rerun, native per-tick comparison or hands-on input readiness claim.

### Next actual hook/transform dependency

UpdateHookPositions82b4d380..82b4d600 SHA256
c94377d2ada951ca786f7e13e476b9cc97e5fcfa91625bc30db5361222689353.
GeneralUpdate calls it before IK/drive Update. It requires CURRENT AnimToWorld
(Skeleton+2d80), adjusted HipsTransformAnim(+2d00), adjusted SkateTransformAnim
(+2cc0), InverseSkateboard(+2f40), SkateRootTransform(+3c80), teleporting byte.
Targets-only cannot supply these. Hook0=AnimToWorld composed with adjustedHips;
Hook1=SkateRootTransform (not mapped target0). First write AnimBoardToPhysicsBoard
(+2ec0)=InverseSkateboard composed with (AnimToWorld composed with adjustedSkate).
Then read OLD Hook0 transform and form oldHook0.P-newHipsWorld.P; set Hook0,
set Hook1, store proxy target from SkateRoot full4 P. Only afterward read teleport
byte and dot3(delta,delta). Raw d5e8 is ordered>1 failure, so NaN distance passes
this gate unless teleporting; decompiler <=1 would be wrong. False at GeneralUpdate
bb2c requires ResetSkeleton, possibly wantRespawn, before later IK/drives.
Without those genuine producers/reset path, return a declared post-write boundary
instead of continuing as if normal. Current checkpoint does not implement this
operation or assume static constructor hooks reproduce it.

### Selected next grounded producer route (read-only, not implemented)

Immediate caller audit selects grounded rather than animated-board routing:
PhysState_PhysicsGround::Update82da0de8 calls Reckoning::UpdateGroundStates
82de0408 at82da0e94, mandatory UpdateHandplants at0e9c, then Skeleton::
UpdateGroundSkateboard82b5e468 at0ea8. Joining only first/last is NOT the whole
state update. GroundStates takes current ProcessedPhysIn groundNormal+1d0,
dynamicUp+210 and deckTransform.Z+60; stores retained Reckoning dynamicUp+1a0
and heading+1e0 then calls UpdateGroundStatesFixedHeading82ddd468.

UpdateGroundSkateboard sets updateHeading1 and calls UpdateRootTransforms
82b42938 at82b5e490. Later actual paired board Body::GetPosition(6) is sampled
AGAIN at82b5e984, and full16 live matrix copies to PhysicsSkateboardMtx+2fc0
and SkateRootTransform+3c80 BEFORE GeneralUpdate82b5e9d8. This supplies Hook1
from the actual current deck, avoiding animated path ApplyBoardAnimation.

UpdateRootTransforms first samples deck6 at82b429a0, computes inverseboard,
optionally predicts board P from fresh velocity*dt+P and clears predicted-valid.
Heading work uses CURRENT SkateTransformAnim.Z (+2ce0) projected horizontally, signed
AngleBetweenVectors and SinCos. AnimToBoard uses retained Reckoning.transform
and negative adjusted SkateTransformAnim.P; predicted-board translation composed
with AnimToBoard yields AnimToWorld, followed by original OrthoNormalizeRotation
and final WorldToAnim rigid inverse. Source initialization/prior state is real
required input, not implied by current raw pose. Full raw branch/service/store
packet must precede implementation. The completed raw packet is now in
`reckoning-grounded-transform-source.md`; it corrects the earlier mistaken
prior-AnimToWorld heading attribution. Predicted position W is unknown native
packed velocity storage, not a proven zero fourth lane.

Missing concrete producers include Reckoning/BodySpin/PID state, initialized
root-transform history, current processed-input fields and adjusted hips/skate
matrices. Existing Rust provides actual board and HookPool owners, full4 affine
composition/inverse/normalization, Vault resolution and PointGraph infrastructure,
not a live Reckoning owner. Reconstruct this grounded producer-to-hook path;
do not add six independent guessed transform parameters and call it gameplay.
Reset-on-hook-failure, handplants and IK stay explicit full-lifecycle requirements.

2026-09-07. Source and implementation evidence, **high** confidence. The retained
SkeletonDrives producer now connects actual two-Part HookPool admission, hook and
eligible bone drives, full retained-pose Skeleton::Set and ordered release through
the SAME manager/backend and complete skeleton owner. This is not a completed
gameplay service or a deterministic Xenon numeric trace.

## Implemented ownership and boundaries

`skeleton_drives.rs` retains the exact SkeletonData Arc, 24 bone payload pairs,
two hook payload pairs and the actual two-Part detached hook session. The concrete
`skeleton_drive_attributes.rs` provider performs repeated original Vault reads;
the mounted observer checks source chronology and raw words rather than replacing
runtime reads with those oracles. Its two cached instances are construction-local
within one immutable Vault, not the native mutable global/editor lifecycle.

`dynamic_assembly_skeleton_drives.rs` installs actual partial ownership before hook
admission. The same HookPool retains correlated whole Existing/Skeleton candidates;
the board's template/session is unchanged. Hook insertion uses shared capacity,
manager indices and Static1 bodies. Bone drives are admitted through the actual
session ledger and checked spy1 store. Native nullable hook results proceed; native
bone null-dereference becomes a typed host stop with the completed prefix retained.
Explicit cleanup also handles those partial failures; no implicit Drop cleanup is
claimed as native behavior. Selective removal follows hooks0/1, hook Assembly and
geometry, then Parts23..0 with LOCAL before LOCAL_ROOT. Live cross-drive guards
prevent early detachment and disappear only after successful actual removal.

Full retained-pose Set runs the existing immediate 24-Part setters, then the two
source root-relative hook setters. The full four-lane proxy target belongs to the
retained Skeleton state and survives drive release. The mixed skater pool and
manager expose narrow initialize/full-Set/release methods, invalidating collision
publications before potentially partial mutations; no mutable session escapes.
The older manager-only 24-Part placement wrapper is superseded by full Set; its
lower-level prefix remains tested and is reused by the actual owner.

Tests cover six producer cases, five concrete-provider cases, two actual-owner
lifecycles and one actual drive-pool spy ownership regression. Original-data mixed
coverage now admits board and skeleton hooks in one pool: 35 actual bodies,
28 joints and 47 drives (one board hook plus 46 skeleton drives). This controlled
case has no truck drives and uses zero solver iterations. It validates shared
ground/constraint preparation, endpoints, Vault reads, placement and cleanup, not
humanoid terrain contacts, solved humanoid dynamics, live player input or skating
feel. Cvars1/10 remain explicit harness samples; live cvar ownership, mutable pose
production and source estimate/trig numeric traces remain open.

Independent source/production/test review passes. Strict validation results are
recorded at the end of this document.

## Constructor and layouts

SkeletonDrives constructor `82b59b08..82b59c60` retains actual owner, Island,
SkeletonBody and SkeletonData, sets ragdoll strength +0, initializes both hook
frame pairs (quaternions [0,0,0,1], all position lanes +0), then initializes both
hook dynamics (all scalars +0, both types NO_DRIVE0). It constructs 24 BoneDrives
ascending, clears 24 IK-active bytes ascending, calls InitDrives `82b50770`,
then InitHooks `82b50868`.

SkeletonDrives offsets: owner00, Island04, SkeletonBody08, SkeletonData0c,
ragdoll strength10, hook Drive pointers[2]14, frames[2]20, dynamics[2]a0,
hook Assembly e0, definition e4, BoneDrives[24] f0 (stride110), IK bytes1a70.
BoneDrives offsets: frames[2]00, dynamics[2]80, parent IDs[2]c0, active[2]c8,
ONE shared flags cc, strength[2]d0, collision soft disp/strength d8/dc,
ragdoll soft disp/strength e0/e4, actual child Body e8, parent Bodies[2]ec,
actual Drives[2]f4, SkeletonData fc, Part ID100, parent BoneDrives104, Island108.

BoneDrives constructor `82b50f40..82b511cc` initializes frames/dynamics as above,
child Body/Island null, Part ID -1; calls InitAttribs; clears active/parent-Body/
Drive/strength fields and shared flags. It does NOT initialize all remaining
pointer/parent-ID fields: retain absent semantic entries rather than defaults.

## Runtime attribute timing

Each of 24 BoneDrives constructors calls InitAttribs `82b48890`: class
animation_drives `3432360e2c7d1d8c`, LOCAL collection `00132d1e84f0bc32`, then
LOCAL_ROOT `550b23128e0c3838`. Each is FindCollection -> Instance::Change.
Static initializer `82f7d7b0` constructs two null instances with DefaultDataArea16
when their initial cached layout is null.

Instance::Change `82976260`: identical collection pointer is complete no-op;
different means Unmodify, release old, store new. A nonnull new collection is
retained and supplies the new cached layout. A null new collection LEAVES THE
PREVIOUS cached layout. No generated fallback follows successful nonnull change
to a collection whose layout is null. An explicitly immutable single-Vault
construction owner can retain two real instance bindings and repeat resolutions
at all 24 calls; it cannot claim full shared mutable global rebinding.

After InitAttribs, every BoneDrives constructor performs four immediate index0
scalar FETCH/load/store calls against physics_skeleton_drives class
`fdf618600d396c3b`, default collection `d7edbd362d7d2152`:

| Key | Field |
|---|---|
| e1a7f2dcfab0b8d0 | collisionSoftDisp |
| f300ffd599e1709e | collisionSoftStrength |
| f508d452b962f68d | ragdollSoftDisp |
| 15a26b6143e7df97 | ragdollSoftStrength |

Missing pointer uses DefaultDataArea4. These calls repeat 24 times, not one
snapshot. Cached animation-drive layout16: linear strength00, displacement04,
angular strength08, displacement0c. Original mounted words were independently
extracted below. They are test oracles, not runtime tuning constants; the
concrete provider must still perform these reads.

### Original mounted oracle

Actual root VLT collection89b8/layoutslot89e0 maps BIN2cbe8; local collection89f8/
layoutslot8a20 maps BIN2cbf8. BOTH selected cached layouts contain words
`3f800000 3f800000 40800000 40800000` (linear strength/disp1, angular strength/disp4).
Both have zero explicit nodes and parent default, but OWN nonnull cached layouts;
they must not inherit default BIN2cbd8's sixteen zero bytes instead.

Global drive default collection is VLT5e088. The four scalar words in source
order are VLT5e0e0=46c35000 (25000), 5e0f0=42c80000 (100), 5e110=47c35000
(100000), 5e120=42c80000 (100). These are explicit inline nodes with flags40,
float type index1 and four-byte definitions. Generic GetAttributePointer(key,0)
selects them directly, NOT the class's 736-byte generated cached layout.

Read-only reproduction used existing inspect_vault.exe on the external extracted
skater.vlt for collection metadata, then the existing skate2-formats.exe on
original db.big to independently verify BIN offset183256 length48 and VLT
offset385160 length230. No assets were extracted into the repository and no
new runtime provider was executed during that initial read-only extraction.
The implemented mounted observer now adds both classes through existing
mounted_physics_attributes_with_classes and exercises the concrete cached16 and
ordinary scalar try_get_data(key,0) routes with the repeated source chronology.

## Payload preparation

InitDrives `82b50770..82b50864` visits Parts0..23, skipping only parent -1.
For each actual parent, SetUpDrive `82b48ad8` is called LOCAL0/flags2/actualparent,
then LOCAL_ROOT1/flags0/Hips23. Both consume retained physics-pose matrices.
SetUpDrive retains data, Part and Island, stores selected parent ID and active,
writes the SINGLE flags field, constructs rigid transpose inverses of parent
and child matrices, then SetDriveFrames and EnableDrive(type,1).

SetDriveFrames `82b3fa90` uses identity rotation at the child pose's translation,
NOT its rotation. Child frame = inverseChild composed with this frame; parent
frame = inverseParent composed with this frame. Full-lane affine multiply/FMA
order is the accepted X/Y/Z graph. SetChildFrame precedes SetParentFrame. Reuse
the actual rigid inverse, compose and frame-quaternion candidate helpers, not
a general inverse or additional quaternion normalization.

Initial LOCAL dynamics: flags2, angular HARD2, spring = LOCAL layout+0c times
bits426fffff, strength = +08 times4560fffe, damping0; linear HARD2 with allzero
parameters. LOCAL_ROOT flags0: both HARD2, corresponding displacement times
426fffff and strength times4560fffe, damping0. The second call leaves shared
flags0 WITHOUT rewriting prepared LOCAL dynamics. Normal-follow raw read order
is +08,+0c,+04,+00; angular strength multiply precedes +04 load, angular spring
multiply precedes +00 load. Preserve float32 products and sampling order.

## Real hook construction and admission

InitHooks `82b50868..82b50d04` allocates definition2 Parts/0 joints/0 drives.
For Part0 then1: actual Box half-extents bits3dcccccd each (0.1), fresh transform,
radius0; clear enabled bit only; install volume; ComputeMassProperties mass
42c80000 (100), density1, null output; ParentFrame identity then ChildFrame
identity. PartDefinition static word stays default0; default material .5/.3/.4.
Reuse fixed box kernels, not the whole differing board hook constructor.
The actual Assembly Part transforms independently start identityXYZ/zeroP/allW0,
not all sixteen lanes zero: Part::Initialize82ae8fc8..82ae9058 stores I at9020,
four-zero P at9024, J at9028 and K at902c. Live globals8227e8f0/e900/e910 contain
the corresponding unit axes. InitHooks creates the Assembly at82b50a94 through
wrapper82ae9820/BL9844 and performs no transform override before admission.
The test includes literal sixteen-word expectations independently of the shared
fresh-Volume constant. Function SHA256:
32096d0f7dbc1bb379cd9f18c10767366ae824495516c9366690877d48371c6f.

Hook0 linear then angular HARD2: spring4415ffff, strength470c9fff, damping+0.
Hook1 samples current cvars Disp -> Strength -> Disp -> Strength, linear then
angular: spring Disp*426fffff, strength Strength*4560fffe, damping0/type2.
kALC_HardDriveDisp initializer82f7d588 defaults1; Str82f7d5e8 defaults10 under
Animation_Locomotion_Collision. Defaults are not hardcoded runtime samples.

Create actual Assembly, group1/all userdata1, then Static1 HookPool insertion
at82b50aec. Add hook0 parent -> Hips23 child at82b50b30, store nullable result
b34; hook1 parent -> Root0 at82b50b7c, store nullable result b88. Then visit
Parts1..22 ascending, skip parent -1, retain child Body/Island and create LOCAL
from actual parent, then LOCAL_ROOT from Hips23. Shipped 22 eligible bones yield
46 drives including hooks, but retain the conditional producer.

CreateDrive `82b48c30` removes prior selected nonnull drive, stores parent Body,
calls Simulation::AddDrive, stores result and immediately writes spy1. A null
capacity result here is a native dereference fault, not successful absence.
Typed host stop must retain previous admissions. Hook calls only store nullable
results; later native Release nevertheless assumes nonnull. Failed construction
cleanup needs an explicit host policy, not claims of native null-removal safety.

## Complete Set suffix and teardown

After 24-Part placement: recompute Hips23 then Root0 volume matrices at
82b4ca3c/ca50; construct identity-axis outer frame whose translation is FOUR-LANE
+0 minus root.P; compose it with Hips and Root separately. Do not merely subtract
XYZ or force root translation0. Place hook Part0 then Part1 at82b4cbcc/cbe4 and
copy final root-hook translation to proxy target Skeleton+2c80 at82b4cbf4.
No extra Gram-Schmidt pass is present. Independent raw suffix audit confirms
this graph and a critical W distinction: affine compose ignores inner.P.w, so
root-relative P.w retains -root.P.w plus the actual zero-axis FMAs. It is NOT
root.w minus root.w = 0. Preserve all four proxy-target lanes at cbf0/cbf4.

Destructor `82b50720`: Release `82b47ce0`, then BoneDrives Part23 down to0.
Release removes hook drive0 then1, removes HookAssembly, releases actual Assembly,
releases each hook Volume ascending then definition. BoneDrives destructor
`82b487f8` removes LOCAL0 then LOCAL_ROOT1 if present, clears parent Body/Drive/
active. This is reverse Part order with forward pair order, NOT forward admission
or globally reverse drive order.

## Shared architecture work

Keep one HookPool. A correlated internal whole-member variant can retain the
existing candidate<M,C> unchanged alongside skeleton candidate<M,SkeletonHookDefinition>.
Do not replace existing board template Arcs or create a second pool. Dispatch
common handle/dense-order/bounds/exclusion internally. Both insertion routes use
the same manager counter, authority and capacity. Board-specific setters reject
skeleton hooks. Existing hook exclusion already visits every actual Part.

Implemented narrow additions: hook-parent -> skeleton-child cross-drive insertion;
hook Part0/1 in-place setters without mutable session escape; actual Drive spy1
setter; selective source-ordered post-drive removal (the older flat forward cleanup
alone is insufficient); complete SkeletonNormalAssembly retaining its SkeletonDrives
and real hook membership capability. Existing incoming-drive detach guards and
payload Arcs must remain. Root owns shared pool/session seams and integration.

## Function hashes

```text
InitDrives 82b50770..82b50864 cfa6934cf46f0e02443daa73ae82da66a43b0c6d680f6f1524a5faf04887a5a9
InitHooks 82b50868..82b50d04 5a56ec6aa7b7df56ebed795a9ad7b8d3bb2a23e01396c3f176f99d44856eabbf
BoneDrives ctor 82b50f40..82b511cc 76c024dfd6d1e833696ed3cf59461285e2fa6ecaed79754331148656f96c1654
SetUpDrive 82b48ad8..82b48c2c dddd3399984666b93cbf4ab86fcbc8ae62f025118abd00d0aaa1dee044ea3f3e
SetDriveFrames 82b3fa90..82b3fbe4 9bda3467558a2f072116ff376552585f4c74df3fd96273eb1277053a6606ea28
InitAttribs 82b48890..82b4892c 45937e7b7fe693c6f0dd4aceef67ddd9137e1c99e5aafc0076f594466d9e0179
EnableDrive 82b48930..82b48ad4 087eba1c74f2ca2454e7354cf8038eac0a96e91ed118c763f6f1f1fa6cb33651
NormalAnimFollow 82b3d828..82b3d8b4 48e2ea0fe9435372b77a9422fd63d6455ebe0c8a7a1a2bb08070392b040300dc
CreateDrive 82b48c30..82b48d10 3d452af66645e69ec1851860fb77e9a8938b14ac5145a5cc9c0005f00352de4c
Release 82b47ce0..82b47e70 63272d8cd4d291aeac5d106239d903d75379ec6e9e85a7867aa7fecbd314ad7f
SkeletonDrives dtor82b50720..82b50770 07b7b10aad4e547764edacf88c4530e8def58cd2482b2e61c5659aa54a26555f
BoneDrives dtor82b487f8..82b48890 a05780e20e7abca79a3cf45e3ea4dbcdad6edc4805727b80117b8fc0c702097e
```

## Validation (2026-09-07)

Focused producer95303 passes6; actual-owner40236 passes2 and core all-target
Clippy. Strict94008 EXIT0: fmt check,2298core+187formats+38app+3symbols =2526
unit tests plus6doc tests, workspace all-target Clippy with warnings denied,
git diff whitespace check. ALL7 original core corpus checks96112 EXIT0 include
the new mounted47-drive mixed path and actual original attribute observer.
The app's separate ignored probe was not rerun. No runtime constants or physics
arithmetic were changed to satisfy validation.

Prior RobPark solved board C/J/D stays6/6/3,6/6/3,6/6/3,4/6/3; COM positions:
[588.9575,-54.351295,996.21686], [588.9615,-54.343952,996.2112],
[588.9654,-54.340397,996.2055], [588.9701,-54.33957,996.1999].
The DMO eight-frame contact hash remains0e7c704e6a10f08a and final position
[588.8619,-54.40114,996.5048]. These are host regression checks, not reference
per-tick trace parity or permission to hand off playable skating tests.

## 2026-09-08 next live collision feedback seam (after checkpoint57482)

Read-only proof packet, NOT ongoing drive Update implementation. Existing
constructor/admission/Set work remains accepted. The actual normal collision
consumer now produces drive_strength from positive original-terrain contacts.
Next connect that SAME state to current animation targets and live payloads.

### Actual ownership and next solver consumer

SkeletonDrives in skeleton_drives.rs already owns the exact retained
Arc<RwLock<SimulationDriveFrames/Dynamics>> supplied to actual AddDrive slots,
24x2 bone payloads and2 hook payloads. It retains Bone flags/strengths/soft/
active/parent fields, ragdoll_strength and ik_active. Ongoing mutation belongs
on this owner, without replacing Arcs or allocating another drive owner.
SkeletonDriveRuntime in dynamic_assembly_skeleton_drives.rs holds prepared,
actual hook membership,24x2 session ledger ordinals and construction/release
stage. Body attributes already own per-Part collision_strength and root
coefficients; state.collision already owns current drive_strength.

Require completed admission, attachment to same backend, real live hook/bone
ledger occurrences and same SkeletonData. Existing backend check alone is not
that full ledger validation; a narrow read-only occurrence check is still
needed. Retained Arc storage is not membership: pool slots retain payloads
after removal. Keep cleanup hook drives -> hook assembly/definition -> bones
23..0 Local/Root. Errors must retain completed writes; no lock spans a callback.

Actual downstream path: skateboard_ground_frame.rs normalizes active frame
quaternions, then simulation_drive_pool.rs reads current frames THEN dynamics
into active Build snapshots, prepares islands and runs Horus. No wrapper or
new payload type is needed. Update before this synchronous interval; copied
prepared batches do not observe later payload mutation and occurrence checks
are not payload revision checks. Frame/dynamics updates alone do not move
bodies or change current collision geometry; they affect the NEXT Build.

Mounted follow-up should use real positive collision drive_strength0, prove at
least one eligible bone changes, preserve all49 actual Drive occurrences and
endpoints/Arcs, inspect the real prepared batch's existing test-only words and
run the next actual solve. Add late-failure/poison/release/refusal checks without
fabricating production handles or replacing authoritative payloads.

### Source producer prerequisite

SkeletonDrives::Update82b54b90 is called at GeneralUpdate82b5bb7c AFTER
UpdateHookPositions and IK, with actual Skeleton+3040 mAnimVolumesPoseBuffer.
ProcessData82b5ecc8 (caller PhysicalPlayerHiLOD::Update1_ProcessInput82e03718)
produces initial rows in loop82b5efe0..82b5f148, Parts0..23 ascending. Copy
input.poseBuffer[retained data.boneIds[part]] to mAnimPoseBuffer, then compose
with data.boneToVolume[part] into mAnimVolumesPoseBuffer, full4 product/FMA
order. Raw loop SHA256:
1b713282780a332fd4c68d492b396c295796373b83ae7e583567837db037a28a.

The loop precedes CalculateTargetTransforms, offboard/grind/landing adjustments,
skateboard offset and later IK. Its raw composition alone is NOT the final
target pose. Existing SkeletonData.physics_pose is a constructor snapshot;
ongoing history is physics output, not this animation input. A controlled
source-input projection must state excluded adjustments explicitly; neither
buffer can silently stand in for a completed live animation producer.

### Ongoing Update recovered graph

Update82b54b90..82b550f0 SHA256:
fc6ee235c01e21c392d2a1800f24f0a92b621a668dde2d43eca3c5495d5a6d2a.
First inverse target matrices for Parts1..23 into scratch, no normalization:
transpose axes with W0; full4 zero-P; Z multiply -> Y FMA -> X FMA translation.
Then three separate lazy global animation_default lookups (singleton+D4):
local scale key0a589569f5fa0e6c (lookup54cd4/load54cec), root scale
13d1a5a3e27d830e (54d2c/54d44), normal-follow BYTEca9e8fa1ab74fdd0
(54d8c/54da0). Genuine missing uses Default4/4/1, not malformed-data fallback.
Byte selects scales vs +0/+0 for normal/unmasked rows; masked soft rows still
use original fetched scales even when byte0.

BEFORE offboard scalar and collision strength, update actual Hook0 LINEAR:
damping+0 at54db8, typeHARD2 at54de8, spring bits4415ffff at54dec,
strength bits470c9fff at54df4. Hook0 angular and Hook1 unchanged.
Caller mask-byte nonzero selects partial mask
[1,1,1,1,1,1,0,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0] and samples current
kOffboardDriveScalar even if later strength selects normal branch. Otherwise
mask is all24 ones and scalar1. Collision strength sampled ONCE at54ee0.
Raw54eec40980148 selects normal when LT-not-set: >=1 OR NaN. Soft only ordered
strength<1. Mutation loops Parts1..22 (not Hips23 or Root0).

Soft masked row: write flags3 BEFORE arithmetic; compute root FMA first54f3c,
local FMA54f40:
root=(1-rootCoefficient).mul_add(driveStrength,rootCoefficient);
local=(1-localCoefficient).mul_add(driveStrength,localCoefficient).
Then multiply offboard scalar and original global root/local scales in source
interleaved order. No clamp. Unmasked/normal writes flags0 and selected scales.
Normal branch caches selected root thenlocal once55034/5503c; soft-unmasked
reloads selected root thenlocal each row54fec/54ff0. Every row checks source
parent_part!=-1 even if both drives inactive; do not skip it before flags/assert.

BoneDrives::UpdateDrives82b511d0 stores local thenroot strengths at+D0/+D4
BEFORE setters. If actual activeLocal, SetDriveFrames then EnableDrive complete
before rereading activeRoot+C9 and root-parent ID+C4, then analogous root work.
SetDriveFrames82b3fa90 composes invChild and invParent with identity-at-child-P.
SetChildFrame82b25248 obtains quaternion candidate, stores quatA then P_A;
only THEN SetParentFrame82b23470 obtains candidate, stores quatB then P_B.
No normalization here. Existing constructor try_bone_frames returns a whole
unpublished value; using it for one late live commit would lose child writes
on parent callback failure. Commit each setter prefix with no locks across
numeric services.

EnableDrive82b48930 flags3: load Bone+DC soft strength, multiply argument;
load+D8 soft displacement, multiply argument; strength product*bits4560fffe.
Read CURRENT kSoftDamping8308e474 target+28/value+d8 at82b48a88 BEFORE dynamics
stores. Then Angular type1@8a8c,spring8a90,damping8a94,strength8a98;
Linear type1@8a9c,spring+0 8aa4,damping+0 8aa8,strength+0 8aac.
Failure at cvar retains prior frame/strength writes but no dynamics stores.

Flags0 NormalAnimFollow82b3d828 captures retained cached animation layout(kind)
once; reads offsets8,12, scales angular strength by4560fffe, reads4, scales
angular spring by426fffff, reads0. Stores Angular.type2 d884; computes linear
spring d88c; Angular.spring d890; computes linear strength d894; Angular.damping
+0 d898,strength d89c; Linear.type2 d8a0,spring d8a4,damping+0 d8a8,strength
d8ac. Stored per-row strength arguments do NOT enter normal-follow dynamics.
Constructor attribute resolution is not permission to freeze ongoing cached
layout access or replace source lazy reads with snapshot tuning constants.

UpdateVolumesStates82b479d0..82b47aa4 (SHA13c97a45021a9da4480f8b34de24c5ce93e19939ef67484dc723ad12302f106e)
only maintains positive signed disable counters when active, toggling enabled
bits after decrement. It reads no collision outputs; fresh reset is inert.
UpdateRecords82b54118..82b54214 (SHA4616b157ccfe78cd73ea1914f1494bce2eb26fa46da16088c8e7a79cd2b3c55a)
first needs CalculateErrors with actual AnimToWorld/animation-volume buffer,
then COM/previous/velocity and current hook-to-hips/proxy/Reckoning producers.
Neither is a substitute for the meaningful current target -> drive feedback.

### Consolidated reviewer supplement / source identities

Global animation_default class babb3ec110849e08, collectiond7edbd362d7d2152:
binding verified at828b2784..828b27b8. Current kOffboardDriveScalar8308e538
target+28,float+d8 is read at82b54ecc; kSoftDamping8308e474 target+28,float+d8
is read separately per active soft drive at82b48a88. FollowAnimInRagdoll8308e6a0
target+20,byte+cd applies only to EnableDrive flags4, not this Update's0/3.
Keep retained SkeletonDriveAttributes Local/Root cached-layout selections;
fresh default slots would not reproduce the current selection.

GeneralUpdate's caller byte is Skeleton+3e43. PDB Skeleton size3e80, animation
volume rows+3040, raw mapped animation rows+3640; trajectory2e00,inverse2e40,
next2e80. SkeletonDrives size1a90, data+c, Hook frames+20,dynamics+a0,
IK active bytes+1a70. BoneDrives size110, Frames+0,Dynamics+80,Parents+c0,
Active+c8,Flags+cc,Strengths+d0,CollisionSoftDisp+d8,Strength+dc,
RagdollSoftDisp+e0,Strength+e4,SkeletonData+fc,PartId+100.

UpdateDrives returns immediately for PartId0. For other Parts it samples
activeLocal before copying child pose/inverse scratch and storing local strength
at82b51260 thenroot82b51264. Preserve this sampling before later callbacks;
activeRoot is reread after the complete local frame/Enable sequence. Outer
Update parent validation still precedes the call regardless of active bytes.

```text
GeneralUpdate 82b5ba88..82b5bc58 6f9035906e5fc80631410e91625af3b7369602c7bb4c51e6b110cfdb0e8f8693
UpdateDrives 82b511d0..82b51370 50ce3397f1b06491ee13a57e35b1d0e25d7714c971ad7b571f3b4a61f0df33d0
SetDriveFrames 82b3fa90..82b3fbe4 9bda3467558a2f072116ff376552585f4c74df3fd96273eb1277053a6606ea28
SetChildFrame 82b25248..82b25404 b0b91fd2e1a3675d15463e1a1af2950a202f53bbc4b25aaf76ddbe3a827d0ec3
SetParentFrame 82b23470..82b2362c f79fb51cfa470cf86a22197da96b9d23b0988c0fd1bb6b2ff25a3271145527be
```

Other EnableDrive branches are not reached by bounded Update flags0/3: flags1
fixed HARD; flags2 cached angular with zero HARD linear; flags4 follows current
ragdoll-follow toggle or full-ragdoll response; unsigned flags>4 unchanged.
Do not inadvertently claim these implemented by adding only the current path.
Source confidence high for reviewed control/store order and ordinary finite
numeric graph; native exceptional-result/estimate/FP-status parity remains open.
