# Original world-environment shading, phase 1 (2026-09-25)

Status: source written and statically reviewed; rustfmt parse PASS on every
touched file. NOT compiled, NOT tested, NOT run.

## What changed

Map draws whose effect is `defaultenvironment`, `alphatestdefaultenvironment`
or `baseenvironment` (technique `default`) now render through ports of the
shipped pixel/vertex programs: baked lightmap and chromaticity, the
CSM-shadowed lightmap term, fog, tonemap and gamma-2 output. For
baseenvironment they also include normal and detail mapping, macro overlay,
specular/eccentricity masks, the fresnel lookup, the reflection cube and phong
specular. All other effects and any draw missing an input keep the previous
diagnostic path. That path is now raw diffuse, because the surface is non-sRGB.

## Reference sources

- Shaders: `render.big` `data/render/shaders/final/*.updb`. `<file id="0">`
  holds the preprocessed HLSL (extractor: `tools/updb_source.py`).
  `default` technique programs: `defaultenvironment_default{VS,PS}` and
  `alphatestdefaultenvironment_default{VS,PS}`. `baseenvironment_default{VS,PS}`
  is the source with `lm_norm` and the `xy_len` clamp; the other two
  baseenvironment sources are the `lod` and `optimizedVS` programs and are not used.
- Helpers ported in source order: `common_shadows.fxh` (CSM_Shadow,
  CSM_ShadowMap_PS), `environment_lighting.fxh` (GetShadowedLightMap with
  SingleTap, GetTangentLight, GetFresnel_TextureLookup, GetReflectionTerm with
  `ref_vec.xy *= -1`, GetCalculatedSpecular, GetSpecularTerm),
  `common_lighting.fxh` (phong_specular), `environment_textures.fxh`
  (ApplyOverlay), `postfx_functions.fxh` (ComputeFogFactor, tonemap,
  PS_Fog_Bloom_Tone, MoveFromLinearSpace = sqrt). The literal arguments are
  the compiled call-site constants (e.g. multiplier (4.5,4.5,4.5,1.5), shadow
  value (0.07,0.09,0.1,0.04), ramp 50, bias (-0.005,10,25), specular
  (8,1,5)/0.1/(18,15,10.5)/290/10/1.5/(0.3,4,1.5)).
- Manifest states: alphatest COLORWRITE RGB, ALPHAREF 30 GREATEREQUAL.
  Lightmap and chromaticity samplers use CLAMP; the diffuse family uses WRAP.
  The cube map uses CLAMP, LINEAR min/mag and POINT mip. The fresnel texture
  has no sampler block, so it uses D3D defaults (POINT, WRAP).

## IDA (debug `sk82_na_m.xex`)

- `AddPegasusMaterialToScope` 0x82e17bb0. Channels are processed from index 2
  (0/1 are `Name` and `AttribulatorMaterialName`). A flag-bit-1 channel is an
  inline float4 bound as `i_<name>`. A name starting with "li" sets
  `i_monoLightmap_Dot = LightMapDot[image_channel & 3]`, where LightMapDot is
  (1,0,0,0) (0,1,0,0) (0,0,1,0) (0.33,0.34,0.33,0).
- `DispatchParamsHelper` skips `SetTexture` for a null `cTextureParam`, so an
  absent channel keeps the previous draw's texture. That is not reproduced:
  such draws stay on the diagnostic path, and the reason is logged.
- `cMaterialScope::Resolve` 0x82e17a98 binds `m_params` from the material
  class/variant collection. For `material_environment[simple]`,
  `m_params[0]` = (0.4, 1.0, 0, 0); the shaders read `.y`.
- `GraphicsMan::Init` 0x8273f6e8: `g_envattributes` = `material_envattributes/default`
  (7 float4). [2].x = 1.0 is the exposure; [5] = (8,1,5,1.5) drives the fresnel table.
- `RenderMan::CreateFresnelLookupTexture` 0x82833a68 and
  `UpdateFresnelLookupTexture` 0x8282a528 build a 256x1 A8R8G8B8 texture.
  For each i: t = 1 - i*(1/255) (fnmsubs, 0x3b808081); v = facing +
  (edging - facing)*t^exp; byte = trunc(v*255/edging); word = byte*0x10101 - 0x1000000.
- `RenderMan::UpdateRenderParams` 0x82852780:
  - `g_vLightDir` = normalized location sun vector (-0.41, 0.4, 0.8).
  - `g_vViewPos` = CameraMan translation.
  - `g_FogK1` = (1/(end-start), -start/(end-start), power, 0).
  - `g_FogColour` = (rgb*strength, -strength), from stvx at GlobalShaderData+0x130.
- `LooseWorldPresentation::GetWorldRenderLocation` 0x827d0cc8: with no
  explicit location, it uses the key low word 0x105b866b (`render_locations`).
  Its `material_fog` reference is `material_fog/default`: colour
  (0.067308, 0.13373, 0.269231, 1), power 1.4, start 50, strength 1, end 300.

## Routing (render_inspection.rs)

A draw uses the original effect only when all of the following hold. Otherwise
the fallback reason is counted and printed as `ported-effect draws kept diagnostic by reason=`:

- the manifest is one of the three effects and the technique is `default`;
- every sampler channel the effect reads is a raster that resolves through the
  shader texture table (active slot);
- `environment` is a cube texture and the other channels are 2D;
- the flag-bit-1 scalars (baseenvironment: macroOverlayUVScale,
  detailNormalUVScale, macroOverlayOpacity) are present;
- an "li*" selector exists;
- `m_params` resolves;
- the mesh has UV0, full TEXCOORD1 (float4 for baseenvironment) and, for
  baseenvironment, TANGENT.

Material bind groups are cached by (effect, stable texture identities) across
city generations and retained only while used.

## Deviations (documented, not silently approximated)

- The CSM atlas is bound empty (1x1, moments (1,0)): CSM_ShadowMap_PS
  returns exactly 1. No dynamic casters (skater, peds, cars, DMOs) are
  rendered into it yet. The CSM rows are zero.
- The `SunLightMan` sun-direction map is not sampled. The location requests it
  (layout byte 3) and `kEnableSunDirectionMap` defaults to on, so the fixed
  location vector is used as the fallback.
- No WorldPainter blending over time and no explicit `cMsgSetWorldRenderLocation`.
- `half` is evaluated as f32 (Xenos ALUs are f32). There is no
  TRILINEARTHRESHOLD equivalent. Anisotropy is 16 instead of the manifests' 4
  (a PC enhancement).
- Culling stays off: `CULLMODE=FRONT` winding under this projection is unproven.
- No PostFX (bloom, colour cube, auto exposure). The effects' own gamma-2
  output is presented directly on a Bgra8Unorm/Rgba8Unorm surface.
- The surface prefers the Opaque composite alpha mode, because defaultPS
  writes alpha 0 on purpose.

## Files

- `apps/skate2-pc/src/gpu_renderer/environment_shading.rs` (new): effect
  enum, WGSL ports, pipelines and layouts, samplers, fresnel and empty-CSM resources.
- `apps/skate2-pc/src/render_environment.rs` (new): Vault reads for
  envattributes, the world location sun/fog and m_params; fresnel/fog/light packing.
- `apps/skate2-pc/src/gpu_renderer/map_draw.rs`:
  - 160-byte instance record (`InstanceMaterial` lanes after world+colour);
  - 208-byte globals (`pack_globals`);
  - `push_environment`, lighting vertex slot 2 (deduplicated), resources at group 2;
  - `set_view(clip, view_position)`.
- `apps/skate2-pc/src/gpu_renderer.rs`:
  - non-sRGB surface and Opaque alpha;
  - cube texture upload (6 layers, face = array layer, D3D face order);
  - TEXCOORD1/TANGENT stream on `GpuMesh`;
  - globals upload; environment resource/material builders;
  - diagnostic fs returns raw diffuse.
- `apps/skate2-pc/src/gpu_renderer/character.rs`: diagnostic fs returns raw
  diffuse (CRLF preserved).
- `apps/skate2-pc/src/render_inspection.rs`: texture resolver, material
  routing, fallback accounting, `set_external_view`, baseenvironment world-space.
- `apps/skate2-pc/src/live_skating_view.rs`: passes `camera_world_position`.
- `crates/skate2-core/src/session/selected_session_public_facade.rs`:
  `SelectedVisualSnapshot::camera_world_position` (CameraMan translation).
- Tests:
  - `map_draw_tests.rs`: record/globals layout, plus a GPU validation test that
    builds every pipeline, a cube, the materials and one real frame, and is
    skipped without an adapter;
  - `environment_shading.rs`: vertex lane layout;
  - `render_environment.rs`: fresnel endpoints, fog lane packing, unit light;
  - `gpu_renderer.rs`: cube level/face order validation.

## What to look for on first run

- `original environment lighting: g_vLightDir=... g_FogK1=... g_FogColour=...`
  once at load.
- `original environment shading draws={Default: n, AlphaTest: n, Base: n}` and
  `ported-effect draws kept diagnostic by reason={...}` per map generation.
  The reasons name exactly which channel, scalar or stream is missing.
- The diagnostic (non-routed) look shifts slightly: raw diffuse on a UNORM
  surface instead of squared diffuse on an sRGB surface.
