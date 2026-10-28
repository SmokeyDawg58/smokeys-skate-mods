# Resident collection streamer

This note bounds the typed resident-collection scheduler in
[`collection_resident_streamer.rs`](../../crates/skate2-core/src/streaming/collection_resident_streamer.rs).
It reconstructs the one-active FIFO, resident reference counts, collection
wrapper publication, and zero-count destruction. Parsing and decompression are
owned by the separate collection-load pump; metadata, constructed assets,
registry storage, and parent relationships remain collaborators.

## Native anchors

| Symbol | Reference virtual address | Confidence |
|---|---:|---|
| `assman::cAssetStreamer::UnloadCollection` | `0x82d549c0` | exact |
| `assman::cAssetStreamer::GetLoadedSize` | `0x82d54ac0` | exact |
| loaded-map `find` specialization | `0x82d56990` | exact |
| `assman::cAssetStreamer::LoadCollection` | `0x82d57c38` | exact |
| `assman::cAssetStreamer::UpCollectionLoadedCount` | `0x82d57d20` | exact |
| `assman::cAssetStreamer::DownCollectionLoadedCount` | `0x82d57de0` | exact |
| `assman::cAssetStreamManager::LoadCollection` | `0x82d57f58` | exact |
| `assman::cAssetStreamManager::UnloadCollection` | `0x82d58018` | exact |
| `assman::cParentCollectionUnloadedCallback::OnComplete` | `0x82d581a0` | exact |
| `assman::cAssetStreamer::PendingRequests` | `0x82d55d90` | exact |
| `assman::cAssetStreamer::RemCollection` | `0x82d59520` | exact |
| `assman::cAssetStreamer::Update` | `0x82d60ec0..0x82d61318` | exact |

Addresses and names come from the supplied linker map/PDB. Control flow and
loads/stores were checked against the matching XEX instructions.

## PDB layout and identity

The represented `cAssetStreamer` fields are:

| Offset | PDB member |
|---:|---|
| `0x00008` | `mAssetManager` |
| `0x06130` | `mCollectionMap` |
| `0x501f4` | `mCollectionRequestList` |
| `0x501fc` | `mLoadedCollectionMap` |
| `0x55250` | `mCollectionRWMutex` |
| `0x55350` | `mCollectionRequestListRWMutex` |
| `0x55480` | `mAssetCollectionRequestMutex` |
| `0x554b0` | `mActiveCollectionRequest` |
| `0x554b4` | `mStream` |
| `0x554bc` | `mHighestStreamBlockSize` |

`tLoadedCollection` is 12 bytes: `cCollection*` at `+0`, `cAsset*` at `+4`,
and `uint32_t mRefCount` at `+8`. `tCollectionRequest` is 48 bytes, with its
raw type at `+0x08`, full `cAssetID` at `+0x18`, completion byte at `+0x20`,
callback at `+0x24`, and allocator at `+0x28`.

The resident container is exactly
`fixed_hash_map<cAssetID,tLoadedCollection,512,512,no-overflow>`. `cAssetID`
contains one `uint64_t`. The specialization at `0x82d56990` hashes by wrapping
addition of the low and high 32-bit halves, then compares chain keys with
`cmpld`; equality therefore uses all 64 bits. The Rust table preserves the
512-distinct-ID capacity and full-width equality. Its linear host lookup does
not claim to reproduce unobservable bucket storage.

Executable enqueue code stores raw request type zero for load and one for
unload. An exported PDB enum assigns a different numeric value to a named
unload member, so the Rust semantic enum is based on the executable stores,
not that conflicting enum label. The recovered scheduler branches as load for
zero and unload for nonzero; this module only constructs the proven values 0
and 1.

## One-active update

Load and unload enqueue append at the intrusive-list tail under its write
lock. The idle path at `0x82d61090` detaches the front and records it as active.
An empty queue returns without kicking decompression.

For a load whose full ID is already resident, the native code wrapping-
increments the `uint32_t` count, unlocks, invokes `OnComplete(id)`, deletes and
clears the request, and immediately continues the FIFO loop. For an unload it
invokes `OnComplete(id)` first, calls `DownCollectionLoadedCount`, deletes and
clears the request, and also continues. Therefore any number of cache hits and
unloads can drain in one update.

The first physical miss performs this order:

1. create/reserve the empty `cCollection`;
2. find required collection metadata;
3. insert `{collection, null wrapper, refcount 1}` into the resident map;
4. initialize/start the load request against that same collection;
5. leave it active and return without examining the next FIFO entry.

The Rust resident table uniquely owns a boxed collection. Only after insertion
does storage derive the non-owning writer passed to the load factory. This is
the explicit identity seam by which `CollectionLoadPump::add_asset` can update
the same resident collection later wrapped and published; no second collection
owner is created.

An active physical request is updated before its completion flag is sampled.
If incomplete, `StartDecompress` is called and the update returns. If complete:

1. replace the global high-water value only for unsigned strict greater;
2. create a collection-asset wrapper from the stable resident collection;
3. install its owner/token in the resident entry;
4. release native collection synchronization, then add the wrapper to the
   asset registry;
5. invoke `OnComplete(id)`;
6. delete/clear the active request;
7. call `StartDecompress` and return.

It never pops another request on the physical-completion update. The native
code uses its collection lock's read mode while storing the wrapper pointer.
Rust object ownership provides exclusive mutation instead; this is a **high**
safe synchronization correction while the observable sequence remains exact.

A zero-asset load leaves the inner request ID exactly `-1` and queues no file.
The outer tick that starts it still returns. Its driver completes on the next
tick; only the following tick may start another queued request. The driver,
not this module, owns that raw pump behavior.

Callbacks receive an opaque context with enqueue-only FIFO operations and an
active-collection `UpCollectionLoadedCount` operation, but no resident lock,
arbitrary resident access, decrement, queue pop, or collection ownership
handle. The active-only increment is the complete capability required by the
recovered Simulation and Presentation parent-registration callbacks. A second
host API may pair that same increment with a non-cloneable
`ResidentCollectionHold`, bound to the streamer domain, full ID, and a
nonreused resident generation. Its explicit release delegates to the exact
underlying `DownCollectionLoadedCount`/zero-teardown path while rejecting a
foreign streamer or later same-ID reload without mutation. A pre-existing
streamer fault returns the token untouched; a fault entered after the valid
decrement consumes it so the decrement cannot be retried. Dropping the token
is inert. This occurrence authority and generation-exhaustion error are
**high** host safety, not native collection fields.

Tests demonstrate reentrant enqueue while the current request remains active,
exact wrapping increment, typed failure when no active resident exists, and
retained mutation when a later callback failure is latched. The observation
passed to the callback records exact invocation-point state: cache-hit count
has already incremented, unload count has not yet decremented, and physical
completion has already published its wrapper. The opaque Rust context and
typed invalid-domain errors are **high** safety architecture; the count
mutations and callback ordering are exact. Delayed asynchronous integrations
must retain and release the occurrence-bound hold rather than decrement by ID
alone.

`PendingRequests` is true when either the FIFO is nonempty or a request is
active. `GetLoadedSize` returns the physical load driver's live value only for
the exact active full-width ID; otherwise it returns zero.

## Wrapping references and destruction

Both refcount directions are wrapping `uint32_t`. Incrementing `0xffffffff`
stores zero without destruction. Decrementing zero stores `0xffffffff` and
retains the entry. Destruction occurs only when the stored decrement result is
exactly zero.

The exact zero-count order at `0x82d57de0` is:

1. capture the resident collection and wrapper;
2. erase the resident-map entry under the collection lock;
3. unlock;
4. call asset-manager garbage collection for every collection member in
   insertion order, including repeated IDs;
5. find/remove the exact registered wrapper identity;
6. destroy the saved wrapper;
7. free the collection's vector storage and collection.

The Rust registry token ties publication and removal to the same stable boxed
wrapper. The wrapper must refer to the collection non-owningly, and its token
is a side-effect-free non-owning identity; neither can create another
collection or wrapper owner. Wrapper installation and successful registry
publication are distinct state. The published observation becomes true only
after `publish` succeeds, so a failed publication truthfully retains an
installed but unpublished wrapper.

Collection member IDs remain full-width and are not deduplicated. Collaborator
calls happen after table erasure. On a result-bearing teardown failure, the
erased owner plus occurrence list and failure index are retained behind the
permanent fault instead of pretending later native effects ran.

## Parent and metadata boundary

Manager load at `0x82d57f58` recursively enqueues each parent before its child.
Manager unload at `0x82d58018` enqueues the child before walking its parent
chain. Both paths use full-width compares against the runtime-initialized
`kInvalidCollectionID` value `0xe171e1abfedb69ed`; zero is an ordinary ID, not
the chain terminator.

On the first parent completion, `OnLoadParentComplete` at `0x82d5c978` inserts
manager use count one. `NotifyCollectionSimReg` at `0x82d5c468` increments the
same active resident before sending Simulation registration, then
`NotifyCollectionPresReg` at `0x82d5c680` increments it again before sending
Presentation registration. An existing manager entry receives only a wrapping
use-count increment.

Ancestor unload requests use `cUnloadParentCollectionRequestCallback` at
`0x82d5cad0`, which appends the completed ancestor's own ID to the separate
manager FIFO before this resident streamer performs that active request's
ordinary decrement. This is distinct from
`cParentCollectionUnloadedCallback::OnComplete` at `0x82d581a0`: later
Simulation and Presentation deregistration completions each use that callback
to perform another resident decrement. Manager update at `0x82d61318` runs the
resident streamer, then Simulation deferred registration, then processes at
most one parent-FIFO entry. Parent request generation, manager use counts,
registration effects, and that separate FIFO remain outside this module; the
new callback context is only the minimum proven resident-side seam.

`RemCollection` at `0x82d59520` removes metadata and asserts if the collection
is still resident. It is not resident teardown.

## Failure and ownership boundary

- **exact:** FIFO and one-active timing, full-ID equality, 512 capacity,
  wrapping counts, callback order, miss insertion/start order, active
  completion order, unsigned high-water comparison, zero-count destruction
  order, duplicate occurrence handling, and zero-asset outer timing.
- **high:** the opaque callback capability and its typed missing-active error,
  typed capacity/missing-record errors, safe cleanup of an
  uninstalled collection on metadata failure, exclusive Rust mutation, stable
  non-owning writer/token contracts, explicit leak-on-abandonment, and
  permanent first-fault replay in place of assertions or invalid memory
  access.
- **unknown:** allocation failures, stream/read/decompression failures,
  registry failure recovery, callback exceptions, partial native rollback,
  actual collection/asset construction, worker scheduling, parent-manager
  bookkeeping, and registration/deregistration effects.

The module does not roll back an already observable resident insertion,
wrapper installation, publication, or map erasure after a collaborator error.
It retains partial ownership where possible and replays the same `Arc` error.
No behavior after such an invalid-domain failure is claimed as native.

Native shutdown/destructor behavior for a streamer with live residents was not
recovered. Implicit Rust `Drop` therefore does not synthesize unload callbacks,
garbage collection, or fallible registry removal. It deliberately abandons
(leaks) any still-resident or failed-teardown collection, wrapper, and token so
an external registry cannot retain a dangling alias. Active non-owning writers
and ordinary collaborators may then drop normally. This inert abandonment is
a **high** safety policy, not native parity; normal request-driven zero-count
teardown remains the only recovered cleanup path.

## Validation

The standalone tests cover executable raw request values; FIFO draining and
the first-miss stop; active incomplete/completion call order; no same-tick pop
after completion; zero-asset timing; cache and unload callback observations;
callback active-Up wrapping/missing/failure retention; callback reentrant
enqueue; stable writer/wrapper collection identity;
wrapping underflow/overflow; unsigned high-water ties; full-64-bit IDs with an
equal native hash; 512-entry capacity with a cached hit at capacity; exact
active loaded-size lookup; erase-before-garbage ordering; duplicate asset
occurrences; wrapper-before-collection destruction; and permanent faults with
retained partial load/teardown ownership. Publication failure verifies that an
installed wrapper is not reported as published, while callback observations
after successful publication remain true. An explicit drop probe verifies that
implicit abandonment invokes no fabricated cleanup and drops neither resident
identity.

Until root exports the new module, its direct validation command is:

```powershell
rustc --edition=2021 --test crates/skate2-core/src/streaming/collection_resident_streamer.rs `
  -o "$env:TEMP\collection_resident_streamer_tests.exe"
& "$env:TEMP\collection_resident_streamer_tests.exe"
```
