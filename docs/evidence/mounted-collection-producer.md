# Mounted collection producer

## Scope

`crates/skate2-core/src/streaming/mounted_collection_producer.rs` supplies the missing
logical-byte producer immediately below `CollectionLoadPump`. It is deliberately
not a reconstruction of the complete RenderWare file-system device, priority
scheduler, cancellation machinery, decompressor, or resident-asset manager.

The module splits ownership into:

- `MountedCollectionProducer<R>`, which performs one synchronous logical range
  read per poll and drives every raw-parser decision possible from the newly
  accumulated bytes;
- `MountedCollectionChunkStream`, the unique consumer placed inside
  `CollectionLoadPump`;
- `MountedCollectionChunk`, an owned `Arc<[u8]>` stride and its exact logical
  source range, retained until the pump releases it; and
- `CollectionLogicalRangeSource`, an injectable seam implemented for
  `MountSet` and usable by deterministic synthetic/corpus tests.

The source file is intentionally not exported from `lib.rs` in this bounded
change. Integration requires only `pub mod mounted_collection_producer;` after
independent review.

## Reference evidence

The reference image base is `0x82000000`. The relevant exact anchors are:

| Symbol | VA | Behavior used here |
|---|---:|---|
| `rw::core::filesys::Stream::getfreerequest` | `0x82bb9b38` | full raw request-ID generation |
| `rw::core::filesys::Stream::GetChunk` | `0x82bb9cc8` | source-order dequeue and gettable-byte subtraction |
| `rw::core::filesys::Stream::IsEndOfStream` | `0x82bb9df8` | idle and zero gettable bytes |
| `rw::core::filesys::Stream::Stream` | `0x82bbaea8` | thresholds, request records, priorities, buffer fields |
| `rw::core::filesys::Stream::parsechunks` | `0x82bbc388` | accumulated availability, parser loop, chunk size |
| `rw::core::filesys::Device::ThreadEntry` | `0x82bbcaf0` | native device-worker callback site |
| `rw::core::filesys::AsyncOp::DoRead` | `0x82bbd720` | driver/quantum/physical-boundary subreads |
| `rw::core::filesys::Stream::startnextrequest` | `0x82bbe628` | full offset and request transition |
| private `rw::core::filesys::Stream::QueueFile` | `0x82bbea08` | 255-byte path retention and request queuing |
| `rw::core::filesys::Stream::readcallback` | `0x82bbebf0` | short-read EOF and parser invocation |
| `rw::core::filesys::Stream::restartstream` | `0x82bbed50` | `0x40000` reads, four-byte offset alignment, circular storage |
| public filename `Stream::QueueFile` | `0x82bbf0b8` | filename wrapper used by the collection request |
| `rw::core::filesys::Stream::ReleaseChunk` | `0x82bbf0f8` | release by the complete parser block size |
| `assman::tLoadCollectionRequest::ParseRawFunction` | `0x82d546b0` | strict `> 24` gate, stride latch, raw 0/1/2 results |
| `assman::tLoadCollectionRequest::Load` | `0x82d54bd0` | filename, full-u64 offset, parser/context, target count |
| `assman::tLoadCollectionRequest::EntryComplete` | `0x82d5e4a8` | asset publication, chunk release, then FIFO tail |
| `assman::tLoadCollectionRequest::Update` | `0x82d5fc98` | GetChunk/EOS sampling and retained decompression work |

The executable, PDB, and linker-map SHA-256 values used for this slice are:

- XEX: `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`
- PDB: `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`
- MAP: `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`

## Recovered behavior

The following behavior is **exact** in the reference and retained by this
valid-input boundary:

- the collection Stream requests `0x0004_0000` bytes at a time;
- the initial file read begins at `payload_offset & !3` and the low two offset
  bits are discarded before parsing;
- parser `datasize` is all accumulated contiguous data, not just the newest
  read;
- availability `<= 24` returns more-data before reading the header;
- a new header supplies one stride at `tCollectionAsset + 0x10`, which remains
  latched across later more-data callbacks;
- each accepted parser `block_size` is the complete stride, not stored payload
  size or data offset;
- one completed read may emit multiple chunks in source order;
- parser raw result 2 emits its chunk and immediately stops that request;
- the stream-read byte counter advances by each whole accepted stride;
- a final chunk prevents EOS while queued; EOS becomes visible when that final
  chunk has been dequeued; and
- release consumes the whole-stride ownership retained across decompression.

The maximum accepted stride is `0x0028_0000`, exactly ten `0x40000` stages.
The implementation keeps the request ID as the supplied raw `i32`; only `-1`,
the ended-request sentinel, is rejected at construction.

The native request path copies at most 255 bytes and forces byte 255 to NUL.
This module accepts a 255-byte UTF-8 path but returns a typed error for a longer
path or interior NUL instead of silently truncating it.

## Safe PC policies and deviations

The following choices are **high** confidence safe host architecture, not exact
native implementation parity:

- `poll_parser` is synchronous and caller-thread driven. No background worker
  or callback is introduced in this slice.
- A zero-length `0..0` logical read obtains the complete entry length before
  staged reading. Native obtains length while opening its file handle.
- `MountSet` clamps each logical request to known EOF. Native requests the full
  `0x40000` and learns EOF from a short read.
- Chunks copy one stride into an immutable `Arc<[u8]>`. Native chunks point into
  its `0x0054_0100` circular buffer.
- Consequently this slice records retained stride bytes but does not reproduce
  native circular-buffer pressure, 16/17-byte reservations, device priority,
  or release-prefix reclamation.
- A chunk carries a weak link to its originating shared state. Explicit release
  updates the correct ownership record even if a caller routes the uniquely
  owned handle through the wrong consumer value.
- Private mutex poison is recovered. No source call, parser callback, or other
  collaborator executes while the shared stream mutex is held.
- Source, parser, range-contract, and premature-EOF failures are typed and
  first-fault sticky. The consumer never reports EOS after such a failure.

The native generic EOF path at `0x82bbc46c..0x82bbc4a8` can fabricate a final
chunk from leftover bytes when the parser still asks for more data. In
particular, an exactly 24-byte final window never enters the collection parser.
This module instead returns `UnexpectedEndOfEntry`; reproducing the fallback
would bypass the recovered collection stride/count validation.

Native open failure at `0x82bbadc0` frees the request without delivering a
typed load failure. Native read callback code observes total byte count rather
than an explicit status, and the underlying BIG-driver error return convention
has not been recovered. Mapping host `MountError` to a sticky typed error is
therefore an intentional safe deviation. Claims that every native read failure
is a zero-byte EOF remain a **hypothesis**.

`MountSet::read_path_range` expands a RefPack-compressed outer BIG entry in full
for every logical range request. The selected Rob's Park XSF entries are raw,
so this does not affect the initial production gate; it remains a performance
and scheduling deviation for compressed outer entries.

## Ownership and poll order

One producer poll performs:

1. the one-time logical-length/storage probe when required;
2. one clamped logical range read of at most `0x40000` bytes;
3. first-stage prefix discard for an unaligned full-u64 payload offset;
4. repeated parser calls while complete strides remain available;
5. after each parser decision, creation of an owned Arc-backed chunk and
   source-order publication under the short shared-state lock; and
6. publication of `producer_finished` in the same critical section as the
   parser-result-2 chunk.

The source read and parser callback both happen outside the shared lock. This
is required because `CollectionLoadPump::parse_raw` calls
`account_stream_read_bytes` through its contained consumer.

After result 2, unread bytes from the completed stage are discarded and no
later source read occurs. `MountedCollectionChunkStream::is_end_of_stream`
returns true only when `producer_finished` is set and the source-order queue is
empty. It intentionally does not wait for already-dequeued decompression
handles to be released, matching native gettable-byte semantics.

## Corpus gate

The public range-source seam, per-stage progress, exact logical source ranges,
and stream snapshots allow a real-data gate without adding game-root knowledge
to core. The previously validated Rob's Park set contains:

- six Presentation XSFs: 2,528 records and 112 native-sized stages;
- four Simulation XSFs: 68 records and 29 stages; and
- four Texture XSFs: 685 records and 149 stages.

All 14 selected entries use raw outer-BIG storage, have payload offset `0x80`,
and have logical length `0x80 + XSM payload_size`. A frozen external harness at
commit `a19a984c7789d05fe6cf0d21eb623b7a08eb96c7` composed the production
`MountSet`, `MountedCollectionProducer`, and `CollectionLoadPump` across that
complete set. It observed 3,281 submissions over 290 producer stages and 361
pump updates. Exact source and stride accounting both totaled 74,129,920
bytes; submitted stored bytes totaled 73,508,371. Every emitted ID, stride,
stored size, data offset, absolute source range, metadata lookup, FIFO
reservation, and sequence matched an independently parsed `CollectionPayload`
and the first-wins XST table. EOS appeared only after the last chunk dequeue,
and every file ended with zero queued or retained chunks.

The test deliberately returned null for all 3,281 construction callbacks, so
it proves transport, metadata submission, FIFO completion, and ownership
release but does **not** prove decompression or asset materialization. Its
one-stage/one-pump-update/all-callbacks loop is a deterministic host test
policy, not native thread timing. Under that policy the peak pump ring was 16;
the largest producer backlog was 327 chunks / 2,883,840 stride bytes. The
harness source SHA-256 is
`EDA03807CB386F41F3CC24D9231018FF76FD34769D34A37284DEDCCCCABE4833` and
the frozen output SHA-256 is
`B639AB0C4A8F498F5FC6019539DC16846F20395FBD07FF97630D1CCC1EF602E5`.

The old audit's `logical_end` and `trailing` fields predate the corrected stride
traversal and are not valid final-span evidence.

## Focused validation

The module's standalone tests cover:

- 255/256-byte paths, interior NUL, and raw `-1` rejection before I/O;
- an unusual full-u64 unaligned offset, exact align-down range, and three-byte
  discard;
- the strict 24/25-byte callback/header boundary;
- two-stage stride latching and the ten-stage maximum stride;
- multiple source-order emissions from one read and immediate result-2 stop;
- Arc-backed block/source ranges and checked payload slicing;
- EOS before and after final dequeue;
- actual `CollectionLoadPump` accounting/release without lock reentrancy;
- source, parser, length, storage, range-size, offset, and truncated-EOF faults;
- first-error Arc identity replay; and
- production `MountSet` missing-entry propagation.

Because the module is intentionally not exported yet, validation uses a
disposable external crate that re-exports `collection_load_pump` and includes
this source by path. After integration, the same tests run as ordinary core
unit tests.

## Remaining unknowns

- BIG-driver read-error return values and malformed partial-read behavior.
- Native device priority, read-quantum, physical-location, cancellation, and
  callback interleavings.
- Exact multi-request interaction when both native request records retain
  buffer spans.
- Native allocation-failure behavior for Stream chunks.
- Whether production should share the existing mounted backend worker pool or
  use a collection-specific asynchronous scheduler.
- Whether reproducing the native `0x540100` circular pressure is observably
  necessary once owned host chunks are used.

Those remain **unknown** and are not represented as generic asynchronous or
failure behavior in this module.
