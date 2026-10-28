# Skeleton ProcessData continuation

Source `Sk8::Physics::Skeleton::ProcessData82b5ecc8..82b5f348`, called by
`PhysicalPlayerHiLOD::Update1_ProcessInput82e03718`. This continuation begins
at input pose reload82b5efd0, after the separately recovered trajectory inverse.
Confidence **high** for ownership, branch and store order; existing explicit
host numeric policies do not establish bitwise Xenon floating-point parity.

## Same owners and executed reads

The constructor retains the actual AnimOutPhysIn Arc. Its same global pose feeds
24 bone/volume mappings and the existing IK mappings. Existing current-animation
state, fractional masses, roots, constructor-bound attributes and animation
record remain on SkeletonNormalAssembly. No second pose, mass or record owner
is introduced. The passed processed deck delta is written at f258 before the
retained previous deck-ZY value at f25c.

The old immutable `SkeletonAdjustmentInput` remains an explicit controlled
numerical input. Live execution uses `AnimationProcessedRead` with references
to the actual processed subfields, not a prefetched complete snapshot. Known
flags are checked only for consumed masks. Dot3 operands consume XYZ; creating
scratch W0 for that operation does not publish W in retained vectors. The
LandingOnBoard deck is an INNER matrix of WorldToAnim composition; inner W is
not a coefficient, so missing physical deck W does not block this composition.

LandingOnBoard82b417c8 first checks retained Skeleton COM-Y velocity, then
processed8d0/10, before reading physical deck/landing adjustment/time-to-land.
LandingAdjust82b401d8 first performs its mandatory maximum-velocity attribute
lookup. Filtered-state loads40274/40294 are conditional on retained previous
state. Balance402f4 is unconditional, even when the air-ground test is false.
Flags8c8 are captured once40350; coffin/disable share that value. Up403c0 is
loaded before COMvelocity403c8, and that same up is reused for the start-position
dot after retained start stores. Later source reads remain independent.

GrindAir routing f180..f1b0 is unchanged. Actual
`PhysicalPlayerAssemblyOwner.grind_air()` owns the collaborator, initialized
by the native constructor projection. `UpdateGrindAirAdjust82b4cd80` calls
`GrindAirAdjust::UpdateGrindAdjust82dd1518`; local adjusted=false at1538,
activated+281 false at1544 jumps1654, skipping kGrindAssistActive. Store165c
always clears the SAME owner's adjusting+282. The caller sees false and exits.
An active collaborator remains an explicit pending callee; it is never assumed
inactive because the processed category is Ground. Controlled numerical tests
retain their own explicit fake collaborator contract only.

## Final retained output and teleport stores

- f258 writes deck-ZY delta to processed9cc, then f25c writes Skeleton3e3c.
- f268 compares splatted adjusted deck Z.Y against zero. f288 sets8c4/1000
  for ordered greater,800 to its complement. NaN and both zeros set800.
- f278 captures old8c8; f294 reads real collision+fbe; f29c inserts its LOW BIT
  into40000. Hook-valid+fc6 atf2a8 instead tests ANY nonzero byte.
- Valid hook copies retained post-physics HookToHips3d80 to processed2e0 atf2c0.
  False preserves the older processed vector.
- f2d8 copies the existing animation record COM delta2a30 to processed2f0;
  f2e0 copies its COM2a20 to processed300. This is not physical COM velocity.
- Outer teleport3e40 is read only afterward. True clears3e40 atf2f8,
  clears root prediction-valid3dec atf300, enables actual IK+1d0 atf30c,
  and edits the actual retained hook0 dynamics. Angular order: damping+0,
  type2, spring bits4415ffff, strength470c9fff. Linear order: spring, damping+0,
  type2, strength. The owner pointer is reread between angular and linear.

Processed Reset82b39098 initializes the new output copies: hook2e0 at39358,
COMdelta2f0 at39360, COM300 at39368 all use the existing zero vector;
deck delta9cc is float+0 at39524. Generic pre-constructor storage stays unknown.

## Cvar registration proof

Each call reads its current linked target at the existing callee read site.
Float GUI ranges are not runtime clamps. Both float initializer r10 callback
arguments are explicitly zero atc8d4/c9d4; misleading Hex-Rays buffer annotations
are not callbacks. Debug drawing callees are proven BLR stubs; actual overlay
singleton acquisitions remain, using the shared holder at their source point.

| API index | Native label | Ref | Initializer / table | Section / title / default |
|---|---|---|---|---|
|60|kLandingAdjustFramesToBlendAwayOffset|8308e9d4|82f7c898 /82fd4574|Physics_Skeleton / Frames to blend away offset /41700000|
|61|kDebugLandingAdjust|8308ec08|82f7c8f8 /82fd4578|Physics_Skeleton / Show Landing Adjust Debug /false|
|62|kDebugUsephyscogland|8308e714|82f7c948 /82fd457c|Physics_Skeleton / Use phys com on landing /false|
|63|kLandingAdjustFirstFrameHack|8308eca0|82f7c998 /82fd4580|Physics_Skeleton / Landing Adjust first frame hack /3c75c28f|
|64|kbLOD_NegateY|8308ea74|82f7cb28 /82fd45a4|Physics_States_LandingOnDeck / NegateY /true|
|65|kbLOD_ShowLandingAdjust|8308e450|82f7cb78 /82fd45a8|Physics_States_LandingOnDeck / Show Landing Adjust /false|
|66|kUseXYZCentreOfMass|8308eb4c|82f7d6a8 /82fd462c|Physics_Animation / Use XYZ Centre Of Mass /true|

Implementation and validation are in progress. Root owns integration gates and
the original-corpus full call. No completed ProcessData or playable-frame claim
is made by this draft packet.

### Constructor reader admission correction

The first original-corpus continuation exposed an absent host reader despite a
retained native PhysicsAnimation instance. Completed Skeleton admission now
passes the actual physical owner's Vault to `retain_constructor_runtime` and
moves `take_physics_animation_instance()` directly into the animation reader.
There is no replacement Skeleton binding and no IK initialization at this step;
the existing constructor-owned current-animation/IK object is transferred as
before. The separate landing-global reader remains the existing immutable Vault
projection; this does not establish editor replacement or native global lifetime
parity. A controlled handoff test uses non-default collection 17, verifies that
its threshold survives, verifies the constructor instance was consumed, and
checks that reader admission does not alter IK initialization. Original-corpus
validation of this correction remains with the root integration gate.

Root validation23/26: independent source review approved same-instance constructor handoff, all7cvars/reset values, final output/teleport ordering and knownW operands. Workspace23 3,799units/6docs, original23all17, Clippy26/format23/diff23 PASS. Actual sameActor original scenario completes this call before post-Skeleton input; active GrindAir adjustment remains an explicit unsupported branch, and no full physical frame or Xenon numeric trace parity is claimed.
