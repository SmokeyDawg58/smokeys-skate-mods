# Four neutral Motion lifecycle callbacks

2026-09-19, fresh connected IDA decompile, raw instructions and PDB layouts.
Confidence **high**. Drafts are unregistered; author ran standalone rustfmt only,
not Cargo. Root must independently review and mount actual lifecycle dispatch.

## Files and mounting

`actor_motion_neutral_lifecycle.rs` is a proposed `motion_runtime_session` child.
It implements actual Actor adapters for ForcePhysics.Begin, EndShimmy.Begin,
SetScoreAugmentation.Update and SetDistComToBoard.Begin. Existing generated
constructor payloads supply all parameters; no invented defaults or new graph
instance fields. `input_cvar_dist_com_to_board.rs` is an input_cvar_runtime child
using root-reserved definition237. `dynamic_assembly_skeleton_force_physics.rs`
is a skeleton_normal child; `dynamic_assembly_force_physics_pool.rs` is a
skater_pool child. No other files were edited by author.

|Class / selected occurrence|Vtable|Create / Begin / Update / End at +2c/+30/+34/+38|
|---|---|---|
|ForcePhysics90|8230069c|82bb3da8 / 82ca2f98 / 825187c0 / 825187c0|
|SetScoreAugmentation177,1923|82301428|82bb3da8 / 825187c0 / 82ca8228 / 825187c0|
|EndShimmy178|823010d0|82bb3da8 / 82c964c8 / 825187c0 / 825187c0|
|SetDistComToBoard722|8230244c|82bb3da8 / 82cb1208 / 825187c0 / 825187c0|

82bb3da8 is the existing nullable base CreateInstance; no private occurrence
state is added. The other empty callbacks are BLR825187c0. All slot addresses
were read directly from native tables. These four callbacks have no floating
conditional branches. DistCom has an FSEL; reused EndChannel has FCMP equality.

## ForcePhysics

82ca2f98 resolves and checks cached/UCOM IPhysical, calls virtual+48 at306c,
then skips only if the returned Skeleton is NULL. PDB IPhysical_vtbl+48 is
GetSkeleton_2, and Skeleton+3df0 is mPhysAnimState (PhysicsAnimationState).
307c writes the constructor's mForce unchanged. This is the SAME existing
SkeletonNormalAssembly.physics_animation_state read by IsSkateboardAnimated;
it is not an Actor flag or HiLOD shadow field. Existing enum values0/1/2 are
FollowAnimation/ForcePhysics/ForceAnimation.

Actor callback accepts an explicit ForcePhysicsWriter. Root must supply the
actual selected mutable world/pool capability and actual Skeleton handle.
Pool method verifies membership and physical owner and calls the actual setter.
Current ActorPhysicalAllocation retains only physical owner, not world/pool;
the draft does not fabricate pool access. Genuinely NULL native Skeleton is a
successful no-op; missing runtime capability must remain an error. This is the
only additional ownership plumbing needed for these four callbacks.

## SetScoreAugmentation

82ca8228 resolves ISkaterAnim then ISkaterMotionGraph, reads mirrored at839c,
selects mirror+20 or primary+1c, calls virtual+b4 at83c8. Existing constructor
preserves the source mirrorAugment=Switching bug: primary overwritten, mirror
uninitialized. Draft errors only if the unknown mirror is actually selected.

Actual callee SetTrickDataAug825f6ce0 updates retained Motion trick_name and
trick_bits (already consumed/reset by transfer_physics_payloads). Named cases
write name before OR'ing a bit; both reverts leave name untouched. Exact mapping:
FSPowerslide80000000, BSPowerslide40000000, FSRevert20000000,
BSRevert10000000, nosemanual08000000, tailmanual04000000, wipeout00800000,
Land00200000, ridingidle00400000, switching00100000. Names were verified from
initializers82f7be20/38/50/68/80/98/b0/c8; capital `Land` is intentional.
This is a necessary Motion producer, not implementation of parked scoring.

## EndShimmy

82c964c8 does a nullable UCOM IChannelAnimatable lookup and returns if absent.
Otherwise end left651c, then right6540, each reading this.mBlendOutTime and
passing r6=1. Initializers82f84638/50 assign
SKCH_2H_SHIMMY_LEFT_CHANNEL / SKCH_2H_SHIMMY_RIGHT_CHANNEL.
Reuse BodySpin's already-reviewed actual EndChannelPrematurely82d7d648 helper:
dictionary miss is a no-op; blendOutTime==0 (both zero signs, not NaN) writes
length then flags OR68000000, updates actual runtime booleans. A right failure
does not undo the left end. No new channel is created.

## SetDistComToBoard

82cb1208 resolves IPhysical, ISkaterMotionGraph, IAnimatable in that order.
Raw13f8/13fc always dereference live kDistComToBoard before testing the manual
branch1400. If !set_manually, Animation output at PhysOut+38/+48 replaces it;
draft reads actual centre_of_mass_to_deck with no write latch.
If adjust_for_velocity, Motion virtual+24 at1444 is GetLastGoodLandingVelocity
(PDB), followed by GetDeltaTime1450, fmuls1454. fsubs1460/fsel1464 selects
dt*velocity when 0.15-product>=0, otherwise0.15; negative products stay negative
and NaN products select0.15. The cap is raw3e19999a. With adjustment disabled
use+0. Final subtraction1474 then AddSettableAtt1488(name,false,-1), reusing
actual animation writes and their NaN assertion.

Reserved cvar237: kDistComToBoard, ref830a3e18, initializer82f86328,
table82fd4f00. SectionController_Animation_Skater, titleSet Dist COM to Board,
default bits3f266666, GUI min3c23d70a/max3fa00000. Null callback proven raw
82f86364 li r10,0; decompiler extra arguments are bogus. Place by actual table
ordering; GUI sorts by section/title independently.

Focused draft tests cover negative/signed-zero/NaN FSEL adjustment and
augmentation revert/name mapping. Root gates and real repeated-Actor execution
remain required; no full Motion frame or active skating claim is made here.
