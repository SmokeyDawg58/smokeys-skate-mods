# Eleven reported failures + adjacent gaps (2026-09-26, second batch)

Status: source written; rustfmt parse PASS on all nine touched files.

Formatting was applied only to this batch's hunks:
- The two selection files and the Motion Sustain file were checked with this batch's lines removed. They were rustfmt-clean, so every hunk in them came from this batch.
- In `actor_motion_locomotion.rs`, only the new function was formatted.

NOT compiled, NOT tested. SOLO.

All logic comes from IDA on debug `sk82_na_m.xex`:
- switch cases resolved with `get_switch_info` / `calc_switch_cases`;
- raw disassembly read past each case to its return;
- update thunks disassembled to their direct calls;
- virtual slots resolved from the concrete vtables;
- struct layouts taken from the IDA local types.

## Reported failures and fixes

| # | Failure | Native | Fix |
|---|---|---|---|
| 1 | Action Update PendingExpression(15), 1085 frames | NewOneFoot40 transitions 80/81 target the inactive OneFootRight41/OneFootLeft42. ValidTarget evaluates the target's Pre regardless of the active flag. Pre 829000d0 = c44 (HasAGIntent RightPush); Sustain 829064dc is an independent c44 read. | Both masks: 15 = c44 and 16 = c45 (sibling: Pre 82900130 / Sustain 8290653c, LeftPush). Updates 41/42 (828ed5d8) were already in the table. |
| 2 | Motion PendingExpression(144), 80 | Out960 (FSGrab Dismount). Pre 8290e7f0 = NOT c230 (HasIntent FSChrist), with `cntlzw` on both the fresh and cached paths. Sustain goes to 829221c8. | 144 = !c230; Sustain true. Children 1106/1107 were already allowlisted. |
| 3 | Action Update PendingExpression(80), 631 | OneFootLeft211/218. Pre 8290371c = (c45 && !c42 && !c44) \|\| (c44 && c42 && !c45). Sustain 82908820 = (c45 && !c44) \|\| (c44 && !c45); the false tail 82906a2c is `li r11,0`. It mirrors the existing 79. | Pre 80 and Sustain 80 |
| 4 | Motion PendingExpression(173), 83 | Out1238 (SeatbeltGrab). Pre 8290f3bc = NOT c266, `cntlzw` on both paths. | 173 = !c266 |
| 5 | Motion PendingExpression(182), 106 | Out1249 (CrailGrab). Pre 8290f764 = NOT c273. | 182 = !c273 |
| 6 | Motion PendingExpression(275), 169 | RunDismount102. Pre 82911dc4 = c20 (ComVelCompare abs >= 4.0) && !c21 (LastState IdleDismount) && !c14 (IsRidingFakie) && !c18 (DisablePushBrake). | 275 as decoded |
| 7 | Motion PendingExpression(156), 152 | Transitions 1175/1274 (ToOffBoard657/668 to OBAir12). Pre 8290ece4 = c251 (WillExpire 0.0). | 156 = c251 |
| 8 | Motion PendingExpression(6), 126 | Transitions 11/313/315 (to RunAround107 / Locomotion33). Pre 8290b3b4 = c22 (OBTimeToLand <= 0.0). | 6 = c22, plus a new Motion **Shared** OBTimeToLand arm (below) |
| 9 | Motion PendingTransitionHook(1647), 1000 | Push1028 to AnticTail1286 (expression 415). Hook 15 MongoPushToAntic(`MONGO_PUSH_TO_ANTIC`), then hook 16 OverideNextAnimTransitionHook(0.2, blend). | MongoPushToAntic ported; Override gate extended (below) |
| 10 | Motion PendingExpression(408), 685 | Transition 1332 (BoardAdjusts Right Idle684 to InAir21). Pre 82919ff4 = c259 && c260 && !c261: filtered BoardAdjustAngle and BoardAdjustMag (abs, < 0.4), then NOT HasIntent BoardAdjustRight. | 408 as decoded |
| 11 | Motion PendingStateUpdate(0x828f1f68), 1036 | `StateUpdate_FromGrind_788` (below). | (787/788/866/867, 828f1f68) allowlists; Sustain 90/294 |

Motion Sustain for 144/173/182/275/156/6/408 lands on the native default 829221c8 in every case (checked individually).

### OBTimeToLand in the Motion graph (item 8)

- `construct` tries the Action `ConditionClass` names first. OBTimeToLand is in that list, so Motion c22/c23/c24 are built as `MotionCondition::Shared`, which had no arm. The unreachable Motion-payload `ComparisonCondition::OBTimeToLand` path was deliberately not used.
- `OBTimeToLand::ActivationFunc` 82c9aff0 (shared by both graphs) does:
  1. owner+0x200 IPhysical, asserted against UCOM;
  2. virtual +0x28 for PhysOut;
  3. PhysOut+0x48 for `mOffBoard`;
  4. +0x1c for `mTimeToLand`;
  5. `EvalCompare`, with no validity gate.
- Both offsets were confirmed against the IDA types `Sk8::Physics::PhysOut` and `PhysOut_OffBoard`.
- New helper: `motion_runtime_offboard_time_to_land`, mirroring the WipeoutTimeToLand helper.

### Transition 1647 hooks (item 9)

**`MongoPushToAntic`:**
- **Constructor** 82cbea28: required `anim`, stored in a FastString<36> at +0x18.
- **OnTransition** 82cbeb00: owner+0x1c IChannelAnimatable (UCOM-asserted), then virtual +0xc.
  - In the SkaterAnim IChannelAnimatable vtable 82302ce0, +0xc is `ChannelAnimatable::NewChannel` 82d7eb98.
  - Channel and animation are both `&anim`.
- **`tChannelParams`** (IDA type `Andale::tChannelParams`, 28 bytes, passed by value in r6..r9):
  - `priority` 0;
  - `cycle` false, `mirror` false;
  - `playBackSpeed` 1.0;
  - `blendInLength` 0.1 (0x3dcccccd);
  - `blendInToFirstFrame` false;
  - `blendOutLength` 0.07 (0x3d8f5c29);
  - `blendOutFromLastFrame` false.
- The trailing bool (r10) is false. The status is discarded.
- **Port:** `motion_mongo_push_to_antic` calls the existing `try_new_channel_with_blends(anim, anim, 0.1, 0.07, false)`, which is the same route BodySpin uses. Its interface and entity identity checks are the same as BodySpin's.
- **Sibling 1648:** AnticNose1287, hook 17 (`MONGO_PUSH_TO_NANTIC`) then hook 16. Same code.

**`OverideNextAnimTransitionHook`:**
- OnTransition 82ca6be8 is transition-independent: IMotionGraph (owner+0x14) virtual +0x18 `NextAnimTransOveride(this+0x18)`.
- The Rust index gate is **not native**.
- Hook 16 is one object shared by 1647/1648/1649, and 1649 was already admitted, so the gate now admits 1647..=1649.
- Other Override hooks (0, 2, 3, 5, 8, 9) stay gated until the PlayAnimation consumer is audited for `play` and `blendWithCurrentFrame`.

### StateUpdate_FromGrind_788 828f1f68 (item 11)

1. `StateUpdate_Ollie_188` 828efa48:
   - Trick_19 for packed 0..22;
   - CreateAttribute (23), ScoringTrick (24), CreateAttribute (25);
   - the merged no-op for 26/27 (IsDoingTrick, ApplyingBodyTilt).
2. PlayAnimation for packed 28.
3. The merged no-op (`AIProcess::OnEnd` symbol) for packed 29, SetTrickHeight.
4. `JumpInto_Update` for packed 30.

- Nollie 866/867 call the same Ollie_188: their 517..521 have the same classes as Ollie's 450..454.
- SetTrickHeight Begin already handles both `from AnticStrength` (787/866) and `setToValue 0.5` (788/867).
- JumpInto Update was already ported, with its first-update SetAttributes.

## Adjacent gaps closed (same evidence standard)

- **Action 81 and 82** (NoFootAirWalk212/219, IdleTailGrab214/IdleNoseGrab221). These are the TailGrab149/NoseGrab152 siblings evaluated right after 79/80.
  - 81 = c41 \|\| (c44 && c45) in both masks: Pre 82903944, Sustain 82908998, true tails 829016d0 / 82906ee4 `li 1`.
  - 82 = !c41 && !c45 && !c44 in both masks, read in that order: Pre 82903a64 / Sustain 82908ab8.
- **Action update 828edc90** (`StateUpdate_FingerFlip_213`):
  - `Regular_158` 828edac8: OnBoard_6, then packed 19/20 CreateMGIntentFromAGIntent, 21 CreateConstMGIntent, 22 BoardAdjust;
  - then CreateConstMGIntent Updates of packed 23/24.
  - Aliases 210-213, 216-220 and 223 were added to `additional_shared_update_slots`, and the alias test now checks each alias's class sequence.
  - IdleTailGrab214/IdleNoseGrab221 use 828edcf0 (`StateUpdate_RocketAir_154` = Regular_158 + packed 23), which was already folded (24 slots).
- **Action Sustain 50 and 51**, needed once NoseGrab leaves can be current:
  - Sustain 50 (NoseGrab152) = c39 \|\| c38 (829071ec).
  - Sustain 51 (RocketAir154) = c39 && c38 (829072b8; the shared tail 8290a900 returns r11 = 1).
- **Motion RunDismount102** (828ef4b0): OffBoard_1, no-op (ForcePhysics), CreateAttribute x2, PlayAnimation, no-op (Dismount).
- **`Dismount` behaviour** (Motion slot class, previously unported):
  - Create 82bb3da8 returns null.
  - Begin 82ca55f8 and End 82ca56f0 call ISkaterAnim virtual +0x90 = `SkaterAnim::SetDismounting` 82cb7678, with true and false.
    - That writes `insrwi` bit 14 of +0xE4C, which is mask 0x20000.
    - This is the same mask the existing InitializeLocomotion hook writes for `isDismounting`.
  - Update is 825187c0 (BLR).
  - Neighbouring setters, checked for the bit convention:
    - +0x94 SetLanding = 0x8000;
    - +0x98 SetRunningOutFront = 0x4000 (both matching the existing Rust);
    - +0x9c SetRunningOutBack = 0x2000.
- **Motion expressions 276, 277 and 7** (RunDismount exits):
  - 276 = c19 (WillExpire 0.1) && c22 (82911f34).
  - 277 = c8 (WillExpire 0.0) && c23 (OBTimeToLand > 0.0) (82911ffc).
  - 7 = c24 (OBTimeToLand > 0.15) (8290b410); the cached path's `lbz`/`clrlwi` reads the same bit 7.
  - Sustain lands on 829221c8 for all three.
- **Motion RunDismountWaitForLanding103** (828ef550): OffBoard_1, no-op, CreateAttribute x3, no-op.
- **Motion OBAir Dismount36** (828eeb00): OffBoard_1, AirDismounting Update, CreateAttribute.
  - Sustain 283 = c76 (PhysFilteredState offboardair) only (82920648); Pre is c76 && c9.
- **Motion Sustain 90 and 294** (FromGrind/FromManual guards) land on 829221c8.

## Files

- `generated_action_runtime_selection.rs`: 15, 16, 80, 81, 82, Sustain 50/51.
- `generated_action_runtime.rs`: 828edc90 aliases and the extended alias test.
- `generated_motion_runtime_selection.rs`: 6, 7, 144, 156, 173, 182, 275, 276, 277, 408.
- `generated_motion_runtime_sustain.rs`: native-true 6/144/156/173/182/275/408, 90/294, 7/276/277; 283 = c76.
- `generated_motion_runtime.rs`: Shared OBTimeToLand arm.
- `actor_motion_runtime_session.rs`: `motion_runtime_offboard_time_to_land`.
- `actor_motion_transition_hook.rs`: `motion_mongo_push_to_antic`.
- `actor_motion_locomotion.rs`: `motion_set_dismounting`.
- `generated_motion_runtime_lifecycle.rs`:
  - hook dispatch (MongoPushToAntic; Override gate 1647..=1649);
  - allowlists 787/788/866/867, 102, 103, 36;
  - Dismount create/begin/end.

## Known next frontier (NOT implemented; expected next reports)

**`LandingOffBoardAir`** (RightFootStride37/LeftFootStride38, update 828eeb60):
- The update runs OffBoard_1, OBAir no-op, CreateAttribute, PlayAnimation, `LandingOffBoardAir_Update`, then ControlAirLegExtension.
- It is now reached from:
  - OBAir12 (Dismount36 transitions 304/305; transitions 1175/1274);
  - RunDismountWaitForLanding103 (transition 12, expression 7);
  - KickoutToAir14.
- A port needs:
  - raw decode of Create 82cb4730, Begin 82cb4610 and Update 82ca6340 (the `fsel` clamps are garbled by the decompiler);
  - six cvars that are missing from `input_cvar_runtime.rs`, each with a verified default, range and registration order: kLandingImpactMinBlendDelta, kLandingImpactMaxBlendDelta, kLandingTimeToWaitToBlendToTwoFoot, kLandingTimeToBlendToTwoFoot, kLandingTimeToBlendToLandPose, kLandingLandPoseBlendDelta;
  - the IAnimatable virtual +0x48 writes of Impact/CloseToLand;
  - the kShowSkaterInAirDebugText overlay path.

**Also open:**
- Override hook gate for hooks 0/2/3/5/8/9 (consumer audit first).
- Motion tail/nose grab families 989/995.
- Boneless602.
