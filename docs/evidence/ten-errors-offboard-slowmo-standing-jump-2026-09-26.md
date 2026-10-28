# Ten reported errors + off-board / slow-motion / standing-jump forward closure (2026-09-26)

**Status:** rustfmt parse PASS on every touched hunk. NOT compiled/tested. SOLO; IDA on the Jan-17-2009 debug `sk82_na_m.xex`.

## The ten reported errors

| Error | Native meaning | Fix (anchors) |
|---|---|---|
| OffBoardAir `PendingCollisionMode(5)` | `SCS_OFFBOARD_AIR` | `EnterState` 82dd488c → `Skeleton::SetOffBoardAirCollision` 82b4efd8. It is `SetAirCollision` 82b4eea0 with Body `SetUpRagdollCollision` 82b47870 in place of `SetUpFullCollision` 82b47780. See note 1. |
| Motion Pre 33 | Mount34 → OnBoard3 | c71 `WillExpire(0.3)` (8290bda8) |
| OffBoardEntry `PendingAirWipeoutTriggers` | `CheckForWipeoutTriggers` 82d91858 | See note 2. |
| Motion callback 121 @ 82cb4730 | `LandingOffBoardAir` | Create 82cb4730, Begin 82cb4610, Update 82ca6340, plus 7 OBLanding cvars (540..546). Previous part of this batch. |
| Motion Pre 2 | Takeoff and mount-jump exits | c8 `WillExpire(0.0)` |
| Action 78 | varial / finger-flip shuv | c53 ∨ c52 ∨ c54 (829033d0 / 82908584) |
| Motion 137 | DBL-grab release guard | ¬c224, `cntlzw` negation on both paths (8290e520) |
| Action 70 | mirror gate | c113 ∨ ¬c17 (8290290c / 82907e8c) |
| `PendingStateUpdate(0x828EF840)` | `StateUpdate_Start_115` | allowlists 115/116/117; Pre 40 / 284 |
| Camera `SlowMotionController` | Skate 2 big-air slow motion | See the slow-motion section below. |

Note 1, collision mode 5: `SetUpRagdollCollision` writes group and Part userdata 7, then enables Parts 1..23. It has no root prefix. The suffix is proxy off, root disabled and count[0] = 0.

Note 2, air wipeout triggers: this runs only when the last state is 502.
- It reads physics_wipeout/default `0xF784AC3BA4422FFD`, then takes `|Processed+0x260|`. The length is two-refinement and zero when the square is zero, so it is `try_length`. The result is +0x74 = `speed > limit`.
- +0x73 = `kOB_MinUpVectorY > EffectiveAnimTransform.yAxis.y` (Processed+0xD0).
- It adds the new cvar 547 `kOB_MinUpVectorY` (Physics_States_OffBoard, "Min Up Vect Y", 0x3f666666, table 82fd59bc).
- IDA misdecodes this VMX128 VA operand by +32 (for example v95 is really v63), so I decoded it from the raw words.

## Slow motion (SlowMotionController camera leaf)

- **Behaviour.** Data is 0x18 bytes: `mTimeInSlowMotion` +8, `mFirstLookAtTimeInAir` +0xC (starts at 1.0), `mTimeBeforeInAir` +0x10 and `mWasInAir` +0x14.
  - Begin 82ca8e20 resets the flag, +8 and +0x10, then evaluates the graph at t = 0.
  - Update 82ca8f48 needs the Actor's IPhysical. It reads PhysOut_Air +0xA8 `mTrajCollisionTime` using an ordered `> 0` test (`ble`).
  - The time fed into the graph is `elapsed / (before + firstLook)`, or 0.1 while on the ground.
  - sim time = `1 / fmadds(fps_at_scale_one − 60, timescale(t)·kSlowMotionScale, 60)`.
  - End 82ca9170 posts `kUpdateTime` with ticks = 0.
  - The layout is `timescale` PointGraphData16 plus `fps_at_scale_one` at +0x80. The new cvar 548 `kSlowMotionScale` is "Animation_Slowdown", default 1.0, range 0..2.
- **Queue.** `SendMsg` 82655840 only enqueues onto eEQGameSim (queue 4). `SimController::Update1` Dispatch 828a739c then runs `Simulation::OnRequestSetSimRate` 8289e350 before commands.
- **Handler.** Offline, the rate is `msg.fSimTime`. An ordered-equal `kUpdateTime` clears the countdown, and ticks > 0 gives ticks + 1. The `kMaxTicksForSlowMotion` branch is pending (no sender reaches it). The Hz is `fctiwz(1/rate + 0.5)` and goes to `SetTimerHz` 82985ec0.
- **Countdown.** `SimUpdate2` 8289fd40 runs after `SkaterManager::PostUpdate` when updateMask bit 0 is set. On reaching zero it sets `kUpdateTime` and 60 Hz.
- **What actually changes.** Per-tick physics does not change. Only the ExecutionMan timer that wakes the loop changes (timerThread 8298a248). The app now paces with `timer_period(hz)` = 10,000,000 / Hz × 100 ns, and 60 Hz keeps the host `FIXED_STEP`. As native, the period re-arms from the value current at each fire.
- **Camera entity.** The camera graph entity is GameCameraManager+0xD0, the pre-change local skater (CreateCameraManInstances 82e3a6a8). The port already mirrors this. A null entity posts nothing in Update, exactly as native does.

## Collision mode 4 (`SetOffBoardCollision` 82b4f038)

- The sequence is group/userdata 7, then `SetUpNoCollision(false)`.
- `kSkeletonOB_UseLightDMOGroup` covers Parts 17,21,15,19,16,20 with userdata 0x11 (82b46328).
- `kSkeletonOB_HaveBodyCollision` covers Parts 5,4,3,9,8,7,2,1 with `SetUpNormalCollision(Part)` userdata 6 (82b47940).
- `kSkeletonOB_CoreCollideWithLW` covers Parts 6,10,11,12,13,14,23 with LW 0x12 (82b463b8), or LightDMO when the cvar is off.
- The suffix is proxy on, a conditional root enable, then root userdata 6.
- New cvars 549–551 (Physics_Skeleton, all default true, table 82fd45f0..45f8).
- The owner's live cvars are bound for each transition (`OwnerBoundCollision`).
- The corpus assertion changed from `Pending(4)` to the concrete state. It has not been run.

## Standing jump, land-on-board and other forward closure

- **Guards.** Pre 26 = c57, Pre 27 = c58 `InStateForTime(Descent < 0.3)`, Pre 28 = c59, Pre 29 = c26, Pre 30 = ¬c60, Pre 34 = c83. Pre 366 = c400 ∧ c401. Pre 155 = c247, Pre 160 = c252.
- **Pre 380.** ¬(c88 ∨ c59 ∨ c89) ∨ (c86 ∧ (c90 ∨ c91)). This needed the new `InTimeWindow` 82c9bdd0 and `IsTransitioning` 82c9bf08 conditions.
- **Sustain.** 423/433 = c76 ∧ c77. 155\|309 = c247, 160\|313 = c252, 267\|366 = c400. Everything else in this batch is the native default.
- **Update allowlists** (each thunk expanded in IDA):
  - Standing jump: 314, 315, 317, 318.
  - Running jump into mount: 9, 10, 11.
  - Land on board: 95, 96, 97.
  - DBL release: 644, 656, 667, 265.
  - Running, sprinting and stall mounts: 108..114.
- **New behaviours.**
  - `StandingJumpLandingCOG` Begin 82cb44b8 writes `mComHeightOnLanding` as a settable attribute (false, −1).
  - `StandingJumpDescent`: Begin sets a byte flag. Update 82ca6148 sets the base animation speed to `fsel`-clamp((GetLength − GetTime) / mTimeToLand, 0.2, 1) when ttl > 0.
  - `LandOnBoard` Begin 82ca5fa8 calls `FlipBoardBackwards` (ISkaterAnim +0x24, 82cb75a8) when `mLandOnSkateboardFlipBoard` is set. That toggles `mIsAnimBoardBackwards`, bit 0 in MSB numbering of SkaterAnim+0x612C. The ISkaterAnim base sits at +0x52E0, and IDA types `this` as the full SkaterAnim.

## Frontier (not in this batch)

- Grind leaves under Grinding18 (Pre 44..56, 67/68, 382..385; allowlists 40..56, 321.., 428..).
- Off-board step states 309..312 (Pre 280/281/282/24) and the moving-object subsystem (MVOBJ 307, Pre 21/10/12/13/14, MovingObject104 Pre 278).
- Late flips 253..258 out of InAir.
- `kMaxTicksForSlowMotion`.
- REGISTRATION_ORDER still places defs 308..313 (kShowSkaterInAirDebugText / kLegAdjust*) after 233. Natively they belong after kKickTurnSmoothing (table 82fd4ee8..4efc).
