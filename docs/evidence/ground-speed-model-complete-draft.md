# Complete UpdateSpeedModel suffix draft

2026-09-12; all files NEW/unregistered, no Cargo/shared edits. Continues the
source-backed existing-entry prefix documented in ground-speed-model-owner-prefix.md.
The draft now covers a2e0 through the final return aca4. It is not mounted and
does not claim validated Ground force scheduling or bitwise Xbox execution.

## Existing owner reuse and admission requirements

`skateboard_speed_model_update.rs` implements the suffix on SAME
`SkateboardEntryState`, consuming `SpeedModelBeforeJumpContribution`. It reads
existing manual_torque+f4 and writes existing target_speed+e4. No second model,
parallel state, or approximate force controller is introduced.

`skeleton_processed_speed_model_source.rs` borrows actual processed Arc, board
session/backend, assembly owner, existing attributes and live debug references.
It calls existing `try_current_board_effective_transform` at a364 and actual
`try_skateboard_body_mass` at aa4c. Each processed read occurs at its source point.
There is no current-input snapshot and no controlled input provider in this path.

Important existing producer matches:

- spinning frames+924 = `prepare_output.count()`, written by actual PrepareVelocity
  at82e038b4 and reset39528. Its stored u32 bits are reinterpreted as signed i32
  for native cmpw; do not numerically clamp/convert them.
- spin+97c = `jump_events.spin()`, source animation Spin producer and reset39458.
- deck speed968, surface speed96c, surface velocity1a0 and ground normal1d0
  use actual prior-output values; body direction160 uses current Toolkit output.
- cheat's vector+a0 is the retained Processed effective transform Z, distinct
  from the actual board effective transform captured at a364.
- balance9a0 and time-since-input9bc use the existing retained current fields.

`skateboard_speed_model_attributes.rs` is a proposed child of existing
slide/collision attributes. It reuses the SAME friction global in
`SkateboardSlideCollisionVaultAttributes` and SAME manual global in constructor
`SkateboardAttributes`, including their pointer capture/value paths. The only
new capability is `ConservationAttributes`, once bound from actual Vault to
classa099c166f4be639f/defaultd7edbd362d7d2152, source global+c4. Parent must retain
it once on actual ForceRuntime; no per-call binding is permitted.

Remaining shared integration prerequisites are narrow registrations, the earlier
source-backed reset-target flag field, retained conservation capability and
cvars120..123. The actual pool caller must prove the same board/processed/owner
identities and split existing runtime/constructor attribute borrows; this draft
does not add a generic outer orchestration. All native numerical source inputs
have existing owner candidates. No forced-ground/zero/manual-inactive fallback.

## Source order and numerical discriminators

1. a2e0 samples8c8 bit1. If set, conservation keyb67e7c6e17589d9d lookup/value
   then multiply by the captured DT. Otherwise contribution is source +0.
2. a364 captures actual effective transform. Read its Z, current surface velocity,
   dot(surfaceVelocity,Z), then current normal. Scale Z by first dot, dot against
   normal, subtract separately rounded normal projection, and NormalizeSafe with
   actual full-zero fallback. Keep every available W; unknown W stays unknown.
3. Conservation gravity key58ca8124b6ca6d74; negate normalized direction.Y with
   actual VXOR sign flip, then multiply. Conservation limitf4eb6567aba4d0f9.
   ManualFrames cvar TARGET capturea4e8 precedes count924 a4ec; VALUEa4fc follows
   the lower-bound subtraction. Source fsel lower/upper clamp and multiply DT
   precede the frame-count branch. Do not replace the clamp with min/max.
4. Manual branch reads existing torque before balance, even when torque is zero.
   Zero torque merely supplies factor0; it does not skip direction, attributes or
   spin reads. Balance NOT GT zero chooses -1 including NaN. BodyZ dot projected
   direction negative flips this sign by multiplication with -1.
5. Manual keys a98ce2a919e80790 and b4ef1a99b05babf6 precede fresh abs(deckSpeed).
   Both thresholds use raw NOT GT (`4099000c`) to select factor1; unordered
   therefore selects1. Read Spin and square before keyb52d8ff06890b800.
   Multiply coefficient*spinSquared, then positiveFactor, sign, torqueFactor,
   each separately rounded; only ordered-negative result multiplies negativeFactor.
   Add manual result BEFORE existing starting target, matching a6f4 operand order.
6. Capture actual selected surface before fresh speed; then capture layout and
   evaluate PointGraph8 X60/Y80. Negate curve, multiply DT. Positive slope adds;
   nonpositive/unordered slope uses the actual fsel(change-slope,slope,change).
   Add special contribution then starting/manual target in native order.
7. Existing friction keya397ebf01bdd6c3c, then fresh time-since-input. Ordered GT
   executes idle graph: capture friction layout0x160 before speed; PointGraph8
   X90/Yb0. Fresh nonzero balance (including NaN) similarly evaluates manual
   friction graph Xe0/Y100. Both use fnmsubs: `-(curve.mul_add(DT,-target))`.
   Final negation is required for exact-zero sign, not `(-curve).mul_add(...)`.
8. fsel(target,target,+0) then actual SAME target_speed storea860 occurs BEFORE
   CheatEnabled target capturea868. Difference/clamp arithmetic surrounds its
   later valuea87c. A missing cvar at this point preserves the committed target.
9. Enabled cheat tests BE byte8cc bit1 => full-word01000000; groundNormalY must
   be ordered >0x3f266666. Read surface velocity then Processed effective Z,
   dot3, THEN capture/read CheatSpeed. Multiply by exact0x3e8e38e4. If dot<0,
   fresh surfaceSpeed<1 flips speed by multiply-1. Clamp speed-absForward to
   [-0.4,+0.4] using original fsel pair. This changes force delta only; no
   second target-speed store occurs.
10. The captured ShouldApply result gates only coefficient lookup. If false,
    coefficient+0 but still actual Body.GetMass and subsequent arithmetic.
    Ordered-negative delta selects conservation key5c505baa91bbf3c6; other values,
    including NaN, selectf728404cf0e1c5f1 (raw9bc NOT LT).
11. Actual mass, separate mass*coefficient then *delta. World output zeroaad0,
    local zeroaadc, scalar divide DTaae0, second identical local zeroaae4.
    Dot(captured actual Z, projectedDirection). Ordered positive selects Z;
    otherwise VXOR sign-flip EVERY known lane of Z. Scale all available lanes
    by magnitude and publish world forceab50. Late numeric failure preserves
    the zero output prefix; late debug failure preserves the final force.
12. SpeedConservationDebugDisplay target/value after force; enabled requires actual
    overlay, no drawing calls. Repeated source failed assertions map to the
    existing explicit missing-overlay error boundary.

## Live cvar registrations proposed

Parent reserved indices120..123 after the earlier118/119 Pumping refs.

|Slot|Ref|Init|Table|Section / label / title|Default / limits|
|---|---|---|---|---|---|
|120|8308dd3c|82f7b648|82fd4450|Physics_Cheats / kSpeedCheatSpeed / Speed Cheat Speed (kph)|90.0 bits42b40000; 0..200|
|121|8308dd68|82f7b6a8|82fd4454|Physics_Cheats / kActivateSpeedCheat / Activate Speed Cheat|false|
|122|8308dcb0|82f7b6f8|82fd4458|Physics_Manual / kManualForceMaxFramesSpinningTheSameWay / max spin frames|30; 0..120|
|123|8308d9ec|82f7b750|82fd445c|Physics_SpeedConservation / kSpeedConservationDebugDisplay / Draw Debug Info|false|

All callbacks NULL: float r10=0 atb684, bool r8=0 atb6cc, integer r10=0 atb71c,
debug bool r8=0 atb774. Decompiler float ABI incorrectly shows extra callback
arguments; raw registers prove null. `input_cvar_speed_model.rs` retains captured
live Target objects across source count/arithmetic stages. Parent registry/oracle
edits remain pending. Original source order is120,121,122,123, before existing
forced-surface113/table4460; GUI title ordering is independent.

## Draft validation scope

Six unexecuted tests cover ordinary target/force and late-read order, false
permission still updating the model/reading mass, target versus force partial
writes on later errors, zero manual torque still consuming its actual inputs,
negative-zero fnmsubs target and active cheat changing force without rewriting
target. Earlier prefix has four separate tests. All new files are formatted;
no compilation, original corpus or native numeric parity claim is made yet.

Original external raw SHA-256, exclusive end:

-82b2a2e0..82b2aca4: 06bedab30cea023b1602986866f084f38e053ec11d533747ee551f5b48a675a8
-82f7b648..82f7b798: d23f4089868d1975ce0f53d815a9807efbb05c1874573be0eee8d9d31318bd40

Root independent review: fresh raw remainder a2e0..a35c/a714..a850/ab58..aca4 and globalConservation ctor828b2680..26b8 match selected source semantics and graph offsets. Earlier main numeric ranges reviewed separately. NormalizeSafe callback correction and repeated local-zero store remain required at registration.
