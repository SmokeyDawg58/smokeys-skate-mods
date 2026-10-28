# HippyJump, Motion 436, Boneless 602 and the full FootPlant 601 chain (2026-09-26)

**Status:** rustfmt parse PASS on every touched file; all new files are rustfmt-clean and no touched hunk has a rustfmt change. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

The four reported pending errors:

1. `Tick(Before(SystemLogic(Selection(PendingHippyJumpDebug))))`
2. `Tick(Actor(Motion(PendingExpression(436))))`
3. `Tick(Before(SystemLogic(Selection(PendingTransition(602)))))` (no complies)
4. `Tick(Actor(Action(Update(PendingExpression(44)))))` (footplants, "and their whole chain")

## 1. HippyJump (CalcSuggestedState 82ddc160..c1dc)

- Ground with `mIsHippyJumping` (8d0 0x8000) and not both feet on deck (8d0 0x3000 != 0x3000) suggests LandingOnDeck 505.
- 82ddc1b0 calls `SingletonHolder<DebugOverlay>::Get`, which asserts the instance. Its "Hippy Jumped!" print is the shared BLR 825187c0.
- The port: `Selection::HippyJumped` requires the overlay, then selects 505. `PendingHippyJumpDebug` is replaced by `Overlay(DebugOverlayError)`. (Ground, 505) was already admitted.

## 2. Motion 436 (Pre 8291cae8, Sustain 829221c8)

- Mute/Stale Cyc670/677 -> WipeOut4: `c(4) || c(87) || (c(253) && (c(227) || !c(85)))`.
- That is wipeout, then runout, then HasTweak(>=0.2) with contact or not-air, through the shared 426 tail.
- The Sustain slot is the native default `true`.

## 3. Boneless 602 (no comply)

- Enter 0x82d89638, Update 0x82da0d20 (DoJump 0x82d9ebe8, CalcBonelessJump 0x82d978e0), UpdatePostPhysics 0x82d896b8.
- Exit and FillPhysOut are the shared BLR.
- The Skeleton update is `UpdateFixedWorldPositionOfBone` 0x82b5d588 with its roots `UpdateRootTransformsFixedPart` 0x82b42110.
- Selection case 82ddd070; GetType 0x25a.
- New cvars 561..564: kDoJump, kTrajPosDelta, kMaxJumpErrorBoneless, kDebugUseRawBonePos.
- All of this was implemented earlier in the batch.

## 4. The FootPlant chain

The action/motion side was done earlier in the batch:

- Action Pre 44/45 swap Right/LeftPush by stance (82901218/8290136c); Sustain 44/45 (82906ef4/82906fc0).
- Pre 10 = c46 `IsFootPlanting` (828fff38).
- `IsFootPlanting` reads Air `is_foot_planting && total_absorb >= 0` (unordered passes).
- The Motion `FootPlantAbsorb` lifecycle publishes `absorblength` (828f3bf8, Update 82cb4f18).

The physics chain, end to end:

- **Arming.** The `OneFootAir` animation attribute sets 8d0 0x08000000 (`mIsDoingOneFootAir`). This already exists in `skeleton_processed_anim_attributes`.
- **KnownAir.Update 82d9983c..874.** It runs `FootPlantManager::UpdateFootplantAir` when:
  - 8d0 0x08000000 is set;
  - 8cc 0x10000000 (`mIsBodyFlipping`) is clear;
  - the state timer is greater than `kMinAirTimeToFootPlant`.

  Otherwise it runs Reset. The old port compared with `<=`; native `ble` also resets on an unordered timer, so the port now uses `!(timer > min)`.
- **Landing flag.** CalculateLanding sets `mLandingValidFootPlantNextFrame`. The existing Fill 82db6890 publishes it as `is_landing_foot_plant`, and the ProcessData tail turns that into 8d0 0x04000000 (`mIsLandingFootPlant`).
- **Selection.** KnownAir selects 601 (82ddc88c; this already existed).
- **Hold.** FootPlant case 82ddd24c:
  - wipeout -> 300;
  - 8d0 0x02000000 (`mIsFootPlanting` = landing || planting) holds 601;
  - otherwise 200 | (8c4 0x400 ? 1 : 0).
- **Transitions.** Admitted: (KnownAir, 601) and (FootPlant, 300/201/200/702). EnterNewPhysicsState(601) stops the controller. GetType 82d97698 is `li r3,0x259`.

### UpdateFootplantAir 0x82dc3928

1. Clear the perfect-leg flag (+1d8).
2. Run CalculateFootToTest.
3. If the foot is off the board, run UpdatePrediction. Otherwise clear ResultsValid, Targeting and the future's ready byte (+b0).
4. If no result is valid, clear the last-IK-target flag and run Reset 82db6760.

### CalculateFootToTest 0x82dbbd10

- **Box.** Extents come from attribute 0x5d73c95431b70c59 (16 bytes); the Y offset from 0x8de498d065a4e4a5. The bounds are ±extents + (0, off, 0, 0), with the negation done by a sign xor.
- **Toes.** The raw animation toe is `GetAnimPartTransformWorldRaw` 82b45f58 (AnimRecord volume composed with AnimToWorld). The physical toe is `GetPartTransform` 82b45e18. Both are read for Part15 and Part19.
- **Board space.** The physical toes are transformed by Processed+100 (`mInverseEffectiveTransform`).
- **COMs.** Skeleton COM +3d00 goes to +130. `GetAnimCentreOfMass` 82b3b978 goes to +140.
- **Test.** A toe is off the board when, on any XYZ lane, `min > p || p > max` (vsldoi drops W; unordered counts as inside). The left toe is tested first.
- **Selection.** Stores the part, the off-board flag, the animation and physical toes, and `NormalizeSafe(phys - COM)` as the leg.

### UpdatePrediction 0x82dc1220

- **Ready future.** Consumed only when every lane of the time is >= 0:
  - `TimeToCollision` 82d83ca0 asserts |t| < 100;
  - `fsel` clamps it to 0..1;
  - `GetContactPosition` 82db57a8 gives the landing position;
  - the surface comes from +44;
  - ResultsValid is set.
- **CheckForGrinds.** Then targeting:
  - to start, 0.11 < t < 0.3 (kFP_Min/MaxTimeToStart);
  - to hold, timer + t >= kFP_MinTimeAfterStart;
  - otherwise the zero vector, 0.0 and 0 are stored;
  - while targeting, `kUpdateTime` (1/60 at 823512a4) is added and CalculateLanding runs.
- **Always afterwards.**
  - LandingNormal = Processed up.
  - If the perfect leg is invalid it becomes (0,-1,0,0).
  - `start = fma(perfectLeg, attr dd9b6fbfb6775050, COM)`.
  - `g = graph ca25595717aeaf32(t)`, or 1 without a valid result.
  - `p0 = fma(COMvel*dt, kTrajStartTestTimeAdjust, fma(start, g, physToe*(1-g))) + up*kFPYFootOffset`.
  - `SetupFakeBodyPrediction` 82e4bb88 -> AddQuery with the predictor's gravity, the COM velocity, kFootPlantTrajRadius/MaxTime and kFootPlantTrajError for both ratios. The true return is asserted.
- **Dead calls, omitted (both callees are pure).** `GetPartVelocity` (var_80 is never read) and `NormalizeSafe(COMvel x J)` (var_D0 is overwritten at 82dc1610).

### CheckForGrinds 0x82dbbaf0 (grinds parked)

- `kFP_GrindBoxSize` builds the AABB for `GrindData::TrajectoryBoxTest` 82cf9cf0.
- The native zero-count branch 82dbbb7c returns before the adjust loop and before its debug check.
- With grinds parked (`ParkedByUser`) the test has no rail to report.
- `RunRecoveredPrefix` returns an explicit pending error, as the TrajectorySelector does.

### CalculateLanding 0x82dbbfd0

- **Perfect leg.** From the landing velocity `fma((0,-9.8,0,0), t, COMvel)`, normalized: `vect = fma(dir, kCOMVelLegScalar, -normal)`. `ClampVectorWithinAngleRange` 82e49fd0 clamps it to -normal within kMaxEntryAngleDegrees*DEG2RAD (0x3c8efa35), then it is normalized and the valid flag is set.
- **Leg to landing.** (landing - COM), with the same clamp and normalize.
- **Blend.** Graph 164b0a6d1bfd0a02(t), `fma(toward, w, perfect*(1-w))`, normalized.
- **Angle.** `XMVectorACos` 824b0278 of dot(blend, leg), wrapped by the inline g2PIReciprocal / vrfim / (>0.5) / g2PI sequence.
- **IK target** (when t < kIKBlendInMaxTimeToLand and kIKFootPrepare):
  - Skeleton+2dc0 WorldToAnim rotates the blend (mul, fma, fma).
  - It is scaled by the two-refinement length |animToe - animCOM| and added to the transformed animation COM.
  - When the last target is valid, the step is clamped by `ClampVectorWithinMaxLength` 82e49430 (kFPM_MaxOffsetDelta).
  - `SetLeft/RightFootIKTargetPositionAnim` 82b41290/82b412b0 store it unchanged: IK+260/270, local-set byte, blend.
  - The blend becomes `select(kIKBlendInSpeed + blend)` clamped to 0..1 by two fsel.
- **Landing flag.** `mLandingValidFootPlantNextFrame = valid && !(t > 0.017) && attr 945bb959a3e377e9 * DEG2RAD > angle`.
- **Collisions.** When t < kFPM_TimeToLandToDisableFootCollision, `DisableRight/LeftFootCollisions(2)` runs on Parts 19..21 or 15..17.

### FootPlant 601 state (vtable 8234a534)

- **Enter 82d89420.**
  1. `EnableAngularDriveOnly(1.0)`.
  2. `SkeletonController::DisableFeetCollision` 82dd4778 -> `Skeleton::DisableFeetCollisions` 82b46220: Parts 19, 15, 20, 16; Volume group 4, enabled bit cleared, counter 0; no active byte.
  3. StartFootPlantGround.
- **StartFootPlantGround 82dbc6a8.**
  - Sets planting, time 0, landing flag 0.
  - IK foot = landing + attr 8c1a73c711a5b513 on all four lanes.
  - Leg = normalize(COM - IK foot).
  - Gain = graph 8a8887218466ed15 of |COMvel·leg along leg|.
  - Cross (COM - IK foot) x COMvel, with the two-refinement length. At or below VECFLOAT_SMALL (2^-16), or unordered, it returns early.
  - Axis: vrefp + two Newton steps.
  - Rate = |cross|·gain; lead/lag = rate·attr d6224b71… / 68c05a7f…; floor = attr 4f45da86…·DEG2RAD.
  - Plane angle `AngleBetweenVectorsOnPlane` 82b3a360(leg, J, -axis), wrapped.
  - Swing = min(max(θ+lead, floor), θ+lag) - θ.
  - Total time = swing / rate.
  - Quaternion(axis, swing/2) via `XMVectorSinCos` 82584f60 rotates the leg (cross form, 2·fma).
  - End = fma(rotated, attr 0de0027b…, IK foot); bisector = normalize(rotated + leg).
  - Exit velocity = fma(attr c5f92168…, bisector, (vel - bisector·(vel·bisector))·gain).
  - Bezier:
    - P0 = COM;
    - P1 = fma(vel·gain·T, attr 5c4341ef…, COM);
    - P2 = end - exit·T·attr 628c228d…;
    - P3 = end.
  - Foot collision disable of `kFramesToDisableFeetCollision - fctiwz(T·-59.999996)` frames (wrapping subf).
  - Three earlier `XMVectorSinCos` results are overwritten unread (dead, pure) and omitted.
- **Update 82d976b0.**
  - UpdateFootPlantGround 82dbcf40:
    - time += 1/60;
    - cubic Bezier at time/total, keeping the native product order;
    - take-off when time > total, which clears planting; take-off position = IK foot + (0, kYTrajectoryOffset, 0, 0); velocity = exit;
    - `SetLeft/RightFootIKTargetPosition` 82b41250/70 at the IK foot with blend 1.
  - Then `Skeleton::UpdateHandPlant` 82b5da30 at the Bezier COM:
    - `UpdateRootTransformsHandPlant` 82b43880. Like the fixed-part roots but with the AnimRecord COM (+2a00). Asserts 0xCAB/CAC/CAE/CB4/CB6; trajectory and root-pose identity.
    - Assert 0xB50.
    - Orientation = AnimToWorld times the MAPPED volume Part0 (+3040, unlike fixed-bone), assert 0xB53.
    - Processed rows + 8c4 animated bit.
    - Fast ApplyBoardAnimation.
    - `TargetPositionWithVelocity` on its translation.
    - Skate root, GeneralUpdate, identity next trajectory.
  - On take-off: GetLaunchInfo; start velocity = exit; trajectory start = take-off position; board start = take-off + (0,-0.1,0,0); jumped and override bytes; Launch + Update.
  - Update5 reads the retained Reckoning.
- **Exit 82d89468.** FootPlantManager Reset; future readiness is retained.
- **UpdatePostPhysics 82d977a8.**
  - `Wipeout::CheckForPlantWipeout` (existing).
  - Then CheckForFootPlantWipeouts 82d89470, using the state's own attributes:
    - HeavyLanding request 6 with the downward speed when COMvel·(-up) > attr 52436d9f…;
    - TooFastOffBoard request 23 when the across-up speed > attr 149a5d0d….
- **FillPhysOut / GetSyncLog.** The shared BLR; the manager Fill runs from Update5 (existing).

### Predictor routing

`PredictionTarget::FootPlant` resolves to FootPlantManager+10: the same retained future, reset by AddQuery and completed by Predictor.Update.

### Cvars 565..583

- All sit between #399 (table 82fd5cb8) and #219 (82fd5d44) in native table order: 82fd5cbc..82fd5d10, with 82fd5cd0 (`kMaxDiffBetweenFootAndComVel`) unreferenced.
- The definitions and defaults were read from each initializer.
- kFP_MaxDistToAdjustFootPlant and kFP_MaxDistBelowCollisionForGrind are only read inside the parked grind loop and are not registered.

## Boundaries and pre-existing issues

- **Host numeric estimates.** Estimates (`vrsqrtefp`, `vrefp`), exceptional VMX selection and FP status remain the established host boundaries.
- **Debug draws.** They only require the overlay, as elsewhere.
- **Pre-existing: cvar title-order test.** `missing_load_and_missing_keys_preserve_targets_and_source_title_order` was already stale (21 labels behind); it is now 40 behind. It was not rewritten.
- **Pre-existing: (GroundAnimation, 300).** It is still not admitted (asserted None by an existing test).
