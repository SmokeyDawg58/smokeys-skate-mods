# Motion 169/426 and SystemLogic OffBoard -> Air mount transition (2026-09-26)

**Status:** rustfmt parse PASS on every touched file; the new hunks are rustfmt-neutral. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

Playtest report (controlled skating test):
- `Tick(Actor(Motion(PendingExpression(169))))` after 54 frames
- `Tick(Actor(Motion(PendingExpression(426))))` after 1609 frames
- `Tick(Before(SystemLogic(Selection(PendingTransition(200)))))` after 309 frames

## 1. Motion PendingExpression(169)

- **What it is.** 169 is the state guard of `Cycle1143` (update 828f3470). Ancestry: Motion0 -> OnBoard3 -> InAir21 -> BoardAdjusts78 -> Up250 -> Grab686 -> Grabbing989 -> Cycle1143.
- **Pre (8290f224).** Cached `HasIntent(TailGrab)` (c264), returned directly.
- **Sustain (8291fb88).** The shared Sustain case for 169 and 326, reading the same c264.
- **Port.** Selection `169 => c(264)?`, Sustain `169 => condition(264)`.

## 2. Motion PendingExpression(426)

- **What it is.** 426 is the guard of transition 1054, DBLGrab `Cyc643` -> `WipeOut4` (priority 2).
- **Pre (8291b7f8).** `PhysicsWantsWipeOut` (c4) OR (`HasTweak(greaterEqual 0.8)` (c228) AND (`TrucksOrDeckInContact` (c227) OR NOT `PhysFilteredState(air)` (c85))).
- **Sustain.** The native default true (829221c8).
- **Check.** `tools/compare_graph_expressions.py` against the IDA-extracted native decision paths: motion 545 matched / 0 mismatches; action 171 matched / 0 mismatches.

## 3. SystemLogic PendingTransition(200): OffBoard (500) -> PhysicsAir (200)

**Which route.**
- `selected_transition` already admitted -> 200 from Ground, GroundAnimation, SlideGround, RevertGround, KnownAir and OffBoardAir.
- The OffBoardLocomotion (504) and LandingOnDeck (505) selectors never return 200. WipeoutGround (300) returns its current state.
- Only the OffBoard (500) selector can return 200, and `(OffBoard, 200)` was missing.

**Native selector: `CalcSuggestedState` case 500 (82ddcaf4).**
- Registers: r9 = Processed+8c4 (loaded at 82ddbf1c) and r6 = Processed.
- Bits follow the PDB `Sk8::Physics::ProcessedPhysIn` bitfields, allocated LSB-first.
- Priority order:

| Step | Test | Result |
|---|---|---|
| 1 | 8c4 `mIsWipingOut` 0x40000 | 300 |
| 2 | 8cc `mIsGrabbingObject` 0x800000 | 501 |
| 3 | 8cc `mOB_Com` 0x10000 | 504 (82ddc248) |
| 4 | 8d4 `mOB_JumpInfoReady` 0x20000000 AND (8cc `mOB_StandingJump` 0x100000 OR `mOB_Air` 0x100) | 503 |
| 5 | 8cc `mOB_Traj` 0x20000 | stay 500 |
| 6 | NOT 8d0 `mOB_BoardIsHeld` 0x40000 | 300 |
| 7 | 8d4 `mIsBoardMountInvalid` 0x800000 | 300 |
| 8 | otherwise | 200 (82ddcb7c) |

The port's `OffBoardStateInput::try_suggest_from_offboard` and its `select_offboard` adapter are identical: the same masks, order and lazy reads. `opaque_word_564_bit_18` is `mOB_BoardIsHeld`.

**Transition body: `SetPhysicsState` (82dff158).** It is generic, with no per-pair code. In order:
1. `EnterNewPhysicsState(200)` (82dd1d50): 500..504 calls StartController, anything else StopController (82dd1710).
2. StopController calls `LetGoOfSkateboard` (82dd03e8) while state+1bc != 0, then publishes state 0 and system off, then the hand assert. LetGo does: board animation off, SetFree, EnableCollisionVolumes, clear body+2098 bit 0x40, DisableHandDrive unless hand == 2, zero the velocity for state 4, and throw when 8cc `mOB_IsSkateboardBeingThrown` 0x2000 is set.
   - This is the same as the port's `try_stop_with` and `try_let_go_retained_controller`.
3. Transition fields, in port order.
4. `OffBoard.Exit` (82d86cc8): `OffboardLocationTest::ClearData`, then `~OffboardJumpingTest` (82dd5fd0), which only calls CalculateResults with a live batch at +50.
   - The port never creates a jumping batch, so its explicit pending guard stays inert.
5. The telemetry block (+174). Same as the port's `transition_actor_queries`.
6. `PhysicsAir.Enter` (82d98dd0).

**`PhysicsAir.Enter` with last = 500.**
- The last category is 500 (not 400) and last is not 701, so +AD useCOM = last not in {100, 103, 201} = true, and Skeleton+3ded = 1.
- The COM velocity is Processed+260, or `CalcVelFromJump` while `mFramesSinceJumpCorrect` < `kPhysicsAirJumpFixFrames`.
- +AE = 8d4 `mRunMount` (bit 26) and +AF = `mSprintMount` (bit 25). Either one selects `InitMountTrajectory`; otherwise the COM trajectory is used.
- The port's `try_enter_pair_air` branches on `last` exactly this way.
- A scan of every `PhysState_PhysicsAir` method finds +AE/+AF read only by Enter and InitMountTrajectory, which is what the port does too.

**`InitMountTrajectory` (82d8abb0), re-checked.**
- Attribute keys: run 0xbac43a7a1c428aaf / 0xf92bfcf1f71c9361; sprint 0x136fa41a2878e71e / 0x0e3c6cbce1494428.
- Launch velocity: Processed+e0 × z plus Processed+d0 × y. `mEffectiveAnimTransform` is at PDB +c0, so columns()[2]/[1] are +e0/+d0.
- Length: vmsum3fp dot, then rsqrte with two refinements.
- It restores Processed+260 only when the length is ordered-greater than `kPhysicsAirMountLaunchTrajMaxChange`.
- Stores go to +50/+60/+70/+80.

**Input producers, verified by native writer.**
- `mOB_BoardIsHeld`:
  - Update1 82e03008 copies it from `PhysOut_OffBoard`+3f `mIsHoldingSkateboard`.
  - That byte's only writer, besides reset, is `SkateboardController::FillPhysOut` 82dc83d0: controller state+1bc == 1.
  - Port: `try_fill_controller` `state == 1` and prior-output 8d0 0x40000.
- `mIsBoardMountInvalid`:
  - Update1 82e0312c copies it from `PhysOut_OffBoard`+40.
  - It is written by Update5 82e058b0..5a0c when 8d0 `mIsMounting` 0x200000 AND `mOB_BoardIsHeld`, AND (the anim-part-0 vs board position distance² > linear², OR |angle| of the Z axes > angular°, OR |angle| of the X axes > angular°). The air thresholds apply when current == 502.
  - Port: `try_fill_post_category` plus the `board_mount_invalid` source.
- `mRunMount`/`mSprintMount` come from the RunMount/SprintMount anim attributes (8d4 0x04000000 / 0x02000000).

**Fix.** `(CurrentPhysicalState::OffBoard, 200) => Some(GroundDestination::Air)` in `selected_transition`, plus a unit-test assertion.

**Gameplay.** This is the run/sprint mount back onto the board: off board, board held, valid mount, then Air with the mount launch trajectory. It is the physics side of the previous batch's Action 32 (TransitioningToOnBoard11 -> OnBoard6).

**Unchanged.** The OffBoardPushing (501) and OffBoardJump (503) destinations remain unported; `GroundDestination` has no variant for them, so they still report pending.
