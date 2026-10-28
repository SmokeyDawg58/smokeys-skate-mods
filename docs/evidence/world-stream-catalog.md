# Parsed world-stream registration catalog

This record covers the shared native ownership reconstructed from the XSM,
XST, XSS, and optional stream-XML callbacks. `WorldStreamCatalog` is both a
caller-thread `StreamRegistrationSink` and a cloneable owning reader. It keeps
this parsed registration domain separate from the later processed-XSS global
publication domain.

## Evidence anchors

| Address | Function or behavior |
|---:|---|
| `0x82d5db60` | `cGameStream::Update` registration and reverse unload order |
| `0x82d5aca0` | `cAssetStreamer::AddCollectionEntries` |
| `0x82d5ad14` | XSF path construction |
| `0x82d54048` | `cCollectionMapInfo` 0x60-byte C-path buffer handling |
| `0x82d643d8` | `cAssetManager::AddAssetInfo` |
| `0x82d64444` | signed `cLoadedAssetInfo::mRefCount` increment |
| `0x82d644d8` | `cAssetManager::RemoveAssetInfoRef` |
| `0x82d64550` | signed `mRefCount <= 0` assertion branch |
| `0x82d64570..0x82d64574` | reference decrement |
| `0x82d599f0` | `cAssetStreamManager::GetStreamOffset` |
| `0x82d599f8` | `cAssetStreamManager::GetStreamSize` |

The manager queries tail-call the lower-level `assman::cAssetStreamer`
functions at `0x82d59610` and `0x82d59688` respectively.

## Reconstructed behavior

XSM registration constructs each collection path as the retained directory
bytes followed immediately by the authored filename and `.xsf`. It does not
insert or normalize a separator. The completed path may contain at most 95
bytes before NUL, and the global collection table has exactly 2,048 distinct
IDs. Payload offsets remain raw `u32` values widened to `u64`; sizes remain raw
`u32`. Equal filenames are allowed, while a duplicate collection ID is
rejected.

Every XST asset occurrence increments one signed reference count. The first
entry for an ID supplies the retained type and all five resource descriptors;
later occurrences do not replace it. Removing one file decrements every
occurrence in that file and erases the global entry only when the signed result
reaches zero. Safe Rust rejects an increment past `i32::MAX` and any nonpositive
count at unload instead of reproducing native signed overflow or invalid-state
continuation.

The per-file lifecycle is explicitly
`Registered -> CollectionsUnloaded -> AssetsUnloaded`. This preserves the
state-5 XSM-before-XST removal order and prevents retained diagnostic source
tables from accidentally reopening XST, XSS, or XML load stages after unload
has begun. XSM-only partial loads may still complete final file cleanup after
their collection registration is removed. A file with active registrations
cannot be removed.

XSS is retained as an opaque owning clone. Every float lane, NaN payload,
signed zero, LOD byte, ID, version, and stamp is preserved without inventing a
stamp-matching rule. Successful empty XML remains `Some(empty)` and is distinct
from no successful XML load. All seven callbacks use stable
`RegistrationFileId` identity, so simultaneous equal kind/path files cannot be
conflated.

`stream_offset` and `stream_size` read this catalog. In contrast,
`is_collection_valid` must query `PublishedStreamStore`: parsed XSM existence
does not mean a processed XSS record is globally published.

An owning collection snapshot retains its full map key alongside owner, source
index, path, offset, size, asset count, format, and parent. This self-binds the
metadata passed to a later operation so a safe backend can reject pairing one
dispatch ID with another collection's snapshot.

## Safe ownership policy and confidence

One `Arc<Mutex<_>>` gives cloneable readers coherent owning snapshots and
returns a typed poison error. The reference uses different native containers
and synchronization. For failures returned as `CatalogError`, each callback
preflights its complete logical change before mutation. This does not claim
recovery from process panic or allocator abort.

Symbols, addresses, path construction, capacities, raw numeric widths, signed
reference semantics, first metadata, per-occurrence counts, callback order,
and valid unload behavior are **exact**. The one-lock snapshot model,
transactional returned-error handling, checked arithmetic, stable Rust identity,
and typed invalid-state rejection are **high-confidence safe policies**. Native
allocator exhaustion and invalid native continuation remain unknown.

The first frozen candidate was rejected for using an unsigned reference domain
and allowing load callbacks after unload began. The corrected candidate was
held again until negative counts, every nonempty late callback, XST-before-XSM,
full first-owner removal, bit-distinct XSS values, and final ownership mismatch
were directly tested. The authoritative source then passed independent review.
Twenty-seven focused tests cover these cases plus capacity, path, duplicate,
same-path owner, poison, early unload, and full reverse-unload behavior.
