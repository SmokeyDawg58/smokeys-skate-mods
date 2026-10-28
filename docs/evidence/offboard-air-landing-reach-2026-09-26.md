# Off-board air landing reach: OBAir stride selection and dismount active sets (2026-09-26)

**Status:** rustfmt parse PASS on every touched hunk. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Report

After an ollie, a grab and an air dismount, native pulls the feet back down to reach for the ground as the skater approaches it. The port lands in the air pose and then snaps to idle.

## How the reach works in native

- **Grab to stride.** Grab `ToOffBoard*` plays `OFB_DISMOUNT_AIR_*`. Its WillExpire transition targets `OBAir12`, whose children are `Dismount36`, `RightFootStride37` (Pre 433) and `LeftFootStride38` (Pre 423).
- **What the stride runs.** Each stride plays `AIR_RF`/`AIR_LF` and runs `LandingOffBoardAir` (Impact / CloseToLand) and `ControlAirLegExtension` (drives `disttocog` and `extend`).
- **The trees.** `air_rf` is a PhaseBlend on `disttocog` between two sides:
  - `ofb_jump_high_jrf_cyc`: tucked, disttocog 0.539.
  - `air_rf_impact`: extended. It is a PhaseBlend on `impact` over `ofb_jump_low_1ft_rf_cyc` (0.945) and `air_rf_2f`, which blends on `closetoland` into `ofb_jump_low_2ft_cyc`/`_mini_cyc` (1.005).
  - `air_lf` is the same with the lf clips.
  - The "reach for the ground" is `ControlAirLegExtension` PreLand (82ca1ab0) raising `disttocog` from tucked toward extended once `IsTimeToPrepareLand` (825f4b60) passes. That check is `OffBoard.trajectoryReady && timeToLand < kOffBoardAirPrelandTime` when neither CanLandOnBoard nor the on-board air trajectory is valid.
  - `LandingOffBoardAir` (82ca6340) adds the two-foot / close-to-land blend.
- **Landing.** The stride's transition to `RunAround107` fires on `OBTimeToLand <= 0.03` (expr 34). The stride Sustain is only `PhysFilteredState(offboardair) && HasIntent(OffBoardAir)`. So if no stride state is active, touchdown drops the graph to the ground locomotion states with no reach and no landing blend.

## Bug 1 (fixed): Pre expression 423 was XOR; native is "no LocoStartingFoot"

- **Native Pre case 423** (8291b2cc):
  1. PhysFilteredState(offboardair), then HasIntent(OffBoardAir).
  2. IsMirrored. If set: LocoStartingFoot false -> return 1 (8291b4f4), otherwise fall to 8291b43c.
  3. At 8291b43c, IsMirrored again: set -> return 0 (82910254). Clear -> LocoStartingFoot false -> return 1, otherwise return 0.
  - Result: `air && intent && !LocoStartingFoot` for both mirror states.
- **Port.** `air && intent && (IsMirrored XOR LocoStartingFoot)`. With IsMirrored clear and LocoStartingFoot absent, native selects `LeftFootStride38` and the port selected neither stride.
- **Which dismounts hit it.** Dismount clips in OffBoard.abin:
  - `ofb_dismount_air_bs_to_jlf` has no `locostartingfoot` and no mirrored event.
  - `ofb_dismount_air_stale_to_jlfmir` has no `locostartingfoot`.
  - fs/dbl/mute and `onb_air_to_ofb_air` carry static `locostartingfoot=1`, so they take 433 (verified identical, including the OB_Mount/CurrentState tail at 8291c5b0).
  - A non-mirrored BS-grab air dismount therefore never entered a stride state in the port. The graph kept the dismount clip's final (air) pose until the physical landing broke the air sustain, then fell to OBGround locomotion. That matches the report.
- **Fix.** `generated_motion_runtime_selection.rs` 423 -> `(IsMirrored && !LSF) || (!IsMirrored && !LSF)`. Condition read order matches native in all four mirror/LSF cases. The test now covers the full truth table and read order.
- **Sustain 423/433** (82921b24 / 82921cb8) = air && intent, already identical.

## Bug 2 (fixed): three grab dismount states had no active set

- `ToOffBoard646` (DBL), `ToOffBoard675` (Mute) and `ToOffBoard682` (Stale) were missing from `update_behaviours`, so entering them returned `PendingStateUpdate`.
- **Native.**
  - 828f11d0 is `StateUpdate_ToOffBoard_646`: Lateflip_80, then packed23 ScoringGrabs, then packed24 PlayAnimation.
  - 828f1750 is `StateUpdate_ToOffBoard_682`, shared by Mute675: Lateflip_80, then packed23 PlayAnimation.
- **Added:** (646, 828f11d0) -> [.., 159, 641, 1257]; (675, 828f1750) -> [.., 159, 1335]; (682, 828f1750) -> [.., 159, 1351].

## Verified native-identical this pass (do not re-audit)

- **`PhysState_OffBoard_Air::Update` 82d94158 / `FillPhysOut` 82d94510:** FeetIK.UpdateAir, height adjust, GetTrajectoryResultAtFrame, HasLanded, spin, wipeout checks, land-on-board query, and the PhysOut OffBoard fields (timeToLand +1c, landingVelocity, trajectoryReady, hasLanded).
- **`OffBoardFeetIK::UpdateAir` 82dd1e50:** the five resets, InitUpdate 82dcd260, Update 82dd1b70 (CalculateFeetDownState / UpdateState / CalculateIKTargets / ApplyIK), and CalculateHeightAboveGroundAir 82dd05d8 (x/z columns, slope adjust, fsel min, default 10). UpdateAir does not run ConditionLineTestResultsForGround; only UpdateGround 82dd1e00 does, and the port matches.
- **`StartSkeletonLineTests` 82dfe850:** toe start +0.33, direction (0, -1.5-0.33, 0), duration 1, zero gravity, kFeetLineTestRadius.
- **`OffboardTrajectorySelector::GetTrajectoryResultAtFrame` 82dc3280:** the height>0 re-aim; the raw trajectory while the query is pending; the blend graph; the plane `(com.x, com.y+h, com.z)` with normal +Y and `VECTOR3_GRAVITY`, time T=-1; time 0 unless h<0.
- **`ControlAirLegExtension`:**
  - ctor 82cb0730: `driveDistToCom` defaults to "disttocog"; the data's `driveDistToCog` key is ignored in both.
  - PreLand 82ca1ab0: dt*vel, then the per-frame toe-bone multiplier.
  - IsTimeToPrepareToLand 82cabab8.
- **`SkaterMotionGraph::IsTimeToPrepareLand` 825f4b60:** CanLandOnBoard, the air-trajectory-valid branch, then the offboard preland time.
- **`PhysOut::Reset` 82e2f740:** runs at the top of every Update5 82e04ea8, so the Air block's `mIsTrajValid` is 0 off-board. The port resets in `try_begin_update5_output`.

## Not yet ported (seen while auditing, out of this fix's path)

Off-board ground leaves with no active set: `Idle105` children 308..312, `StandingJump106`, `MovingObject*`, `DismountFromFakie100`, `SpawnDismount31` and `InAirMount35`. They are reachable only when `LocomotionValid` fails (Idle105 = !445). Standing off-board normally stays in `RunAround107/LocoNew319`.

## Trace for the next test

`play-offboard-air-trace.bat` sets `SKATE2_TRACE_OFFBOARD_AIR` and omits `--quiet`, writing `offboard-air-trace.txt`. It prints:
- `offboard-air motion:` every Motion state change, with names;
- `offboard-air physics:` frame, height, time_to_land, landing_valid, COM y/vy, landed, surface validity and animation toe y;
- `offboard-air landing:` impact and close_to_land;
- `offboard-air leg:` phase, disttocog distance and down_time.

A correct run shows `...ToOffBoard* -> OBAir12/Left|RightFootStride`, leg phase 3 (preland) with the distance rising, then `-> RunAround107/LocoNew319` near time_to_land 0.03.
