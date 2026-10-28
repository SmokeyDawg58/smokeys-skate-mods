# Marker teleport 702 and the offboard standing jump: OffboardJumpingTest + PhysState_OffBoard_Jump 503 (2026-09-27)

**Status:** rustfmt parse PASS. Every new hunk is rustfmt-neutral, the two new state files are fully formatted, and pre-existing diffs were left alone. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`. Raw VX128 operands were re-decoded, because IDA mis-prints VA.

Playtest reports (controlled skating test):
- Going to a marker: `Tick(Before(SystemLogic(Selection(PendingTransition(702)))))` after 112 frames.
- Jumping while fetching the board: `Tick(Before(OffBoardFeetIk(PendingOffboardPostOrientation { next: UpdateJumpingPreperation, .. })))` after 3380 frames.

## 1. Marker: 702 from every onboard/offboard state

`CalcSuggestedState` 82ddbf50 runs a shared prefix before its per-state switch: Processed 8c4 bit 4 or 8c8 0x80000 returns 702 for every current state.
- The port admitted 702 only from WipeoutGround, RevertGround, SlideGround, KnownAir, Boneless and FootPlant.
- A marker used while in Ground/GroundAnimation/Air/OffBoard/OffBoardAir/OffBoardLocomotion/LandingOnDeck had no admitted pair, which gave `PendingTransition(702)`.

None of those Exits reads the destination: PhysicsGround 82d84a80, GroundAnimation 82d83ed0, PhysicsAir 82d84190, OffBoard 82d86cc8, OffBoard_Air 82d88318, LandingOnDeck 82d897f8, and the Locomotion BLR 825187c0. So the pairs are admitted to `GroundDestination::Teleporting` (plus OffBoardJump, section 3).

OffBoard.Exit's `~OffboardJumpingTest` 82dd5fd0 no longer refuses a pending batch. A pending line batch now completes through `CalculateResults` 82dd5660, exactly as native, and a null batch returns immediately.

## 2. OffboardJumpingTest (the pending "UpdateJumpingPreperation")

**Chain.**
1. OffBoard(500).Update ends with `UpdateJumpingPreperation` 82d86d00, reached when the grab gate is skipped.
2. It returns unless Processed 8d0 bit 0x2 (JumpAboutToTakeOff, from the Motion StandingJump AttachIntent) is set.
3. The input comes from the effective animation transform (Processed+c0 X, +e0 Z, +f0 translation) and OB_JumpX/OB_JumpZ (+a14/+a18):
   `v = (Z*jumpZ - X*jumpX) * kOB_MaxJumpDistance`.
   - Ordered length > 0.1: normalize, and distance = max(length, kOB_MinJumpDistance) (fcmpu/bge).
   - Otherwise: direction = Z and distance = kOB_MinJumpDistance.
4. `StartQuery` 82dd5078 clears the counter (+0x60) and asserts the three batches are null. It stores the input, then acquires a line batch labelled "SkateboardBody::StartWheelLineTest" (native reuses that label; IWorld vtable+0x30, BF_Default 7).
   - Five fat lines (radius r = kOBJT_TestRadius), each from its start along dir*kOBJT_MaxJumpDist. They start at H-r, H-3r, H+r and H+3r above the ground point (H = kOBJT_MaxJumpHeight), and at -r just below it (82dd5240..53dc).
   - Two straight sweeps (IWorld vtable+0x3c; zero acceleration, radius r, duration 1.0, zero errors). The first starts at ground + dir*dist + (0, 2(H+r)): native adds (0,H+r) twice (82dd5480/5484). It moves down by kOBJT_MaxJumpDown + H. The second is the same sweep moved back by dir*-2r.
5. The next `Update1`: `Tick` 82dd5f98 does a signed decrement clamped at 0, then `CalculateResults` if a line batch is live.
6. `CalculateResults` syncs, reads and releases all batches, then applies the wall / top-of-wall / ground-normal bands.
   - Band edges: `fcmpu/ble`, then `bge`; only ordered values strictly inside a band move.
   - Selection uses `fsel` clamps. Landing = fma(dir, dist, ground) + (0, hy, 0, 0), the leap-of-faith byte is stored, and counter = 30.
7. Update1 publishes 8d4 0x20000000 (OffboardJumpInfoReady) as `counter > 0`.
8. `CalcSuggestedState` 82ddcb7c (OffBoard): JumpInfoReady and (8cc 0x100000 standing jump or 8cc 0x100 OffBoardAir) returns 503.

`Reset` 82dda070 (TeleportPlayerNow's ResetSystems) completes a pending batch through CalculateResults, then clears the counter. With no batch it only clears the counter.

**Cvars 597..609** (source defaults; initializer, table and reference are in the Definition comments):

| # | Name | Default |
|---|---|---|
| 597 | kOB_MaxJumpDistance | 1.0 |
| 598 | kOB_MinJumpDistance | 0.75 |
| 599 | kOBJT_TestRadius | 0.165 |
| 600 | kOBJT_MaxJumpHeight | 1.0 |
| 601 | kOBJT_MaxJumpDist | 2.0 |
| 602 | kOBJT_MaxJumpDown | 5.0 |
| 603 | kOBJT_MinDistFromWall | 0.3 |
| 604 | kOBJT_MinDistFromTopOfWall | 0.1 |
| 605 | kOBJT_MinGroundNormalY | 0.71 |
| 606 | kOBJT_DebugDraw | false |
| 607 | kOBJT_DebugTestBadTraj | false |
| 608 | kOffboardJumpMaxLandOnBoardAssist | 0.5 |
| 609 | kOffboardStandingJumpMaxLandOnBoardAssist | 1.75 |

The stale cvar-title test is now 66 behind (pre-existing, not changed here).

## 3. PhysState_OffBoard_Jump (503)

**Vtable 8234a60c:**
- Update 82d98960, Enter 82d9f3f8, Exit = shared BLR 825187c0.
- PredictFutureOfDeck = base 82d83ae8.
- GetType 82d98590 = `li r3,0x1F6` (502), the same function OffBoard_Air uses.
- FillPhysOut 82d89a88, UpdatePostPhysics 82d899f8, +0x24/+0x28 = BLR.

**The folded GetType drives the whole integration:**
- `SetPhysicsState` 82dff158: `last` = GetType and its category (82dff1e0..1f4), so leaving the jump reports last 502 / category 500. next/current = 503 (82dff1fc/204), and the 503 object is this+0x294.
- `CheckForStateChange` 82e005a8 passes GetType into CalcSuggestedState, so the OffBoard_Air case selects while jumping. A 502 suggestion equals GetType and keeps the jump (SetPhysicsState also asserts on a same-type request). The 503 switch case is dead.
- Update5 82e053f0 publishes GetType (502) to PhysOut state+0x10. The next Update1 copies it into Processed+8e4 **and** +8e8 (82e02460/68), so from the jump's second frame FeetIK UpdateState's `current == 502` test is true, exactly as native.
- Update5 82e05900 compares its GetType read (r27, 82e055f8) with 0x1F6: the jump uses the air board-mount thresholds.

**Enter 82d9f3f8:**
- zero +0x80..+0x9c (body spin, adjusted spin, transition/current angle, air time, total air time, time to land, landing COM height) and bytes +0xa0/+0xa1;
- trajectory {0, 0, 0, time = splat(-1)};
- `OffBoardFeetIK::ConsiderReset` (last 500 so no reset; foot locking = current==501);
- CalculateTrajectory;
- `SetSkeletonCollisionState(4)`.

**CalculateTrajectory 82d985a8** (raw operands):
1. Assert counter > 0. leap = test+0x41.
2. landing = test result if +0x40 is valid, else Processed+f0. delta = landing.y - Processed+f0.y.
3. COM height = 8-point graph key 0x0dbba2ea14abe028, with `DefaultDataArea(0x50)` when absent. landing_com = landing + (0, com_h, 0, 0).
4. apex = 4-point graph key 0xec7dd184f869b215 (`DefaultDataArea(0x30)`); assert apex >= 0.
5. launch = sqrt(apex * 19.6).
6. disc = fmsubs(launch, launch, (-(landing_com.y - anim_com.y)) * -19.6), then fsel-clamped at 0, root = sqrt(disc).
7. t = -0.10204081 * (-launch - root), then fsel(-t, 0, t).
8. v = (landing_com - anim_com) * refine_recip(t) - (g*0.5)*t.

Both refinements are two fused Newton steps (checked against `refine_drive_reciprocal[_sqrt]`). Stores: position = anim COM, gravity = VECTOR3_GRAVITY, time = -1, air = 0, ttl = total.

**Update 82d98960:**
1. FeetIK.UpdateAir; air += 1/60; ttl = total - air.
2. If FeetIK height < 0 (ordered) and 0 > Processed+260.y (COM velocity), ttl = 0.
3. 8d0 0x800: RequestDropBoard.
4. com = p + v*t + (g*(1*0.5))*t^2 (fused); has_landed = !(ttl > 0).
5. Land-on-board query when any of these holds:
   - SMALL > |Z*jumpZ - X*jumpX|^2;
   - dot(normalize_safe(Processed+b0 - Processed+f0), input) >= 0;
   - dot(that offset, normalize_safe(horizontal trajectory velocity)) >= 0.
6. The query uses COM Processed+250, velocity = g*t + v, and assist = kOffboardStandingJump... (609) when air < 0.06 (ordered), else kOffboardJump... (608).
7. `Skeleton::UpdateOffBoardAir(com, +0x84 turn, true)`.

**UpdatePostPhysics 82d899f8:** CheckForOffBoardAirWipeout, LandingOnDeckManager.UpdatePostPhysics, then leap && ttl < 0.08 (ordered) && !(8d0 0x200000) gives inline Wipeout request 26 (OffBoardDeathLeap): flag +0x2e, value +0x9c = 0, count +0xb4.

**FillPhysOut 82d89a88:** LandingOnDeck fill, then OffBoard ttl (+0x1c), landing COM height (+0x28), has_landed (+0x45), then OffBoardFeetIK.FillPhysOut.

**Transitions admitted:**
- (OffBoard, 503) -> OffBoardJump.
- (OffBoardJump, 300/504/500/505/200/702) -> Wipeout / OffBoardLocomotion / OffBoard / LandingOnDeck / Air / Teleporting. These are the OffBoard_Air selector's outputs; 502 stays.
- Previous id/category (502, 500); EnterNewPhysicsState starts the offboard controller for 503.

**Dispatch sites:**
- ground_to_air: destination, Exit BLR, selected, Enter.
- system_logic: current 502.
- update3_prefix: bindings.
- update3_controller_session: allowlist.
- ground_pre_solve: Update then the shared offboard controller/prephysics continuation.
- post_solve_conditioning: UpdatePostPhysics.
- selected_ground_conditioning: FillPhysOut.
- update5_state_publication: type 502.
- update5_post_category: air mount thresholds.

## 4. Remaining explicit boundaries (unchanged, not hidden)

- OffBoard.Update's grab gate (8cc byte bit 0x01000000) still returns `PendingOffboardPostOrientation { next: CheckForGrabbableObject }`. Native calls UpdateJumpingPreperation after that check, so jumping while an interactable/skitch object is in reach stays pending.
- kOBJT_DebugDraw = true returns `PendingDebugDraw` (DebugOverlay singleton not owned).
- OffBoardPushing 501 remains unported. (GroundAnimation, 300) is still not admitted.

## 5. Tests added (not run)

- ground_to_air `offboard_jump_enters_from_offboard_and_leaves_through_the_folded_air_case`, plus the 702 loop.
- `physical_player_offboard_jumping_tests.rs` (band window and fsel/lanes helpers).

## 6. What to try

1. Use a marker from Ground/Air/OffBoard and walking; each should respawn instead of stopping.
2. Get off the board, walk, then press jump while pressing the board fetch. The skater should do the standing jump (short hop with the ballistic COM arc, feet IK in air) and land to walking, on the board (LandingOnDeck) or into Air/Wipeout, as native.
3. A jump off a ledge with no landing found is a "leap of faith" and requests the OffBoardDeathLeap wipeout just before landing.

## Follow-up (playtest): Action Teleport24 update

- **Report:** using a marker while riding failed with `Tick(Actor(Action(Update(PendingStateUpdate(2190398432)))))`. 2190398432 is 0x828ed3e0, `StateUpdate_Teleport_24`.
- **Native:** Teleport24 (parent OnBoard6, guard expr 28 = c127, Sustain 28 native true) owns no behaviours. Its update is a single `b StateUpdate_OnBoard_6` 828ecfe0, which does:
  - `StateUpdate_Action_0` (packed 0..10);
  - direct Updates of packed 11..18 (8-byte stride from +0x58): CreateMGIntentFromAGIntent x3, BodyFlippingSignal, CreateMGIntentFromAGIntent x2, CreateMGTimeIntentFromAGIntent, CreateMGIntentFromAGIntent.
  - These are behaviours 42..49.
- **Port:** `additional_shared_update_slots` admits `24 => (0x828ed3e0, [0..10, 42..49])`. The alias test checks the packed classes.
- The Motion side (Teleport5: expr 271 = c402, Sustain 271, update 828ee130, exit expr 367) was already admitted; bail respawns use it. Only an on-board teleport (marker) reaches Action Teleport24.
