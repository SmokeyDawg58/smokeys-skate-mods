# ShakeEffect noninitial phase and finite continuation (2026-09-23)

Read-only fresh IDA review of `ShakeEffect::Apply82e43700` against the
integrated `camera_shake_apply_frame0.rs`. Exact `Test-Path` was false before
creating this note. The mounted original roster completed frame 0 and then
stopped at `Shake(NonInitialTime(1022740701))`; that bit pattern is
`0x3cf5c8dd`, approximately `0.0300030056f`. No registered file, Cargo, or
IDB state was changed by this audit.

## Exact phase producer

The reciprocal denominator is the scalar +1 written at `82e43828` and
loaded/splatted at `82e4384c..50`. `vrefp128` at `82e43854` produces an
estimate `r0`. `vcsxwfp128` at `82e4385c` supplies +1 for the residuals.
The VMX instruction graph, with VMX `vmaddfp(VA,VB,VC)=VA*VC+VB` and
`vnmsubfp(VA,VB,VC)=VB-VA*VC`, is:

```
e0 = 1 - r0*1     82e438a4 vnmsubfp
r1 = r0*e0 + r0   82e438bc vmaddfp
e1 = 1 - r1*1     82e438d0 vnmsubfp
r2 = r1*e1 + r1   82e438d8 vmaddfp
```

These are the same two fused Newton refinements in the existing
`simulation_drive_jacobian::refine_drive_reciprocal(1.0, seed)` helper.
The host `ReckoningNumeric::reciprocal_estimate(1.0)` supplies the seed;
its bits are an explicit ordinary-host policy, not a proven Xenon `vrefp`
bit trace. The draft obtains this seed before the new-time store, matching
the native `vrefp` site; a seed-service refusal therefore cannot publish
the time. The native old time is loaded from `ShakeEffect+0x24` at
`82e43838`, before `fmadds` publishes `new_time=old_time+(fps*frequency)*dt`
at `82e438a8..ac`. Scalar `v12` at `82e438b4..b8` is this **old** time.

At `82e438f0`, `v59=r2*old_time`; `vrfiz128` at `82e438f4` truncates that
f32 result toward zero while retaining it as an f32 vector. At
`82e438f8`, `v2=old_time-trunc(r2*old_time)*1` by a fused negative
multiply-add: the displayed `v9` is a copy of the +1 vector made at
`82e43868`, while displayed `v12` is the old-time splat. The stored X
fraction is loaded at `82e43900`; `fsubs f4,f9,f0` at `82e43904` forms
`integer_source=new_time-fraction` as an f32 subtraction. It is **not**
`trunc(new_time)` or a conventional fractional part of new time.

`fctidz` at `82e43914` converts that f32 source to signed i64 toward zero.
The low word at `82e43920` is reinterpreted unsigned; `divwu` at
`82e43924` and `mullw/subf` at `82e4392c/4c` compute
`index=low_u32%count`. The second `divwu` at `82e43958` computes
`next=(index.wrapping_add(1))%count`. In parallel, `fsubs f2,f31,f0` at
`82e43928` forms `other=1-fraction`. Current rotation and position load at
`82e4395c/64`, next rotation and position at `82e439c8/43a00`; their
known XYZ lanes use native `current*other` then
`next.mul_add(fraction,base)` (`82e439d0..43a1c`). The unknown allocated
keyframe W still remains unknown, but the subsequent angle/offset XYZ
splats discard it before final frame publication, as shown in
`camera-shake-keyframe-w-output-projection-2026-09-23.md`.

## Bounded host behavior

The unregistered continuation draft replaces the initial-time stop with
that exact phase arithmetic and interpolation. It preserves multiplier
publication, then time publication, then phase selection, keyframe reads,
local/final offset, returned matrix, one-shot decay, caller Frame.matrix,
SetShakeOffset, and Frame.previous_matrix. Finite `vrfiz` uses f32
truncate-toward-zero; finite in-range `fctidz` uses a checked signed-i64
conversion before low-u32 unsigned modulo. Nonfinite phase inputs and
out-of-range integer conversion return typed errors **after** the native
time-write prefix, without fabricating an index or frame. Count zero remains
an explicit typed boundary immediately after time publication and the
distance calculation: native `twllei r7,0` at `82e438c4/c8` traps there,
before `vrfiz`, fraction, and `fctidz`. The count itself was loaded at
`82e43840`, and the first reciprocal refinement at `82e438bc` precedes
the traps; the host does not model FP status from that pure refinement.
The source reciprocal
seed, exceptional VMX/FP status, and exact PPC signed-zero/NaN behavior are
not claimed bit-for-bit by the host model.

For the observed frame-1 old time `0x3cf5c8dd`, the ordinary reciprocal of
+1 refines to +1 and `vrfiz(old_time)` is +0, so the fraction is the old
time, approximately `0.0300030056`, rather than zero. This is precisely why
the frame-0-only interpolation cannot be replayed for frame 1.
