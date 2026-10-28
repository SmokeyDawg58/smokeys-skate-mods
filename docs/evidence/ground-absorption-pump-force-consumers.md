# Actual absorption and pump-force consumers

2026-09-12. New unregistered drafts only; no shared registrations or Cargo.
These leaves continue PhysicsGround's actual force calculation sequence; they
do not implement the outer force queue or complete the controller/frame.

## Ownership and caller positions

Ground.UpdateSkateboard82d9a018 calls AbsorptionFootForceMag82b28c60 at a6d0
after Braking a6bc and PushAcceleration a6c8. Its returned scalar is the real
argument to FrontFootForce at a6ec. PumpForce82b28ed8 is called at a72c after
SlideFriction a720; its returned force is later queued at a920.

Both read the existing SAME Pumping allocation referenced by HiLOD+2c0 and
board+108. Existing `PhysicalPlayerPumpingOwner` already contains absorption+38
and acceleration+30, written by the complete earlier Pumping.Update. New child
`physical_player_pumping_force_read.rs` adds only two read-only accessors; it
does not allocate/reset or create a second state. Constructor null remains an
error at the actual consumer. `PumpingForceRuntime` borrows that exact Arc.

Absorption reuses existing `runtime.foot_attributes` (global+bc). PumpForce
reuses existing `PhysicalPlayerAssemblyOwner::pumping_attributes()` (global+f8,
classab3a006f3a2956e2/defaultd7edbd362d7d2152). There is no new binding.
Its keyed pointer captures and value reads stay separate.

## Absorption82b28c60..8d94

- Crouch9d4 at8c80; EQ zero8c88 enters calculation without reading delta.
  Otherwise delta9d8 at8c8c, ordered LT zero8c94 enters calculation. NaN crouch
  is nonzero; NaN delta does not satisfy LT.
- The other arm requires the actual DebugOverlay singleton at8c98..8ca4,
  without reading any debug cvar, and returns +0. No drawing call exists.
- Calculating arm captures feet key `e4864a9453f41775` at8d00, null-only
  DefaultDataArea(4) fallback, loads value8d18, reads same Pumping.absorption+38
  at8d24, then separate fmuls8d28. Board's Pumping pointer load8d14 precedes
  the value load; typed runtime retains this same immutable Arc identity.
- Live kDrawAbsorptionCalc target/value8d2c/30 occurs BEFORE fneg8d38 and
  fsel8d3c. Result is `if -product >= 0 { +0 } else { product }`: negative and
  either signed zero return +0; positive returns product; NaN retains product.
- Only if the captured debug value is enabled does the calculating arm require
  the singleton. Source failed-assert retry is represented by explicit error,
  consistent with other actual overlay consumers; no fake overlay is supplied.

## PumpForce82b28ed8..90f4

The decompiler drops all important scalar computations. Raw source confirms:

1. Hidden result r3, real board r4. Read Processed8cc word8f04, then SAME
   Pumping.acceleration+30 at8f08. Extract player-controlled-pump bit4.
2. If bit4 is clear: live DrawPumpForceIntention target/value8f20/24, optional
   actual overlay requirement, global+f8 pointer lookup key
   `99a7684b304ba44d` at8f94, null-only DefaultDataArea(4), scalar8fa8,
   multiply scalar*capturedAcceleration8fac. Set bit skips all those reads.
3. Fresh Processed: mass970 at8fbc, separate multiply8fc0, DT93c at8fc4,
   scalar division8fc8. No zero-DT shortcut or reciprocal estimate replaces it.
4. Global+f8 lookup key `b9cabbb88c47204b` at9004, null-only DefaultDataArea(1).
   Actual lbz901c reads a boolean: any nonzero enables, zero substitutes +0
   magnitude902c. This read occurs after mass/division even if disabled.
5. Caller output writes world force full +0 at90a0 and local position +0 at90a4
   and90a8. Earlier failures preserve the caller's old output. Later missing
   velocity/numeric failure preserves these committed zero writes.
6. Fresh Processed ZAxisVelocity+1b0 passed to Math.NormalizeSafe824b18e0 at90b4,
   fallback full+0. The existing shared normalization body is reused with its
   explicit host numeric collaborator. This happens even if disabled.
7. Full normalized vector multiplied by magnitude90e4, world force stored90e8.
   Successful normalization preserves unknown W; the actual zero fallback makes
   W known zero. A disabled scalar does not authorize unknown-W promotion or
   skipping normalization (nor sanitizing infinities/NaNs).

The live input adapter reads flags8cc and mass/DT from current Processed fields,
and vector1b0 from `toolkit_state().z_axis_velocity()`. It uses neither surface
velocity1a0 nor already-normalized direction1c0 as a substitute.

## Proposed registrations and cvar proof

Parent reserved118/119; current registered batch remains unchanged. New files:

- force_runtime child `skateboard_pumping_forces.rs` + five focused leaf tests;
- Pumping child `physical_player_pumping_force_read.rs` + same-owner read test;
- constraint-session child `physical_player_pumping_force_source.rs`;
- InputCvar child `input_cvar_pumping_forces.rs`.

| Slot | Reference | Initializer | Native table | Label/title |
|---|---|---|---|---|
|118|8308d834|82f7b278|82fd4424|kDrawPumpForceIntention / Draw Pump Force Intention|
|119|8308dddc|82f7b2c8|82fd4428|kDrawAbsorptionCalc / Draw Absorption Calc|

Both section Physics_Skateboard; boolfalse and nullcallback, proven raw r7=0/r8=0
atb29c..2a4 andb2ec..2f4. Table4424 precedes4428, both after WallRide4414.
GUI title sort places Draw Absorption Calc before other section's Draw labels,
and Draw Pump Force Intention before Draw Wall Ride Info. Reserve/store arrays
and source/title oracles must be updated when parent admits registration.

Tests cover source lazy paths, unconditional inactive overlay, negative/NaN
absorption selection, pump input/attribute order, disabled-but-still-normalized
force, unknown W versus genuine fallback zero, and old output preservation on
early failure versus committed zero output on late failure. These drafts were
formatted but not compiled/tested; no live/frame completion claim is made.

## Original raw hashes

SHA-256 of original external IDA bytes, exclusive end:

| Span | SHA-256 |
|---|---|
|82b28c60..82b28d98|eda1bb3d5a53a76539ee906b155ca0ada4ea01b3b9c5251144d2022ef7549d5d|
|82b28ed8..82b290f8|96c75d7d90964eb28f78bd71da28bf159571ab72ea9f9b7ce0c9e961551b06dd|
|82f7b278..82f7b318|8bfa4093764756880e38647a732e033bc6d130776a9fb884c567917a5f8c4b0e|
