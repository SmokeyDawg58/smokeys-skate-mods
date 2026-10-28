# Authored seven-part board into the existing assembly transaction

2026-09-07. Implementation, independent review and mounted validation pass. Confidence:
`high` for the source-to-owner mapping; exact source keys/stores and hashes
are retained in the joint, initial-placement, wheel, truck and deck packets.
This is local seven-body/six-joint admission, not complete SkateboardBody,
SkaterPool publication, collision integration, a native tick or playable skating.

## Source producer and reused consumer

`SkateboardBody::Init` (82b374e8) allocates seven definition parts, a byte-selected
zero/six joints and zero drives. It creates wheels, trucks, deck, conditional
joints and the culling table; then `Assembly::Initialize` (82ae1338) invokes
`InitializeComponentsFromDefinition` (82ae0bf8). Parts retain their own inertia,
material, volume and optional inverse-body frame. Joint records retain ordered
parent/child references and distinct frame/limit slots without an additional
COM transformation. `InitializeTransforms` (82b2bc10) supplies initial local
Part matrices before the outer constructor admits the board as ACTIVE=4.

The new crate-private `skateboard_assembly.rs` bridge takes the actual existing
wheel/truck/deck owners, six source-created joint records and original culling
table. It appends exactly four wheel payloads, two truck payloads and one deck
payload in definition order. Each receives a separate retained semantic inertia
allocation, its original material, and source inverse-presence/static metadata.
The actual collision allocations remain in an Arc-owned definition. This is
immutable safe-host lifetime policy, not literal native addresses or live
attribute-edit alias behavior. No serialized bytes or asset identifiers are
generated as a substitute for authored ownership.

`DynamicAssemblyRuntimeTemplate` and `DynamicAssemblySession` now accept a
retained collision-owner type, defaulting to the existing clustered-mesh type.
The original public clustered-mesh constructor remains unchanged. The new
crate-private authored constructor receives sealed equal-length Part metadata
and payloads, validates counts and all joint/drive indices, then retains the
actual owner. Its generic validation cannot establish source correspondence;
the skateboard producer explicitly supplies that seven-part mapping.

The original transaction implementation is unchanged: body, joint, drive
attachment; stage-specific rollback; forward body/joint/drive removal and
native pose copyback. `SimulationAssemblyBackend` supplies the same existing
body and joint pools. This avoids a parallel lifecycle implementation and
does not pretend authored primitives are `DynamicClusteredMeshTemplate` data.

## Numeric reuse

Joint frame setters (82b22f58,82b23108,82b232b8) reuse the now-separated
`world::try_simulation_quaternion_candidate`. Each setter invokes one vector
estimate callback for all four candidate magnitudes, then two vector refinement
rounds and strict trace/diagonal selection. No final normalization is added.
The pre-existing public body conversion still applies its original subsequent
normalization and remains covered by its earlier bit-oracle tests.

Root added an analytic discriminator: identity with reciprocal-root seed1/4
refines to11/32 then7117/16384, yielding candidate W7117/8192=3f5e6800, not1.
A second test proves estimate error propagation without scalar retries/fallback.
These are controlled host arithmetic tests, not captured native estimate bits.

## Explicit remaining integration boundaries

- The new bridge is not full Init: temporary collection lifetime, the separate
  wheel-distance read, material caches and group/userdata publication are not
  silently represented by a complete native constructor claim.
- Source initial transforms are local; the outer spawn transform happens after
  truck drives and mandatory hook construction. No arbitrary terrain placement
  is substituted for that final spawn operation.
- The native definition has zero drives at this stage. The later two mutable
  truck drives are now implemented via a separate post-admission owner, not
  definition-drive insertion. Optional wheel drives and the separate static
  HookPool body/drive still follow. Zero definition drives do not mean these
  later behaviors are optional.
- Collision geometry and the49-bit self-culling table are retained, but the
  authored source is not yet routed into the arena-only serialized collision
  template, packed Add-time inertia authority or island pair producer. There
  are19 shipped primitive occurrences, not19 distinct volume allocations.
- The existing Part/body conversion has documented finite-XYZ/host-estimate
  boundaries. Detached initial Part matrices preserve raw W lanes; body Add
  consumes them through its existing orthonormalization. Full raw four-lane
  GetTransform and exceptional Xenon arithmetic remain unverified.
- Host preflight/allocation/authority checks and returned-owner publication are
  safety policy, not native allocator failure or partial callback visibility.

The expanded raw mounted corpus checks source joint words, distinct six frame
and limit owners, original initial placement, retained collision/culling and
owner lifetime, invalid-pair preflight, insufficient-body/joint rollback,
successful body/joint endpoint records and explicit teardown. It calls the
independently sampled CalcTruckTransforms only after successful local admission.
No solver-frame, SkaterPool or complete freeroam claim follows from these checks.

Strict checkpoint command59159 completed exit0: formatting check,2294 passing
unit tests (2066core+187formats+38app+3symbols),6 doctests and warnings-denied
all-targets Clippy. All four core original-data tests were then explicitly
enabled and passed; the app ignored corpus was not rerun. Twenty new focused
tests cover joints10, initial placement/culling8 and shared numeric candidate2.

Later truck-drive checkpoint: dynamic_assembly_post_drives.rs retains
constructor-created handles privately in this same persistent session. Its
per-Add authority/index/storage preflight distinguishes host backend errors
from capacity null and preserves earlier successful handles. Ordinary detach
refuses until explicit forward truck removal; native-fault null slots are
skipped only as documented host recovery. The board borrow adapter never
exports body/drive capabilities. Native payload Arcs remain in free slots until
reuse/pool destruction, and the tests check that lifetime.

The mounted corpus now creates2drives after CalcTruckTransforms, then reads
their actual pool frame/dynamics owners and checks original Vault-derived
coefficients and deck6->truck4/5 endpoints. Twelve constructor/numeric tests and
four ownership-extension tests pass separate independent audit. Final gate4281:
2310unit+6doctests, fmt, warnings-deniedClippy and all4core original-data tests.
No live board constraint solve, contact tick or complete spawn claim follows.

## Later managed constraint-frame checkpoint

The no-solve statement above describes the earlier truck-drive checkpoint.
The mounted corpus now also executes four actual pool-backed board constraint
frames through the managed owner in `simulation_assembly_constraint_session.rs`.
Each frame builds six joints and two drives, solves them with existing Horus
and integrates the same seven bodies. Separate zero-iteration and fifty-iteration
owners produce different poses; actual RF rows clear and Vault gravity returns.
The manager static body is created before board admission. Full evidence and
numeric limitations are in `authored-constraint-solver-path.md` and
`drive-normalization-source.md`. Strict gate 71989 passes 2325 unit tests,
six doctests, Clippy and all four original-data core probes.

These are contactless, pair-free, non-Hook component frames, not complete spawn,
the native whole Tick, authored world collision or playable skating. The next
required path was the separate source HookPool assembly and cross-assembly drive.

That next component now passes: retained source hook definition, separate Static
body, manager-bound membership/shared index allocation, actual third deck-hook
drive and membership-backed exclusion feed four additional real solver frames.
The fresh hook is source-disabled, verified through the actual post-Horus record
and unchanged board motion versus the truck-only control. Explicit parent-drive,
hook, board teardown passes. See `skateboard-hook-source.md` for full proof and
host safety boundaries. Gate61189:2356unit+6doctests and all4core mounted probes.
Original in-place spawn, board SkaterPool and authored collision publication
remain missing; this does not establish complete constructor or Tick parity.

## Later live relocation and authored-ground checkpoints

Original in-place board/hook relocation now preserves live body occurrences,
motion and constraints; see `skateboard-spawn-transform-source.md` (2379).
The same retained board now additionally publishes its real19 primitive
occurrences into original resident-ground collision, producing22 actual contacts
over four frames alongside six joints and three drives. The scoped semantic-XYZ
owner and unchanged serialized/image route both feed shared existing consumers.
See `authored-skater-ground-publication-path.md` for independent audit, original
source addresses, exact Create/Update rules and numerical/host boundaries.
Gate90328 passes2384unit+6doctests and all4core original-data tests.
Full SkaterPool registration is still missing: board-before-hook shared world
indices and the real skater counterpart/pair scheduler are the next integration,
not a claim already established by these controlled component frames.
