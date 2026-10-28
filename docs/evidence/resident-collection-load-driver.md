# Resident collection load driver

## Scope

`resident_collection_load_driver.rs` composes one
`MountedCollectionProducer`, its paired `CollectionLoadPump`, an owned
decompression FIFO, and an explicit result-bearing asset builder. It implements
the existing `ResidentCollectionLoadDriver` boundary so the resident streamer
can advance the physical request without borrowing or self-reference tricks.

It does **not** construct a `WorldStreamCatalog` factory, choose a native
resource allocator, decode RW4 resources, or claim resource materialization.
Those remain separate evidence and integration boundaries.

## Native anchors

| Symbol | Reference virtual address | Recovered behavior |
|---|---:|---|
| `assman::tLoadCollectionRequest::Update` | `0x82d5fc98` | parser/pump admission and completion ring |
| `assman::cAssetStreamer::StartDecompress` | `0x82d54b68` | separate dirty observation and worker wake |
| `assman::cAssetStreamer::DecompressThread` | `0x82d5adb0` | FIFO snapshots, source slices, dispatch, callbacks |
| `assman::cAssetStreamer::AddDecompressRequest` | `0x82d5c280` | copy one 40-byte request into the FIFO |
| `assman::cAssetManager::AddAsset` | `0x82d634d8` | default raw-format constructor |
| `assman::cAssetManager::AddCompressedAsset` | `0x82d63830` | raw format 1 |
| `assman::cAssetManager::AddCompressedArenaAsset` | `0x82d63a10` | raw format 2 |
| `assman::cAssetManager::AddCompressedChunkArenaAsset` | `0x82d63bf0` | raw format 3 |

The exact worker proof, PDB request layout, instruction ranges, and concurrent
dirty-word caveat are recorded in
[`collection-load-pump.md`](collection-load-pump.md#decompression-worker-queue).

## Ownership

The pump submission is borrowed only for the duration of `submit`. The owned
FIFO synchronously copies its scalar fields, clones the typed asset-info
snapshot, and clones a `MountedCollectionBlockView`. That view owns immutable
Arc-backed bytes and source provenance but has no stream origin or release
method. The non-Clone `MountedCollectionChunk` remains uniquely retained in the
pump ring until the matching FIFO callback calls `complete_slot`.

This owned representation is a **high** safe-PC policy. Native requests retain
asset-info, source, allocator, callback, and userdata pointers instead.

## Deterministic update policy

One driver update performs:

1. drain a wake requested after the preceding update;
2. poll exactly one producer stage; and
3. run exactly one collection-pump update.

One drained wake repeatedly removes FIFO-front snapshots of at most 32 until
the queue is empty. Within each snapshot, the first job in a source slice
always runs. Before each later job, the wrapping unsigned sum of previously
completed source sizes is compared with `0x40000`; equality or overshoot starts
a new slice. Each job calls the explicit builder with the exact raw format and
its mapped constructor family, then completes the matching FIFO pump slot.

Snapshot order, source-budget arithmetic, raw-format mapping, callback order,
and FIFO completion are **exact**. Serial execution, `AtomicBool` concurrent
coalescing, and omission of native `ThreadSleep(1)` calls are **high** host
policies. They do not claim native thread interleavings or sleep timing.

`Ok(None)` from the builder represents the native callback receiving a null
asset: the driver releases and advances the pump slot without adding an asset
or loaded-byte count. A typed builder failure retains the failed job, the rest
of its snapshot, the untouched FIFO tail, and the pump's unique chunk. It
permanently replays the first top-level error by Arc identity. Native
constructor failure semantics are unknown, so this retention is a **high**
safe policy rather than reconstructed recovery.

Exactly zero target assets are rejected before producer construction. A future
factory must route them to a separate driver preserving the resident streamer's
established deferred-zero timing. Negative signed targets remain valid physical
requests and preserve the pump's signed `parsed >= target` behavior.

## Validation

Fourteen focused tests cover owned submission lifetime, lack of duplicate
release authority, no construction before a wake, concurrent host coalescing,
32-plus-1 snapshot draining, source-budget below/equal/overshoot boundaries,
all raw format branches, FIFO builder/add order, null completion, payload-range
failure, failed-job restoration, zero and negative targets, resident-driver
trait accounting, and sticky producer/pump/builder errors.

The prior real-data transport gate drove all 3,281 Rob's Park records through
the producer and pump with null callbacks. Running this new driver against real
resource constructors remains a required later gate; this module alone does
not upgrade that result into asset materialization or freeroam readiness.
