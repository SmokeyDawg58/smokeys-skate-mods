# Vert crouch root cause (unloaded vault classes) + ten reported failures (2026-09-26)

**Status:** rustfmt parse PASS on every touched file. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## 1. Vert crouch / pumping / speed conservation: two vault classes were never loaded

**Evidence.** From a playtest `vert-trace.txt` (SKATE2_TRACE_VERT) on a ramp descent:
- The Reckoning ground normal rotated correctly: y fell from 0.9998 to 0.818, about 35 degrees.
- Every Pumping output was exactly zero: angular speed, absorption, ground absorption, `min_crouch` and deck absorption.
- `smooth_dh` was also 0 although the COM height changed.
- Crouching therefore stayed at crouch=0, giving disttocog 0.92 upright at 15 m/s.
- Reckoning attributes resolved (ground_blend 0.16-0.19), so the vault itself worked.

**Root cause.** `AttributeRuntimeVault::try_load` builds only the classes listed in `selected_original_asset_classes.rs`, and `instance()` binds null (every read returns 0) for a missing class. Two classes bound by ported code were absent from that list:
- `physics_pumping` 0xab3a006f3a2956e2 (default). Bound by `PumpingVaultAttributes` (Pumping Update/Calculate/Fill) and the pump force.
  - Holds `MinCrouchVsGroundAngle` (layout +0), `CompressionVsGroundAngle` (+0x40) and `CompressionVsDeckAngle` (+0x80).
  - Also smoothing 0.2, the speed graph, the height-delta smooth 0.1 / threshold 0.004 / max 0.006, the pump-time graph, multipliers 18/1, the no-intention scale 0.7 and the enabled flag.
- `physics_speed_conservation` 0xa099c166f4be639f (default, global+c4). Bound by `ConservationAttributes` for `Skateboard::UpdateSpeedModel`: Gravity 9.8, slope limit 7.0, contribution -0.4, force coefficients 0.25/0.25.
  - With it absent the speed-model force coefficient was 0, so the port never applied the speed-conservation force.

**Audit.** Every `vault.instance(class, ...)` binding in `skate2-core` was resolved per file and checked against the selected list. Only these two were missing; camera, skateboard, skeleton, reckoning and the anim_motion classes are all present.

**Fix.** Both classes were appended to `selected_additional_classes()`.
- The loader is type-agnostic; its only errors are duplicate definitions or nodes. Neither collection has a parent.
- Payload sizes match every reader: 4-byte floats, 0x50 entry graphs, the 0xc0 layout, and the 1-byte enable flag read by `load_enabled`.
- Neither native physics code nor the port reads the unread keys: conservation `MinSpeed` and 1.2; pumping 0.01, 0.72, 50, 1.0 and a flag. Scanned 0x82b00000..0x82e40000 for lis/ori pairs.

**Expected effect.**
- On a 35 degree ramp, min crouch is about 0.44, so disttocog is about 0.77 against 0.92 now. On a vertical wall, min crouch 0.79 gives disttocog about 0.66.
- Curvature absorption and pumping acceleration now drive Crouching, the pump channels and the camera pump lag.
- The speed model now applies its target-speed force.
- The direct pump force stays off: the vault's enable flag 0xb9cabbb88c47204b is 0, so native zeroes it too.

## 2. Motion PendingStateUpdate 0x828f1510 / 0x828f27d8 / 0x828f3420 (plus sibling families)

**Method.**
- A local IDA script, `thunk_expand.py`, expands each native `StateUpdate_*` thunk through its parent thunks into the ordered list of `StateBehaviourDirect_<Class>_Update` / `Behaviours::<Class>::Update` calls. `AIProcess::OnEnd` is the merged no-op.
- `thunk_compare.py` checks every alias's authored packed slot classes position by position.
- Control: the existing 0x828f1750 and 0x828f1448 entries also verify as matches.

**Added `(state, update)` entries.** 106 in total, each generated from graph data and asserted against the native sequence:

| Update | Aliases |
|---|---|
| 828f1510 | Mute Into669, Stale Into676 (647/656/658/667 already present) |
| 828f27d8 | OneFootOllie Into/Cyc/Out 997..1008 (12) |
| 828f3420 | tail/nose Into1142/1148 |
| 828f3470 | Cycle1143 |
| 828f3540 | Cycle1149 |
| 828f2778 | Out990/996 |
| 828f26c8 | 992/994 |
| 828f2718 | 993 |
| 828f1600 | Mute/Stale Cyc670/677 |
| 828f16f0 | Out674/681 |
| 828f3310 | 16 aliases |
| 828f3c58 | 16 aliases |
| 828f3cd0 | 16 aliases |
| 828f4fd8 | 14 aliases |
| 828f4990 | 8 aliases |
| 828f3d20 | 1234/1245 |
| 828f1070 | DBLGrab Into642 |
| 828f1120 | Cyc643 |
| 828f2288 | SupermanInto953 |
| 828f2350 | SupermanCyc954 |
| 828f23f0 | 955/956/957 |

**FilterMotionGraphIntent gate.** Indices 153, 154, 156, 157, 1249, 1250, 1324, 1325, 1333, 1340, 1341 and 1349 were added, so all 27 FilterMotionGraphIntent instances in the graph are now admitted.
- They use the same TweakX/TweakY, BoardAdjust and HandPlantTweak payload shapes as indices that were already admitted.
- `mirrorFilter` is applied by the existing generic filter pipeline.

**Still Pending, deliberately.**
- 828f2568 and 828f3e20 (GrabOut ×8) need the unported `FingerFlipOut` behaviour: CreateInstance 82712148, Initialize 82cbe948, Update 82cca470.
- 828f3bf8 (Absorb ×8) needs `FootPlantAbsorb` (Update 82cb4f18).
- The HandPlantAir families (32 leaves), and MVOBJ/step under Locomotion.

## 3. Motion PendingCallback {index 1249, address 0x82ca7628}

- 0x82ca7628 is a folded `CreateInstance` (the symbol reads EnterSkitchingBehaviour).
- Behaviour 1249 is DBLGrab `Cyc643`'s TweakX FilterMotionGraphIntent. It was outside the index gate, which is now extended as above.
- Cyc643's own update, 828f1120, was also missing and is added.

## 4. Motion PendingTransitionHook(11), (306), (308)

- **The transitions.** RunDismountWaitForLanding103, RightFootStride37 and LeftFootStride38, each to RunAround107.
  - Each runs an InitializeLocomotion hook (supported), then an Override hook: #2 (blend 0.05) or #5 (blend 0.4).
- **Where the override goes.** `OverideNextAnimTransitionHook::OnTransition` 0x82ca6be8 only writes MotionGraph+0xa68.
  - `InitializeLocomotion::OnTransition` 0x82ca6cc0 only sets five ISkaterAnim flags. None of the 77 SkaterAnim functions reference `GetNextAnimTransOveride` (decompiler scan).
  - So in native, as in the port, the override stays until the next PlayAnimation Begin consumes and clears it.
- **The gate.** Blend-only Override hooks 2, 3, 5, 9 and 16, and channelblend 10..12, share the audited consumer. The transition gate now admits 9, 11, 12, 306, 308, 322, 1110, 1122, 1134, 1146, 1209, 1221, 1233 and 1245, plus the existing set.
  - Hook 0 (`play`) and hook 8 (`blendWithCurrentFrame`) remain gated pending a consumer audit.

## 5. Motion PendingExpression(262) and Action PendingExpression(32)

- **Motion Pre 262 = c392 `WillExpire(InTime=0.075)`** (829117a8, cached, direct return). Sustain 262 is the native default true (829221c8). Guard of transition 1706 Right1294 -> FromFakie1339.
- **Action Pre 32 = !c138 && !c140** (82900970): NOT `HasAnimAttribute(OB_Mounting)`, then NOT `PhysFilteredState(offboard)`. Sustain 32 is true (8290a90c). Guard of transition 107 TransitioningToOnBoard11 -> OnBoard6.
- **Check.** `tools/compare_graph_expressions.py`: motion 541 matched / 0 mismatch; action 171 matched / 0 mismatch (+2 each).

## 6. Camera PendingBlendValue(8), and every blend type used by the shot data

- **`ShotManager2::GetBlendRawValue` 82e44898.** Shot `BlendType` (layout 0xd8) values in the `camera_shots` vault are 0, 1, 5, 6, 7, 8, 9, 13, 16, 20, 21 and 22. Types 5, 7, 8, 9, 20, 21 and 22 are now ported:
  - **5:** `GetTrajectoryProgress` 82e3e3e8 (raw end time, fsel clamp [0,1]; 1 on the ground).
  - **7:** `mPreviousShot.mPositionDistance` (port `shots.previous.distance`; `SetShot` copies it before blend setup).
  - **8:** `GetTrajectoryProgressWarped` 82e3e4f0.
    - Up to twice the apex time it returns elapsed/(2*apex).
    - After that it returns 1 + (elapsed - 2apex)/(collision - 2apex), fsel clamped to [1,2].
    - On the ground it returns 0 if the previous update was grounded (CameraMan+380 bit 0x20, written by `UpdateTrajectoryData`), else 1.
  - **9:** `mInclineAngleAbs` (+324) × 57.29578.
  - **20:** `mGrindInclineAngle` (+338) × 57.29578.
  - **21:** Subject `GetPaintedRegionBlendValue`.
  - **22:** negated `GetSkitchGrabPosition`.
- **Shots using them.** These drive the reentry (vert) shots `bl_reentry*`, the high-air-to-landing shot, ollie anticipation, grind, race and skitching blends.
- **Mapping source.** Field mapping comes from the PDB `Iris::CameraMan`, `Iris::Shot` and `ShotManager2` layouts.
- **Error variant.** The new `MissingApexTime` variant covers an unwritten apex time, which native never initializes.
