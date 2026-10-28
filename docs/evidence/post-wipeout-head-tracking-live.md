# Post-wipeout HeadTracking mandatory invalid-target update

2026-09-13. Unregistered bounded draft, source reviewed against fresh raw
HeadTracking.Update82dca850..82dcb2a0. Root registration, independent review and
Cargo gates remain required. This executes the complete invalid-target,
debug-disabled callback, including its mandatory PID and integration. A valid
target stops at82dca998; enabled debug stops at82dcb128 after output publication.
Neither stop is a successful callback or a substituted zero target.

## Files and root edits

New files checked absent before creation:

- physical_player_post_wipeout_head_tracking.rs: child
  `skeleton_head_tracking::post_wipeout_update`.
- physical_player_post_wipeout_head_tracking_runtime.rs: sibling child
  `skeleton_head_tracking::post_wipeout_live`.
- physical_player_post_wipeout_head_tracking_tests.rs: test child of kernel.

Root shared changes in skeleton_head_tracking.rs:

1. Remove the constructor-only HeadRotationFilter struct and its dead_code
   expectation. Add imports `reckoning_pid::{PhysicsControllerData,
   VectorPidFilter}`. Change `filter` to `Option<VectorPidFilter>`.
2. Preserve the existing constructor's cached-layout lookup and store position.
   Replace only construction of the filter with:

```rust
self.filter = Some(VectorPidFilter::new(
    PhysicsControllerData {
        proportional: control[0],
        integral: control[1],
        derivative: control[2],
        integral_blend: control[3],
    },
    [0.0; 4],
));
```

3. Register both children with their explicit path attributes. No duplicated PID
   state or reset/copy bridge is required. Existing VectorPidFilter.update is
   the actual82dc8f20 graph, preserves all four lanes, and has replace_control.

Actual call:

```rust
head.try_post_wipeout_update(
    world_to_animation,
    &captured_head_transform,
    &mut axis_angle_out,
    cvars,
    numeric,
)?;
```

`head` is the SAME constructor.head; call after the selected foot/hand IK and
mandatory IK.PostPhysicsUpdate. Caller82b57c18 loads Skeleton+3e1c, which is
the existing init.named_bones[6] Head bone lookup. At57c44..74 it indexes the
actual animation-bone buffer Skeleton+2ca4->0, copying all16 lanes. This is not
the 24-part PhysRecord or mapped animation-volume array. Supply the original
full pose buffer retained by the actual producer, not an invented Head Part
transform. The worldToAnim argument at57c40 is actual Skeleton+2dc0. The
caller initializes axisAngleOut to fullzero before copying the head matrix,
then calls57c78. Continue at57c7c; the callback alone does not publish final pose.

The kernel takes actual matrices even for invalid target, although that arm
does not use their coordinates. Shared immutable borrows preserve captured
input identity; unknown W is neither rejected nor promoted. `axis_angle_out`
is a mutable caller local because native b114 writes it BEFORE debug and final
valid-target clearing, and a late failure must retain that publication.

## Cvars

Root reserved215..219. All use category `Physics_Skeleton_HeadAdjust`, null
callback. Ref addresses, native initializer/call, labels and titles:

|Index|Ref|Initializer/call|Label|Title|Default/range|
|---|---|---|---|---|---|
|215|830db008|82f97a20/7a4c|kDebugForceLookAt|Debug Force Look At|false|
|216|830db2f0|82f97b20/7b60|kHeadTrackingMaxAngleToLookAtTargetXYZ|Max Angle To TargetXYZ|60,0..180|
|217|830db22c|82f97a70/7a9c|kHeadTrackingClampIdeal|Clamp Ideal|true|
|218|830db084|82f97ac0/7b00|kHeadTrackingLargestUpAngle|Largest angle to look down|45,0..180|
|219|830db640|82f97980/79ac|kDrawHeadAdjustInfo|Draw Head Adjust Info|false|

The bounded runtime reads215,216,219 at actual stages.217/218 belong to the
explicit pending valid-target branch and are not sampled by invalid targets.
No force/look/debug bool is supplied as a hardcoded runtime default.

## Native state and sample order

HeadTracking own Instance class58dbc2815ca666fd/defaultd7edbd362d7d2152 is the
existing constructor owner. a8a0 captures cached layout+4; a8d8/a8e8/a8f0 capture
head I/K/J. a904..a928 copy live controller layout+20/+24/+28/+2c to PID+10.
There is no per-call controller reset. Constructor-installed DefaultDataArea48
is retained when cached layout is null; the adapter reflects its genuine zeros.

a92c..a944 sample force-look and max-angle cvars. Max-angle is read and multiplied
by exact degrees-to-radians3c8efa35 (823557f4), even when valid_target=false.
With force enabled, a968 fetches own keydb6c599d0f2bece1 (DefaultDataArea16 on
absence), a984 writes full target+70 and a988 writes valid_target+ a0=true.
Only then a98c reads validity. Those writes survive the pending valid-target
orientation error. The pending branch cannot silently fall through to zero.

Invalid target branches directly toafc4 with original fullzero target local.
afe0 calls existing VectorPidFilter.update. b000 then fetches own key
ccafd906f9356d1e (DefaultDataArea4 on absence), and b028 reads its scalar.
Hence a missing/malformed cap preserves the complete PID mutation.

Corrected raw VMX128 register decoding of b048/b06c yields:

```text
delta = filtered - oldRotation
acceleration = delta - oldRotationVelocity
length = Length(acceleration)
direction = NormalizeSafe(acceleration, fullZero)
selected = FSEL(length - cap, cap, length)
base = oldRotation + oldRotationVelocity
next = FMA(direction, selected, base)
rotationVelocity = next - oldRotation
rotation = next
axisAngleOut = next
```

Length b070..b0b0 executes dot3 and TWO inverse-square-root refinements even
at zero, then zero-selects. Reuse reckoning_numeric.try_length. NormalizeSafe
b0b4 is a separate dot3 and conditionally requests its estimate; reuse existing
skeleton_ik_line_frame.normalize_safe. All scaling and subtraction retain W,
while dot3 excludes W. FSEL b0e4 uses ordered length-cap>=0, so NaN selects
length; it is not f32::min. Negative caps are preserved. There is no dt factor.
Velocity b10c, rotation b110, and caller output b114 are three ordered stores.
Debug cvar sampled b118..124 follows them. Finally b28c clears valid_target.

Enabled debug executes additional length/reciprocal/sincos calculations before
requiring DebugOverlay and two native BLR text-draw calls. This bounded draft
reports DebugArithmeticPending atb128, preserving prior PID/rotation/output
stores and not clearing validity. It does not pretend that branch completed.

## Pending valid-target math, no generic replacement

a998..aa7c transforms the actual target by worldToAnim, subtracts captured head
P, normalizes, and builds two normalized cross-product directions using actual
head I. aacc/aae4 also execute two separate horizontal-normalization calls;
their results are overwritten but their numeric samples still occur. aae8 uses
head J dot normalized target direction for native acos, followed by floor-turn
angle wrap. Strict max-angle>abs(angle) permits the remaining branch.
ab50 samples ClampIdeal; its nested branch samples LargestUpAngle and calls
ClampVectorWithinAngleRange. ad24..af28 builds the relative basis and converts
to quaternion with a specific vector selection/refinement graph; af2c..afa4
converts to axis-angle, including I fallback and acos. afac/afb8 freshly load
layout max/min vectors and clamp the four-lane target before PID. Ordinary
simulation quaternion-from-matrix code is not an established equivalent.

No current producer may fabricate a target transform to bypass this boundary.
The real constructor produces valid_target=false, so that first-frame path
can execute every mandatory callback stage implemented here.

## Validation limits

Five synthetic tests cover constructor-invalid callback sampling/publication,
live controls and debug-force partial writes, PID retention after cap failure,
acceleration-cap integration including W, late debug stop, negative cap and
NaN FSEL behavior. They are source-order/numeric checks, not gameplay replay.
Standalone rustfmt passed; no Cargo by author. Existing numeric host boundaries
remain: Xenon estimate/reduction rounding, denormals, NaN payload and FP status.
