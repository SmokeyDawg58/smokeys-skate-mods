# Collection load request parser and pump

This note bounds the reconstructed outer control flow for
`assman::tLoadCollectionRequest`. The implementation is
[`collection_load_pump.rs`](../../crates/skate2-core/src/streaming/collection_load_pump.rs).
It deliberately does not implement file I/O, decompression, asset
materialization, the resident-collection map, or the streamer worker loop.

## Native anchors

| Symbol | Reference virtual address | Confidence |
|---|---:|---|
| `assman::cAssetStreamer::LoadCollection` | `0x82d57c38` | exact |
| `assman::tLoadCollectionRequest::tLoadCollectionRequest` | `0x82d55e20` | exact |
| `assman::tLoadCollectionRequest::Load` | `0x82d54bd0` | exact |
| `assman::tLoadCollectionRequest::ParseRawFunction` | `0x82d546b0` | exact |
| `assman::tLoadCollectionRequest::Update` | `0x82d5fc98` | exact |
| `assman::tLoadCollectionRequest::EntryComplete` | `0x82d5e4a8` | exact |
| `assman::tLoadCollectionRequest::DecompressCallback` | `0x82d5e558` | exact |
| `assman::cAssetStreamer::AddDecompressRequest` | `0x82d5c280` | exact |
| `assman::cAssetStreamer::StartDecompress` | `0x82d54b68` | exact |
| `assman::cAssetStreamer::DecompressThread` | `0x82d5adb0` | exact |
| `assman::cAssetStreamer::Update` | `0x82d60ec0` | exact |
| `assman::cAssetStreamer::DownCollectionLoadedCount` | `0x82d57de0` | exact |

The asset-stream manager reaches the streamer update at `0x82d61318`. The
symbols and addresses are from the supplied linker map/PDB and the control
flow below was checked against the matching XEX instructions.

## PDB layouts used by the slice

`tLoadCollectionRequest` is exactly `0xc0` bytes in the PDB. Fields required
here are:

| Offset | Field represented here | Native type/width |
|---:|---|---|
| `0x30` | stream collaborator | pointer, 4 bytes |
| `0x34` | filename (outside this slice) | pointer, 4 bytes |
| `0x38` | file offset (outside this slice) | `uint64_t` |
| `0x40` | stream format | enum/raw `int32_t` |
| `0x44` | active request ID | `int32_t` |
| `0x48` | target asset count | `int32_t` |
| `0x4c` | parsed asset count | `int32_t` |
| `0x50` | next-record flag | `bool` |
| `0x54` | loaded collection collaborator | pointer, 4 bytes |
| `0x58` | queue allocation | pointer, 4 bytes |
| `0x5c` | raw tail counter | `uint32_t` |
| `0x60` | raw head counter | `uint32_t` |
| `0x64` | highest stored block | `uint32_t` |
| `0xb8` | pending parser stride | `uint32_t` |
| `0xbc` | loaded stored bytes | atomic `uint32_t` |

The inherited/base region is `0x00..0x2f` and is outside this slice.

PDB `tQueueEntry` is 12 bytes: retained chunk pointer at `+0`, request pointer
at `+4`, and stored byte count at `+8`. PDB `tCollectionAsset` is 24 bytes:
asset ID (`uint64_t`) at `+0`, stored size at `+8`, data offset at `+0xc`, and
record stride at `+0x10`. The remaining serialized content is opaque here.

## Raw parser

The following ordering is exact for `ParseRawFunction`:

1. `datasize <= 24` returns the more-data result without dereferencing a
   record.
2. When the next-record flag is set, the stride is read from record `+0x10`,
   the flag is cleared, and the stride is saved at request `+0xb8`.
3. Native asserts unless the stride is in `1..=0x0028_0000`. This validation
   also applies to the final record.
4. `datasize < stride` returns more-data. The latched stride remains in use;
   another supplied header is not read.
5. The output block size becomes the stride, the signed parsed count increments
   with 32-bit wrap, and the stream-wide byte counter adds the stride.
6. The signed comparison `parsed >= target` produces the end-request result.
   Otherwise the next-record flag is restored and an ordinary chunk is
   produced.

The host API accepts the already-decoded 24-byte record. It does not claim to
recover the surrounding raw-buffer pointer ABI.

## Queue admission and update

`Load` allocates `0x1800` bytes: 512 stable 12-byte entries. Raw head and tail
are wrapping `uint32_t` counters. Physical indexing uses `counter & 0x1ff`, and
the native full test is

```text
((head.wrapping_add(1) ^ tail) & 0x1ff) == 0
```

Consequently the usable capacity is 511. Empty/drained is the stricter raw
comparison `head == tail`, not masked-index equality.

One `Update` performs the following exact outer sequence:

1. A completed request is a no-op. Request end is exactly signed value `-1`;
   other negative values are not treated as ended.
2. At entry, an active request whose stream already reports EOS calls
   `GetChunk`. A non-null result is native assertion territory; otherwise the
   request ID becomes `-1`.
3. The chunk loop checks the 16-chunk gate, then ring-full, then calls
   `GetChunk`. A null chunk stops this update without ending the request.
4. After a returned chunk, EOS is sampled before metadata reads. The chunk's
   signed request ID must match the active request.
5. Metadata is looked up by the full asset ID. Resource-size words 1, 0, then
   2 are added with wrapping 32-bit arithmetic and interpreted as signed.
6. A total `<= 0` releases that occurrence immediately. It consumes neither a
   ring reservation nor destination budget; occurrences are not deduplicated.
7. For a positive total, head is incremented with wrapping arithmetic, then
   the slot retains the chunk and stored size. The highest stored block is
   replaced only on unsigned strict `stored_size > highest`.
8. Decompression submission forwards slot identity, full asset ID, metadata,
   chunk/source plus serialized data offset, stored size, raw format, and the
   callback boundary. Inner decompression and construction are opaque.
9. Only after submission are the first three resource sizes read again. Their
   wrapping sum is added to the wrapping per-update destination-byte counter.
10. If the earlier EOS sample was true, the request ID becomes `-1` and the
    loop stops. Otherwise the observed-chunk counter advances. Continuation
    requires both count below 16 and the unsigned budget below
    `0x0040_0000`. A chunk that reaches or exceeds the budget is already
    accepted.
11. Final completion requires request ID exactly `-1` and raw `tail == head`.
    The ring allocation is then released and the request is marked complete.

The Rust `CollectionLoadSlot` carries both physical index and raw reservation
counter. Native callbacks carry a stable queue-entry pointer; rejecting a
stale callback after a 512-reservation alias is a **high** safe policy for an
invalid asynchronous domain, not an exact native recovery.

## Completion ordering

`EntryComplete` consumes only the FIFO entry at `tail & 0x1ff`. When the
callback returns a non-null asset, it first adds the entry's stored byte count
to the wrapping loaded-size counter and then calls the loaded collection's
`AddAsset`. Whether the asset is null or non-null, it next releases the retained
chunk and finally increments raw tail. No asset-ID deduplication occurs here.

The Rust collaborator returns `Option<Asset>` and preserves this ordering.
Out-of-order, empty, post-drain, and stale-generation completion tokens are
permanently latched typed errors. Native uses assertions/pointer invariants in
these invalid domains, so the typed first-fault replay is **high**, not exact.

Metadata lookup is result-bearing at the Rust boundary. `Ok(None)` retains the
native-domain missing-record fault, while a catalog/backend error remains a
distinct typed source and is replayed by identity. This distinction is a
**high** safe-host policy; the native valid-data path assumes the lookup
succeeds.

## Decompression worker queue

PDB `tDecompressRequest` is exactly `0x28` bytes: asset ID at `+0x00`, asset
info pointer at `+0x08`, source pointer/size at `+0x0c/+0x10`, allocator at
`+0x14`, raw format at `+0x18`, callback at `+0x1c`, and user data at `+0x20`.
`cAssetStreamer::AddDecompressRequest` at `0x82d5c280` copies this complete
record to the global FIFO under its writer lock. It does not wake the worker.
`StartDecompress` separately atomically observes the dirty word. If that read
observes zero, it separately writes one and posts the semaphore. Sequential
kicks on the evidenced single-caller path therefore coalesce. The native update
is not one compare-exchange; stronger concurrent coalescing would be a safe-host
policy.

On each wake, `DecompressThread` at `0x82d5adb0` clears the dirty word, removes
at most 32 FIFO-front records into a local snapshot under lock, then unlocks
before construction and callbacks. The global FIFO itself is not capped at
32. Within a snapshot the first request always executes. Before each later
request, an unsigned accumulated prior-source-size value is compared with
`0x40000`; equality or overshoot ends that source slice. The current request's
source size is added with `u32` wrap only after its unconditional callback.

Raw format dispatch is exact: `1` calls `AddCompressedAsset` (`0x82d63830`),
`2` calls `AddCompressedArenaAsset` (`0x82d63a10`), `3` calls
`AddCompressedChunkArenaAsset` (`0x82d63bf0`), and every other word calls
`AddAsset` (`0x82d634d8`). The callback receives the returned asset pointer even
when it is null. One wake can consume further 32-record snapshots after the
worker's sleeps until it sees an empty snapshot. Exact thread scheduling and
the observable significance of the one-millisecond sleeps remain unknown.

## Boundaries and remaining unknowns

- Exact: parser gates and signed comparison; ring size/index/full and raw-empty
  arithmetic; 16-chunk and `0x0040_0000` gates; initial signed size decision;
  post-submit reread; EOS/request transitions; stored/high-water bookkeeping;
  FIFO completion side-effect order.
- High safe policy: typed errors and permanent first-fault replay replace
  native assertions, null dereferences, and stale callback memory hazards.
- Unknown/unimplemented: native mount-read/device timing; exact worker
  scheduling and sleep observability; decompression/materialization failure
  semantics; resident-map/refcount behavior; allocator ownership; and
  `DownCollectionLoadedCount`. The typed builder boundary intentionally
  prevents this module from inventing them.

## Validation

The focused standalone suite exercises the 24-byte early guard, stride values
0/1/`0x280000`/`0x280001`, final-record validation, signed/wrapping parser
counts, exact request `-1`, live call ordering, signed size sums, second reads,
budget equality/overshoot, 16/17 chunks, all 511 usable slots, raw counter wrap,
same-index stale generations, out-of-order no-mutation, EOS, null assets,
loaded-size wrap, repeat occurrences, first-fault identity, and completion
side-effect order.

Because root integration intentionally has not exported this new module yet,
the direct command is:

```powershell
rustc --edition=2021 --test crates/skate2-core/src/streaming/collection_load_pump.rs `
  -o "$env:TEMP\collection_load_pump_tests.exe"
& "$env:TEMP\collection_load_pump_tests.exe"
```
