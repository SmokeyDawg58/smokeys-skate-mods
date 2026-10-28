# Resident deferred Simulation notification corpus

## Scope

`--inventory-bam-sim-resident` is an opt-in production-composition gate for
the deferred-eligible portion of the real BAM Simulation stream. It runs only
after the unchanged `--inventory-bam-sim` V1 inventories have passed, and the
older flag does not enable this gate. The composition uses the mounted archive
backend, XSM/XST catalog, catalog-backed resident load factory, owned resident
storage, nonowning collection and Arena registry, one occurrence-bound
collection hold, real resident Arena builder, and the resident-backed
notification router.

The recovered admission inputs remain distinct: notifier type raw `1`, enabled
defer cVar, manager defer raw `1`, false `forceImmediate`, and unsigned payload
size strictly above `0x7d000`. XSM `stream_format == 2` is the Arena payload
format, not the notifier type. Selection preserves XSM order and yields 79
collections containing 1,392 dense Arenas.

## Independent V2 oracle

The formats-only V2 oracle source is outside the repository at
`world_probe_temp\arena_id_census\src\bin\resident_deferred_v2.rs`.
Its source SHA-256 is
`8B79751A0AABFFE23CF29A18AA4AF5E626AC791975A2EEBF4B9026FE6EA41C20`;
the shared manifest SHA-256 is
`DAA459C65FBABD5B94FAE67D95B1B46446376F28C2AE88BBCD46CC41E48F3C3E`.
It depends on `skate2-formats`, not `skate2-core`, parses all 284 XSF
containers, and independently checks all 2,135 flattened XSF outer IDs against
XST order. It decodes RW4 Arena payloads only for the 1,392 Arenas in the 79
deferred-selected collections.

The oracle seeds FNV-1a-64 with
`SKATE2/BAM_SIM/RESIDENT_DEFERRED_NOTIFICATION\0V2\0`. Its per-collection
sequence is:

```text
C, S, (A,K,N,L)*, F, M*, R, D, X*, (n,l)*, U
```

All integers are unsigned big-endian. `K` is a 29-byte diagnostic record:

```text
tag, collection ordinal:u32, Arena ordinal:u32,
requested outer ID:u64, resolved outer ID:u64, resolved inner RW4 ID:u32
```

`K` is not a native message layout. It proves that the runtime reached the
Arena FIFO front, performed a full-ID lookup in the shared Collection/Arena
domain, and resolved the expected real asset before Normal and Location. The
terminal `Q` preserves the original
20 counters and appends lookup attempts, hits, misses, and identity mismatches.
The frozen protocol is 277,522 bytes with signature `92164ff9ad83bb59`.

The independent lookup table is keyed by full outer `u64` ID. Its duplicate
insertion check establishes 1,392 unique outer IDs within the deferred subset;
the V2 oracle does not establish an all-2,135 uniqueness or nonzero invariant.
Inner RW4 IDs are not global keys: the deferred subset contains only 1,179
distinct inner IDs and 35 duplicate groups.

## Diagnostic lifecycle and result

For each selected collection, the gate:

1. physically loads and publishes the resident collection and Arena candidates
   into the shared Collection/Arena occupancy domain;
2. checks callback identity and exact collection reference transition `1 -> 2`
   while acquiring one occurrence-bound hold;
3. performs one gate-only inspection lookup/preparation, separate from router
   telemetry, to freeze expected IDs and weak plan owners;
4. resolves the collection at deferred Setup, freezes each producer Arena's
   full ID in its distinct FIFO node, and performs a fresh Arena lookup only
   when that node reaches the front;
5. keeps the selected safe-host Arena owner through Normal then Location,
   resolves the collection again at activation-zero Finish for DMO, and
   dequeues the registration Callback;
6. independently re-resolves at `DRegCollection`, dispatches DMO then
   Normal/Location deregistration, and dequeues its Callback;
7. returns the exact hold (`2 -> 1`), requests unload (`1 -> 0`), observes
   ordered child garbage callbacks, registry absence, and dead captured weak
   plan owners.

The release gate matches the V2 oracle exactly:

- 79 collections and 1,392 Arenas;
- 79 Setup, 79 terminal, and 79 DReg collection lookups;
- 1,392 Arena lookup attempts and 1,392 hits, with zero missing rows or
  identity mismatches;
- 237 collection preparations visiting 4,176 Arena assets;
- 1,708 outer nodes and 1,550 child-FIFO nodes;
- 1,392 calls in each of Normal Reg, Location Reg, DMO Reg, DMO DReg, Normal
  DReg, and Location DReg, with phase-specific userdata checked;
- final signature `92164ff9ad83bb59`.

Beyond the oracle protocol, the gate asserts 79 physical loads, zero cache
hits, 79 exact unloads, 1,392 ordered garbage callbacks, quiescent
streamer/router/source state, registry detachment after owner drop, and an empty
catalog after manifest unload. It additionally reports, but does not assert or
freeze, the observed 469 decompression kicks.

Router resolution counters are saturating host diagnostics, not reconstructed
native fields. The external oracle predicts only clean static identity; runtime
tests separately cover first-wins duplicates, exact-pointer removal, missing
rows, stale removal, and cross-collection winner mismatch.

## Boundaries

This remains a dense-success corpus. The owned representation cannot encode
native null slots, so it does not prove runtime allocation-null behavior.
Arena rows are batch-published with the completed host collection and removed
with its registry row; native outer Arenas publish individually during
`ProcessArena` and disappear later through child asset GC. Recovered
first-insert/exact-pointer rules and receiver lookup positions ground the host
model, while those visibility windows remain explicit host boundaries. Native
Collection and Arena objects share one container key domain. The host mirrors
that occupancy for those two classes and reports a typed
`PublishedKindMismatch` on a cross-class collision; the 79/1,392 gate processes
one collection at a time and does not exercise that collision case. Other
native registered classes, including processed inner assets, remain outside
this host registry model. The gate therefore also does not claim native
pending-vector ownership, per-tick batch interleaving, or cross-producer queue
races. Callback capabilities are dequeued and inspected, not invoked as native
function pointers; production acknowledgement and stream-class changes remain
open.
