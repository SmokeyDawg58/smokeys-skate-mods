# ComputeFirstPushStrength source and retained owners

The fresh raw-A public facade replay reaches Motion occurrence 2441 in the
`Default` push child state 1279, after `Push` state 1028 and `InitPush` 1932.
The current recorded boundary is `ComputeFirstPushStrength::CreateInstance`
`0x82ca0c90`, not a completed push-speed or animation trace. The generated
binding uses factory `0x82c91048` (behavior allocation `0x1c`), callbacks
`Create 0x82ca0c90`, `Begin 0x82ca0b18`, `Update 0x82cc54b0`, and the shared
single-`blr` End `0x825187c0`.

Create allocates `0x10` bytes. Its payload is elapsed `f32` at +8 (zero) and
flags at +0xc (bit31/bit30 set; bit29 clear); lower bits are not established by
this constructor. Begin sets elapsed to zero, sets bit31/bit30, validates the
cached `ISkaterMotionGraph`, and captures a fresh scalar pointer from the
global `anim_motion_pushing` member +0x13c, key
`0x5860_f565_4c4b_9cdf`. A missing scalar uses `DefaultDataArea(4)` (zero).
It then calls the graph's `TimeSinceTeleport` virtual +0x90, loads the captured
scalar, and sets bit29 iff `time_since_teleport < threshold` (`fcmpu`/`blt`,
unordered false). This pointer-before-virtual-call/value-after order matters.

Update validates `IMotionGraph`, `ISkaterMotionGraph`, and `IPhysical`, then
checks membership of `Skater::PushingIntent` in the **Motion** intent map.
Absence clears bit30. If both bit30 and bit29 are clear, it skips the strength
body and clears bit31. Otherwise bit29 selects the *previous* elapsed value as
push time, then stores `dt + elapsed`; this branch never calls `HasIntent`.
Without bit29 it separately calls `HasIntent(Pushing)` at `0x82cc5748` and uses
its float output (zero when absent). The physical output source is
`PhysOut.mSkateboardMotion.mSkateboardZSpeed`, read through `IPhysical` virtual
+0x28 and clamped by native `fsel(speed, speed, 0)` (negative/unordered to zero).

Update next captures the second fresh pushing scalar, key
`0x9dcc_b3a8_1a4c_a176`, with the same zero fallback, compares clamped speed
strictly greater than it, **then** reads `kCapDvAtTopSpeed`'s live boolean
target. When both are true, push time becomes zero. The CVar is sourced from
initializer `0x82f85fe8`, table `0x82fd4ed8`, ref `0x830a4570`/target byte
+0xcd: section `Animation_Skater`, label `kCapDvAtTopSpeed`, title
`Cap push acc. at top speed`, default false, null callback. It follows
`kUsePumping`'s table entry `0x82fd4ed4`; Rust registered it as stable index
344 immediately after index 238 in the source registration order.

`MapButtonPressToChangeVel 0x82c95328` uses two cached
`PointGraphData<8>` fields of the **same** pushing collection layout (size
`0x5c0`). The +0x230 graph maps speed to duration. Native `fdivs` computes
`push_time / duration`; `fneg` and two `fsel` operations clamp it to [0,1]
(an unordered quotient selects 1). The +0x1e0 graph maps that ratio to delta
velocity. Each graph's eight x/y words begin at field +0x10/+0x30. Finally,
`ISkaterMotionGraph` virtual +0x38 returns its retained PushState; `fcmpu`
and `ble` write delta velocity to `PushState.mCurrPushDv` (+4) unless the new
value is ordered less-than-or-equal to the old value. An unordered comparison
reaches the write. Bit31 is cleared at the end; bit30/bit29 persist.

Rust reuses `ActionRuntimeServices.pushing`, already bound to the native +0x13c
global collection (class `0x90c3_decf_0105_631e`, collection
`0x9dc4_354d_be42_8a7f`) and the existing retained
`ActorSkaterGraphState.push[1]`, `time_since_teleport`, Motion intents, and
physical z-speed. The isolated `actor_motion_compute_first_push.rs` callback
and focused tests are source-backed but are not a live-runtime claim until the
root integrator registers its module/instance/lifecycle dispatch and the raw-A
facade replay moves past occurrence 2441. Confidence: high for control flow,
keys, offsets, and owner reuse; exceptional PPC/host floating-point payload
identity remains unproven.
