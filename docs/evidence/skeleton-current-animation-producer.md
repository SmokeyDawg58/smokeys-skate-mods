# Current Skeleton animation preparation

2026-09-08. Continues retained-root checkpoint85223. Source confidence **high**
for the supported numerical graphs and ownership projection; **exact** for the
recorded bytes, offsets, branch conditions and source call/store ordering.
No captured Xenon tick trace or playable/complete ProcessData claim.

Latest68261 adds active LandingOnBoard, complete QueryRotate/ATan/SLerp math,
lazy authored curve and actual same-owner COM/WorldToAnim inputs. Strict2813
units/six docs/all nine original core/fmt/Clippy/diff and independent review PASS.
See [landing evidence](skeleton-landing-on-board.md). Selected inactive Landing
now returns through its own gates; old blanket unsupported claims below are
superseded. Active landing spring and GrindAir, full IK/GeneralUpdate remain open.

Latest continuation33408: the formerly read-only COM/error producer packet
below is now implemented with volume states and hook/normal-displacement suffix,
on the same owner after collision. See [records evidence](skeleton-post-physics-records.md).
Strict2779 units/six docs/all nine original core/fmt/Clippy/diff and independent
review PASS. This also corrects earlier offset NaN evidence: NaN counts are
inactive. Current-animation readiness is invalidated by raw drive-pose remapping.
Active LandingOnBoard/SLerp/QueryRotate remains the next source implementation.

## Connected scope

`SkeletonNormalAssembly` retains current animation preparation beside its
existing mapped animation targets, physics history, drives and roots. It owns
one IK-target projection, one animation record, and the represented adjustment
histories. IK retains the same `Arc<SkeletonData>` as the actual body. Animation
COM borrows the existing initialized fractional masses and total mass; it does
not construct another mass authority or use current effective rigid-body mass.

The checked physical-world methods initialize four target lookups and retain
the actual physical owner's immutable global Vault instance, then execute:

1. Existing current raw pose -> 24 mapped animation/volume rows.
2. Mapped volume0 -> retained RawAnimSkateboardMtx.
3. Four current raw-pose IK target compositions.
4. Source Offboard activation, remaining supported adjustment gates/effects,
   and retained full/Y offset decay/application to actual volume0 and IK targets.
5. All24 animation record matrices, animation COM, retained COM differences.
6. Adjusted volume23 -> Hips; volume0 -> Skate; passed output deck-ZY difference.
7. A separate grounded-root call consumes that same owner's completed Skate
   transform and the existing actual paired Part6/retained Reckoning path.

This implements the bounded interval `ProcessData`82b5efe0..82b5f25c. Earlier
input retention, activation, ProcessAnimAttributes and trajectory work are not
called by this projection. The later processed orientation/collision flags,
hook-to-hips/COM publication and teleport drive suffix remain separate work.
The explicit dt/processed fields and supplied current pose are still controlled
inputs; a live Actor or animation graph does not yet produce this stream.

The successful volume rows include all supported adjustment and offset effects;
they are not treated as adjusted merely because their input category is ground.
Selected LandingOnBoard, activated GrindAir and landing start/spring still stop
with a typed unsupported-stage error. This intentionally also excludes internal
no-op cases inside the selected LandingOnBoard/activated GrindAir collaborators.
Native GrindAirAdjust is an externally retained collaborator; its fresh-state
projection does not establish its missing HiLOD ownership or active producers.

## IK source and mounted names

Native symbols: `Sk8::Physics::SkeletonIK::{Init,CalculateTargetTransforms,
GetTargetVolumeMtx,Reset}`. Init82b5eb80 first stores actual data bone IDs for
Parts15/19/3/7 at IK+204. Four five-word packed-name lookups then write +34c:
LeftToeBase_Reparented, RightToeBase_Reparented, LeftHand_Reparented,
RightHand_Reparented. Parts+460 are15/19/3/7, parents+470 are16/20/-1/-1.
GetBone82590fc0 returns the first exact packed match, or -1. Init preserves a
missing name and continues subsequent lookups; it does not assert there.

GetTarget82b3de60 loads all four raw pose[targetBone] columns before invalid-Part
assertion, then composes with actual BoneToVolume[data+c70+64*part]. The reused
source compose performs X multiply (translation X FMA into P), Y FMA, Z FMA,
all four output lanes; inner W is not a coefficient. Correct VMX128 VA decoding
is essential; displayed aliases such as v93 are not reliable operands.
Calculate82b4a140 visits four ascending targets and commits each complete matrix
before the next read. A failed host read/assertion retains completed targets
and the older suffix. Init readiness is a disclosed host guard for unknown
pre-Init IDs. Target Reset82b5146c..82b514c8 initializes I/J/K/P0, all W0,
without discarding mappings.

Original anim.bh OnBoard and SkaterBodyPres hierarchy records both have36 bones.
Target indices in source order are **33,32,35,34**; limb data IDs are24,20,16,12.
OnBoard reparented parent IDs are25, while BodyPres has-1: supplied pose hierarchy
need not be pointer-identical to the mapping hierarchy. Raw hierarchy-range
SHA256 (record+30..580): OnBoard
17285fe195f48043953e1137683285a981447d9bb545bce885b453884f07f4f3,
BodyPres5d3842f2ef4446725064a331a01671956b7e96dd1860099e0550cd30a3fceef7.

## Adjustment gates and failure prefixes

Offboard82b4c808 enters only category+8ec==500 and state+8e4!=505. ProcessData
tests retained processed+8cc bit200: set selects LandingOnBoard unless40000
also set; clear calls GrindAir. Fresh inactive GrindAir skips the assist cvar
and writes retained Adjusting=false at82dd165c. An activated unsupported call
must not prematurely make that final store.

Landing82b401d8 ALWAYS fetches physics_animation_default key
5199e2e749e7220b,index0/default4 before transition tests. Global ctor828b1608
uses collectiond7edbd362d7d2152; generated ctor828b0f00 synthesizes class
74bc9c93affe6aac; instance is stored global+CC at828b2724. The runtime provider
retains that actual immutable Vault instance and redoes scalar lookup each call.
Only genuine absence becomes DefaultDataArea4 positive zero; malformed data
remains an error. Independent raw db.big node at archive+5bd80 is
`5199e2e749e7220b4100000000054000`: original value bits41000000 (8.0), used
as a corpus oracle, never hardcoded runtime tuning.

Landing constructor retains adjustingfalse, lastFiltered0, secondLastComVel0.
Filtered states are GROUND1/AIR2/GRIND3/OFFBOARD6. Start triggers are:

- (OFFBOARD->GROUND or AIR->GROUND) and flags8cc&40000000.
- AIR->GROUND and balance!=0; unordered NaN also satisfies !=0.
- AIR->GRIND and !(flags8c4&20).
- flags8c8&4 and (AIR->GRIND trigger or AIR->GROUND).

flags8c8&1 without coffin bit4 clears adjustment before the unconditional
dot3(processed+260,processed+220) and current kDebugLandingAdjust sample.
Debug true/service failure stops before start logic. Unsupported start stops
before first store82b405a0; existing active spring stops before its work. These
stops retain any earlier disable/pose/IK writes and do not write terminal history.
Successful inactive Landing stores secondLastComVel at410d8, then lastFiltered
at410dc. Snapshot input excludes concurrent callback alias mutation, while
preserving the separate retained input and passed-output contract.

Correction to the prior handoff: offset counts+3dd8/+3de0 are **f32**, not
signed integers. CORRECTION after68185: raw counter branches are BO4/BI25,
testing NOT CR6.GT. NaN therefore skips the active branch, just like <=0.
Earlier source review and tests incorrectly interpreted the printed BLE alias;
those NaN-active expectations are withdrawn. Full offset is processed before Y.
Both active branches now
execute; only reaching42058/5c clears both JustSet bytes.
Fresh full/Y counts and both bytes are0. Returning to ground does not reset
those counts or any landing history.

## Animation record and root copies

`SkeletonState::UpdateAnimRecord`82b50e40 first copies24 matrices, then calls
CalculateCentreOfMassAnim82b48550. That helper requires totalmass>0 (NaN fails),
then accumulates all24 ordered full-four-lane position*mass-fraction FMAs.
Only afterward does it sample kUseXYZCentreOfMass. False uses deck position XZW
and weighted COM.Y; true uses weighted COM full4. It stores COMdelta before COM.

UpdateAnimRecord transforms COM and deck position SEPARATELY using CURRENT
AnimToBoard, subtracts them, stores COM-to-deck delta before current value,
scales delta by retained ResetScalar, then stores scalar1. Transforming a
pre-subtracted vector changes rounding and is not equivalent. Constructor
AnimRecord82b4b870 initializes24 I/J/K/P0 (all W0); SkeletonState ctor82b50d08
initializes COM/delta vectors0 and ResetScalar0. Zero scalar does not repair NaN.
Unknown outer matrix W remains typed unknown even with zero coefficients or
equal transformed points. Input position W is never a transform coefficient.

Actual SkeletonState::Reset82b3d238 clears the represented COM/delta vectors
after physics reset; it preserves animation matrices and ResetScalar. That
full ongoing reset is not currently implemented and this suffix is not attached
to ordinary Set. Skeleton TeleportReset82b54228 separately resets deckZY at54254
and calls IK::Reset5427c, preserving animation record/landing/offset histories.
The new explicit animation-reset projection is NOT attached to the existing
body-only TeleportReset82b3bf88 and does not claim full Skeleton reset.

Only after record success do f214..220 copy adjusted Hips, f238..248 copy Skate,
f258 store currentSkate.Z.Y-oldZY to PASSED processed+9cc, and f25c update oldZY.
Readiness is host bookkeeping: failed preparation leaves retained old copies
visible but refuses exporting them as a completed current root input.

## Review and validation

IK and animation-record implementation authors are separate from the
source reviewer. The reviewer independently read raw routines and reviewed the
two modules and their focused tests, plus root's supported adjustment sequence.
Source function hashes agree with ASTRA_RESTART_HANDOFF's final packet.

The existing original paired construction/solver corpus is extended in place:
same35-body/28-joint/49-drive owners, same initialized mass history, actual
mounted hierarchy/offsets and Vault, two solved frames. Independent scalar
mapping/IK/COM oracles include changed current pose, deckZY and retained
AnimToBoard; wrong member and staged failures are checked. The actual root
consumer obtains the current pose from the same retained preparation owner.
This is source/corpus validation with controlled input and host numerical
policies, not a captured reference trajectory. Test execution and final strict
gate results are recorded in RESEARCH_LOG after completion.

## Active-offset source packet and connected implementation

The retained offset state, original full setter and active updater are now in
`skeleton_skateboard_offset.rs`; IK AddOffset mutates the same retained target
owner. A separate reviewer independently audited raw code, implementation,
seven focused offset tests, twelve pipeline tests and the mounted corpus: PASS.
Strict execution results are recorded in the latest RESEARCH_LOG checkpoint.

UpdateSkateboardOffsetTransform82b41c80..82b42068 SHA256
80941b93ad7c9d2fa5ae5c65eee97e29d085327e4935848f656d005473f3a40b.
AddOffsetTransform82b3df68..82b3e030 SHA256
9ad8375f7c80a66aa0fb514e0d356155a6a42b010a99d724788738a56e54a573.
Retained offset matrix+3c40; IK pointer+3dfc and targets+360.

Active full count with JustSet0 computes separate f32 (count-1)/count, squares
it, and computes1-weight. Each basis becomes oldAxis.mul_add(weight,
identityAxis*(1-weight)), with separate identity product. Translation multiplies
all4 by weight then restores ORIGINAL Y (vrlimi mask4 at41e54): full decay
changes X/Z/W, while Y is independently controlled. The already audited
reverse-axis classical Gram-Schmidt/two-rsqrt-refinement graph normalizes the
interpolated axes, no repair/assertion. Restore decayed P, reload/decrement
full count. Y then independently computes its ratio-square, scales only P.Y,
preserves X/Z/W, reloads/decrements its count.

Either active count at entry marks the transform for application even if the
count becomes0/negative during decay. JustSet skips decay AND decrement but
still applies. If full JustSet is set, native skips full decay and proceeds to
the Y gate before application. Both formerly unsupported offset branches now
execute on the same current-animation owner's retained state.

Application composes offset OUTER with current animation volume0 and commits
volume0 X/Y/Z/P before calling IK AddOffset. That helper visits four targets
ascending, compose(offset,oldTarget), committing each in Z/X/Y/P column order.
It performs no name/data lookup, normalization, initialization check or cvar
read, and does not solve IK. Both JustSet clears occur only after it returns;
they also clear when neither count was active.

SetSkateboardOffsetTransform82b3b178..82b3b1c8 copies offset, stores full
JustSet1, then samples kLandingAdjustFramesToBlendAwayOffset8308e9d4 through
target+28/value+d8 and stores count. A cvar error must retain copied matrix and
flag. Checked world/pool/normal-owner setter bridges validate physical ownership
and attachment before the source setter; they do not invent an IK-init gate.
Current preparation updates actual mapped volume0, then all four IK targets,
then rereads adjusted volume rows before animation-record/COM/root copies.
The earlier raw deck copy remains unadjusted. Landing's Y activation and other
active adjustments remain additional work. Focused discriminators include
simultaneous counters, count1/fractional/NaN, JustSet, independent Y, fullW,
source interpolation/normalization ordering and all four changed IK targets.

The original paired corpus now seeds a controlled quarter-turn/translation
through the real setter. Wrong-kind calls sample no cvar; a cvar failure leaves
the new matrix/flag with old count. Frame0 applies at count4 with JustSet, while
frame1 verifies early failures leave offset unchanged, a late COM failure
advances full count4->3 and changes actual volume0/all4IK/record rows, and retry
advances3->2 before successful COM/root publication. An independent scalar
source-form oracle reconstructs full decay, classical normalization, full4
composition and subsequent COM/root values. This proves the connected host
graph, not a Xenon runtime trace or native gameplay activation producer.

## Actual activation producer: offboard (connected and independently reviewed)

Root and the independent reviewer read `UpdateOffboardAdjust`
82b4c808..82b4c930 (exclusive end), SHA256
882dd4193a3ac42369d33a11c717b5570d0cb4e274a9b26e7375c00d2e1a7a93.
Its actual ProcessData gate is category500 and state!=505. It calls
`SkeletonIK::GetIKHandToAnimHandMapping`82b3dd48..82b3de5c (exclusive end,
including blr), SHA256
10028e1c6f7385ce1480eba164ff67e198d449b1da75e068e755cdd13c1fb325.
Mirrored flags8cc&8 selects IK index2/LeftHand; false selects3/RightHand.
The helper reads RAW pose[targetBoneId] before RAW pose[actualBoneId], builds
the target's rigid transpose inverse, then composes actual-hand OUTER with
inverse-target INNER. It does not read retained IK target matrices or
BoneToVolume. Inverse translation uses arithmetic +0-P, Z multiply/Y FMA/X FMA;
the actual-hand outer W participates in full4 composition. No normalize,
assertion, tuning read or retained mutation occurs inside the helper. Output
temporary stores are Z/Y/X/P. Existing accepted rigid inverse/compose match
this graph. This is a pose-derived producer, not a supplied offset matrix.

Offboard copies the resulting full matrix to retained offset atc8d4/d8/e0/e8,
sets fullJustSet atc8ec, THEN samples blend cvar8308e9d4 atc8f4/f8 and stores
fullCount atc8fc. It separately reloads that cvar target atc900, copies current
offsetP into a temporary and replaces temporaryY with mappingP.Y, THEN reads
the second float value atc914, before retainedP writec91c, YJustSetc920 and
YCountc924. A first-read failure retains matrix/fullJustSet but old counts; a
second-read failure retains matrix/fullJustSet/newFullCount and OLD Y flags/
count. The two reads cannot be coalesced or moved after a generic Y setter.

`skeleton_offboard_adjust.rs` now implements this producer on the existing
offset owner. `SkeletonIkTargets::try_hand_to_animation_mapping` has explicit
target-before-actual typed read failures and reuses the source-reviewed rigid
inverse/compose. The current-animation pipeline runs the category/state gate
after raw mapping/IK calculation and before the other adjustments, using the
same supplied current raw pose (retained native input store82b5ece8). A lazy
borrowing adapter samples the cvar twice, without inventing an early read or
merging the two values. Offboard succeeds into existing offset application,
record/COM and root copies. Earlier failed writes remain visible; failed
preparation still refuses stale root publication.

The independent reviewer verified IK ctor/reset leave bone-ID storage unknown;
only Init establishes it. Ctor550f0 clears DataIn(+20,size1b0), then calls
Reset55170. Reset's scalar loops leave actualIDs+204..210 and targetIDs+34c..358
untouched. Explicit host NotInitialized is a sequencing boundary, not a native
boolean field, and target-transform reset retains learned IDs/readiness.

Six new focused tests use frozen full4 outputs, nonunit target axes, target-W
NaN and actual-hand W, changed retained targets/large BoneToVolume, missing
named/read-order errors, and both cvar failure prefixes. Pipeline suite now16
tests, including both mirrors, gate skips, preparation before later unsupported
LandingOnBoard and Offboard->ground independent retained counts.

The mounted same-pair corpus retains both prior setter frames and extends to
five actual solves: four successful history/animation/root component frames,
then the existing late physics-history failure. Added phase2 changes actual
LEFT hand16 and raw target35 independently; mirror selects their source-derived
mapping. First blend failure preserves new transform/flag with old full count;
second preserves new full count3 with old Y state. Success samples counts3/5
independently and applies without decay. Added grounded phase3 retains the
offset and decays full3->2/Y5->4 before actual mappedvolume0/all4IK/record/COM/
root checks. Independent scalar inverse/compose/decay oracles validate the data.
The driver explicitly supplies grounded Reckoning during the offboard animation
case: this is component integration through actual solves, not reconstructed
native offboard GeneralUpdate/state scheduling. Pose/input/cvars remain
controlled; native input, full IK solve and Actor production remain unfinished.

Separate AstraHIGH reviewer read raw source, implementation/shared bridges,
six kernel tests, sixteen pipeline tests and four-frame corpus: PASS. Root
execution results are recorded in the latest RESEARCH_LOG checkpoint.

The standalone full setter82b3b178 has exactly one code caller:
`PhysState_Skitching::UpdateDeckAnimOffset`82d96430..82d966a8, call82d96698;
its own caller Update calls it82da1184. Offboard, GrindAir and LandingOnBoard
inline their setters, and active landing spring writes Y. Thus a controlled
standalone-setter corpus does not prove skitching or landing production.
Offboard now supplies the bounded actual full/Y activation path. Raw source gate,
lane selection and ordering confidence exact; host float graph high with the
existing Xenon exceptional/estimate boundaries. Neither this source packet nor
the offset consumer closes the remaining Actor, input or full IK solve.

## Next retained COM/error producer (read-only, not implemented)

LandingOnBoard82b417c8 first gates on retained Skeleton+3d20.Y<0 and then
processedflags8d0&10 clear. This is mCentreOfMassVel, a different field from
processed+260 and PhysicsRecord's mass-weighted velocity. Root's dependency
peek also found active branch inputs WorldToAnim, processed deck transform,
current mappedvolume0, OB_TimeToLand+a0c and a PointGraph curve. Its matrix
interpolation82e4acf8 calls affine SLerp82b4b418, which calls QueryRotate and
the accepted SinCos. These active landing graphs remain unimplemented and
require their own raw/independent review; decompiler text alone is not proof.

Independent reviewer identified actual retained velocity producer:
Skeleton::UpdateRecords[82b54118,82b54214), SHA256
4616b157ccfe78cd73ea1914f1494bce2eb26fa46da16088c8e7a79cd2b3c55a.
Sole code caller is UpdatePostPhysics82b5789c AFTER collision.Update57888 and
Body.UpdateVolumesStates57890. Actual UpdatePostPhysics span is
[82b575d0,82b578b4). Existing earlier UpdatePhysicsRecords576d8 is a separate
operation; do not retime this derivative into that history producer.

UpdateRecords first calls SkeletonState::CalculateErrors54138; that helper
touches no FPR or callees, so input dt in f1 survives (decompiler temporaryv4
is misleading). It copies old retainedCOM+3d00->prev+3d10 at54174, reads current
PhysicsRecordCOM atSkeleton+29d0, stores retainedCOM54190, and computes full4
(newCOM-prevCOM) multiplied by reciprocal-dt estimate with TWO refinements,
stored at+3d20/541b4. Only afterward processedflags8c8&1000 can recopy current
COM->prev at541d0. The suffix selects CalculateHookToHipsWithProxy82b4d688 or
CalculateHookToHips82b45778. Those consumers must be recovered before claiming
full UpdateRecords. Native constructor zeroes all three COM vectors at5732c,
5733c,57344 from the zero v62 defined56e80. Both ResetSkeleton and TeleportReset
preserve these three retained vectors, as independently audited below.

Author independently recovered CalculateErrors[82b3d408,82b3d498), SHA256
da2457d00a244ce42d0ad10dabc0fd2e6568c29f1fe3d4fd7064dbb406cfc010.
State+11c0 first stores error0=PhysicsRecord.proxyPosition(State+10b0) minus
State.ProxyVolTarget(+1340), full4. Existing state.proxy_volume_target is this
same authority (Skeleton+2c80), not a new tuning field. Then ascending parts1..23
transform adjustedvolume[part].P through AnimToWorld X-FMA intoP/YFMA/ZFMA,
and store PhysicsRecord.TRANSFORMS[part].P minus predicted. The source reads
transform translation, not the separate history positions array. Existing
AnimToWorld translationW can be unknown, so errorW1..23 must remain typed
unknown; error0 has known full4 inputs. No assertion/cvar/callee in this helper.

Existing normal owner already retains current mapped volumes, same physics
history, proxy target, collision data and roots. Missing persistent error/COM/
hook output projection and proper post-collision caller still need integration.
Body.disable_counts/volume_disable_count_active already exist as Option-valued
source state, with original full reset known; use that same authority for
UpdateVolumesStates, preserving unknown-before-initialization semantics.

Independent lifecycle/order follow-up:

- UpdatePostPhysics[82b575d0,82b578b4) SHA256
  75202f926bc7035f871f25d4c332fb8ab29d80f80a1de98e9010596279fc896f.
  It samples OLD retained velocity577e0 into collision info577e8; only after
  collision/volume states does UpdateRecords produce NEW velocity. Feed later
  landing and NEXT collision, not this same collision.
- ResetSkeleton[82b45bf0,82b45dd4) SHA256
  4105b4ae7a9094f55f033a43fb8bacbde2b8f26095077222a5f62ca95384dc64;
  TeleportReset[82b54228,82b542bc) SHA256
  08d3b8f82a3276ecdd112e66f4276db80b18d5ef9f45d98dd76b26494b8daa45.
  Both preserve all three retainedCOM vectors. Embedded SkeletonStateReset
  [82b3d238,82b3d408) SHA256
  01112c81d3b355c743f57bc0340cce6fc9ed0a304b3abfd1ecad1248be2bc82e
  operates within embedded+1940..2c80, not retained+3d00..3d20.
- UpdateVolumesStates[82b479d0,82b47aa4) SHA256
  13c97a45021a9da4480f8b34de24c5ce93e19939ef67484dc723ad12302f106e.
  Read active byteBody+1e60. If active, visit24 signed countersBody+1e00
  ascending: only positive entries decrement (store47a24), then enable volume
  flagbit0 if result0, clear if stillpositive (store47a68); nonpositive entries
  leave headers untouched. Final active flag is OR of remainingpositive47a98.
  Folded OnEnd825187c0 is empty. Known inactive permits source no-op, None
  must not become inventedfalse. Reads no collision output/COM input, but its
  position is part of full UpdatePostPhysics. Same small gate is also recorded
  in skeleton-drives-next-source.md. Use actual existing runtime authority.

Author suffix packet (source reviewed by author, full independent audit still
needed before implementation): both HookToHips paths select error Parts
[23,17,18,16,21,22,20,1] by squaredXYZ length. First compare is strict
err23²>err17²?23:17 (tie chooses17); later strict-greater candidates preserve
winner on ties. MaxDisplacement+3dd0 is s*rsqrt(s) with two refinements and
explicit s==0->0. Both read Reckoning+1f0 GROUND_NORMAL, not up+1b0. Existing
Reckoning retains ground_normal but lacks getter. Proxy path uses error0 and
optionally calls CalculateNormalDisplacement; normal path calls it8times and
selects max again. CalculateNormalDisplacement82b3e960 is absent and contains
safe normalization, signed-angle/wrap/cos/arbitrary-axis rotation that require
raw source review. Existing collision.normal_info/num_normals are the correct
data authority. Actual proxyEnabled+3e43 production remains unconnected; do
not infer it from volume flags, is_ragdoll or a controlled drive mask.

Reciprocal dt refinement graph: twice seed=seed.mul_add(
(-seed).mul_add(dt,1),seed), preserving the separate residual then FMA and
full4 final product. Processedflags8c8&1000 only restamps previousCOM AFTER
velocity is written; it does not recompute or clear velocity. This packet
must still receive implementation/test review when the next component is built.

Source ordering refinements from author: reciprocal estimate54184 occurs BEFORE
newCOM store54190. If host numeric estimation is fallible, preserve all error
rows and Prev=oldCOM but leave Current/Vel old on that failure. The normal hook
path's first max-length reduction starts17 then23 (tie17); after displacement
calls in order23/17/18/16/21/22/20/1, its second vector winner starts23 then17
(tie23), followed by strict-greater candidates. Do not reuse one tie policy for
both passes. Normal path reads DrawHookToHips twice, first before ground-normal/
helpers and second before HookToHips store; proxy reads normals toggle then
scaleY, stores HookToHips, THEN reads draw toggle. Unknown collision/error W
remains unknown through plane projection, including zero factors. CORRECTION:
the rotation suffix can recover known W from its source-generated rotation
matrix, whose permutation makes columnW duplicate columnX. It consumes only
projectedXYZ, so inputW does not contaminate that final known result. See
`skeleton-post-physics-records.md` for the independent raw audit.
