# Intermittent visual lean snap — investigation, NOT a fix

Playtesting shows occasional adjacent-frame left/right lean reversal at speed in the
city, with no corresponding stance, feet, board, or physics
change. Do not infer a stance bug or call this fixed without capturing it.
Work remains SOLO; no Cargo/rustc/game run for this task.

## Source audit

Searched the existing implementations before considering new gameplay code.
Reviewed IDA decompiles for SettingBodyTilt::Update82c9e288,
Reckoning::CalculateTilt82dd7e60, SetTurning::Update82cc8408 and
Reshape82c954e8, AnimationConditioner::UpdateAnimationTurn82e2b470, and
ACS NBlend82a26348. First four captured in the accompanying IDA text file.

- SettingBodyTilt uses the existing ApplyingBodyTilt edge reset, native mirror
  sign, spin graph, acceleration/velocity clamps, then the tilt_x attribute.
- SetTurning blends quick/slow reshaping, handles retained slide state, smooths
  angle with its authored graph and maximum step, and publishes dir/quickness/
  holding/speedtuck through the existing attribute owner.
- CalculateTilt's low-slope gate retains the previous value in the native code;
  that is not by itself evidence of a host defect. Checked the raw PPC wrap,
  absolute value, subtraction and sign branch at82dd7ff8..8048 as well. IDA's
  VA labels on these VMX128 instructions are wrong; decode the operand bits.
- Also inspected existing tree Mirror/PhaseBlend/BlendSpace/NBlend and
  presentation quaternion packing. No demonstrated cause was found.

This is a scoped audit, not certification of all animation math. In particular,
the existing NBlend alias-preservation deviation remains documented in
city-moving-view-2026-09-25.md; it was not changed speculatively.

## Added diagnostic only

Run build-release.bat, then play-lean-trace.bat. Skate normally and press Escape
immediately after seeing the snap. The launcher names the CSV in artifacts.
Normal play-quiet.bat behavior remains unchanged. SKATE2_LEAN_TRACE is opt-in;
capture stores at most131072 recent events in memory and writes once on exit,
including returned errors. Forced termination cannot save it. Retained duration
depends on active graph and vehicle count, so exit promptly. No continuous disk
or console output. Diagnostic capture has some CPU/memory cost.

The capture reuses the actual callback values, tree evaluation, pre-compression
local pose and final palette-input matrices. It creates no second animation,
simulation, input, or presentation implementation. It makes no new gameplay
getter calls or fallible graph reads. The app uses the existing read-only
diagnostic_state_ids getter without turning missing diagnostics into failures.

CSV: tick,stage,owner,id,v0..v15. Tick increments before each attempted live tick;
it can differ from the completed-frame number in position/states rows. Startup
before the first tick is omitted. Rows remain in observation order.

| Stage | Values |
| --- | --- |
| body_tilt | raw x, signed x, abs spin, target, acceleration clamp, velocity clamp, angle, delta, published value, mirrored, air |
| turning | target lean, turn, direction, quickness, holding, reshaped angle, reshaped direction, target, smoothed angle, elapsed, state, old state, abs speed, tuck, fakie, maximum step |
| blend_query | current BlendSpace attributes; owner=pool identity, id=node index |
| blend_volume | volume set, selected volume, time |
| blend_weights | weights at Eval, same pool/node |
| clip | database record index, time, old time, length, speed, loops, do_eval, trimmed |
| phase_blend | coefficient, first/second child indices, do_eval |
| mirror | child index, do_eval |
| local_pose | 16 column-major local matrix lanes; owner=player ID, id=bone index |
| palette_pose | 16 matrix lanes passed to palette construction; owner=palette identity, id=bone index (includes vehicles) |
| position | selected stream focus XYZ; id=completed-frame number |
| states | Action, Motion, physical state; id=completed-frame number |

Pool/node IDs are lifetime-local. Clip record index is database-relative.
Local and palette matrices are different spaces; do not subtract them directly.
Compare each stage over adjacent ticks, then follow hierarchy/compression if
inputs remain continuous but final output jumps. Determine which palette is
the skater (bone count/hierarchy and matching motion), excluding vehicle rows.

## Validation and next step

Eight edited Rust files pass rustfmt syntax parsing (edition2024, skip_children).
No compilation or runtime validation performed. This diagnostic does not claim
to fix the visual snap. Next: inspect the captured last ticks and locate the
first discontinuity in inputs, selected nodes/weights, local pose or restored
palette; then compare that precise path with IDA before changing behavior.
