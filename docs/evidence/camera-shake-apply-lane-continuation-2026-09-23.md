# Normal camera ShakeEffect Apply: Bezier, phase and output lanes (2026-09-23)

Read-only IDA audit continuing `camera-shake-apply-publication-audit-2026-09-23.md`
and `camera-frame0-speed-producer-audit-2026-09-23.md`. Exact `Test-Path` was
false before creating this evidence. No registered Rust, Cargo, or IDB state
was changed. The native owner is `Iris::ShakeEffect::Apply82e43700`, reached
from `CameraMan::UpdateRig82e487a8` on the mounted normal path. This note
corrects the earlier Apply note's unresolved VMX high-register aliases and
fractional phase. Source confidence is **exact** for cited raw instruction
dependencies and caller order, **high** for ordinary finite scalar formulas,
and **medium** for host bit parity without a PPC trace.

## BezierLite source dependency

`Math::BezierLite::EvaluateYatX829b1e88` is called for amplitude at
`82e437b8` with GameCameraParameters curve `+0x0`, and frequency at
`82e43804` with curve `+0x40`, only when the respective top speed compares
unequal to +0. The bound speed/top ratio is the existing two-`fsel` graph.
The curve matrices are already retained as `CameraParameters` fields; their
Vault read offsets differ from their destination parameter offsets by design.

The `bezierBasisMat` at `0x8307ffc0` is BSS, so static IDB bytes are zero.
Its dynamic initializer `82f763f0..76444` writes four full vector columns,
in order: `[-1,3,-3,1]`, `[3,-6,3,0]`, `[-3,3,0,0]`, `[1,0,0,0]`.
`BezierLite::Evaluate829b0c60` reads all four supplied curve columns and
these four basis columns. For each polynomial coefficient it multiplies the
first curve column by the corresponding basis X scalar, then adds the other
three curve columns in order with `vmaddfp` (`829b0d18..44`). Its result is the ordered
vector sum `A*t^3 + B*t^2 + C*t + D`, with `t^2` then `t^3=t^2*t`, the first
term a `vmulfp`, and the next three `vmaddfp` at `829b0ddc..dec`. When the
tangent pointer is non-null, `829b0d4c..db8` separately builds
`3*t^2`, `2*t`, `1`, `0` and evaluates derivative with one multiply followed
by three fused adds. `EvaluateYatX` always passes that tangent pointer.

`EvaluateYatX` starts `t=0.5`, calls `Evaluate` at least once, and keeps the
last full result and derivative. After each call, unless it has reached ten
evaluations, it computes `targetX-result.x`; it stops when ordered
`abs(error)<=0.01` or ordered `abs(derivative.x)<=2^-23` at
`829b1f04..20`. Otherwise `829b1f24..28` performs `t += error/derivative.x`
using separate `fdivs` and `fadds`, then reevaluates. Unordered comparisons
fall through to the divide/next evaluation until the ten-call cap. The return
at `829b1f50` is last `result.y`. A generic cubic solver or standard
`powf`/Horner evaluation would change source rounding and branch order.

## Phase and keyframe owner

`82e43838` loads old `ShakeEffect.time` `+0x24` before update. Frequency is
already chosen, including the existing one-shot positive-or-unordered branch;
`82e43864..ac` computes `step=(ShakeDataFPS*frequency)*dT`, then stores
`new_time=old_time+step` with `fmadds` to `+0x24` **before** selecting data.
`82e43848/80` loaded rotation/position array pointers and `82e43840`
loaded unsigned count. The reciprocal-estimate and two-refinement graph
`82e43850..dc` is applied to a staged +1. The `vrfiz128` at `82e438f4`
truncates `old_time*refined_reciprocal_one` toward zero; raw
`vnmsubfp v2,v3,v12,v9` at `82e438f8` means
`fraction=old_time - truncated*one`, because VMX fused operand order is
`VB - VA*VC`. The apparent high operand `v93` at `82e43858/60` is the
recently loaded `v61`, not an external constant. `82e43904` computes
`integer_source=new_time-fraction`; `fctidz` at `82e43914` converts that
to a signed 64-bit integer, whose low word is then interpreted as unsigned
by `divwu` at `82e43924`. Native `index=low_word % count` and
`next=(index+1) % count` use two unguarded unsigned divisions at
`82e43924/58`. These are deliberately **not** a conventional `floor(new_time)`
and `fract(new_time)` pair. Zero count, conversion overflow/NaN and PPC
rounding/status need a reference boundary; no fallback frame is source-backed.

`82e4395c/64` read current rotation/position vectors at `index*16`;
`82e439c8/43a00` read next vectors at `next*16`. With
`other=1-fraction` (`82e43928`), `82e439d0..43a1c` forms each full vector
as `current*other + next*fraction`, source multiply first, then `vmaddfp`.
The Rust `ShakeKeyframes` owner retains count and both arrays, including
unwritten elements and unknown W lanes after file load. In native
`ShakeEffect::Load82e43ce0`, `operator new[]` allocates 16-byte vectors and
the eight text tokens per keyframe update XYZ while copying each prior full
vector; the fourth token of each four-token group is consumed but not stored.
The later mean subtraction also acts on full vectors. A reconstructed Apply
must propagate the retained unknown W rather than insert +0.

## Amplitude, distance, angles and offset

`f28` after `82e437e8` is the already interpolated amplitude plus one-shot,
multiplied by the stored multiplier. Its addition at `82e437cc` is exactly
`fadds f13,f0,f29` (one-shot left operand, amplitude right operand), which
matters for exceptional/sign-of-zero fidelity. `82e43870..e0` forms the distance factor
from `(3.4f-subjectDistance)*1.6666663f` and two `fsel` bounds. Native
`+0x310/+0x314/+0x30c` are rotation, dutch, translation multipliers.
At `82e43a1c..80`, each rotation axis is scaled in source order by amplitude,
distance factor and rotation multiplier; Z is subsequently multiplied by the
dutch multiplier and inserted into the third angle lane. The three
`XMVectorSinCos` calls at `82e43a84/94/a4` sample those X/Y/Z lanes in
order. `reckoning_sin_cos::try_sin_cos` is an existing checked implementation
of that same native service. The position vector is scaled by translation
multiplier, then amplitude, then distance factor (`82e43a70,43ba4,43bb4`).
The resulting full vector is stored first as the local `ShakeEffect+0x10`
offset at `82e43bf4`. Its W remains allocation-unknown at this transient
store, but only its X/Y/Z lanes are selected for the following transform.

The apparent `v64`, `v67`..`v70`, `v72`, `v77`, `v78`, `v83`, `v84`,
`v87`, `v89`, and `v93`..`v95` in IDA's VMX128 display alias the actual
recently produced registers `v32`, `v35`..`v38`, `v40`, `v45`, `v46`,
`v51`, `v52`, `v55`, `v57`, and `v61`..`v63`. They are not external
constant vectors. This is verified against raw operand fields, for example
`82e43b78 16c009e5` (`vperm128` source `v32`, shown `v64`) and
`82e43a54 177350b1` (source `v51`, shown `v83`).

Let `sx,cx`, `sy,cy`, `sz,cz` be the three returned SinCos pairs. The
scratch rotation columns stored at `82e43b88/ba0/bb8` are:

```
S0 = [cy*cz, cy*sz, -sy, cy*cz]
p = cx*cz; q = sx*sz; r = cx*sz; s = sx*cz
S1 = [sy*s-r, sy*q+p, cy*sx, sy*s-r]
S2 = [sy*p+q, sy*r-s, cy*cx, sy*p+q]
S3 = [+0,+0,+0,+0]
```

The `vperm128` mask is `VectorPermuteConstant<0,5,0,0>` at `82e43b3c`;
it copies X into W, then `vrlimi128` inserts Z. This is the same basis
formula already present in `camera_frame_composition_math::framing_basis`,
although that helper fixes its third angle to +0 and cannot be called
unchanged for shake. Preserve the individual multiply/FMA dependency graph.

`82e43bd8..43c04` splats local offset X/Y/Z, computes
`I0.mul_add(offset.x,+0.0)` at `82e43bf0`, then `+I1*offset.y`, then
`+I2*offset.z` with two further `vmaddfp` operations, and overwrites
`ShakeEffect+0x10` with that world
offset. `I0..I3` are the input matrix's four full columns. The input
translation is not added to shake offset. Every W lane participates in the
vector arithmetic on the input matrix columns, but the position keyframe W
does not: only `vspltw` indices 0, 1 and 2 of the local offset are used.
The final offset W depends on `I0.w/I1.w/I2.w` and local XYZ, and is
independent of keyframe W.

## Returned matrix and caller publication

`82e43c08..94` composes the scratch columns into the returned full matrix.
For each scratch column `Sj`, it forms `I0*Sj.x`, adds `I1*Sj.y` by FMA,
then `I2*Sj.z` by FMA. For `j=3` it first computes
`I3 + I0*S3.x` by FMA, then adds the Y and Z terms. All four 16-byte
columns, including W, are written to result at `82e43c88..94` in X, Y,
Z, translation order. The native translation path still executes its
zero-multiplies, so a direct `I3` copy is not bit-equivalent for exceptional
inputs. `82e43c98..bc` then decays one-shot, retaining the product only
for ordered `>=0x3ba3d70a` and unordered. Only ordered less overwrites
+0.0: `fcmpu` unordered clears LT, so `bge` branches.

The caller copies those four columns to `Frame.matrix`
(`82e487b4..d0`), passes the full world shake offset into
`LogicalCameraPositioner::SetShakeOffset82e3c748` (`82e487d4..e8`), and
then chooses `Frame.previous_matrix` at `82e487ec..48854` from either the
new matrix (hard cut/teleport) or the saved old four columns. The teleported
getter runs only after the matrix copy and SetShakeOffset, and is skipped
when the hard-cut `0x40` bit is set (`82e487ec..48818`). The setter
first stores the full offset at Positioner `+0x140` (`82e3c79c..a8`);
for ordered `time_since_collision<5`, it overwrites with
`offset*(time_since_collision*0.2f)` (`82e3c7b0..e0`). Its preceding
vector assertion and exact exception behavior still need separate review.

## Implementation boundary

The recovered math graph supports a typed implementation plan. The mounted
`.shk` vector-W provenance is unknown, but the Apply XYZ splats prevent it
from reaching the final offset or returned matrix; the exact projection is
reviewed separately in `camera-shake-keyframe-w-output-projection-2026-09-23.md`.
Existing
`camera_shake_subject_inputs.rs`, `reckoning_sin_cos::try_sin_cos`,
`CameraParameters`, `ShakeKeyframes`, `CameraShake`, `CameraFrame`, and
`CameraPositioner` should be reused. A numeric boundary for PPC `fctidz`,
`divwu` and exceptional/NaN cases must be explicit. Running only a partial
matrix publication would falsely clear `PendingCameraSubjectSpeed`.

## Mounted keyframes and exact remaining publication boundary

The project `skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES}`
resolved `data\\camera\\1.shk` from the mounted retail archives under
`<game folder>`. A standalone `rustc`
probe linked the already-built `libskate2_formats.rlib`; no Cargo or repo
runtime was run. The entry is 10,603 bytes, begins with count **136**, and
contains exactly `1 + 136*8 = 1089` whitespace tokens. First keyframe tokens
are `0.000000 -0.000000 0.000000 1.000000 4.061613 10.244023
54.145176 1.428138`. Rotation fourth tokens vary from about `1.000000`
to `0.999817`; every position eighth token is `1.428138`.

Those fourth/eighth tokens **do not define native vector W**. The native
`ShakeEffect::Load` at `82e43f08..14` calls `atof` for rotation token four
without an `frsp`/store; `82e43fa4..c8` does the same for position token
eight. At `82e43ff0..440a4`, each actual XYZ scalar is stored into a
scratch vector copied from the newly allocated array entry. No W scalar is
written before the full vector is stored back. At `82e440b8..44174`, full
vector sums/mean subtractions propagate whatever W the allocation supplied.
Both arrays are allocated via `operator new[]` at `82e43e08` and `82e43e30`
with `flags=1`. The wrapper at `828b4a30` uses **`oris r6,r27,0x100`**,
setting bit `0x01000000` (not bit `0x00000100`), and delegates to the
category core allocator. `kMemCategory_Camera` at `82ff1a4c` is category 14;
its setup row `82ff0ad8` has mask `0xffffffff` and OR field `0`.
`CategoryAllocator::Alloc82973594..a4` therefore forwards flags
`0x01000001`. `XAllocator::Internal::AllocHeapMem82972240` calls `MemClear`
only when bit `0x00000100` is present, in both the small-object
(`8297232c..40`) and ordinary-heap (`829723a8..bc`) paths. That bit is
absent for this Camera allocation. No later W store appears in Load before
the full-vector mean subtraction, and no zero-fill or other W initialization
has been established. Consequently the mounted nonzero W
tokens must **not** be copied into Rust `ShakeKeyframes.w`, and W=0 is also
unsourced. The native full-width local offset at `82e43bf4` briefly retains
that unknown W, but the following XYZ-only splats discard it before the final
offset and matrix publication. This
contrasts with the previously audited Sim category, whose setup row ORs
`0x00000100`; that Sim zero-fill proof does not apply to Camera.

The setter `LogicalCameraPositioner::SetShakeOffset82e3c748` reads the full
offset at `82e3c764`, copies and modifies a comparison vector with
`vrlimi128 ... 1,1`, self-compares it via `vcmpeqfp128.` and asserts on a
false CR result at `82e3c780..98`. The exact comparison-lane mapping of
that `vrlimi128` remains to verify, but the full offset is stored at
`82e3c7a0..a8` before the collision-time branch. Ordered collision time
`>=5.0` or unordered returns it unchanged; ordered time `<5.0` multiplies every lane by
`frsp(time*0.2f)` and overwrites at `82e3c7bc..e0`. This includes offset W.

The phase integer conversion is `fsubs new_time,fraction` at `82e43904`,
`fctidz` at `82e43914`, low 32-bit extraction at `82e43920`, then unsigned
`divwu` at `82e43924` and again for index+1 at `82e43958`. Count 136
eliminates the divisor-zero case for the mounted successful load. A host
conversion can be proven for finite values in signed 64-bit range by
truncating toward zero, taking the low unsigned 32 bits, and dividing by 136;
nonfinite/out-of-range `fctidz` behavior is not established and must remain
an explicit typed boundary. This numeric policy alone does not resolve the
unknown keyframe W; finite and exceptional numeric behavior are the
remaining boundary for a source-backed Apply plus publication patch.
