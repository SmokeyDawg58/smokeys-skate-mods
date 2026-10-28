# September 25: nine Motion/SystemLogic failures and push investigation

Work SOLO. No diagnostic replay, Cargo invocation or game launch in this batch.
Rustfmt parse-only checks passed; compilation, runtime and controller feel unverified.
The intermittent X/A push interruption is **NOT fixed or root-caused yet**. Do not
claim the earlier GetFootPos projection correction completely resolved pushing.

## Implemented source paths

- Motion Pre122 (8290df58): cached IsRidingFakie condition14 negated. Sustain122
  uses default true829221c8.
- Pre251/252 (8291135c/113bc): condition357 PhysicsSpeedCompare/direct or negated.
  Sustain251/252 (829202b8/20318): same direct/negated condition.
- Pre340 (829154f0): short-circuit condition290 OR291. Sustain340 uses condition291
  at82920ce4. The old pending-Sustain340 test now uses still-pending341.
- Update828f2450: Dismount660 parent27 callbacks then PlayAnimation slot27.
  All ten descriptor aliases958,959,961,970,971,973,1106,1107,1124,1125 connected.
- Update828f4c68 Moving1329: Brake1271 parent35, AttachIntent35, PlayAnimation36.
- Update828f45c8: Right1276/Left1277, Push1028 parent31, FirstPushStrength31,
  PlayAnimation32, ComputeTargetCoefs33. Existing implementations reused.
- Update828eeeb8 CoffinInto84: Coffin23 parent25, native no-ops25..29,
  PlayAnimation30, CreateAttribute31.
- SetManualAngle occurrence2417 Create82ca2be8 (folded UpdateRidingFakie factory,
  16-byte allocation, two zero floats), Begin82caf320 resets both floats,
  End825187c0 is a genuine no-op. Update82cc2920 reads raw Manual intent/default0;
  PointGraph8 uses anim_motion_manual layout X420/Y440 on abs(input), signed fsel.
  Scalar acceleration key aed41843fd93af02 and speed key a353ce1670d3aa40 were
  reconstructed from raw high/low word assembly, not truncated decompiler values.
  Two bounded delta stages update velocity then angle, without dt. Publishes authored
  attribute name via existing AddSettableAtt(value,false,-1). Vault collection
  5b834c31011879cd/class90c3decf0105631e verified constructor828b30e8..310c.
- Also connected immediate manual descendants1327/1350/1351 from native
  updates828f4bb8/5038/50c0: exact37/39/38 ancestor-expanded slots. This prevents
  simply moving the reported predicate/Create failure into its next Update call.

## Physical RevertGround102

This state already had retained constructor storage; it did not have a live update.
Reused that storage and the shared physics routines; no second physical state owner.

Source references: Enter82d86bd8; Update82da0190; CalcRevertTorque82d91390;
UpdateSkateboard82d9c0e0; FillPhysOut82d86c60; Post82d86c58; vtable8234a3cc.
Exit vtable+0c is BLR825187c0. physics_revert constructor82d91230 fallback layout80.
Existing selected Vault class list already includes physics_revert018e8a2e5028ab3f.

Entry enables truck tilt, disables board animation, resets time/start/velocity,
sets direction from Processed.start_revert and Wipeout ground state. Update waits
for native delay6bf6544475d2a5ad, captures start velocity and sign from velocity/deckZ,
then Ground Reckoning, Skeleton GroundSkateboard, Pumping, board error, revert torque,
zero steering tilt, anti-flip, pump force, manual effect(false), three angular
applications and pump queue kind8. Timeout035d9805eb9e2ee2 ends reverting.
Torque uses native PointGraphs X10/Y30 and X60/Y70, signed angle normalization,
revert-direction winding, angular-velocity feedback0e5b939fd4ecdbaa, and original
stopping thresholds. Raw VMX128 VA registers were decoded:82d91458/68 multiply
forward/velocity themselves by their normal dots before subtraction. Do not
silently substitute a conventional normal-plane projection for these instructions.
Start velocity migrated from array to existing SkeletonRootVector to retain
unknown W provenance without inventing W or rejecting otherwise valid XYZ.
Post calls existing CheckForGroundWipeout only (not Ground's extra AI post branch).
Fill writes actual is_reverting into existing PhysOut.State.

Integrated enum, retained-pair checks, transitions100/101->102 and source exits,
selector102, shared grounded Skeleton eligibility, Update3 and Update5 publication.
Selector82ddbd58 preserves off-ground before jump/brake/revert-finished/wipeout
priority and sets revert_exited_normally only on the native finished branch.

## Push investigation and next diagnostic

Fresh source comparison of UpdateContact82b55560, ContactLineTest82b49760 and
GetDistToContact82b53220 found their current contact math consistent with the
checked source. GetDistToContact already rotates its offset by board axes (it is
NOT adding an unrotated world-Y offset). Checked collision RequestPushStop producer
82d9a018/a650 and existing BreakOutOfPush path; no justified timing/strength fix
identified. More than one native branch can produce the reported visual symptom.

Optional `SKATE2_TRACE_PUSH_INTERRUPTION=1` now logs only relevant events:
- actual IK veto: previous query miss, contact beyond reach, or foot/contact gap,
  with foot, offset, target, position and contact information;
- collision-driven push exit dot/threshold/velocity/force;
- BreakOutOfPush clock/length/out-factor when continue flag is false;
- Motion transitions out of the existing Push subtree and already-evaluated
  condition-cache results (does not evaluate extra gameplay predicates).

Compile with build-release.bat, then use the normal game-root setup:

```powershell
$env:SKATE2_TRACE_PUSH_INTERRUPTION = '1'
$env:SKATE2_TRACE_XINPUT = '1'
.\target\release\skate2-pc.exe --live-skating *> artifacts\push-interruption.log
```

Hold X and then A from low speed until the interruption occurs; Escape closes.
Next agent reads that local log and traces the identified producer upstream.
Do not tune contact limits, suppress cancellation, or claim pushing fixed without
identifying the responsible path. Rendering/chunk stutters remain deferred.
