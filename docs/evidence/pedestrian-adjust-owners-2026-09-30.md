# Pedestrian retained adjust owners

Source confidence: **high** for constructors, the small head stack/straight
ahead methods, ground reception and spatial frame calculations. This does not
complete head/post-adjust updates or publish a live spatial task.
No compile, cargo check, tests, or runtime launch performed. The new Rust file
was syntax parsed with `rustfmt --edition 2024 --emit stdout --config
skip_children=true`; output was discarded, existing sources untouched.

## Source and ownership

Pedestrian constructor `82e97748` requests store7 at `82e97fd4`, calls
`cLivingWorldPostAdjust::cLivingWorldPostAdjust82e8aac8` at `82e980bc`, then
requests store8 and calls `cLivingWorldHeadTracking82e74250` at `82e98100`,
then requests store9/calls `cLivingWorldSpatialAdjust82e6b8b8` at `82e98108`.
Only afterwards does store3/NavRig allocation occur. All arguments use actual
pedestrian owners, not SkeletonHeadTracking (the separate skater subsystem).

Raw PPC `82e97c34` sets r25=this+0x1960. PDB identifies this as
`Pedestrian::mAttribulatorData` (`Attrib::Gen::livingworld_entities`). The
post/head references are therefore fetched from resources.entity:

- postadjust `639545955147d889`, assembled `82e9807c..90`;
- headtracking `14227727ead7f757`, assembled `82e980c4..d8`.

`MemStore::RequestBlock82a6e450` returns the selected block and marks its
freelist entry; this function does not clear the returned block. The broader
lifecycle audit establishes that Init82a6de38 and FreeBlock82a6df68 clear the
block, including reused slots. Consequently RequestBlock alone does not prove
stale native bytes. Typed `Option` fields currently mark values without a
translated member producer (head flee timers, jump trajectory/downward flag,
postadjust slots not visited by the array); this is a conservative ownership
check, not a claim that native pooled bytes are nonzero. Spatial defined flags
are also cleared explicitly by the constructor.

## Post-adjust

PDB: copied animation bind pose/hierarchy at+16; attribute instance+28;
six replay adjustments+44 (7 bytes each), bone yaws+96, max bone yaws+192,
head yaw+288, max head yaw+304, head pitch+320, max pitch+336.

Constructor retains actual bind pose and hierarchy, reads cached layout
MaxHeadPitch+12 and multiplies by native float `0.017453292`. The bone map key
is `43a2b449378045fb` (raw `82e8ab9c..b0`). Runtime length and element reads
reuse existing Vault readers. PDB `tPedestrianBoneAdjustMap` is enum+0 and
float MaxAngle+4. Original skater.vlt schema confirms type
`772b033a57a57dbd`, size8, array flag5. Constructor lookup order:

1. Read record at loop index, select enum slot.
2. Resolve `postAdjustBoneName[enum]` in retained hierarchy, narrow returned
   bone to replay byte. Table `83025110`: Head, Neck1, Neck, Spine3, Spine2,
   Spine1. Missing bone becomes255, matching native -1 narrowing.
3. Initialize that replay quaternion with compressed identity.
4. Read the array again at **enum**, store MaxAngle converted to radians.
5. Sum six maxima in slot order; initialize six current yaws/head yaw/pitch0.

`QuatToAxisAngle82e7c5a8` takes identity's small-angle branch and produces
yaw/pitch/half-angle0. `QuatCompressAxisAngle82e7c7c8` uses
floor(0.5*65535+0.5)=32768 for first two u16 components and0 for half-angle,
yielding bytes `80 00 80 00 00 00`. This is a constructor constant only; no
second general animation codec was introduced.

The implementation retains the partial PostAdjust before `construct()`. An
absent cached layout fails explicitly; the native generated null-layout
decompile has an ambiguous mCollection/mLayoutPtr write, so no fabricated
layout was supplied. A missing map pointer uses its proven DefaultDataArea(8)
zero record; a missing unwritten maximum currently stops construction. That
checked failure is stricter than the proven zero-filled native pool lifecycle
for sparse maps; all inspected authored maps initialize all six slots.

Read-only corpus inspection used existing `tools/vault_dump.py` on the
already-extracted `%TEMP%/skate-ped-vault/data/db/skater.vlt` and `.bin`.
All three postadjust collections contain six ordered enum0..5 records:
default maxima10/5/5/5/5/5 degrees; collection2b99ee5b669309db
8/5/5/5/5/5; bfbc545c9a73d32e10/5/5/0/0/0. These are inspection evidence,
not hardcoded runtime values. This was not a simulation test.

## Head tracking

PDB fields: resolved target+16, base mode+32, three20-byte stack entries+52,
stack level+112, attribute instance+116, point of interest+144, flee lookback
time+160 and boolean+164. Constructor zeroes both vectors, sets all stack
entries StraightAhead3, invalid targetable IDs(-1,-1), null behaviour source,
stack index-1; final base mode is WanderDynamic4. Flee fields and each stack
entry's flee timer are not written. `PushHeadTrackingMode82e6b4d8` admits
0/3/4/5/6/7, does nothing when full, and preserves the timer. Pop82e6b818
clamps stack level at-1 without clearing slots.

SetStraightAheadTarget82e744b8 requires the actual SimLODPose head height.
Its fused operation is up*height+translation, followed by forward addition.
Raw instruction82e7452c is11606b2e: A0, B13, C12, hence v0*v12+v13;
IDA's printed operand order must not be interpreted as v0*v13+v12.

Full Update82e85290, wander82e7ca08 and flee82e7caa8 were identified. Wander
uses live nav interim target with absolute bearing threshold80degrees; full
Update chooses targetable/behaviour/skater/flee modes then copies current
animation POI. These full callbacks are not implemented here; no placeholder
target or uninitialized flee timer is returned to impersonate them.

## Spatial adjust and integration boundary

PDB jump+16 contains initial/down velocity, landing/ledge points, initial
height/horizontal speed, elapsed/up/down times, jumping/downward bits.
Constructor initializes jumping=false and all three times0; other jump values
remain absent until JumpNow82e7d5f0. Ground normal+112 and up+144 are+Y,
ground+128/locked+160/sliding+176/last-good+192 positions are zero, surface+208,
upshift+212 and sliding speed+216 are0. All five semantic bits at+220 clear.

Spatial Update82e745a0 and ForcePositionToGroundPlane82e795e0 are now
implemented below; JumpNow82e7d5f0 was completed in the later follow-up. The actual ground
owner publishes reception into this retained SpatialAdjust; see
pedestrian-ground-dmo-2026-09-30.md. No collision or IK pipeline was duplicated.

Root integration must register `pedestrian_adjust_owners`, add retained fields
to PedestrianConstruction, reserve7/8/9 in order after animation and before
NavRig, and preserve partially constructed PostAdjust on failure. Root owns
all existing/shared source edits; this packet changed only its new module and
this evidence document. These states alone do not authorize live publication
or a claim that pedestrians are working.

## Follow-up: ground reception and SpatialAdjust.Update

`force_position_to_ground_plane` uses actual MeshStore and caller runtime
kDefaultNavMeshCheckYTolerance. Source vector assertions reject XYZ NaNs (W
ignored); an XYZ absolute position above1000000 returns without mutation.
Negative normalY flips all four lanes. Groundness is computed only for a null
hit owner and nonwater surface; actual mesh score at threshold0 must be<800.
Ground normal/position, valid-ground flag and surface are stored even for a
submitted miss. Last-known-good position changes only on qualifying ground.
Water `(surface&f80)==300` sets pulling-back; qualifying nonwater ground clears
it; all other results preserve it. Thus misses/DMO hits are not dropped or
silently converted to clear ground.

Raw ABI correction for `Update82e745a0`: dt is f1/r4's ABI slot, r5 is the
orientation, r6 collision, r7 applyCollisionGroundOffset, r8 applyCollisionSliding,
r9 NavRig. Hex-Rays labels r5 as a collision object and shifts later argument
names; its struct field expressions therefore must not drive implementation.
The corrected owner reads orientation+48 as position and+16 as current up.

Update resets DidSlide first, then exits when neither valid ground nor jumping.
The priority is locked position, jump, pullback, normal ground correction.
Normal correction lerps from orientation up to ground normal (or world up when
ground-offset flag false) by0.1, applies unsafe two-refinement normalization
through the shared ReckoningNumeric owner, saves UpVector, and applies
`((1 / kYShiftTimeValue) * (groundY-positionY)) * dt` along that up vector.
There is no invented clamp on dt or ground snap. A scripted slide normalizes
XZ(delta) and replaces the just-computed ground correction with original
position+direction*(SlidingSpeed*dt), preserving native ordering.

Collision sliding reads actual PedestrianCollision constructor fields added by
root. It is entered only with applyCollisionSliding, either RequiredToBeginColliding
or Colliding, world part0 NOT colliding, and skater part0 colliding. The lazy
`PedestrianCollision::skater_part_transform(0, world)` call then fetches the
actual part transform; all its axes get native
vector validation. Position difference is clamped per lane to
[-kSlidingClampAmount,0,-kSlidingClampAmount,0] and its positive bound. When
a rig exists and dot3(shift,shift)>0, the existing heading-distance calculation
is reused. Shift is zeroed if its squared length exceeds clearance squared;
it is not shortened to fit. DidSlide is any XYZ absolute shift greater than
float bits34000000 (loaded from823953d0). The final position store precedes
orientation validation, as in native. Native IsValid mesh-cell return here is
discarded and does not gate movement.

Jump integration increments elapsed time before gravity, selects the actual
up/down initial velocity, forms Y*(velocityY-elapsed*9.8000002)*dt plus the
horizontal velocity*dt, then adds original position. Pullback replaces that
movement with NormalizeSafe(lastGood-position)*pullbackSpeed*dt. Up-phase
completion uses elapsed>=upTime, then optional ledge proximity, then optional
landing proximity (strict squared distance-radiusSquared<0); it sets elapsed0
and downward. Down-phase completion clears jumping using downTime or landing
proximity. Missing produced trajectory/phase remains an explicit error, not a
zero-velocity jump. JumpNow's solver/producer is covered by the later follow-up.

`SpatialAdjustTuning` fields must be populated by the retained live cvar owner:
kPullbackSpeed, kYShiftTimeValue, kSlidingClampAmount, kJumpUseLanding,
kJumpUseLedge, kJumpArrivalRadius. Root subsequently assigned exclusive edits
to input_cvar_runtime.rs: all six are now registered/linked with native startup
defaults and original labels/titles, and
`try_pedestrian_spatial_adjust_tuning()` reads the live targets on each call.
Task scheduling and actual admitted base AssemblyPool part access remain root
integration obligations. Shared host dot/refinement/VMX min-max exceptional behavior remains
a numeric parity boundary; no reference tick comparison has been performed.

Follow-up syntax parsing passed with the same rustfmt stdout-only command.
No compiler, test or game run was performed.

The six native cvar initializer addresses/defaults/UI ranges are:

| Label | Initializer | Default | UI range |
| --- | --- | --- | --- |
| kYShiftTimeValue | 82fa6580 | .12 (3df5c28f) | .001..1 |
| kSlidingClampAmount | 82fa67f0 | .1 (3dcccccd) | 0..2.5 |
| kPullbackSpeed | 82fa6850 | 12 (41400000) | 0..20 |
| kJumpArrivalRadius | 82faa178 | .1 (3dcccccd) | 0..5 |
| kJumpUseLedge | 82faa408 | false | bool |
| kJumpUseLanding | 82faa458 | true | bool |

First three belong to Pedestrian_SpatialAdjust, last three to
Pedestrian_Pathogen_NavRig_Jump. Raw f1/f2/f3 loads corroborate float defaults
and ranges; BoolRef calls supply0/1. Shared serializer remains unchanged and
does not impose UI float-range clamps on loaded values. Stable new indices
1055..1060 preserve all prior indices; appended unique section/label pairs
retain their relative initializer order. Both modified Rust files parsed.

## Follow-up: complete jump trajectory producer

New child `pedestrian_jump_trajectory.rs` is registered directly by the adjust
owner. Existing sources were searched for SolveForInitialVelocity and jump
trajectory solvers; no implementation of this pedestrian geotest routine was
found. Skater offboard jump logic is a different source graph. The new routine
reuses `ReckoningNumeric`, shared reciprocal/refinement functions and shared
native length graph; it adds no physics pipeline or host sqrt substitute.

`Pedestrian::SetJumpToPosition82e701f8` calls actual Entity.GetFacing and
GetPosition. It forms ledge=position+(facing*ledgeDistance), with multiply/add
separate, stores authored landing, zeroes both velocities and all five scalar
fields, clears jumping/downward. This is the concrete writer for every member
that the constructor deliberately left absent. `set_jump_to_position` now
implements these writes; `jump_now` errors before a missing prior producer.

Entity `JumpNow82e80f90` has an important interface adjustment: this is
IPedestrian at entity+172. Raw this+20 points to Chaser at entity+192, whose
virtual slot+128 is **GetMaxChaseSpeed**. It is not current locomotion velocity
or IMotionGraph. Source then calls Entity.GetPosition via this-172 and forwards
both into SpatialAdjust.JumpNow. Root's graph/entity adapter must use this
actual speed producer.

JumpNow82e7d5f0 projects initial-to-ledge XZ onto initial-to-landing XZ. A
projection<=EPSILON or not strictly below landing distance squared selects
the fallback ledge initial+landingXZ*kJumpLedgeFailureDistance (fused).
It then raises ledgeY to the greater landing/initialY plus clearance, resets
elapsed, sets jumping/upward, and saves initial height. Horizontal speed is
`(((abs(height)-minHeight)/(maxHeight-minHeight))*(maxRatio-minRatio)+minRatio)
*GetMaxChaseSpeed`, with the ratio multiply/add fused and no ratio clamp.
The up/down height bounds are selected by native fsel height>=0.

With UseLedge, it solves initial-to-ledge into initial velocity/upTime. With
UseLanding too, it solves ledge-to-landing into downward velocity/downTime.
Without UseLanding, it copies the whole initial velocity into downward velocity
and sets downTime=horizontalDistance(ledge,landing)*(1/horizontalSpeed).
Without UseLedge, it solves directly to landing, copies the whole velocity
unchanged into downward velocity, then uses upTime=velocityY*float3dd0fac6
for velocityY>=0, otherwise float3d088889 (1/30). DownTime is total-upTime if
nonnegative, otherwise that same1/30. Zero initialY selects upTime0, not1/30.
These unusual whole-velocity copies are native, not a generic ballistic redesign.

`geotest::SolveForInitialVelocity82e56468` initializes velocity0/travelTime-1,
returns that result if horizontalSpeed is not positive or no horizontal lane
exceeds EPSILON34000000, otherwise computes horizontal length through the
native two-refinement rsqrt graph and time=refinedReciprocal(speed)*length.
It separately refines reciprocal(length), multiplies each horizontal lane by
that and speed, and replaces Y with `(1/time)*deltaY + time*4.9` fused.
The scalar1/time division must not be replaced by the vector reciprocal path.
The constant4.9 is409ccccd; both jump epsilons are34000000 at8238b0c0 and
823953d0. Degenerate zero/-1 results are preserved rather than rejecting jumps
or inventing a minimum horizontal speed.

Root granted another exclusive edit to input_cvar_runtime.rs. New live
`try_pedestrian_jump_tuning()` shares existing UseLedge/UseLanding targets and
registers these original float constructors under Pedestrian_Pathogen_NavRig_Jump:

| Label suffix after k | Initializer | Default | UI range |
| --- | --- | --- | --- |
| NavRigJumpUpHeightMinHeight | 82fa9f38 | .3 | 0..5 |
| NavRigJumpUpHeightMaxHeight | 82fa9f98 | 1 | .01..5 |
| NavRigJumpDownHeightMinHeight | 82fa9ff8 | .5 | 0..5 |
| NavRigJumpDownHeightMaxHeight | 82faa058 | 1 | .01..5 |
| NavRigJumpMinHorizontalRatio | 82faa0b8 | .8 | .01..1 |
| NavRigJumpMaxHorizontalRatio | 82faa118 | .4 | .01..1 |
| JumpLedgeClearance | 82faa1d8 | .15 | 0..5 |
| JumpLedgeFailureDistance | 82faa238 | .5 | 0..1 |

Also registered `kPedestrianPoolUpdateInParallel`, section
Physics_Pools_PedestrianPool, title Update In Parallel, defaultfalse from
initializer82f6a580; getter `try_pedestrian_pool_update_in_parallel` serves the
separate base AssemblyPool used by InitWorld827b4360. The new stable indices
are1061..1069 and definition count1070. Existing targets/serializers are reused.
New child and both adjusted parent/runtime files passed stdout-only rustfmt
syntax parsing. No compile, check, tests or runtime launch occurred. Entity
motion-graph delivery and end-to-end jump traces remain root integration work;
this packet establishes a concrete trajectory producer, not a live jump claim.

## Concrete entity jump callbacks

`pedestrian_entity_jump.rs` supplies SetJumpToPosition and JumpNow methods on
the retained PedestrianConstruction. SetJumpToPosition reads actual core
transform Z and translation in native order. GetFacing8258f128 obtains actual
Entity.GetTransform and returns its axis+32; GetPosition82e94640 returns the
core translation. PDB LWEntityCoreData places mTransform at16 and translation
at64, separate from mTargetTransform at80. No navigation cache substitutes for
the actual entity transform.

GetMaxChaseSpeed82e6f378 enters through Chaser+192 and reads +18b0/+18b4,
resolving entity mChaseParameters collection/layout at6512. PDB
Attrib::Gen::livingworld_entities_chase::_LayoutStruct identifies float
MaxChaseSpeed at0. The wrapper reads that cached layout using the existing
AttributeRuntimeInstance field reader and reports absent collection/layout;
it does not substitute current velocity. JumpNow reads that speed before
actual position, obtains the registered live jump cvars, then calls the retained
SpatialAdjust trajectory producer. Root registers the child module. Only
stdout syntax parsing was performed.

The next navigation adapter cannot faithfully return only seek_direction:
GetNavTargetDirection82e8b320 includes ordered active modifier target selection,
five retained direction samples, NavRig best-ray selection, jump-clearance and
mesh tests, and ChaseJump packet insertion. These are identified follow-up
owners, not implemented behavior in the entity jump wrapper.

## Six navigation modifier owners and skater avoidance

New `pedestrian_navigation_modifiers.rs` retains separate ChaseGlue, General,
Skater, Vehicle, DynamicObject and BigBlack records. Constructor82e97748 list
order is exactly that sequence. Embedded constructors precede the road target
iterator; after NavRig allocation all six Init methods receive MeshNavigation
even though the active navigation is initially PhysicsNavigation. The explicit
initialize_with_mesh method records that binding; callers supply that same
retained owner at use sites. Entity itself is allocated from MemStore0, so
Init82a6de38/FreeBlock82a6df68 zeroing supports constructor-omitted zero members.
This differs from treating RequestBlock as the sole lifecycle operation.

General ctor writes target0, mask119, null pointers, disabled active flag and
enabled=true. Skater writes null pointers, active/intent=false, enabled=true.
DMO similarly writes its separate byte flags. Vehicle ctor82e8c530 writes
target0, null pointers, mask119, active=false, enabled=true and intersection
status2 (PDB eIntersectInfo_INVALID;0 is Parallel). BigBlack82e8cf40 writes
target0, null navigation/target, active=false, enabled=true and last-left=false.
ChaseGlue constructor writes enabled=true and creates its embedded packet.
Three unmanaged packets are registered in the existing IntentQueue arena in
native constructor order Vehicle/BigBlack/ChaseGlue; no second queue exists.

ResetNavModifiersToDefault82e6d460 reads active navigation layout bytes in
order4,2,1,3,0,5 for ChaseGlue/General/Skater/DMO/Vehicle/BigBlack. The method
retains earlier enabled writes if a later field read fails. Per-evaluation
resets preserve pointer/recommendation payloads unless native writes them:
General82e6c498 clears four flags, Skater82e6c630 clears two, DMO82e6c7f0 clears
two, Vehicle82e6bf78 clears four and restores mask119, BigBlack82e6c800 clears
four. ChaseGlue82e6c4a8 clears target pointer/active/recommendations/explicit/
glue flags, speed and timer; max-turn becomes4.25, while tuning stays intact.
Vtables confirm Skater/DMO never recommend targets or speeds; General max
rotation is the shared constant-zero body82e6c588. General/Vehicle use their
stored masks and other modifiers return119. Intent getters assert nonnull
recommended packets when their recommendation flag is set.

ChaseGlue::RecommendTarget82e77360 is fully translated. It retains actual
target ownership, clears three output flags, handles an explicit target first
and skips timer decrement in that branch. Otherwise full XYZ distance comes
from bound MeshNavigation.position to actual target position. Inside the strict
activation range, distance above the optimal range recommends max speed;
distance below it starts glue timeout once and recommends mesh navigation
speed*0.75 while timer>0. Active glue decrements timer even outside activation.
Equal distances select neither recommendation. The method has no internal
enabled test: original entity traversal supplies that gate. Tuning fields still
need their live source writer before the normal branch can activate.

`pedestrian_skater_avoidance.rs` translates RecommendTarget82e973f0,
Condition_ShouldAvoidSkater82e6c648, ResolveSituation82e93de8 and the simulation
portion of HandleBurdened_SkaterAvoid82e8f5d0. It consumes actual entity position,
facing, target position/velocity and target PhysOut facts; no player defaults.
PDB resolves the large-avoid exclusions as PhysOut_Filtered.mFilteredPhysicsState
==6 and PhysOut_Scoring2.mHoM_BailInProgress at17985. The source first blocks
when the existing avoid-jump packet is queued, then tests XZ influence radius
and XYZ target speed, then closest-point time and outer radius. Raw bgt/blt
gates deliberately preserve unordered continuation. The handler tests inner
radius and big speed, blocks repeated step avoidance, and uses the existing
signed Math::AngleBetweenVectors implementation to select Forward/Left/Back/
Right with the original thresholds. It uses shared NormalizeSafe and real
MeshStore when nav-mesh-only is enabled: valid candidate cost must not exceed
current NavRig cell cost; invalid current cell fails explicitly.

Existing segment closest-point helpers clamp endpoint proportions and cannot
replace geotest::ClosestPointToRay82e54a28. The added primitive shares the
existing reciprocal estimate/two-refinement owner and preserves source
Y-multiply, X-FMA, Z-FMA numerator order, full XYZ denominator, unclamped time
and fused point construction. Both Skater and General conditions use it.
General HandleBurdened_Stop82e770f0 is also concrete; other General situation
resolution and avoidance handlers remain follow-up work.

Successful avoidance writes the existing unmanaged AvoidJump/AvoidStep packet
with one stage [Avoid, Jump|Step, direction], rotate=false. Added
IntentPacket.replace_single_stage mirrors82e8c410/82e8c4a0: only stages/count
change, retaining active index, state, in-queue and managed fields. Large
avoidance returns the mandatory event3 mood side effect with actual
perpetrator/victim IDs and victim position; original exposure/magnitude are1/1.
The caller must deliver this event before claiming complete evaluation.

Registered live cvars1076..1088 via the shared runtime, new count1089:

| Initializer | Label suffix | Default | UI range |
| --- | --- | --- | --- |
| 82fa7178 | AvoidOnNavMeshOnly | false | bool |
| 82fa7218 | NavMeshLargeAvoidDistance | 1.4 | 0..5 |
| 82fa7278 | NavMeshSmallAvoidDistance | .8 | 0..5 |
| 82fa72d8 | AvoidNavMeshYTolerance | .75 | 0..5 |
| 82fa7528 | DebugSkaterAvoidanceNavModifier_AvoidNPCSkaters | true | bool |
| 82fa7578 | DebugSkaterAvoidanceNavModifier_PredictTime | .75 | 0..15 |
| 82fa75d8 | DebugSkaterAvoidanceNavModifier_OuterAvoidRadius | 3 | 0..15 |
| 82fa7638 | DebugSkaterAvoidanceNavModifier_InnerAvoidRadius | 1 | 0..15 |
| 82fa7698 | SkaterAvoidanceNavModifier_influenceradius | 10 | .1..360 |
| 82fa76f8 | SkaterAvoidanceNavModifier_FrontAvoidHalfAngle | .5236 | .01..1.5708 |
| 82fa7758 | DebugSkaterAvoidanceNavModifier_MinVelocityToStartAvoid | 1 | 0..15 |
| 82fa77b8 | DebugSkaterAvoidanceNavModifier_MinVelocityForBigAvoid | 5 | 0..15 |
| 82fa7868 | DebugSkaterAvoidanceNavModifier_AllowHandleAvoidSkater | true | bool |

First four sections are Pedestrian_Navigation_Modifier; remaining nine are
Pedestrian_Navigation_Modifier_SkaterAvoidance. Labels retain their leading k.
Getter try_pedestrian_skater_avoidance_tuning captures live targets.
SkaterModifier.refresh_tuning reproduces EvaluateNavModifiers82e80110..120:
predict+68, inner+60, outer+56, front-angle+64 and big-speed+76; it deliberately
does not write minimum-speed+72. Original caller performs this refresh after
enabled check even when there is no local skater.

HeadTracking.NotifyEntityCulled82e6b840 now visits stack0..=StackLevel only.
Matching targetable(kind1,entityID) becomes invalid/None, StraightAhead and
null behavior source; base/dormant slots and flee timer are preserved. This
serves the actual EntityMan roster cull notifications, not a parallel roster.

Syntax parsing only. Remaining integration: concrete evaluation traversal and
actual PhysOut facts/mood delivery, normal ChaseGlue tuning, full General/DMO/
Vehicle/BigBlack recommendation updates and stateful IntentGen navigation
direction adapter. These owners do not establish live pedestrian avoidance.

## Stateful direction adapter and synchronous packet mutation

Follow-up implements the direction adapter in `pedestrian_navigation_intent.rs`
and GetBestNavRay82e51b10 in `pedestrian_best_nav_ray.rs`. Both are children of
the registered modifier module. NavigationIntentState retains original five
direction samples, initialized bit, explicit direction and distance razor;
root attaches it to the concrete entity. Constructor82e8e3f0 clears flags and
razor, then first direction query fills samples from supplied orientation Z.

GetBestNavRay shares existing heading_distance/closest_ray and clearance array
owners. Raw82e51b9c calls CRT double acos(dot), followed by frsp; this is not
XMVectorACos and the dot is not clamped. The zero-direction check ignores W
through vrlimi W<-X and reads CR6.EQ(all-false). The angle threshold is3d567750.
Early successful near-forward clearance preserves caller rayId; successful
heading clearance and exhausted search set rayId0. Fallback expands both
adjacent fronts, ties select plus, and turn threshold is advisory. Source
InitializeRig82e52278 stores degree values [0,-11,-55,-125,125,55,11], while
fallback comparisons subtract signed acos radians directly. This observed
mixed-unit calculation is retained. Scalar ble/bgt unordered behavior is
preserved independently of vector-all-true comparisons.

GetNavTargetDirection82e8b320 starts with selected Navigation.seek_direction,
uses the first active modifier with valid X/Z and full squared distance>.01,
normalizes the delta, then clears only Y. ExplicitNoSeekNoAstar, disabled
override, active SpatialAdjust jump or queued jump packet bypass rig selection.
Otherwise it calls best ray, existing HasToJump, actual facing/position and
MeshStore. The landing query occurs even when kCheckForJumpNavMesh is false.
Allowed landing writes existing SpatialAdjust.SetJumpToPosition, clears only
native jump packet reset fields, builds ChaseJump and synchronously calls the
existing IntentQueue.suggest. That branch returns before history updates.
Other branches use real selected nav speed and entity velocity for low-speed
suppression; over-turn calls AI set_timer(14,4), exactly82e9aeb8.

PDB/interface correction: GetMaxTurnRadius82e6eb48 enters IPedestrian+172;
load+1960 resolves entity mpNavigation+6668, virtual88 is GetCurrentTargetIterator,
then iterator64 is GetMaxTurnThreshold. Null iterator alone returns180.
IsAbleToJump82e94cd0 similarly reads NavRig.mbDoJumpTest+321; adapter now reads
existing rig.distances.do_jump_test directly. Entity allow-overrides bit10 and
smoothing bit20 at6724 are distinct from navigating bit80. Base ctor sets first
two; adapter requires their actual retained values. GetTurnMask virtual100
resolves shared constant119.

Direction history uses three samples for core.current_losim==2, otherwise five.
Raw82e8bc18 copies candidate into accumulator, then82e8bcc0 adds sample0
(candidate again) and remaining samples before multiplying1/N. This native
extra candidate weighting is retained; no normalized average was substituted.
Updates occur even with smoothing disabled. SetCharacterRotateToTarget reloads
4.25 at82e8be64, ignoring modifier maxRotateAngle; strafe direction writes0.
Bearing shares existing two-refinement numeric helpers and XMVectorACos;
dot/estimate evaluation order and raw cross sign are preserved. Strafe intent
selection retains scalar branch fall-through for unordered values. Clearance
razor uses native fused fmadds82e8e9a0.

PacketNavigation/BaseNavigation methods now receive mutable queue/AI input
synchronously. A StageRotation return updates only target/rotate/max fields;
the caller reacquires its original stage slot after suggestion, so resetting
base during ChaseJump insertion cannot be undone by stale whole-stage copies.
The turn stage is copied only at the native copy points. Update82e937b0's final
motion output retains the preselected packet identity across callbacks instead
of reselecting the newly changed queue front. A removed managed packet fails
explicitly rather than accessing freed ownership.

Additional live cvars1089..1091 are execution rates HiFi30/LoFi10/NoFi3,
range0..60, section Pedestrian_Sys, constructors82fa6308/6360/63b8. Native
LoFi/NoFi display titles are swapped and retained. Getter returns raw unsigned
words in source Init82e9ef00 order. Direction getter registers1092..1101:

| Initializer | Label | Default | UI range |
| --- | --- | --- | --- |
| 82fa3db8 | kJumpDownLedgeDifference | 0 | 0..5 |
| 82fa8d58 | kNavRigBestRay_DoSmoothing | false | bool |
| 82fa9338 | kBestNavDirectionDistanceInital | 1.5 | 0..8 |
| 82fa9398 | kBestNavDirectionDistanceScalar | .25 | 0..8 |
| 82fa93f8 | kNavRigUsageVelocityThreshold | 1.4 | 0..10 |
| 82fa9e28 | kNavRigJumpHeightHeight | 2 | 0..5 |
| 82fa9e88 | kNavRigJumpHeightLead | 1.5 | 0..5 |
| 82fa9ee8 | kNavRigJumpOverride | true | bool |
| 82faa5f8 | kCheckForJumpNavMesh | true | bool |
| 82faa648 | kJumpNavMeshYTolerance | 1 | 0..2.5 |

Definition count1102. Getter try_pedestrian_navigation_intent_tuning reuses
existing authored jump-height-bound cvars. Syntax parsing only; actual frame
execution and candidate/mood integration remain root-owned work.

### All89 condition bindings and factory suffix cvars

`pedestrian_graph_condition_binding.rs` binds each exported condition occurrence,
with an error for an unknown declaration. Common conditions, entity collision/
RAIN conditions, guard state predicates, and ChaseConditionFrame remain their
existing owners. Comparison constructor82cf7ae8 and evaluator82ceeea0 are reused
from generated_action_load, including native unordered comparisons. Conditions
do not call a generic name-to-Boolean fallback. Parent registered this new child.

New constructor/activation traces: IsFacingWantTarget826efb70/8270c2a8 converts
degrees then halves the angle, zeros both position Y lanes before subtraction,
and calls shared Math::AngleBetweenVectors829b0628 with flattened facing and +Y
wind. Distance cap is strict `squared-distance²>0`; final angle comparison is
`angle<half || !(angle<=2pi-half)` (raw8270c468 includes unordered success).
DistanceToWantTarget826f2120/826f21e0 squares the shared comparison reference,
uses full XYZ distance, and returns false for an absent target. Alert condition
826f1f60/826f2010 instead returns true for absent target, reads actual chase
attribute314597eda6b921f2, and tests strict squared distance less than range².
IsEndChaseReason826f00f0 parses the existing enum helper.

NeedToRest8270c7f0 resolves IChaser slots37/38/41 through pedestrian vtable
823a4c90 to82e6fc78/82e89a30/82e6fd40. Exhaustion reads actual chase attribute
4686fec68a60bd69, strict limit<state.exhaustion. Resting is state.flags[1]bit2;
rested is !exhausted && timer4<=0 (unordered timer remains not rested).
InvestigateTimeExceeded826f0200 is timer1<=0. IsChasing826f1488 calls IChaser
slot3 GetChasee82e86220, so resolved chasee existence is the predicate.

HasHandProp82e70040 is state.flags[2]bit2 OR state.flags[3]bit2; the first
requires an actual mpHandProp (native assertion), whereas the second permits
virtual holding. CanAttackThrowHandProp8270cd80 then reads handprop cached
layout byte62. IsHoldingSpecificHandProp826f1e38/826f1eb0 hashes the authored
name (default empty string) and compares the full64 collection key (`cmpld`,
not Hex-Rays' truncated32 view). Added minimal AttributeRuntimeInstance
collection_key accessor with parent authorization. Metadata selection alone
does not imply possession. Plugin collision override82705ef0 reads existing
state.override_external_collision at+C60.

Explicit ConditionServices operations await session connections for plugin
presence, actual physical handprop presence, road iterator HasTarget/IsCrossing/
CanProceed, targetable position lookup, and patrol-interest boundary/radius.
The last two activation bodies8270f210/8270f310 intentionally differ: missing
valid zone/interest target yields true for boundary, false for radius. Existing
RoadNetworkTargetIterator implementations remain sole road logic; peer is
attaching retained iterator owners. No graph readiness claim follows from
these bindings without the concrete session callback adapter.

Factory suffix registrations1102/1103 extend the existing runtime count1104:
kGroundPlaneTestDistanceUpDefault ctor82fa66e0, sectionPedestrian_SpatialAdjust,
initial1.6 UIrange0..10; kAllowHandProps ctor82faad90, sectionDMO_HandProps,
initialtrue. Getters try_pedestrian_ground_test_distance_up_default and
try_pedestrian_allow_hand_props supply factory reads. Files syntax parsed with
stdout rustfmt only; no build/check/test/runtime performed.

### PedestrianColliding lifecycle

New graph child pedestrian_graph_colliding.rs follows vtable820cef54:
constructor826eabe8 (warnChance.7,broadcastradius20), CreateInstance82711d20,
Begin82716648, Update8271ae50, End827074c0. PDB tCollidingData32-byte layout:
Instance+0..7,RecoveryTimer+8=1,State+12=Standing0,recoveryStage+16=0,
attempts+20=0,getupDirection+24=Front0,disabledModifier+28=false. Hex-Rays
mistypes float ABI helper arguments; raw standing/falling/recovery routines
pass entity in r5, collision object in r6, retained instance in r7.

Begin creates the unmanaged collisionintent packet in the existing queue,
disables modifier index2 (Skater) only if enabled, processes event flags,
then builds Collision/Knockdown-or-Standing/FromDirection. Standing pushes
targetable head tracking; actual holding flag alone gates the native global
Bernoulli/drop callback. Knockdown builds Recover and the retained recovery
collision stage, loads selected animation recovery time and up-shift, pushes
NoHeadTracking, sets state.flags[1]bit4, sends guard notification, performs
career/player-collision GoodSamaritan conditions, and throws the handprop.
Suggestion uses the existing queue, then Colliding=true and navigationmode2
(Physics). This is not a LoSim switch.

Standing826ead78 clears required flag and waits for original packet completion.
Falling82714c40 decrements the retained recovery timer; another unforced hit
resets that timer; otherwise moves to recovery and clears spatial up-shift.
It increments attempts after either expired branch and copies attempts into
the collision owner. Nonexpired falling returns to packetstage0. Raw bge
means an unordered timer follows the expired path. Recovery82716d28 writes
collisionflag20, observes repeat hits, rewrites only AnimIntentCount on the
retained stage (preserving stage controls), and uses LyingDown/Crouched/
Standing intents. Transitioning leaves just Collision/RecoveryCollision.
It advances the SAME packet, returns to falling and reloads timer/up-shift.
Complete state3 is an assertion if Update is incorrectly called again.

DetermineGetupCollisionType826fd728 uses original IAnimatable transition/time
owners; right and left both use WipeoutLeftGetUpCollisionIntervals. Attributes:
recoveryTime ad3c483f0c9dad67,upShift a062915263ce2bec,frontIntervals
e37f00409deab4d2,backIntervals74c0b72217a7704e,leftIntervals0162b7a9b0c086ee.
Absent generated attributes use DefaultDataArea; no invented timing fallback.
Raw bge at826fd850/86c preserves unordered fallthrough to earlier interval.

End releases the monitor before head pop, restores modifier only if this
instance disabled it, clears collision/required/forced/attempt count, removes
actual WORLD body only when world-only-in-HiFi and core.current_losim!=0,
clears knockdown bit and sets collisionflag20. It does not remove skater body,
clear all intents, or replace body transforms.

ProcessCollisionFlags82714b80 requires flags10+80; flag40 updates player
collision stats, SetDidCollideWithPlayer82e94cb8 updates PedStateOut bit40,
Broadcast82711df8 sends mood event10, magnitude/exposure1/1, actual position,
perpetrator and victim. Constructor warn/radius and force argument are unused
by this native broadcast. Guard knockdown message definition8216c79c is
cMsgSecurityKnockedDown kind88c76dfa with entityID payload. GoodSamaritan's
fallback type check is LWBasicEntity::GetType82e4bfd0 => core.entity_key.

Shared queue create_monitored/release_monitored implement82e9c130/82e9a338/
82e8dc60. Native fixed_map has overflow enabled (no artificial capacity16).
Release transfers an active/inqueue packet to managed queue lifetime; otherwise
destroys it. IntentPacket exposes exact stage mutation and increment-before-
assert advance. Added HeadTracking.push_targetable82e6b5a0: validated mode,
actual target resolution, capacity assertion, and native storedCenterOfMass
even for requestedVisibilityPoint. Invalid/missing target pushesStraightAhead.

Entity collision_info mirrors native+A98/A9C, initially zero from MemStore
clearing (no ctor stores). The explicit record_accepted_full_collision method
implements source82e95c60..d74 only AFTER original contact/velocity/direction
and knockdown classification. Full OnEntityCollision82e95158 was audited;
its producer still requires integration with actual live Part velocities and
animation thresholds, not a fabricated contact Boolean. External animation,
handprop holding/throw, stats, challenge and mood callbacks remain concrete
CollidingServices obligations. mHandPropUsageData.DropOnCollisionProbability
is usage+24, NOT constructor spawn-list probability; peer confirmed distinct
later holding producer is required.

Targeting peer's cvars also registered: kDoRAINPedIteration82fa6da8 true,
kAllowIncompletePathInChase82fab450 true,kChaseNavFailDistance82fab4a0 10
(UIrange0..100). Runtime definition count1107; live getter names communicated.
All changed Rust files stdout-rustfmt parsed only, no compile/check/test/run.

### Actual full-contact producer and physical velocity read

The later contacts-child implementation now provides the missing full-response
producer, superseding the earlier suffix-only limitation above. It accepts the
shared sealed PhysicalContactInfo, actual other-pedestrian reference, selected
regular/override animation instance, and the same PhysicalAssemblyWorld.
It runs after the existing determine_collision_type selects Full, with no
second collision classifier or synthetic contact representation.

record_collision_contact_owner implements82e951ac..953d4 before response:
skip self contacts and nonLight/nonFull; an actual owner sets flag80; local
IActor sets flag40; its GetPlayerIndex writes PPCshift scoring bit. Actual
ITargetable collider selection preserves TakeDownSink priority, local-player
priority, and strict greater collision type; absent ITargetable still retains
actor/scoring effects. GetTakeDownSinkId is native ILWTakeDownSource slot4.

resolve_full_collision implements82e957d8..95d74. ForcedRecovery exits before
body reads. SKATER Part0 Body+32/+48 supplies linear/angular XYZ; shared
length_from_squared_and_estimate preserves dot/estimate ordering, two native
refinements, and zero mask. Authorized minimal Parts/world/collision getters
reuse the original rigid-body pool observations and validate body lifetime and
Island membership. Detached Part with no body is an error; there is no entity
velocity, relative contact speed, or transform fallback. Host W is padding;
this consumer uses only XYZ, not the native adjacent intrusive link.

Entity attributes MaxLinearVelAfterCollision c04236fb548697d0 and
MaxAngularVelAfterCollision797aa1d5f828819b preserve lazy second lookup and
raw bgt/ble unordered behavior. Direction normalizes only actual facing from
GetTransform82e94670 (core+16), zeros contact normalY without renormalizing,
uses shared XMVectorACos/clamps and cross sign, then native35/145-degree
branches. Back is strict between±35; boundary±35 is side; outside±145 Front.

Selected animation cached layoutbyte17 plus AIflags[1]bit20 gates knockdown.
Other pedestrian IsAttemptingTakedown82e6fdb0 reads its flags[0]bit2; success
increments THIS victim's takedown_count+C90 before further attribute reads.
Parent authorized Some(0) constructor initialization from State MemStore zero;
Reset still preserves this cumulative counter. Both ordinary threshold reads
precede pedestrian overrides, even forced takedown: linear541ffa2e9d81c947,
angular7c5e39ece5a5572e,pedLinear081f978f8706d77b,pedAngular5e566360fdc5fdd1.
Standing rejection uses flags[1]bit10. Accepted knockdown overwrites collision
info even if already colliding; standing only overwrites while not colliding.
Required=true/collisionflag20clear precede live cvar reads; forced-recovery
updates only for configured skater force or non-skater contact; DMOinteraction
bits40/80clear only when bit40 was set. Earlier state persists on read failure.

New live cvars1107/1108: kNumberOfRecoveryAttempts82fab500 initial5 range0..50,
kForceRecoveryIfCollidingWithSkater82fab558 initialtrue, count1109. Integer
cast matches unsigned cmplw source. The full contact call is ready for the
retained pool's callback dispatcher; world-adjust/light-channel branches and
session contact traversal are separate integration work, not silently claimed
complete. All touched files syntax parsed only; no builds/tests/runtime.

### Contact response dispatcher and live traversal

OnEntityCollision82e95158 now has one all-kind dispatcher. AdjustWorld/Vehicle read WORLD Part0 actual transform, subtract SimLODBoneGlobal[8] offset only when **IsRenderable82d1ceb8** (not IsHiFi), replace Y from target, validate XYZ million-unit bound, and invoke existing target setter even for rejected candidate. DMO interaction sets state.flags[2]80. Light gate raw82e95598 reads cPedAnimData channel status7; remaps actual retained animation data with shared RNG, calls existing channels.play(.2,.2,false), then SetAudioState82e94ca8 writes8. Full invokes the existing actual Part velocity classification producer. Self-collision skips metadata but does not suppress native classification/response.

LivingWorldGrindServices.pedestrian_resolve_contacts consumes sealed existing Island contacts for admitted WORLD/SKATER handles and invokes classifier then dispatcher for every contact: GetContacts82e50e60 only breaks for true; pedestrian returns false. Contact clones preserve same Spy frame and body identity. Actual owner resolution uses Island; Actor.GetPosition825f9df0 uses PhysOut Skeleton root translation+1a0 rather than COM. Model radius82e51708 uses half X/Z dimensions and fsel(X-Z,X,Z).

Native retained fields added: renderable false (LWBasicEntity82e614b4); ignore_collision_target invalid (+1950/54 at82e97c48/50); interacting_object None (+1a30 at82e97d2c); adjust_world_collisions TRUE (raw82e97d44 inserts7<<4 into byte1a44). Existing ChaseSettings/ChaseeState are attached before entity attributes, following base Chasee825f6990->Reset82de2310, with authored global formation values. Cvars skater/ped ignorance16/2 and ranges.01..100 verified initializers82fa6188/61e8, registered indices1109/1110.

Remaining parity boundaries: host traversal currently uses EntityMan order with WORLD before SKATER, while native ResolveContactSpyList827b3a90 traverses Body pointer-map order; existing host contact storage does not preserve native pointer order. The live adapter eagerly observes several classification prerequisites before native branch gates. Its has_chasee uses retained target validity; native additionally resolves target to existing Entity. Renderable/interacting-object fields now have native constructor state but their presentation/interaction setters still need all caller connections. Caller wiring in selected frame is coordinated with root. Syntax parsed only; no compile/check/test/runtime.

### Collision body target transforms

New collision::transforms implements FillWorld82e90e30 and FillSkater82e91530 on retained entity/Part/animation/PropMan owners; entity.add_collision_models_to_world delegates the existing ordered add_models_to_world admission owner, and update_collision_models delegates existing drive loop. No second bodies or pose pipeline.

WORLD normal path: target transform, +X half-pi rotation using shared original SinCos; renderable/colliding uses SimLODBoneGlobal8 and -Y half-pi rotation, renderable/noncolliding replaces translation only. Nonrenderable&&!model.world_collision.from_pipe adds globalup*(model.dimensions.Y*.5)+globalup*.2. Shared actual OrthoNormalize3x3 and affine composition used. Plugin bit40 retains actual WORLDPart0; no interacting pointer and timer42<=0 or !(distance>0) clears40 (raw bgt preserves NaN fallthrough). Bit80 with real current_waypoint resolves PropMan; raw equality-only distance0 gate leaves transform, otherwise actual Part orientation plus waypoint.location+waypoint.orientation*liveShift+globalup*height/2, clears80/sets40. Jump suffix adds live Yshift only actual IsJumping.

SKATER renderable: SpatialAdjust+220 bit08 is DidSlideThisFrame; retains actual SKATERPart0 if set. Otherwise LookAtUpEye uses bone7position+bone7Z, headposition minus bone7position, eye bone7position. Arm matrices use bone4/bone1 orientations and midpoint positions4/5 and1/2. Nonrenderable&&!skater.from_pipe uses same modelheight/2+.2 shift for all3. Shared OrthoNormalize applied to all3; actual entity velocity*1/60 added to all translations, then primary capsule localXhalfpi composition unless Part0 retained. Arms stay unrotated. SourceGetRadius, FromPipe and actual Part reads reused.

Cvars appended1121..1125, count1126: JumpShiftCollisiontrue82faa4a8, JumpShiftAmount.25 range.01..582faa4f8, PluginCollisionBodyShiftAmount.5 range0..2.582faabc0, PluginLockTolerance.1 range0..2.582faac20, PluginUnlockTolerance.15 range0..282faac80. Native renderable=false constructor means first spawn legitimately reads target+model, with no fabricated evaluated/bind pose. Shared world affine helpers retain their documented ordinary-finite XYZ/W-lane host boundary. Parser checks only, no compilation or runtime.

### AI callback ownership and navigation facts follow-up

The concrete generated backend retains actual occurrence bindings and restores
partial graph/instance state after checked callback failure. EndAll82934830 now
uses that same backend: descending active occurrence End, then erase; state
clocks/current/last survive. Concrete session callback binding remains ongoing;
these owners alone do not establish live AI execution.

Actor construction now publishes the existing ChaseeState after the original
Reset formation reads succeed; constructor-prefix scalar slots are consumed,
not maintained as a second live chase owner. The input flag consumer reads the
actual chaser list. Pedestrians retain mpHandProp+1a34 as an optional DMO id,
separate from selected spawn attributes, and the null LWBehaviourSink plugin
pointer as the existing typed TransferableBehaviour declaration. No processed
plugin controller or successful attachment lifecycle is fabricated.

NavigationEntityFacts uses actual core transform, NavRig cell/history and shared
EulerXYZFromNormalizedVector825f3e20 math called by Entity.GetHeading825f4880.
Seek82e75b68/ResolvePath82e76120 gate AStar on entity6724 bit80, already retained
as navigation_rig_enabled. Update82e7de5c enables HiFiValidation for LOSim !=2,
including LoFi. Max depth82e6ed30 defaults to live NoFi depth; with bit80 it uses
HiFi for LOSim0 or LoFi for1. Guard virtual82e70488 uses its own live depth.
Registered original depth initializers82fa9028/9080/90d8/9190:15/30/40/24,
all range3..500. Shared frame owns borrowed actual world/RAIN/mesh/ChaseMan,
Manager RNG, PropMan/DMOMan, other sole-roster payloads and Actor references.
Actor target position uses completed PhysOut skeleton-root translation; target
Chasee reads borrow the existing state under its original actor lock.

Corrected light-collision RemapAnim RNG to Manager.random:82e8d9b8 calls
Man.GetRandomObject then LW.GetRandomVal. It does not use rw::math global RNG.
Hand-prop throw timeout82faaed0 is registered .2, range0..1.5. Rust syntax was
parsed through rustfmt stdout only; no compile, test or runtime execution.

### Concrete AI frame and collision callbacks (follow-up)

The shared FrameServices now borrows actual entity peers mutably without removing or cloning roster entries. Conditions read selected branch prerequisites only. Guard lifecycle uses the existing GuardBehaviour implementation with the sole entity and ChaseMan; navigation, channels, patrol and target queries use actual retained owners. DrawTazer Begin826ff968/Update826ffa38 now retain their original occurrence boolean and select the authored tazr collection only after timer37 expires.

PedestrianColliding Begin82716648/Update8271ae50/End827074c0 now dispatch through this same frame. Animation data, transition state and body reference clock come from the live Animatable; handprop actions use the actual entity methods. BroadcastCollisionEvent82711df8 enqueues cMsgMoodEvent (kind -164509450, native size80) on the existing GameSimulationBroadcaster queue4. Position and factor words preserve native float bits. This is enqueue integration, not a claim that all subscriber delivery is implemented. IsCareerModeGame826f55c8 reads the existing CommonBackEnd gameMode and compares to1 (raw addi -1/cntlzw). Selected missing branches remain explicit: BackEndManager module14 collision statistics, Challenge broadcaster guard knockdown kind -2000196102, and Net GrantAchievement. No absent backend or achievement success is fabricated.

Live cvars appended through index1152 include AStar depths, handprop throw timeout, takedown disabling, animation pools/foot tags, entity knowledge suppression, warning timer, target rotation angle, plugin DMO shifting and handprop position prediction. The last initializer82faae30 is DMO_HandProps/kDoHandPropPositionPrediction, true, with the native title. Verification remains source inspection and rustfmt syntax parsing only; no compilation, tests or runtime execution.

### Navigation modifier frame integration

EvaluateNavModifiers82e7fcc8..8007c now has a concrete frame prefix: reset all six retained records; refresh General prediction from original cvar; follow actual Chasee owner into existing ChaseGlue, otherwise evaluate the retained pathing master and clear native slave chain82e6ecb8 in-place. General neighborhood traversal reads the SAME retained LocatorKdTree with native max10 and respects returned order, self/ignore target gates and first-active exit. EntityMan gained a scoped split returning disjoint mutable payloads plus its immutable spatial field; no second spatial tree or roster was introduced.

General Recommend82e93d40/Resolve82e7e260/ResolveRulesOfTheRoad82e76a30 and BurdenedAvoid82e8f340/Oncoming82e77190/Stop82e770f0 now use shared numeric and quaternion owners and the existing intent packets. LineSegmentTestXZ82e54ae8 was absent from host searches: translated raw numerator loads (decompiler aliases both temporaries), actual epsilon8238b0c0=0x34000000 and strict endpoint exclusions. Existing Skater and General share the native avoid packet builders and direction selection. General cvars from82fa7da8..8168 and prediction/range82fa8538/8598 retain authored defaults; table count1167. Native ranges respectively60,60,90,90,90,15,15,20,20,20; front half-angle .01..1.5707999 and prediction/range0..15.

The concrete skater slice82e80080..80364 refreshes tuning then resolves local Actor, visits remaining actual Actor roster in order when enabled, and publishes original event3 through the SAME GameSim queue. Position comes from completed skeleton root; velocity uses Actor.GetVelocity825ed338's actual damped system velocity. Filtered physics and HoM come from existing completed PhysOut owners. This is still NOT a complete EvaluateNavModifiers caller: Vehicle, DynamicObject and BigBlack recommendation leaves remain to be translated before publishing a full live pass. No missing modifier is treated as an inactive result.

### Completed modifier aggregator and DMO geometry producer

FrameServices.evaluate_all_navigation_modifiers now runs prefix/General, Skater, Vehicle, BigBlack, then DMO in EvaluateNavModifiers82e7fcb8 order. Vehicle82e8c600 consumes the existing VehicleNavRigValidator four sphere positions/radii and line starts. Its distinct geotest ray/sphere helper82e4e460 shares existing numeric sqrt/refinement; the rwcollision bounded-segment helper has incompatible inside/end gates and was not substituted. Native mDistance splat30 bound expression and late equal-distance corner selection are preserved. Road state uses the same source-backed IsOnRoad condition; flee updates the sole wants table.

DMO82e975a8/82e7e470/82e8fc28 consumes actual model bbox center, actual velocity and retained farthest_corners. Source UpdateFarthestCorners82d2c7d0 reduces the two larger half-extents by the smallest half-extent, preserving branch/tie order, then transforms the four plane corners through existing LWModel::bbox_transform. The sole DynamicObject.UpdateTransform host now writes these after common target-to-current publication and before locator movement, matching82d3d1f4..200. Initial corners follow the native zeroed MemStore allocation; constructor82d3dc58 does not overwrite them. Flee branches in Vehicle/DMO preserve the entire original target ID: raw ld r8 / li r7,1 / rlwimi flags / std r8 proves the decompiler's type=Pedestrian assignment is false.

Manager.mpBigBlack is now retained as a nullable entity reference, explicitly null in ctor82721fec. GetBigBlackEntity826f5598 and IsBigBlackServiceActive8271d3a8 are pointer reads, so ordinary inactive evaluation is concrete. An actually nonnull BigBlack still returns an explicit missing typed protection-owner error; no fabricated radius or disabled service replaces it. Live cvar definitions reached1177 before ownership passed to the plugin agent. No compile, tests or runtime execution.
