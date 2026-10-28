# First selected OffBoard entry

Confidence: exact for selector flags, `CheckForStateChange` and `SetPhysicsState`
ordering, the selected Ground/GroundAnimation to OffBoard state transition, and
the initial OffBoard `Enter` calls. The composed Y-to-OffBoard latency and the
next OffBoard update are not validated against a deterministic reference trace.

IDA `PhysicalPlayerStateChanger::CalcSuggestedState` `0x82ddbd58` tests the
processed animation flag at word 563 (`+0x8cc`) mask `0x20000`. Ground100 returns
OffBoard500 at `0x82ddc238`, after handplant. GroundAnimation103 returns 500 at
`0x82ddc3a0`, after wipeout and boneless. `Skeleton::ProcessAnimAttributes`
`0x82b59de8` sets that bit for the authored `OB_Traj` attribute. It is not a
direct mapping from Y input: `action_graph_input_fill.rs` produces
`NewToggleOffBoardState`/`ToggleOffBoardState` from face Y, and the loaded
Action/Motion animation must publish `OB_Traj` before the physical selection.

`PhysicalPlayerHiLOD::Update2_SystemLogic` `0x82e01c40` calls
`CheckForStateChange` at `0x82e01c54`, then clears Wipeout requests at
`0x82e01c5c`. `CheckForStateChange` starts at `0x82e005a8`: old GetType
`0x82e005d4`, CalcSuggestedState `0x82e005e4`, comparison `0x82e005ec`,
framesSinceJump=100 `0x82e005f8`, SetPhysicsState `0x82e005fc`.

`SetPhysicsState` `0x82dff158` first calls SkateboardController.EnterNewPhysicsState
at `0x82dff1b0`. It zeros state count, HiLOD timer and processed timer at
`0x82dff1c0..1cc`; records processed old state, old category, next and current
at `0x82dff1e8..204`; calls old Exit at `0x82dff218`; performs Actor ID and
telemetry reads; chooses retained mPhysState_OffBoard at `0x82dff928`; then calls
new Enter at `0x82dffd1c`. The Rust runner reuses this same transition sequence
and source-selected Ground/GroundAnimation exits.

`PhysState_OffBoard::Enter` `0x82d9c1d8` resets the five entry booleans and
vectors, grab descriptor, blend and auto turn (`0x82d9c230..260`); calls
`OffBoardFeetIK::ConsiderReset` (`0x82d9c268`), collision state 4
(`0x82d9c274`), GrabSpline query and interactable invalidations
(`0x82d9c27c/284`); resets OffboardJumpingTest.mResult position and two flags
(`0x82d9c2a4..b0`). `ConsiderReset` `0x82dcd110` resets foot IK and disables
Skeleton IK unless last state is 500..502 or 504, then sets foot locking active
if current state is 501. PDB `JumpingTestResult` offsets 0/16/17 confirm the
three result fields; the other JumpingTest fields are preserved.

For last state other than 504, Enter reads processed COM velocity at `+0x260`,
then the live `kOB_MaxLandingSpeedForBlend` reference at `0x82d9c2d8`, then
calls `PhysicsUtility::ClampVectorWithinMaxLength` at `0x82d9c2e0`. The cvar's
initializer `0x82f92af8`, static table `0x82fd5998`, and target ref use prove
section `Physics_States_OffBoard`, title `Max Landing Speed Blend`, default 10,
range 0..20 and a null callback. Runtime getter uses the linked target and
startup overlay. `CheckForWipeoutTriggers` at `0x82d9c2f0` only acts when last
state was OffBoardAir502; Ground and GroundAnimation entry return after the
clamped velocity store.

On the mounted raw-Y frame-59 path, that full-vector store initially stopped
at a host `Processed COM velocity W` check. IDA `OffBoard::Enter` `0x82d9c2e0..e4`
loads the complete processed `+0x260` vector and writes the complete clamp
result to `mStartingComVel`; `ClampVectorWithinMaxLength` `0x82e49430` uses
dot3 for length but multiplies all four input lanes in its selected output.
The retained `SkeletonPostPhysicsRecords::centre_of_mass_velocity_typed()`
can mark W unknown when history COM W came from attached Part pointer lanes;
`Update1` faithfully copies that knowledge through PhysOut to processed input.
No source-backed scalar can replace it. `OffBoard.starting_com_vel` now retains
the existing `SkeletonRootVector` with known XYZ and optional W. Constructor and
entry zero stores remain known four-lane zero. This removes the artificial
full-lane gate without treating host scratch as native W; later consumers must
respect that optional lane. Combined build and replay are parent-owned.

The selected runner can now finish Ground/GroundAnimation to OffBoard `Enter`
when its actual collaborators and COM vector are available. It then performs
Update3 prediction. The next native call in `PhysState_OffBoard::Update`
`0x82da03a8` is `OffBoardFeetIK::UpdateGround` at `0x82da03c8`. That producer
is not mounted, so the selected pre-solve returns an explicit pending error at
that boundary before any Ground state behavior. A hands-on Y dismount is not
claimed complete. Neither current-frame OffBoard.Update nor later post-solve,
FillPhysOut, camera, and presentation are validated in this packet.

Validation: `cargo check -p skate2-core --lib` passed. The focused selected
transition, linked cvar, and source title-order tests each passed. The broader
input cvar suite initially had 32 passes and one failed fixed-list expectation;
the corrected expectation passed on rerun. The original-data corpus and full
workspace gate are owned by the root integrator.
