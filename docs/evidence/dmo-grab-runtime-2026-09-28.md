# DMO grab continuation, September 28

Goal: finish DMOs end to end (load/see/collide/grab/move). SOLO.
This is an implementation checkpoint, **not a completed gameplay claim**.
No Cargo
build/check/test/clippy or runtime launch has been run for these changes.

Current cross-document status: the later assembly-collision and moving-object
animation continuations implemented the DMO-DMO pipeline and MovingObject
motion flow named as remaining in older sections below. See
dmo-assembly-collision-2026-09-28.md and
dmo-moving-object-animation-2026-09-28.md before adding code. Presentation
opacity/ghost/bounds are connected; world-shadow production and native replay
scheduling remain absent (dmo-render-2026-09-28.md). The January17 bank's93
models all use the supported single-part serialized-assembly branch. These
are source/asset audit results; load/grab/move/release still need a build
and controller test. Older remaining-work lists are chronological checkpoints.

## Resume point and source

The earlier DMO work stopped after
GetBestSpline, probing the embedded tGrab pointers before writing grab geometry.
Its saved decompiles/types (~90 functions; `grab42_notes.md` records the flow) are kept
outside the repository.
The connected IDA input is `Skate 2 Debug Executable\sk82_na_m.xex`.
Check existing implementations before additions; preserve this very dirty tree.

## Implemented now, source checked

- Formats `pegasus_grab.rs`: tGrabData type **0x00eb001f**, header 0x20,
  records 0x50. Fixup829c5798 adds the **containing object base** to every
  tGrab point/vector pointer, not the grab-record address. Counts at44/45/46,
  pointers30/34/38, flags3c/type40. Original bank probe: 53 grab objects,
  522 grabs; all nonempty arrays fall inside their declared backing arrays.
  That was an independent Python corpus probe, not execution of the Rust parser.
  0x00eb001e is not tGrabData (the old log's tentative classification was wrong).
- DMO models now retain decoded GrabData + validated subreference offsets.
  Reuses the existing model/bank lifetime and section-2 resolver.
- InitSplines82d2bf18 -> GetNextGrabSplineId82e4d368 -> AddGrabSpline82e638b0:
  counter .data83025058 starts at **1**, increments even when capacity rejects.
  Ordered object map cap64, owner map cap512, separately gated. DMO object
  lifecycle stores these maps. Pending culls retain grab references; destructor
  82d36088/DeInit82e62c00 removes owner IDs before collision-body removal.
  Like existing next_entity_id, the counter is in the sole active DMO manager;
  future additional LW grabbable kinds must share it, not initialize another.
- Existing GrabSpline now retains source data and actual DMO assembly identity,
  Part ordinal and occurrence-qualified body observation. `init_dmo` implements
  82d1a988. New read-only source Part snapshot uses the same attached owner,
  source-qualified body pose and inertia. `try_source_body_motion` reuses the
  existing packed-body provider, preserving the velocity W link word.
  **GetPartVelocity82d2aa80 reads physics Part0 body+20; it is not core-data
  transform-delta velocity.** Both query and refresh use the physics value.
- Grab geometry: ComputeSplineLength827ab710, GetPosAlongCurveBySplineLength
  82d89ef0, GetNearestPointOnSpline82d83780, GetBestSpline82d97d68, box initializer
  82d8cc28, DoesPointIntersectBox82e49950, CanGrabSpline82e4b4f8 and AtPoint82e4a9a8.
  Reuses CalcClosestPointOnLine and existing numerical routines. Retains curved
  local traversal, swapped-point status20, transform FMA order, strict limits.
  Raw VMX decoding checked where IDA reports wrong high register bits. Host
  numeric estimate/reduction limitations remain those of reckoning_numeric.
- LivingWorld box provider82d30a40 + refresh82d2db78. Entity-map order then
  ascending spline IDs, target transform, source broad tests, max32 append/remove.
  IDynamicObject vtable+50 is **IsStatic**, not in-use/inactive. Sphere test's
  boolean path is from 82ad9500 ->82acdef0 (cross-product discriminant, fused
  fmsubs), not the formats crate's different generic sphere algorithm.
- Cvar855 `kFeedDMOGrabSplines`, init82f8c160/ref830a7634, true, AI_DynamicObject.
  Existing `kMaxDynamicObjectSize` reference69 reused. Definitions count856.
- Deferred box query admission827b13a0 and update827b23e0; native fixed16,
  clear future.ready, process in order, clear counts/cursor, feed, sort, cap,
  publish ready, then clear queue. Sort827b1e20 ->_Sort827b14d0 selects insertion
  sort827afe70 for <=32; strict-less rotate preserves ties. Player Start box
  82dc7280 requests5 and saves reach position after admission; IsQueryReady
  82dc7380 reads validated flag40, not future.ready.
- World query processing now runs before Update0 in existing pre-solve flow,
  matching World::Update1's query-manager-before-gather order. Box producers
  now feed locomotion state504 CheckForGrabbable (see continuation below).

Focused tests added for parser ranges/root-relative sharing; ID caps/wrap;
rotated curved spline + reversal; reach box; stable sort/exclusion; and actual
deferred admission -> sorted/capped ready publication. **Tests not run.**
Scoped rustfmt has parsed the changed files; rerun after further edits.

## Required continuation (not done)

1. Reach validation, data requests and locomotion504 CheckForGrabbable are now
   connected (details below). State501 runtime is still missing. OffBoard500's
   separate CheckForGrabbable82d9c338 also needs its interactable-query fallback;
   physical_player_offboard_feet_ik_session still stops at that suffix.
2. Alternate data request entry remains. Deferred messages and grabbed-body
   collision filtering are now connected (see latest continuation below).
3. State501 Pushing already has constructor/reset/PIDs/attributes in
   physical_player_remaining_states_pushing.rs. Reuse; add source Enter/Update/
   Post/Fill/Exit and hand IK/skeleton/controllers. No replacement force law.
4. Force/torque and notify messages now reach existing DMO bodies/in-use flags.
   Reference correction: queued Activate/Freeze are native no-ops in this build;
   force/torque application wakes frozen bodies. Pushing501 must produce these
   messages at the original sites, not invent a physical freeze operation.
5. Audit the current DMO rendering/collision/live paths. In particular,
   run_dmo_dmo is sampled but has no pipeline consumer yet; pool assembly-pair
   handling is not complete. Renderer distance/spawn fading and world shadow
   sampling were disclosed as absent. Don't equate query plumbing with full goal.
6. Compile and gameplay evidence still required before completion.

Do not mark the persistent goal complete on this checkpoint.

## Further continuation: reach validation and data publication

- DynamicPool AddVolume827c86d0 -> ClusteredMeshData::Reset827c8378 explicitly
  carries the owner assembly into the mesh. Removed the port's `owner: None`
  loss in DMO line queries. World line batches/aggregation retain the actual
  SimulationDmoAssemblyHandle; identity equality uses Arc::ptr_eq, not value
  equality on (). Ground remains None. Wheel/camera numeric payloads unchanged.
- PlayerGrabSpline Sync82dd0110 now connects to Update3 prefix with the same
  World/ground, HostCollisionLineNumeric and existing GrindNum. It clears40,
  Syncs/processes/releases an older batch, refreshes surviving descriptors from
  a newer ready future, copies pending->validated, moves80->40, then consumes
  the new future and launches its batch. Zero-result queries also get a batch.
- Spawn82dcbcd0 uses AcquireLineBatch(name)827b1a58 -> BF_Default=7; original
  closest-point-on-chord helper, BOTH endpoints' Y overridden. **Raw82dcbe0c
  multiplies radius by2** (missing from Hex-Rays output); spacing is diameter,
  startY=(reachY-(count/2 signed)*spacing)+verticalOffset; even count subtracts
  spacing*.5. AddTest uses radius and its capacity refusal is ignored natively.
- Process82dcbf80 rejects ownerNone or another assembly (if grab assembly exists).
  Raw82dcc080 and82dcc0fc confirm BOTH index increments; preserve the unusual
  index progression even after early obstruction, and swap-remove last candidate.
  Copy82dcf1f0/f2a0 mutates source cursor, even consuming one failed append plus
  fetching the next at capacity. FixUp82dcc110 refreshes matching type+ID only.
- Invalidate82dcc290 now receives the existing World line owner everywhere and
  Sync/Releases outstanding batches after manager removal, before ready clears.
- Cvars856..858 appended: kNumGrabSplineLineTests (3, bounds1..20), radius .2,
  verticalOffset0. Initializers82f982d8/330/390; all Physics_States_OffBoard_Pushing.
- Primary RequestGrabSplineData82dcbac8 -> Init(type,id)82d15f08 ->manager827b1d50.
  Init is NOT Reset: sort/flags/assembly/Part/body/inertia untouched; geometry
  zeroed, source pointer null, status upper bits80. Both retained data futures
  now Arc<Mutex<...>> so manager queue retains actual output identity, no copies.
- Manager UpdateDeferredData827ae8d8 after queries; fixed16, duplicates retained,
  ready cleared on request; lookup return ignored and readiness published even
  on missing object. Vtable820fc6a8+40 is GetGrabSpline827abff0 (raw confirms r5
  output despite incorrect decompiler prototype). +44 is **empty825187c0**;
  do not invent an UpdateGrabSpline behaviour. DMO refresh reuses provider above.
- UpdateProcessedPhysIn82dcbb18 now copies ready/non-null primary+alternate into
  actual Processed members, sets flags8d0 bits24/23, consumes readiness. Alternate
  Processed member added at+7c0: ctor82e00f6c, Reset82b393fc confirmed raw. Primary
  already existed and is reused. Interactable payload remains pending.
- Focused tests added for actual empty World query spanning two syncs/cancellation,
  descriptor refresh, copy cursor, obstruction removal/index ordering, descriptor
  init retention, duplicate data futures/failed lookup ready. Two stale World
  tests expected DMO membership refusal before BeginUpdate; corrected to current
  native mesh-publication behavior (no meshes until BeginUpdate). Ground corpus
  comparisons adapted for typed owner results. Tests NOT executed.

Next: offboard CheckForGrabbable + state501 runtime, deferred object messages,
nonnull grabbed-body collision input, DMO-DMO pipeline/render audit. Full goal
still not playable/verified. Compilation/playtests still required.

## Locomotion grab query producer now connected

- physical_player_offboard_locomotion_state.rs CheckForGrabbable82d9c698 now
  replaces PendingGrabbableObject. Update82da0678 confirms InvalidateQuery is
  the ELSE arm only, and landing-on-board handling follows either arm.
- Read actual Processed effective animation+C0 and COM+250 (PDB UDT verified).
  Reuses existing XYZ-only grind geometry `lanes` projection; unknown W is not
  written back into Processed. This follows existing host numeric limitations,
  not a bit-exact unknown-lane claim.
- Full keys recovered via raw insrdi (Hex-Rays omitted high halves):
  halfDimensions C8B6F949FDFFEAAE, offset FE50188A8336FDF9,
  horizontalDeviation E5D0E7B2FBB4F684, normalDeviation B631F1A5EBC5470A,
  endExclusion 25236F554677F74F. Read in native order, angles multiplied by
  existing DEG_TO_RAD. Query type GSQ_DMO=4, no excluded descriptor when selecting.
- If validated ready, store winner flag/descriptor in existing state504; always
  start the next box query; request primary data only when a winner was selected.
  Original per-update grab_object=false remains. No duplicate state or query owner.
- Cvar859 kDrawOffBoardGrab, initializer82f92a48, defaultfalse. Enabled debug-overlay
  branch remains explicit unsupported (not part of normal gameplay). Static
  inventory verifies 860 definitions and complete registration permutation.
- Standalone tests additionally cover target vs another actual assembly identity,
  primary/alternate processed copies and unready payload retention. NOT executed.
- Scoped rustfmt parsed all changed files. No Cargo or runtime execution.

Next state501 work: existing physical_player_remaining_states_pushing.rs already
contains Reset82d91d70 inline in construct, starting at the three spline-hand
identity matrices. Extract/reuse that body for Enter rather than duplicate it.
Enter82da07b8: Reset -> FeetIK.ConsiderReset -> RequestDropBoard -> offboard collision;
if Processed8d0 bit24 and nonnull tGrab/body, copy grab, UpdateSpline(first=true),
set grab flag20, ComputeTargetHandIK, initialize normal/WorldGrabZ/Y, Juice enter.
UpdateSpline82d9cb70 first calls deferred Activate, then reads real inverse mass/
inertia and original PointGraph attributes. Saved Push_* files contain the rest.
OffBoard500 check differs: when no grab winner it starts InteractableObjectQuery.
Do not silently treat that as locomotion's no-op fallback.

## Latest continuation: deferred forces and held-object collision input

- Extracted the existing Reset82d91d70 body into OffBoardPushing::reset. The
  constructor calls it; PID controls and collaborator links stay retained.
  Enter/Update are still not implemented, so this is not a live Pushing claim.
- Replaced UnlaunchedMessage with typed GrabMessage/GrabMessageKind in the
  existing World manager, reusing remaining_states::GrabDescriptor. Fixed16,
  ordered duplicates, clear only after all dispatches. Player wrapper forwards
  to that manager and checks World provenance. No second manager/body owner.
- Source827ae780: ACTIVATE and FREEZE only enter/leave the recursive lock,
  with no physics side effects. NOTIFY_GRABBING827ac568 affects vehicles only;
  DMO is no-op. NOTIFY_LOCAL_PLAYER827ac428 calls existing SetInUseByPlayer(true).
  Vehicle notifications remain explicit unsupported when reached.
- APPLY_FORCE dispatch calls local force, then world torque (clamp=true), with
  a separate current entity lookup for each. Source827ac2b0/827ac370 dispatch
  only type2. Raw827b09b0/0a08 copies BOTH descriptor words; 0a24 stores message
  type3 at+40. Hex-Rays incorrectly aliases that store with descriptor.type.
- DynamicObject82d2a4f8/82d2a588 require a nonnull collision Body/island and
  Part0. Existing DMO pool membership and source-qualified Part snapshot supply
  the same actual body occurrence. Missing owner/detached body is a no-op.
- MaxAngularAccelerationWhileMoving82d29348 uses full attribute hash
  **116FCEB308561152**, recovered from raw insrdi82d29368. Existing node_bytes
  reads the same characteristics instance and returns native zero default.
- Body::AddLocalForce82de8518 uses WORLD force and BODY-space position/lever
  arm. Orientation transforms the arm, without translation. Raw85f0..862c
  verified; decompiler's force.y/z +0 are wrong (raw86a8/86ac load scratch Y/Z).
  Source force*inverseMass, arm cross force, cached WORLD inverse tensor,
  XYZ accumulation and cooldown reset now act on the existing rigid-body slot.
- Body::AddTorque82de8358 WORLD arm reuses the same cached tensor product
  already used for deck forces. Both routines reuse actual Frozen->Active
  primitive if state&7==2, even with zero force/torque. Positive authored limit
  clamps the ENTIRE accumulated angular acceleration after stores, using the
  existing ClampVectorWithinMaxLength82e49430 numeric graph. Packed W untouched.
  Existing host float/estimate limitations remain; not a bit-exact claim.
- World::Update1 raw/decompile827ae138 calls Update_PreSimulation827af780
  after Update3 and before Island::Step_Update1. selected_player_tick now drains
  messages at that boundary. begin_dynamic_update_frame only sets timestep/
  epoch, so it does not discard these newly accumulated forces.
- Skeleton.UpdatePostPhysics raw82b57744..7760 reads primaryGrab+6c0 status40
  and body+c8. Replaced GrabRigidBodyPending with actual body observation in
  existing SkeletonCollisionUpdateInput.ignore_rigid_body. Existing consumer
  skips the opposite spy body at native82b4bdd4; no new collision algorithm.
- Added tests for rotated local arm/off-diagonal tensor, real frozen activation,
  full-accumulator clamp, world torque independent of orientation, foreign pool
  refusal, message order/duplicates/no-op arms/capacity, and processed grabbed
  body publication gated by status40. **Not run**. Scoped rustfmt and static
  call/signature/source checks only; compilation/testing still outstanding.

Next: full Pushing501 Enter/Update/MoveObject/Post/Fill/Exit, original IK and
Skeleton.UpdateOffBoardPushing, CurrentPhysicalState routing; OffBoard500
interactable fallback; DMO-DMO pipeline and renderer audit. Goal remains active.

## Pushing geometry, skeleton movement, hand setters and output continuation

These services are implemented on the existing owners, **not yet connected as a
complete live state501**. No Cargo build/check/test or runtime launched. Scoped
rustfmt and static call/field checks only; tests added below remain unexecuted.

- physical_player_pushing_geometry.rs extends existing OffBoardPushing. Source
  GenerateGrabbableBox82d92218, IsGrabSplineGrabbable82d877a0/Still82d87908,
  UpdateSpline82d9cb70, ComputePushNormal82d87678, ComputeTargetHandIK82d92f30,
  StartTargetHandIKBlend82d93428 and UpdateTargetHandIKBlend82d87a40.
  Uses existing GrabSpline geometry, raw animation hand/hips callback, PointGraph,
  real mass/tensor, manager deferred Activate and affine interpolation82e4acf8.
  Preserves grip reversal, endpoint clamps, hand latch80 and native signed blend
  completion comparison. No alternative spline, IK solver or object body.
- Full hashes recovered from raw insrdi: pushing box dimensionsA03EA2AC5576E738,
  shared offsetFE50188A8336FDF9, horizontal6B82E41E1DA0DB91 and
  normal5ACA7C5D3094E026 (these differ from locomotion's initial query),
  exclusion25236F554677F74F, forceMassScale57D37D696363167E,
  torqueScale1CECE1CC07C5CD94, maxRotSpeedEABFCC79873A2859,
  grip distance96ECC98838ECCC11, hand blend speed1B8F17A8DA7B75AA.
- Added missing SplineUtils.CalculateWorldHorizontalNormal82e48ac0 to existing
  physical_player_grab_geometry. Thresholds3f7fbe77/3f7ff972; near-vertical blend
  deliberately NOT renormalized. Reused existing inverse point transform for
  hand offsets. Added facing/vertical fallback/non-unit transition tests.
- physical_player_offboard_pushing_skeleton.rs is a child of the existing feet
  session module. Skeleton.UpdateOffBoardPushing82b5def0..e464 now reuses the
  existing root routine82b448e8 and GeneralUpdate. Shared root helper accepts
  optional caller trajectory; both prior callers still select retained animation.
  Raw82b5df64..dffc: inverse-rotation X multiply then Y/Z FMAs transform WORLD
  displacement to local; inverse translation and motion.W are not read. The
  local displacement becomes identity trajectory translation. Actual call ABI
  at82b5e000: turn=f1, trajectory=r5, height=f2=0, correction=r7=zero,
  minimum velocity=r8=zero, reorient=r9=1. Hex-Rays signature/register names
  are misleading; do not turn this into reorient=false or animation trajectory.
  Root*volume0 publication, flag8c4 bit19, optional held-board animation gate
  8d0&40000, GeneralUpdate, NextTrajectory identity. NO UpdateGrindStates call.
  Tests cover rotated displacement independent of city translation and unknown W.
- SetLeftHandIKTargetPosition82b41100 / Right82b411a8 now reuse existing
  ClampVectorAWithinRangeOfVectorB82e496d0 and retained SkeletonIK state.
  PDB/raw: offsets37b0/38b0 are **mAnimPoseBuffer+3640 bone5/9 translations**,
  NOT mAnimVolumesPoseBuffer+3040 or the saved AnimRecord. Transform by current
  AnimToWorld; clamp requested WORLD target around shoulder to maxArmLength;
  store external target2/3, set byte, blend. New animation_row getter only exposes
  existing raw mapped bone storage. Added SkeletonIk aggregate forwarding too
  (ik_runtime_mut returns aggregate, not SkeletonIkState).
- SkeletonController.DisableHandCollision82dd4768 ->82b4ef00 uses Parts
  [7,3,8,4,1]; DisableLowerLegCollisions82dd4770 ->82b4f3b8 uses
  [19,15,20,16,17,21]. Each writes active=true, counter=frames, folded empty
  OnEnd, clears enabled bit only. Reuses existing foot-disable primitive and
  existing countdown consumer; no group changes. Actual pair adapter supplies5
  frames, hands before legs. Countdown/selectivity test added.
- Same-pair hand adapter uses existing pool identity/attachment checks and
  verifies actual PhysicalPlayerAssemblyOwner; calls left then right, retaining
  left writes on right failure. IK state test checks external-mode selection and
  preservation of feet/local targets. No gameplay execution claim.
- FillPhysOut82d87eb8 in physical_player_phys_out_pushing_state.rs plus same-owner
  adapter: grabbing; speed dot blendedHands.Z; speed dot -blendedHands.X;
  smoothedForce dot Z; force dot -X; angular speed literal0; crouch;
  descriptor from actual retained GrabSpline (NOT mGrabSplineDesc); mass;
  ground repush delay; FeetIK.Fill82dc8ba8 reads actual left/right footDown booleans.
  Guard/component checks retain partial prior stores. Raw VMX operands verified.

### First-entry read-before-write investigation (important next continuation)

Ctor82d9c980 and Reset82d91d70 leave targetPushNormal+170, lastPushNormal and
targetXDirection unwritten, matching existing Option fields and constructor tests.
Enter82da07b8 calls UpdateSpline(first=true) before initializing targetPushNormal
from negative current hands.Z. UpdateSpline's suffix uses the old normal to offset
approximateSkeletonPosition. ComputeTargetHandIK does NOT initialize that normal.

Allocator is independently rechecked: HiLOD82e04af4 flags1, call82e04b00 ->
XCore_EASTL_Category_Alloc828b4bf0 -> GetCoreAllocator8296f4e0 -> category
Alloc82973570 -> Internal.Alloc82972a18 -> AllocHeapMem82972240. PhysicsSkater
row82ff0d48 in memCategorySetups82ff0988 (stride24) is name821677d4,
limit19999, heaps8/0, maskFFFFFFFF, OR0. MemClear requires flag100. Therefore
do NOT fix this by assuming zero allocation or changing Reset's original writes.

New evidence narrows its observability: CalculateSkeletonPosition82d93690 uses
old approximate position ONLY for the approximate speed derivative, then stores
the newly calculated position at82d9394c. Its approximate-speed accesses are
load82d938e8, store82d93948, and later zero store82d93d84. That later store is
selected by stateTimer < kMOVOBJ_NoSpeedTimeout; initializer82f937f0 gives0.1
(bounds0..1). There is no intervening approximate-speed read in this function.
Next implementation should trace timer initialization/call order and preserve
unknown intermediate data until this native overwrite if selected, rather than
invent a normal. Current UpdateSpline still explicitly refuses unknown normal;
full CalculateSkeletonPosition consumer is not yet implemented. Nondefault
timeout0 or first-entry timer>=timeout must not silently invent a derivative.

Juice MoveDMO_Enter82a76160 and Exit82a780a8 inspected: both gated by
sJuiceReady. Enter captures telemetry location, Exit logs DMO_Movement; neither
changes physics/grabbing. Record this boundary when implementing lifecycle;
do not treat it as a missing movement-force function.

Remaining full-goal work: Pushing Enter/Update/CalculateSkeletonPosition/
MoveObject/CalculateCollisionResponse/emergency/Post/Exit + routing; OffBoard500
interactable fallback; DMO-DMO pipeline and render omissions; compile/playtest
proof. Source files saved under the prior grab42 scratch directory remain useful.
Goal is active and end-to-end completion is not claimed.

## Continuation: full Pushing501 services and live routing

This section supersedes the earlier remaining-work and unknown-normal notes.
All changes remain UNCOMPILED / UNEXECUTED; no cargo or
runtime launch was performed. Scoped rustfmt and static source checks only.

- physical_player_pushing_position.rs now implements CalculateSkeletonPosition
  82d93690, CalculateCollisionResponse82d87ca8 and ConsiderEmergencyExit82d87d80.
  Raw VMX decoding recovered the hand-midpoint Y scratch store omitted by
  Hex-Rays, separate minimum-delta/minimum-blend references, native graph order,
  fixed dt, crouch fsel clamps, signed alignment and world displacement clamp.
  Collision response is oldOffset*CollisionBlendOut(.95) + hook*BlendIn(.5),
  NOT an assumed complementary blend. Emergency exit latches flag/delay.
- Unknown first-entry target normal remains unknown during UpdateSpline(first).
  CalculateSkeletonPosition carries an unknown derivative until the native
  stateTimer<NoSpeedTimeout(.1) zero overwrite. SetPhysicsState82dff1cc..204
  resets both actual and Processed stateTimer before Exit/Enter. Nondefault
  timeout0 does not fabricate a derivative; a later required read still fails.
- physical_player_pushing_move.rs implements MoveObject82d923d0..2f18. Uses
  authored forward/back/side/rotation graphs, body mass and inertia, collision
  velocity response, jerk clamp, torque clamp and actual deferred force messages.
  Existing VectorPid now has the native82d89b98 accumulating update, distinct
  from Reckoning's PIDFilter. Force limiting is written back to PID.output.
  Scalar angular PID has its own D+I then P order. Lever origin is BLENDED hand
  transform translation(+100), not current hand translation(+c0). Hand lever Y
  uses Processed ground position+1f0. Products/FMA/negated FMA were raw-checked.
- physical_player_offboard_pushing_state.rs implements Enter82da07b8,
  Update82d9cff8, Exit82d87760 and UpdatePostPhysics82d87e38. Reuses real FeetIK,
  controller drop-board, collision SCS_OFFBOARD, raw animation hands, spline
  query/data manager, limb collision disables, shoulder-clamped hand targets,
  Skeleton.UpdateOffBoardPushing and wipeout reason25. Candidate replacement
  correctly checks OLD spline data/body at82d9d218/224. Loss of holding,
  excessive partial-ragdoll time or emergency clears grabbing at82d9d2a0.
  Query runs after either grabbing or zero-motion skeleton update.
- FeetIK UpdateGround was extracted from the old OffBoard500 wrapper: the
  remainder was a full stationary-state Update suffix, unsuitable for calling
  from Pushing. Both callers share the real feet prefix without duplicating
  OffBoard500 steering, locomotion viability or Reckoning behavior.
- Skeleton.DisableProxyVolume82b42068 now uses existing runtime Part0 collision
  disabling (native folded OnEnd is empty). Exit queues native Freeze (no-op).
  Juice MoveDMO_Enter/Exit are telemetry only and remain unported.
- Cvars860..884 append25 source definitions. kEnableRepushingTimeLimit reuses44.
  Source init82f92f90..82f93a30; bool/float null callbacks verified from raw ABI.
  CollisionBlendOut initializer82f939d0 was separately found via raw reference
  830d8444 after its load was missed by Hex-Rays.

### Live state routing

GetType82d9cb58 is li r3,501. CurrentPhysicalState/GroundDestination now include
OffBoardPushing. Transitions from500/504 to501 are admitted;501 exits to500/300
and the shared teleport702. CalcSuggestedState82ddcc64..cc8c raw instructions:
8c4&40000 ->300; otherwise8cc&800000 ->501; otherwise500. Controller start and
stateTimer reset retain the existing SetPhysicsState order.

Update3 validates the retained state binding, runs Pushing.Update, the actual
active board controller, then existing prephysics and deferred force dispatch.
Postphysics and FillPhysOut are routed to the new same-owner services. Fill uses
the caller's ReckoningNumeric dot primitive; entry controller/IK and postphysics
also retain caller numeric. Geometry uses existing host GrindNum services.

Skeleton.UpdateOffBoardPushing82b5def0 ends with GeneralUpdate82b5e414 and reset
of NextTrajectory, with NO Reckoning.UpdateGrindStates call. GeneralUpdate
82b5ba88 was independently checked for this. Update5 always calls retained
Reckoning.FillPhysOut82e04f48. Added RetainedPushing owner/state validation;
no fabricated completed Reckoning phase and no extra walking update.

## Continuation: stationary grab + interactable fallback

OffBoard.CheckForGrabbableObject82d9c338 and Locomotion82d9c698 share the same
box/candidate/request prefix. Refactored the existing implementation into
check_offboard_grab with explicit retained StateKind and shared field access.
All five 64-bit attribute hashes were rechecked from insrdi operands. Debug
drawing is folded out in these functions; enabled cvar checks the real overlay
singleton rather than returning a made-up unsupported-debug error.

OffBoard.Update82da063c now calls this prefix and, if no spline was selected,
StartInteractableObjectQuery82d9c620 with the ORIGINAL effective transform.
JumpingPreperation82da0658 still follows either branch. The old
PendingOffboardPostOrientation endpoint was removed.

physical_player_grab_interactable.rs implements:

- AddDeferredInteractableObjectQuery827b06e8: clear requested future.ready even
  when another query occupies the one-entry queue; only first query accepted.
  BF_Dmo=2 confirmed at827b081c. NumLineTests unsigned conversion/frsp then
  MaxHeight/count scalar fdivs. Start=P + J*step*i (FMA); end=start+Z*distance
  (FMA). VMX registers rechecked with raw decoder, not Hex-Rays high-bit guesses.
  Each AddTest uses live fatness; count tracks accepted additions to the fresh
  batch, preserving capacity refusal. Existing World line pool Begin/End used.
- UpdateDeferredInteractableObjectQueries827ae980 and
  CalculateInteractableObjectQueryResults827ac108: reset count/cursor, Sync,
  ignore t==0 and require strict t<best, resolve actual DMO assembly owner,
  require existing DynamicObject and !IsStatic. IDynamicObject vtable8232cfa8
  slot80 resolves82d36320(IsStatic), not a guessed grabability predicate.
  Publish one real entity ID and ready only on a hit; no-hit leaves unready.
  Release original batch, clear queue. Manager.Update827b24b0 runs this AFTER
  spline box queries and data refresh (24c4/24cc/24d4).
- Player's constructor-only interactable scalar fields were replaced by ONE
  shared Arc<Mutex<InteractableFuture>>, used by manager and publication.
  UpdateProcessedPhysIn82dcbc80 resets cursor; success advances to1, sets
  8d0&00400000 and writes ID to8c0; empty ready clears bit, unready preserves it.
  Both invalidation sites clear readiness without cancelling the queued future.
  Updated existing publication/invalidation tests and stale startup corpus
  reads; none were executed.
- Cvars885..889: source initializers82f69800/858/8b8/918/978, defaults5/.2/1.8/2/
  false, null callbacks raw-checked. Registry now890 definitions. Static check
  verified registration is a permutation0..889, all30 newly added labels unique
  against the registry, and Pushing enum IDs match actual definition positions.

Remaining: audit live DMO animation/intents and frame flow; DMO-DMO collisions
(run_dmo_dmo remains configured without a pipeline consumer); DMO rendering
omissions (world-shadow sampling/fades/ghost/interpolation); compile and
playtest evidence. End-to-end completion and bit-exact Xenon parity unproved.

Final static check this continuation: scoped rustfmt --check passed for27 touched
Rust files; no remaining references to removed interactable scalar fields,
InteractablePayloadPending or PendingOffboardPostOrientation in core Rust.
This is parsing/format validation, NOT Rust type checking or gameplay validation.

Next collision source anchors confirmed live in IDA: ref830525fc callers are
Step_UpdatePrePhysics2 (DMO-ground Run827b2954 then DMO-DMO
AssemblyAssemblyCollisionPipeline.Run827b2978, argument0) and Step_Collision3
(DMO-DMO Sync827b4198 after skater-ground/DMO-ground/skater-DMO/skater-vehicle
Sync). Current skateboard_ground_frame.rs only consumes run_ground and
run_skater_dmo. Reuse/audit simulation_skater_dmo_contact.rs assembly-pair math
before implementing the DMO-DMO driver; do not copy it into a competing solver.
