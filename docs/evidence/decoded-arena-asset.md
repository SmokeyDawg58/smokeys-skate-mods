# Decoded world-stream arena asset

## Scope

`decoded_arena_asset.rs` implements the smallest common resource-construction
step shared by the selected BAM Presentation, Simulation, and Texture streams.
It accepts only native stream format 2 (`AddCompressedArenaAsset`), expands the
five compressed-arena resource segments, validates all five decoded sizes
against the owning XST metadata, and parses the RW4 arena/dictionary into owned
Rust values. The owner also retains the exact five XST `(size, alignment)`
pairs. Those allocation descriptors are intentionally separate from the five
descriptors embedded in the decoded RW4 `Arena`.

The output is deliberately generic. It does not run `cProcessArenaAsset`,
publish TOC entries, send Simulation/Presentation registration messages, build
world-painter or spatial-map state, or claim native allocator placement.

## Evidence and confidence

`cAssetManager::AddCompressedArenaAsset` is at `0x82d63a10`; raw format 2
selects it from `cAssetStreamer::DecompressThread` at
`0x82d5aff8..0x82d5b008`. PDB `tCompressedArenaHeader` supplies five 16-byte
segment records. The existing formats implementation preserves each raw
destination size, source size, compression word, and unused word, then applies
RefPack only when the compression word is nonzero.

The XST type `0xab329a6a` is corpus-observed for every BAM arena asset used by
this boundary. The builder rejects every other asset type and every mode other
than the exact raw-format-2/CompressedArena pair. These typed rejections,
stored-size checks, and error propagation are **high** safe-host behavior for
invalid inputs. Segment decoding, five-lane size agreement, and RW4 parsing are
exact for the valid corpus. Rust `Vec` ownership neither reproduces native
allocator addresses nor guarantees that host buffers satisfy the retained
alignments.

Native `cAssetManager::CreateCompressedArenaAssetData` at
`0x82d61fe0..0x82d621fc` forwards each XST `cAssetInfo` resource size and
alignment unchanged to `DoAllocate`. The built-in allocators allocate every
nonempty resource lane separately and produce null for an empty lane. The
generic allocator interface does not establish that every allocator must use
null for empty lanes, nor does this path establish ordering, contiguity, or
non-overlap between allocations. Retaining the XST pairs therefore preserves
the native allocation inputs without guessing the later five live bases.

This evidence is captured in
`world_probe_temp\arena_live_binding_raw.txt`, SHA-256
`F73BBFCAC6F4A7E4A827A2565DF9215943B370D42E9298A19381F4614EE44879`.
It also confirms that `Arena::FixupStart` copies five allocator-supplied bases
into the live Arena at `+0x94..+0xa4`; neither host `Vec` addresses nor the
serialized words at those offsets are substitutes for that runtime state.

## Rob's Park gate

A frozen external harness exercised the actual builder through
`MountedResidentCollectionLoadDriver` over the exact 14 Rob's Park focus
collections:

| Stream | Collections | Assets | Stored bytes | Stride bytes | Decoded bytes |
|---|---:|---:|---:|---:|---:|
| Presentation | 6 | 2,528 | 28,203,546 | 28,679,552 | 62,162,080 |
| Simulation | 4 | 68 | 6,884,133 | 6,897,408 | 11,164,760 |
| Texture | 4 | 685 | 38,420,692 | 38,552,960 | 68,791,160 |
| Total | 14 | 3,281 | 73,508,371 | 74,129,920 | 142,118,000 |

Every record used raw format 2 and asset type `0xab329a6a`. For each record the
test checked the 80-byte header, `stored_size == 80 + sum(source_size)`, all
five decoded resource lengths against XST, RW4 parsing, builder success, full
asset ID, and source order. No record failed. The optimized dedicated
125-arena DMO resident gate additionally fingerprints the retained descriptors
as `14ccc0e348011f23`: FNV-1a over the V1 domain tag, then each owner's
big-endian asset ID and five big-endian `(size, alignment)` pairs in resource-
lane order. The frozen Rob's Park harness used the pre-retention builder source
SHA-256
`252549A7512DCB7EAD41CCB7EC77348AFDBE99730F770173D5185E1D5CB87469`;
the current descriptor-retaining builder source SHA-256 is
`FD105EC2693EEEAE8ECFAE69E72BF173A981DFDD96AE48F57264B78234669B53`.
The external harness and output SHA-256 values were
`79D96A7A5195312B1F0C74D1A22190EE925A4541F3536EFC2F41F3EC9FD5052A`
and `1D2649F8529D765F3C7AD6E65722456C1C40AD1B47FC5DDD05C63FE90F1AA0E2`.

This proves generic arena decompression and RW4 ownership for the selected
world data. It is not yet the registration/destruction behavior required for a
live freeroam world.

## Validation

Focused tests cover valid owned construction plus raw-format, mode, full-ID,
asset-type, stored-size, XST resource-size, envelope, and RW4 failures. A
discriminator retains XST alignment `0x80` while the embedded Arena alignment
is independently `0x10`. Synthetic downstream fixtures derive sizes from
their actual buffers and copy embedded alignments only inside an explicitly
test-only constructor; production construction always retains the XST values.
Independent frozen review passed after checking the builder's ownership,
validation order, typed source chains, and absence of subsystem side effects.
