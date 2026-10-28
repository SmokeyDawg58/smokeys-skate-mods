# Native initial wheel/contact source and first Ground selector

2026-09-23 read-only audit of the live matched `sk82_na_m.xex.i64` IDA/PDB,
registered Rust, and existing controlled capture. Confidence is **exact** for
the identified native stores, call and branch order; **high** for the split Rust
ownership comparison. No native post-spawn per-tick trace was captured. The
existing Windows release log is a Rust trace and cannot establish the native
first-tick outcome.

## Constructor state before any wheel batch

`Sk8::Physics::SkateboardBody` constructor `0x82b382dc` calls
`SkateboardBody::CollisionInfo::Reset` (`0x82b23630`) before its
`GetPhysicsIsland` call at `0x82b382e8`, later `Init` at `0x82b387e8`, and
spawn `SetTransform` at `0x82b388b4`. The Reset itself writes, in order:

| Native store | Field/result |
|---|---|
| `0x82b236b0`, `0x82b236bc` | `CollisionInfo.numPartsInContact=0`, `numWheelsInContact=0` (Body `+0x360/+0x361`) |
| `0x82b236f0`, seven iterations | `inContact[0..6]=false` (Body `+0x34c` onward) |
| `0x82b23710` inside that loop | `wheelHeightToGround=+0.0` (CollisionInfo `+0x318`, Body `+0x358`) |
| `0x82b2373c..0x82b2375c`, four iterations | `lineTestNormal[0..3]=gJVector`, `wheelToGroundDist[0..3]=+0.0` (`0x82b23748`), `contactNormalValid[0..3]=0` (`0x82b2374c`) |

The retained line batch is set null at `0x82b382fc`. These zeros are
constructor output, not evidence that a terrain ray missed or a solved contact
frame completed. `StartWheelLineTest` (`0x82b24530`) creates four wheel lines
and one deck line during `Skateboard::UpdatePrePhysics` (`0x82b37cfc`);
`EndWheelLineTest` (`0x82b24700`) follows the solve at `0x82b37d48` before
`SkateboardBody::UpdatePostPhysics` at `0x82b37e5c`. End first Syncs, writes
height `0.2` at `0x82b24764`, then resets each wheel normal to `gJVector` and
distance to `+0.0` **before** reading that wheel's result (`0x82b247a0..b8`).
Each hit can reduce the height and publishes its own normal/distance; a null
result retains that wheel's defaults. The line result alone does not set
`inContact`. Body PostPhysics traverses solved contacts and later publishes
wheel count at `0x82b372ec` and the no-wheel timer at `0x82b3749c..b8`.

The separate `ProcessedPhysIn` is constructed by the HiLOD path and its
constructor calls `ProcessedPhysIn::Reset` (`0x82b39098`) at `0x82e00f78`.
Its Reset writes the whole flags word `+0x8c4=0x2000` at `0x82b39428`,
`+0x8c8=0` at `0x82b39430`, `+0x8d0=0` at `0x82b39478`,
`+0x8cc=0` at `0x82b39558`, `mNumWheelsInContact=0` at
`0x82b394f8` (`+0x90c`), and `mFramesSinceJump=100` at `0x82b39518`.
These are a second contact/count owner, distinct from Body `+0x361`.
`Update1_ProcessInput` (`0x82e01f98`) can subsequently copy prior PhysOut
contact fields into this same Processed object; the Reset snapshot alone does
not establish the first `Update2` snapshot after that processing.

## First ordinary Ground selection

`PhysicalPlayerHiLOD::Update2_SystemLogic` (`0x82e01c40`) calls
`CheckForStateChange` at `0x82e01c54` and clears Wipeout requests afterward
at `0x82e01c5c`. `PhysicalPlayerStateChanger::CalcSuggestedState`
(`0x82ddbd58`) first decrements a positive teleport countdown, stores current
state and clears two requests (`0x82ddbd70..94`); it then gets the actual board
Body and reads Body `+0x361` wheel count at `0x82ddbdac`. Its early Processed
reads are `+0x90c` wheel count, `+0x8c8` flags, and category
(`0x82ddbdb4..bc`). The local `somethingColliding` condition is Processed
wheel count **greater than zero**, or `+0x8c4` bit `0x10000`, or bit
`0x20000` (`0x82ddbde8..dec`). Body wheel count instead drives the two- and
three-wheel counters (`0x82ddbe88..bebc`) and handplant priority; those counts
must not be merged.

The common teleport, fall-reset cvar, and AI path checks precede the current
Ground branch. In the ordinary Ground branch, handplant/offboard/wipeout,
`framesSinceJump==1`, revert, and `CheckForGrind` precede the off-ground
test (`0x82ddc1f4..0x82ddc2a4`). When `somethingColliding` is true, the
Ground branch sets `IsOffGround=false` without calling the condition; when it
is false, it calls `ConditionIsOffGround` at `0x82ddc2ac`. A true condition
with zero teleport countdown selects Air `200` (`0x82ddc2cc`). Otherwise it
queries `Skeleton::IsSkateboardAnimated` before the remaining Ground decision.
These are branch predicates, not a claim that neutral native spawn meets them.

`ConditionIsOffGround` (`0x82dd78d8`) reads Body `+0x358` height, then the
live `physics_airstates.default` clear-height attribute, then the recent-height
attribute, then recaptures Body and reads `mTimeSinceWheelsTouching` at Body
`+0x1e00`, then reads the contact-grace attribute. Only after all three
attribute lookups does it return
`height > clear_height || (height > recent_height && time > grace)`.
The mounted Vault values recorded in `physics-state-machine.md` are `0.12`,
`0.06`, `0.2`; the runtime still fetches the current attributes. Strict `>`
means equality does not pass. The source-selected Ground-to-Air transition
then writes `framesSinceJump=100` before `SetPhysicsState` at
`0x82e005f8..fc`; the old Ground Exit and new Air Enter happen inside that
transition.

## Rust ownership and ordering comparison

The successful Rust path represents the same initial values in
`skateboard_contact_state.rs::SkateboardContactState::reset`,
`skateboard_wheel_lines.rs::reset_constructor_collision_output`, and
`skateboard_body_construction.rs::SkateboardBodyPostFields::from_constructor`.
It keeps Body wheel count in `post_fields`, Processed wheel count/flags in
`skeleton_processed_physics_state.rs` and
`skeleton_processed_jump_events.rs`, and selects on Processed collision flags
in `physical_player_system_logic_session.rs`. That selector reads Body wheel
height from the retained `wheel_lines` owner and Body timer from
`post_fields`; its normal Ground path checks `!colliding`, off-ground, and
zero teleport countdown in source order. The generated wheel-line and
post-physics owners run after Update2, not as a fabricated early query for it.

There is a **construction failure-prefix divergence**. Native's single
`CollisionInfo::Reset` finishes the wheel height/normals/distances before
`GetPhysicsIsland`. Rust `physical_player_assembly.rs::try_construct_board`
resets `contact_surface.collision_state`, then calls
`require_simulation_island`, and only on success invokes
`board_construction.try_construct`, which resets `wheel_lines`. Therefore an
island-routing failure retains initialized contact/count fields but unknown
wheel-line fields in Rust, where native had already written the wheel fields.
On the successful route both projections receive the source values. This
failure prefix is **not** a proven cause of the completed live boot's Air
transition.

The existing controlled log
`artifacts/live-contact-trace-release-2026-09-23.log` reports, after Rust
frame 1, Ground; Body and Processed wheel counts zero; four wheel distances
and height `0.14285605`; Body no-wheel time `0.016666668`; teleport countdown
zero. With live thresholds `0.12/0.06/0.2`, Rust's frame-2 off-ground branch
is explained. It does not show the corresponding native state. In particular,
command-3 `SkaterManager::OnCommand` calls `AdjustSpawnPosition` at
`0x82600998` before constructing the new Actor at `0x826009b4`, and the
offline adjustment depends on the live `kbEnableAdjustSpawnPosition` value
and its World line result (`0x825f1a20`). The initial native transform and
wheel geometry therefore cannot be inferred from the Rust unadjusted capture.

## Discriminating native measurement

Capture one neutral command-3 native spawn with the actual cvar and World line
result, then at constructor return, immediately before first and second
`CalcSuggestedState`, and after each `EndWheelLineTest`/Body PostPhysics record:
spawn matrix translation and board Part 0..3/6 positions; Body wheel height,
four distances, four `inContact` bytes, Body `numWheelsInContact`, no-wheel
timer; Processed `+0x90c` count and `+0x8c4/+0x8c8/+0x8cc/+0x8d0` words;
teleport countdown and chosen state. Compare the same stage boundaries with
the adjusted-spawn Rust trace. This directly distinguishes spawn placement,
line hit/miss, solved contact publication, Processed copy timing, and selector
logic without inferring native behavior from a visual pose or a single final
state.
