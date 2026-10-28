# Ground continuation after force queues

Read-only source audit, 2026-09-12. No implementation or validation claim.
IDA original executable/PDB evidence; original grinds remain explicitly parked.

## Consecutive calls and actual ownership

Ground.UpdateSkateboard calls ManageHangUps82d8d720 at82d9a984,
ManageHalfpipeWheelCatches82d8da78 at a98c, ConsiderPinning82d8dc58 at
a994, then returns at a998. Existing GroundBeforeManageHangUps retains the
same Ground/board/Processed occurrence after all preceding queue/torque stores.

Processed's embedded GrindInvestigation is a distinct owner from the manager's
investigation. Raw82e00ea4 sets r28=Processed+420; ea8 constructs its geometry;
ef0 passes r28 and ef4 calls Reset82b39088, clearing all190 bytes. Its hang flags
at Processed+5ac therefore really begin zero. The existing processed constructor
projection omits these fields. Generic typed construction must remain unknown;
the actual constructor may establish this selected projection once. Ordinary
Processed.Reset does not repeat the embedded reset (394f0 clears grindPossible
at+590 only). Full GrindManager.Update82de1798 ends by copying its investigation
into this embedded owner; the parked schedule does not execute that producer.
Do not substitute the manager's current reset flags for this retained field.

## Hang-up source boundaries

ManageHangUps reads exists mask08000000 from+5ac. When absent, it resets Ground
hang_timer+100 to0. When present, deckSpeed968 is read before kHangSpeedThresh;
deck.Y.Y54 before kHangTiltThresh. Ordered speed/tilt tests increment the timer;
failed tests leave it unchanged. Independently of exists, timer is compared for
equality with live kHangFrames. Equality reads kHangPulseFrames and stores the
countdown+104 before calculating normalized primitive direction/upmost normal,
then kHangUpForce/kHangSkewForce and the retained Ground+80 force.

Positive countdown with a fresh exists flag applies the stored force at the
current deck position, then requires the actual DebugOverlay without a debug
toggle. Countdown decrements afterward; reaching0 resets the timer.

The final test reads fresh deck speed (<1.1), exists and boardslide04000000,
then Processed antiFlipTorque+340 dot3>0. Its true branch requires the actual
board island and GrindGeometryAnalyzer.CalcGrindGeometry. It increments or resets
Ground hung_up_counter+108 before live kHungUpWipeoutFrames. Ordered integer
counter>threshold requests Wipeout index12 (byte20=true, float64=+0, counterb4++).

Thus flags0 do not by themselves justify an unconditional no-op: live
kHangFrames=0 reaches pulse calculation, and a negative wipeout threshold can
request a wipeout after counter reset. Any unrecovered active primitive/geometry
branch must stop at its actual service call after preceding writes. No query
miss or assumed false result may replace it.

HalfpipeWheelCatches reads exists first and returns before any cvar when absent.
Active predicates require actual speed, deck transform, hang location and live
MinWheelCatchDeckSpeed/WheelCatchXY/WheelCatchYY/WheelCatchDistToDeck. The active
torque uses the existing recovered ApplyAngularDisplacementToDeck, followed by
an unconditional DebugOverlay requirement. Active grind geometry stays parked.

## Complete pinning is independently bounded

ConsiderPinning dc70 captures Ground.is_human; dc78 always reads actual
Processed.framesSinceTeleport928 before branching on human. Human samples live
kMaxGroundPinFrames. If eligible, was_pinning bypasses the later live
kMaxGroundPinStartFrames read. It reads the first BE byte of8c8 only when frames>0:
the tested mask is fullword01000000, not lowbit1. Existing player_provided_controls
is ORed with that condition and stored at dd10 before spawn/eligibility tests.

The active path reads DT93c then deckPosition.Y, actual Ground spawn X/Z, and
the full typed deck position. Target is [spawnX, deckY, spawnZ, +0]. Its difference
from deck position preserves unknown W. The source estimates reciprocal DT at
dd98, performs two final-negated VNMSUB/FMA refinements ddac..ddb8, multiplies the
difference at ddbc, and calls the existing actual board SetVelocity at ddc4.
Only after success it sets is_pinning=true. All paths copy is_pinning to
was_pinning at ddd0/ddd4. Ground.Update's existing prefix clears
is_pinning earlier; do not add another clear here. Existing Ground fields and
actual board velocity service suffice; no additional physics policy is needed.

Four cvars needed for the constructor/default hang route and complete pinning:

| Label | Ref | Initializer | Table | Default / GUI |
|---|---|---|---|---|
| kHangFrames |830d9ad4|82f8f468|82fd5748|20 / 0..100|
| kHungUpWipeoutFrames |830d76d8|82f8f518|82fd5750|20 / 0..100|
| kMaxGroundPinFrames |830d8170|82f8f750|82fd5768|90 / 0..180|
| kMaxGroundPinStartFrames |830d7904|82f8f7a8|82fd576c|30 / 0..180|

All section Physics_States_PhysicsGround, null callbacks. Titles respectively
Hang Frames, Hung Up Wipeout Frames, Max Ground Pin Frames, Max Ground Pin
StartFrames. Values remain live; defaults do not authorize eliding reads.

## Remaining solver schedule

After this return Ground.Update calls UpdatePredictedSkateboardPos82da1024,
conditionally RequestPushExit1038, publishes Skeleton+3dd4 at1040, increments
Ground.time_in_state at1054, then tests Processed BE8cc bit0/fullword01000000:
CheckForGrabbableObject106c or actual GrabSpline.Invalidate1088. These precede
Ground.Update completion and are separate from environmental grind exclusion.

Outer Update3 subsequently calls controller.Update at e7d8, shared empty stub
e7f0, Board.UpdatePrePhysics82b37cb8 at e808, StoreForceDisplacement e820 and timer
update e824..834. Board prephysics samples SAME Reckoning.up+1b0, starts actual
wheel line tests at37cfc, then ApplyForces37d04. Existing wheel queries/queue
application must be mounted here in order, not invoked early to manufacture
state inputs. Full solver/Fill completion is not established by queue appends.

Recommended exclusive next slice: new Ground hang/default and complete pinning
children, source-late services/tests, plus narrow constructor-only processed
hang-flag storage. End at a998; preserve explicit active-grind pending boundaries.

## New-only draft admission plan

Drafts now exist, formatted but not registered, compiled or tested:

- constraint_session children ground_hang_pinning and ground_hang_pinning_session,
  files physical_player_ground_hang_pinning.rs and corresponding _session.rs.
- input_cvar_runtime child ground_hang_pinning, slots150..153 in table order above.
- processed Contact child hang_flags from skeleton_processed_hang_flags.rs;
  one member hang_flags: hang_flags::ProcessedHangFlags, generic Default unknown;
  actual try_initialize_constructor_fields calls initialize_constructor BEFORE
  primary GrabSpline construction and ordinary reset. No reset mutation.
- physics_state cfg(test) child physical_player_ground_hang_pinning_tests.rs,
  four focused tests using the real Ground field constructor: live thresholds,
  old countdown decrement without exists, NaN speed read order, unknown W,
  pin start-read laziness, velocity failure and nonhuman unconditional read.

Session API try_update_pair_ground_hang_pinning(board,skeleton,before,refs,numeric)
consumes GroundBeforeManageHangUps and returns GroundSkateboardComplete only after
the selected three calls. It validates the same pair and Ground, releases the
short Reckoning guard before body mutation, and uses existing real SetVelocity.
All source pending cases are errors with completed prefix writes preserved.
Active first exists stops before additional hang predicates/cvars; newly changed
exists at the late geometry predicate stops before its remaining branch. Full
ManageHangUps and active Halfpipe recovery are explicitly not claimed. The native
default values are sampled live, not constants in the kernel.

## Opt-in grind prefix and future publication obligation

The current SKATE2_TEST_GRIND_PREFIX corpus branch calls
physical_player_grind_session::try_update_player_grind_prefix, which invokes
GrindManager::try_update_prefix. It resets only manager.investigation at17b0,
executes Monitor/Permission, updates manager time and optionally FindNearby,
then returns explicitly before Materials.Update1830. It does NOT execute the
later Processed assignment. Repository-wide examination found no existing Rust
assignment of manager investigation into Processed+420, nor any other writer of
the proposed Processed+5ac projection. Constructor flags therefore remain valid
under this currently implemented opt-in prefix as well as the parked schedule.
Invalidating them at prefix entry would invent a native write before its source
position and destroy legitimate prior-tick state.

Fresh source publication: GrindJumper.Update1914 completes first;1918 loads the
manager's actual Processed pointer;191c selects the actual manager investigation;
1920 forms Processed+420;1924 calls GrindInvestigation::operator=. This is the
mandatory future hook. When that continuation is recovered, its same-owner copy
must publish the actual bitfield assignment into processed.hang_flags together
with represented investigation output, under the existing Processed lock. The
callee82dd6c88 is not a whole-word memcpy: it captures old destination18c at6d84,
then copies source MSB-numbered bits0..6 individually, preserving lower25 bits.
Stores are6dc8/6dd4/6dec/6dfc/6e0c/6e1c/6e28; hang exists08000000 commits at6e0c,
boardslide04000000 at6e1c. Its final word is
(old & 0x01ff_ffff) | (source & 0xfe00_0000). Preserve individual source loads and
stores if any intervening partial observer is represented. Do not wholesale
replace the destination with manager.flags. If the
source assignment is admitted while any flag bits remain unreconstructed, set
that projection to None at1924 and let downstream reads refuse it. Never leave
constructor Some(0) across an admitted real assignment. No publication hook is
mounted now because there is no current production call at that boundary.

Future regression oracle: establish constructor0, execute only prefix and verify
it is preserved; execute actual final assignment using distinct nonzero flags
and verify the SAME Processed word changes; execute ordinary Reset and verify
that word remains unchanged. Do not call a test-only setter and claim the full
native investigator/balance/engagement/jumper chain ran.

## Root acceptance checkpoint58

Latest accepted connected Skeleton output checkpoint: **workspace58 passes 4,085 unit tests and six doc tests; original50 passes all 17 core corpus tests (9.07s); Clippy94, format43 and diff44 pass.** A fresh real Actor-backed pair now executes actual ProcessData, original-terrain solve/feedback, complete Skeleton post-physics and full Skeleton.Fill through mandatory IK/audio collision-data publication. COM velocity is captured once at585b0 and carried through the same output tokens. Ground.UpdateSkateboard also reaches its a998 return through retained constructor hang state and complete pinning. Active hang geometry/pulse services remain explicit pending branches. This is a component schedule with controlled solver iteration/freezing settings; full Board.PostPhysics, outer Ground/controller/force scheduling, output conditioners and animated trick consumption remain pending.
Root independent raw review confirmed same-owner bindings, all mandatory IK/audio source calls, COM capture, guard ordering, actual controller non-held predicate and Hang/Pin defaults. Narrow records reciprocal uses final-negated VNMSUB at541a0/a8; generic drive policy remains unchanged. Original50 includes successful freshActor solve/fullFill and all prior fixtures.
