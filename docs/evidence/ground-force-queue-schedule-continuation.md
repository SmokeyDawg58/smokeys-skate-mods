# Ground force production through the solver boundary

Read-only source/runtime audit, 2026-09-12. No code changes or Cargo. Confidence
high for source ordering and current owner inventory. This is a continuation
plan, not a claim that the Ground frame or force controller is complete.

## After the real wallride producer

Ground.UpdateSkateboard82d9a018 receives the force and actual jump-info writes
from WallRideForce82b29300 at a1a0. It updates/reset time_wall_riding+b0 and
clears exiting_wallride+f0. Next live kApplyFakeTorque lookup a1d8 gates a1e4:
enabled executes source log/exp graph and ApplyLocalTorqueToDeck a42c; disabled
reaches actual Ground doJump+e4 at a434. No constant-false debug substitute.

True jump path:

| Call site | Actual callee | Existing owner/service |
|---|---|---|
|a444|SetVelocity82b29ee0|`try_set_board_velocity` already writes same actual board Parts; typed XYZ consumer|
|a450..454|copy newVelocity to Processed190|requires SAME typed Ground jump vector publication, not a new calculated velocity|
|a460|SetLinearDrag82b23a98(+0)|`try_set_board_linear_drag` already writes actual board body|
|a468/a474|TrajectoryLaunchInfo ctor / Skeleton.GetLaunchInfo82b46098|complete live launch-info producer still missing|
|a478..498|overwrite startVel/velBeforeJump with Ground jump velocity, playerJumped=true|must preserve every other field produced by real GetLaunchInfo|
|a49c|TrajectorySelector.Launch82dc4098|retained selector exists; active launch/query producer not complete|
|a4a4|TrajectorySelector.Update82dc6528|existing constructor polling is not the completed active query route|
|a4a8..4d0|actual DebugOverlay singleton require|real singleton can be reused; no draw invocation here|
|a4d0|exiting_wallride=true|actual retained Ground field|

The jump path bypasses the ordinary force calculations because source branches
there; a host first-test policy must not manufacture this decision.

## Actual non-jump collision decision

At a52c, source calls `Toolkit_CalcCollisionForce82b27530` with an initialized
force output, a required torque output, and a NULL optional third vector.
`SkateboardSlideCollisionServices::try_collision` already implements that leaf,
with SAME runtime.slide_collision_attributes and actual typed inputs needed.
Current trait implementations are controlled corpus/tests only. The next useful
adapter must read actual Processed collision flag, velocity, collisionHookToHips,
surface velocity, mass/DT, up and retained Reckoning normal at their getter sites.
No new generic collision response should be written.

On true, source first copies force/torque into retained Ground fields, reads
global collision timer tuning, stores timer, calls TargetAngularDisplacement
a5d8, sets board force flag+530/80000000, and adds kCollisionResponseForce at
a5f4. It then normalizes the actual returned force, dots it with Processed190,
reads live kDotToStopPush and stores actual request_push_stop. The existing
queue add API is reusable; target-angular-displacement accumulation still needs
its own original body/drive consumer. A true result cannot be treated as merely
an alternate force record while discarding these effects.

## Ordinary force production order

Only the source false collision branch reads the feet wallride-time gate and
performs this sequence. Most calls return records through hidden output
pointers; a Rust caller may keep typed local records but must not enqueue early.

| Site | Callee | Current implementation |
|---|---|---|
|a6bc|Braking82b26df0|existing `SkateboardBrakeDragServices::try_braking`|
|a6c8|PushAcceleration82b272f8|missing live/source evaluator|
|a6d0|AbsorptionFootForceMag82b28c60|missing live/source evaluator|
|a6ec|FrontFootForce82b28b70|existing `SkateboardFootForceServices::try_front`|
|a704|RearFootForce82b28d98|existing `try_rear`; same absorption value, fresh timeOnGround read|
|a710|UpdateSpeedModel82b2a240|existing constructor/Reset is not Update; live evaluator missing|
|a720|SlideFriction82b28260|existing `SkateboardSlideCollisionServices::try_slide`|
|a72c|PumpForce82b28ed8|actual Pumping history/effect exists; force-producing wrapper still missing|
|a73c|StraightenOutTorque82b28560|missing source evaluator/retained torque consumer|
|a748|HeadingTorque82b30cc8|missing source evaluator/retained torque consumer|
|a754|AntiFlipTorque82b299a8|missing source evaluator; result retained in actual Ground+90|
|a774|CalcManualEffect82b32130(false)|missing evaluator; produces torque AND corrective-force record|
|a77c|LinearDrag82b27078|existing `try_linear_drag`|

DO NOT reuse `try_update_board_grounded_force_tail` as this sequence. Despite
its broad name it reconstructs GroundAnimation82d8a8b8..a9cc: front/rear first
with fixed parameters1/0, then brake/slide/drag, collision LAST. PhysicsGround
uses collision FIRST, different foot inputs, and many intervening producers.
Reuse its individual leaves and same runtime providers, not its caller schedule.

## Commit order after calculation

The caller calculates push magnitude/combined push+brake, may set
is_push_accelerating, then calls UpdateAntiFlipNudge a7e4. The wallride-time
gate controls adding front/rear forces at a800/a810. Brake then push are added
a820/a830. An active collision timer triggers late fadeoff lookup, DT subtraction,
retained collision-torque scaling and TargetAngularDisplacement a89c. Then:

1. Target straighten torque a8a8, target heading torque a8b4.
2. Apply anti-flip angular displacement a8c0, manual displacement a8cc.
3. Conditionally add speed-conservation force a8f0 (actual Processed AI bit).
4. Add manual corrective, slide, pump, wall forces a900/a910/a920/a930.
5. Set captured linear drag a93c; set board force flag from actual combined force
   magnitude; ManageHangUps a984, ManageHalfpipeWheelCatches a98c, ConsiderPinning a994.

`SkateboardForceQueue` and `PhysicalAssemblyWorld::try_add_board_force` already
retain source insertion order and silently ignore entries above capacity21.
The actual runtime queue belongs to the same admitted Skateboard; each force
record carries typed local position and world force. Existing controlled corpus
proves original body application but is not the complete Ground caller.

## State tail and later prephysics consumer

Returning from UpdateSkateboard does NOT complete Ground.Update. At82da1024
it calls UpdatePredictedSkateboardPos, conditionally RequestPushExit1038, writes
Skeleton+3dd4 at1040, increments Ground time_in_state+a8 at1054 using DT, then
tests the BIG-ENDIAN first byte at Processed8cc bit0 (full word01000000).
True calls CheckForGrabbableObject106c; false invalidates actual PlayerGrabSpline
query1088. This environmental-grab path is distinct from airborne board grabs.

Outer `PhysicalPlayerHiLOD::Update3_State82dfe708` waits for the actual selected
state.Update virtual call e7d0 to return, then executes:

| Site | Stage |
|---|---|
|e7d8|actual SkateboardController.Update (retained controller exists; full Update dispatch still required)|
|e7ec/e7f0|fresh GetSkateboard then shared empty native stub, misleadingly named AIProcess.OnEnd; raw body is empty|
|e804/e808|fresh GetSkateboard then Skateboard.UpdatePrePhysics82b37cb8|
|e81c/e820|fresh GetSkateboard then StoreForceDisplacement (needs real deck torque/inertia/omega calculation, not just queue length)|
|e824..834|fresh Processed DT then HiLOD state timer+14c increment|

UpdatePrePhysics captures the SAME Reckoning.up+1b0, gets actual body and calls
StartWheelLineTest37cfc, THEN ApplyForces37d04. Reuse existing
`try_start_board_wheel_lines` and `try_apply_board_forces` in that order. Existing
wheel corpus does exactly this around a selected authored-ground step, but the
complete state/controller path above must precede it before a full-frame claim.

Queue Apply35380 captures its authored local-Y offset only for nonempty queue,
walks the retained insertion range, resolves real deck Part6 anew per entry,
then uses the existing backend `try_apply_skateboard_queued_force` to commit
actual force/torque. It DOES NOT clear the queue. Source postphysics37e78 clears
it later; errors preserve the native committed prefix and retained entries.
The host solver step must consume the same bodies/forces/drive payloads after
all required prephysics producers. Update4_PostEngineAdjust82e01c78 begins with
EndSkeletonLineTests, actual board UpdatePostPhysics, conditional bad-acceleration
wipeout request, then Skeleton/state postphysics and wipeout checks. Running
selected authored-ground solving in a corpus is not proof this full outer
simulation schedule is connected.

## Smallest next closure recommendation

After accepted WallRide and live fake-torque gate, add the actual non-jump
CollisionForce input adapter and preserve its real boolean/force/torque output.
Complete its positive side effects and the next false-path braking/push producers
in source order. In parallel-sized future slices reuse existing foot/slide/drag
leaves, recover PushAcceleration/Absorption/PumpForce and the retained torque
pipeline, then join the existing real queue application and wheel Start path.
No solver-before-force shortcut, reordered GroundAnimation tail, fabricated
noncollision result, or omitted active jump selector is justified.
