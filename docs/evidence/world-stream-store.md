# Global world-stream publication store

This record covers the concrete ownership boundary for the global
`cAssetStreamSystem` collection-ID map and three path-sorted stream-file
vectors. It reconstructs valid-domain publication, lookup, lifetime, and
unpublication semantics without reproducing native dangling-pointer behavior.
The concrete `world_stream_runtime` publisher now connects this store to
`WorldStreamAdapter`.

## Evidence anchors and layouts

| Address | Function |
|---:|---|
| `0x824e1ad0` | `cAssetStreamSystem::UpdateStreamFileLoaded` |
| `0x824dc098` | `cAssetStreamSystem::RemoveStreamFile` |
| `0x824e1ce0` | `cAssetStreamSystem::UpdateStreamFileUnloaded` |
| `0x824d7838` | `cStreamFile::Unload` |
| `0x824e0ba8` | `cStreamFile::Update` |
| `0x824d9c70` | `cStreamFile` scalar deleting destructor |
| `0x824dae58` | `cAssetStreamSystem::GetOctreeInfo` |
| `0x824dfea8` | `cAssetStreamSystem::FindCollections` |

PDB `cAssetStreamSystem` is `0x11950` bytes. Its three 16-byte
`vector<cStreamFile*>` values begin at `+0x28`; the fixed
`cAssetID -> tOctreeInfo*` hash map is at `+0xb8`; the map Futex is
`+0x117d0`; and the vector RW lock is `+0x117f0`. Stream kinds are exactly
Presentation 0, Simulation 1, and Texture 2. The fixed map owns 2,048 nodes
with overflow disabled; the allocator/assert outcome for a 2,049th distinct
ID remains unknown.

PDB `cStreamFile` is `0x290` bytes. Relevant fields are state `+0x28`, kind
`+0x40`, local Octree `+0x48`, path `+0x5c`, compact accepted count `+0x25c`,
processed-record vector `+0x260`, and aggregate bbox `+0x270`. Global map
values point directly to `0x80`-byte `tOctreeInfo` elements in that processed
vector, not to the file-local Octree.

## Publication

`UpdateStreamFileLoaded` first updates the file and proceeds only for exact raw
state 4. It then takes the map Futex and attempts unique insertion for every
accepted compact record in order. Equal IDs leave the existing pointer in
place and the insertion result is ignored, so the first record wins. After
releasing the map lock it takes the per-kind vector write lock and inserts the
file before the first strictly greater unsigned first-NUL C path. Equal paths
are skipped, giving stable insertion after all existing equals.

Consequently duplicate IDs inside one file can survive processing, colliding
files are both published, and neither duplicate IDs nor already-mapped IDs
consume another fixed-map node. Normal sequential processing filters IDs that
an earlier file has already published, but the global containers themselves do
not enforce that invariant. Repeated native publication of a raw-state-4 file
can append its pointer again.

The Rust store uses the canonical `StreamKind`, `NativeCPath`, and comparison
function. It publishes the accepted ID prefix and file identity atomically
under one lock, caps only distinct winning IDs at 2,048, retains the original
compact index, and returns a weak publication token containing only IDs that
actually won. Both colliding file identities remain visible and a loser is
never promoted if the winner later unloads.

## Unpublication and lifetime

Native unload first waits in file state 5. Once the asset manager reports the
stream unloaded, `cStreamFile::Update` removes local compact entries, frees the
local Octree and XML map, and only then writes state 6. The outer unloader then
removes every record ID under the map lock, removes the first pointer-equal file
while preserving vector order, and finally destroys the processed vector and
file object.

The reference assumes collision-free ownership here. It unconditionally erases
the result of each map lookup and never verifies the mapping belongs to the
file. Same-file duplicates, a collision loser, a missing mapping, or duplicate
file pointers can therefore erase another file's record, dereference an invalid
iterator, or leave a dangling vector entry.

The safe store transfers ownership of one local-spatial value and tracks it as
`Retained -> Releasing -> Released`. Its destructor runs without registry locks,
allowing reentrant observation, but unpublication cannot succeed until actual
store-owned destruction completes. A panic leaves the entry non-unpublishable.
Dropping the last store releases every retained local value before clearing the
global map and file vectors; tokens hold only weak store/entry identities and
cannot invert that order.

The concrete local `Octree::Remove` primitive and its exact ascending compact
caller contract are now reconstructed in `octree.md`. A future store-owned
wrapper can therefore execute the accepted-count removal loop before dropping
the Octree. This store deliberately does not perform that loop itself. The
adapter now transfers its unique local value into publication, retains the
returned token through raw state 5, requires synchronous local release in the
final inner callback, and requests global unpublication only after raw state 6.
`world_stream_runtime.md` records the concrete publisher that composes those
two existing implementations and its checked release-report policy.

Unpublication validates the exact file entry, every token-owned ID's owner, and
record-storage identity before mutation. It then erases IDs in compact winning
order and the exact file identity under one lock. Foreign, repeated, missing,
duplicate, or owner-mismatched operations are typed errors with no partial
global mutation. Supplying an intrinsically shared local value such as `Arc<T>`
releases only the ownership transferred to the store and does not claim that
external aliases were destroyed.

## Concurrency and confidence

The native map and vector locks are separate: publication exposes map entries
before the file vector, and removal can hide map entries before taking the
vector write lock. `GetOctreeInfo` returns a raw record pointer after releasing
the map lock, while `FindCollections` holds the vector read lock as it consumes
files and takes nested map locks. Higher-level serialized scheduling supplies
the native lifetime invariant.

Rust deliberately uses one lock and owning `Arc` snapshots so readers never see
a map-only or vector-only state and cannot retain dangling identities. Atomic
rollback, owner checks, weak tokens, transferred local ownership, and typed
errors are **high-confidence safe deviations** for native-invalid states.

Confidence is **exact** for the symbols, layouts, state gate, compact iteration,
first-ID-wins rule, 2,048-node capacity, unsigned C-path ordering, stable equals,
per-kind vectors, native lock/call order, local-before-global unload order, and
processed-record lifetime. Native allocator exhaustion, invalid concurrent
scheduling, and the concrete AssMan heap implementation remain **unknown**.
The typed call at `0x824e1440` is now proven to be the one-instruction empty
`rw::collision::Octree::Release` at `0x825187c0`, so it requires no additional
collaborator between local removal and storage destruction.

Twenty-four focused tests cover zero and more-than-16 files, path edge cases,
within/across-file and cross-kind collisions, full-capacity duplicate insertion,
no promotion, token and invariant failures, coherent owning snapshots,
concurrent publication/unpublication, local release races, reentrant and
panicking destructors, external shared ownership, and last-store drop order.
The initial candidate was rejected for an `Arc` lifetime race and duplicate
canonical types; the corrected frozen source passed independent review.
