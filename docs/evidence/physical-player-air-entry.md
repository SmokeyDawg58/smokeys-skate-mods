# Retained Air entry and FootPlant manager

Date: 2026-09-12. Author: ground_path. Root independent
review and complete post-integration gates are pending at authorship.

## Source scope

Fresh connected-IDA decompilation, raw disassembly and PDB layouts were used.
Primary source routines:

- `PhysState_PhysicsAir::PhysState_PhysicsAir`82d8a9e0; selected HiLOD
  constructor call82e04688 precedes Ground constructor82e04b98.
- `PhysState_PhysicsAir::Enter`82d98dd0.
- `FootPlantManager::FootPlantManager`82dc1168; HiLOD call82e042f4, after
  Wipeout reset82e0427c and the omitted GrindManager constructor.
- `FootPlantManager::Reset`82db6760, `FullReset`82dbbae0.
- `SkeletonController::InitAirCollisionUpdate`82dd9fb8,
  `EnterState`82dd4788, `Skeleton::SetAirCollision`82b4eea0,
  `SkeletonBody::SetUpFullCollision`82b47780.
- `Skateboard::EnableAngularDriveOnly`82b2aca8,
  `SkateboardBody::HookEnableAngularDriveSoft`82b24ad8.

This closes the actual Ground100 predecessor's no-COM branch of Air entry
once the root integrator mounts the same owners and supplies outer transition
scheduling. Other predecessor COM/mount trajectories retain explicit refusal
after their source prefix. No native continuous Air Update is claimed here.

## FootPlant owner

PDB FootPlantManager size is0x1e0. The constructor's raw1190..11a0 assembles
class `0x172d553937d8e07c`, default collection `0xd7edbd362d7d2152`; IDA's
pseudocode misassigns halves to the skeleton argument. The host retains the
actual Vault instance rather than replacing its tuning with constants.

Prediction future begins+10: its result contains TrajectoryResult+0..4f,
tTrajectory+50..8f, radius+90, max errors+94/+98, drawTrue+9c; readiness+a0.
The constructor initializes only trajectory (position/velocity/gravity all
zero, time[-1;4]) and readinessfalse. Collision result/other fields remain
explicitly unknown. Native stores Processed/Skeleton pointers at1208/120c,
repeats readinessfalse1210, and calls Reset1214. Rust binds to the retained
same pair/skeleton, with separate allocation-not-attempted and allocation-null
states and no fabricated prediction readiness.

Reset preserves prediction contents/readiness, surfaceID+1d0, nine vectors
(c0,d0,e0,f0,100,170,180,190,1a0) and attribute/binding owners. It writes:

- footOffBoard=false677c; timeInFootPlant/IKBlendIn/timeTargeting zero680c/
  6814/681c; resultsValid/landingNextFrame=false6824/682c;
- timeToLand and totalTime=-1 at683c/6844; footPlanting=false6848;
  footPart=-1 at684c; perfect/lastTarget/targeting validityfalse6850..58;
- six complete zero vectors in **110,120,150,130,140,160 order** at6864,
  6870,687c,6880,6884,6888. Raw stack stores establish every W as zero.

FullReset82dbbae0 first clears future readiness atbae4, then tail-calls Reset.
Normal Reset does not clear it. The retained implementation keeps unknown
source cells as Options and never clears preserved fields as a convenience.

## Air entry ordering and constructor state

PDB Air state size0xc0: inherited PhysicsState0..1f, local air/reckoning
instances+20/+30, shared FootPlant+40, COM trajectory+50, landingNormal+90,
time/startY/maxY+a0/a4/a8, reachedApex/useCOM/runMount/sprintMount+ac..af,
trajectoryTimer+b0. Constructor makes COM trajectory zero/-1 and landing
normal UP with W0, scalar counters zero and reachedApex/useCOM false.
RunMount/SprintMount are **unwritten until Enter**, represented as unknown.

Enter must run all concrete operations, including the temporary collision mode:

1. InitAirCollisionUpdate8df0: need-air=true (82dd9fc0), request state**7**
   (raw82dd9fbc). SetAirCollision calls full collision, disables proxy, repeats
   root disable/count0. FullCollision first writes root group4/disabled/count0,
   then Assembly/all Part userData6, then conditionally enables Parts1..23.
   This ordering differs from NormalCollision and cannot be replaced with it.
2. EnableIK8df8 without Ground's truck-tilt side effect.
3. SetSkeletonCollisionState STANDARD6 at8e04, with actual normal collision.
4. EnableAngularDriveOnly(1)8e18, same retained board hook.
5. Same FootPlant.FullReset8e20.
6. reachedApex=false8e40, trajectoryTimer=0 at8e48, time=0 at8e5c,
   landingNormal UP8e60/64; actual deck transform GetTransform8e68.
7. maxY from actual deck8e74; startY from same Processed.GroundPosAllStates
   +1f0 at8e7c; independent last category/state reads8e80/8e8c and run/sprint
   bits at8e94/8ea0. Reset82b3924c provides genuine initial full zero ground
   position; no current Update1 publication is implied by its retained reset.
   Processed bitword+8d4 reset394e0 preserves low22 and clears high10;
   run/sprint masks are04000000/02000000, distinct from+8d0.
8. PDB PHYSICS_CATEGORY_GRIND is**400**, not wipeout300. If category400 or
   last state701, useCOM reads negated Processed8c4 bit4000. Otherwise last
   state100/103/201 chooses false and other predecessors choose true.
   Ground100 therefore completes without constructing a replacement COM or
   mount trajectory. Missing COM continuation remains explicit after stores.
9. Late debug boolean read8fd8; no debug owner is invented if enabled.

Entry requires already published current Air and the retained Air bindings.
It acquires the state mutex itself: outer transition releases its lock first.
It does not select a destination or call old Ground.Exit. Those belong to root
integration of SetPhysicsState's original caller/field/publication order.

## Angular-only board hook

HookSoft reads fresh linked Disp at4b08/2c, multiplies by caller strength at34,
computes rsqrt estimate44, THEN reads fresh linked Str at54/60. It writes
angular type1 at64, strength=`Str * f32(4560fffe)` at6c, spring at70, and
damping=`2 * refined_sqrt(spring)` atbb0. Two source VNMSUB/FMADD refinements
retain final-negated residual and rounded separate multiplications. Even zero
spring still requests the estimate before selecting zero result.

Caller82b2aca8 reacquires the same Body and writes linear type**2 (HARD)**
atacd4, then zero spring/damping/strengthacd8/acdc/ace0. This is the source
meaning of angular-only; replacing the linear type with NO_DRIVE would differ.
The Arc-backed dynamics are the actual constructor/drive pool's shared payload.
Prepared drive batches retain their prior snapshots until the source build.

Fresh cvar initializer evidence:82f7ba80 Disp group Physics_Skateboard_Drives,
name kSkateboardSoftDriveDisp, title SoftDrive Disp, default20000, range0..100000;
82f7bae0 Str same group, name kSkateboardSoftDriveStr, title SoftDrive Str,
default10, range0..100. Air debug82f8e968 group Physics_States_PhysicsAir,
name/title kbDrawPhysicsAirDebugInfo, defaultfalse. These are real registry
bindings with fresh runtime getters, not literals embedded in physics.

## Tests, confidence and remaining scope

Confidence **high** for selected source order, fields and retained ownership.
Host rsqrt/dot/VMX denormal and NaN payload behavior remains a numerical
boundary. No per-tick reference trace parity is claimed.

Focused tests cover FootPlant reset preservation, controller need-air/state7
publication before failed effect, and FullCollision late unknown retaining
root/group/earlier Part writes. Root runs all Cargo and real-corpus gates.
The previous grounded placement measured all five real hits and height0.02790059;
ragdoll corpus assertions were changed to compare before/after request readiness
and state instead of assuming the earlier Update2 never completed.

Native Air constructor is now represented; all other HiLOD constructor owners,
full Air Update, mount/COM alternative entry, and WipeoutGround constructor/
entry remain separate closures. A successful selected Air entry does not itself
establish a playable skater or original animation/input/physics frame loop.
