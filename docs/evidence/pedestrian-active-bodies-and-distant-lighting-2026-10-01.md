# Pedestrian collision admission and distant lighting

Status: source corrections only. Compilation and runtime testing are pending
for this follow-up. No cargo build/check/test or game launch was performed.
Changed Rust files passed rustfmt syntax parsing with `--emit stdout` and
`skip_children=true`; that does not type-check Rust or validate WGSL.

## Collision bodies stayed behind the animated pedestrian

`selected_living_world_pedestrian_spawn.rs` passed state 1 for both collision
bodies. Native `LWPedestrianCensusMan::Spawn` (82e9a510), immediately after
SetTargetTransform, instead loads r6=4 at 82e9aad4 and r7=4 at 82e9aad8 before
AddModelsToWorld at 82e9aae0. These are the WORLD and SKATER body states.

The existing CharacterAssemblyCandidate admission forwards the state to the
existing Parts/body pool. State 1 is STATIC_BODY, and the existing Island
registration puts it in static_order. The animation-driven velocity publisher
only activates state 2 (frozen), so it does not repair an incorrectly static
body. Passing ACTIVE_BODY=4 restores admission to the dynamic body list and
allows those velocities to move the collision bodies with the pedestrian.
No extra collision capsule, proximity response, or alternate solver was added.
The original group 13 and existing culling/contact policy are retained.

This is a concrete cause of visual/body separation and missed player contacts.
Actual contact response after the correction still needs a playtest.

## Defaultlivingworld used an obsolete shader variant

Read-only disassembly with tools/xenos_disasm.py of the original
`data/render/shaders/final/defaultlivingworld_defaultPS.fpo` is the authority.
The extracted asset is under `%TEMP%/skate2-pedestrian-shaders-20260930`.
Its older embedded source (hash 6788fdd8a297) omits operations present in the
shipped GPU program, and the previous renderer followed that older text.

Final PS bindings:

- c0..2: WorldShadow_MatRow; c3..5: g_charattributes.
- c6..13: eight m_params rows; c14: instance opacity.
- c15/c16: red/blue recolouring; tf0: world shadow; tf1: diffuse.

PC4..20 projects world position into the shared world-shadow map, with
UV=(.5*x+.5, -.5*y+.5), strict max(abs(x),abs(y)) < .99, and
saturate(coverage * (stored_depth >= depth)). PC18..23 multiplies diffuse gain
by m_params[5].w and saturate(worldShadow + m_params[7].w), before fog. These
factors were absent. PC24..32 applies character exposure, tone mapping,
sqrt(abs(mapped)), and instance opacity. Diffuse is squared and recoloured
using the original squared-green threshold .001225.

The plain effect now retains all eight authored parameter rows and uses the
existing character lighting bind group and WorldShadow map/row buffer.
The normal and alpha layout slots reuse the resident diffuse texture for this
effect; its entrypoints never sample those slots. No second shadow owner or
texture cache was introduced. Ordinary and ghost passes use the corrected
shader. The obsolete unshadowed defaultlivingworld shader/pipelines were
removed, rather than retained as an alternative fallback. Other pedestrian
material families retain their existing lighting implementation.

This corrects a concrete reason distant, low-detail pedestrians were too bright.
It does not undo the separate PC world/sky brightness setting.

## Brief floating on flat pavement: not established as fixed

It happens on flat pavement while the walking animation
continues. The source trace checked AddTestsToGroundDMOBatch82e789c8,
ProcessGroundDMOLineTestBatch82e72c50, ReceiveGroundDMOLineTestBatch82e72d70,
ForcePositionToGroundPlane82e795e0, SpatialAdjust.Update82e745a0,
UpdateTargetTransform82e7f660, and SetJumpToPosition82e701f8. Preparing a jump
clears is_jumping; JumpNow is the distinct producer. Submitted query misses
are deliberately consumed by the original receive path. No evidence yet
justifies a new ground clamp or removing authored vertical root motion.

`SKATE2_TRACE_PEDESTRIAN_HEIGHT=1` now logs original target Y, trajectory delta
Y, composed animation Y, selected ground position, final target/ground gap,
and jump/lock/pullback/slide/collision flags. It logs periodic samples and
elevated-gap/root-motion samples, with an 8192-line cap. The .08/.02 thresholds
are diagnostic filters only, not physics or animation tuning. Normal quiet
play has no height logging. No runtime capture has been made with these changes.

After building, `play-pedestrian-height.bat` runs the normal controller path
with only this optional trace enabled, writing
`artifacts/pedestrian-height-run.log`. Exit shortly after a visible lift to
preserve a useful capture. A target already on the selected ground would point
to a presentation/pose issue; a changed ground or trajectory Y would identify
the upstream path to inspect. Do not report the floating issue solved without
that evidence or an independently established source correction.
