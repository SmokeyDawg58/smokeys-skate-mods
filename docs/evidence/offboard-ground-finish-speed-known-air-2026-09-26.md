# Parity goals batch: off-board ground height, crouch speed, speed/known-air traces (2026-09-26)

**Status:** rustfmt parse PASS on every touched hunk. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Goal 2: skater standing slightly above the ground off-board (ROOT CAUSE FIXED)

- **Native.** `OffBoardFeetIK::UpdateGround` 0x82dd1e00 runs InitUpdate, then ConditionLineTestResultsForGround, then Update (FeetDown/UpdateState/IKTargets/ApplyIK), then **CalculateGroundCorrectionAndWipeout 0x82dcd328**, then **CalculateHeightAboveGround 0x82dd0788**, and finally stores the height to FeetIK+0x110.
- **Port bug.** `try_begin_pair_offboard_ground_feet_ik` stopped after ApplyIK. Its own header said "first two calls". It then read `feet_ik.height_above_ground`, which only the air path (`CalculateHeightAboveGroundAir`) or Reset ever wrote. PhysState_OffBoard::Update 0x82da03cc passes that value to Skeleton::UpdateOffBoard, so the standing skater kept a stale air/zero height. That is the reported hover. It depends on the previous air phase, hence "sometimes".
- **Also missing as a consequence:**
  - `mBadGroundCorrection` (+0x50), the push away from a foot over bad ground, never updated. It is consumed by PhysState_OffBoard::CalculateBadGroundCorrection 0x82d870d8.
  - `mRequestWipeout` (+0x131) from the bad-ground and feet-not-down counters never fired. It is consumed by OffBoard post-physics as ground wipeout request 25.
- **Port.** New `physical_player_offboard_feet_ik_ground_finish.rs`, called right after ApplyIK:
  - **CalculateHeightAboveGround.** Returns AnimToWorld.w.y (also stored to +0x100) minus min(L', R'), where each valid foot's value is surface.y + kSlopeEffectScalar * dot(NormalizeSafe(n - (x.n)x), z). The slope adjust is stored to +0x108/+0x10C. It uses fsel min, with NaN keeping the left/animation value. No valid foot gives 0.
  - **CalculateGroundCorrectionAndWipeout.** VMX128 operands were decoded from the raw words: v95->v63, v94->v62, v84->v52, v79->v47, v71->v39, v69->v37, v92->v60, v91->v59, v90->v58, v89->v57, v86->v54.
    - If neither surface is valid, the frame is "both bad" and the feet-not-down counter is skipped.
    - Otherwise:
      - A foot is "not down" when it is invalid or `!(animY - surfY <= kHeightAboveGroundFootTest)` (ordered `ble`). The feet-not-down counter increments only when both feet are not down, and resets otherwise.
      - A foot is "bad" when it is invalid, or `animY - surfY > kHeightAboveGroundToWipeout` (ordered `bgt`), or `kYNormalValueToWipeout > normal.y`.
    - The wipeout counter increments on both-bad and resets otherwise. RequestWipeout is set when the wipeout counter exceeds FramesToWipeoutForBadGround, or when the feet-not-down counter exceeds FramesToWipeoutForFeetNotDown. Both compares are signed, and the second cvar is read lazily.
    - **Correction branch** (either foot bad and Processed+8d0 bit 0x00200000 clear):
      1. Clear trajectory validity. Take GetTrajectoryWorld 0x82b3b488, which is AnimToWorld rotating mTrajectoryTransform.w with two FMAs.
      2. If its squared length exceeds MinTrajValue^2, store the two-refinement normalization to +0x60, mark it valid and zero the counter.
      3. Pick the away direction: animR-animL when the left foot is bad, animL-animR when the right foot is bad, and -EffectiveAnimTransform.z (Processed+0xE0) when both are bad. Flatten Y and NormalizeSafe it.
      4. The direction is "opposed" when trajectory-valid and 0 > dot(direction, traj). If not opposed, subtract z. Flatten and NormalizeSafe again.
      5. Increment the counter. At or above FramesForGroundCorrection, zero the correction and clamp the counter.
      6. Otherwise the correction is speed * push. When opposed, the flattened unnormalized trajectory is also subtracted.
    - **Else branch:** zero the counter, validity, correction and trajectory direction.
- **New cvars** 552..559 (Physics_Systems_OffboardIK, native table 82fd5df4..5e14 between kOB_IK_MaxFeetYDist and kSlopeEffectScalar; registration order placed identically):

  | Def | Name | Default | Range |
  |---|---|---|---|
  | 552 | kHeightAboveGroundToWipeout | 1.0 | 0..10 |
  | 553 | kHeightAboveGroundFootTest | 0x3e3851ec | 0..1 |
  | 554 | kYNormalValueToWipeout | 0x3f19999a | 0..1 |
  | 555 | kOBIK_SpeedAwayFromBadFoot | 0x3ba3d70a | 0..1 |
  | 556 | kOBIK_MinTrajValue | 0x3c23d70a | 1e-4..1 |
  | 557 | kOBIK_FramesToWipeoutForBadGround | int 20 | 0..60 |
  | 558 | kOBIK_FramesToWipeoutForFeetNotDown | int 40 | 0..120 |
  | 559 | kOBIK_FramesForGroundCorrection | int 30 | 0..60 |

  The title-order oracle in input_cvar_runtime_tests.rs is updated (section/title sort simulated).
- **Scope:** PhysState_OffBoard_Pushing::Update also calls UpdateGround, but that state (MVOBJ) is not implemented in the port (known frontier). Locomotion passes 0.0 height to UpdateOffBoardLocomotion (82da06c0), so it was unaffected.

## Goal 3: speed wobbles and high-speed skater (pipeline audited; one parity fix; trace added)

The whole native pipeline was compared, and the port already implements it:

- **Speed wobble physics.** `SpeedWobble::CalculateSpeedWobble` 82b2eb68 is called from PhysicsGround::UpdateSkateboard 82d9a108. Every attribute hash matches, as do graphs +0x0/+0x40/+0x80/+0xC0, the 2pi angle step, the 0.5 blend and the reset. Its inputs are tilt, Processed+944 ZSpeed, com-to-deck length, truck +9C4 and dt +93C, and its output feeds UpdateSteeringTilt. The port's `try_update_pair_ground_control` matches.
  - Vault `physics_speed_wobble` tuning: base 12 m/s, truck scale 5, crouch scale 3, com-to-deck 0.5..0.9, speed range 10, time range 20, frequency scale 6, amplitude 0.3.
- **Skater lean.** PhysOut SkateboardMotion+B0 mSpeedWobbleEffect feeds PhysOutConditioner_Animation::UpdateAnimationTurn 82e2b470 (wobble-weighted lean and the output blend graph +0x120). The port's `try_update_animation_turn` matches.
- **Skater speed tuck (anim data).** OnBoard.abin has btree_riding(tilt_x) -> btree_crouching(disttocog) -> l_idle(angle) -> `speed_tuck` PhaseBlend on `speedtuck`, containing r_idle_speedtuck_n_0_cyc.
  - `speedtuck` is written only by SetTurning::Update 82cc8408, as graph anim_carving+0x240(|SkateboardMotion+A8 ZSpeed| - 12) above 12 m/s. The port's motion_set_turning matches.
  - The tuck is visible only in the crouched branch (disttocog about 0.59).
- **Parity fix (Crouching).** Native Crouching reads SkateboardMotion **+0xA4 mSkateboardSurfaceSpeed** at Initialize 82cb4160, auto-pump mode 1 82ccb02c and the final disttocog 82ccb76c. The port read DeckSpeed +0xA0 in all three. `PhysicsValue::SurfaceSpeed` now maps to `try_motion_surface_speed`. CORRECTION (same day, next batch): the shipped `crouching_max_height` graph is NOT flat. It maps x=[0,8,11,14,17.44,20.80,24.44,28] m/s to y=[0.95,0.95,0.95,0.95,0.875,0.729,0.604,0.567], so native lowers disttocog (auto-tuck) above 14 m/s. DeckSpeed and SurfaceSpeed are both magnitudes and nearly equal on static ground, so this fix alone does not create the tuck; the port still has to exceed 14 m/s. The auto-pump target graph now sees the correct signed speed.
- **Open.** Every stage matches statically, so the missing effect is most likely that the port never exceeds the 12 m/s thresholds, or never crouches at speed. New `SKATE2_TRACE_SPEED` plus `play-speed-trace.bat` print, only above 8 m/s:
  - speed, threshold, active and effect (wobble);
  - z_speed and speedtuck (SetTurning);
  - surface_speed, crouch and disttocog (Crouching).

## Goal 1: KnownAir drop after manual pops (still open; more native-identical stages ruled out)

The following were compared to IDA this batch and are identical in the port:

- SkeletonCollision::Update 82b4b8f0 gates.
- SkeletonController InitAirCollisionUpdate/UpdateAirCollision/EnterState, and SetAirCollision/SetGroundCollision (identical part volumes).
- PhysState_KnownAir::Enter/Update: foot-disable happens only while targeting a grind.
- The UpdatePostPhysics info build: DZ = 8c8 bit 0x10000, ground trick = category 600.
- UseSkateboardError (horizontal plus the downward component only).
- The UpdateRootTransforms prediction (pos + vel*dt, pre-launch).
- GroundAnimation::Update/UpdateSkateboard/Exit.
- Island::CreateContactSpyList, and SpyContactJacobians selecting nb&8 and jacobian+0x50.x > 0 (combined velocity+position normal impulse).
- Sequence/Clip::AddTime overflow (no speed term).
- ProcessAnimAttributes: only type-0 attributes are processed. DangerZone sets bit 0x10000. NoDangerZone 0x40 is read only by CheckAllowGrindLogic.

What remains is the frame-1 toe body pose. The port's toe sits 3-6 cm below the ground during manuals near the tail tip. The `known air: foot contact` trace now prints the toe body record position and velocity plus the contact position, for the next capture.
