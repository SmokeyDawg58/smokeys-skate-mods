# Actual DMO and vehicle line-publication integration

2026-09-08, bounded read-only owner/caller audit. No Rust or Cargo
changes. Native BeginUpdate/AddVolume/Remove/Reset semantics reuse the
independently reviewed [owner packet](skeleton-ik-world-line-owners-source.md);
only missing constructor-capacity inputs were additionally inspected in IDA.

The immediate prerequisite is actual generic serialized-assembly admission
into the selected island's DynamicPool. The current original DMO loader and
body-only session preserve valuable source ownership, but there is no live DMO
membership array to which a complete BeginUpdate publisher can attach yet.
Adding a mesh list to ConstraintSession without that admission would create a
second scene authority.

## Existing chain and its precise stopping points

Links below refer to the inspected files; line numbers identify relevant
definitions and may move during concurrent root integration.

| Responsibility | Existing authority | What it does / missing connection |
|---|---|---|
| Original main-bank asset bytes | [DmoBankProcessedArenaAsset](../../crates/skate2-core/src/streaming/dmo_bank_processed_asset.rs#L97) | Retains exact normal arena plan and dictionary-filtered SerialisedAssembly templates; builder sessions validate the main-bank catalog/fingerprint. Loading an asset is not spawning a body. |
| Resident route/lifetime | [DmoBankResidentModelCatalog](../../crates/skate2-core/src/streaming/dmo_bank_resident_model_catalog.rs#L385) | Resolves a current registry winner to its exact resident model owner; no generic pool insertion. |
| Locator/model association | [SimulationDmoRegistrationManager](../../crates/skate2-core/src/physics/solver/simulation_dmo_registration_manager.rs#L170) and [ResolvedLocatorPartOwner](../../crates/skate2-core/src/physics/assembly/simulation_dmo_assembly_part_session.rs#L371) | Uses registered locator and sealed model key, retains the exact locator/model association. Native query/SpawnStatic timing remains explicitly outside it. |
| Source-bound Part lifetime | [SimulationDmoAssemblyPartOwner/Session](../../crates/skate2-core/src/physics/assembly/simulation_dmo_assembly_part_session.rs#L175) | Retains asset, template ordinal, Assembly ObjectId, attachment batch and backend authority. Constructs detached Parts from explicit transforms and attaches/detaches them. No world index, pool membership, complete Create publication, mesh list or live spawn caller. |
| Definition volumes | [SerialisedAssemblyTemplate](../../crates/skate2-core/src/physics/assembly/serialised_assembly_template.rs#L260) | Owns Arc normal plan and definition-ordered Parts. Each Part has volume ObjectId/root index, source inertia, inverse-body fields and material. This is the correct root graph; flattened Create volume records are a different traversal. |
| Collision graph/bytes | [CollisionArena](../../crates/skate2-core/src/world/world.rs#L35) | Ordered aggregate roots, SMA children and meshes; retained via normal plan → static plan → SimulationCollisionAsset.collision. Each mesh retains original object bytes. |
| Current source-aware body backend | [SerialisedAssemblySimulationBackend](../../crates/skate2-core/src/physics/assembly/simulation_assembly_backend.rs#L427) | Owns its OWN rigid-body pool, image domain and arena-inertia routes. Its body-only binder explicitly refuses definition Joints/Drives. It is not the constraint world's shared backend. |
| Shared live physics backend | [SimulationAssemblyBackend](../../crates/skate2-core/src/physics/assembly/simulation_assembly_backend.rs#L764) | Owns the manager's actual body/Joint/Drive pools. Its current inertia enum has immutable/mutable semantic payloads, not the source backend's routed arena binding. |
| Actual Part transform getter | [dynamic_assembly_part_transform](../../crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs#L128) | Owner-checked live body getter then inverse-body adjustment; detached branch reads actual cache. Specialized to DynamicAssemblySession with the shared backend, not the staged DMO body-only session. |
| Existing real AssemblyPool admission | [SimulationSkaterAssemblyPool](../../crates/skate2-core/src/physics/assembly/runtime/dynamic_assembly_skater_pool.rs#L366) | Owns actual board/skeleton candidate+publication pairs in admission order, manager/body authorities and active list. This supplies the existing Insert/Remove ordering pattern, not a place to disguise a DMO as a skater. |
| Serialized Create/refresh arithmetic | [PreparedSerialisedAssembly](../../crates/skate2-core/src/physics/assembly/serialised_assembly_create.rs) and [pool_update](../../crates/skate2-core/src/physics/assembly/serialised_assembly_pool_update.rs#L34) | Existing owner-qualified Create/Update/bounds machinery awaits an attached DMO membership bridge. The refresh function receives borrowed inputs in caller order; it owns no member list. |
| Selected simulation world | [SimulationAssemblyConstraintSession](../../crates/skate2-core/src/physics/assembly/simulation_assembly_constraint_session.rs#L78) | Retains actual backend, contacts, hooks, optional skaters and physical-player registries. No DMO/vehicle DynamicPool or line collection is present. Ground remains its separately borrowed actual StaticGroundPool. |

Call-site inspection found the staged DMO Part session attached by its own and
resident-catalog tests. No production caller currently advances that session
through a living-world spawn into generic membership. The original asset
builder/registration path does not change that conclusion. No live vehicle
Part-session/admission owner was found in the inspected core sources.

`world::DynamicCollisionBody` and `OwnedDynamicCollisionBody` are value-shaped
inputs to existing overlap/cull helpers (model ID, transform, expanded Parts
and triangles). They do not retain actual manager body observations, source
session lifetime or DynamicPool publication. Their use of the word "runtime"
does not make them the missing pool authority.

## Reuse the graph, not the frozen leaf list

[DynamicClusteredMeshTemplate](../../crates/skate2-core/src/streaming/dynamic_clustered_mesh_template.rs#L64)
retains Arc CollisionArena, Part/root mapping and ordered leaf occurrences.
Its documentation explicitly says enable policy is sampled at construction.
Disabled parents/children can disappear from that list. Therefore live
AddVolume must walk the actual Part volume and retained original SMA graph
again, reading the disabled-node cvar at each visited disabled volume. It
cannot replay only `leaves()` or rebuild a filtered template ahead of the
source getter/cvar sequence.

The original DMO owner already retains the normal-plan chain containing this
graph; keep that owner plus indexes/paths in publications. Do not clone the
whole CollisionArena merely to obtain the template's Arc shape. A checked
borrow can resolve the Part's root through its actual template and plan.
An occurrence needs a stable path through root/SMA child ordinals, because
repeated references to one mesh or identical Volume values remain distinct.

Current source storage is immutable. Before supporting native live volume
flag mutations, identify their actual per-Part/per-volume owner and writers.
Do not invent a second mutable flag map in the query publisher or claim that
immutable parsed flags cover unimplemented mutations. The publisher can read
the presently authoritative retained flags while keeping that scope explicit.

## Required same-manager admission seam

The smallest coherent implementation can specialize initially to proven
body-only original DMO definitions, preserving refusal of nonzero definition
Joint/Drive counts. It still needs all of these connected responsibilities:

1. Bind the original selected model/locator to the SAME constraint-world body
   pool. Reuse source-inertia bindings and checked Add operations, with a
   borrowed source-aware backend facade or retained source variant in the
   shared backend. Do not create a second body pool and publish its transforms
   as if those bodies belong to the IK query's island. Exact API unification
   belongs to the root; current backend types are not interchangeable.
2. Carry actual initial Part transforms and requested body state from the
   recovered spawn/placement producer. The current explicit inputs can prove
   a controlled corpus path, not complete SpawnStatic/SetPosition scheduling.
3. Create a generic serialized member using the existing manager's world-ID
   allocator and source Insert ordering: ID before Add; preserve failed
   candidates/attachment prefixes; bind the actual Create publication and
   session together. Never use a model catalog ordinal as an assembly ID.
4. Keep one dense membership owner per actual DynamicPool, with a private
   checked occurrence handle, manager/body-pool identity, original source
   owner, mutable session, serialized publication and bound/active state.
   Reuse existing checked Create/refresh kernels rather than creating a
   second serialized snapshot authority.
5. Add lazy same-session Part access for this admitted type. BeginUpdate must
   read the Part's volume reference BEFORE its actual GetTransform, even for
   a primitive/unsupported aggregate that emits no mesh. Cached transforms
   or locator matrices cannot replace the attached-body getter.

The current Create helper deliberately validates before publishing a host
owner; its documentation disclaims native invalid-input partial writes.
Retain that explicit host boundary when reusing it. Likewise source-address
image/packed-W dependencies of generic serialized publication must remain
owner-qualified, not zero-filled to make admission succeed. A more complete
jointed vehicle path needs its own actual definition/binding/admission proof.

## DynamicPool publication design and observable prefixes

Retain the published mesh collection INSIDE that DynamicPool membership
owner. A record contains owner occurrence handle, retained arena owner and
volume occurrence/mesh index, forward matrix, cofactor inverse and root bounds.
It references original bytes through the owner; it does not own copied mesh
bytes, independent triangles or another spatial index. Retain per-record
construction progress/typed absent fields when Reset can fail after count
admission. A query against an incomplete required record returns a capability
error; it must not silently skip the slot or fabricate bounds.

BeginUpdate must preserve the reviewed sequence:

1. Synchronize line, box, trajectory and step-selector batches in that order.
   With synchronous host jobs, the real retained batch state must establish
   completion. Four no-op callbacks over absent batch owners are not proof.
2. Clear mesh count, capture current member count, then visit members and all
   Parts ascending. Load volume, then actual Part transform, then AddVolume.
3. Disabled volume alone samples IgnoreVolumeEnableFlag. Rejected disabled
   or unsupported primitive/aggregate returns success. SMA recurses ascending
   and forwards the SAME Part transform and owner, without composing parent
   or leaf Volume.transform.
4. Mesh admission checks capacity, increments count, then Reset stores owner,
   volume, mesh and forward columns; inverse math and bounds follow. Keep
   earlier visible entries/count/Reset fields on a later refusal.
5. Filling the last slot succeeds in publication but returns false from
   AddVolume, terminating traversal and SKIPPING base BeginUpdate. Only full
   traversal reaches existing serialized refresh. An empty real pool also
   reaches that tail.

Remove synchronizes the same four batches, executes base AssemblyPool removal
first, then removes every matching published mesh owner with swap-last and
rechecks the same slot. This order changes later nearest-hit ties. Keep owner
identity alive through the cleanup prefix; a checked query cannot resolve a
removed/reused member by a stale numerical index. Publication generation or
the existing occurrence-handle pattern should qualify returned candidates.

The retained collection spatial pointer is constructor-null and has no
identified later producer. Use its actual ordered-array path. Do not add an
octree as an optimization or infer a native dynamic octree from a generic
optional traversal implementation.

## Capacities and construction inputs newly checked in raw IDA

DynamicPool constructor copies params+10 into owner+1768 at 827cafec/aff0;
this is the mesh capacity checked by AddVolume. It allocates 0xc0 bytes per
mesh at cb04c..6c. Capacity is not the unrelated 64-result overlap scratch.

`Island::InitWorld` constructs distinct actual pools at 827b46bc (DMO) and
827b486c (vehicle), stored through island+6cc8/+6ccc. DMO params+0 is 50 at
45dc and params+10 is 50 at 4618. Vehicle params+0/+10 are 24 when island
flags+8 mask1 is set (4798..47a4,47d0), otherwise both zero (47bc..c8,47d0).
A constructor-proven zero-capacity vehicle pool is a legitimate empty owner;
an absent pool is not equivalent to it.

The constructor samples distinct live `kDmoPoolUpdateInParallel` at 45d4 and
`kVehiclePoolUpdateInParallels` at 4790. Line/box/test counts are loaded from
their actual cvars at 4640..4654 and again 47f0..4804. Trajectory/step counts
also depend on island flags. Preserve runtime values and selected unsupported
parallel behavior rather than hardcoding a nonparallel replacement. The full
InitWorld lifecycle is outside this bounded follow-up.

Additional raw windows (half-open) for reproducibility:

| Window | SHA256 |
|---|---|
| 827b45d0..827b46d0 | 5f041f628934354ae49aaef010c5b137a02e6012abc940238b269a0dbc373a2a |
| 827b47b0..827b4880 | 541825f716d60ab79a9009a501a1b1cf38adf206072af972ba2a4aa11ad71a86 |

## Coherent validation and remaining gates

First integrate one actually admitted original body-only DMO using its real
model/locator source, routed inertia and same-world backend. Verify its
published record's forward matrix against the actual Part getter, source
volume/mesh owner against the retained plan, and inverse/bounds against
independent arithmetic. Move the SAME body, run the next BeginUpdate, and
prove the query sees its republished transform. Two admitted occurrences of
one original model must remain distinct; removal must prove swap-last tie
order and stale-handle refusal. This is a useful original corpus path even
while full living-world spawn scheduling is explicitly incomplete.

Synthetic owner tests should isolate synchronization failure before count
clear, volume-before-transform reads, disabled-parent/child lazy cvar reads,
count-before-Reset refusal, full-capacity early return before base refresh,
and Remove's repeated matching-owner swap-last loop. These tests complement
the actual-owner story; neither can substitute for it.

The world's default IK request requires Ground, DMO and conditionally vehicle
routes through real selected owners. This packet establishes an actionable
DMO admission/publication dependency, not a way to declare default-query
completion from geometry alone. Until the actual pool exists and required
publication completes, preserve an explicit unavailable-owner failure.

Confidence: high for inspected Rust ownership/caller gaps and reviewed native
publication order; exact for newly inspected constructor stores/cvar sites.
Proposed owner factoring is an implementation design, not source proof.
No new original corpus, Cargo result or deterministic native trace is claimed.
