# Actual Motion preland query

Author draft 2026-09-19. Fresh IDA decompile, raw PPC/VMX words and PDB
member layouts. Four unique unregistered Rust files, standalone rustfmt passed;
no Cargo or test execution by author. Independent review required before mount.
Confidence **high**, not a claim of complete Air state or rendered parity.

## Identity and scope

`Sk8::Skater::SkaterMotionGraph::IsTimeToPrepareLand`825f4b60 is the concrete
ISkaterMotionGraph vtable82069688+20 entry. Its first virtual+1c call at4b90
dispatches **HasTiltToLargeForPreland825f48d0** (entry820696a4). The decompiler
incorrectly calls this GetNextAnimTransOveride because it uses the wrong this
type. Do not inspect transition override flags for this query.

BodySpin30's existing pending query can call
ActorConstruction::motion_time_to_prepare_land with the same actual bound
context. The new actor adapter is intended child motion_runtime_session::prepare_land;
it includes motion_prepare_land_query.rs and the six focused kernel tests.
No instance, flag shadow, new physics producer, lifecycle bypass, or writes.

## Required virtual helper825f48d0

1. Resolve cached Actor IPhysical+200 against actual UCOM IPhysical occurrence.
   Actor's missing cache asserts; a genuinely null context/interface returns
   false. The actual Actor adapter uses existing checked cached_physical_allocation,
   preserving the selected Actor's required-owner boundary.
2. PhysOut+8 Air+1ac `mIsTargetingWallRide` true returns true immediately.
3. Capture Air+90 `mLandingNormal`.Y, THEN read Global+bc physics_feet.default
   scalar keybaba91606102af9a (4a24). `normalY > limit` returns false. Raw
   4a481be1ff37 splats v127.Y into v63; 4a5c1ba0f735 splats limit into v61;
   4a601b9fe965 actually compares **v63 > v61** (IDA alias v95 is wrong).
   mfocrf/extrwi4a64/68 tests CR6 all-true bit. Splatted NaN gives false,
   so unordered normal/limit continues to the tilt test.
4. Capture PhysOut+38 Animation+10 `mLastAnimTilt`.X, clear its sign bit,
   THEN fetch Global+148 anim_motion_body_tilt `overide_preland_x`.
   4aa01b5bd8f5 is v58=v59<<v59, sign mask80000000; 4aac16f8d275
   is v55=v56 & ~v58. No numerical max/min or NaN sanitizer.
   Scalar raw4ad041990008 requires ordered GT; otherwise false including NaN.
5. Capture SystemReckoning+10 velocity.Y, THEN body_tilt overide_preland_velY.
   Raw4b2c41990008 requires ordered GT; NaN returns false.

## Main query825f4b60 order

True helper result returns false before second physical resolution. Otherwise
resolve IPhysical again. Capture SystemReckoning+10 velocity.Y. Raw4c68
41980008 requires ordered LT zero; either zero, positive or NaN returns false.

Fetch Global+150 anim_motion_inair_disttocog scalars in order:
preland_final_disttocom then preland_disttocom_vel. No clamping or division guard.

* Read OffBoard+42 mCanLandOnSkateboard. If true, fetch landingondeck_disttocog,
  subtract from final distance with fsubs4cf0, divide by velocity fdivs4cf8,
  THEN capture OffBoard+1c mTimeToLand. Return ordered time<threshold
  (4d1441980008). This branch has priority over Air trajectory and cvar.
* Otherwise capture Air+1a5 mIsTrajValid. If true, read Animation+48
  mCentreOfMassToDeck; threshold=(final-distance)/velocity using separate f32
  subtraction and division4d74/78. Capture Air+a8 mTrajCollisionTime. Resample
  Air+1a5 (4da8); this second read is deliberately retained by the kernel.
  If still true, capture Air+20 mTrajCollNormal.Y. fneg/fsel4dd8/4ddc lower
  clamp chooses +0 for either signed zero and negatives; NaN is retained.
  FETCH Global+d4 animation.default LandingDistanceScalarVsNormalY graph,
  THEN upper fsub/fsel4e00/04 selects1 for NaN or above1. Evaluate existing
  PointGraph8 and multiply graphResult*threshold (fmuls4e0c). Return ordered
  collisionTime<threshold (4e184198ff04). No FMA or invented length estimate.
* If first Air-valid read false, capture OffBoard+44 mTrajectoryReady; false
  returns without cvar access. Otherwise capture OffBoard+1c time FIRST,
  THEN live cvar239 target+28/value+d8 (4e60/64); return ordered time<value
  (4e704198feac). All final comparisons reject unordered.

## Typed actual outputs

Fresh PDB confirmed the exact field identities above. Adapter uses existing
try_air_output/try_offboard_output/try_animation_output on the retained actual
PhysicalPlayerAssemblyOwner PhysOut. System velocity reuses existing
try_landing_system_vector(CentreOfMassVelocity), with Y operand only.
These source const reads do not sample the write latch. Each native GetPhysOut
read maps to its own locked read, including repeated Air-valid sampling. The
second cached physical resolution checks the same Arc issuance. No fabricated
copy of Air state or host landing heuristic is introduced.

## Retained global views and reuse

Mount actor_motion_prepare_land_attributes.rs as **body_tilt::prepare_land**,
under existing input_cvar_runtime::body_tilt. This placement lets the adapter
reuse the existing BodyTiltMotionAttributes private instance/vault; do not
rebind a new +148 body-tilt collection per call. PrepareLandRuntime borrows
that retained provider, PrepareLandAttributes and actual cvars.

Retain PrepareLandAttributes once in ActionRuntimeServices, bound from its
same actual immutable Vault. It provides readonly projections of:

* Global+150 anim_motion class90c3decf0105631e collection995bb00613b8645a,
  allocation828b2fbc/ctor2fe0/publication2fec.
* Global+bc physics_feet.default, reusing existing SkateboardFootVaultAttributes
  CLASS5338958c408b6009 and default collectiond7edbd362d7d2152.
* Global+d4 animation.default, class74bc9c93affe6aac and same default collection,
  matching the existing actual Animation conditioner provider.

These are immutable retained attribute views, not additional gameplay state or
per-call bindings. Query checks physical inputs Vault Arc identity at first
global use. The original selected corpus must mount all three classes so an
omitted class is not mistaken for original missing-attribute fallback.

Each scalar reruns generic Vault lookup/inheritance with index0. Missing pointer
uses DefaultDataArea4 zero; no authored value is frozen. Exact keys/accessors:

| Attribute | Key | Accessor |
|---|---|---|
| Feet normal limit | baba91606102af9a | inline825f4a08..24 |
| overide_preland_x | 87c7cfb04f54f9da | 825edc70 |
| overide_preland_velY | e571887a9967e796 | 825edc28 |
| preland_final_disttocom | a766556a9f24a64b | 825edd00 |
| preland_disttocom_vel | 174776fa810206ff | 825edcb8 |
| landingondeck_disttocog | ae54b965f21bdac0 | 825edbe0 |
| LandingDistanceScalarVsNormalY | 29a73f047bef8865 | 825ecef0 |

The last graph is a **generic FETCH**, not cached-layout lookup. Its pointer
contains x[8] at0 and y[8] at20, size40; accessor825ecf20/24 supplies missing
DefaultDataArea40. Existing PointGraph.evaluate is reused unchanged. Invalid
non-null data size errors explicitly; a missing graph is all-zero source data.

## Cvar239 integration packet

kOffBoardAirPrelandTime reference83047fdc, initializer82f56930, constructor
call56970, initializer table82fd1038. Section82063cc0 Animation_Skater_InAir;
title82063c8c `OffBoard Air Preland Time`; label82063ca8 kOffBoardAirPrelandTime.
Default3f99999a (1.2), minimum00000000, maximum40400000 (3); update null
from li r10,0 at5696c. Definition/index239 reserved by root.

Table1030 DisplaySkaterMotionDebug,1034 DisplaySkaterActionDebug,1038 this,
103c AnimScoreDebug. Next currently mounted entry51 UseSprintButtonInput is
1058: insert239 before51 in registration, not by numeric index. GUI belongs
Animation_Skater_InAir; GUI title sorting places EnableLandingOnBoardBodySpin231
before OffBoard Air Preland Time239. This differs from initializer registration order.

## Focused validation and remaining integration

Six authored tests call the production kernel: wallride early return; tilt
bitwise abs and unordered normal/tilt gates; descending velocity gate including
both zeros/NaN; skateboard landing priority and NaN threshold; repeated
Air-valid sampling and NaN normal upper-clamp selection; OffBoard cvar late
failure after time capture. Tests have not been run by author.

Root must mount the two child modules, retain global view, register239 and wire
BodySpin's query to the actor method. Do not infer full Air-state admission from
this query. Other Air Motion callbacks and producers retain their existing
explicit boundaries. Generic PointGraph shares the existing host arithmetic
policy; this packet makes no new Xenon trace-equivalence claim.
