# Nine reported gaps + Air COM entry (2026-09-26)

Status: source written; rustfmt parse PASS on all ten touched files, with every
formatting hunk touching this batch applied. NOT compiled, NOT tested. SOLO.
All logic comes from IDA on debug `sk82_na_m.xex`:
- switch cases resolved with `get_switch_info` / `calc_switch_cases`;
- raw disassembly read past each case to its return;
- native update thunks disassembled to their direct calls.

## Reported failures and fixes

| # | Failure | Native | Fix |
|---|---|---|---|
| 1 | Action Update PendingExpression(98) | Pre 82904998: `c137` (HasAnimAttribute IdleMounting) OR `c138` (OB_Mounting) OR `c139` (IsInLocomotion) OR NOT `c127` (PhysFilteredState teleport); the tail `li 0` / `li 1` negates the last read. Sustain lands on 8290a90c `li r3,1` (jumptable comment lists 98). | Pre 98 and Sustain 98 = true |
| 2 | Motion PendingExpression(121) | Pre 8290def8: one cached `HasIntent` read at sConditions+0x324 = c201 (N_360InwardHeelflip). Sustain goes to 829221c8. | Pre 121 = c201; Sustain true |
| 3 | Action Update PendingStateUpdate(0x828ed108) | `StateUpdate_Idle_43`: OnBoard_6 828ecfe0 (Action_0 for packed0..10; direct Updates of packed11..18 = CreateMG, CreateMG, CreateMG, BodyFlippingSignal, CreateMG, CreateMG, CreateMGTime, CreateMG), then direct CreateMGIntentFromAGIntent Updates of packed19/20. The Rust dispatched only Idle43. | All seven aliases in `additional_shared_update_slots`: 17/39/40/43/78/79 (…62, 63) and 22 (…88, 89). The Idle43 one-off block is folded into the table. |
| 4 | Motion PendingCallback{1271, 0x825187c0} | 1271 is ScoringGrabs `nofoot_air` (Dismount649). Its Begin is the shared BLR 825187c0, which the index gate rejected. Every ScoringGrabs shares binding vtable 82302b78, Create 82bb3da8 (null instance), Begin/End 825187c0, and Update 82c8ef88, which is one virtual Create call (payload-driven). | Index gate removed. The Dismount649 child allowlists (958/959/961/1106/1107) already existed. |
| 5 | Motion PendingStateUpdate(0x828ee2e0) | `StateUpdate_KickoutDismount_13`: OffBoard_1 828eddc8 (Motion_0 packed0..3, AttachIntent 4..11, ToggleBoard 12, CreateAttribute 13), `PlayAnimation::Update` packed14, `ControlAirLegExtension` packed15. | (13, 828ee2e0) = [0..13, 48, 49] |
| 6 | Action Update PendingExpression(17) | Pre 82900190: `c72` (PhysFilteredState air) true gives 0; else the result is 1 only when `c73` (offboardair) is false. Transition 84 guard (TransToOffBoardAir44 to OnBoardGeneral1). Sustain lands on 8290a90c. | Pre 17 = !c72 && !c73 |
| 7 | Motion PendingStateUpdate(0x828f4940) | `StateUpdate_PushQueued_1188`: Switch_1031 828f2fc0 (Riding_277 packed0..27, SetTurning 28, SetScoreAugmentation 29, AttachIntent 30, PlayAnimation 31), then direct `ComputeFirstPushStrength_Update` packed32. | (1188 \| 1294 \| 1295, 828f4940) = the NoPush1187 list plus 2184 |
| 8 | Motion PendingExpression(328) | Pre 82914b74: c234 (FingerFlipShuv) AND c266 (SeatbeltGrab), cached. Sustain goes to 829221c8. | Pre 328; Sustain true |
| 9 | Before SystemLogic Entry PendingComTrajectory (manual revert off a ledge) | RevertGround102 to Air200. `PhysicsAir::Enter` 82d98dd0: predecessor not in {100, 103, 201} sets use-COM plus heading. | Full COM branch and InitMountTrajectory (below) |

## Adjacent aliases and immediate continuations (same evidence standard)

- **Action 25/26 (Mirrored/UnMirrored under TransitioningToOnBoard11):**
  - 828ed3e8 = Action_0, then `CreateConstMGIntent::Update` packed11 (57), then
    a direct CreateMGIntentFromAGIntent packed12 (94/95).
  - This is reached as soon as expression 98 selects state 11.
- **Motion 1239/1250:**
  - Seatbelt/Crail FingerFlipShuv share 828f3f20: Grabbing_995 for packed0..26,
    the merged no-op (`AIProcess::OnEnd` symbol) for HandBusy/EndGesture
    packed27/28, ScoringTrick packed29, PlayAnimation packed30.
  - The class sequences of both aliases match.
  - Expression 334 (Crail, previous batch) also stopped here.
- **Motion expressions:**
  - 36 = c84 (WillExpire 0.033), Pre 8290bec4.
  - 37 = NOT c85 (PhysFilteredState air), Pre 8290bf24; `cntlzw` on both the
    fresh and the cached paths.
  - Sustain for both lands on 829221c8.
  - These are KickoutDismount13's exits.
- **Motion 14/15:**
  - 828ee340 (symbol TrickDismount_15; KickoutToAir14 aliases it) = OffBoard_1,
    CreateAttribute ×2, PlayAnimation, ControlAirLegExtension.
  - The classes of both aliases match.

## PhysicsAir::Enter COM branch (82d98ee8..82d98fb0)

1. Load `mFramesSinceJumpCorrect` (Processed+920) and `mCentreOfMassVelocity`
   (Processed+260; confirmed store 82e0297c = `prior_output().centre_of_mass_velocity()`).
2. `cmpw`/`bge` against the live `kPhysicsAirJumpFixFrames`. If fewer frames,
   run the existing `CalcVelFromJump` port (`air_jump_velocity::try_calculate`).
   - The Processed guard is released first, because that port relocks it.
3. **Run or sprint mount:** InitMountTrajectory, which ignores the velocity
   from step 2.
4. **Otherwise:** fill `mComTrajectory` (+50/60/70/80) = {Processed+250
   `centre_of_mass()` (store 82e02944), the step-2 velocity,
   `VECTOR3_GRAVITY` (initializer 82f76360: 0, -9.8, 0, 0), splat(-1)}.
5. If the heading flag is set, store `Skeleton+3ded = 1`, via the new
   `SkeletonRootTransforms::request_air_entry_heading`.
   - The same byte is read by `UpdateRootTransformsKnownAir` 82b3b1e4 and
     `UpdateRootTransforms` 82b42aa8.

Existing consumers were already ported and are now reachable:
- `physical_player_air_update_finish.rs` routes use-COM to the KnownAir
  skeleton update, which advances `com_trajectory`;
- the COM launch-info path reads `com_trajectory.velocity`.

## PhysicsAir::InitMountTrajectory 82d8abb0 (decoded from raw instructions)

IDA prints VMX128 registers 32 too high (v95 is v63, v92 is v60, v89 is v57, v86
is v54, v83 is v51, v81 is v49, v79 is v47). VA-form operands print as
vD, vA, vB, vC.

1. `base = v = Processed+260`.
2. **Run:** `v = Processed+e0 * a`, then `v = fma(Processed+d0, b, v)`.
   - `a` = key 0xbac43a7a1c428aaf, `b` = key 0xf92bfcf1f71c9361.
   - Both are Air-own `mAttributes`+20 `physics_airstates` reads (null gives
     `DefaultDataArea(4)` = 0).
   - Processed+c0 is `mEffectiveAnimTransform` (store 82e035b0 from
     `GetEffectiveAnimToWorldTransform`), so +d0 is Y and +e0 is Z.
3. **Sprint:** the same with keys 0x136fa41a2878e71e and 0x0e3c6cbce1494428.
4. `len` = `|v - base|`:
   - `vmsum3fp` squared;
   - `vrsqrtefp` with two VNMSUB/VMADD refinements (identical to the existing
     CalcVelFromJump correction);
   - zero when the square is zero.
5. `kPhysicsAirMountLaunchTrajMaxChange` (new cvar 539: default 4.0, range
   0..20, initializer 82f8eb30, table 82fd56e0 between kKnownAirPhysAirDamping
   and kOBA_TrajPosDelta, so registration order is ...247, 539, 249...).
   - `fcmpu`/`ble`: only an ordered `len > max` restores `v = base`.
6. Debug overlay requirement, then the same four `mComTrajectory` stores
   (+250 position, `v`, gravity, splat(-1)).

## Files

- `generated_action_runtime_selection.rs`: Pre 98 and 17; Sustain 17 and 98.
- `generated_action_runtime.rs`: table aliases 17/22/25/26/39/40/43/78/79,
  the Idle43 block folded in, and a test deriving each alias's ancestry and
  the native direct-call classes.
- `generated_motion_runtime_selection.rs`: 121, 328, 36, 37.
- `generated_motion_runtime_sustain.rs`: those four are native true.
- `generated_motion_runtime_lifecycle.rs`: allowlists 13, 14, 15,
  1188/1294/1295, 1239, 1250; ScoringGrabs gate removed.
- `input_cvar_runtime.rs` (+ its test's sorted label list): definition 539
  and `try_physics_air_mount_launch_traj_max_change`.
- `physical_player_air_state.rs`: `try_scalar` (own attribute, null gives 0).
- `physical_player_air_state_session.rs`: the COM branch,
  `init_mount_trajectory`, and error variants (the pending ones are removed).
- `skeleton_root_transforms.rs`: `request_air_entry_heading`.

## Known next frontier (NOT implemented; expected next reports)

- **Tail/nose grab families (Grabbing989/995: Into, Cycle, NoFootAirWalk,
  OneFootAir, FingerFlip), not ported:**
  - FingerFlipShuv exits into Cycle1143 (expression 169, update 828f3470) and
    Cycle1149 (expression 179, update 828f3540);
  - their siblings use 828f3420/34e8/3538/35c8/3d20/3e20/4990/4fd8 and
    expressions 146, 149, 150, 170, 322..331 and 411.
- **Off-board air strides:**
  - KickoutToAir14 exits to RightFootStride37/LeftFootStride38 (828eeb60 =
    OffBoard_1, OBAir no-op, CreateAttribute, PlayAnimation,
    `LandingOffBoardAir_Update`, ControlAirLegExtension);
  - `LandingOffBoardAir` has a payload but no runtime lifecycle yet.
- **Boneless602** (physical state) remains unfinished.
