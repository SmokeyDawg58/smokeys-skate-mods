# Presentation geometry reconstruction

Last updated: 2026-09-02

## Source-level boundary

Presentation data is reconstructed as named Rust records and native vertex/index
arrays. The Xbox D3D records are used only to recover asset metadata and buffer
references; no Xbox graphics API or emulator renderer is part of the runtime.

| Reference routine | Address | Recovered behavior |
|---|---:|---|
| `pegasus::RegisterArenaTypes` | `0x829c8640` | Maps Pegasus render-model and mesh type IDs |
| `tRMeshData::Fixup` | `0x829c6e10` | Distinguishes arena IDs from object-relative draw/blend tables |
| `tRModelData::Fixup` | `0x829c7080` | Fixes inline mesh/bone/island tables and mesh/material IDs |
| `tROptiMeshData::Fixup` | `0x829c74f0` | Fixes optimized mesh buffers, islands, remaps, and blend shapes |
| `renderengine::RegisterArenaReadCallbacks` | `0x82bcf900` | Maps render resource type IDs |
| `MeshHelper::RegisterArenaReadCallbacks` | `0x82bd13c8` | Maps `00020081` to `MeshHelper` |
| `MeshHelper` archive fixup | `0x82bd1330` | Fixes the index-buffer then vertex-buffer ID array |
| `VertexBuffer::Fixup` | `0x82bcf718` | Applies the serialized GPU fetch-pointer rule |
| `IndexBuffer::Fixup` | `0x82bcf410` | Resolves the serialized raw-data object ID |

## Proven arena type IDs

| Type ID | Native meaning |
|---:|---|
| `00eb0000` | `pegasus::tRMeshData` |
| `00eb0001` | `pegasus::tRModelData` |
| `00eb0023` | `pegasus::tROptiMeshData` |
| `00020081` | `renderengine::MeshHelper` |
| `000200e9` | `renderengine::VertexDescriptor` |
| `000200ea` | `renderengine::VertexBuffer` |
| `000200eb` | `renderengine::IndexBuffer` |
| `00010031` | Raw GPU resource bytes used by buffers |

`VertexBuffer` fetch word zero encodes the raw-resource arena ID as
`serialized_id << 2`, leaving the low two fetch flags intact. `IndexBuffer`
stores the raw-resource ID directly at offset 24. All 85,438 buffer-to-resource
links in BAM resolve through these two rules.

## Native decoding

The flexible vertex declaration supplies stream number, byte offset, format,
usage, and usage index. The complete BAM corpus uses eight named reference
formats; position is either `FLOAT3` (42,713 meshes) or `FLOAT16_4` (six meshes).
Both are decoded from big-endian resource bytes. Index depth is recovered from
the serialized index-buffer common word and decoded as big-endian 16- or 32-bit
values. Inline `DrawIndexedParameters` records retain primitive type, base
vertex, start index, and index count.

PDB `renderengine::VertexFormat` names and the complete declaration census
establish the native decoders:

| Format | Serialized value | Native interpretation | Declaration elements |
|---|---:|---|---:|
| `FLOAT3` | `002a23b9` | three IEEE `f32` components | 42,713 |
| `UBYTE4` | `001a2286` | four unsigned bytes | 6 |
| `HEND3N` | `002a2190` | signed normalized 11/11/10-bit vector | 29,284 |
| `SHORT2N` | `002c2159` | two signed normalized 16-bit components | 8,238 |
| `SHORT4N` | `001a215a` | four signed normalized 16-bit components | 34,318 |
| `SHORT2` | `002c2359` | two signed 16-bit integers | 6 |
| `FLOAT16_2` | `002c235f` | two IEEE half components | 39,406 |
| `FLOAT16_4` | `001a2360` | four IEEE half components | 3,207 |

All 157,178 elements use stream zero, method zero, and class one. Named D3D
usage values map them to position, blend weight, blend indices, normal, up to
three texture-coordinate sets, tangent, and sample attributes. PDB `XMHEND3`
bitfields prove that `HEND3N` stores signed X/Y/Z fields at bits 0/11/22 with
widths 11/11/10; this is decoded directly as a domain vector, not emulated as a
GPU fetch instruction.

## Complete-corpus validation

`--inventory-bam-pres` currently proves:

- 285 collection records, 44,035 RW4 arenas, and 477,643 objects;
- 1,908 render models and 42,719 model-to-mesh entries;
- 42,719 optimized meshes and no regular meshes in this BAM stream;
- 42,719 declarations containing 157,178 elements;
- 11,980,512 derived and successfully decoded positions;
- 114,459 non-position arrays containing 29,931,990 decoded attribute values;
- 24,642,246 successfully decoded indices;
- 73,197 island draw packets, all `D3DPT_TRIANGLELIST`;
- 24,642,246 indices covered by those draw packets exactly;
- 426,075 checked local object links, zero external links, and zero failures.

One source-authentic exception is retained rather than sanitized: one
`TEXCOORD0/FLOAT16_2` array in `cPres_300_900_high`, asset
`289d27314f2b857c`, contains serialized half `-infinity` in the Y component of
vertices 461 through 464. No position, normal, or tangent is non-finite. The
inventory reports this four-value sentinel while accepting the structurally
valid mesh; effect reconstruction will determine its shader meaning.

Geometry, materials, and complete texture mip chains now have native owned
representations. A first visible frame additionally requires the native
processed-TOC registration boundary: TOC records expand into ordered typed
assets, Presentation arena references coalesce on first use and last release,
and processed instances publish through their subsystem managers before the
FIFO completion callback. Only after that live-scene bridge is reconstructed
do PC GPU upload, the renderer backend, and shader/effect parameter mapping
become the remaining display boundary. Decoded models and meshes alone are not
native publication.

## Rob's Park runtime ownership

At `A_RobsPark` the exact presentation focus returns six cells, ordered by XZ
distance:

1. `cPres_500_900_high`
2. `cPres_500_1100_high`
3. `cPres_700_900_high`
4. `cPres_700_1100_high`
5. `cPres_300_900_high`
6. `cPres_300_1100_high`

The extra 300-grid pair is a real edge overlap with the 195-unit presentation
focus box, not a naming heuristic. The runtime scene owns 2,528 arenas, 2,173
meshes, 85 models, 117 instances (85 render-bearing), 734,053 positions,
5,859 non-position arrays containing 1,983,887 values, 1,696,275 indices, and
4,281 draw packets. Its ownership chain is explicit Rust data rather than
dictionary buffers retained behind opaque handles.

Moving the focus 200 units along +X unloads the two 300-grid cells and loads the
two 900-grid cells. The adjacent scene owns 2,072 arenas, 1,857 meshes, 72
models, 109 instances, 745,388 positions, 1,802,817 indices, and 4,121 draws.
Returning restores the complete initial count and per-mesh data signature.

Mesh material IDs no longer survive as opaque runtime handles. All 42,719 IDs
in the corpus use section 2 and resolve through the RW4 subreference dictionary
to exact 12-byte `tRMaterial` records inside `tRMaterialData`; the owned scene
stores material indices and copied semantic channel values instead. See
`material-textures.md` for the texture-stream and layout evidence.
