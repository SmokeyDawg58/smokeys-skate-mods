# Authored skateboard constraint producer-to-solver path

Original 2026-09-07 read-only audit of the actual Rust code and its retained source
contracts. No implementation or Cargo execution was performed for this packet.
Confidence: `exact` current API/visibility and ownership restrictions; `high`
proposed composition of the already-reviewed source graph. This document is
not itself a constrained-frame claim. The later implemented/verified checkpoint
at the end supersedes the missing-facade/normalization statements in this audit.

## Outcome

Joint and Drive complete pool-backed Build composers and the shared Horus/RF
consumers already exist. They do not require a fabricated arena, serialized
asset, Xenon address binding, or raw synthetic Jacobian. The short missing
connection is a real frame-owning caller inside the existing backend/manager
authority, with genuine island-label production and sealed Contact ownership.
The private Build producers currently have only test-only cross-module adapters.

Root owns `skateboard_assembly.rs`, which retains actual wheels/trucks/deck,
culling, source joint definitions and initial placement through the generic
owned-source assembly template. Fresh post-admission truck drives are being
implemented independently. A 7-body/6-joint/2-drive attachment is a meaningful
producer, but attachment or snapshot inspection alone is not solver execution.

## Backend and manager lifetime must begin before body admission

`SimulationAssemblyBackend<M,JF,JL>` owns private:

```text
rigid_bodies: SimulationRigidBodyPool
joints: SimulationJointPool<JF,JL>
drives: SimulationDrivePool
```

The exact board instantiation is
`SimulationAssemblyBackend<AssemblyPartMaterial,SimulationJointFrames,
SimulationJointLimits>`. Its existing Add ports retain actual inertia Arcs,
joint frame/limit Arcs, and mutable drive frame/dynamics Arcs and validate body
occurrences. Public snapshots are observations, not inputs from which to rebuild
an equivalent authoritative pool.

`simulation_contact_jacobian::network_preparation::SimulationManagerContactDomain`
already implements the narrow source manager Contact/static-body/label domain.
Its historical network naming does not require network Contacts.

Constructor:

```text
SimulationManagerContactDomain::try_new(&mut body_pool, maximum_contacts)
```

It checks the native contact-byte count (maximum*0x100) and requires
`body_pool.try_is_manager_construction_pristine()`. It reserves Contact backing,
mints host manager identity, constructs `SimulationRigidBodyIslandState` with
every slot byte -1, THEN adds and retains the dedicated manager Static body:

```text
body_pool.try_add_rigid_body(IDENTITY_AFFINE,None,1)
```

Source Static Add does not retain its supplied inertia pointer; None expresses
that resulting state, not missing dynamic inertia. The manager retains the
actual returned handle and its occurrence. The body is not a board part, hook,
or caller-selected arbitrary static occurrence.

Therefore create this manager domain inside the pristine backend/owning manager
constructor BEFORE attaching the board. A backend already containing the seven
board bodies cannot validly bolt it on afterward. Avoid weaken-the-precondition
or remove/re-add-the-board workarounds.

Minimum capacities for the restricted current graph:

```text
bodies = 8: manager static1 + board dynamic7
joints = 6
drives = 2
```

The mandatory native hook later requires at least a ninth body (another static
body) and a third drive. Additional skater/skeleton/world bodies need further
capacity. Configure physics before gameplay Add; root owns actual source
configuration. Do not infer the global game's full capacities from these minima.

The domain methods/types are currently `pub(super)` inside the Contact module,
and its host Contact numeric backend is private. A controlled production caller
requires appropriately scoped visibility or a narrow facade owned there. Do not
duplicate manager static-body/label creation in the board module.

## Real frame and label production

Existing ordered APIs:

```text
body_pool.begin_dynamic_update_frame(timestep)
domain.try_start_island_frame_labels(&body_pool)
map = domain.try_island_map_snapshot(&body_pool)
phase = body_pool.try_prepare_dynamic_update_phase()
```

`begin_dynamic_update_frame` stores timestep/frequency and advances the host
frame occurrence. It is not full native StartFrame.
`try_start_island_frame_labels` calls persistent
`SimulationRigidBodyIslandState::try_start_frame_labels`: walk actual Active
right-link order assigning zero, then assign the retained manager static body
-1. Other slot bytes are not reset. State snapshots bind each current allocated
body occurrence to the persistent byte and same frame.

Do not construct `SimulationRigidBodyIslandMapSnapshot::try_new` with manually
assigned zeros merely because the board's bodies are connected. Use the source
StartFrame producer. Its all-Active-zero result is justified only for an
explicitly pair-free, non-Hook restricted frame. The offline full route later
does HookPool exclusion, drive normalization, optional thawing, eligible
assembly-pair partitioning and table construction. Those producers cannot be
replaced by a permanently zero map.

The returned dynamic-update phase exclusively borrows the live body pool. It
preflights body topology, Active inertia and the later integration plan, then
holds topology/configuration/body fields/RF storage stable through Build and
solver work. It is the SAME phase that must finally be consumed by integration.
Do not drop it and create another phase between building records and solving.

The map must be captured before this exclusive borrow. The map has frame and
body occurrence identity; the prepared batches additionally seal exact phase.
The real owner should split its distinct field borrows internally:

```text
&mut rigid_bodies -> one dynamic-update phase
&joints          -> immutable Joint batch/source lifetime
&drives          -> Drive snapshot and later binding revalidation
&mut domain      -> frame-local sealed Contact backing
```

No public mutable-pool getters or source-independent snapshot rehydration are
needed. Contact batches borrow the manager until dropped, so keep per-frame
preparation/solve values in a scope ending before the next manager frame.

## Genuine empty Contact branch, without synthetic input

An explicitly contactless constraint test can use the actual manager backing:

```text
open = domain.try_begin_contact_frame(&phase)
sealed = open.try_seal(&phase)
contacts = try_prepare_network_contact_island_batch_with_backend(
    sealed, &phase, SimulationContactIslandMapSource::Present(&map),
    &mut HostContactNumericBackend)
```

No queue, placeholder Contact, dummy body, raw-word constructor or native
pointer image is needed. The zero-length manager frame still validates manager,
body-pool, frame, phase and retained static-body identity. The producer forms
ten empty island subsequences from genuine empty backing.

`try_begin_contact_frame` clears the retained vector and refuses a repeated
start for the same host frame. Sealing checks unchanged source identities.
Build returns its real typed failure rather than a test helper's panic.

This proves ONLY a contactless constraint path. Until collision jobs query the
actual authored board/world, empty Contacts are not evidence that the geometry
has no intersections. A world-contact test must use the real producer and seal
its accepted dense prefix. Never call the deliberately contactless setup a
complete freeroam frame.

## Existing Joint producer

File `simulation_joint_preparation.rs`:

```text
try_prepare_active_joint_phase_with_island_map_and_backend(
    &exact_joint_pool, &phase,
    SimulationJointIslandMapSource::Present(&map), &mut numeric)
 -> Result<SimulationPreparedJointIslandBatch<'joint>,
           SimulationJointPreparationError<NumericError>>
```

It is presently private, as are the map-source enum/error and ordinary host
`SimulationJointHostNumericBackend`. The only outward convenience adapter is
`#[cfg(test)] prepare_joint_horus_island_batch_for_test`, which is not the
production route to invoke or copy. Promote a narrow checked production API
around the existing composer; do not rebuild its stage graph in a board module.

Source coverage: `JointBatchBuild 82ae7a78..82ae7b0c`, table Joint block
`82ae3eec..82ae3fec`, existing full `JointJacobian::Build` stages and final record.

The producer visits actual Active Joint occurrences in right-link order,
reads body B state then A, admits only `((stateB|stateA)&4)!=0`, selects island
using B then A signed labels (B unless EXACTLY -1, else A), snapshots A then B,
reads the actual retained frames/limits/spy, and compiles dense records. Source
binding and destination capacity checks precede numeric callbacks.

The resulting batch borrows the immutable exact Joint pool, retains live body
observations plus pool/frame/phase identity, and owns stable per-island dense
subsequences. It is not merely a value snapshot: Horus checks these bindings
against the phase before permitting RF mutation.

`build_simulation_joint_inert_record` is the numerical composer actually called.
Its body adapter keeps inverse-mass/state fields and uses the independently
proved dead-word boundary for seven adjacent body metadata W lanes. Its output
record is minted with checked bindings, not caller-supplied words.

## Existing Drive producer

File `simulation_drive_jacobian.rs`:

```text
try_prepare_active_drive_island_batch_with_backend(
    &drive_pool, &phase,
    SimulationDriveIslandMapSource::Present(&map), &mut numeric)
 -> Result<SimulationPreparedDriveIslandBatch,
           SimulationDriveIslandBatchPreparationError<NumericError>>
```

Also presently private, including numeric/map-source/error types. Its outward
convenience adapter `prepare_drive_horus_island_batch_for_test` is cfg(test).
`SimulationDriveHostNumericBackend` already supplies explicit ordinary-finite
host reductions and lazy estimates. Expose the existing fallible path instead
of duplicating its geometry/prediction/bias/effective-mass/finalization leaves.

It calls `SimulationDrivePool::try_active_build_snapshots_for_phase`, which
validates pool identity and ring integrity, walks actual Active drives, reads
B then A state, gates on Active bit, snapshots A then B body values, then reads
the actual retained frames and dynamics through their locks. Poison errors are
returned. Frame/dynamics Build-time sampling is intentional; it is not a
license to manufacture independent snapshots from an unrelated public dump.

The composer forms dense records and stable ten-island subsequences, with
B-then-A label selection identical to source. It seals Drive pool, body pool,
frame and exact phase; the solver revalidates Drive occurrences/endpoints and
record binding words against the still-live pool before effects.

## Checked inert tokens are NOT native addresses

Both existing Joint and Drive composers derive body RF and joint/drive record
binding tokens using checked `slot+1` from their ACTUAL live occurrences. Zero
remains absent. The sidecars separately retain authority, frame, phase and
non-reused occurrence; a numeric token alone is no mutation capability.

These are legitimate existing host-native bindings, not fabricated Xenon
addresses. Revalidation reconstructs the expected token from the live sidecar
and rejects mismatch/replay. No arena-inertia source or
`SimulationArenaXenonAddressBinding` is required by these constraints.

This is not proof of native pointer bit patterns, pointer-derived packed W
arbitration, exceptional VMX arithmetic or all intermediate source word images.
Keep the current Joint dead-metadata proof and Drive ordinary-finite boundaries
visible. Do not use this absence of an address requirement as authority to mint
the different packed address capabilities required by existing ground/serialized
collision code.

## Existing solver really mutates RF and integrates

`simulation_horus_pipeline.rs` already exposes:

```text
try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite(
    &mut contacts, &mut joints, &mut drives,
    island_count, max_iterations, &drive_pool, phase)
```

Source outer schedule `HorusPipelineIslandUsingIndirectJacobians`,
`82ae2858..82ae3190`; native all-island dependency barrier from
`PopulateJobTree`, `82e0b410..82e0b67c`.

For Active bodies and positive iterations, it preflights all three families
for every scheduled island, then runs each pass Contact -> Joint -> Drive.
It reads then-live RF rows on each visit, updates the records' accumulated
impulses, and performs the actual checked A0/B0/A2/B2 RF stores. After all
scheduled islands, it consumes the one dynamic-update phase: integrates actual
body slots, clears RF, restores force from configured gravity and clears torque.
This is materially different from inspecting a final Jacobian or snapshot.

Serial ascending-island order is explicit PC policy, not a claim of original
job arrival ordering. Source max iterations is sampled once per native island;
the existing serial host function uses one stable caller snapshot for the frame.
Empty Active bypass and zero-iteration integration-only branch are preserved.

Preflight failures prevent record/RF/body solver effects. This does not roll
back earlier StartFrame configuration, labels or Contact-frame start. After
execution begins, panic retains the exact mutation prefix and does not become
a fabricated native transaction rollback.

## Required per-frame retained Drive normalization

Live IDA/PPC follow-up identifies the prerequisite precisely:
`Sk8::Physics::DriveNormalizer::operator()(rw::physics::Drive const&)`
at `0x827b2738..0x827b27d0`. Raw function-byte SHA-256 is
`83da34eec111e2897045fbf1b08bd63d112517bdc579db1fdf77504e4e4f14ee`.
This is a mutation of the retained DriveFrames owner, not normalization of a
temporary Build snapshot or a DriveDynamics spring/damping conversion.

`Island::Step_Solver2` calls DrivesEnd (`0x82e0bd70`) at `0x827b35f8`,
DrivesBegin (`0x82e0bd58`) at `0x827b3604`, then the specialized `eastl::for_each`
(`0x827b30c8..0x827b3124`) at `0x827b3610`. This runs after single-player/online
island setup, before optional thaw (`0x827b3690`), partition (`0x827b3698`), and
CreateSimulationIslandTables (`0x827b36a8`). Begin loads Simulation+0x38's
Active Drive sentinel and its right link +8; End returns that sentinel.
The loop visits every Active Drive in right-link order, without the later
Build's either-body-active admission filter, and reads the next link after
normalizing the current Drive.

The normalizer reads Drive+0's frame pointer and returns only if it is null.
It loads quaternion A at Frames+0 and B at Frames+0x20, computes separate
four-lane squared lengths, issues reciprocal-square-root estimates in A-then-B
order, applies two fused refinement rounds to each, multiplies each original
four-lane quaternion by its refined scale, and stores A then B back to the
same owner (`0x827b27c4/c8`). Positions and dynamics are untouched. No zero,
nonfinite, already-unit, or identity shortcut exists. Numerical porting still
requires the explicit VMX dot-product reduction/refinement operation graph and
estimate boundary; this audit does not substitute a generic host normalize.
If multiple Drive slots alias one frame owner, each list visit normalizes that
same owner again: deduplicating owners or using an earlier copied snapshot is
not source-equivalent.

The actual reusable Rust storage is
`crates/skate2-core/src/physics/solver/drives/simulation_drive_pool.rs`:
`SimulationDriveFrames::{quaternion_a, position_a, quaternion_b, position_b}`
and the private slot's `Option<Arc<RwLock<SimulationDriveFrames>>>`. Add retains
the supplied owner. `try_frames` is deliberately read-only; the original owner
remains the live mutation capability. The pool currently has no active-ring
normalization producer. `simulation_drive_jacobian.rs` has angular-axis
normalization during Build, but that is a different operation and does not
perform this retained-frame mutation. The new producer should therefore run
through the real Active pool/retained owners before Build samples them, with
explicit host lock-poison/error semantics, rather than add a public arbitrary
frame rewrite or reuse the filtered Build snapshot traversal.

Consequently, the recommended contactless path below is a restricted
constraint-build/solve/integrate test, **not the original whole Tick**. Before
promoting it to a source-chronological persistent solver step, this retained
normalization prerequisite must be implemented and independently audited, or
the diagnostic must explicitly disclose its omission. Fresh constructor
quaternions and a successful solver test cannot justify silently skipping it.

## Configuration and remaining real producers

Read `physics-world-configuration.md`, not generic library defaults:

- Gameplay generated `physics.default` layout supplies gravity [0,-9.8,0,0].
  Its class is physics, NOT physics_world; use accepted runtime layout fetch.
- World timestep bits 3c888889; freezing threshold 3727c5ac; cooldown2.
- Constructor iterations25 is overwritten by prephysics each frame.
  Single-player cvar default is50 (range25..200); use current source value.
- Offline island capacity/count route is10. Controlled tests using fewer
  iterations/islands must say so rather than claim gameplay settings.

Hard remaining full-frame producers, not new numerical leaves:

1. Manager/backend ownership wiring and narrow production visibility above.
2. Original pre-Build Drive normalization, plus source frame configuration and
   update schedule. No normalization implementation was found by this audit;
   immutable fresh frame values do not prove that the per-frame call is absent.
3. Mandatory HookPool body/drive creation and Hook label exclusion.
4. Eligible assembly-pair append producer and recycled-label partition. Contact
   count zero does not imply no partition pairs; self-culling still appends self.
5. Actual authored 19-volume collision/serialization source bridge into existing
   ground/world contacts; arena-backed capability construction must not be faked.
6. Later authored body spawn/drive/control updates, skater body integration and
   input-dependent physics. A contactless board constraint step is not skating.

## Recommended next validation slice

Create actual manager domain first, attach the mounted-input authored board
and source truck drives, execute a deliberately contactless/non-Hook restricted
frame using StartFrame-produced map, existing Build composers and shared Horus,
then observe actual body/RF changes. Keep the same owners and body occurrences
across successive frames and teardown through the real session.

Check counts (manager Static1, board Active7, Joint6, Drive2), source endpoint
order, six/two compiled dense records, same-frame/phase/occurrence seals,
Contact-empty branch, real RF impulse effects and once-only integration/RF
clearing. Inject unavailable capacity/stale authority before effects and ensure
there is no hidden raw-word or snapshot-created capability. Compare constrained
versus integration-only results as an effect discriminator, not a native trace.
Actual per-tick console trace remains the eventual parity gate.

## Implemented producer-to-solver checkpoint

2026-09-07: simulation_assembly_constraint_session.rs now owns the real backend
and manager Contact/label domain. Manager creation remains before any gameplay
Add on the pristine pool. Only the existing opaque assembly backend is exposed
for admission/configuration; no mutable pool or manager replacement is exported.

Each explicitly contactless/pair-free/non-Hook step advances frame configuration,
produces StartFrame labels, normalizes actual retained Active Drive frames,
captures the current-occurrence island map, and borrows ONE dynamic-update
phase. The new narrow production facades call the existing checked Contact,
Joint and Drive composers with Present(map) and unchanged host numeric backends.
Contact backing is actually begun/sealed empty, not replaced by fabricated rows.
The existing ten-island serial Horus function solves C/J/D and consumes that
same phase for integration. Errors preserve earlier StartFrame/normalization/
Contact-start effects and do not fabricate rollback or an implicit frame retry.

Actual raw-Vault board corpus now creates two independent managed worlds from
shared immutable original definitions, with distinct mutable truck-drive owners.
Both retain managerStatic1 + boardActive7 + Joint6 + Drive2. Gravity is read via
generated cached physics layout (origin and exact words asserted), not generic
node FETCH or runtime frozen tuning. Timestep3c888889, minimum energy3727c5ac
and cooldown2 follow the source configuration. Fifty iterations is the inspected
single-player cvar default, not a newly implemented live cvar service. The other
world uses zero iterations solely as an integration-only effect discriminator.

Four frames in each world compile C/J/D counts0/6/2, retain the same body
occurrences, produce finite poses/velocities, clear actual live RF rows, restore
source gravity forces and clear torques. The positive-iteration world's actual
quaternion/COM bits differ from the integration-only control, proving constraint
effects rather than merely differing host authority IDs. At frame4 the observed
deck COM is[-0.0013456892,-0.027246058,0.00017813768], quaternion
[0.002885051,0.014536118,0.024791507,0.99958277]; zero-iteration COM is
[0,-0.027222227,0] with identity quaternion. These are host observations, NOT
native output oracles, a long-term stability test or a skating-feel assertion.
Explicit drive/board cleanup leaves only the manager Static occurrence.

Three new managed-owner tests cover unavailable manager Static/contact-byte
overflow, repeat empty frames preserving the same Static occurrence, and later
normalization poison preserving first-drive writes while body slots and phase
allocation identity remain unchanged (no integration). Twelve normalization
tests and all shared prior consumers remain covered by the full gate.
Independent source/code/test/corpus review passes. Final root71989:
2097core+187formats+38app+3symbols=2325unit tests,6doctests, fmt/Clippy-Dwarnings
and all4core original-data tests pass. App ignored corpus not rerun.

At that checkpoint still required: mandatory HookPool body/drive and exclusion, original spawn,
actual authored geometry into collision/pair partition, full source scheduler,
mutable updates/controls, skater/input, camera and renderer. The empty Contact
route intentionally does not query the world; absence of Contacts here is not
evidence of no collisions or no eligible assembly pairs.

## Later HookPool-connected checkpoint

The managed owner now retains a manager/body-pool-bound HookPool. Its real
membership produces exclusion labels after StartFrame and before normalization
and map capture. The source hook prefix creates a separate Static body and
deck6/bodyB-to-hook0/bodyA drive. An additional mounted four-frame run uses
nine bodies and actual C/J/D counts0/6/3 through the unchanged shared solver.
It preserves occurrences, clears RF, retains hook state and checks real labels.

Fresh hook HARD2/zero-strength settings are intentionally disabled. The corpus
reads the actual third record AFTER Horus/integration: all six limits and both
accumulated impulses are zero. All seven board bodies' COM/quaternion/velocity/
omega bits equal the truck-only control. A separate nonzero-RF/bias test checks
zero-cap behavior; no fabricated records feed the production run. The earlier
expectation that constructor hook insertion must move the board was disproved
by source disable helpers and corrected without changing production arithmetic.

Ordered drive/hook/board teardown and owner lifetimes pass. Full proof and
counter/publication scope are in `skateboard-hook-source.md`. Independent review
and strict gate61189 PASS:2356unit+6doctests, fmt, warnings-denied Clippy and
all4core mounted tests. Original spawn, board SkaterPool, authored collision/
pairs, full Tick and live skating remain absent. Host arithmetic is not a native
per-tick trace. The three four-frame runs remain contactless and pair-free.

The subsequent spawn checkpoint adds two controlled in-place board/hook
relocations AFTER actual retained frames, each followed by another real
C/J/D0/6/3 solve. Live getters, original source write order and retained motion/
occurrences/constraints are checked; no detachment/re-addition occurs. Full
scope is in `skateboard-spawn-transform-source.md`. Gate77951 passes2379unit,
6doctests and all4core corpus tests. Actual player spawn-location/cvar producers,
SkaterPool publication and board-ground collision remain open.

## Later authored-board ground checkpoint (2384)

The new `skateboard_ground_frame.rs` child retains this same managed backend,
StartFrame/hook exclusion/normalization and exclusive phase. Source-owned board
Create/Update/bounds now feed the original resident-ground CPU job; its real
records enter checked Contact append/Build before existing Joint/Drive Build
and shared Horus/integration. The contactless control route remains unchanged.
Four original-data board/Rob's Park frames produce C/J/D6/6/3,6/6/3,6/6/3,4/6/3.
Actual nine-body occurrences, constraints, finite states, RF clearing and static
hook state pass checks. No contact-effect isolation or long-run/native trace
claim follows from these four frames. Placement/capacity/settings remain explicit
component inputs. Full source/evidence/ownership limits are in
`authored-skater-ground-publication-path.md`; independent review passes.
Gate90328:2384unit+6doctests, fmt, warnings-denied Clippy and all4core mounted
tests. Actual SkaterPool membership, counterpart/pairs, live controls and Tick
remain open. No alternative Contact/Joint/Drive numerical implementation added.

The2392 checkpoint replaces managed direct-board attachment with actual
manager-bound SkaterPool ownership, shared board/hook indices0/1, nonparallel
whole-pool refresh and full dense publication iteration. The same18 board
component frames (14contactless plus4ground) retain their earlier numerical
results, with source handle-based spawn/drive/cleanup. No C/J/D math changed.
Eight ordinary ownership/configuration tests and three mounted multi-member
scenarios pass; strict gate79399 passes2392unit+6doctests and all4core corpus
tests. See `skater-pool-owner-source.md`. Actual humanoid and full Island/Tick
remain absent.
