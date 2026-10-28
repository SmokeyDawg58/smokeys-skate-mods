# Actual board force-displacement record

2026-09-13. Confidence high for source graph; independent review
accepts implementation. New integration validation pending.

Sk8::Physics::Skateboard::StoreForceDisplacement82b23830..82b2390c is called by
HiLOD.Update3 at82dfe820 after PrePhysics. SAME board+10 deck body supplies
+a0 torque/cooldown and +90 force/kinematic value. It reads neither inertia nor
omega. Native DT at8225c488 has bits3c888889. Permute tables8225c4b0/c0 select
ZXYW/YZXW. For each lane, retain:

    residual = -fma(+0, torque.ZXYW, -force)
    effective = fma(+0, torque.YZXW, residual)
    partial = (effective * dt) * dt
    board.expected_displacement = partial
    fresh deck velocity = board.deck_body.velocity
    board.expected_displacement = fma(velocity, dt, partial)

Both stores are outer board+60 (238f0/23904). Standard VMX fields at238bc and
d0 use VA*VC with VB additive. VMX128 raw173cd8b5 decodes VA60, not IDA v92;
14b9caf1 decodes VA57, not v89. Do not simplify zero*torque: nonfinite torque
propagates. Do not collapse dt products; force bits3f000004 distinguishes
source3911a2b9 from collapsed3911a2ba. Preserve outer-negated fused subtraction.

physical_player_board_force_displacement.rs is a skater forces child accessing
actual same Part6/body and runtime. Existing body snapshot mapping supplies
force W=mKine and torque W=raw cooldown. Fresh native velocity W is a pointer;
portable actual adapter retains unknown W, not an invented pointer/zero. First
store has known W and survives a checked fresh velocity-read failure. No body
force/torque or queue mutation occurs. Corpus calls after ApplyForces and before
actual solve; caller remains selected until full Update3 closes.

Tests cover finite/full-lane math, two-product rounding, negative-zero FMA,
nonfinite torque, late-read prefix preservation and unknown pointer lane.
Xenon denormal, NaN-payload and FP-mode equivalence remain unverified host numeric
boundaries; no instruction translator or guest CPU runtime is introduced.
