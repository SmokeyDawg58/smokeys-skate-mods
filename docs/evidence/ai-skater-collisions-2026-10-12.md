# Player and AI skater collisions — 2026-10-12

Scope: PC; physical collisions with AI
skaters, including running into them and impacts/bails during deathraces, using
the original debug executable as the reference. Android was not edited.

## Confirmed omission and correction

The shared PC collision frame had skater/ground, skater/DMO, skater/pedestrian
and skater/vehicle workers, but no skater/skater worker. This was an explicit
earlier deferral recorded in
`island-culling-toe-targets-crouch-graph-2026-09-26.md`; adding more AI did not
automatically produce contacts between their physical assemblies.

The frame now runs the original same-pool assembly worker over the complete
SkaterPool after skater/DMO and before skater/pedestrian. It includes local
players, ambient AI, challenge AI, and each assembly's own self-collision path.
No distance-triggered bail or substitute body capsule was added.

`SelectedPlayerTick` supplies the linked native switch to the shared frame.
This is independent of the optional LivingWorld/DMO frame settings and therefore
also works when those managers are absent. Both gameplay and front-end world
callers already use this selected tick. Isolated collision-component helpers
retain their explicit exclusion of this extra pipeline.

## Native reference

| Owner / function | Address | Recovered behavior |
| --- | --- | --- |
| `Sk8::Physics::Island::Step_Collision1` | `0x827b3818` | Flags 3 submits ground, DMO, skater/skater, pedestrian, vehicle in that order; adjacent board/skeleton partition pairs are separate. |
| `Island::InitWorld` | `0x827b42c0`, raw `0x827b4bf4..4c28` | Constructs the same-pool SkaterSkater pipeline with **two assemblies per job**. |
| Same-pool `AssemblyAssemblyCollisionPipeline` constructor | `0x82e0abd8` | Both sides are the same pool; intra-pool flag is true. Max jobs comes from pool capacity. |
| `AssemblyAssemblyCollisionPipeline::Params` | `0x82e0ace8` | Additional padding **0.0**, `allowMultipleContactWrites=false`; InitWorld changes only the job count for this pipeline. |
| `AssemblyAssemblyCollisionPipeline::Run` | `0x82e0bef8` | Empty pools produce no jobs; skips jobs whose A assemblies are all static; advances B with A for a same-pool job. |
| `Sk8::Physics::AssemblyAssemblyCollisionJob` | `0x827cc410` | Same-pool ordering, assembly AABB/group/static/frozen filters, own culling table for self pairs, pair partition and thaw publication. Decompilation fails; raw PPC inspected and saved. |
| `RunIntersectionTests` | `0x827cbdc8` | Enabled primitive volumes, distinct-body/self-table or cross-assembly group-table filtering, original material combination, RenderWare contact points, validity check. |
| `TempContactBuffer::GetNextFreeContact` | `0x827cb7e8` | **50** local contacts; when multiple writes are disabled, full buffer returns null. Worker `0x827ccaf0..b1c` flushes and resets after each A assembly. |
| `kRunSkaterSkaterCollisionPipeline` initializer | `0x82f6a170` | `Physics_Collision_Pipelines`, label `kRunSkaterSkaterCollisionPipeline`, title `Run Skater Skater Collision Pipeline`, true, null callback. Ref `0x8305256c`, initializer table `0x82fd2210`. |
| `Island::InitCullingTable` | `0x827b8c10` | Shared 19-by-19 table; a set bit rejects a pair. Normal board/skeleton groups 4/6 are culled; skeleton/skeleton 6/6 and board/board 4/4 are admitted for flags 3. |
| `SkeletonCollision::Update` | `0x82b4b8f0` | Skips self-contact for collision reactions; opposite group 6 uses player/AI-specific Vault force multipliers and publishes `mSkaterContact`. |
| `Wipeout::CheckForGroundWipeout` | `0x82de09c0` | Original invincibility timer and conditional player skater-contact threshold scalar, then existing displacement, body/arm force, deck and balance checks. |
| `CheckForOffBoardWipeout` / `CheckForSkeletonCollisionWipeoutAir` | `0x82ddec70` / `0x82dd8c90` | State-specific existing walking/running and airborne bail consumers. |
| `PhysicalPlayerHiLOD::UpdateCollisionStatus` | `0x82e016a0` | Existing world-only/restoration logic, including nearest same-flags skater clearance and LivingWorld overlap checks. |
| `PhysicalPlayerHiLOD::DisableNonWorldCollisionsForFadeout` | `0x82dffe40` | Existing board and skeleton world-only filtering during fade-out. Actor wrapper `0x825ed3d0`. |

Params layout from the PDB: job count +0, additional padding +4, multiple-writes
byte +8, debug-shape provider +0xc. `SkeletonCollisionUpdateInfo.mIsAISkater`
is byte +0x56; raw `0x82b4c29c..334` verifies the multiplier selection and keys.
The ground player-only scalar gate was checked in raw `0x82de0ad8..b2c`.

## Reused contact and bail chain

The existing assembly worker now accepts an explicit multiple-writes setting.
DMO/vehicle/pedestrian callers retain true; only the restored skater/skater
wrapper uses the original false setting. It otherwise shares the existing
primitive intersection, exact culling-table inputs, material selection, contact
validation, global contact capacity, pair partition and thaw mechanisms.

New contacts enter the same Horus solve and solved-contact spies as other
physical contacts. `physical_player_contact_list.rs` publishes both sides and
marks same-body/same-owner contacts. `skeleton_collision_update.rs` consumes
those actual contacts; `selected_player_tick.rs` performs post-physics updates
for **every** retained skater, rather than only the selected player.

The existing normal/ragdoll self tables, disabled volumes, world-only groups,
static/frozen filtering, ground invincibility, state-specific bail thresholds,
and AI/player force differences remain authoritative. The primitive dispatcher
already implements the sphere/capsule/box/cylinder combinations this route uses.
There is no deathrace-only enable check in native Collision1; deathraces use
the common physical flow.

Original Xbox `data/db/db.big` → `skater` Vault inspection confirms these
existing runtime lookups (values are **not** copied into gameplay code):

| Collection | Key | Shipped value |
| --- | --- | --- |
| `physics_collision/default`, AI group-6 force multiplier | `98dbb06a81ec4f67` | 1.0 |
| `physics_collision/default`, player group-6 force multiplier | `3f3079e086067292` | 0.5 |
| `physics_wipeout/default`, `SkaterSkaterThresholdScalar` | `31aba15bf262ec6f` | 1.5 |
| `physics_wipeout/default`, ground body-contact threshold | `d50ac97307b0117c` | 8.5 |
| `physics_wipeout/default`, ground arm-contact threshold | `744bb592dd7e0cfb` | 30.0 |

These are intermediate force/threshold values, not a universal collision-speed
threshold. State, impact direction, body part and existing modifiers still apply.

## Files and review

Production edits:

- `physics/collision/simulation_dmo_dmo_contact.rs`: original skater/skater
  wrapper and parameterized buffer flushing.
- `physics/board/runtime/skateboard_ground_frame.rs`: submission and integration
  of contacts, pairs and thaws at the original stage.
- `session/selected_player_tick.rs`: live native switch supplied each selected frame.
- `input/tuning/input_cvar_runtime.rs` and `input_cvar_living_world_dmo.rs`:
  recovered cvar definition, registration and typed accessor; existing API indices retained.
- `physics/solver/contacts/simulation_contact_append.rs`: corrected stale comment
  about the assembly/assembly flush boundary; buffer implementation unchanged.

Validation performed:

- Live IDA pseudocode, raw PPC, type-layout and original-data inspection above.
- Scoped comparison against pre-edit snapshots; all call sites and both contact
  sides reviewed. Other dynamic-pair callers retain their prior settings.
- Rust syntax parsed for all six modified source files without rewriting them.
- `cargo check -p skate2-core --lib -j 1`: **passed**, exit 0, 1m 02s; compiler
  emitted 435 warnings. This was a type check, not a release executable build.
- No tests added or run. No gameplay launch or deterministic reference collision
  trace was performed. The release executable needs rebuilding for playtesting.

Confidence: **exact** for native switch/defaults, pipeline parameters, branch
ordering and recovered filter/buffer rules; **high** for the connected source
flow. In-game collision feel and per-tick numeric parity remain unverified.
The shared host solver/narrow phase retains its existing floating-point limits;
host job/assembly allocation ordering is not a claim of Xenon scheduling parity.

Evidence packet: `.scratch/ai-skater-collisions-20261012/` contains pre-edit
snapshots, scoped diff, native full functions/raw PPC, impact tuning, syntax
review and the successful compiler-check log.
