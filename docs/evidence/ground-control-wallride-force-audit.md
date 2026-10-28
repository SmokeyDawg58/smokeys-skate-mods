# Ground control wallride force: source and runtime audit

Read-only audit, 2026-09-12. No implementation or Cargo. Confidence high for
reported owner/call/branch mapping; active vector arithmetic still needs a
separate full raw operand review before implementation.

Follow-up: parent authorized new UNREGISTERED complete drafts. These now exist
as `skateboard_wallride_force.rs`, its controlled tests,
`skateboard_wallride_attributes.rs`, and `physical_player_wallride_source.rs`.
No shared mutation, registration or Cargo has occurred; the original read-only
audit above describes its initial status. Source refinements and remaining
registration dependencies are recorded below.

## Mandatory caller and actual outputs

After control setters a118/a124/a12c, Ground.UpdateSkateboard82d9a018 captures
Ground.time_wall_riding(+b0) at a138, creates a full-zero local force pair,
captures actual Ground jumpInfo+d0 (r7=a180), then reads current
Processed.timeOnGround9c0 at a19c. The call at a1a0 is
`Skateboard::Toolkit_CalcWallRideForce82b29300`.

Important ABI: reference arguments are r6=outputforce and r7=actual Ground
jumpInfo. The decompiler's r4/r5 stack-pointer labels are not those arguments;
the two preceding float inputs occupy their ABI slots. Result is a boolean in
r3, not a hidden returned force. Existing Ground fields map the SAME jumpInfo:
newVelocity=d0 `jump_direction`, COMheight=e0 `jump_com_height`, doJump=e4
`do_jump`. PDB `SkateboardJumpInfo` offsets0/10/14 establish this mapping.

On true, caller reads current Processed DT93c and current Ground wallride time
again, adds and stores+b0 at a1c0. On false, stores +0 at a1c8. Both then clear
Ground.exiting_wallride+f0 at a1d0. Neither path directly enqueues the returned
force; later ordinary-force application calls AddSkateboardForce(kWallForce)
at a930. Preserve the actual local force until that consumer.

## Consecutive measured inactive route

Source reads three actual global physics_feet.default scalars, using existing
class5338958c408b6009/defaultd7edbd362d7d2152 provider:

| Lookup | Key | Later role |
|---|---|---|
|29384|baba91606102af9a|maximum absolute ground-normal Y|
|293dc|19c804c3e3e2b566|minimum dot(up,groundNormal)|
|29434|90c4e79cbfbaed5e|active hit-normal alignment limit|

All three lookups/default branches and scalar loads occur before any jump-info
write, including the third scalar even if the initial gate fails. Native stores
doJump=false at2945c, COMheight=+0 at29488; newVelocity is NOT cleared. It then
captures Processed and reads groundNormal1d0, upVector220, computes dot3(up,
groundNormal), reloads groundNormal to test abs(Y). The first comparison is
maximumY>abs(Y), ordered. Only if true does scalar dot>minimum pass; raw29500
BO4 BI25 includes unordered in the skip branch. Both false gates reach29974,
which writes two full positive-zero vectors to the force output, and returns
false. Those zeros are native successful outputs, not fabricated query misses.

Thus the smallest exact branch-complete implementation can perform these
reads/writes and finish on a genuinely measured false gate. On a passing gate
it must stop before the active reads with an explicit continuation, preserving
the already-cleared actual jump flags. Flatground cannot be presumed from the
state name, ctor normal or expected Vault value; the live gate must be evaluated.
There is no query or debug cvar read on the false route.

Existing owner/input reuse: `force_runtime.foot_attributes` supplies scalar
pointer lookup/load. Processed.prior_output().ground_normal() is1d0 and
reckoning_up() is220, retaining source provenance/unknown W. Ground fields
already exist. Reuse `ReckoningNumeric::dot_vector` because W is not consumed.
The output carrier can reuse `SkateboardFootForce` (two typed vectors), retained
until the later real queue add. No new World or board force owner is needed.

## Active route dependencies

Passing the gate sets local resulttrue at29510 and captures groundNormal1d0 and
velocity190. Additional feet scalar at29558 and cached feet PointGraph8 at
layout+10 are read before the vertical force is computed from Processed mass970
and gravity964. Native operation is negative((mass*gravity)*graph(timeOnGround)).
The first live kDrawWallRide read is here at295d8, followed by actual singleton
require only when enabled.

Query preparation is conditional on (timeOnGround>0.01 and numWheels90c>0) OR
timeOnGround>the newly read scalar, with source short-circuit reads preserved.
It uses actual physics_trajectory.default cached layout+1a0, board position70,
captured normal and two cross operations/NormalizeSafe, then a length gate.
Degenerate normalized displacement returns false early at2974c before common
force-output stores; the caller-created zero force remains intact.

Nondegenerate route obtains the SAME Entity's actual PhysicsIsland at2976c and
calls `PhysicsUtility::LineWorldCollisionTest82e4a408` at2978c with fatness+0.
On a hit it reads these authored feet values, preserving source order:

| Getter | Key |
|---|---|
|WallRideAutoJumpHeight82b22d70|843a276ac05b4a8b|
|WallRideMaxTime82b22e00|569df1130ffddbb6|
|WallRideVelTimeToConsider82b22e48|c2bce9a7d58be7bb|
|conditional WallRideAutoJumpYDownScalar82b22db8|72914fa624d2947b|
|WallRideAutoJumpForce82b22d28|fd00fa386f0ac466|

Hit-normal/height/velocity/time tests can set actual doJump=true before later
attribute reads and publish full newVelocity. Late failures must preserve that
prefix. kDrawWallRide is sampled again after query, during hit evaluation and
after velocity publication; no eager cvar capture or invented inactive target.
Source reference is8308db08; initializer/default/table still to be recovered
before reserving/registering its live getter.

## Actual collision service is reusable

Fresh original vtable proof: Island vtable820ffa10+44 and+48 both point to
827b2538 (named LineCollisionTestWithVehicles). Utility82e4a408 calls slot+44
at a470. That actual implementation forwards to World.LineCollisionTest827ab338.
Therefore the existing `SimulationWorldLineQueries::try_line_test` default7
route, already used by live IK and original terrain tests, is applicable after
checking the actual owner Island. Reuse the SAME world_line_queries, DMO owner,
StaticGroundPool and existing synchronous acquire/begin/add/end/sync/get/release
lifetime; do not use the lagged skeleton-trajectory batch in its place.

Existing outer `PhysicalAssemblyWorld::try_world_line_test` exposes this route.
The utility wrapper itself copies successful lineParam, hit=true, full normal,
full position, then surfaceId; failure writes ONLY hit=false. PDB result layout:
hit0,param4,position10,normal20,surfaceId30. Current line result already includes
normal and position; typed W remains whatever the real collision producer knows.
No custom terrain scan, unrelated island, replacement dynamic collection or
empty result is justified. Native label is `Skateboard::Toolkit_CalcWallRideForce`.

## Following caller branches remain explicit

a1d8 reads kApplyFakeTorque. Disabled proceeds to actual doJump a434; enabled
needs source log/exp graph and real ApplyLocalTorqueToDeck a42c. DoJump true calls
actual SetVelocity, drag0, Skeleton launch-data producer, selector Launch+Update
and singleton check. DoJump false calls actual CollisionForce a52c; its false
result selects ordinary braking/push/feet/speed/slide/pump/torque calculations.
These are source-produced decisions, not permission to skip wallride, jumping
or collision because the first test starts on Ground. Existing force/steering
leaves should be mounted on their retained owners in later bounded slices.

## Complete draft and precise integration dependencies

The draft covers active and inactive branches, including the degenerate early
return, real synchronous query, optional hit, jump selection and partial writes.
Controlled tests exercise a positive hit with unknown velocity/normal W,
genuinely measured flat gate, and missing jump coefficient AFTER doJump=true
but BEFORE newVelocity/common force stores. They are not run or claimed passed.

Raw active numerical mapping:296a4 forms J cross normal;296c0 forms that result
cross normal. Reuse the single typed cross evaluator with its reviewed VNMSUB
rounding, then the existing NormalizeSafe824b18e0 body with exact zero default.
Position70 is captured before trajectory scalar1a0; start is full fused
normal*height+position at296bc. Displacement is normalized direction*4, and
the early false gate is SMALL_FLOAT0x37800000 greater than its dot3 squared.
NaN does not take that ordered gate. The hit prediction is the FULL fused
`velocity.Y * timeToConsider + abs(hit.Y-start.Y)` at29864. Jump velocity first
subtracts rounded normal*dot(normal,velocity), conditionally multiplies only Y,
then full fused normal*jumpForce+tangent at29954. Native target capture29844
precedes the numeric hit-normal dot29848; bool load2984c follows it. The draft
keeps distinct capture/load callbacks rather than eagerly sampling all debug.

Correct global trajectory proof (do not use the similarly named HandPlant
class): generated physics_trajectory ctor828b1068 raw1088/108c loads
class429a8f4385dd539d and fallback layout0x24c. Global constructor828b1608 calls
it at828b291c and retains+e8. The earlier handplant class2656db32bef0b68d is a
DIFFERENT embedded selector class. New `WallRideTrajectoryAttributes::bind`
must be invoked once as a retained projection of the existing owner/global
lifecycle, not per call or by borrowing the unrelated HandPlant instance.
Existing global feet provider remains the sole feet instance; new child method
reads its cached0xf0 layout (generated828b0e10) at graph offset10.

Debug reference8308db08 initializer82f7b138/table82fd4414 is now recovered:
sectionPhysics_Skateboard, labelkDrawWallRide, title`Draw Wall Ride Info`,
defaultfalse/null callback. Registration needs a newly coordinated slot and a
small capture-target/read-value adapter equivalent to the existing sideways
debug adapter. Draft source trait requires that real adapter; no constant false.

Actual runtime source borrows existing fields and services. It errors on absent
World queries/DMO owner only at active2978c, and calls the same query method and
same host collision numeric policy used by IK. It does not initialize a new
World. Source selection must verify the board/Skeleton/Ground/Island identity
before constructing those borrows. The outer caller still needs to hold its
source-initialized force record through a930 rather than enqueue it early.

One shared type amendment is required before registering the active sink:
migrate SAME Ground.fields.jump_direction from [f32;4] to SkeletonRootVector.
Actual normal/velocity can have unknown W, which full newVelocity29958 must
retain rather than reject or fabricate. Constructor still writes four known
zeros. Direct callers are only Ground exit110 (full acceleration FMA) and one
physics-state test fixture/oracle; migrate that full FMA preserving W knowledge.
The similarly named field in remaining_constructor_owners is unrelated and
must not be edited. The draft assigns the typed vector directly and therefore
intentionally waits for this narrow shared migration before compilation.

## Root source and integration acceptance, September12

Root independently reviewed full29300 decompile and raw29448..29514,
29594..296f8,297b4..29910 and actual caller82d9a130..a1e4. Branch bytes
29500/29610/29628/29894 are4099 NOTGT;29880 is4098 NOTLT. Fused/cross
operand order, live debug capture and jump publication match the draft.
Registered same-owner runtime adapter, once-bound global+e8 read capability,
cvar117 and existing Ground jump field typed migration. Frame independently
reviewed root caller/cvar and shared migration: PASS, no defects.
workspace39 PASS3,948units+6docs; original36 PASS17/17; Clippy60,
format31,diff31 PASS. Native per-tick traces and complete force/solver schedule
remain absent; current original scenario plus focused hit/jump tests establish
selected coverage, not all-branch original execution.
