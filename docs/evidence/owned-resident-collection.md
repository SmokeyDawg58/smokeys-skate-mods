# Owned resident collection bridge

## Scope

`owned_resident_collection.rs` supplies the first concrete
`ResidentCollectionStorage` implementation. It keeps one private strong owner,
an ordered asset list, and a non-owning writer that can move through the load
driver. It is a safe interim home for `DecodedArenaAsset`; it is not the final
native asset-manager representation.

The reference `cCollection` contains a `cAssetList`, whose vector stores
non-owning `assman::cAsset*` entries. The current Rust bridge owns decoded
values because the per-stream registration and stable registered-asset handle
do not exist yet. This ownership difference is explicit and **high**, not
exact.

## Native evidence

| Symbol | Address | Recovered behavior |
|---|---:|---|
| `assman::cAssetList::AddAsset` | `0x82d5e170` | rejects null, appends one pointer at the vector end, grows if full; no sort or deduplication |
| `assman::tLoadCollectionRequest::EntryComplete` | `0x82d5e4a8` | adds stored size, appends a non-null completed asset, releases the block, then advances the FIFO tail |
| `assman::cAssetStreamer::Update` | `0x82d60ec0` | allocates a 16-byte collection, initializes/reserves its vector from the raw asset count, inserts it in the resident table, then starts loading |
| `assman::cAssetStreamer::DownCollectionLoadedCount` | `0x82d57de0` | erases at zero, invokes asset-manager cleanup for each list occurrence in insertion order, removes/destroys the wrapper, then frees vector storage and collection |

PDB types establish `cCollection` and `cAssetList` as 16 bytes each,
`assman::cAsset` as 16 bytes with its full `u64` ID at offset zero, and
`tLoadedCollection` as 12 bytes containing collection pointer, wrapper pointer,
and `u32` reference count.

## Host ownership

`OwnedResidentCollection<A>` has one private `Arc` and deliberately is not
`Clone`. `OwnedResidentCollectionWriter<A>` contains only a `Weak` and also is
not cloneable. `OwnedResidentCollectionView<A>` supplies an equally non-owning,
non-cloneable opaque identity for a future registry wrapper, with only
attachment and weak-pointer equality observations. Consequently neither
writer nor view can keep the collection alive or create a second persistent
owner. Every entry caches the full `CollectionId` beside its owned value;
snapshots preserve high ID words, duplicates, and append order.

A mutex plus an `alive` tombstone linearizes writer/destruction overlap. An
append either commits while the collection is alive and is included in
destruction, or observes the tombstone and fails. This is **high** PC safety
architecture; the reference append has no local lock and no owned-value
tombstone. `destroy` takes and drops values only after setting the tombstone,
and does not duplicate garbage collection or registry removal already owned by
the outer streamer.

The current `LoadedAssetCollection::add_asset` contract is infallible. A stale,
destroyed, or poisoned writer therefore panics as an explicit invalid-domain
failure rather than silently losing an asset. Making those failures fully
result-bearing requires widening the pump and driver interfaces. Similarly,
`Arc::new` and a later `Vec::push` can abort on allocation failure. Initial and
snapshot vector reservations use fallible allocation and map into the caller's
unified error type.

Negative reserves are rejected before conversion to `usize`. This is a
**high** bounded-host policy for malformed input; it does not claim the unsafe
native signed-to-allocation behavior. Valid Skate 2 collection counts are
nonnegative.

## Registration boundary

`DecodedArenaAsset` supplies its outer XST `u64` through
`ResidentAssetIdentity`. The retained value still contains all five decoded
resource buffers and the RW4 dictionary. It has not been registered with
Simulation, Presentation, Texture, a TOC asset map, world painter, or spatial
map. The intended later replacement is a stable non-owning registered-asset
handle implementing the same identity trait; this is future architecture, not
recovered behavior.

## Validation

Eleven focused tests cover negative and zero reserves; one persistent strong
owner; weak-writer lifetime; full-width IDs; duplicate insertion order;
owned-value retention and exactly-once destruction; stale-writer and
tombstoned-writer failure; serialized append/snapshot behavior; typed mutex
poisoning with cleanup recovery; expiring non-owning registry identity; and the
`DecodedArenaAsset` identity trait boundary. Strict formatting and warning-
denying Clippy pass.

This module has not yet been composed into the application resident streamer
or driven through the 3,281-record Rob's Park corpus. Such a future gate proves
composition and lifetime behavior only; native asset ownership remains open
until registration returns stable manager-owned handles.
