# Selected character material admission audit

Read-only author audit, 2026-09-22. Confidence: high for the fresh source slices
below; selected original-data execution remains pending. No code, Cargo, GPU
execution, or external asset mutation accompanies this proposal.

## Reuse and actual allocation

`MountedPlayerRecipeLoad` already retains the finalized `Arc<CharacterScene>`.
`FinalizedRecipeScene::instances` returns references into those exact arenas,
including native component/model/LOD order and Beard suppression. The palette
retains the same scene. Extend this existing finalization view with checked
component/model/LOD/instance handles if necessary; do not decode another scene
or choose an arbitrary first instance from an arena for material admission.

`PresentationModelMesh.material`, `PresentationMesh.material`, full ordered
`PresentationMaterial.channels`, flags and original string values are retained.
`CharacterScene::texture(source_asset_id)` already resolves the original RX2
texture GUID to the owned decoded texture. `GpuRenderer::upload_native_texture`
already accepts that texture. Existing `render_inspection.rs` selects the source
channel named `diffuse`; scoped shader name `i_diffuse` is not the channel name.
Its first-match policy is not sufficient proof for native duplicate-channel
overwrite semantics.

The existing app `resolve_material_manifest` traverses exported Vault nodes,
parents/layout and relocated text, but reloads material data through app helpers.
Use the already retained runtime Vault instead. Its `try_get_data` implements
the required scalar node search but rejects relocated payload bytes; its
`try_cached_layout_text` resolves text only through the cached layout. Native
generic effect lookup requires a narrow pointer/text projection sharing the
existing node search and relocation logic, not a layout-only shortcut or a
second material Vault loader.

## Fresh native source

`RebindComponentToMaterial82d08fc0`: clone type check at `82d08fd4..82d08fe4`
enters mutation only for the two cloning policies. Selected clone3 returns
without rebind. Therefore retained recipe shader parameters must NOT override
the original RX2 channels in this selected packet. Earlier general material
rebind discussion in `character-gpu-skinning-source.md` does not override this
now-established selected policy.

`CreateEffectListFromPegasusMaterialSkinned82e20828` reads channel index1's
stream-name pointer at `82e2084c`, not a search for a metadata name. Null alone
selects `character.default_cloth_ropa` or `character.default_cloth` by flags bit0.
It creates an actual Vault instance, falls back again if the selected collection
is absent, then creates the effect list and applies original channels. Exact
fallback key operands need raw confirmation before implementation. The selected
non-null authored path can be admitted first, with a checked unsupported
boundary for parser representations that lost null-versus-empty provenance.

`CreateEffectListFromMaterialKey82e203f8` uses generic `Attrib::Instance::Get`
for effect at `82e2046c`, dereferences its text pointer, retains literal `default`
only when null, then reads DrawOrder/its referenced guid and builds effects.
Do not replace this with a table mapping component names to shaders. Existing
evidence records original `defaultcharacter`, `defaulthair`, and
`defaultcharacter_ropa` Vault strings; the new caller must resolve them live.

`AddPegasusMaterialToEffectList82e19f88` visits actual effects; each
`AddPegasusMaterialToScope82e17bb0` visits channels starting at index2 in order.
Flags bit1 selects scalar storage. Raster channels use their actual stream
pointer or supplied missing-texture owner. Every iteration calls
`SetParamPointerByHash` at `82e17d2c`; duplicate names therefore cannot be
silently resolved by first match. A selected diffuse adapter should retain
last applicable binding or explicitly refuse duplicates until checked. Missing
resident texture is a typed boundary unless the genuine missing-texture owner
is supplied; no white/checker texture is authorized here.

## Ropa is a separate retained producer

Current `SelectedPresentationStartup` contains animation, restored views and
twist attributes, but no `SkaterPresEntity::i_vtx_params` or cloth state.
Fresh constructor `827e5750` calls InitFromRecipe/InitSkinning, then at
`827e58b4..827e5944` compares actual schema to a generated marquee collection,
combines actual local-player and supplied cloth-disable state, and lazily samples
`cv_EnableRopa` before `InitCloth`. Only afterward `827e5960..827e59d0` inserts
X=1 in the retained vector. It does not initialize the other lanes there.

Existing independently recorded `Update827e8568` changes X to0 after successful
cloth initialization, then skins/waits/double-buffers real CPU vertex output.
The renderer already rejects missing X and the CPU branch. A material adapter
must preserve these errors. Creating a literal X=1 in the adapter would bypass
the missing constructor and ongoing state producer. The first actual selected
schema comparison, local flag and cloth path need a source-backed retained
producer before claiming all LOD0 parts render repeatedly.

## Proposed bounded implementation, pending root approval

1. New core material-admission child and tests: borrow finalized scene/palette
   identities; resolve exact model material, live Vault effect and original
   diffuse texture. Return retained indices/borrowed source data and typed
   unsupported branches, without geometry/texture copies or shader defaults.
2. Narrow shared runtime-Vault text projection reusing the scalar search; checked
   finalization handle access. Preserve null/empty distinction or restrict the
   selected entry explicitly where the current parser erased that distinction.
3. New app binding child: map only the actual supported manifest names to the
   existing `CharacterShaderFamily`, upload the borrowed actual texture, call
   existing selected GPU consumer. Ropa requires a real supplied presentation
   constant owner; no adapter-created value. Existing diagnostic fragment
   shading/hair-alpha limitations remain explicit.
4. Original-data test in the same mounted recipe completion chain verifies
   finalized membership, per-mesh manifest/channel/texture identity and exact
   failures. This is not a separate recipe selection or default scene load.

Suggested exclusive new paths (not created):
`crates/skate2-core/src/rendering/character_material_admission.rs`, its `_tests.rs`, and
`apps/skate2-pc/src/selected_character_material.rs`. Exact-path checks are required
before creation. Shared edits stay artifact patches; root owns integration.
Ropa constructor/update admission is an explicit follow-on dependency unless
root includes its bounded source-backed owner in the packet.

## Authorized implementation packet

Root approved the six unregistered Rust files and isolated shared patch. Current
artifact is `artifacts/character-material-admission-2026-09-22.patch`, generated
by the similarly named Python builder. Source review notes are independently
recorded in `artifacts/selected-material-source-review-2026-09-22.md`.

The producer retains the exact finalized scene owner via `Arc`; it is shared
from the mounted recipe load into the existing selected presentation startup.
No second finalization or scene decode runs. The material admission checks this
scene against the palette and checks its exact finalized instance membership.
Until the existing broader palette model selector supports additional cases,
an instance/model disagreement rejects admission. GPU creation now obtains
geometry from this checked binding, never from an arbitrary first model.

Fresh `Resolve82853eec..82853f30` confirms model table entry's first pointer is
the mesh, then mesh+36 is the material. The formats parser retains that field
as `OptimizedRenderMeshData.material` / `PresentationMesh.material`. The table's
second pointer, exposed as `PresentationModelMesh.material`, is not read here.
IDA's mesh-helper annotation on this load is misleading. No fallback between
the two parsed material fields is introduced.

The exact authored nonempty channel1 path is admitted. Missing collection or
null-versus-empty metadata ambiguity remains checked unsupported admission;
the native fallback strings above are not silently guessed from component names.
Generic effect text reads now reuse the SAME existing child/parent/layout node
search, validations and source pointer precedence. A new projection decodes
only the selected pointer field through the loaded relocation table. Plain-data
reads retain their relocation refusal. Cached-layout text uses the same text
reader, without gaining generic fallback. Null node text stops the search.

Diffuse projection starts at channel2 and retains the last matching scope write;
a later scalar does not resurrect an earlier raster. It resolves the original
GUID into the already decoded same-scene texture. There is no recipe override,
default texture, effect-name heuristic or new shader calculation.

### Actual ropa constructor inputs and store boundary

Fresh `UpdatePresEntity827e6298..827e630c` reads the full signed64 peer at
header+160 and sets supplied cloth-disabled when it differs from -1; it extracts
header+192 bit15 for locality. The implementation consumes these fields from the
same generation-checked Produced frame view. It does not resample the backend
or accept an invented cloth-disable answer. Subsequent peer/local replacement
requires an explicit unsupported replacement path rather than stale reuse.

Raw constructor argument construction at `827e589c..827e58b0` proves marquee
collection `2275b90050166969`, message-port0. The actual retained Vault cached
schema is compared before the local/disabled gates. Existing mounted assertions
in `physical_player_assembly_corpus.rs` resolve this same collection to
`tutorial_male`; the new caller still performs the live comparison to its actual
scene schema. Unit tests separately exercise a different schema with enabled
cloth and retain `disabled=false` before returning `EnabledClothPending`, with
X still unknown. The disabled/cvar-false branch alone reaches the native X=1
store. Other vector lanes are not fabricated or exposed.

One live cvar is added, index296: `cv_EnableRopa`, section `ROPA_Core`, title
`Enable ROPA`, default true, null callback. Fresh initializer `82f6bda0` calls
the bool constructor at `82f6bdcc`; table `82fd23b8`, reference `830530c0`.
It registers before existing table `82fd2414` index115. The manager's sorted
label oracle includes its section in the actual section/title order. Root owns
subsequent indices297 onward; camera295 belongs to the board packet.

### Existing GPU and mounted consumer

The app adapter maps only resolved `defaultcharacter`, `defaulthair`, and
`defaultcharacter_ropa` to the existing families, uploads the actual borrowed
texture and invokes the existing GPU consumer. Bindings can move separately
from `GpuObject` into the existing window vector. An allocation-identity token
shared with that object's existing character state checks object identity in
addition to palette identity, so another object is rejected even when it has
the same palette dimensions. No new render loop or pose owner is introduced.

The original selected frame probe installs materials from its retained
finalization source, calls the same production presentation update, then checks
actual manifest names, texture allocation identity and ropa parameter only
after successful construction. The production path does not call the probe.
The opaque unlit diffuse/hair-alpha limitations of the already approved renderer
remain unchanged and visible; this packet does not claim full material shading.

Formatting and shared patch application checks passed. Focused tests cover
parent-node text precedence/null stopping/plain-read refusal, schema/local ropa
cvar laziness, enabled-cloth partial state and live cvar-false admission, and
ordered diffuse binding. Author has run no Cargo or GPU tests. Independent source/code review approved
the final packet, including pre-construction refusal, signed64 peer admission,
text-pointer overlap checks, and same-object GPU identity. Root original-data
execution and GPU validation remain pending.

Reviewer-requested scope tightening: raw `GetAttribulatorKeyFromAttrMaterialName`
at `82e13420` asserts suffix length greater than1 before hashing. The selected
adapter now checks that condition, rejects embedded C terminators, and admits
prefix at most246/suffix at most255 bytes to remain inside source scratch space.
These are checked host bounds, not a claim of native malformed-input safety.

Repeated GPU publication is restricted to the actual retained
`mIsClothDisabled=true` constructor branch. A non-disabled constructor with live
cvar false still produces its correct X=1 leaf state, but the connected material
runtime returns `ClothUpdatePending`; it does not silently omit the later live
`Update827e8684` gate. This preserves the selected tutorial schema path without
claiming CPU cloth/update completion for other characters or settings.
