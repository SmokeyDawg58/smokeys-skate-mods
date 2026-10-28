# Physics conditioner: first state publication

Selected implementation, 2026-09-12. This is the first three calls of
`Sk8::Physics::PhysOutConditioner_Physics::Update` at **82e2f970**, not a completed
physics frame, `UpdateAll`, or a forced transition out of the constructor state.
No original source files or assets are copied into the repository.

## Actual caller and ownership

Fresh IDA/raw review establishes this schedule:

1. `World::Update3` calls Solver3, physical Update4_Adjust, physical
   Update5_ProcessOutput, then `World::UpdateConditioners` at its **827af9e8**
   call site. PhysicsOutput.UpdateAll follows conditioners.
2. `World::UpdateConditioners827af468` constructs and resets its external input;
   reads Simulation tick/visualTick and DataSysMan history; consumes actual
   challenge/scoring/PlayersBackEnd inputs; walks PhysicalCollection order and
   flags; then invokes the selected physical's UpdateConditioners.
3. `PhysicalPlayerHiLOD::UpdateConditioners82dff150` passes its retained
   `mPhysOutConditioners` to `PhysOutConditioners::UpdateAll82e29f88`.
4. UpdateAll order is Intents, Grinds, Mission, Audio, Camera, Physics, Animation,
   Scoring, Stats, PathRecording, GameCam. No earlier callback is silently skipped
   by this implementation; the World/owner scheduling entry is deliberately not
   mounted yet.
5. Physics.Update calls SystemVelocity at **82e2f984**, EmpericalMeasurements at
   **82e2f98c**, FilteredPhysicsState at **82e2f994**, then DampedComPos, FootSide,
   LandingQuality, WatchForStairs. The selected returned continuation is before
   the DampedComPos call at **82e2f99c**.

Existing `Conditioners.physics` retains the actual constructor/reset payload
`simple::Physics`, bound to the same PhysOut occurrence as its parent.
`try_update_physics_prefix` checks both identities and uses that payload, without
replacing histories. New conditioned stores write existing component fields;
no parallel output fields were added. There is no Actor/world mounting method.

## System velocity and empirical measurements

`UpdateSystemVelocity82e25a40` reads SystemReckoning+10 (COM velocity), multiplies
all four lanes by .25, then computes old conditioner+10 times .75 plus that
separately rounded product using FMA. Raw **15bfe0b1** at82e25ae0 has VMX128 VA63,
not the disassembler's erroneous95. The ordinary VMADD at82e25ae8 uses VA0,
VC12, VB13. It stores the conditioner vector before the second output guard and
the SystemReckoning+0 store. The coefficients are native float constants
3e800000 and3f400000, not tuning invented by the reconstruction.

`UpdateEmpericalMeasurements82e25830` obtains SkateboardReckoning+90 and
Physics+0 timestep with independent guards, in that order. Signed
framesSinceReset>0 computes `(currentPosition - oldPosition) * reciprocal(dt)`.
Signed frames>1 separately computes `(newVelocity - oldVelocity) * reciprocal(dt)`
with a second native estimate/refinement sequence. No dt clamp or normalization
occurs. Native vector loads/stores include W; unknown W remains unknown.

Each VREFP is an explicit `ReckoningNumeric::reciprocal_estimate` boundary. The
local refinement executes two exact graphs:

`residual = -(estimate.mul_add(dt, -1)); estimate = estimate.mul_add(residual, estimate)`.

The final negation matters: when estimate*dt is exactly1, residual is negative
zero. This does not reuse the broader shared helper's algebraically rearranged
positive-zero residual. Host seeds/FMA remain the existing declared host numeric
policy; Xenon estimate tables, denormals and NaN payload parity are not claimed.
No reciprocal-square-root or FSEL instruction occurs in these two functions.

At82e2597c onward, the store order is: wrapped frames+1, velocity, lastPosition
from oldPosition, position from current input, independently guarded output
empiricalVelocity, empiricalAcceleration, framesSinceReset. A failing second
estimate has not yet stored empirical histories; a later output failure retains
all preceding history stores and the prior SystemVelocity update.

## Filtered state

`UpdateFilteredPhysicsState82e2d800` first copies retained current filtered state
to last filtered state at82e2d820. It then independently reads State+10 state,
State+c category, Collision+d74 any-contact, Ground+120 wallRideExit, State+c
again, and only for category100 Collision+10 surface type (8 means stairs).

It writes mustChange, airCount, nonspecificCount, collisionFreeCount,
collisionCount and framesSinceGroundStairs in native order. The two Nonspecific
contact reads are separate lazy reads. Integer increment uses wrapping addition;
the branch comparisons are signed. mustChange is set for previous Wipeout or
Invalid, or previous physical Teleport702 followed by another physical state.

Supported source state mapping:

| Actual state | Result |
| --- | --- |
|100|Ground, or Air when captured wallRideExit is true|
|101..105,602|Ground|
|200|Air-count/stairs hysteresis, or forced change from retained mustChange|
|201|Ground only when captured any-contact and a late Air+1ab targeting-grind read is false; otherwise Air|
|300|Wipeout|
|500,501,504|OffBoard|
|502,503|OffBoard/OffBoardAir from late OffBoard+45 hasLanded|
|505|Ground/OffBoard from late OffBoard+41 onDeck|
|600,601|Air|
|701|Retains current filtered state|
|702|Teleport|

Native grind states400..404 return the explicit parked-state boundary after the
preceding counters. Their grind-specific producer is not expanded. Other invalid
states return the native assertion boundary82e2e390. In particular, constructor
state0 does not become Ground.

For state200, the `kGroundStairsFrames` target is read only when retained filtered
state is Ground and airCount>3. The signed stairs comparison must also be > the
live target to switch, unless mustChange requires it. Cvar initializer
**82fa0418**, table**82fd6454**, ref**8311e518**, section
`Debug_Physics_Conditioners`, label`kGroundStairsFrames`, title`Ground Stairs Frames`,
default9, GUI0..10, null callback. It occupies reference index80; source static
order places it before the existing82fd6494 reference. Startup values remain
unclamped and live.

The selected state's retained store precedes lastPhysicsState, then output
Filtered current, last, lastGrindDist. Non-Grind metadata stores are individually
guarded in order: typeNone, isGrinding=false, isFront=false, name, scoringName,
score=-1, crouch=+0, pathedGUID=-1, localGUID=-1. NULL_SKATINGGRINDNAME's initializer
82f7bd30 uses source empty string8225fe43 (first byte0); all five FastString words
are zero. GUID stores82e2e858/884 use the sign-extended `li r28,-1` frome6b0.
Existing retained Grind history can remain selected by Nonspecific701; its
publication asserts that its GrindType is not None rather than inventing history.

## Field ownership

| Conditioner offset | Existing `simple::Physics` field |
| --- | --- |
|10|damped_system_velocity|
|20/30/40|deck_position / last_deck_position / velocity|
|90/94/98|last_filtered_state / filtered_state / last_physics_state|
|a4/a8/ac|frames_since_reset / air_count / nonspecific_count|
|b0/b4/b8/bc|nonspecific_collision_free_count / nonspecific_collision_count / frames_since_ground_stairs / must_change_filtered_state|

Output destinations reuse `SystemRemainder.damped_centre_of_mass_velocity`,
`MotionRemainder.emperical_deck_velocity/acceleration`, `State.frames_since_reset`,
and the existing `Filtered` fields. Retained grind metadata remains in the existing
Physics payload. No new scalar or vector owner was introduced.

## Validation and outstanding producer

Eight focused tests cover actual same-output binding, first-frame numeric call
absence, all four velocity/acceleration lanes, separate estimate failures,
intermediate signed zero, later component failure after history writes, lazy/live
stairs cvar, KnownAir late reads, constructor assertion, counter wrap and unknown
W. They are added for the parent's combined Cargo gate; this agent ran rustfmt
only and has not claimed a test pass.

Required real producer: `Update5_ProcessOutput82e04ea8` begins with PhysOut.Reset,
line-test output and actual board/skeleton/managers' FillPhysOut calls. It later
reads the actual state owner's GetType to publish state and category. Neither
that sequence nor the prerequisite World conditioner inputs are replaced by this
selected implementation. The recognizer corpus currently verifies Ollie intents
against unloaded controllers; the loaded Actor corpus verifies two Default
ticks. Neither is evidence of a completed flick-to-loaded-graph route.

Next graph boundaries, after real state publication: Action OnGround19 enters
Grabbing45 and first reaches unsupported expression57 atDBLGrab80; after genuine
selection, OnBoard behavior45 needs BodyFlippingSignal CreateInstance82caec70 and
Begin82c94840. These are recorded, not implemented ahead of the state producer.
