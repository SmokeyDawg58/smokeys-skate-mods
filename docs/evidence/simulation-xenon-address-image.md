# Simulation Xenon address image

Confidence: **exact** for the ordinary valid 32-bit layout graph and relative
addresses; **host policy** for checked refusal; **unresolved** for the absolute
address chosen by the native allocator in a particular run.

## Authority

- `rw::physics::Simulation::GetResourceDescriptor`,
  `0x82ae67a8..0x82ae6834`.
- `rw::physics::Simulation::Initialize`, `0x82ae7b10..0x82ae7edc`.
- `EA::Physics::SimulationManager::SimulationManager`, beginning at
  `0x82e0c324`; its descriptor and Initialize calls are
  `0x82e0c520..0x82e0c588`.
- PDB sizes: `Simulation=0x300`, `RigidBody=0xb0`, and
  `Joint=Drive=0x20`.
- Matching XEX/PDB/MAP SHA-256 values are respectively
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`,
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`,
  and `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
- Raw Initialize extraction SHA-256:
  `8D17A696733EA988C5F92C9446362271040FA5ECA817BFEE9DBF5824D3A9AD35`.
  Normalized evidence SHA-256:
  `71B67B606CD3DDDF12033F1B54A7709B4E163CC9DE149EB331DB0B8BB3A4C0E3`.
  Checked oracle SHA-256:
  `4553E68A49910DDA43A1562F2B0112360E95A38A58005E204DC1D11A74386672`.

## Initialized address graph

For a resource base `S` and initialized capacities `N/J/D`, the source stores:

```text
simulation = S
rf         = align64(S + 0x300)
bodies     = align16(rf + N*0x40)
joints     = align4(bodies + (N+4)*0xb0)
drives     = align4(joints + (J+2)*0x20)
end        = drives + (D+2)*0x20
```

The four body sentinels are at `bodies + 0`, `+0xb0`, `+0x160`, and
`+0x210` for Free, Static, Frozen, and Active. Usable body `i` is
`bodies + 0x2c0 + i*0xb0`, and its RF row / native `mId` word is
`rf + i*0x40`. These formulas distinguish body addresses, body-list links,
and RF rows from host slot IDs.

The resource contract requests alignment `0x80`. The Rust image requires a
nonnull aligned base and checks every addition, multiplication, and align-up
before publishing an image. Native PPC operations wrap modulo 2^32 and expose
no recoverable failure; rejecting null, misaligned, or wrapping host inputs is
an explicit safety policy.

## Five packed body words

PDB layout fixes `RigidBody::mId/+0x1c`, `mRight/+0x2c`, `mLeft/+0x3c`,
`mStasis/+0x4c`, and `mInertia/+0x5c`. For usable slot `i`, Initialize stores
`mStasis = S` at `0x82ae7ca4/0x82ae7cbc` and
`mId = rf + i*0x40` at `0x82ae7c94/0x82ae7cc8`. `mStasis` is the Simulation
base, not a body-relative address, self pointer, sentinel, or null for an
initialized usable slot. Active Add dereferences it as `Simulation*` at
`0x82ae6f14..0x82ae6f1c`.

A live body's right/left words are the exact current intrusive-ring node
addresses. Body node `j` projects to `bodies + 0x2c0 + j*0xb0`; an anchor
projects to the applicable Free/Static/Frozen/Active sentinel above. A
one-body list therefore links to its sentinel, not itself. Initial linking is
at `0x82ae7c40..0x82ae7cf8`; Add rewires the rings beginning at
`0x82ae6ec0`, Remove at `0x82ae3190`, Activate at `0x82ae3228`, and Freeze at
`0x82ae3298`. The provider samples the live node identities rather than
reconstructing links from state or counts.

Active and Frozen Add store the supplied inertia word at `0x82ae6ef8` and
`0x82ae72bc`. Static materializes and stores null at
`0x82ae7604/0x82ae764c`. Reset, Remove, Activate, and Freeze preserve
`mStasis`, `mId`, and `mInertia`; Remove leaves stale inertia bits until the
slot is reused. Only the live occurrence's recorded Add outcome is therefore
authoritative.

`Part::GetTransform` consumes RF/stasis/inertia as W lanes from body
`+0x10/+0x40/+0x50` and `mTag` from `+0x60` at
`0x826c3480..0x826c35a4`. `CalculatePartBBox` consumes right/left beside the
velocity/omega vectors at body `+0x20/+0x30` (`0x82e0b844/0x82e0b7f8`). Rust
publishes these as exact `u32` bit images through `f32::from_bits`; it never
performs a numeric pointer-to-float conversion or NaN canonicalization.

## Manager descriptor asymmetry

`SimulationManager` calls `GetResourceDescriptor` with `B=N+1`, then calls
`Initialize` with `N`. The descriptor graph is independently:

```text
header_and_rf = align16((B+12)*0x40)
after_bodies  = align4(header_and_rf + (B+4)*0xb0)
after_joints  = align4(after_bodies + (J+2)*0x20)
size          = after_joints + (D+2)*0x20
alignment     = 0x80
```

For ordinary representable inputs and an aligned base, that allocation is
exactly `0xf0` bytes larger than the initialized span. It is slack, not an
extra usable body. Rust keeps the initialized image and manager allocation
descriptor as separate types and rejects `N+1` overflow before descriptor
math.

The literal discriminator `S=0x10000080,N=2,J=3,D=4` produces RF
`0x10000380`, bodies `0x10000400`, usable bodies `0x100006c0/0x10000770`,
Joints `0x10000820`, Drives `0x100008c0`, end `0x10000980`, initialized size
`0x900`, and manager allocation size `0x9f0`.

## Current boundary

This value does not choose an allocator base. The Definition-backed Part/DMO
route requires its arena Definition's runtime resource address for a
non-Static inertia word; generic `AddRigidBody` may instead receive a null or
otherwise arbitrary pointer and is not claimed to be Definition-derived.

The source template now retains the exact owner-qualified relative Inertia
locator for Definition Part `i`:

```text
parts = wrapping_u32(definition object offset + serialized parts displacement)
inertia = checked(parts + i*0xa0 + 0x10)
span = [inertia, checked(inertia + 0x30))
identity = (arena owner, Definition resource index, inertia offset)
```

The first addition deliberately wraps because the fixed-up serialized pointer
may address a Part array placed before its Definition header. Ordinal
conversion, the `0xa0` stride, `+0x10`, `+0x30`, resource selection, and full
span are checked host publication policy. `Part::Initialize` takes
`&PartDefinition::m_inertia` at `0x82ae9838`; the retained address is stored in
Part `+0x48` at `0x82ae8fec`, reloaded at `0x82ae0ad8`, and passed to
`AddRigidBody` at `0x82ae0bd8`. Definition fixup is
`0x82ae166c..0x82ae1678`, and the runtime Part stride walk is
`0x82ae0c38..0x82ae0c68`.

This locator exposes its exact immutable `0x30` bytes, including source
padding, but it does not expose or derive an absolute pointer. Same-owner,
same-resource, same-offset locators alias; byte-identical locations owned by
different arena plans do not. Synthetic tests cover a backward displacement,
the `0xa0` ordinal stride, complete-span boundaries, source resource
selection, suffix overflow, and owner identity.

The decoded owner now retains the exact five XST asset-allocation descriptors
which native `CreateCompressedArenaAssetData` forwards to `DoAllocate`.
These are allocation size/alignment inputs, not bases: the five live resource
bases are still not recoverable from the current parsed plan, and serialized
resource offsets, embedded Arena descriptors, or host `Vec` addresses are not
substitutes. Those values remain behind the injected provider
until one owner binds this image to the exact body-pool occurrence, live ring,
arena resource-address map, and allocation generation. No host address is
truncated or presented as a Xenon pointer.

The optimized dedicated DMO resident gate freezes the 125 owners' retained XST
pairs in physical asset order with signature `14ccc0e348011f23`. Its FNV-1a
schema hashes the V1 domain tag, then each big-endian asset ID followed by five
big-endian size/alignment pairs in resource-lane order. That closes descriptor
provenance only.

A crate-private staged binder now accepts five explicit caller-supplied bases
asserted to be allocator results and binds them to the exact normal-arena
`Arc`. It cannot prove their provenance or own an allocator lease. It
rechecks XST sizes against retained buffers, validates every nonempty lane independently, uses
checked absolute spans, and requires the final Inertia start to be 16-byte
aligned. Nonempty nonzero/power-of-two alignment and base validation, checked
rather than wrapping live-address arithmetic, and its process-local nonzero
binding epoch are explicitly safe-host publication policy, not native
recoverable failure paths. The epoch is minted only after complete five-lane
preflight and cannot be supplied or assigned to another independently
constructed binding by callers.

The returned non-Static candidate borrows that binding, carries its owner and
host binding-generation identity, and therefore cannot itself outlive the
staged address map. Its copyable diagnostic `u32` has no lifetime or packed-
word authority. It is not by itself a final rigid-body `mInertia` word. The
Add-time outcome owner below supplies Static selection and exact live body
occurrence authority.
The binder rejects the current nonzero-old-base case until resource-relative
parser semantics are expanded. Native does not prove cross-lane ordering,
contiguity, or non-overlap, so none is required; tests admit overlapping and
reverse-ordered bases. Empty lanes retain arbitrary explicit bases without
alignment or null normalization because that behavior remains allocator-
specific. Embedded descriptors, serialized resource-offset words, and host
buffer addresses are excluded from all address math.

The rigid-body pool exposes one crate-private topology snapshot for the
compositor. It first validates the complete
`SimulationRigidBodyObservation` and all four intrusive rings, then copies the
current right and left node identities plus whether the safe-host slot retains
an Inertia owner. It returns no address and no mutable link capability. The
address domain projects those nodes to exact body/sentinel addresses and the
observation's slot to its exact `rf + i*0x40` row.

`AddRigidBody` stores its input
at body `+0x5c` for Active (`0x82ae6ef8`) and Frozen (`0x82ae72bc`). Static
materializes zero at `0x82ae7604` and stores it at `0x82ae764c`. Reset,
Remove, Activate, and Freeze preserve that word. Therefore neither current
list/state nor equal decoded Inertia values proves its source.

A crate-private preallocated outcome owner now records the branch-selected
result against the complete pool/slot/occurrence identity. It distinguishes no
entry, Static-forced null, generic non-Static null, and a non-Static Definition
address. The Definition input is an opaque pair minted from one template Part,
so the semantic `Arc<SimulationInertia>` and exact source cannot be crossed
between ordinals in non-test code. The owner obtains the drop-inert body
reservation first, refuses a stale occupied sidecar cell, skips Definition
binding entirely for Static, and binds/clones the exact paired source before a
non-Static commit. The successful body commit is followed only by an infallible
move into its preallocated cell.

Lookup revalidates the live occurrence and all four rings before consulting the
record, and treats retained semantic-Inertia presence only as a consistency
check. Removal validates both body and exact record, mutates the pool, then
retires safe-host address authority; it does not claim native Remove cleared
body `+0x5c`. Slot reuse receives a new occurrence and cannot inherit an old
record. Direct mutation around this private foundation fails closed as missing
or occupied rather than reconstructing provenance. The table does not own or
prevent direct pool mutation; only the eventual production backend can enforce
that routing.

The complete five-word provider combines these sources. A persistent,
non-Clone image domain binds one checked Simulation image and a fresh nonzero
generation to the exact body-pool authority. A private short-lived provider
borrows that domain, the immutable pool, and one arena's outcome owner for the
entire Create or Update call. Its retained authority is the pair `(Simulation
image generation, arena binding generation)`, so a separately reconstructed
equal-valued image domain or arena binding cannot authorize an existing
prepared assembly. A replacement outcome table sharing the same binding has
the same pair but still fails closed when it cannot match the recorded body
occurrence. Production-shaped Create/Update wrappers construct the provider
internally from the template passed to that same call; the concrete provider
cannot be retained or detached and used with another arena.

Both transform and motion requests validate the complete occurrence, all four
rings, retained-Inertia consistency, and matching Add record before projecting
an address. The exact result is `stasis=S`, `solver row=rf+i*0x40`, current
right/left node addresses through the same image, and either zero or the
recorded Definition address for inertia. Static null and generic non-Static
null remain distinct below this packed layer even though both publish zero.

Whole-ring validation, immutable occurrence-consistent sampling, provider
epochs, ABA refusal, and recoverable errors are safe-host strengthening.
Native performs progressive unchecked reads and has none of those authority
tokens. The current image domain still accepts a caller-asserted base and does
not own the Simulation allocator lease; the arena binding likewise lacks its
allocator lease. Pool/image allocation-generation provenance, multi-arena
routing, exclusion of direct pool mutation around the outcome sidecar, and
native concurrent/racy rereads remain open. The production backend must route
multiple resident arena bindings and pass Part's effective post-override
state: exact requested state `4` plus nonzero `internalFlags.bit0` becomes
Static state `1`, while every other requested state passes through.

The optimized real-data DMO gate additionally proves that all 93 retained
source assemblies contain 93 Parts, zero Joints, zero Drives, and zero
runtime/Definition Joint or Drive count mismatches. This permits a first
DMO-bank bridge to reject nonzero Joint/Drive definitions without silently
discarding any member of the checked reference corpus. A separate V1 corpus
fingerprint covers every Part's arena asset ID, Assembly/Definition IDs,
Definition resource index, resource-relative offset, and raw `0x30` inertia
span: `ce61e8a464f9ccdd`. The older Create and Update fingerprints remain
unchanged at `2de19d3348a1226b` and `eac3e52c13270389`.

Additional authority artifacts are `simulation_add_rigidbody_raw.txt`
SHA-256 `4F46E8D55F2B2F4985A8833CC831624A946FA2CFBC15765B8F2D9B9DE2974D69`,
`rigidbody_lane_mutators_raw.txt`
`2999835F5CB23FFF9CF4CE64A22250C3D3450CB24980DE6BBE1DC4EF4110C5B8`,
`dmo_provider_raw.txt`
`FBCCC7AE83588A0E3C21C0664D664DBE6E67F9666C63C85626746880D7328C81`,
and `_temp_bbox_audit.log`
`2502B37462901CD7010DFF9B795C2A952A05649083D9674FC9398D95AF981731`.

## Staged source-aware Assembly backend

The native DMO ownership chain fixes the order which a later production owner
must preserve. `SimManager::SpawnStatic` resolves the raw Assembly Definition
through `tInstance +0x84 -> cModelData`, the special mesh-table route, and the
mesh entry's `m_BoneName.Offset` at `0x825eb19c..0x825eb1c8`. DMO `Body`
construction at `0x825ea858..0x825ea954` retains that Definition and separately
allocates the runtime Assembly through `PhysicsUtility::CreateAssembly`
(`0x82e4a540..0x82e4a5b8`). `cLivingWorldCollisionBody::AddToWorld`
(`0x82e50cb0..0x82e50e60`) writes Part transforms forward, assigns the Assembly
group, calls the Island insertion slot, and only then registers the resulting
Part bodies as spy targets.

`Island::AddDMOBody` (`0x827b6798..0x827b6840`) holds the Island critical
section across `AssemblyPool::Insert`. Insert (`0x82e0dbb0..0x82e0dc98`)
assigns and increments `worldIndex`, calls `Assembly::AddToSimulation`, ignores
its Boolean result, publishes the Assembly pointer, optionally calls
`SerialisedAssembly::Create` and copies its AABB, then increments the assembly
count last. `Assembly::AddToSimulation` (`0x82ae0df0..0x82ae1158`) walks Parts,
Joints, and Drives forward and owns its exact family-specific rollback on the
first null result. Native therefore orders `Create` after the Add call but does
not expose a recoverable Insert failure; successful continuation relies on the
upstream pool/capacity invariant. This is not evidence that Insert checks the
ignored result.

For each Part, `Part::AddToSimulation` (`0x82ae0aa0..0x82ae0bf8`) changes only
requested state `4` with nonzero internal-flags bit zero to effective state
`1`, then loads the inverse transform and raw Inertia pointer and calls
`Simulation::AddRigidBody`. That callee checks zero Free first at
`0x82ae6e20..0x82ae6e3c`; Static does not consume caller Inertia data and
stores null, while Frozen/Active retain the supplied word at `0x82ae72bc` and
`0x82ae6ef8`. The returned pointer, including null, is written to the Part
before conversion to the caller's Boolean result.

Native `SimulationManager` obtains the Simulation descriptor, allocates
`SimulationMemory`, stores its base at manager `+0x08`, and passes that base as
resource lane zero to `Simulation::Initialize` at
`0x82e0c520..0x82e0c588`. Its destructor frees that stored allocation through
the retained allocator at `0x82e0bce0..0x82e0bcf8`. The streamed arena side
likewise receives five allocator-returned bases before `Arena::FixupStart`, but
the audited path has not yet proved the exact lease/release owner that keeps
those bases alive for every DMO occurrence.

The new crate-private source-aware backend is a staged safe-host owner around
that recovered transaction. It owns one rigid-body pool and one checked image
domain, preallocates one Add-outcome owner per explicitly supplied arena
binding, and retains immutable route records. Construction rejects duplicate
exact arena owners, repeated host binding epochs, Simulation-image capacity
mismatch, and binding-owner mismatches before publishing the backend. A
Definition Inertia is prebound by exact arena-owner identity; its private route
token is retained in both the payload and returned body handle, so rollback/
removal reaches the same occurrence outcome owner. Full capacity remains the
native-shaped `None` result. Other routing failures are compressed to `None`
by the existing generic backend trait and retained only as a host diagnostic.
Joints and Drives are explicitly unsupported rather than silently discarded.

Those route tokens, binding epochs, recoverable validation errors, duplicate
checks, complete ring preflight, and occurrence sidecars are host safety
policy. They are not native fields or native failure branches. Static still
receives the session's effective state and reaches the outcome owner's proven
source-skip path; the staged prebinding step checks owner identity but does not
prematurely resolve the Definition address.

No attached-Part handle traversal is exposed from `DynamicAssemblySession` at
this checkpoint. Its backend handle is `Copy`, so even a borrowed callback
would let crate code retain a mutation-capable token beyond the intended
scope. The future post-attach `Create` path must instead keep handle use inside
an owner-integrated operation which does not publish that capability.

Eighteen focused source-backend discriminators cover byte-equal arena owners,
Static source skipping, an unregistered owner, a foreign backend route, an
invalid non-Static binding, late-Part rollback/outcome retirement with the
diagnostic preserved, Unsupported-Joint rollback, duplicate routes,
constructor refusal precedence, capacity rollback, same-slot route reuse, and
seven serialized Part-batch boundaries. Together with the eight existing
backend transaction tests, the module has 26 focused tests. The optimized DMO
corpus remains exactly 93 Assemblies / 93 Parts / 0 Joints / 0 Drives, with
zero runtime/Definition constraint-count mismatch. That corpus permits a
body-only first bridge; it does not prove that arbitrary Assemblies have no
constraints.

The backend now seals a reusable attachment-only batch from one parsed
`SerialisedAssemblyTemplate`. It first applies the explicitly host-defined
Joint-count mismatch, Drive-count mismatch, matched nonzero Joint, then matched
nonzero Drive refusal order. After both output reservations, it walks the
template-preserved Definition order once and pairs each matrix/raw flag record
with its three-float Material snapshot and inseparable semantic/source Inertia.
Exact arena-owner route lookup runs for every Part, including Static, but does
not validate the routed Xenon Definition-Inertia address. That safe-host check
remains at body Add, where Static does not consume the address.

This batch creates no body or session and is not by itself the selected
serialized-source owner. Nonempty routed payloads retain the arena plan; an
empty batch retains no arena. At this layer the batch carries neither the
selected DMO asset nor the selected typed Assembly/Definition identity,
culling association, or Create/Update source lists. The enclosing source owner
described below now retains the exact processed asset and
dictionary-filtered template ordinal beside it. Material remains an immutable
three-word host snapshot, not native Part pointer identity or later Definition
alias visibility.

This checkpoint does **not** wire `SerialisedAssembly::Create` or a production
DMO spawn. Still required are ownership of the actual Simulation and arena
allocator leases; a live occurrence caller which consumes the resident
model-ID route described below; an owner-integrated post-attach body operation;
production multi-arena route selection/lifetime; live Part transforms and
mutable Volume inputs at the native call time; the remaining numeric
transform/motion provider beside the five packed W words; and Joint/Drive
payload support for nonzero-constraint Assemblies. It makes no boot or
playability claim.

The additional raw packets for this boundary are
`dmo_live_placement_raw.txt` SHA-256
`0FF8604D86BDD60DFD12B5452E7EA3A17D0D6BD1FB2CB9D921340BE8D87362D8`,
`assembly_transaction_raw.txt`
`71D2D6F7F938D69B15D1E4AE510A3004AD3E3D606934264D458489B89BA2255E`,
`dmo_body_binding_raw.txt`
`8BA5D8692B87B688693B0E9E0E19A8354DA7C4012416CE654A5A5458118F4E8D`,
`assembly_materialization_raw.txt`
`E500AD9477E93109AAA30FA4951A302977025C014CD9DE5283D93644432BEFE8`,
`arena_live_binding_raw.txt`
`F73BBFCAC6F4A7E4A827A2565DF9215943B370D42E9298A19381F4614EE44879`,
and `simulation_initialize_raw.txt`
`8D17A696733EA988C5F92C9446362271040FA5ECA817BFEE9DBF5824D3A9AD35`.

## Exact serialized DMO source ownership

The attachment-only batch is now enclosed by a crate-private immutable source
owner. Selection uses exact raw equality against a local Assembly `ObjectId`
in `DmoBankProcessedArenaAsset`'s dictionary-filtered template vector. The
owner retains the exact asset `Arc`, the validated ordinal and ID, the sealed
Part batch, and one backend authority. Nonlocal, missing, duplicate,
template-owner, route-owner, and backend-affinity refusals are safe-host policy;
they are not native fields or native recoverable branches. In particular, the
ordinal is scoped to the retained asset rather than treated as a global or
native identity.

Native does not perform this host lookup at `SpawnStatic`. By then,
`CollisionModelData +0x08` has already fixed the Assembly pointer relationship.
The Rust lookup preserves the serialized local ID which produced that
relationship while avoiding a guessed default template or entry-index-only
match.

The corresponding opaque session retains the source owner and the handle-free
Part transaction. It accepts one explicit initial transform per Part and a
requested state at attach, preserves forward Add/reverse rollback/forward
copyback-removal, and checks repeated lifecycle calls and backend affinity
before effects. It exposes no Copy body handle, route token, routed payload,
attachment batch, or callback.

The source owner closes the previously open exact asset/template association.
The resident model catalog described below now also retains a weak route from
each cataloged local model ID to the exact processed-asset allocation and
local Assembly ID. What remains open is the live occurrence owner that selects
that route and carries it through native transform/state authority, allocator
leases, multi-arena occurrence lifetime, internal post-attach body access,
`SerialisedAssembly::Create`/Volume inputs, `worldIndex`, numeric motion
providers, and nonzero constraints.

The seven new tests distinguish exact second-template payload selection,
full-ID/nonlocal/missing/duplicate behavior, byte-equal foreign arena owners,
backend preflight order, late-Part rollback/outcome retirement, owner lifetime,
zero Parts, and generic body-only copyback order. The workspace gate passes
1,538 core tests plus the unchanged formats/application/symbol/doctest suites
at that source-owner checkpoint; the optimized 93-Assembly real-data
fingerprints remain unchanged.

## Resident main-DMO model catalog

PDB evidence fixes `pegasus::tInstance` at `0xa0` bytes, with primary GUID at
`+0x60`, local GUID at `+0x68`, and the collision-model pointer after fixup, or
its serialized optional `ObjectId` before fixup, at `+0x84`.
`Dmo::Registration::RegDmoModel` (`0x825eab98..0x825eac28`) loads the full
local GUID at `0x825eabb8`, looks it up at `0x825eabc4`, and exits on an
existing key at `0x825eabe0`; native therefore keeps the first registered raw
`tInstance*`. An absent key reaches the count/`0x100` comparison at
`0x825eabe4..0x825eabec`, assert/debug path at
`0x825eac00..0x825eac04`, and insertion at `0x825eac08..0x825eac1c`.
The 256-entry limit is manager-global, not a per-bank or per-collection native
capacity.

`SimManager::SpawnStatic` (`0x825eb0a0..0x825eb3a8`) loads the locator model ID
from `tDMO +0x70` at `0x825eb168`, performs the model lookup at `0x825eb17c`,
obtains the registered `tInstance*` at `0x825eb19c`, follows its collision
pointer at `0x825eb1a8`, tests the special collision-model marker at
`0x825eb1b4..0x825eb1bc`, follows `CollisionModelData +0x08` at
`0x825eb1c0`, and obtains the Assembly Definition at `0x825eb1c4`. This runtime
path follows fixed pointers; it does not perform the host validations below.

Core commit `739bcb6` adds a weak, owner-qualified catalog over one exact
resident collection. It walks physical asset order and then operation order,
selecting only `DmoInstance` rows. It first obtains the retained record and raw
local-GUID key, then rejects duplicate keys and the 257th unique row before
validating deeper primary/local identity and the serialized collision-model to
Assembly relationship. It requires a nonzero local collision-model `ObjectId`,
the exact dictionary type, one projected collision model, a present Assembly
relationship, and one exact local Assembly template. All typed refusals are
atomic safe-host policy and publish no partial catalog.

This deliberately differs from native duplicate handling: native silently
keeps the first pointer, while Rust rejects the catalog before publication.
Rust also applies 256 per catalog. That guard is not manager-global parity if
multiple catalogs coexist. The checked main bank is below either boundary at
125 assets and 93 unique local-GUID routes.

The real hull probe seals the catalog during host wrapper construction before
publication. It verifies residency and host V1 signature
`a54da4551bdf2f49` at the evidence-backed raw-state-8 checkpoint. After the
explicitly synthetic cleanup it verifies expiration without losing diagnostic
count/signature state; that observation is not native unload behavior. The
catalog and each row retain weak identity only, so the catalog itself does not
extend bank residency.

The signature is a host FNV-1a regression diagnostic, not a native table hash,
pointer value, or model-table layout. Its domain separator is
`SKATE2/DMO/RESIDENT_MODEL_ROUTES\0V1\0`. Each physical route contributes,
in big-endian form, resident asset ID, asset ordinal, operation ordinal,
primary GUID, local/model GUID, raw collision-model `ObjectId`, and raw
Assembly `ObjectId`. The catalog source SHA-256 is
`B38CB2E1A13C71D2BA80B7DF77EA73D2F7EF9267005C859EC017572710614E6A`;
the integrated probe source SHA-256 is
`43C7E8A70D67C806B5A24BBE9832D338C16B7FA3BCF41C3A54CA7D1872F1397D`.

Twelve catalog tests cover physical ordering, local-key selection, validation
and refusal precedence, duplicate/capacity behavior, stable signature,
resident tombstoning, exact allocation identity, and selected-owner lifetime.
Validation passes 1,550 core unit tests plus two compile-fail doctests, 173
format tests, 19 application tests, and three symbol tests, together with
formatting, diff checks, warning-denying workspace Clippy, and the optimized
real-data probe.

The hull probe does not call the crate-private resolver, choose a live locator
occurrence, install the native model table, execute `SpawnStatic`, create a DMO
Body/runtime Assembly, source live transforms or requested state, call
`AddToWorld`, `Assembly::AddToSimulation`, or
`SerialisedAssembly::Create`/`Update`, publish `worldIndex`, supply allocator
leases or live Volume/numeric inputs, run offline simulation, or support
nonzero Joint/Drive payloads. Those remain production boundaries rather than
results of the resident-catalog checkpoint.

The authoritative packet is `dmo_live_placement_raw.txt` SHA-256
`0FF8604D86BDD60DFD12B5452E7EA3A17D0D6BD1FB2CB9D921340BE8D87362D8`.
The XEX, PDB, and MAP SHA-256 values are
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`,
`0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`,
and `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.

## Exact retained locator occurrence

Core commit `93e0cc9` adds a crate-private occurrence selector over the already
owner-bound kind-31 source. Its source SHA-256 is
`2F000578874780EF246DEC4214B2A2614FCC6147DE4CDB571FEB6AADA3645310`.

The PDB fixes the serialized layouts used by the selector:

| Type/field | Offset | Size |
|---|---:|---:|
| `tDMOData::m_TypeID` | `0x00` | `4` |
| `tDMOData::m_uiNumDMOs` | `0x04` | `4` |
| `tDMOData::m_uiNumStrings` | `0x08` | `4` |
| `tDMOData::m_DMOs` | `0x0c` | `4` |
| `tDMOData::m_StringList` | `0x10` | `4` |
| complete `tDMOData` header | | `0x14` |
| `tDMO::m_TransformMatrix` | `0x00` | `0x40` |
| `tDMO::m_BBox` | `0x40` | `0x20` |
| `tDMO::m_uiGuid` | `0x60` | `8` |
| `tDMO::m_uiGuidLocal` | `0x68` | `8` |
| `tDMO::m_uiGuidInstance` | `0x70` | `8` |
| `tDMO::m_Instance` | `0x78` | `4` |
| `tDMO::m_Name` | `0x7c` | `4` |
| complete `tDMO` record/stride | | `0x80` |

`AddLocatorsForStreamedLocatorChunk` reads the container count at
`0x82d3bf28`, reloads `m_DMOs` at `0x82d3bf40`, and advances its record byte
offset by `0x80` while rereading the count at `0x82d3c0a4..0x82d3c0b4`.
It loads `m_uiGuid` at `0x82d3bf4c`, loads `m_uiGuidInstance` into locator
model state at `0x82d3bf78..0x82d3bf8c`, and calls `CreateLocator` with the
former at `0x82d3c048..0x82d3c054`. The outer container ID is not that call's
locator ID. `SpawnStatic` independently reads the count and array pointer at
`0x825eb11c` and `0x825eb12c`, advances by `0x80` at
`0x825eb35c..0x825eb36c`, and uses record `+0x70` as its model-map key at
`0x825eb168..0x825eb17c`.

The selector follows the retained serialized `dmos_offset` and therefore also
supports a valid record array which does not begin at canonical header size
`0x14`. It checks operation and ordinal before checked offset/span arithmetic,
then freezes the three big-endian `u64` lanes at `+0x60`, `+0x68`, and `+0x70`
inside the crate-private result. `+0x60` and `+0x70` have the consumers above;
`+0x68` remains a preserved authored lane because no use was recovered in
these immediate native paths. Zero values are preserved rather than rejected.
That is the honest source-selection behavior: the inspected native functions
have no generic zero guard, but neither this selector nor the corpus proves
zero-ID registration or spawn behavior.

The result's identity is explicitly safe-host policy:

```text
exact retained source backing
+ authored operation index
+ full outer container asset ID
+ locator ordinal
```

This keeps two processed registrations that alias the same `tDMOData` source
distinct. Native provides the constituent relationships—`ProcessDMOData` at
`0x82d5f4f0` carries a full TOC/container ID and whole `tDMOData*`, while
`RegDmoLocator` at `0x825eac28..0x825eacac` keys that whole-object registration
by the full ID—but it defines no comparable composite occurrence identity.
Container ID alone is likewise not individual locator identity. The checked
corpus has 815 unique container IDs and zero source-span aliases, so the alias
discriminator is a safe synthetic boundary rather than an observed corpus
collision.

Seven new occurrence tests cover exact ordinal and three-ID selection,
noncanonical valid `dmos_offset`, positional operation/source ownership,
refusal precedence, all four identity components, zero-ID retention, and
source lifetime. The module now has 13 tests; the current core count
is 1,557 unit tests plus two compile-fail doctests. The full workspace test and
warning-denying Clippy gates pass.

This checkpoint neither installs `RegDmoLocator`/`RegDmoModel`, exposes the
private three IDs publicly, invokes the resident catalog resolver, nor calls
`SpawnStatic`. It supplies no live transform/state, allocator lease, DMO Body,
post-attach body access, Volume/numeric provider, `SerialisedAssembly::Create`,
or Joint/Drive payload. The next implementation boundary is one combined
owner/session that consumes this exact occurrence through its resident model
catalog route and retains both sides' owner authority.

The direct packets are `dmo_live_placement_raw.txt` SHA-256
`0FF8604D86BDD60DFD12B5452E7EA3A17D0D6BD1FB2CB9D921340BE8D87362D8`
and `dmo_body_binding_raw.txt` SHA-256
`8BA5D8692B87B688693B0E9E0E19A8354DA7C4012416CE654A5A5458118F4E8D`.

## Consume-only exact locator/model/Assembly ownership

Core commit `ccf00f3` joins the exact retained locator occurrence to the weak
resident model catalog without exposing a freely callable or public raw-key
catalog lookup. The only value used for model selection in this bridge is the
occurrence's private `tDMO +0x70` lane, which
matches the `SpawnStatic` load at `0x825eb168` and lookup at `0x825eb17c`.
The `+0x60` locator GUID passed to `CreateLocator` and the separately retained
`+0x68` local GUID do not participate in this selection.

The catalog issues a non-constructible, one-shot resolver to the parent
broker; the occurrence consumes itself and privately supplies `+0x70`.
Resolution first validates the exact resident collection occurrence, then
looks up the full 64-bit key and upgrades the exact weak processed-asset
allocation. The combined owner next binds that asset's already-validated
local Assembly Part source to the caller's backend authority. The session
retains both owner chains and delegates only the previously recovered
body-only Part lifecycle.

All ordering around the native relationship is stated narrowly. Catalog
choice, capability topology, `Arc` lifetime, typed errors, and liveness-first
lookup are safe-host policy. Bridge errors are intentionally key-free, even
through `Debug`, `Display`, and `Error::source`. Zero is sent through ordinary
lookup rather than treated as a host sentinel; this does not prove that native
registration or spawn succeeds for zero. Initial Part transforms and requested
body state are still explicit inputs and therefore are not native live values.

The five new discriminators prove:

- exact `+0x70` selection while all three lane values are distinct catalog
  routes and the target occupies different locator-record/catalog positions;
- zero-key lookup and distinct identity for equal serialized records;
- missing, stale-plus-missing, and ABA catalog behavior before backend effects;
- exact Part-owner route failure and transform-count refusal before mutation;
- zero-Part lifecycle and retention of both locator and model source owners.

The full validation gate passes 1,562 core tests, 173 format tests, 19
application tests, three symbol tests, two compile-fail doctests, formatting,
diff checks, and warning-denying workspace Clippy. Source SHA-256 values are
`502D72B9C87C9D9F6158FF14EC7E14665B9B1C6B16074A460002580AA3DB8D53`
for the catalog, `713DC786978E929F7398E9992F9036301F7786BC41C7A4D5FFFAA2EA78B16C61`
for the locator plan, and
`D781B137CB863D78DF823206909BDC751DDF2443B72D426550FED795557FAD2E`
for the combined Part owner/session.

This checkpoint does not install `RegDmoLocator`/`RegDmoModel`, select a
catalog for a production multi-arena occurrence, traverse live registered
DMOs, execute `SpawnStatic`, create a DMO Body/runtime Assembly, call
`SerialisedAssembly::Create` or `Update`, publish `worldIndex`, obtain
Simulation or arena allocator leases, source native transforms/state or
mutable Volume/numeric motion inputs, or support nonzero Joint/Drive payloads.

## Manager-scoped locator registry mutation

The authoritative production-chain packet is
`world_probe_temp\dmo_production_chain_raw_3f17155988daceb338e4b6cff7dc70ad31b25fa7a74f3d41796ca3e42da7d028.txt`,
SHA-256
`3F17155988DACEB338E4B6CFF7DC70AD31B25FA7A74F3D41796CA3E42DA7D028`.
The original mutable output path was later regenerated; this immutable packet
was recovered exactly from `sk82_dmo_live.i64` and reverified at the cited
SHA-256.
It was extracted from the XEX whose SHA-256 remains
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.

PDB types place the model and locator tables inside the `0x270c`-byte
`Sk8::Dmo::Registration` base:

| Field | Offset | Native type/capacity |
|---|---:|---|
| `mModels` | `0x04` | `fixed_hash_map<AssetID, tInstance const *, 256, 257>` |
| `mLocators` | `0x2068` | `fixed_hash_map<AssetID, tDMOData const *, 50, 51>` |
| locator element count | `0x2074` | `u32` |

`RegDmoLocator` first finds the complete container `AssetID` at
`0x825eac44..0x825eac50`. A hit branches directly to return at
`0x825eac6c`; the original stored pointer wins. Only a miss loads the element
count and compares it to `0x32` at `0x825eac70..0x825eac78`, reaches
`FailAssert`/`DebugBreak` at `0x825eac8c..0x825eac90` when full, invokes map
insertion at `0x825eac94..0x825eac9c`, and stores the exact `tDMOData*` at
`0x825eaca0`. The debug-break continuation is not modeled as a native
recoverable error.

`DRegDmoLocator` finds by the complete ID at `0x825ea6d0..0x825ea6dc` and
erases a hit at `0x825ea6fc..0x825ea714`. Its source-pointer argument is never
read. A missing key branches directly to return. `Registration::Clear` calls
the locator table's node release at `0x825ea744..0x825ea750` and zeros its
element count at `0x825ea754..0x825ea758`; it does not touch `mModels`.

Core commit `b3b408a` adds a crate-private, manager-scoped owner corresponding
to those mutation rules. Its 50-row preallocation, typed capacity refusal,
retained `Arc` source ownership, and pointer-equivalent identity checks are
safe-host mechanisms. Duplicate-before-capacity, first-wins source retention,
key-only deregistration, missing no-op, and locator-only clear follow the
recovered native branches. Full zero and nonzero 64-bit IDs remain ordinary
keys.

The selection gate resolves the exact current full-ID winner and verifies its
backing object before minting one opaque locator occurrence. That occurrence
is only a point-in-time membership attestation; it is not a recovered native
token or epoch. No table iterator is exposed. Native `SpawnStatic` traverses
EASTL bucket/node order, and this checkpoint deliberately does not substitute
Rust storage or registration order for it.

Six focused tests bring the core inventory to 1,568 and distinguish 50th/
51st registration, duplicate precedence at capacity, loser-source removal,
source ABA, two full IDs with identical low 32 bits, two IDs sharing one exact
source, alias-qualified occurrences, clear ownership and reuse, and selection
refusal order. The full workspace test, formatting, diff, and warning-denying
Clippy gates pass. Source SHA-256 is
`75CB780DB9A9ACE3A25599CF957D3162C7128769DA504635B02676FDF7ECCA94`.

This does not reconstruct the separate 256-entry model table, install the
registry in the live collection adapter, recover AssetID hash/bucket traversal
order, run the network-collision query, call `SpawnStatic`, allocate either
body, call `Assembly::AddToSimulation` or optional
`SerialisedAssembly::Create`, source live transforms/state, or simulate a DMO.

## Manager-scoped model registry mutation

The direct evidence packet is
`world_probe_temp\dmo_model_registry_raw.txt`, SHA-256
`4718F3FC499112AD38D4387037B553573FE54BAFE258F764200571E3EF4CE28F`.
It supplements rather than replaces the production-chain packet above; that
packet retains its `3F17...` SHA-256 attribution.

PDB types place
`fixed_hash_map<assman::cAssetID, pegasus::tInstance const *, 256, 257>` at
`Sk8::Dmo::Registration +0x04`. `RegDmoModel`
(`0x825eab98..0x825eac28`) performs the following exact visible sequence:

- `0x825eabb8`: load the full 64-bit local GUID from `tInstance +0x68`;
- `0x825eabc4`: find that key in `mModels`;
- `0x825eabe0`: return on a hit, preserving the first pointer;
- `0x825eabe4..0x825eabec`: on a miss, load the element count and compare it
  unsigned against `0x100`;
- `0x825eac00..0x825eac04`: call `FailAssert`, then `DebugBreak`, when full;
- `0x825eac08`: reload `tInstance +0x68`;
- `0x825eac18..0x825eac1c`: obtain the insertion slot and store the exact
  incoming `tInstance*`.

The separately passed `AssetID` is not read. `DRegDmoModel`
(`0x825ea628..0x825ea6b8`) loads `+0x68` at `0x825ea650`, finds the key at
`0x825ea658`, tests the end sentinel at `0x825ea65c..0x825ea674`, compares
the stored pointer with the exact incoming pointer at
`0x825ea678..0x825ea680`, and erases only a match at `0x825ea69c`. Its passed
`AssetID` is also unused; missing keys and pointer mismatches return unchanged.

The native vector-deleting destructor destroys `mLocators` first at
`0x825eafd4..0x825eafe4`, then `mModels` at
`0x825eafe8..0x825eafec`. In contrast, `Registration::Clear`
(`0x825ea730..0x825ea770`) invokes locator-node release at
`0x825ea744..0x825ea750` and zeros only locator count `+0x2074` at
`0x825ea754..0x825ea758`; it never clears the model table.

Core commit `c8ad2e6` implements a private `SimulationDmoModelRegistry` with
preallocated 256-row storage. Registration consumes an opaque exact source,
keeps the first full-key winner before checking capacity, and retains that
winner's complete `DmoBankProcessedArenaAsset`. Deregistration consumes the
same capability shape and removes only a full-key plus
`SimulationRecordHandle::is_same_backing_record` match. No key, record, asset,
row count, iterator, or `clear` authority escapes the module.

Typed allocation/capacity refusal, one-time immutable `+0x68` sampling, and
operation/kind/span/exact-size/local-GUID validation are safe-host policy.
Native reloads `+0x68` before insertion, so mutation between its two reads is
not modeled. Native full capacity follows `FailAssert`/`DebugBreak`; the Rust
error is an atomic host translation, not a recovered native return. The
current source accepts only `DmoBankProcessedArenaAsset`, so it is a staged
main-DMO-bank path rather than arbitrary normal-arena kind-11 coverage.

Eight focused tests bring the core inventory to 1,576 and discriminate all
source refusals, 256/257 and duplicate-at-full precedence, local versus
primary/full-64-bit key behavior, pointer-equivalent removal across equal and
aliased sources, record-offset identity, ABA, and full-asset retention. Source
SHA-256 is
`DDEB9B1B7E9FD8568DEBE8C4A33D340E4583CE9AA0E931080896C9823D560E3A`.

This checkpoint does not wire a collection/normal-arena adapter, admit every
live qualifying kind-11 operation, combine locator and model manager
lifetimes, expose model lookup, replace the existing explicit-catalog bridge,
recover native hash/bucket traversal order, or execute `SpawnStatic`. It makes
no live registration or spawn claim.

## Manager-winner-qualified locator/Part bridge

The applicable raw packet remains
`world_probe_temp\dmo_model_registry_raw.txt`, SHA-256
`4718F3FC499112AD38D4387037B553573FE54BAFE258F764200571E3EF4CE28F`.
Within native `Sk8::Dmo::SimManager::SpawnStatic`
(`0x825eb0a0..0x825eb3a8`), the visible ordering is:

- `0x825eb158`: call the per-locator virtual network-collision query;
- `0x825eb160..0x825eb164`: skip this locator when the query returns false;
- `0x825eb168`: load the full lookup key from locator record `+0x70`;
- `0x825eb17c`: call `mModels.find`;
- `0x825eb194..0x825eb198`: skip an end-sentinel/missing result;
- `0x825eb19c`: load the winning stored `tInstance*` from the model row.

There is no `tInstance +0x68` reread between that successful find and use of
the stored winner. The registration-time `+0x68` loads documented above and
the spawn-time locator `+0x70` load therefore form distinct evidence
boundaries.

Core commit `3ae556a` implements only the pure staged join beneath that native
control flow. The model registry is compiled once as a child of the private
DMO Part-session module. Its non-Clone/non-Debug resolver accepts a model key
only from a consumed `SimulationDmoLocatorOccurrence`; the former catalog-only
resolver that accepted that private value has been removed. A current winner
is borrowed from the manager-scoped registry to qualify the candidate catalog;
the synchronous join holds an immutable registry reference, and the successful
catalog projection supplies the strong exact asset used for Part construction.

Catalog selection is explicit safe-host policy, not a reconstruction of
native catalog discovery. The candidate must first remain resident, contain a
route for the private key, and match the current manager winner by both exact
`Weak<DmoBankProcessedArenaAsset>` allocation identity and
`SimulationRecordHandle::is_same_backing_record`. Only then does the catalog
perform its locked, alive-aware exact membership upgrade and mint the strong
model source used by the Part owner. The keyless refusal order is manager
miss, resident expiry, catalog miss, exact winner mismatch, resident-access or
asset expiry, then Part-owner binding. None of the sanitized bridge errors
exposes the raw key, source handle, or asset through formatting or its error
source chain.

After a successful join, the combined owner/session strongly retains the
locator occurrence and selected resident model/Part owner. Tests prove that
this ownership survives manager deregistration and resident collection/catalog
teardown until the combined owner/session is dropped. This is a Rust lifetime
contract only; native destruction, query, and spawn timing are not inferred
from it.

Two new tests raise the core count to 1,578 and directly discriminate same-key
candidate catalogs in both winner-registration orders plus a shared record
wrapped by a distinct processed-asset owner. The adapted bridge suite retains
manager-first refusal, loser deregistration and replacement, byte-equal
processed assets, teardown lifetime, and backend mutation atomicity. The exact
source SHA-256 values are:

| Source | SHA-256 |
|---|---|
| `simulation_dmo_model_registry.rs` | `2E026BCF861543DC8288EC029849ED762481BC381DBDCBFC205DD78499B6EC6C` |
| `dmo_bank_resident_model_catalog.rs` | `2BCD6E5FBEB8D42F87DC6A2A5CF5DF06C1F0712DCD6F668994CA72BC1BF47ADB` |
| `simulation_dmo_arena_plan.rs` | `2C4B503356B375437E5EBB0B4881671FD3B7E713FD20DE62A0754B55E0D45F2B` |
| `simulation_dmo_assembly_part_session.rs` | `6C9D0DF5C485033A1F05AB882CDB065DE32F43367E6C5FD934C385EB2D8DDAEC` |

This does not implement the native query gate or its timing, automatically
discover a resident catalog, admit arbitrary normal-arena kind-11 sources,
wire live model/locator registration, traverse or execute `SpawnStatic`,
construct a DMO Body or live Simulation owner, or recover EASTL hash/bucket
order.

## Combined manager-scoped DMO registration ownership

Core commit `9b02e4e` composes the two already-recovered registration tables
under one private owner. The direct packet is
`world_probe_temp\dmo_model_registry_raw.txt`, SHA-256
`4718F3FC499112AD38D4387037B553573FE54BAFE258F764200571E3EF4CE28F`.
The native lifetime evidence is narrow and explicit:

- `Registration::Registration` (`0x825eb020..0x825eb094`) constructs the model
  fixed map at `0x825eb05c`, then the locator fixed map at `0x825eb074`;
- the independent `Registration` vector-deleting destructor
  (`0x825eafb8..0x825eb020`) destroys locators at
  `0x825eafd4..0x825eafe4`, then models at `0x825eafe8..0x825eafec`;
- the separate `SimManager` vector-deleting destructor
  (`0x825eb520..0x825eb5a4`) also destroys locators at
  `0x825eb558..0x825eb568`, then models at `0x825eb56c..0x825eb570`;
- `Registration::Clear` (`0x825ea730..0x825ea770`) selects `mLocators` at
  `0x825ea744`, frees its nodes at `0x825ea750`, and writes zero only to
  locator count `+0x2074` at `0x825ea754..0x825ea758`.

No `Clear` caller is present in the recovered call list, so invocation timing
and frequency remain unknown. The native inline-storage fixed-map constructors
also expose no fallible result or allocation-error branch. Rust preallocates
256 model rows before 50 locator rows and returns typed initialization errors
with their exact nested source; both fallibility and heap preallocation are
safe-host policy, not claims about native fixed-map initialization.

`SimulationDmoRegistrationManager` declares its fields as locator then model,
so Rust's ordinary field destruction follows the visible native destructor
order, while `try_new` constructs the model owner before the locator owner.
The manager delegates model and locator mutation independently, preserving
their different duplicate-deregistration rules, and exposes a locator-only
`clear_locators`. Its combined resolution performs the registered-locator
selection first; only a successful opaque occurrence reaches the current
model-winner and caller-supplied candidate-catalog/Part binding path. The
concrete registries and the low-level registry-taking Part constructor are
sealed below the manager, with no raw key, row, source, iterator, or registry
getter.

Six adversarial tests prove locator-only clear is idempotent and reusable while
the model remains, locator errors precede model and even expired-catalog
errors, locator duplicate-loser deregistration removes the winner while model
duplicate-loser deregistration does not, full 64-bit locator/model domains are
independent, both retained sources live exactly with the manager, and nested
initialization error sources survive wrapping. The full validation gate passes
1,584 core tests, 173 format tests, 19 application tests, three symbol tests,
two compile-fail doctests, formatting, diff checks, and warning-denying
workspace Clippy.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_registration_manager.rs` | `31B437C8F9EA91B404E95E3CB1B0063AFA6406F86653C0EBD4156E5128E003D4` |
| `simulation_dmo_model_registry.rs` | `00BE6DCAB7CB7F31BC19FB7C0B19834E369BB8EC8ABF5D114872F273CA7AA8FB` |
| `simulation_dmo_locator_registry.rs` | `2F1B9ED80C856DBC541BEC5086A516D579FEDD2406C841EB50F1DC064F8B0806` |
| `simulation_dmo_assembly_part_session.rs` | `A6A4EC90546010F86FF81661C9E17193052382015C68AFAF8F774E6AA75307DD` |
| `dmo_bank_resident_model_catalog.rs` | `1275E0DA614F0AD85EC87F60D2E1EB3E3B74772004FD1CD59E0552865DCF3FED` |

This is not a live registration adapter and does not automatically select a
candidate catalog, execute the native query or `SpawnStatic`, admit arbitrary
normal-arena kind-11 sources, create a DMO Body, reproduce native hash/
traversal/layout, establish `Clear` timing, or introduce a combined
registration transaction/rollback contract.

## Exact resident main-bank model-source preparation

Core commit `20d84d5` adds a private, effect-free batch that prepares the exact
main-DMO-bank sources required by the model registry without mutating that
registry. Its only production input is
`&OwnedResidentCollection<DmoBankProcessedArenaAsset>`. Construction obtains
the collection's weak assets in resident authored order, upgrades and retains
the exact `Arc<DmoBankProcessedArenaAsset>` wrapper for every row, then scans
the full normal-operation streams in authored order and records only each
`DmoInstance` subsequence. Each source is validated before publication through
`SimulationDmoModelRegistrationSource::try_prepare`. Duplicate keys and every
valid source row survive unchanged: this stage neither deduplicates nor
applies the manager's 256-row capacity.

The sealed pass consumes actual callback record handles and primary asset IDs
in the prepared subsequence. `try_issue` samples resident liveness at that
instant, compares exact backing-record identity before the full 64-bit primary
ID, and leaves its cursor unchanged on every refusal. On success it prepares a
fresh consumed registration source from the retained exact asset/operation
pair and only then advances. `finish` checks callback completeness only and
does not resample or attest resident liveness. An issued owning source may
remain valid as an identity/lifetime carrier after the pass, batch, and
resident collection are gone; this is not live-dispatch authority. Creating
multiple fresh passes from one prepared batch is explicit safe-host reuse, not
native occurrence authority.

The source split remains material. The immutable production-chain packet
`world_probe_temp\dmo_production_chain_raw_3f17155988daceb338e4b6cff7dc70ad31b25fa7a74f3d41796ca3e42da7d028.txt`
(SHA-256
`3F17155988DACEB338E4B6CFF7DC70AD31B25FA7A74F3D41796CA3E42DA7D028`)
supports the outer manager chronology. Separate processed-bank corpus evidence
fixes the staged main bank at 125 resident assets and 93 kind-11 operations
with userdata `0x0a`; the world/BAM kind-31 locator path instead retains an
exact whole-object handle. These sources are intended to feed one manager
under the outer chronology, not a new per-arena combined registration
transaction.

Nine tests cover authored ordering across two assets with interleaved
non-DMO operations, exact-record-before-full-ID mismatch precedence, duplicate
retention, construction atomicity, empty/extra/incomplete passes, point-in-time
resident expiry, unchanged cursors, retained owners, and delayed exact source
use. The full validation inventory is 1,593 core tests, 173 format tests, 19
application tests, three symbol tests, and two compile-fail doctests, plus
formatting, diff checks, and warning-denying workspace Clippy. The batch source
SHA-256 is
`C182115397C1BCB989384C817C16DAB6791C6F42EC2494B4142872556E06A3FB`.

This checkpoint adds no live adapter, manager mutation, wrapper inference from
a plan or record, arbitrary normal-arena kind-11 admission, manager capacity/
first-winner transaction, rollback, native liveness or typed-failure path, or
`SpawnStatic`/Body execution.

## Synchronous prepared model-registration manager passes

Core commit `28a5b1d` composes the exact source batch and combined manager at a
private registry-stage boundary. `SimulationDmoRegistrationRuntime` owns the
manager and can mint either a model-register phase or a model-deregister
phase. The two phase types make the wrong manager operation unavailable. Each
borrows the runtime manager and one exact batch pass, exposes only
`try_next(&SimulationRecordHandle, u64) -> Result<(), _>` and consuming
`finish`, and never returns the opaque model-registration source.

On each `try_next`, an already-faulted phase refuses first. Otherwise the
batch performs its point-in-time liveness, exact record, and full primary-ID
checks in their established order. A successful issue is immediately consumed
by the appropriate manager operation. An issuer failure is latched before
manager effects. Registration capacity failure is latched after the issuer
cursor advances but before the current source is inserted. Duplicate
registration and missing or exact-pointer-mismatched deregistration remain
successful no-ops under the previously recovered manager semantics.

Every successful prefix effect survives a later refusal. The phase performs
no rollback or drop cleanup, and subsequent callbacks plus `finish` report
`AlreadyFaulted`. On a nonfaulted phase, `finish` checks callback completeness
only and makes no resident-liveness statement. A consumed failed phase does
not poison the runtime: a fresh phase may begin normally. Typed errors and
this pass-local latch are explicit safe-host policy, not reconstructed native
failure propagation or a global manager fault.

Eight tests bring the complete validation inventory to 1,601 core tests, 173
format tests, 19 application tests, three symbol tests, and two compile-fail
doctests, plus formatting, diff checks, and warning-denying workspace Clippy.
They discriminate forward two-asset ordering, phase reuse, duplicates and
deregistration no-ops, source/full-ID precedence, latched-prefix behavior,
resident expiry, capacity timing, empty/incomplete phases, and inert drop. The
runtime source SHA-256 is
`70B48D9BE07B5BEFAE6CF05BBA540EF3AFA129EA06C8C5AE006ADA521B71C2B4`.

This is not a live trait adapter and makes no claim for the surrounding
collection/`LivingWorld` route, userdata gating, locator wiring, outer
collection begin/end, global fault behavior, rollback, `SpawnStatic`, or DMO
Body creation.

## Lifetime-free exact model-source cursor

Core commit `ef6e8b8` narrows a Rust lifetime constraint without changing the
prepared callback values or manager effects. The private
`SimulationDmoBankModelRegistrationBatchInner` owns the non-owning resident
view, exact upgraded processed-asset wrappers, and expected callback rows
behind one safe-host `Arc`. The outer batch remains non-Clone. `begin_pass`
creates an owned ordinal-zero cursor by retaining only that private inner
allocation, allowing the cursor to remain exact after the outer batch value is
dropped.

The cursor itself is non-Clone, non-Copy, and non-Debug and has no manager,
source, row, inner-owner, or ordinal getter. It therefore cannot fork a
partially consumed sequence or grant mutation authority. Each `try_issue`
still samples logical resident liveness before selecting the row and retains
the established exact-record-before-full-ID comparisons and no-advance refusal
semantics. The retained `Arc` keeps backing storage alive but does not make a
tombstoned or destroyed collection resident.

Runtime model-register and model-deregister phases now borrow only their
manager lifetime and own the source cursor. All phase-local fault, prefix,
manager-capacity, and completeness behavior from the preceding checkpoint is
unchanged. Three new tests bring the validation totals to 1,604 core tests,
173 format tests, 19 application tests, three symbol tests, and two
compile-fail doctests, plus formatting, diff checks, and warning-denying
workspace Clippy.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_model_registration_batch.rs` | `B441A8B01018C6925B228133E7F01D86B7F031D156A87F6A8B5121CA581178B0` |
| `simulation_dmo_registration_runtime.rs` | `DF743723168E49A96FA494B536520557229B444BB7D155D717E717A96AA05464` |

The `Arc` and cursor-outliving behavior are safe-host ownership mechanisms,
not native layout or lifetime facts. Native `Simulation::RegArena`
(`0x828a01e0`) and `DRegArena` (`0x828a05f0`) construct fresh local per-arena
iterators; no persistent cursor or outliving contract is evidenced. This does
not install the live normal/collection adapter or complete userdata, locator,
outer collection, `SpawnStatic`, Body, or teardown lifecycle behavior.

## Detached runtime-domain model controllers

Core commit `883d7fa` adds distinct detached model-register and
model-deregister controller types around the lifetime-free exact source cursor.
Each controller privately owns its cursor, a nonforgeable `Arc` domain cloned
from the creating runtime, and its local fault state. It is non-Clone,
non-Copy, and non-Debug, has no source/domain getter, and retains no manager.
The runtime continues to declare its combined manager first, followed by the
domain, preserving manager ownership and field drop order.

One manager operation requires `&mut` access to the runtime and the matching
nominal controller. Exact `Arc` identity is tested first. A foreign or stale
controller therefore returns a keyless/source-free `RuntimeMismatch` without
checking its fault, sampling residence, advancing its source cursor, applying a
manager operation, or latching. The controller's retained domain also cannot
match a replacement runtime after the original is dropped. Multiple dormant
controllers may coexist under an explicit safe-host preparation policy; their
effects remain serialized by the mutable runtime borrow.

For a matching domain, the previously frozen source and capacity ordering is
unchanged: source or manager refusal latches only that controller, already
applied prefix operations survive, and drop performs no rollback or cleanup.
The `must_use` borrowed register/deregister pass wrappers now hold these
controllers and call the same runtime effect methods, preserving their public
shape and providing one implementation path.

Six tests bring the complete inventory to 1,610 core tests, 173 format tests,
19 application tests, three symbol tests, and two compile-fail doctests. Source
SHA-256 is:

| Source | SHA-256 |
|---|---|
| `simulation_dmo_registration_runtime.rs` | `483244148606DC2A6576D45572053B19BD27981DB8CE9C4AFAD61DF21753A3A1` |

The detached controller, private domain, multiple-dormant-controller policy,
and associated lifetime/concurrency properties are safe-host Rust design. No
native controller, domain, concurrency, or lifetime contract is claimed. The
outer coordinator is not wired by this checkpoint.

## Controlled normal-arena DMO-model tail

Core commit `c470889` installs a crate-private safe-host substitution point for
the second kind-11 callback in the normal-arena adapter. It does not alter the
public dispatch trait. The legacy and controlled entry points share one
registration loop and one deregistration loop, so non-DMO ordering is unchanged.
On the controlled path the existing Add/Remove callback still runs first, but
the exact runtime controller replaces the legacy DMO-registry callback; it is
never an additional third call.

The debug-XEX PPC sites in the frozen model-registry packet establish:

- registration userdata-bit gate: `0x828a02e4`, `0x828a02e8`,
  `0x828a02ec`;
- registration ID/manager setup through direct Add call:
  `0x828a02f4..0x828a0304`, with the call instruction at `0x828a0304`;
- following DMO-manager virtual dispatch: `0x828a0324`;
- deregistration userdata-bit gate: `0x828a06f4`, `0x828a06f8`,
  `0x828a06fc`;
- deregistration ID/manager setup through direct Remove call:
  `0x828a0704..0x828a0714`, with the call instruction at `0x828a0714`;
- following DMO-manager virtual dispatch: `0x828a0734`.

Thus bit 3 clear skips both operations, while bit 3 set orders Add/Remove before
the DMO-manager effect. The native calls are `void`. The Rust typed controller
errors, exact error wrappers, fault latch, token policy, and no-rollback
behavior are safe-host controls, not inferred native branches. If the
substituted second operation refuses, its already-applied first effect and
earlier prefix remain, the suffix does not run, and the adapter records
`DmoControllerRefusal` at `DmoRegistry`. A controller-level `RuntimeMismatch`
itself remains non-latching, although the adapter latches because the first
effect was reached.

The evidence source remains
`world_probe_temp\dmo_model_registry_raw.txt`, SHA-256
`4718F3FC499112AD38D4387037B553573FE54BAFE258F764200571E3EF4CE28F`.
The validated source hashes for this checkpoint are:

| Source | SHA-256 |
|---|---|
| `simulation_normal_arena_adapter.rs` | `372596BC331409C3CD5C96E5FF0EAA452CBF8EA10D4CB8157F4A0F3B19819506` |
| `simulation_dmo_registration_runtime.rs` | `C9D0974628C0BB63184C6E44C2CB4EE7E53853FA39904CA860AA02EEB04DB97B` |
| `simulation_dmo_assembly_part_session.rs` | `63BC0DFA9E6CF0BDC886C63578D030C17B405382A762463003F86468E73674F9` |

This does not claim complete `RegArena`/`DRegArena` collection wiring, the
outer active/deferred policy, locator or `RegDMOArena`/`DRegDMOArena`
coordination, `SpawnStatic`, or Body construction.
