# Source-qualified DMO Parts in the shared simulation backend

2026-09-08, bounded read-only audit. No Rust, Cargo or shared-document
changes. This is an implementation design for the admission prerequisite in
[world-dynamic-line-runtime-integration-plan.md](world-dynamic-line-runtime-integration-plan.md).
It reuses the accepted source and authority proof in
[simulation-xenon-address-image.md](simulation-xenon-address-image.md).
The pool admission/publication author owns the separate native Insert/Remove
and DynamicPool lifecycle packet.

The recommended extension is a source-routed inertia variant and retained
route services INSIDE the existing `SimulationAssemblyBackend`. It must use
that backend's actual rigid-body pool. A temporary backend facade alone is not
a drop-in replacement for the current DMO session's statically fixed backend
type. No second body pool or re-created semantic inertia is needed.

## Current owners and reusable interfaces

| Existing type | Proven responsibility | Bridge implication |
|---|---|---|
| `SerialisedAssemblyPartInertiaSource` | Arc exact normal-arena plan, resource index, resource-relative 0x30 span | Preserve pointer-equivalent identity; equal bytes in another Arc do not alias. |
| `SerialisedAssemblyRuntimeInertia` | Inseparable semantic `Arc<SimulationInertia>` and exact source from one Definition Part | Reuse this pair; do not independently rebuild/cross its two halves. |
| `SimulationArenaXenonAddressBinding` | Exact arena owner, five explicitly supplied resource bases, private generation | Needed for source address selection; currently staged, not a proved allocator lease. |
| `SimulationRigidBodyAddInertiaOwner` | One arena binding and actual pool authority; preallocated occurrence-qualified Add outcomes | Reuse on the shared pool, including StaticNull and removal retirement. |
| `SimulationPackedBodyWordImageDomain` | One actual body-pool authority/capacity and immutable Simulation image generation | Retain once for the shared manager, never one competing domain per DMO. |
| `SerialisedAssemblySimulationBackend<M>` | Own body pool, image domain, immutable routes, routed inertia and body handles | Its routing logic is reusable; its separate pool cannot be inserted into the shared island. |
| `SimulationAssemblyInertia` | Private immutable/mutable storage; one public shared inertia type | Add a private source-routed arm, keeping current public type and existing payload aliases. |
| `SimulationAssemblyBackend<M,JF,JL>` | Actual shared body, Joint and Drive pools, one body-pool authority | Own the source route registry and source removal dispatch here. |
| `DynamicAssemblyPartAttachmentTransaction<B>` | Definition-order metadata/payloads, mutable Part cache and private live handles | Both full sessions and body-only sessions already use this transaction. Extend its read/copyback access, not a replacement Part list. |
| `SimulationDmoAssemblyPartOwner/Session` | Sealed resident asset/template selection plus that exact Part transaction | Retain this owner chain while changing its backend binding to the shared backend. |

Relevant source files are
[simulation_assembly_backend.rs](../../crates/skate2-core/src/physics/assembly/simulation_assembly_backend.rs),
[simulation_assembly_payload.rs](../../crates/skate2-core/src/physics/assembly/simulation_assembly_payload.rs),
[simulation_xenon_address_image.rs](../../crates/skate2-core/src/physics/solver/bodies/simulation_xenon_address_image.rs),
[serialised_assembly_template.rs](../../crates/skate2-core/src/physics/assembly/serialised_assembly_template.rs),
[simulation_dmo_assembly_part_session.rs](../../crates/skate2-core/src/physics/assembly/simulation_dmo_assembly_part_session.rs),
and [dynamic_assembly_session.rs](../../crates/skate2-core/src/physics/assembly/runtime/dynamic_assembly_session.rs).

The selected playable world already uses
`SimulationAssemblyConstraintSession<AssemblyPartMaterial,SkateboardHookDefinition>`
and therefore the matching shared backend material type. Definition Material
does not need conversion into a fabricated generic payload. Its existing
three-word immutable snapshot remains the established scope; native future
mutation/alias visibility is not added by this bridge.

The shared backend's actual body-pool identity also owns the contact manager,
Drive pool and physical-player bodies. Source DMO bodies added to this pool
participate in its actual capacity, intrusive rings and later solver frame.
They do not become skater members; the separate generic DynamicPool member
owner retains their admission and publication.

## Source Inertia identity and exact Add/removal chronology

The accepted Definition locator graph is:

```text
part_array = wrapping_u32(definition_object_offset + serialized_parts_displacement)
inertia_offset = checked(part_array + part_ordinal*0xa0 + 0x10)
identity = (exact arena Arc, definition resource index, inertia_offset)
backing span = [inertia_offset, inertia_offset+0x30)
```

`Part::Initialize` takes this embedded address at `82ae9838`, stores the
retained Part pointer at `82ae8fec`, and Add reloads it at `82ae0ad8` before
calling `Simulation::AddRigidBody` at `82ae0bd8`. Part's effective-state rule
is exactly requested state4 plus internal flag bit0 => state1; all other raw
requested states pass through. Existing attachment metadata and
`effective_part_body_state` already implement this graph.

`SimulationRigidBodyAddInertiaOwner::try_add` already provides the correct
checked operation:

1. Validate actual pool authority, then request the pool's drop-inert Add
   reservation. Empty Free returns None before Definition address binding.
2. Validate the preallocated outcome cell for that reserved slot.
3. Static chooses StaticNull without validating the Definition address.
   Nonstatic Definition resolves through the exact arena binding and retains
   the same `SerialisedAssemblyRuntimeInertia`; generic null stays distinct.
4. Commit the actual body with the paired semantic Arc, then move the outcome
   into the preallocated cell using that complete new body observation.

Native Active stores input inertia at `82ae6ef8`, Frozen at `82ae72bc`, and
Static forces zero at `82ae7604/82ae764c`. Current state or a present semantic
payload cannot reconstruct the historical Add-selected pointer. Freeze,
Activate, Reset and Remove do not establish a new Add outcome.

The helper's removal validates both live occurrence and exact outcome,
removes the actual body, THEN retires its authority record. This is host
authority retirement, not a claim native Remove clears the stale pointer.
Shared removal must route source bodies through it even when the recorded
outcome is StaticNull. Direct shared `try_remove_rigid_body` would leave the
source cell stale and corrupt the next same-slot admission.

Existing source binding rejects unknown exact arena owners but deliberately
does not validate a Definition address during Part-template binding. The
Static skip and empty-Free-before-address-read remain at body Add. Preserve
the current diagnostic across rollback: a later failed Part must not lose its
original source Add error while earlier Parts are copied back and removed.

## Minimal shared-backend extension

The following names describe proposed interfaces, not already implemented
APIs or native fields:

- Extend private `InertiaStorage` with `SourceRouted(SerialisedAssemblyRoutedInertia)`.
  Only checked exact-owner template binding can mint this arm. Cloning retains
  the original semantic/source pair and private route token.
- Extract the existing route registry into a backend-retained source service
  containing the single image domain, existing per-arena outcome owners and
  immutable/private route tokens. Each token remains qualified by actual pool
  authority and arena binding epoch. Registration validates duplicate exact
  owners/epochs and image capacity before publishing the new route.
- Keep shared `SimulationRigidBodyHandle` unchanged. Add a preallocated
  occurrence-to-route directory in the source service. An entry includes the
  complete `SimulationRigidBodyObservation`, not just slot index. This is
  derived host routing metadata, not another body owner or native field.
  Source Add publishes this entry after successful Add outcome commit; removal
  dispatches through that route, then clears it. Ordinary shared bodies remain
  unrouted and retain their existing add/remove behavior.
- Dispatch Add at `SimulationAssemblyBackend`, where both actual pool and
  source service can be mutably borrowed. `SimulationAssemblyInertia::try_add`
  cannot complete the new arm with only its present `&mut body_pool` argument.
  Either narrow that old helper to ordinary storage or explicitly pass the
  backend's source service; do not hide mutable route state in inertia Arcs.
- Add a checked source-error variant to the backend Add diagnostic, preserving
  the generic trait's existing `Option<Handle>` null/failure shape and actual
  Part rollback behavior. Directory allocations occur before body mutation.

Registration must not replace or unregister a route/image generation while
live bodies or `PreparedSerialisedAssembly` retain it. Existing private epochs
and exact-owner checks are reusable. Additive late route registration can be
supported by stable token identity; it must not reinterpret a moving vector
ordinal as a permanent token. An initial fixed registry is smaller if the
actual resident admission lifecycle permits it. The bridge itself must not
guess which arenas will be resident.

`try_read()` of the new payload can use the already paired semantic Arc.
`as_immutable()` must retain its documented ordinary-storage meaning rather
than erase the route. `try_read_packed()` must not invent semantic meaning for
the source tensor's fourth raw serialized word: that word is preserved in the
exact source span, while the existing general semantic projection has only
three tensor lanes. Source-specific full-lane consumers need their own proven
raw read, not implicit canonicalization through this generic accessor.

Proposed source binder:

```text
shared_backend.try_bind_serialised_part_attachment_template(template)
  -> Arc<DynamicAssemblyPartAttachmentTemplate<
       SimulationAssemblyInertia, AssemblyPartMaterial>>
```

Reuse the current binder's host refusal order: Joint-count mismatch,
Drive-count mismatch, matched nonzero Joints, matched nonzero Drives; reserve
metadata and payload storage; then Definition-order Part route lookup and
metadata/material/inertia pairing. The checked original DMO corpus has
93 Assemblies/93 Parts/0 Joints/0 Drives, so this is a useful complete body-only
admission subset, not a claim that arbitrary vehicles lack constraints.

Migrate the DMO session's `SourceBackend` binding to the concrete shared
backend with `AssemblyPartMaterial/SimulationJointFrames/SimulationJointLimits`.
Its sealed asset/template owner, locator association and explicit initial
Part transforms remain the same. Reuse `DynamicAssemblyBodyOnlyPartSession`
with that backend, rather than constructing a dummy full assembly template
with guessed collision/Joint/Drive data. The old isolated backend can remain
as a staged test facility, but live DMO admission must not allocate it.

A borrowed facade is possible only with extra factoring: currently
`DynamicAssemblyBodyOnlyPartSession<B>` stores a transaction whose handle,
inertia, material and authority types are fixed by B, and `attach` accepts
`&mut B`. A temporary `Facade<'borrow>` therefore cannot simply substitute for
the stored old backend type. A facade design would need a lifetime-independent
storage/payload contract and methods generic over compatible backend views,
or an equivalent higher-level owner operation. That is broader than the
private inertia variant and would also need to expose the shared Part getter.

## Native Part getter: canonical host pose is insufficient

Fresh connected-IDA raw audit of
`rw::physics::Part::GetTransform[826c3480,826c35a8)`:
SHA-256 `ad60181e8e2c92d1f75b60b092d975aeb92f3166f64ea6627d7633fc827edf32`.
The hidden result pointer is r3, actual Part is r4; decompiler argument labels
are reversed. `826c3480` loads Part+4c body pointer before the inverse pointer
at +44. Detached branch copies actual cached X/Y/Z/P full vectors.

Attached without inverse-body transform:

```text
X = body[+40 .. +50)
Y = body[+50 .. +60)
Z = body[+60 .. +70)
P = body[+10 .. +20)
```

All sixteen lanes are copied, in X/Y/Z/P output order. These full vectors
include native adjacent words; they are not canonical affine W values.
The existing `serialised_assembly_create::packed_body_transform` already
reconstructs the proven layout from an occurrence snapshot and the existing
private packed-word provider:

| Lane | Actual source |
|---|---|
| X.W | Simulation/stasis address from the one image domain |
| Y.W | exact Add-time inertia outcome: forced/null zero or bound Definition address |
| Z.W | current body tag bits from the actual body snapshot |
| P.W | solver delta-row address for that actual body slot in the same image |

Attached with inverse-body transform computes body OUTER times inverse-body
INNER. Each output basis lane is separately rounded X product, then Y FMA,
then Z FMA. Position begins X FMA into body.P, then Y FMA, then Z FMA. Inner W
is not consumed; all four outer lanes participate. Stores at `826c3530..353c`
publish X/Y/Z/P. There is no normalization, identity shortcut or W cleanup.

Both current backend `rigid_body_pose` implementations and the shared
`try_part_body_pose` call `SimulationRigidBodyPool::try_rigid_body_pose`, whose
documented host boundary supplies basis W=0 and position W=1. Consequently
`try_live_part_transform_host` is not the exact source getter required by
DynamicPool line publication. Merely adapting its handle type would silently
lose observable output W and later GP-normal residuals.

Add a separate checked source-qualified getter on the actual body-only
session, conceptually:

```text
session.try_native_part_transform(shared_backend, part_ordinal)
  -> Result<[f32;16], SourcePartAccessError>
```

It validates the session's bound/attached authority, then the selected Part.
Detached reads its actual cache. Attached resolves the private handle through
the backend occurrence-to-route directory, takes the actual immutable body
snapshot and packed words from that same route/image, and performs the optional
inverse-body composition. Extract the existing private packed-body helper
into a shared internal operation; do not create a second word provider or
derive addresses from host pointers. No mutation-capable body handle escapes.
Typed unknown-lane refusal remains appropriate if its required image/source
authority is absent; canonical W is not a fallback.

The same issue applies during attachment rollback and detach:
`DynamicAssemblyPartAttachmentTransaction::copy_back_and_remove` reads
`backend.rigid_body_pose`, applies optional inverse-body composition, stores
the actual Part cache, then removes the body. Source-routed bodies must use
the same native packed transform there. Ordinary existing host-pose behavior
can remain explicitly scoped. The generic trait's copyback method is currently
infallible, so image/route invariants must be established before source Add;
do not introduce a normal recoverable missing-provider condition after body
mutation that this interface cannot represent.

## Actual Part volume and Create/refresh access

BeginUpdate needs separate lazy operations: load the selected Part's volume
reference first, then its actual Part transform, then invoke AddVolume.
A `try_part_volume(ordinal)` view can borrow the source owner's exact template
Part (volume ObjectId/root index) and the retained plan's CollisionArena root.
Return a borrowed occurrence view, not a cloned CollisionArena or filtered
`DynamicClusteredMeshTemplate::leaves()` list. Do not precompute every Part's
transform or skip the getter because its volume emits no mesh.

The current body-only transaction hides its handles intentionally. Add an
owner-integrated Create/read operation which constructs the existing
`SerialisedAssemblyCreatePartInput` in Definition order using checked body
observations and actual retained metadata/material. Read-only observations
may be retained by `PreparedSerialisedAssembly`; writable handles must remain
inside the session. It must verify all bodies belong to the selected same-arena
route, including each Static outcome record, before selecting that route's
existing packed-word provider.

Reuse `try_prepare_serialised_assembly_create_from_xenon` with actual template,
shared body pool, selected outcome owner, shared image domain, owner-generated
Part inputs and actual live assembly/Params/culling data. Reuse
`SerialisedAssemblyPoolUpdateInput` and existing pool refresh after admission.
An immutable borrowed context can expose these three authorities to that
owner-integrated operation without exposing mutable body pool access. This
borrowed read context is useful and does not suffer the backend-facade lifetime
problem described above.

Generic pool membership, world-index allocation, Body ID, bounds, active-list
publication, spy registration and DynamicPool record lifetime remain owned by
their actual enclosing admission owner. This bridge must not invent an ID,
pool list or serialized publication while merely binding source Parts.

## Essential limits and validation

The existing Simulation image and five arena resource bases are explicitly
caller-asserted staged values. Their checked layout and private generations
do not prove the real allocator lease. No currently inspected normal plan
provides actual live resource bases; XST size/alignment descriptors, serialized
offsets, Vec addresses and chosen test constants are not substitutes. A first
controlled corpus can exercise the same shared pool with explicitly staged
bindings, but that must not be called complete native address provenance.
Exact live Part full-W/Create publication still requires that authority. This
is an existing concrete dependency, not a reason to create another pool.

Focused bridge tests should prove:

- Existing player/manager bodies and source DMO bodies consume the SAME Free
  pool and share authority; independent byte-equal arenas remain distinct.
- Static source address skip, empty capacity precedence, correct effective
  state, failed late-Part reverse rollback, route-correct outcome retirement
  and preserved original Add diagnostic.
- Same-slot reuse across different arena routes receives a new occurrence;
  stale observation/directory/provider/Prepared authority fails closed.
- Full native Part getter with no inverse and with nontrivial inverse,
  noncanonical packed W, source tag, live moved body, detached cache, missing
  source authority and foreign manager; no canonical fallback or handle leak.
- Rollback/detach copyback stores the same full source Part transform before
  removing each body, including Static-null inertia and source pointer words.
- Exact source asset/template/Part volume identity and volume-before-transform
  access feed the real admission/Create/BeginUpdate path, followed by an
  original body-only DMO corpus case and two occurrences of one model.

Confidence: high for inspected owner/API facts and recommended factoring;
exact for the newly audited Part getter raw loads/math/stores. This document
does not claim new integration, allocator ownership, Cargo validation or a
native runtime trace. No new PPC math leaf is required for this bridge beyond
the already accepted packed-body/Part composition graph.
