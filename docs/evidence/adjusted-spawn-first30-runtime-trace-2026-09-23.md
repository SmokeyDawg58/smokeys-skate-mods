# Adjusted-spawn neutral-pad first-30-frame trace

The release client ran `--live-skating --live-completions 30` with
`SKATE2_TRACE_ADJUSTED_POSE=1` and the original Xbox game archive on
2026-09-23. It exited zero and captured
`artifacts/controlled-skating-release30-adjusted-pose-trace-2026-09-23.png`.
The complete phase-labelled measurements are in the adjacent `.log` file.
The direct Windows probe previously found XInput slots 0-3 disconnected;
there was no intentional live pad input in this trace. These are Rust runtime
observations, not a native Xbox per-tick comparison.

The source-gated spawn line query puts board Part 6 at approximately
`[588.8617, -54.2810, 996.5048]` before the first solve. The earlier
unadjusted run had frame-1 wheel height `0.14285605` and entered Air before
frame 2. With adjusted spawn, the post-frame-1 wheel height is `0.093501695`
and frame 2 remains Ground. That is below the live `CLEAR_HEIGHT=0.12`; the
spawn correction therefore changes this Rust branch as predicted. It is not
proof of a native-equivalent spawn or contact result.

| Frame/phase | Physical state | Wheel height | Board Y | Board up-axis Y |
|---|---|---:|---:|---:|
| 1 BeforeSolver | Ground | 0.0 constructor value | -54.281002 | 1.000000 |
| 1 AfterSolver | Ground | 0.0 until post-solve | -54.447670 | 0.999996 |
| 2 BeforeActor | Ground | 0.093502 | -54.447670 | 0.999996 |
| 2 AfterSolver | Ground | 0.093502 until post-solve | -54.526500 | 0.940849 |
| 3 BeforeActor | Ground | 0.200000 | -54.526500 | 0.940849 |
| 5 AfterSolver | Ground | 0.200000 until post-solve | -54.692825 | 0.449567 |
| 6 BeforeSolver | WipeoutGround | 0.200000 | -54.692825 | 0.449567 |
| 6 AfterSolver | WipeoutGround | 0.200000 until post-solve | -54.708385 | 0.300678 |

All recorded Processed wheel counts for these early frames are zero. The
board falls about 0.1667 metres during the first solve and increasingly
rolls before WipeoutGround is selected during frame 6. The retained
`Simulation::Initialize` gravity is `-600` units/s squared; a 1/60-second
semi-implicit gravity step from rest would yield `-10` units/s and a
`-0.1667` displacement, so that first drop alone is not evidence of an
integrator error. The first-frame wheel
line height is published only in post-solve, which is why the `AfterSolver`
row still shows the constructor value. All `physout-root` values sampled at
tick observer stages are likewise from the previous post-solve fill; the
same-frame `Produced` submitted root and `Published` palette rows are logged
separately. This places the cause of the visible fall in the retained
board/physical path before final rendering, but does not yet identify a
contact, velocity, or constraint step that differs from the native build.

An independent IDA construction audit in
`native-initial-wheel-contact-source-2026-09-23.md` confirms the successful
initial wheel/reset values and Ground selector order. It found a separate
failure-prefix ordering mismatch on island routing, which is not the cause
of this successful live boot. The next discriminating trace needs the first
two frames' solver inputs: board linear/angular velocity before integration,
terrain contact candidates/manifolds and normals, contact impulses, and the
wheel-line post-solve hit results. Compare them with a native trace if one
can be obtained; do not tune the board pose from the screenshot.

## Follow-up two-frame solver-count capture

An env-gated release `--live-completions 2` run after the independent trace
review completed in
`artifacts/controlled-skating-release2-contact-counts-2026-09-23.log`.
Before frame 1's solve, deck linear velocity was `[0,0,0]`; after that solve
it was approximately `[0.0038,-10.0001,-0.0047]`, consistent with the
gravity-step explanation. The ground job processed two assemblies, visited
12 meshes, gathered two ranges and submitted two triangles, but the compiled
frame had **zero contacts**, 28 joints and 51 drives. Before frame 2's solve,
deck Y velocity was still about `-10`; its ground job submitted four
triangles and compiled **13 contacts**, 28 joints and 51 drives. After that
solve deck Y velocity was about `-5.436` and the board up-axis Y had fallen
to `0.94085`. Thus terrain work runs and contacts do appear on frame 2;
the missing information is their body/part assignment, normals, impulses
and whether they should support the board without excessive roll. The
counts do not prove those 13 rows were all deck-ground contacts.

A further two-frame release trace in
`artifacts/controlled-skating-release2-contact-spies-2026-09-23.log`
correlates board Part ordinals 0-6 with rigid-body IDs 1-7. Its frame-2
completed batch still has 13 compiled Contact rows, but only five solved
Contact spies with positive exported force: three rows bind board Part 6
(body ID 7) to static terrain (ID 0), and two bind other bodies (IDs 25 and
29) to terrain. No Part 0-3 wheel body occurs in those exported positive
spies. The deck's strongest exported normal force is about 3,279, while the
two other bodies report about 23,486 and 24,190. These are source-format
post-solve force projections, not direct impulse magnitudes. The absence of
wheel bodies in this *filtered* spy output does not prove there were no
compiled wheel Contact rows among the 13, or that native physics would
support those wheels through Contact rather than wheel-line forces. A
targeted source audit is checking that support path before implementation.

The later part-position trace
`artifacts/controlled-skating-release2-wheel-parts-2026-09-23.log`
records all four wheel centers at Y `-54.3375` before frame 1, with deck
Part 6 at `-54.281002`. The original wheel sphere radius is about `0.031`,
and the terrain line hit near this spawn is Y `-54.431`, placing the
initial wheel spheres roughly `0.0625` above ground. Before frame 2, the
wheel center Ys are about `-54.4868` to `-54.5226`; the board has moved
into the ground-contact region. The original wheel geometry and Rust
startup path were independently audited in
`wheel-spawn-geometry-2026-09-23.md`. This geometry makes frame-1 zero
contacts plausible but does not explain the frame-2 solved wheel support.

The ground-producer row trace
`artifacts/controlled-skating-release2-ground-records-2026-09-23.log`
resolves the 13 compiled frame-2 rows before contact import: nine rows
come from board assembly volumes 11, 13 and 15, all bound to board Part 6
(body ID 7) and static terrain; four rows come from the skeleton assembly
and bind bodies 24, 25, 28 and 29 to terrain. There are **no wheel-body
rows** in this producer output, so their absence from the later filtered
solved spies was not merely an exporter filter. This establishes Rust
producer behavior at this spawn, not whether native ground collision is
supposed to include wheel spheres. A targeted IDA eligibility audit is
checking volume flags and ground-job gates before any physics change.

The optional `SKATE2_TRACE_WHEEL_GROUND` capture in
`artifacts/controlled-skating-release2-wheel-gates-2026-09-23.log` now locates
the Rust rejection. In frame 2, the board ground Run has two flat triangles
at Y `-54.431004`. Each of the four wheel volumes is enabled, unfrozen,
non-Aggregate, non-static (body state 12), and not culled (Part group 2,
ground group 0). Each of the eight wheel/triangle attempts fails the inclusive
triangle/Part-bbox overlap before velocity-dependent contact padding or narrow
phase. Decoding the logged `AabbWords` shows wheel Part 0..3 maximum Ys of
`-54.45053`, `-54.48543`, `-54.46631`, and `-54.46747` respectively; the
highest lies about `0.0195` below the two triangle planes. The bbox is the
source-shaped swept Part bbox, not a sphere radius plus the later narrow-phase
padding. The source audit in
`wheel-ground-contact-eligibility-2026-09-23.md` found no original wheel-type
exclusion. This proves the local Rust reason for missing wheel rows, but not
whether native frames at the same spawn would use identical transforms,
triangles, bounds, and contacts. The trace's triangle ordinal restarts per
Run batch, and it does not observe assembly-level skips or contact reservation
failures; those limits do not affect this observed overlap rejection.

An assembly-level capture using the same flag is in
`artifacts/controlled-skating-release2-first-ground-query-2026-09-23.log`.
For frame 1 (`SimulationRigidBodyFrameIdentity(2)`), board assembly 0's
refreshed Header bbox has minimum Y `-54.37005` and maximum Y `-54.23859`.
Three resident mesh KD roots overlap that overall box; meshes 0 and 1 return
zero KD ranges and mesh 2 returns one. After ripping, **board assembly 0
submits zero triangles**. The two frame-1 triangles in the earlier aggregate
ground-job report therefore belong to the other processed assembly, not the
board. Frame 2 (`FrameIdentity(3)`) submits two board triangles. This localizes
frame 1's missing wheel candidates before per-volume Run, but the current
trace does not distinguish an empty/malformed-zero KD descriptor from rejected
ripped triangles inside mesh 2. The flat plane Y `-54.431004` is about
`0.06095` below the frame-1 board bbox minimum, consistent with a triangle
overlap rejection for that plane; the full candidate triangle list was not
captured. The native source query order is in
`native-first-frame-ground-query-gates-2026-09-23.md`; no native first-frame
Header or triangle trace exists for comparison.
