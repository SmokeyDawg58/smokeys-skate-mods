# City rendering and streaming audit

Work in progress, solo. Priority after walking corrections: moving city
textures/buildings and foliage alpha, with original-source fidelity.

## Confirmed and corrected shader gaps

The live view uses `Inspection`, whose map shader previously rendered all
textured geometry opaque with depth writes and no alpha rejection. The original
final manifests for tree, treeanimate and alphatestdefaultenvironment specify
ALPHATESTENABLE=true, ALPHAFUNC=GREATEREQUAL, ALPHAREF=30, ALPHABLEND=false.
The current change retains per-technique references from the parsed original
manifest and shares one GPU pipeline per reference. Only these source-reviewed
diffuse-alpha shaders enter this path; other effects are not guessed.

Original UPDB embedded HLSL confirms tree.fx lines24/43 and treeanimate.fx
lines28/47 restore unmodified diffuse.a after lighting and tone mapping.
Alphatestdefaultenvironment.fx lines23/33/46 likewise returns diffuse.a.
IDA D3DDevice_SetRenderState_AlphaRef82c34c30 stores Value*0.0039215689.
The WGSL rejects alpha strictly below this threshold before depth writes.

The same original VS source confirms tree and alphatestdefaultenvironment use
P*VP directly; treeanimate adds wind to world-space position then multiplies VP.
The diagnostic view had applied an extra instance transform to these effects.
They now use their original world-space convention. Wind remains unimplemented.

Evidence extraction reuses BIG/RefPack/ShaderManifest production decoders:
`tools/inspect_shader_assets.rs`. Original UPDB bytes are in the OS temp folder,
not copied into the repository. Manifest census:
`artifacts/city-material-manifests-20260925.log` (124 debug/final manifests).

Validation: release build PASS (`city-alpha-build-20260925.log`), Vulkan window
render/capture PASS exit0 (`city-alpha-runtime-20260925.log`,
`city-alpha-20260925.png`). Captured foliage has cutout silhouettes. This is
not full original material parity: lighting, fog, shadows, material color masks,
native culling/stencil, authored samplers and remaining effect families still
need their own source-backed implementation.

Final instrumentation build PASS1m36 (`city-alpha-final-build-20260925.log`),
final Vulkan capture exit0 (`city-alpha-final-runtime-20260925.log`, matching PNG).
109 live draw objects use the source-reviewed alpha-test pipeline. No gameplay
logic was changed in this renderer packet.

## Main unresolved city residency gap

`live_skating_client` loads one fixed RobsPark Inspection scene. `live_skating_view`
updates camera and character only. Collision focus movement does not update this
map inventory. In the captured scene 1189 draws have diffuse textures and 896
remain diagnostic solid because their diffuse resource is not resident.

Existing `main.rs` move_runtime_presentation_scene/move_runtime_texture_scene
already implement inventory transitions. Do not create duplicate loaders or
mislabel these inventory probes as native Presentation registration. They reread
XSF independently, retain distance-indexed handles and include instances before
native registration/unlock admission. See renderer-first-material-path.md's
resident presentation continuation and proxy-pres-kind0-kind2-production-audit.

Further missing native behavior: PROXY low/global texture lifetime, high/low
shader texture table transitions, material registration's low/high/fallback
lookup, native processed-arena order and environment admission. Existing drafts
include proxy_shader_texture_table_draft.rs, proxy_kind0_resident_builder_draft_20260923.rs
and proxy_shared_loaded_arena_draft_20260923.rs; inspect/reuse before implementing.

Fresh shader-table review: AddTexture82e1b058 and RemTexture82e1b548 match the
existing draft. Raw PPC82e1b0bc/82e1b554 clears only bit0 of the full 64-bit ID;
82e1b118 tests that low bit. Do not follow the decompiler's corrupted high-half
expressions. Existing draft's four standalone Rust tests PASS
(`city-texture-table-tests-20260925.log`). It remains unregistered and is not yet
the live renderer's texture binding table. The kind0 resident-builder draft
expects a GpuTextureUploader service that does not currently exist; reuse/extract
the existing native upload implementation rather than duplicate texture decoding.

Fresh IDA: Pres_LocalPlayerStreamerController::UpdateStreamer8277d3d8 uses the
local SkaterPresentation GetSkaterRoot translation when time-controller mode
allows, then MoveStreamFocus(position, owner, false,255). CameraStreamerController
8277d5f0 is a distinct controller using CameraPresentation matrix translation.
MoveStreamFocus824d8250 gates the disable cvar/NaNs, then flags1/2/4 move the
existing presentation/simulation/texture foci. Do not substitute camera position
for the local-player controller or infer streaming coordinates from skinned hips.

Next: complete native retained resource/publication integration and validate a
city movement/return route, including stable GPU handles and memory lifetime.
The broader walking audit remains unproved beyond documented foot-IK/turn fixes.
