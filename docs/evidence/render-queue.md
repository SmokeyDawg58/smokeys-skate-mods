# Native render queue evidence

This checkpoint reconstructs the world mesh submission and sort model as
ordinary typed Rust. The Xbox executable and PDB establish behavior; no PPC
instruction stream, Xbox graphics API, or emulator renderer is present in the
runtime implementation.

## Reference functions

| Function | Address | Recovered behavior |
|---|---:|---|
| `PresentationEntity::AddLODModelInstances` | `0x827fa1c0` | Creates one `MeshContext` and one material sort key per model mesh |
| `PresentationEntity::FillMtlSrtKeyListWithCulledKeys` | `0x827f9ec0` | Appends the current LOD's material keys |
| `WorldPresEntityOptimesh::FillMtlSrtKeyListWithCulledKeys` | `0x827e3068` | Emits only optimized meshes with at least one visible island |
| `SceneRenderView::Sort` | `0x82839b38` | Gathers visible keys and performs a stable unsigned radix sort |
| `SceneRenderView::RenderMeshes` | `0x82850670` | Dispatches effect/material state only when their key bytes change; defers multi-pass work |
| `cModelInstance::Resolve` | `0x82853de8` | Builds each mesh's effect list and selects technique `default` |
| `cMaterialManager::CreateEffectListFromMaterialKey` | `0x82e203f8` | Resolves effect name and draw order, then fills the four-byte instance key |
| `cEffectScope::cEffectScope` | `0x82e1ffa8` | Pre-increments the global effect-scope identifier |
| `cMaterialScope::cMaterialScope` | `0x82e1d2c0` | Pre-increments the global material-scope identifier |
| `cFxHandle::Technique::Technique` | `0x82e1cfb8` | Names scopes `<effect>_<technique>` |

`cInstanceScope::InstanceSortKey` is four bytes in this exact order:

1. draw/sort order;
2. effect-scope identifier;
3. technique pass count;
4. material-scope identifier.

The native `RenderSortKey::packed` therefore uses big-endian byte packing. A
key with bytes `03 04 02 09` is the unsigned integer `0x03040209`. Rust's stable
slice sort preserves insertion order for equal keys, matching the reference
stable radix result without reproducing the console implementation.

## Mesh material ownership

The raw loads in `cModelInstance::Resolve` read the model-table mesh pointer and
then offset `0x24` from the pointed mesh. For `tROptiMeshData`, that offset is
the mesh-owned material. The model table's second pointer is not the material
selected for these optimized world meshes. `NativeRenderQueue` follows that
pointer ownership explicitly.

`WorldPresentation::GetRenderInstanceType` at `0x827d0a28` separately uses the
model table's second pointer only to distinguish old-format from new-format
instances. `WorldPresentation::AddRenderInstance` at `0x827e4f00` then creates
the appropriate human-level presentation entity.

## Missing reference fallback asset

Five VLT material families contain an inline null `effect` node. The reference
factory consequently selects the literal name `default`:

- `decal.default`
- `glare.default`
- `ocean.default`
- `proxyworld.default`
- `tree.default`

This is not an attribute-parser inference. Each relevant node has `kByValue`
set and a zero inline pointer; `Attribute::Attribute` (`0x82979d90`) and
`Node::GetPointer` (`0x82979ac8`) therefore expose a null effect string, after
which `CreateEffectListFromMaterialKey` retains its literal fallback.

No `data\render\shaders\final\default.xml` exists in any of the 19 available
mounted archives, including hash-only archives probed by exact virtual path.
The only unavailable boot archive in the supplied corpus is
`colourcubes.big`. Public searches did not establish its contents, so mapping
`default` to a visually plausible shader would be unsupported.

The missing effect is not dead metadata. In the initial Rob's Park focus set,
real high-LOD fused models reference 52 `decal.default`, 3 `glare.default`, and
33 `tree.default` mesh contexts. These 88 contexts are retained as typed
`DeferredRenderSubmission` records containing exact scene and material
provenance. They are not emitted to a native backend and are not counted as
parity-complete rendering.

**Correction (2026-09-27).** None of the five falls back to `default`. Their
`effect` entries are inline Vault nodes whose own relocation points at the
string: `decalzbias`, `glare`, `ocean`, `proxyworld` and `tree`
(`tools/vault_dump.py`, identical in the Nov-28-2008 and Jan-17-2009 Vaults).
The reading above dereferenced the inline slot once too often; the resolver
was fixed with the sky (`render-sky-inline-effect-2026-09-27.md`).

## Corpus checkpoint

The Rob's Park runtime probe currently produces:

- 2,085 resolved native mesh contexts;
- 4,175 resolved draw islands;
- 88 deferred contexts with the absent `default` manifest;
- 18 effect scopes and 19 material scopes in observed creation order.

Moving the presentation focus 200 world units selects the adjacent six-cell
set and produces 1,777 resolved contexts, 4,027 islands, and 80 deferred
contexts. Returning to Rob's Park restores the exact resolved and deferred
queue signatures, including scene handles, material identities, manifest and
technique names, and packed sort keys.

## Remaining parity work

- Obtain or otherwise establish authoritative behavior for the missing
  `default` effect asset.
- Reconstruct visibility/frustum and island culling before queue gathering.
- Validate global scope-ID creation order across all preloaded and dynamically
  created effects.
- Reconstruct readable native shader programs and parameter bindings from the
  XML/state semantics and visual/reference testing; Xbox `.vpo`/`.fpo` code
  remains evidence only.
- Implement native geometry, texture, and draw backends.
