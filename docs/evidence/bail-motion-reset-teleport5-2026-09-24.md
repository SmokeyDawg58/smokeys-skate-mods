# First bail continuation: Motion Teleport5 resets

The original-input bail corpus enters `WipeoutGround` at step 8, requests
teleport at step 256, completes Teleporting entry at step 257, and reaches
generated Motion state 5 at step 258. Frame 258 completes physical Ready
Fill; frame 259 reaches the first physical teleport reset operations. These
are reached-path observations, not a claim that remount is complete.

`ResetSkaterAnimation` behavior 36 has callbacks null Create
`0x82bb3da8`, Begin `0x82ca4f10`, and empty Update/End `0x825187c0`.
The Begin first verifies the Actor's published/registered IActionGraph,
ISkaterAnim, and ISkaterMotionGraph interfaces (Actor `+0x1f4`, `+0x1fc`,
`+0x1f8`) before calling their resets in ActionGraph, SkaterMotionGraph,
SkaterAnim order (`0x82ca5114`, `0x82ca5128`, `0x82ca513c`). The final
`SkaterAnim::ResetSkaterAnim` at `0x82c8b6a8` tailcalls
`ChannelAnimatable::ResetChannelAnimatable` at `0x82d7d220` once. The Rust
path uses the existing registered graph and animation occurrences, clearing
their retained stores rather than constructing parallel owners. The active
blend-space-job wait remains an explicit typed boundary in the isolated
animation leaf.

`ResetToGivenStance` behavior 37 likewise uses null Create and empty
Update/End, with Begin `0x82ca2ea0`. Raw vtable dispatch through
ISkaterAnim slot `+0x2c` resolves `SkaterAnim::ResetToSpawnStance`
`0x82cb77e0`; the Hex-Rays interface label for this call is misleading.
The leaf reads natural stance `+0xe58` and spawn-relative stance `+0xe64`.
For `(1,0)` or `(0,1)` it clears the top two flag bits and sets relative
stance from `natural==0`; otherwise it sets the top bits and sets relative
stance from `natural!=0`. Both branches then clear spawn-relative stance.
Raw PPC disassembly establishes the branch and store order.

The reached StateUpdate_Teleport_5 at `0x828ee130` calls parent Motion0
`0x828edd40` for behavior slots 0–3, calls the shared empty update twice
for 36–37, then calls PlayAnimation.Update for slot 38. The existing
generated Motion dispatcher now lists `[0,1,2,3,36,37,38]` for this one
state/address pair. An independent read-only IDA review checked each of
these integrations for callback order and duplicate effects.

Validation: the low-memory `skate2-core --lib --no-run` builds passed after
both reset integrations and the Teleport5 state-update arm. The original-data
bail replay passed behavior 36, behavior 37, and Teleport5 StateUpdate;
frame258 completed Ready Fill. The reached frame259 transition expression367
was source-audited at `0x82916bac`: it evaluates condition285
(`InParentStateForTime`, greater than 0.1) and only then the negation of
condition0 (`PhysFilteredState(teleport)`). Both existing condition runtimes
use retained state/physical owners. Source-audited Sustain271 returns the
existing cached condition402 (`PhysFilteredState(teleport)`) directly.
The current original-input bail replay passes both and reaches the physical
`CheckForTeleport` call at frame259. That caller captures all four typed
matrix columns, then the mounted `TeleportPlayerNow` prefix invokes the
retained SkeletonController Set(6) before clearing the HiLOD dismount
counter. The next source-backed prefix builds a yaw-only reset orientation
from the horizontal input Z axis, adds `UpABit=[0,0.1,0,0]` to the full
translation, applies the exact HiLOD flag mask, and clears the existing spin
history. The original-input replay passes this prefix and stops at
`TeleportPlayerNowLineResetPending` before three skeleton line-record clears.
The next two source-backed prefixes now clear the three retained line results
and starts in place, preserve their query-batch pointers, invoke the already
constructed full PhysOut reset, then call the existing SkateboardController
Stop and in-place retrieval reset. An earlier original-input bail replay
reaches `TeleportPlayerNowBoardResetPending` before
`Skateboard::TeleportReset`. Its adjusted translation Y is `-54.181004`
after the addition. Independent
IDA reviews checked the reached caller, matrix arithmetic/order, owner
identity, flag mask and stores. Xenon reciprocal/rsqrt seed and rounding bit
parity remain open. Neither generic orthonormalization nor a second reset
owner was introduced.

The attached board's no-op AI end callback precedes its original seven-Part
`InitializeTransforms` in 6, 0..5 order. `SetEffectiveTransform` then reads
Processed+`0x8c4` bit `0x00100000` and applies the same full-lane X/Z sign
XOR already used by `GetEffectiveTransform`. Its Body setter reuses the
existing attached board/hook transform graph and the live linked
`kOrthonalTolerance` cvar. Unknown W stops typed before that setter; the
raw-bail frame259 matrix has all lanes known, and the replay passes it.

`SkateboardBody::TeleportReset` resets each of the seven existing Part
bodies' velocity and angular velocity to zero, force to their current gravity,
and torque to zero. The shared motion helper now requires an attached body on
this path; Skeleton reset retains its distinct nullable branch. The same
retained truck-drive owner receives force-turn=true and positive-zero front
and back angles in source order. `CollisionInfo::Reset` spans the retained
contact surface, construction post fields, and wheel-line output. The live
path interleaves its prefix, seven-Part loop, and four-wheel tail; it does
not call the narrower constructor/test reset projections wholesale. The
prefix clears max contact velocity, stores both full up normals, clears
counts and max speed, then masks flags. Each Part trip repeats the zero
max-velocity swizzle, clears contact fields, shared wheel height, opposing
delta and surface words. Each wheel writes full up line normal, zero
distance, and zero validity in that order. The final Body stores clear only
the two modeled high grip bits, then zero the grip cosine and full vector.
Independent IDA reviews checked owners and store order. The original-input
bail replay passes the complete Body reset and stops at
`TeleportPlayerNowOuterBoardResetPending` at frame259.
The outer `Skateboard::TeleportReset` now reuses the retained steering,
truck-drive, hook-drive, board-animation, and queued-animation owners. Its
source-ordered positive-zero deck and truck stores, identity steering error,
hook dynamics reset, animation queue clear, target-speed zero, and jump-normal
world-up pass the original bail replay. `ResetSystems` then resets the same
Reckoning owner and reads the attached Part6 board transform without the
Processed board flip. The frame259 replay stops before its grounded Reckoning
update with that typed transform; all four Part6 W lanes remain unknown.
Independent IDA reviews confirmed both call sequences and the single-owner
reuse. Evidence: `artifacts/raw-bail-teleport-outer-board-reset-2026-09-24.log`
and `artifacts/raw-bail-teleport-resetsystems-part6-2026-09-24.log`.
The retained `Reckoning::UpdateGroundStates` is now called with two full
world-up vectors and that unflipped Part6 Z column, preserving its unknown W.
The same numerical graph and live Processed reader used elsewhere execute its
native dynamic-up then heading writes before the fixed-heading body. Independent
IDA review found no duplicate graph or argument mismatch. The fresh frame259
replay in `artifacts/raw-bail-teleport-resetsystems-grounded-2026-09-24.log`
passes it and stops at the next `Wipeout::Reset` call. Native reciprocal seed
and rounding parity in the shared graph remain unmeasured.
`Wipeout::Reset` now writes only state zero and request count zero on the
retained owner; the ledger alias of that native count is synchronized without
clearing request flags or values. `TrajectorySelector::Reset` then writes the
eleven source-ordered onboard fields on the existing owner, preserving query
futures and targeting fields. Independent IDA reviews found no duplicate
owners or extra clears. The focused preservation tests pass, and the original
frame259 replay in
`artifacts/raw-bail-teleport-wipeout-selector-reset-2026-09-24.log` reaches
the next `PhysicalPlayerStateChanger::TeleportReset` call.
Evidence logs: `artifacts/raw-bail-reset-skater-animation-dispatch-2026-09-24.log`
`artifacts/raw-bail-reset-to-given-stance-2026-09-24.log`, and
`artifacts/raw-bail-teleport5-update-2026-09-24.log`, and
`artifacts/raw-bail-teleport-now-prefix-2026-09-24.log`, and
`artifacts/raw-bail-teleport-matrix-early-stores-2026-09-24.log`,
`artifacts/raw-bail-teleport-physout-reset-2026-09-24.log`, and
`artifacts/raw-bail-teleport-controller-reset-2026-09-24.log`,
`artifacts/raw-bail-teleport-board-init-transforms-2026-09-24.log`,
`artifacts/raw-bail-teleport-board-effective-flip-2026-09-24.log`,
`artifacts/raw-bail-teleport-board-body-set-2026-09-24.log`,
`artifacts/raw-bail-teleport-board-motion-reset-2026-09-24.log`,
`artifacts/raw-bail-teleport-collision-prefix-2026-09-24.log`,
`artifacts/raw-bail-teleport-collision-part-loop-2026-09-24.log`, and
`artifacts/raw-bail-teleport-body-reset-complete-2026-09-24.log`.

Confidence: high for the reached reset and state-update control flow and
retained owner identity; native per-tick animation/physics parity remains
unmeasured. The first live skating test still requires off-board and
teleport continuation plus ready physical Fill.

`PhysicalPlayerStateChanger::TeleportReset` now clears seven retained
counters and loads the live `Frames To Stay in state` cvar from the original
registration. A focused test confirms the countdown and preserved fields.
`OffBoardFeetIK::Reset` reuses the existing reset called by `ConsiderReset`;
`LandingOnDeckManager::Reset` clears only its current trajectory, timers,
flags, error and velocity in source order, preserving the ideal trajectory,
batch and bindings. Independent IDA reviews approved all three. The
frame-259 original-input replay
`artifacts/raw-bail-teleport-landing-reset-2026-09-24.log` passes these
calls and stops before `OffboardJumpingTest::Reset`.

The retained Jumping reset now checks its line batch first and, on the
reached null-batch branch, clears only `result_valid_counter`; an active
batch remains typed pending before the source `CalculateResults` call.
IDA and repository review found no active line-batch producer or duplicate
reset in the current path. ResetSystems then performs five inline stores
on the retained SkeletonController, followed by the existing
`PlayerGrabSpline::try_invalidate_query` on its retained query owner.
The next explicit boundary is `Skeleton::TeleportNow` after ResetSystems;
the fresh original-input frame-259 replay
`artifacts/raw-bail-teleport-resetsystems-complete-2026-09-24.log`
passes the last stores and stops exactly there. Independent IDA review
approved owner identity and partial-write behavior.

`Skeleton::TeleportNow` begins with the larger `Skeleton::TeleportReset`.
The live mount uses one admitted skeleton candidate and its retained
constructor children: deck-ZY zero, teleport-reset flag true, want-respawn
false, truck tilt true, existing Body reset, physics animation state zero,
HeadTracking reset, full retained IK reset, and the five wobble scalar
stores. It does not call the earlier constructor-only body reset again or
the test-only combined animation projection that would reset IK too early.
The pool invalidates cached collision publication before the Body mutation.
An independent IDA review approved source order and owner identity. The
original-input frame-259 replay
`artifacts/raw-bail-skeleton-teleport-reset-2026-09-24.log` passes the
full reset and stops before the inner `mTeleportingNow=true` store and
`UpdateGroundSkateboard` call.

`Skeleton::TeleportNow` now sets its retained teleporting flag, runs the
existing paired `UpdateGroundSkateboard` leaf once, and clears the flag
after that call succeeds. The outer `TeleportPlayerNow` then calls the
same retained SkateboardController's second source-required Reset and
stores `0x3ecccccd` to the retained Wipeout ground-invincibility timer.
Independent IDA review confirmed both calls, order, and partial writes;
the second Reset is a distinct callsite, not a duplicate implementation.
The original-input frame-259 replay
`artifacts/raw-bail-teleport-controller-second-2026-09-24.log` passes all
of this and stops before `PhysOutConditioners::ResetAll`.

`PhysOutConditioners::ResetAll` now visits the eleven retained children in
native order. The five empty virtual resets only validate their published
children; Grinds, Audio, Physics, Animation, Scoring, and GameCam mutate
their existing payloads. Grinds reads its linked cvar between its first
five stores and its existing reset suffix, preserving partial writes on
failure. Independent IDA review found the child order, values, and owner
reuse match the debug build. The frame-259 original-input replay
`artifacts/raw-bail-teleport-conditioners-resetall-2026-09-24.log`
passes the full reset and stops before `SetPhysicsState(Ground)`.

`SetPhysicsState(Ground)` now admits Teleporting as the predecessor of the
existing shared transition. It reads old type 702/category 700, enters
controller state 100, publishes state counters and Processed last/next
fields, skips the native empty Teleporting.Exit, uses the actual Actor's
player-ID/telemetry queries, and runs the existing full Ground.Enter on
the same pair. Independent IDA review found no duplicate Ground entry or
source-order mismatch. The frame-259 replay
`artifacts/raw-bail-teleport-setphysicsstate-2026-09-24.log` observes
physical state `Ground` and stops before the outer second
`Skateboard::SetStandard` call.

The outer source-required `Skateboard::SetStandard` then runs again on the
same board, followed by the existing `Skeleton::SetUpNormal` on the same
skeleton. The final optional network query selects the retained HiLOD
collision mode, publishing its flag before any board/Skeleton collision
change and returning early when it already matches. Independent IDA
reviews approved both pieces and the return to Update1's existing
controls continuation. The original-input 300-frame replay
`artifacts/raw-bail-teleport-return-2026-09-24.log` now completes with
no explicit boundary and observes the player back in physical Ground.
This verifies that selected route through recovery; native frame-by-frame
physics and animation parity remain unmeasured.
