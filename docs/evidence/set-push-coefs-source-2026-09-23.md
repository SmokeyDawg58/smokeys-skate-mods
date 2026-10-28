# SetPushCoefs source and isolated callback

The original-data raw-A facade replay enters the `Default` push child and
stops during Motion `enter` at occurrence 2442, `SetPushCoefs::CreateInstance`
`0x82cba730`, after `ComputeFirstPushStrength` Create and Begin. The replay
has **not** yet called either child's Update; reaching the next Create proves
only the earlier Create/Begin path returned. The source binding uses factory
`0x82c91108`, behavior allocation `0x20`, callbacks Create `0x82cba730`,
Begin `0x82c95bf8`, Update `0x82ca92f0`, and the shared single-`blr` End
`0x825187c0`. IDA names the shared Create body
`CreateMGIntentFromAGIntent::CreateInstance`; its actual retained behavior
payload is a `0x0c` instance with byte +8 initialized to 1. Begin writes that
same byte to 1. Constructor `0x82c8d7f0` reads `onFirstUpdateOnly` with false
default; occurrence 2442 has no attributes, so it is false. The Update thunk
calls `SetPushCoefs::Ramp` `0x82ca00b0` every tick for this occurrence, then
clears byte +8. For other authored instances, true gates Ramp to first Update.

Ramp validates the cached `ISkaterMotionGraph`, `IAnimatable`, and `IPhysical`
owners before reading the retained PushState through graph virtual +0x38.
Native load order is PushState float indices 4, 5, 6, 7, 2, 3. The target
triplet is indices 5..7; indices 2..4 are the current triplet. When index 4
is negative, the target
triplet substitutes for the current triplet. An unordered index 4 retains
the current triplet because the native `fcmpu`/`bge` test does not set LT.
Physical board z-speed comes from
`IPhysical` virtual +0x28 / `PhysOut.mSkateboardMotion.mSkateboardZSpeed` and
is clamped to nonnegative by `fsel`.

The **same** existing `GlobalAttributeCollection.anim_motion_pushing` +0x13c
instance supplies two more cached `PointGraphData<8>` fields from its `0x5c0`
layout. The +0x280 graph evaluates at the current third coefficient; the
+0x2d0 graph evaluates at nonnegative board speed. Their x/y arrays begin at
field +0x10/+0x30. Each graph result is multiplied by source constant
`kUpdateTime_56` `0x3c888889` (bytes at `0x822f2204`); reciprocal of the
product is the ramp rate. After both graph evaluations, Ramp reads the actual
controller delta time. For each of the first two coefficients it computes
`target - current`, checks `fabs(diff) > 0x3a83126f`, and otherwise takes the
target exactly. A positive difference uses fused `dt * rate + current` and
`fsel` saturation at target. A negative difference uses fused
`current - dt * rate` and `fsel` saturation at target. The third coefficient
uses its own speed-derived rate and the same branch pattern. The final
`fneg`/two-`fsel` sequence clamps each coefficient to [0,1] (unordered inputs
select 1 under this branch shape), and stores PushState indices 2, 3, 4 in
that order.

Only after all three PushState stores does Ramp write `IAnimatable`
settable attributes through virtual +0x48, in order `HStr_Vel_B`,
`LStr_Vel_B`, `Vel_E`, with normalized=true and sequence=-1. Those names are
verified by their original static initializers at `0x82f82000`, `0x82f82018`,
and `0x82f81fe8`. The Rust actor bridge reuses the already published Motion
PushState, physical output, Pushing Vault owner, and mounted animation's
`try_add_gesture_attribute`; it does not create another attribute collection.

The next selected child occurrence is 2443,
`ComputeTargetCoefsFromSpeedAndStrength`: Create `0x82bb3da8` is the shared
null-instance callback, Begin `0x825187c0` is a BLR, Update `0x82ca8e18`
still needs a separate source trace and integration before a complete push
Update replay can run. The isolated SetPushCoefs kernel and focused tests
exist, but live parity awaits root lifecycle registration and a replay past
the child's enter sequence. Confidence: high for callback control flow,
storage, graph offsets, and ordered writes; exceptional PPC/host floating
payload identity remains unproven.
