# Active walking fidelity audit

Goal: review walking against the original IDA reference and correct mismatches,
including right-stick turning that currently does not rotate the skater. Solo.
Do not equate absence of pending errors with functionality or visual parity.

## Requirements and evidence still needed

- Foot planting and swinging: corrected ApplyIK locomotion dispatch; source
  and validation in walking-foot-targets-2026-09-25.md. Full gait/terrain
  coverage remains to audit.
- Stationary right-stick turn in both directions: INCOMPLETE. Motion754 runs
  in baseline frames180..290 but the skater does not visibly rotate.
  `artifacts/walking-turn-before-290-20260925.log/.png`.
- Walking steering, including right stick while moving: source/input/heading
  trace and runtime heading evidence required, not just state entry.
- Gait start, sustained movement, stop, directional changes and sprint:
  review authored animation selection, trajectory generation and root motion.
- Jump preparation, moving/stationary jump and landing: review source flow
  and physical/visual outcome. Prior single forward-jump replay is partial
  evidence only.
- Terrain/step/slope contact, collision correction and walk-off transitions:
  audit existing code against source, retaining explicit unsupported paths.
- Shared publication, mirroring, camera inputs and return to standing/board:
  verify changes reach the existing owners; avoid duplicate implementations.

## Right-stick turn investigation

Generated Motion754/753 uses shared update828f1e88: OffBoard1, CreateAttribute
755 (OB_TRAJ), SteppingBoardNoBoard1434/1431, AttachIntent1435/1432 (OB_Steer
to ob_Turn), then PlayAnimation1436/1433 (OFB_SPOT_TURN_90_L/R, speed1.2,
blend0.3). Existing dispatch contains those slots; therefore no new intent
handler should be invented to address this symptom.

Native PhysState_OffBoard.Update82da03a8 consumes Processed+984 mOB_Turn,
offboard attribute51975cef9bf48a85, degrees-to-radians and fixed1/60 before
Skeleton.UpdateOffBoard. Existing implementation contains this calculation.
Diagnostic SKATE2_TRACE_OFFBOARD_TURN now records the actual attribute,
factor, radians and heading to locate the first broken handoff.
Diagnostic release build log: artifacts/walking-turn-diagnostic-build-20260925.log.

### Confirmed stationary-turn defect

Diagnostic290 replay succeeds but reports attr=1.0, factor=0.0, radians=0.0
for118 turning ticks. Input and AttachIntent publication are therefore present.
Raw PPC at82da0418..0434 constructs LOW r4=51975cef, HIGH r9=9bf48a85,
then `insrdi r4,r9,32,0` inserts the high half. Correct key is
**9bf48a8551975cef**, while Rust had its two halves reversed. The wrong key
fell through the original missing-attribute zero fallback, disabling rotation.
Corrected the key and added an original-asset bootstrap regression requiring
the authored positive rate and absence of the old reversed key. Runtime and
new test validation pending in walking-turn-key-fix-build-20260925.log.

Adjacent key audit: SetOffBoardSkateRootTransform82b4d198 correctly constructs
31bd1d8686684e85; CalculateOBStepTurn82c8b608 correctly constructs
24e76e2caf7a90f8. CreateLocomotionTrajectory82cac998 sprint increment/decrement
320f3450e99ab013/f2cdca4243861327 and turn curve e8c68cdbc3a35230 match Rust.
No blanket key-swapping is justified.


Original-asset regression PASS: correct key resolves **135 degrees/second**;
reversed key is absent. Log: walking-turn-key-asset-test-20260925.log.
This uses the real mounted Vault, not invented tuning.

One post-fix replay did NOT test offboard turning: the demo's initial sample
pressed D-pad Up and entered a gesture (Action49/Motion1165), so Y never
reached the offboard state. That run's exit0 is not turning evidence.
ControlsDemo bootstrap now uses genuinely neutral buttons0, matching its
subsequent samples, and an offboard-right-right sequence complements left.
No gameplay input handling was changed. Rebuild/replay validation pending.


## Verified stationary-turn result

Neutral-bootstrap release build PASS (`walking-turn-neutral-demo-build-20260925.log`).
Both offboard-right-left and offboard-right-right replays PASS540 ticks and
enter Motion754/753 respectively, then Standing752. Live traces show authored
factor135, signed input+1/-1 and yaw+0.03926991/-0.03926991 radians per tick.
Heading changes and distinct final facing directions are visible in captures.
Logs/captures: `artifacts/walking-turn-final-offboard-right-{left,right}-20260925.*`.
The matching offboard-jump540 replay also passes and returns to Standing752.

Tests PASS152: IK107, locomotion44 and original-asset bootstrap1.
Logs: walking-turn-final-ik-tests-20260925.log,
walking-turn-final-locomotion-tests-20260925.log, walking-turn-key-asset-test-20260925.log.

This establishes the reported stationary right-stick defect is fixed, not
that every walking branch has been audited. The broad goal remains active.
City streaming/rendering is queued after this fix: buildings/textures and
foliage alpha now become the next investigation, while remaining walking
coverage above stays explicitly unproved.
