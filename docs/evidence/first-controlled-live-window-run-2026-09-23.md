# First controlled live window run (2026-09-23)

The Windows `--live-skating` command now uses the public retained
`SelectedLiveSession`, original A_RobsPark inspection scene, XInput adapter,
selected palette/material renderer and provisional CameraMan projection.
It remains an explicitly controlled first-test profile, not native whole boot:
PlayerUI command-3/status, enabled PROXY-before-BAM, restored camera,
original shading and source W transport are unresolved.

The first direct 600-presented-frame run in
`artifacts/controlled-first-live-2026-09-23.log` uploaded 2,085 map draws
but overflowed the Windows main stack before the first tick. The retained
session was moved into the winit closure by value. Boxing that one owner
before the move fixed the overflow without constructing a second session.

The next run in `artifacts/controlled-first-live-boxed-2026-09-23.log`
completed terrain stream and resident boot, produced the first selected
queued frame (mask 3, CameraMan counter 1), and created real character and
board GPU objects. Its next tick stopped at the typed Action update
`PendingStateUpdate(0x828ed930)`; IDA review yielded the guarded shared
grounded-slot31 dispatch in `action-grounded-slot31-live-2026-09-23.md`.
The original 600-frame capture could not finish after that error.

The reviewed `--live-completions 1 --live-capture` path waits for a completed
selected tick to be actually presented, captures that frame, then exits
before another tick. `artifacts/controlled-first-skating-frame-2026-09-23.log`
exited successfully, and
`artifacts/controlled-first-skating-frame-2026-09-23.png` is the first
captured map/skater/board frame. It shows the skater near the ground and the
board intersecting the ground; this is a visual/pose fidelity defect, not a
claim of correct skating physics. Materials and camera are diagnostic.
The bounded path leaves `--live-frames` inspection-style present counting
unchanged.

The direct `--live-completions 30 --live-capture
artifacts/controlled-skating-frame30-2026-09-23.png` run also exited zero.
Its log is `artifacts/controlled-skating-frame30-2026-09-23.log`. The
captured skater is seated near the ground with the board ahead of his legs;
the visual mismatch persists beyond the first completed frame. This run
establishes continued selected-frame execution and presentation, not correct
onboard/bail state, posture, collision, or controller response. The input
state and precise source of this pose have not yet been isolated.

After the grounded Action dispatch and live completion integration,
`cargo test --workspace` passed 4,958 unit tests and six doc tests in
`artifacts/workspace-after-first-live-2026-09-23.log`.
The mounted original-data suite passed 29/29 in
`artifacts/original-selected-after-first-live-2026-09-23.log`, excluding only
the separately identified PROXY diagnostic census. A direct Windows XInput
status test found all four controller slots disconnected (1167) in this
environment; its log is `artifacts/windows-pad-live-status-2026-09-23.log`.
Thus the seated pose occurred without an attached pad, though the exact
Actor/animation/physics cause still needs source tracing.

A temporary read-only `CurrentPhysicalState` probe in the same public facade
ran another 30-completion live capture, exiting zero in
`artifacts/controlled-skating-state30-2026-09-23.log`. The state after each
completed frame was Ground at frame 1, Air at frames 2 through 22,
GroundAnimation at frame 23, then Ground through frame 30. It never reported
WipeoutGround. This rules out a simple bail-state explanation for the captured
reclined pose and points to initial ground-to-air motion or presentation pose
as the next discriminator. The temporary logging branch was removed after
the probe; the captured image is
`artifacts/controlled-skating-state30-2026-09-23.png`.

The independently reviewed controlled host catch-up cap now runs at most one
fixed `1/60` tick and pad poll per redraw, dropping older overdue wall-clock
slots while preserving the fractional remainder. A subsequent 30-completion
capture exited zero in `artifacts/controlled-skating-capped30-2026-09-23.log`
with the same visible seated pose. It logged roughly 20 to 33 dropped slots
on slow redraws, so this debug build is not yet responsive enough for a
controller test. Optional `SKATE2_PROFILE_REDRAW=1` timing on three completed
frames in `artifacts/live-frame-split-debug-2026-09-23.log` found selected
tick times of 221, 480, and 371 ms; map/skater view updates of 62, 39, and
39 ms; and GPU presentation of 44, 56, and 60 ms. The selected tick is the
main debug-mode cost. These are one local machine's measurements, not a
native cadence claim.

The same full scene in release mode completed 120 selected frames and captured
`artifacts/controlled-skating-release120-2026-09-23.png`; the run exited zero
in `artifacts/controlled-skating-release120-2026-09-23.log`. Across 120
profiled release ticks, selected-tick p50/p95 were 33.71/34.46 ms, view
update p50/p95 2.70/3.01 ms, and presentation p50/p95 4.42/5.75 ms
(144 redraws, including boot). The debug build is the main performance loss,
but the release selected tick still exceeds the 16.67 ms host step. It dropped
at least 120 overdue wall-clock slots in this bounded run. Separate release
captures at frames 30 and 60 match the debug frame-30 pose and show the
skater progressively reclining with the board partly below the ground; the
frame-120 capture additionally has visible face/head geometry spikes. These
are current behavior and require physical-root/presentation diagnostics before
any skating-feel claim.

The earlier boot-only captures
`artifacts/controlled-first-live-frame2-2026-09-23.png` and
`artifacts/controlled-first-live-frame3-2026-09-23.png` are not skating
successes; those commands exited with an explicit no-completed-frame error.
