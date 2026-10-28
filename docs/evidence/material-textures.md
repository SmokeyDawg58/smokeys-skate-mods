# Material and texture reconstruction

Last updated: 2026-09-04

## Material layouts and ownership

The PDB types and `pegasus::tRMaterialData::Fixup` at `0x829c5b38` establish
the exact source-level records:

| Record | Size | Relevant fields |
|---|---:|---|
| `tRMaterialData` | 20 | material/channel counts, two inline arrays, string list |
| `tRMaterial` | 12 | channel count, flags, pointer into the channel array |
| `tRMaterial::tChannel` | 32 | shader-input string, flags, image channel, value union |

Flag bit 1 (`kScalarConstant`, value 2) selects four floats. Otherwise the
union contains a 64-bit GUID, stream-name string, and runtime stream pointer.
The parser keeps object-relative strings distinct from arena object IDs.

The complete BAM presentation corpus contains 2,187 material blocks, 45,678
materials, and 538,383 channels: 398,502 raster-shaped records and 139,881
scalar constants. Material flags are always zero and channel flags are exactly
zero or two. All 42,719 mesh material references use RW4 section 2 and resolve
through a type-zero subreference to an exact inline material boundary inside a
`00eb0005` material-data object. There are no unresolved references.

The Rob's Park presentation focus owns 2,377 materials and 27,972 channels
(20,704 raster-shaped) as ordinary Rust values. Meshes refer to native material
indices rather than serialized `ObjectId` handles.

## Texture identities and streams

Render-engine registration and serialization map `000200e8` to `Texture` and
`00010031` to its raw payload. Fetch word 1 stores the payload object ID shifted
left by 12. The named width, height, depth, and format accessors at
`0x82bca840`, `0x82bca8f0`, `0x82bca998`, and `0x82bcaa30` establish the fetch
bit fields decoded by Rust.

The dedicated BAM texture stream has 338 collections, 39,935 arenas, 159,740
objects, and 39,935 TOC entries. Every arena contains one TOC, one Texture, one
raw resource, and one arena metadata object. Every TOC link is local and lands
on its Texture. The corpus formats are:

| Format | Textures |
|---|---:|
| BC1 / `DXT1` | 33,274 |
| BC3-family / `DXT5` | 6,650 |
| `A8R8G8B8` | 11 |

There are 39,633 2D textures and 302 cube maps.

Raster channel GUIDs are texture identities, not collection/arena IDs. Of the
nonzero presentation-channel references, 165,377 occurrences match an inner
BAM texture TOC. Other names include material metadata (`Name`,
`AttribulatorMaterialName`) and textures supplied by global/non-BAM stores, so
absence from the BAM texture TOCs is not treated as a broken local link.

## Native tiled-layout decoding

`XGRAPHICS::UntileSurface` at `0x82c303c0` and
`XGUntileTextureLevel` at `0x82c309e8` were used as behavioral evidence for a
small, named asset-layout algorithm. The runtime does not call or include those
Xbox routines. Rust expresses their 32x32 macro tiles, 8x8 micro tiles, and
memory-bank interleave as a closed-form byte-offset function.

Synthetic tests prove that offsets are a bijection for 2, 4, 8, and 16-byte
blocks and reconstruct known linear block order. Complete-corpus validation
then linearizes all 39,935 textures: 41,445 top-level surfaces and
2,123,877,888 bytes, including all six faces of every cube map, with zero
layout or bounds failures. BC1/BC3 data remains block-compressed.

The PDB `SurfaceEndian` enum and the `GPUTEXTURE_FETCH_CONSTANT` layout place
the endian selector in fetch word 1 bits 6..7. Rust exposes the four modes as a
named enum and normalizes components after untile: 8-in-16 swaps adjacent bytes,
8-in-32 reverses each four-byte unit, and 16-in-32 swaps its two halfwords. The
entire corpus uses only two modes: all 39,924 BC1/BC3 textures use 8-in-16 and
all 11 A8R8G8B8 textures use 8-in-32. Revalidation still reports zero failures.

## Complete mip chains

The fetch constant's `PackedMips` bit and aligned `MipAddress` become the
semantic Rust fields `packed_mips` and `mip_data_offset`. `SetID`/`SetAddress`,
`D3D::FindSurfaceWithinTexture` at `0x82c3b2d0`, and the two mip-tail coordinate
routines establish the source-level storage rule:

1. level zero uses the base allocation unless it already belongs to the tail;
2. independent mip levels are page-aligned and stored level-major, then face;
3. levels at or below the 16-texel threshold share one packed-tail allocation
   per face and use explicit two-dimensional origins.

There are 8,901 single-level textures. Every one of the 31,034 multi-level
textures enables packed mips; 228 small textures start their packed tail at
byte offset zero. Adding the serialized base offset, independent allocations,
and one tail allocation per face predicts the exact raw-resource length for
all 39,935 textures, with no extra or missing bytes.

The native decoder now emits a record for every mip and cube face. Corpus
validation produces 294,646 level/face surfaces and 2,535,339,380 row-major,
component-normalized bytes with zero layout or bounds failures. Synthetic tests
also place distinct BC1 blocks into every packed-tail level and recover each
level without overlap.

At Rob's Park, the 70-unit texture focus selects `cTex_525_975_high`,
`cTex_675_975_high`, `cTex_525_1125_high`, and `cTex_675_1125_high`. The owned
scene contains 685 textures, 68,386,816 encoded bytes, and 53,813,952 linear
bytes across their complete mip chains. A 150-unit east-and-back stream
transition restores all counts and per-texture decoded signatures.

## Runtime material bindings

The native presentation and texture scenes meet through a
`ResidentTextureHandle` containing collection, arena, and texture indices. A
GUID index is rebuilt from current texture residency; materials never retain an
RW4 `ObjectId`, serialized pointer, or platform resource. Bindings are checked
by dereferencing every resolved handle and comparing its GUID.

At Rob's Park the presentation focus owns 20,704 raster-shaped union channels.
The native layer classifies 4,754 `Name`/`AttribulatorMaterialName` channels as
material metadata, leaving 15,950 sampled raster channels. Of those, 5,704 bind
to the current 685 streamed textures and 10,246 remain explicitly unresolved
because their global store or nonresident cell has not yet been incorporated.
No fallback texture is silently substituted.

## Processed material registration

`cProcessArenaAsset::ProcessMaterial` at `0x82d5ebd8` copies the TOC entry's
full GUID and fixed-up object pointer into one Presentation kind-2 `cAsset`.
The pointer is the complete `tRMaterialData`, not an inline `tRMaterial`.
`Presentation::RegArena` at `0x827f1574..0x827f1584` and `DRegArena` at
`0x827f17c4..0x827f17d4` pass only that data pointer to
`TextureMan::AddMaterial` (`0x8282c838`) and `RemMaterial` (`0x82825cf8`).
Neither arm samples userdata.

`AddMaterial` walks materials and their channels forward, starting at
material-local channel index 2. It skips scalar-flagged and zero-GUID channels,
looks up `guid | 1`, then `guid & ~1`, then the configured missing-texture ID,
and writes the selected runtime stream value into the channel's `m_pStream`
field at `+0x1c`. Missing texture or already-bound-channel assertions have no recovered
rollback. `RemMaterial` is asymmetric: it starts at channel zero and clears
`+0x1c` for every channel without testing flags or GUIDs. These mutations prove
that copied high-level material values cannot stand in for native source
identity.

The main DMO bank's sole processed material is GUID `8f51812cef1b09a4` in
outer arena `d3096f13cf1c0aab`, TOC 68, dictionary 1/resource 0 span
`544..30451`. The bank-scoped adapter retains that complete owner-bound object
and orders its add/remove before the 93 DMO calls. It deliberately leaves the
mutable fixed-up material representation, texture-registry contents, shader
table, failure-after-prefix behavior, and GPU visibility outside its current
dispatch seam.
