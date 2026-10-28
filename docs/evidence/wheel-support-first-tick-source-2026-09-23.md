# Wheel support on the first adjusted-spawn tick

2026-09-23 read-only audit of the matched `sk82_na_m.xex.i64` IDA/PDB,
current Rust, and the two-frame release logs. **Exact** below identifies native
calls/stores and their order at stated addresses. **High** identifies the
implemented Rust route and the meaning of its logged fields. The proposed
native first-tick outcome remains a **hypothesis**: there is no matched native
adjusted-spawn contact/velocity trace. No code or Cargo command was changed or
run for this audit.

## Native sequence: a line hit is not a support impulse

`PhysicalPlayerHiLOD::Update3_State` calls the selected state's Update,
`SkateboardController::Update`, then `Skateboard::UpdatePrePhysics`
(`0x82dfe7d0`, `0x82dfe7d8`, `0x82dfe808`). The freshly decompiled
`Skateboard::UpdatePrePhysics` (`0x82b37cb8..0x82b37d20`) copies the current
Reckoning `up` at `+0x1b0`, gets the actual Body, calls
`SkateboardBody::StartWheelLineTest` at `0x82b37cfc`, then
`Skateboard::ApplyForces` at `0x82b37d04`. `ApplyForces`
(`0x82b35380..0x82b355a0`) walks the queued force records if nonempty and
accumulates force/torque on the deck rigid body. It does not read wheel-line
results or generate a wheel contact.

`StartWheelLineTest` (`0x82b24530..0x82b246fc`) acquires and begins one
island line batch with flags `7`, adds tests from the actual Part 0..3 centers
to `position - up * 0.2`, adds a separate fat world-down deck test from Part 6,
then calls batch `End` at `0x82b246ec`. That submits the queries; Start has no
`GetResult`, contact append, force, or body-velocity store. The batch can do
work across the solve, but the first code in this board route that consumes
its five results is `EndWheelLineTest` (`0x82b24700..0x82b24878`).

`Skateboard::UpdatePostPhysics` calls that End method at `0x82b37d48`, before
`SkateboardBody::UpdatePostPhysics` at `0x82b37e5c`. End `Sync`s, sets height
to `0.2`, then reads each wheel result. On a hit it writes the wheel's
normal and `lineParam * 0.2` distance and reduces the stored minimum height;
on a miss its initialized `+Y` normal and zero distance remain. It also
records the deck result, then releases the batch. It does **not** set Body
`inContact[0..3]`, append a solver Contact, or apply a force. In Body
PostPhysics, a missing wheel contact can use the line normal if its distance
passes the live threshold (`0x82b36ee4..0x82b36f24`); that fallback does
not set `inContact`. Body wheel-contact count is subsequently published at
`0x82b372ec` from contact processing. These separate paths are detailed in
[skateboard-wheel-line-source.md](skateboard-wheel-line-source.md) and
[skateboard-body-ground-normal-source.md](skateboard-body-ground-normal-source.md).

Therefore, **exact for the inspected board call path**, a current tick's wheel
line hit cannot directly support that tick's earlier solve. Its post-solve
height/normal can affect later state or control inputs; this audit does not
rule out such next-tick effects. The constructor's zero wheel height and
distances likewise precede any completed line batch; see
[native-initial-wheel-contact-source-2026-09-23.md](native-initial-wheel-contact-source-2026-09-23.md).

## The separate route that can support the first solve

The board has authored collision volumes for Parts 0..3 (sphere wheels),
trucks 4..5, and deck 6. Native SkaterPool publication feeds the skater-ground
Assembly/terrain collision pipeline, which can emit Contact rows from volume
against terrain geometry before the solver. That route uses volume bounds,
triangle ripping/narrow phase, Contact compilation, island labels and Horus
constraint solving; it is separate from the thin wheel lines. After solving,
the Body's contact-list traversal classifies contacting Parts and publishes
wheel `inContact`/count. Source ownership and ordering are recorded in
[authored-skater-ground-publication-path.md](authored-skater-ground-publication-path.md),
[assembly-ground-run-integration.md](assembly-ground-run-integration.md), and
[flags3-collision2-source-gap-2026-09-23.md](flags3-collision2-source-gap-2026-09-23.md).

For flags `3`, native `Step_Collision1` appends the board/Skeleton assembly
pair and submits skater-ground Run when its cvar permits; `Step_Collision3`
synchronizes it, and `Step_Solver2` partitions assemblies before solving
(`0x827b3818`, `0x827b4060`, `0x827b35b8`). The pair append does not
produce a terrain Contact, and flags-3 `Step_Collision2` is a no-op. Whether
native first-tick wheel volumes overlap the actual adjusted-spawn terrain,
and whether their constraints carry positive support force, is **unmeasured**.
The source route makes first-solve support *possible*; it does not prove a
first-tick wheel Contact or a stationary native deck.

## Current Rust mapping and observed two ticks

`physical_player_update3_prephysics_tail.rs::try_finish_update3_prephysics`
retains the native order: capture Reckoning up, start the actual five-test
wheel batch, apply queued board forces, and store force displacement.
`selected_player_tick.rs::try_step_queued` then reaches `BeforeSolver`, calls
`skateboard_ground_frame.rs::try_step_authored_ground`, and only after
`AfterSolver` enters the board post-solve route. The latter calls
`try_end_board_wheel_lines` before Body contact processing
(`physical_player_board_post_outer_live.rs`). The Rust line result is therefore
also unavailable as a direct frame-1 solver support force.

The Rust ground job refreshes the authored board/Skeleton publications,
traverses retained terrain, compiles Contact/Joint/Drive rows, runs Horus,
integrates bodies, then exports contact spies and physical-contact lists.
Its `skateboard_ground_frame.rs` frame currently uses a synchronous ground job
and has no native adjacent assembly-pair append, pair-buffer partition, or
full asynchronous pipeline submission/synchronization. This is a source
schedule/coverage gap, **not** evidence that the absent pair removed a
board/terrain narrow-phase Contact: Active bodies may already carry the same
island label for a single pair. The pair-label and pipeline differences require
their own comparison before assigning them a first-tick force effect.

The adjusted-spawn Rust release capture is specific:

| Boundary | Direct observation |
|---|---|
| Frame 1 `BeforeSolver` | Part 6 at Y `-54.281002`, deck velocity `[0,0,0]`; constructor wheel height/distances still zero. |
| Frame 1 compiled solve | 2 assemblies, 12 meshes, 2 submitted terrain triangles; **0 Contact**, 28 Joint, 51 Drive rows. |
| Frame 1 `AfterSolver` | Part 6 Y `-54.44767`, deck Y velocity `-10.000123`. With retained gravity `-600` and `dt=1/60`, the first-order gravity increment is `-10`; this agreement is a calculation, not a native comparison. |
| Frame 1 post-solve / frame 2 `BeforeActor` | Four line distances and height `0.093501695`; Body no-wheel timer `0.016666668`; Processed wheel count zero. These four hits were published **after** frame 1's solve. |
| Frame 2 compiled solve | 4 submitted terrain triangles; **13 Contact**, 28 Joint, 51 Drive rows. |
| Frame 2 positive solved spies | Three Contact force projections bind board Part 6/body ID 7 to static ID 0; two bind bodies 25 and 29 to static. No Part 0..3/body IDs 1..4 occurs among these **filtered positive** spies. |

The first five rows come from
`artifacts/controlled-skating-release2-contact-counts-2026-09-23.log`; the
body-ID/positive-force rows come from
`artifacts/controlled-skating-release2-contact-spies-2026-09-23.log`.
`counts.contacts=13` is the compiled row count, whereas the five spies are
positive exported force projections. The latter do not establish that all
13 rows lack wheel bodies. Nor does a line hit mean the wheel sphere touched
the terrain or that Body marked it `inContact`.

## Earliest measurable parity boundary

First capture the *same* adjusted spawn in native and Rust: command-3
adjustment cvar/World line result, Part 0..3/6 matrices, wheel radii/volume
bounds, and initial body velocities. This is the earliest boundary overall;
otherwise different initial geometry can account for different contact rows.
For the **first-solve support question**, the earliest decisive boundary is
after native skater-ground Run/Sync and Contact compilation but before solver
integration on tick 1: record each board/terrain Contact's Part/body IDs,
normal, separation/bias, island label and solver input, plus deck velocity and
force. Match Rust's `BeforeSolver`/compiled-Contact boundary. Then compare
post-solve velocity and normal impulse/force projection, and only afterward
compare line hits and Body `inContact` at post-physics. Repeat on tick 2 if
frame 1 agrees.

If native tick 1 also has zero board/terrain Contacts and a roughly `-10`
deck Y velocity, Rust's initial drop is compatible with the inspected native
path. If native has wheel Contact rows or materially different force/velocity,
the first divergence precedes or lies inside contact compilation/solve; the
post-solve wheel-line values cannot repair it. These are **conditional
inferences**, not measured native outcomes. No wheel-force tuning or synthetic
Contact insertion is justified by the present trace.
