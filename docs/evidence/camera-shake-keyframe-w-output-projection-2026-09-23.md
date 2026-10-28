# Shake keyframe W versus published camera output (2026-09-23)

Read-only IDA continuation of `camera-shake-apply-lane-continuation-2026-09-23.md`.
Exact `Test-Path` was false before creating this note. The mounted keyframe
arrays retain unknown W because Camera-category `operator new[]` does not
request `MemClear`, and `ShakeEffect::Load82e43ce0` discards each fourth text
token. This note follows that unknown lane to its last use without filling it.

## W dependency graph

Native `ShakeEffect::Apply82e43700` loads two full rotation and position
vectors (`82e4395c/64`, `82e439c8/43a00`) and interpolates all four lanes
with `vmulfp`/`vmaddfp` (`82e439d0..43a1c`). Rotation W remains unknown in
the interpolated/scaled vector, but all three `XMVectorSinCos` inputs are
separate `vspltw` selections 0, 1, 2 (`82e43a80`, `82e43a8c`,
`82e43aa0`). There is no lane-3 selection or horizontal reduction between
the interpolation and these calls. The scratch rotation columns
`82e43b88/ba0/bb8` therefore depend on rotation XYZ and live scalar
parameters only; their native W duplicates an X result of the scratch
rotation, not keyframe W.

Position W likewise persists through the full-lane multiply sequence
`82e43a68/70`, `82e43ba4/bb4` and the transient full-vector store to
`ShakeEffect+0x10` at `82e43bf4`. Immediately afterward `82e43bd8/e0/e8`
select only local offset X/Y/Z with `vspltw` indices 0, 1, 2. Three
full-lane multiply/FMA operations combine these scalars with the input
matrix's first three columns and overwrite `ShakeEffect+0x10` at
`82e43c04`. Its final W is
`I0.w.mul_add(O.x,+0.0)`, then FMA with `I1.w*O.y`, then FMA with
`I2.w*O.z` in native instruction order; `O.w` is
not an operand. The transient unknown W has no call or branch consumer
before the overwrite, although an instruction-level fault snapshot could
observe it.

The returned matrix `82e43c08..94` selects only scratch transform column
X/Y/Z (`vspltw` 0/1/2 at `82e43c28..64`) and combines them with full input
matrix columns. All four returned W lanes are functions of the input matrix
column W lanes and those selected scratch XYZ scalars. No interpolated
keyframe W reaches returned matrix XYZ **or W**. The normal caller copies
all four returned columns to `Frame.matrix` (`82e487b4..d0`), passes the
already-overwritten full final offset to `SetShakeOffset` (`82e487d4..e8`),
and selects `Frame.previous_matrix` (`82e487ec..48854`). The teleported
getter is after both matrix and offset publication and is skipped on the
hard-cut `0x40` branch. These publications
therefore cannot reintroduce keyframe W. The setter stores all lanes and may
multiply them by `time_since_collision*0.2f` at `82e3c79c..e0`; its W
depends on input matrix W, not keyframe W. Rust's later
`camera_rig_desired_position::calculate_position_with_shake` adds that
retained offset lane-wise, but its XYZ are already independent of keyframe W.

## Bounded implementation contract

`ShakeKeyframes` must retain `SkeletonRootVector.w=None` after loading the
mounted asset; the discarded fourth/eighth text tokens are not substitutes.
Apply can project XYZ from current/next rotation and position without asking
for W. It must preserve the native full-vector operation schedule for each
known XYZ lane. The transient `ShakeEffect+0x10` store should retain W=None;
the final overwritten offset is derived from input matrix W and local XYZ.
The returned matrix likewise derives W from the input matrix and scratch
XYZ, never from keyframe W. If input matrix W is itself unknown, its output
W stays `None`; visible XYZ remain computable.

This removes keyframe W as a blocker to the normal frame's visible
orientation, position, and full returned-matrix/offset publication. It does
not license guessing other unknown W, replacing native zero-multiplies,
or using saturating host casts for `fctidz`. Finite in-range phase conversion
can be checked; nonfinite/out-of-range PPC integer conversion remains a
typed refusal boundary. Full Apply and caller publication still require
independent review of the draft numeric and Bezier instruction schedule.
