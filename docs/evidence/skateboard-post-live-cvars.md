# Live board post-physics targets and caller

New-only `input_cvar_board_post_live.rs` implements the existing
`SkateboardPostCvars` interface over the actual retained `InputCvarReferences`.
Capture clones the existing target Arc without reading its scalar; the later
read locks that captured target. Forced surface uses existing index113 at its
actual late read. No default/provider fallback or new cvar manager is created.

Root reserved154..156. Fresh raw initializer review confirms all float callbacks
are NULL (`li r10,0` at82f7b82c/b8ec/b99c); Hex-Rays's buffer pointer is wrong.
The native GUI bounds belong to registration evidence and must not become
runtime clamp operations. Proposed appended definitions:

```rust
//154 ref8308dcdc init82f7b7f0 table82fd4464, GUI0..1, null callback
Definition {
    section: "Physics_Skateboard",
    label: "kMinDPWithZToBeConsideredDeckEnd",
    title: "DP Skateboard Contact",
    initial: Value::Float(f32::from_bits(0x3f6b_851f)),
},
//155 ref8308da60 init82f7b8b0 table82fd446c, GUI0..3dcccccd
Definition {
    section: "Physics_Skateboard",
    label: "kFreeSkateboardWheelInertia",
    title: "Free wheel inertia",
    initial: Value::Float(f32::from_bits(0x3d23_d70a)),
},
//156 ref8308d8ac init82f7b960 table82fd4474, GUI0..3f000000
Definition {
    section: "Physics_Skateboard",
    label: "kSkateboard_LineDistToUseNormal",
    title: "Line Dist For GNormal",
    initial: Value::Float(f32::from_bits(0x3d8f_5c29)),
},
```

Increase count154->157, register child as `board_post_live`. In current native
registration array replace `123,113,28,29` with
`123,113,154,155,156,28,29`. These reflect table4460,4464,446c,4474,4480,4484;
the intervening unimplemented references must not be inserted as fabricated
entries. GUI-title oracle requires DP Skateboard Contact before the existing
Draw Absorption entry; Free wheel inertia and Line Dist For GNormal follow the
existing Physics_Skateboard Draw entries. Exact oracle edits: insert
`kMinDPWithZToBeConsideredDeckEnd` after `kGroundMaxCosAngleDelta` and before
`kDrawAbsorptionCalc`; insert `kFreeSkateboardWheelInertia`, then
`kSkateboard_LineDistToUseNormal` after `kDrawWallRide` and before `kDotToStopPush`
(whose native title is Stop push DP).

Deck-end target capture36d28 occurs before bodyZ/vector dot36d34; scalar load is
36d38. The adapter preserves that distinction through existing kernel callbacks.
Line fallback captures its target before actual wheel distance and reads its
scalar afterward. Free-wheel target/value37148..54 is reached only when actual
Body+2098 bit80 is set; the adapter does not eagerly read it at construction.
Forced-world-surface reuses the existing actual enum target and raw-bit getter.

Focused new test links the actual manager, captures all three targets, reloads
their values, then relinks a different manager. Captured targets must retain old
identity with updated values; newly captured targets see new defaults. Forced
surface must follow the currently linked target and preserve negative raw bits.
The test is unregistered/unrun pending root integration.

## Concrete caller sequence and remaining admission

Fresh raw outer `Skateboard.UpdatePostPhysics`82b37d20..37e8c:

1. Actual virtual Body then EndWheelLineTest37d48.
2. IsBadData82b2b398 at37d50. True sets outer101, initializes transforms,
   TeleportReset from actual Processed+40, return-code2, then queue clear.
3. Otherwise HasAcceleratedStrangely82b26070 at37d8c. True additionally reads
   outer102; only zero enters InitializeTransforms/TeleportReset from
   Processed+80, SetVelocity zero, return-code1, root angular velocity (mOmega+30) update,
   then queue clear.
4. Ordinary branch37e20 captures actual Reckoning pointer+104 and Processed
   pointer+110, loads actual Reckoning up+1b0 at37e3c, then dt+93c at37e40.
   Calls Body.UpdatePostPhysics37e5c, UpdateUpVector37e64, then queue clear
   37e78/7c/80. The up captured here is not GroundNormal or a prior-output copy.

Existing concrete callee chain, using the same retained pair/owners:
`try_update_board_post_ground_normals(board,dt,up,&mut BoardPostCvars::new(refs),
numeric,rounding)` -> `try_finish_board_postphysics(board,truck_numeric)` ->
`try_finish_board_ordinary_up_post(board,skeleton,&actual_processed,numeric)`.
The first already calls Body prefix internally and includes surface choice;
do not also invoke either standalone prefix or ChoosePhysicsSurface.
The last performs actual queue clear. Use short locks for initial Reckoning/up
and dt capture. Its Processed borrow may live through the actual UpdateUpVector
input adapter; do not retain a contact-state guard across this call.

This is not yet a complete outer caller. Repository search found no live
IsBadData/HasAcceleratedStrangely caller. The latter requires two additional
source cvars kMinBadVelY and kMaxGoodVelY (not reserved or implemented here),
current actual deck velocity, and Processed body velocity+190. The second cvar
is short-circuited by the first comparison. Outer102 must be read only if it
returns true. Do not mount the ordinary path as an unconditional claim of native
branch selection. A selected component-callee invocation remains explicit until
these predicates and exceptional owners are supplied. Root owns further scope.
