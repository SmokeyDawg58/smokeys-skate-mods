# Ground UpdateSpeedModel: actual owner and first continuation

2026-09-12 independent source audit and new unregistered draft. No Cargo or
registered mutations. This is a bounded prefix, not the full force calculation.

## Owner and exact initial route

Ground.UpdateSkateboard82d9a018 calls UpdateSpeedModel82b2a240 at a710, between
RearFootForce a704 and SlideFriction a720. Hidden output is r3; real board r4.
The existing `SkateboardEntryState` already owns native targetSpeed+e4 and
manualTorque. Its ResetSpeedModel82b2a1e0 already computes the actual initial
speed from live Part transform and velocity. Do not create a generic model or
another target-speed scalar.

At a268 it calls ShouldSpeedModelApplyForce82b23e38. Raw source:

- Read one Processed8c4 word; flags02000000,40000000,20000000,00010000 inhibit.
- Only if all are clear, read8cc and test20000000.
- **Always read numWheels90c at3e88**, even if an earlier flag inhibited force.
  Signed wheel count<2 inhibits. Decompiler's short-circuit expression is wrong.
- This result controls a later force coefficient only. False does NOT skip
  target-speed calculations or state updates.

The new `skateboard_speed_model_prefix.rs` is a child of the EXISTING entry
owner. After the complete permission calculation, it captures actual board
reset-target flag530/80000000, selected Processed surface904, signed Z speed944
(then fabs), DT93c, and only then selected surface layout+104. Surface scalar
comes from the existing retained selected collection, with layout fallback
0x10c; no per-call Vault binding. New surface reader and Contact source adapter
provide those staged reads without broad readiness gating.

With speed s and maximum difference d, raw a2a4..a2c8 executes:

```
lower = s-d
upper = d+s
lower = fsel(-lower, +0, lower)
bounded = fsel(lower-oldTarget, lower, oldTarget)
startingTarget = fsel(upper-bounded, bounded, upper)
```

Each subtraction/addition is separate; do not replace with clamp/min/max.
If reset-target was set, native a2d4 stores actual s into SAME targetSpeed,
then a2dc clears the bit; startingTarget=s. In the ordinary branch, this prefix
does NOT publish startingTarget. Full target publication happens much later.
The returned token retains permission, s, DT, d and startingTarget for a2e0.
Output force has not yet been written at this boundary.

Four unexecuted draft tests discriminate unconditional wheel reads, source
surface/read order, collision-target prefix stores, no ordinary target store,
failure preservation, and negative/NaN bounds.

## Constructor flag proof and proposed amendments

Fresh original raw proves:

- 82b38b50 load Board+530;38b54 clrlwi1;38b58 store: clear80000000.
- 82b38bfc load Board+530;38c00 rlwinm0,2,0;38c18 store: clear40000000.

Thus actual constructor establishes reset-target=false and abort-push=false.
Existing entry.abort_push=None is an earlier incomplete projection, not source
uncertainty for this completed constructor. Parent was notified immediately;
no registered fix was applied by this audit. Preserve other unknown bits and
the distinct earlier/later constructor store positions.

Proposed narrow amendments, still unregistered:

1. Retain actual reset-target bit on existing Entry, false only when its source
   constructor store executes. Set it in real collision response at a5ec;
   consume/clear it at SpeedModel a2dc. No duplicated full flag word.
2. Correct existing abort constructor knowledge at actual38c18, preserving its
   existing Update3 clear and forthcoming PushAcceleration set.
3. Register entry::speed_model_prefix and tests, Contact::speed_model_input,
   selected-surface attributes::speed_model_surface.

The draft refers to proposed `entry.reset_target_after_collision`; that field
is deliberately not added to the frozen registered owner. Compilation and
same-Actor mounting await parent admission.

## Remaining exact SpeedModel continuation

Full function ends82b2aca4. Decompiler loses important scalar operations, so
the following is a source/owner map, not a finished numerical implementation:

| Source stage | Required actual inputs / reuse |
|---|---|
|a2e0..a358|Processed8c8 bit1; optional global speed-conservation+c4 keyb67e7c6e17589d9d, multiply by captured DT|
|a364..a3c0|Existing live board effective transform; current surface velocity1a0, ground normal1d0; project then shared NormalizeSafe with +0 fallback|
|a414..a4c4|Two speed-conservation scalar lookups; slope acceleration and bound with exact fsel graph|
|a4cc..a6f8|Live ManualForceMaxFramesSpinningTheSameWay; Processed frame count924, SAME entry.manual_torque, balance9a0, direction160, original manual global scalars and spin inputs. May not presume manual inactive|
|a718|Actual selected surface graph X60/Y80 (8 points), sampled surface speed; preserve captured selected instance|
|a784..a7f0|Existing friction global: input-idle scalar gate, then cached graph+80 metadata / X90,Yb0 at fresh surface speed|
|a800..a84c|Fresh balance gate; friction graph+d0 metadata / Xe0,Y100 at fresh surface speed|
|a850 onward|Publish final nonnegative targetSpeed to existing+e4, then bound actual-minus-measured speed difference|
|a858..a9a8|Live ActivateSpeedCheat and optional SpeedCheatSpeed, BE-byte8cc bit1 (fullword01000000), groundNormalY>.65, surface velocity versus effective Z, direction sign tests. These source branches remain required even with source-default cheat disabled|
|a9ac..aa44|Captured ShouldApply result chooses live positive/negative speed-conservation coefficient, or source zero coefficient; never skip preceding model updates|
|aa4c|Actual Body.GetMass via existing constructed board owner, not a new mass constant|
|aa50..ab2c|Output zero prefix, effective-Z orientation sign by dot against projected direction, mass*coefficient*boundedDelta/DT, full-lane output multiply|
|ab30..ac88|Late SpeedConservationDebugDisplay and actual overlay requirement; source repeated failed-assert checks are explicit error boundaries|

Global speed-conservation+c4 has no existing compatible retained read capability.
Fresh ctor828b2680..26a8 binds classa099c166f4be639f/defaultd7edbd362d7d2152,
store+c4 at26b4. A future capability must bind once, alongside existing selected
global projections. Existing friction, manual/global and surface owners should
be reused or narrowly extended; no duplicate generic tuning subsystem.
No new cvar slots were reserved for this prefix.

## Independent WallRide integration review

Read-only review found no defects in root's registered integration: global+e8
capability binds once on SAME ForceRuntime; cvar117 retains its real live target
and correct metadata/table order; Ground.jump_direction is the SAME typed field;
GroundExit applies gravity FMA to every known lane before XYZ magnitude clamp;
same-Actor corpus invokes WallRide immediately after GroundControl and retains
owner+force before FakeTorque. This does not certify the rest of the force frame.

## Raw reproduction

SHA-256, original external IDA bytes, exclusive end:

| Span | SHA-256 |
|---|---|
|82b23e38..82b23ecc|7c9e160d8bb4c512db4ff6a9ae7b1fb55811ec9be29fe64289d30e3cabb3d137|
|82b2a240..82b2a2e0|980c05919576bd547da3e438b2de42cf3671036997f7cc27638041d7f6860d03|
|82b38b50..82b38b5c|2fc367d098657fce49d423cfb18cfec462d50d43d76737f8dee368bb775a14a2|
|82b38bfc..82b38c1c|8cf38dca771a43c110658455f005b26eeea080a5412e9fa49d456ad1f8da0ce5|
