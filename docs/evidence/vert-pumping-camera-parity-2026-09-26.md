# Vert, pumping and camera parity: shake speed fix, graph-expression proof, vert trace (2026-09-26)

**Status:** rustfmt parse PASS on every touched hunk. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Report

- **Vert crouch:** on vert ramps native crouches automatically and gains speed differently; the port stays in a plain idle riding pose.
- **Vert speed:** pumping speed gain on vert is missing.
- **Camera:** speed shake and catch-up are missing, and vert camera behaviour differs.

## Fix 1: camera shake speed used world X only

- **Native `CameraMan::UpdateRig` 82e48670..82e487a8.**
  1. The rig subject velocity (rig+0xE0) is stored to two stack slots.
  2. `lfs f12, sp+0x70` and `stfs sp+0x80` copy velocity X. `lfs f11, sp+0x98` and `stfs sp+0x88` copy velocity Z. `stw r25` (0) writes W.
  3. The Y lane (sp+0x84) was last written with `f27`, which is 0.0 (`lfs f27, __real_00000000` at 82e47e9c).
  4. `vmsum3fp128 v51, v63, v63` then takes the length. Decoding the VX128 word 0x167ff9b5 gives VD=51, VA=63, VB=63. IDA displays VA as "v95" because its VA128h/VA128H decode is wrong.
  - Result: the shake speed is the horizontal speed sqrt(vx^2 + vz^2).
- **Port bug.** `camera_shake_subject_inputs.rs` built `[vx, 0, 0, 0]`, so shake amplitude and frequency followed only the world-X component of velocity. Riding along world Z gave near-minimum shake.
- **Fix.** The vector is now `[vx, 0, vz, 0]`. The test is updated.
- **Native tuning that now applies at speed** (`camera_shake` vault collection):
  - AmplitudeMin 0.3, AmplitudeMax 6.0, AmplitudeTopSkaterSpeed 26;
  - FrequencyMin 0.02, FrequencyMax 0.5, FrequencyTopSkaterSpeed 20;
  - RotationMultiplier 4, TranslationMultiplier 0.05, DataFPS 90, OneShotDecay 0.02.
  - Apply also scales by clamp((3.4 - cameraDistance) * 1.6667, 0, 1). Native shows no speed shake when the camera is 3.4 m or more from the anchor.

## Proof: every ported motion/action graph selection expression equals native

- **Method: symbolic execution of each native switch case in IDA.** Local IDA scripts `ida_expr_paths.py` and `ida_expr_paths_generic.py` implement it.
  - The interpreter covers the PPC subset used: lwz/stw/lbz, rlwinm, clrlwi, clrrwi, extrwi, insrwi, rlwimi, cntlzw, ori/oris, and cmplwi/cmpwi with branches.
  - It forks at every `StateConditionDirect_*` call and records each decision path (condition assignment → result).
  - It models the controller's condition cache, so a re-read condition returns its first value.
- **Tables covered.**
  - Motion Pre `jpt_8290AA8C` and Sustain `jpt_8291E250`: 447 cases each.
  - Action Pre 828ff990 and Sustain 82906048: 107 cases each.
  - Condition table bases are taken from the operands (motion 0x8306E4C0).
- **Port side.** `compare_exprs.py` and `compare_action.py` parse the Rust match arms and evaluate each native path under four completions of the unread conditions.
  - Motion files: `generated_motion_runtime_selection.rs`, the grounded fallback and `_sustain.rs`.
  - Action files: `generated_action_runtime_selection.rs`, `ground_selection` and `trick_selection`.
- **Result.**
  - Motion: 539 ported expressions match, 0 mismatch, 355 unported (they return Pending).
  - Action: 169 match, 0 mismatch, 45 unported.
  - The checker reproduces the old Pre 423 XOR bug (2 of 6 paths wrong) and passes the fix.
  - No other selection-logic divergence exists in either graph.

## Vert crouch and pump chain: verified native-identical, runtime values pending

- **`Pumping::Update` 82de0428.** Arguments: position = Processed deck w axis, ZAxisVelocity (unused), normal = `Reckoning.mGroundNormal` +0x1F0, COMToDeckWorld, deckAngle, dt.
  - Callers: PhysicsGround::Update 82da0de8 (deck angle from `mUpVector` +0x1B0 and `mGroundTransform` x +0x20) and RevertGround::Update 82da0190 (|angle(deckZ, up)| - pi/2, zero in manual). Both match the port.
  - `Reset` 82dd8a60 runs only in PhysicsGround Enter/Exit and the ctor, as in the port.
- **`Pumping::Calculate` 82dde2f8.**
  - Speed = |(pos - lastPos) / dt|.
  - `angularSpeed` = smoothed (0.2) of dot(cross(lastN, n) / dt, normalize(cross(disp, n))).
  - absorption = -speed * angularSpeed.
  - The ground angle comes from `mLastNormal.y` via acos * 2/pi.
  - The MinCrouch, GroundAngle and DeckAngle layout graphs are at +0, +0x40 and +0x80, each with a -8 scalar. The deck-versus-ground absorption choice matches.
  - The return value is angularSpeed * speedGraph.
- **`Pumping::FillPhysOut` 82dd4348.** Writes the Skeleton absorption, ground and deck compression and min crouch, plus Ground `mPumpingAcceleration`. It runs every Update5 in the port.
- **`Reckoning::UpdateGroundNormal` 82dd8178.** PID control from layout +0x4C0, `PIDFilter::Update`, then normalize into +0x1F0 and +0x240. Matches.
- **`Actor::GetPhysOut` 825f99c0** returns the raw HiLOD+0x2AC PhysOut, the same object animation reads in the port.
- **`Toolkit_CalcPumpForce` 82b28ed8.** Intention is Processed+0x8CC bit 4 (`PlayerControlledPump`, which the port routes through motion attributes into AnimOutPhysIn). Without intention the acceleration is scaled by 0.7. Direction comes from Processed+0x1B0.
- **Crouching (`anim_motion/crouching`).** baseline 0.59, output scale 1, deck scale 1, absorption/ground divisors 10, desired scale 0.075, rate limit 0.02.
  - disttocog = 0.59 + (maxHeight - 0.59) * (1 - crouch), and the crouch is clamped at or above MinCrouchVsGroundAngle.
  - That minimum is 0.36 at about 31 deg, 0.54 at 41, 0.67 at 55, 0.75 at 71 and 0.79 vertical. A vertical wall therefore gives about 0.67 disttocog, against 0.95 upright.
  - `BTREE_RIDING` blends `btree_crouching` on disttocog between the 0.965 stance and the 0.573 crouch (plus the speed_tuck branch). The PhaseBlend SetAttributes path matches native.
- **Motion `Pumping` behaviour (82c9ecc0).** Opens one of five `B_PUMP` animation channels from ground `mPumpingAcceleration`. `SetDistComToBoard` writes only on Begin, so it never overrides the per-frame crouch.
- **Camera side.**
  - `Rig::UpdateAnchorTracker` 82e42b18, `CalculatePushSmoothing` 82e3ca80 and `CalculatePumpSmoothing` 82e3cba0 match lane for lane. This includes the `vperm <0,5,0,0>`/`vrlimi` horizontal forms, the 3 m distance gate and the velocity-direction factor.
  - `CameraMan::Update` 82e48870: `dT` occupies the r4 slot, so cameraIndex is r5 and shotEditor is r6. It matches the port's mirroring/drop gates.
  - `ShotManager2::UpdateBlendValue` 82e45b00 matches `smooth_value`.
  - `GameCameraManager::UpdateSubject` 82e38e80 matches.
- **Conclusion.** No static divergence remains in these chains. The vert crouch, pump and catch-up differences must come from a runtime input value (ground normal motion, COM-to-deck height, speed, camera distance). The trace below measures each link.

## Trace for the next test

`play-vert-trace.bat` sets `SKATE2_TRACE_VERT` and omits `--quiet`, writing `vert-trace.txt`. It prints:
- `vert pump`: normal, deck angle and every Pumping field (valid, height, smoothed dh, pump time, pump amount, acceleration, angular speed, absorption, ground/deck absorption, min crouch).
- `vert crouch`: crouch, delta, minimum, absorption/ground/deck inputs, auto-pump mode, player input, surface speed and disttocog.
- `vert pump force`: intention, acceleration, mass, dt and magnitude.
- `vert anim attribute`: disttocog, tilt_x/z, speedtuck, angle, holding, quickness and dir as written.
- `vert camera shake`: speed, active, multiplier, amplitude, frequency, one-shot and camera distance.
- `vert camera anchor`: push/pump lag, smoothing, clamps, anchor lag and subject velocity.
- `offboard-air motion`: every Motion state change (shared with the off-board trace).

## Update: a static divergence was found upstream

- The conclusion above covered the Pumping, Crouching and camera chains themselves.
- Their shared Reckoning input was wrong. ProcessData fed `mComToDeck` the animation-space COM instead of the world COM-to-deck.
- The grounded up vector therefore tracked the surface instead of lagging about 12-16 deg behind it through transitions (old port 1-4 deg).
- Details and fix: `reckoning-com-to-deck-2026-09-26.md`.

## Not fixed (seen while auditing)

- **Grinds.** Grind investigation and preparation are parked in the live policy (`park_grind_preparation`/`exclude_grind_fill`), so coping hang-ups and halfpipe wheel catches cannot trigger. The active branches of `ManageHangUps`/`ManageHalfpipeWheelCatches` 82d8da78 also remain Pending.
- **Unported expressions.** The 355 motion and 45 action expressions without port arms raise Pending (fatal) if reached.
