# Skeleton mass and initial physics history

## 2026-09-08: ongoing paired-owner history prefix (checkpoint82288)

High-confidence source-backed host implementation; no native per-tick trace.
`physical_player_history.rs` and the existing mixed skater pool now connect the
actual completed manager frame to the SAME SkeletonNormalAssembly history.
This supersedes the earlier initial-history-only boundary below, not the
remaining full Skeleton/Actor lifecycle limitations.

Live raw `?UpdatePhysicsRecords@Skeleton@Physics@Sk8@@AAAXXZ`
0x82b4cc00..0x82b4cd08 SHA256
`c63d7094c06fac363605d1112e32c2d44ae7eb7c64882a740ebf1a5c862bfef8`.
Root and independent reviewer read all66 instructions. Caller is
UpdatePostPhysics82b576d8, before the Island collision-list readout.
Unlike Init, it resolves GetPhysical->Skateboard and gets actual BOARD Part6
atcc4c, then acquires current ProcessedPhysIn atcc5c/up+220. It calls the existing
UpdatePhysicsRecord kernel ONCE atcc70, using deck matrix as record0 while that
kernel separately gets skeleton0 for proxy and skeleton1..23 for remaining rows.
Never substitute the constructor's supplied skeleton0 matrix at this callsite.
The suffixcc74..cce8 transforms Skeleton+3d00 and saved deckP through the missing
world-to-animation owner+2dc0 into+3d40/+3d60; it is NOT implemented or claimed.

Pool validates actual board/skeleton kinds and shared retained physical-owner
Arc, then disjointly borrows their existing sessions. Same-backend attachment
and completed constructor history are checked. No new mass initialization or
record allocation. Actual deck getter precedes current input acquisition; COM
mode/up-lane timing and all ordered kernel writes remain unchanged.
World requires actual completed feedback AND contact-list creation for the exact
current exporter Arc, including legitimate zero-spy frames. Reviewer caught and
root fixed the initial feedback-only gate. No fake frame or pose is supplied.
Same Skeleton owner retains attempted Arc plus NotStarted/Updating/Complete;
same-frame replay is refused even after failure, completion only follows success.
This is HOST attempt bookkeeping, not a native field or perpetual freshness
certificate after later body mutations. Full gameplay scheduling is still open.

Existing original-body corpus now runs the actually constructed35-body pair
through three authored-ground frames with C/J/D0/28/49,50 solver iterations,
Vault gravity[0,c11ccccd,0], controlled no-freezing and manager dt3d088889.
Original RobPark terrain is resident, but the controlled identity spawn is
outside its bounds: these are real CONTACTLESS paired constraint frames, not
positive skater terrain collisions, pair partitioning or freeroam input.
The first two frames update actual history and check every matrix, position,
velocity, delta, proxy and COM word against prior SAME history/live getters.
Tests explicitly prove deck6 XYZ differs from skeleton0 and that zeroing prior
history would change an expected XYZ velocity. Source history dt stays3c888889
despite different manager timestep. Mass/fractions and actual bodies/joints/
drives remain unchanged by the history writer. Replay is refused before input.
Third frame's controlled COM-mode failure leaves all24 rows/proxy updated but
BOTH old COM values intact; progress remains Updating and retry is refused.
Missing-frame, stale-epoch, incomplete-feedback/list and wrong-kind gates run
before input; test invalidation restores original actual list/phase owners.
Current processed-input/COM services are controlled fixtures, not reconstructed
live Actor provenance. Constructor and post-solve completion remain distinct.

Final strict82288 EXIT0: format/check,2380core+187formats+38app+3symbols=
2608unit,6doc, all-target Clippy-Dwarnings, ALL7ignored original core checks,
gitdiffcheck. Six new unit tests across history(1), vector representation(2),
cached mass(3); ongoing positive/failure coverage extends the existing ignored
body test. Focused original-body8cbe99 passed before final stronger phase and
nonvacuity assertions;82288 covers all of them. Initialstrict35234 passed the
workspace then stopped on four style lints, fixed without behavior changes.
Prior RobPark board trajectories and DMO8hash0e7c704e6a10f08a are unchanged.
No app ignored probe rerun. Next is actual normal/world collision response.

2026-09-07. Source and implementation evidence, high confidence. The actual
skeleton owner now performs mass initialization and both initial physics-history
updates after successful full retained-pose Set. This does not complete the
native constructor, gameplay input loop or per-tick physics-history lifecycle.

## Implementation and scope

`skeleton_physics_history.rs` owns the constructor's actual unscaled PartMass
snapshot, source fractions/total/dt and one retained physics record. Its 24
matrices and vector arrays have the recovered constructor defaults. Every update
uses ordered matrix, delta, velocity, position, proxy and COM writes; checked
errors retain the completed prefix. XYZ-only NaN checks and full W arithmetic
follow the independent raw audit below. Current COM mode and lazy up-vector reads
are services; the host dot association is explicit, not a Xenon reduction claim.

`dynamic_assembly_skeleton_history.rs` connects those operations to the existing
actual normal assembly. It requires same-backend attachment, complete drives,
and evidence of successful full Set. Once attempted, initialization cannot be
silently retried. Masses initialize first, then each of two source samples gets
actual root Part0, acquires the current processed-input collaborator, and updates
through actual live Part getters. Only a fully completed sample advances the
checked completion count. A failed second acquisition retains the entire first
record. The up-source token is read only after a false COM-mode sample.

The same mixed skater pool and manager expose this narrow continuation without
exporting mutable sessions or body handles. History-only writes do not invalidate
collision publication: no body, constraint or geometry is mutated. Full Set's
proxy target remains separate from the history record's actual proxy position.
Both history and target survive drive release. This represents the initial
constructor continuation, not later dynamic PartMass updates or the missing
animation-record/generated-attribute/IK/registration lifecycle.

Ten kernel tests and five actual-owner tests cover ordered f32 sums, one rounded
reciprocal versus separate division, fused COM cancellation, initial defaults,
two distinct samples, failure prefixes, invalid lanes, lazy reads and actual
owner/handle/Set prerequisites. Existing original-data mixed coverage initializes
history through the actual manager after full Set, compares all retained masses
and record words with the actual live bodies, and proves bodies/joints/drives and
publication freshness are untouched. Saved history survives the later controlled
zero-iteration ground frame and source drive teardown. Current input/cvar values
remain explicit harness samples; no native execution trace or skating-feel
equivalence is asserted by these source-derived host checks.

## Source order and retained ownership

Skeleton::Init82b62710 calls Set82b62a64, InitializeMasses82b62a70,
fresh Body::GetPosition(part0)82b62a84 -> UpdatePhysicsRecord82b62a9c,
then fresh GetPosition(part0)82b62ab0 -> UpdatePhysicsRecord82b62ac8.
Each update reloads current ProcessedPhysIn and passes its UpVector at+220.
The misleading parameter name skateboardTransform receives SKELETON Part0,
not board Part6. Body::GetPosition82de7b20 resolves actual Assembly Part and
calls Part::GetTransform82de7b4c. Existing owner-qualified
DynamicAssemblySession::try_live_part_transform_host is the actual reuse point.

Later IK initialization/doIK changes/proxy disabling/root collision/count stores
are separate lifecycle work. Do not claim them as part of this bounded slice.
Root independently decompiled the outer Init and verified the two distinct calls.

SkeletonState is embedded at Skeleton+1940:

| State offset | Field |
|---|---|
| 0000 | Generated physics attributes |
| 0010 | Animation record |
| 0610 | Physics record |
| 1100 | PartMass[24] |
| 1160 | FractionalPartMass[24] |
| 11c0 | LinearError[24] |
| 1340 | Proxy-volume target |
| 1350 | Total mass |
| 1354 | Time delta |
| 1358 | Reset scalar |

SkeletonRuntimeState holds actual live material/collision/header state and the
full-Set proxy target; a separate retained SkeletonPhysicsHistory now lives in
the SAME normal assembly owner. The input
mass is the already-produced SkeletonBodyDefinition.parts[i].unscaled_part_mass,
not effective mass, current inverse inertia or animated/ragdoll mass caches.

## InitializeMasses82b3d0e0..82b3d234

Write total+0; visit24 masses ascending, each individual f32 operation is
mass[i]+accumulated_total, with total stored after every addition. Require
total>0. Raw fcmpu82b3d17c/bgt82b3d180 means NaN also fails; decompiler's
total<=0 is insufficient. Root independently verified this raw branch.
Compute ONE fdivs reciprocal1/total82b3d1b0, then24 ascending products
mass[i]*reciprocal. No Vault read or reciprocal estimate occurs here.

## Fresh physics record

PhysRecord constructor82b3d6f0..82b3d7b0:

| Record offset | Field |
|---|---|
| 000 | 24 matrices |
| 600 | 24 positions |
| 780 | 24 velocities |
| 900 | 24 velocity deltas |
| a80 | COM |
| a90 | COM velocity |
| aa0 | Proxy-volume position |

Matrices start identity I/J/K with four-zero translation; all vector fields
start four-zero, including proxy W. SkeletonState constructor82b50d08..82b50e40
constructs animation/physics records, zeros mass/fraction/error arrays and scalar
fields and creates its own generated physics instance. A narrow physics-history
owner must not claim the missing animation-record/attribute lifecycle.

## UpdatePhysicsRecord82b48288..82b4854c

1. Store fixed dt bits3c888889 at82b482cc, independent of manager frame dt.
2. Copy supplied root matrix to record matrix0 through PhysRecord::SetTransform.
3. All four lanes: velocity=(newP-oldP)*bits426fffff; delta=velocity-oldVelocity.
4. Validate/store delta, then validate/store velocity, then validate/store P.
5. Separately get live Assembly Part0 transform82b483e8 and store its translation
   as the record proxy position.
6. Visit Parts1..23 ascending: live getter, matrix copy, same ordered history writes.
7. CalculateCentreOfMass call82b48524.

This does not read solver velocities or compute angular velocity. Fresh zero
history plus unchanged nonzero positions across two initial samples yields:
first velocity=P*bits426fffff; second velocity=0 but second delta=-firstVelocity.
Do not collapse the two source calls or clear both arrays afterward.

PhysRecord::SetTransform82b3a800..82b3a93c checks axes and position sequentially,
then copies all16 lanes. It does not update the separate position array.

Independent RAW GATE PASSES: validity assertions use VMX self-comparison after
W-lane permutation, NOT a generic finite-value test. Corrected VA at82b3a820 is
v63, not IDA's v95. Mask1/rotation1 at82b3a824 produces [X,Y,Z,X]; corrected
self-compare at82b3a828 compares v62 with itself and extracted CR6 bit24 requires
all-true. The predicate rejects XYZ NaNs, accepts infinities, and ignores original
W. History delta/velocity/P and COM/COM velocity use the same pattern. Existing
actual live getter canonical-W/host numeric boundaries still apply.

Exact update prefix: dt -> validate all4 matrix columns -> copy16 -> delta
validate/store82b48374 -> velocity validate/store82b483a0 -> P validate/reload/
store82b483e0 -> separate Part0 getter82b483e8 -> proxy store82b48408 (NO
validity assertion). Parts1..23 then follow. Checked errors must retain preceding
writes. Native FailAssert/DebugBreak may resume; typed stop is an explicit host
failure policy, not rollback or a native fault-continuation claim.

## CalculateCentreOfMass82b3d518..82b3d6ec

Require total>0. Ascending24 full-four-lane FMA sums: COM uses stored matrix
translation*fraction; COM velocity uses stored velocity*fraction. Sample current
kUseXYZCentreOfMass target BYTE at82b3d630 via global8308eb4c, target+cd.
Nonzero keeps weighted COM. Zero selects:

```text
displacement = weighted_com - supplied_root_matrix.translation
projection = dot3(displacement, current_up_vector)
com = current_up_vector * projection + supplied_root_matrix.translation
```

Final operation is full-four-lane FMA. Never normalize up. COM velocity stays
unprojected. Validate/store COM, then validate/store COM velocity.
Initializer82f7d6a8..82f7d6f4 registers groupPhysics_Animation,
namekUseXYZCentreOfMass, defaulttrue. Default is NOT a current runtime sample.
Use a current-value service. Independent raw audit confirms ONE vmsum3fp128 at
82b3d650, corrected VA v59, ignores W and broadcasts the result. Its internal
Xenon reduction rounding is not recoverable from this opcode alone; preserve a
dot3 numeric seam and explicitly qualify the host association.

Sampling correction to the initial packet: outer Init acquires the current
ProcessedPhysIn pointer after each supplied-root getter; it does NOT load the
UpVector lanes at that time. COM samples the target pointer82b3d62c/byte82b3d630
after all24 interleaved COM/COM-velocity FMAs. Only the false branch loads up
at82b3d64c and invokes dot3. True must not read up or call dot3. Projection uses
stored record.matrix0.P, not another live getter. COM validate/store82b3d69c
precedes COM-velocity validate/store82b3d6cc. Both total assertions require >0:
NaN fails, +infinity passes. All three audited function hashes match below.

## Connected integration prerequisites

The narrowly named SkeletonPhysicsHistory is retained in the actual Skeleton owner.
Seed from exact retained binding masses and run the two initial updates through
actual session live getters, separately sampling current up/cvar each call.
No mutable session export, synthetic body handles or replacement pose required.
Acquire a retained up-source token after each supplied-root getter and read its
lanes lazily only inside the false COM-mode branch; do not pre-sample the vector.
Require actual successful full Set chronology. Existing drive runtime's Complete
stage means DRIVE admission complete, NOT that full Set has run. A separate
initialization phase/guard is used rather than treating that stage as sufficient proof.
A successful-Set flag proves prior complete placement, not freshness after later
body integration. This is a checked initial-call sequencing prerequisite, not
proof that the entire native Init lifecycle is enforced by the host facade.

Tests: ascending mass-sum rounding and NaN/nonpositive refusal; two-call nonzero
delta discriminator; repeated root getter/up/cvar chronology; projected COM with
non-unit up and unusualW; actual mounted definition masses through admitted owner.
Use existing component manager test, not a disconnected fake record provider.
Still not whole Tick, native pose provenance or hands-on skating.

## SHA-256 function hashes

```text
InitializeMasses 468a05aa647b5fdb5914cf0b3fbd7443af53b47d662c1d2ef83a8cef69bce134
UpdatePhysicsRecord f217d2efc5db14815d53cf17398c7eb3bed02b034497733a9bf5b7d7e246280d
CalculateCentreOfMass d2ddf5e577c2161f3dd3c791774a2347f340d5bad02410c8c07ff73e5c162689
PhysRecord constructor 1559deaa58db891801cbe69fc48e24f697620ce1c0fe453a6dfd18f71eafb485
PhysRecord SetTransform 6d9df63a8c4bf7ba7e9d4bea11c30e4cd77a2caedbff51c4064a452468e8e4ba
SkeletonState constructor 2807ab56322ebbafc01c2ea17dfe292803f9c5c3a10b68357fa912b54cc1d821
COM cvar initializer d82c60f9ef8a8be88b61a5184724f70de3ce72d4d8e3847db0dbe53f0faeb055
```

## Validation and remaining work

Independent raw source, production, all 15 new tests and mounted continuation
reviews PASS. Focused16256 passes16 matching tests (15 new history tests plus
one existing substring match) and core all-target Clippy. Strict43976 EXIT0:
format check, full workspace tests, all-target workspace Clippy with warnings
denied, ALL7 ignored original core corpus checks, and whitespace check. The quiet
workspace rerun fe50a7 confirms2313 core +187 formats +38 app +3 symbols =2541
unit tests and6 doc tests. The app's separate ignored probe was not rerun.

The original mixed manager case now initializes this retained history after full
Set and before the existing35-body/28-joint/47-drive zero-iteration ground pass.
Prior solved RobPark board C/J/D remains6/6/3,6/6/3,6/6/3,4/6/3 and COM remains
[588.9575,-54.351295,996.21686], [588.9615,-54.343952,996.2112],
[588.9654,-54.340397,996.2055], [588.9701,-54.33957,996.1999].
DMO eight-frame hash0e7c704e6a10f08a and final[588.8619,-54.40114,996.5048]
remain unchanged. No runtime tuning or numeric workaround was introduced.

Next connected gap is actual Island Body membership/contact-spy registration
and its original contact-feedback producer/consumer. See the new read-only
`skeleton-island-registration-next-source.md` packet. Final IK/proxy/root-disable
tail, full player/input/animation/camera/renderer lifecycle and native tick traces
remain required; this checkpoint is not hands-on skating readiness.
