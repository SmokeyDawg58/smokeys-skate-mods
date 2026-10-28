# Simulation resident collection and Arena registry

## Scope and native boundary

`simulation_resident_registry` supplies a nonowning host publication domain for
later full-`u64` Collection and Arena lookups. Native collection receivers call
`cAssetRegistry::GetRegAsset` at `0x8277ea08` and `0x8277ec38`. The deferred
individual-Arena receiver loads the full ID from message `+0x10` and performs a
fresh container-registry lookup at `0x8277e7fc`; it then reads `cAsset::mAssetData`
at `+0x0c`, calls `Simulation::RegArena(arena, userdata)` at `0x8277e844`, and
calls `cLocationManager::RegArena` at `0x8277e884`. The registry read lock has
already been released before either manager call.

The native map mutation rules are exact. `AddRegAsset`
(`0x82d64890..0x82d64994`) takes the configured write lock, reads the full
64-bit ID, and inserts `(ID, cAsset*)` only when the ID is absent. It does not
replace or assert on a duplicate. `RemRegAsset`
(`0x82d63dd0..0x82d63ed0`) treats null as a no-op and erases only when the
current row contains the exact supplied pointer. Removing a winner never
promotes a previously losing duplicate.

Collection wrappers and outer Arena assets use the same native
`cRegisteredAssetContainer`, not separate per-class maps. Their full IDs
therefore compete for one row: the first inserted Collection or Arena pointer
wins even when the losing candidate belongs to the other class. This evidence
does not mean that the current host models the whole native container registry;
other registered classes, including processed inner assets, remain outside its
scope.

Primary evidence inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- receiver packet SHA-256
  `7EE4B5F7F16FF866EFCBD9AD16D0237AE6E0C40C81A0502967F6B76001A0E4E4`;
- registry/lifetime packet SHA-256
  `D59D85040D3C03F6086628AD1C710E8011AB78AD16DA3ADB258E066CE5D5C6A0`.

## Safe-host ownership and occupancy semantics

The host's shared Collection/Arena container domain retains the existing
monotonic collection occurrence token. Pending
rows are invisible until exact publication, and stale occurrence/removal
tokens cannot bind to a same-ID reload. Each pending collection row also
captures its assets at wrapper creation in authored order as
`(full ID, Weak<A>)` identities. Appending assets after that boundary is an
invalid host composition and does not extend the pending snapshot. On
publication, Arena rows are inserted in that order only when their full ID is
absent. On exact collection removal, an Arena row is erased only when both its
ID and `Weak::ptr_eq` allocation identity match that collection's candidate.
Losing duplicates stay unregistered and are never promoted.

The host records the winning row's class. A Collection lookup that encounters
an Arena winner, or an Arena lookup that encounters a Collection winner,
returns `PublishedKindMismatch`. That is a typed safe-host stop before an
invalid cast or dereference, not native recovery and not a claim that native
rows carry this Rust discriminator.

Registry rows, wrappers, lookup handles, and occurrences add no persistent
owner of the collection or Arena. A scoped Arena lookup upgrades only the
selected winner while holding the registry mutex, releases that mutex, runs
the synchronous visitor, and drops the temporary owner afterward. This is the
scoped safe-host lifetime hold around native's raw-pointer use after
`GetRegAsset` unlocks; it does not replace the separate native loaded-count
exclusion or delayed-GC lifecycle. An expired published weak row is an
invariant fault; lookup does not silently delete it or promote a loser.

With the current load interfaces, all Arena candidates become visible as one
safe atomic batch when the completed collection wrapper is published. Native
`cProcessArenaAsset::ProcessArena` instead publishes inner assets during its
forward authored-TOC walk and publishes the outer Arena wrapper at
`0x82d6048c`, before the later collection-wrapper publication and callback.
Likewise, current host Arena-row removal is coupled to exact collection-row
removal; native collection-zero teardown queues each child for delayed asset
GC, and `UnprocessArena` removes the outer wrapper only when that child's
asset-level count later reaches zero. The recovered first-insert and
exact-pointer removal rules are represented across the shared Collection/Arena
domain; the Rust class discriminator, atomic batch, mid-load visibility, and
delayed removal timing remain explicit host boundaries.

Allocation, poisoning, token exhaustion, stale identity, and expired-row
failures are typed fail-stop host behavior. Native missing Arena asset or null
data converges on the assertion at `simthreadmsgs.cpp:39`, then would continue
unsafely if execution resumed after `DebugBreak`; no Rust error is described as
native recovery. Drop performs no hidden publication, removal, callback, or
teardown effect.

## Loaded-count lifetime

Deferred collection registration acquires one collection-level hold in
`NotifyCollectionSimReg` before the defer decision. Neither
`SetupNextActiveCollectionReg` nor each deferred Arena message increments an
asset count. DReg does not release the hold. The matching decrement occurs in
`cCollectionUnloadedCallback::OnComplete` only after actual stream unload
completion. This collection hold protects the native nonowning `cAsset*` list
through receiver-time Arena lookup.

Standalone `SendSimRegArenaMsg` and isolated `cSimAssetLoader::AssetAct` do use
asset-level `UpLoadedCount`, but they are separate paths and are not evidence
for per-Arena increments in a deferred collection batch. At collection count
zero, child IDs are queued in insertion order by `GarbageCollectAsset`.
Eligibility uses `GarbageReleaseTick + 3` and a strict `scheduled < current`
test; cross-thread scheduling means this is not claimed to be exactly three
frames. Asset-level zero invokes `UnprocessArena`, registry removal, and later
deferred raw-resource release.

## Verification and open work

Eighteen focused registry tests cover collection occurrence behavior plus
Arena invisibility before publication, full-width IDs, first-wins duplicates
within and across collections, shared cross-class Collection/Arena occupancy,
typed wrong-class lookup, exact-pointer removal, no loser promotion,
delayed-old-removal ABA safety, lock release before visitor code, scoped
lifetime retention, and expired-row fail-stop behavior. Router tests add
FIFO-front-only lookup, missing winner retention, cross-collection current
winner resolution, and pre-effect plan-mismatch handling.

The opt-in real-BAM gate now performs 1,392 successful receiver-time Arena
lookups across all 79 deferred-eligible collections and matches the independent
V2 oracle. The dense corpus has no duplicate outer IDs, so duplicate and ABA
semantics remain synthetic evidence-backed tests. Native null resident slots,
mid-load Arena publication, delayed per-asset GC timing, native callback
invocation, and production acknowledgement remain open. See
[`simulation-resident-notification-corpus.md`](simulation-resident-notification-corpus.md).
