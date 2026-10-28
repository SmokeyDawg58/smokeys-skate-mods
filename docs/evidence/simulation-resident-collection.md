# Simulation resident arena ownership

## Scope

`simulation_resident_collection` composes the already checked Simulation arena
decoder/collision projection with `SimulationNormalArenaPlan` construction. A
successful resident asset owns one immutable plan through `Arc`, exposes the
outer XST asset ID, and performs no manager or notification effect. Its session
factory delegates the existing canonical collection-snapshot and owning
`StreamKind::Simulation` checks.

This is a high-confidence safe-host ownership bridge, not a reconstruction of
the native allocation or object layout. It makes successfully constructed
dense arenas durable until later collection preparation; app-level corpus
integration remains a separate gate.

## Proven ordering and provenance

The native `cProcessArenaAsset::ProcessArena` path at `0x82d5fee0` consumes the
used-dictionary TOC forward. The recovered `Simulation::RegArena` and
`Simulation::DRegArena` dispatch packet establishes the supported action order,
IDs, and pointer-backed record direction later represented by the normal plan.
The module preserves the existing host construction sequence:

1. validate the registered collection and Simulation file owner;
2. decode the format-2 streamed arena;
3. project the currently reconstructed collision/grind view while retaining all
   decoded objects and resources;
4. build the static and normal action plans completely;
5. return one resident asset containing the immutable plan owner.

The primary evidence inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- arena-processor packet SHA-256
  `C4C11506EA241F207998C3ED78E6E68E339DCFCDE2A726C22D31CBA04B8566C2`;
- Simulation dispatch packet SHA-256
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`.

## Ownership and failure behavior

`SimulationResidentArenaAssetBuilder` preserves an inner `Ok(None)` and maps
decode/collision failure separately from normal-plan failure. No partial plan
enters resident storage. Rust `Arc` retention, typed malformed-input errors,
and destruction are deliberate host safety and make no native allocator claim.
Dropping the builder or asset sends no Reg/DReg message, invokes no callback,
and acknowledges nothing.

Five asset-construction tests cover full-width outer identity, retained arena identity and
resources, shared plan lifetime, delegated Simulation/snapshot admission, and
both collision/decode and normal-plan failures before a successful format-2
result.

## Exact-occurrence dense preparation

A `SimulationResidentOccurrence<SimulationResidentArenaAsset>` can now be
consumed into `PreparedSimulationResidentCollection`. Preparation revalidates
the exact full-ID/token occurrence, visits its resident assets once in insertion
order, fallibly retains each immutable plan owner without sorting, filtering,
or deduplicating, and only then builds the normal, Location, and DMO receiver
views. Success keeps the non-cloneable occurrence bundled with the prepared
owner. Failure returns that same occurrence alongside the exact nested visit or
collection-preparation error, so no ID-only retry is invented.

This operation is effect-free safe-host prevalidation. Native receivers walk
raw collection pointers incrementally and do not build an all-view Rust owner.
The router performs lookup and preparation at the evidenced seam: immediate
and terminal Collection work at the FIFO front, and deferred setup at caller-
invoked producer activation. Notify admission performs none of these lookups.

Four additional tests cover exact full-width collection identity, dense source
order and duplicates, empty collections, plan lifetime after resident teardown,
same-ID reload rejection with recoverable stale authority, bundle Drop without
registry mutation, and detached resident storage remaining an error rather than
an empty success.

Two prepared resident bundles can also compare their exact ordered immutable
plan-owner snapshots. Length, collection ID, authored order, duplicates, and
the underlying `Arc` pointee identities must all match. This allocation-free
comparison is safe-host stability policy for a router ledger: it detects a
post-publication append even though the registry occurrence token is unchanged.
Native code does not build or compare these snapshots.

## Open boundaries

The current owned resident collection stores only successful assets. It cannot
encode the native null slot produced by allocation failure or the exact
`PreventNavmeshLoadingHack` path, and therefore does not yet prove general
first-null termination when a later asset exists. The supplied BAM corpus is a
dense-success case, but a future resident representation must retain null slots
before claiming that behavior.

The notification router now uses resident lookup for immediate Reg, caller-
invoked deferred producer activation, the terminal activation-zero Collection
receiver, and stable supported-class DReg. Its older prepared-plan APIs remain
safe-host compatibility paths. The opt-in resident corpus now carries an exact
loaded-count hold through the deferred registration Callback dequeue position,
returns it before unload, and proves plan-owner destruction for all 1,392 dense
BAM arenas.
Individual deferred Arena receiver lookup, stateful pending-vector ownership,
null slots, stream-class transitions, native notifier callback invocation, and
acknowledgement remain separate. See
[`simulation-resident-notification-corpus.md`](simulation-resident-notification-corpus.md).
