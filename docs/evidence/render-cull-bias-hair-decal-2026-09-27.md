# Culling, depth bias, the hair technique and `decalzbias` (2026-09-27)

**Status:** written, rustfmt PASS on every new hunk. NOT compiled or tested.

## Why

Z-fighting appeared almost everywhere: the scalp texture showing through the hat, and many walls. The sky also did not render properly.

The port drew every pass with culling off, no depth bias, and the hair as one opaque pass. The original does none of these.

## Evidence (IDA, debug `sk82_na_m.xex`)

**Per-pass reset.** `cFxParser::EndElement("Pass")` 0x82e1f778 rebuilds every parameter block after a pass is added:

| Block | Reset value |
|---|---|
| Blend | `cBlendStateDefinition` ctor 0x82e13db8: no blend, no alpha test (ref 0, func ALWAYS), ONE/ZERO ADD, RGBA write |
| Depth | `mZEnable` 1, `mZWriteEnable` 1, `mZFunc` 3 (`kStateGPUFunc_Less_Equal`), no stencil |
| Raster | cull 0 (NONE), bias 0.0, slope 0.0, fill 2 |
| Samplers (all 16) | WRAP/WRAP/WRAP, LINEAR mag and min, NEAREST mip, anisotropy 1 |

The parser ctor 0x82e1a4e8 starts from the same raster values.

**Parsing.**

- `cRasterizerStateHandle::SetState` 0x82e19d68:
  - CULLMODE goes through `ShaderSystem::CullMode` 0x82e15dd0. It accepts exactly `NONE` 0, `FRONT` 1 and `BACK` 2; anything else is a `FailAssert`.
  - DEPTHBIAS and SLOPESCALEDEPTHBIAS go through `ConvertAToF`.
- `cDepthStencilStateHandle::SetState` 0x82e19368: `DEPTHWRITEENABLE` is an alias of `ZWRITEENABLE`, and `DEPTHENABLE` of `ZENABLE`.

**Cull chain.**

- `cFxStateHandle<cRasterizerStateDefinition>::SetupState` 0x82e143c8 maps the cull value through `cullModeTranslation` 0x83024f8c = `[0, 2, 1]`. It copies both biases unscaled.
- `renderengine::Device::SetState` 0x82bcbf08 calls `D3DDevice_SetRenderState_CullMode` 0x82c34770. That inserts the value into PA_SU_SC_MODE_CNTL bits 0-2: CULL_FRONT, CULL_BACK, FACE (0 means counter-clockwise is the front face).
- So FRONT becomes 2: CULL_BACK with FACE 0, i.e. D3DCULL_CW. Screen-clockwise triangles are culled.
- BACK becomes 1: CULL_FRONT with FACE 0. Screen-counter-clockwise triangles are culled.

**Bias.**

- `D3DDevice_SetRenderState_DepthBias` 0x82c35338 stores the float unscaled in the FRONT/BACK_OFFSET shadows (device +0x2A54/+0x2A5C). That is an offset in normalized depth.
- `D3DDevice_SetRenderState_SlopeScaleDepthBias` 0x82c35270 stores value × 16 (+0x2A50/+0x2A58), in the register's 1/16-pixel unit. The per-pixel slope factor is therefore the manifest value.
- Both functions set the poly-offset enable bits.

**Projection.** The port's projection equals `ViewCamera::SetPerspectiveMatrixRadians` 0x82840770, including the mirrored x axis. Every triangle therefore keeps its original screen winding. wgpu `FrontFace::Ccw` names the same counter-clockwise screen order.

**Multi-pass order.** `SceneRenderView::RenderMeshes` 0x82850670:

1. Draws single-pass meshes in sorted order and collects the rest.
2. For pass p from 0 up to the largest pass count, draws pass p of every collected mesh that still has passes left.

## Manifests (`render.big`, all 62 `.xml`)

- **Values used anywhere.**
  - CULLMODE is FRONT ×82, BACK ×25 and NONE ×11.
  - DEPTHBIAS is only -0.0001 (4 passes).
  - SLOPESCALEDEPTHBIAS is only 1 or 1.0.
  - Every value parses, so the port's strict parser cannot fail a load.
- **The ported world effects and `sky`.** Their `default` pass is CULLMODE FRONT.
- **defaultcharacter(_ropa).** The `default` and both `ghost` passes are FRONT; `shadow` is BACK.
- **defaulthair `default`.** No pass enables alpha test. Every pass has STENCILENABLE with STENCILPASS REPLACE 255.

  | Pass | CULLMODE | Blend | Depth write | COLORWRITE |
  |---|---|---|---|---|
  | 0 | BACK | SRCALPHA/INVSRCALPHA (alpha the same) | FALSE | RGB |
  | 1 | FRONT | same as pass 0 | FALSE | RGB |
  | 2 | FRONT | ZERO/ONE | default (on) | 0 |

- **decalzbias `default`.** FRONT, DEPTHBIAS -0.0001, SLOPESCALEDEPTHBIAS 1.0, SRCALPHA/INVSRCALPHA, alpha test GREATEREQUAL 16, Z write on, COLORWRITE RGB.
  - Its samplers match baseenvironment's: the diffuse family WRAP, lightmap and chromaticity CLAMP.

## Shader sources (UPDB HLSL)

**`defaulthair.fx`.**

- `VS_IN` has `UV1 : TEXCOORD1`, and `Out.UV = half4(In.UV0.xy, In.UV1.xy)`.
- `vDiffuse.a = tex2D(i_alpha, In.UV.zw)`. Truncating the fetch to a scalar keeps its first lane.
- `outcolor.a = vDiffuse.a * i_params[0].x`.
- `sampler i_alpha;` has no `sampler_state`, and the manifest has no `<Sampler>` block for it. It therefore gets the reset sampler.

**`decalzbias.fx`.**

- `defaultVS` is baseenvironment's body without `Out.dataconstants`, which its pixel program never reads.
- `defaultPS` runs:
  - `GetDiffuseTextureValue` (rgb²);
  - the specular masks, and `GetNormalMapValue` (2·tex−1, unnormalized);
  - `normalize(mul(n, tbn))`, and the CSM-gated `GetShadowedLightMap`;
  - `GetTangentLight(vNormal, (0.64,0.64,0.420465), …)`;
  - `GetCalculatedSpecularTerm` (fresnel LUT.g×5 × masks.r × phong(10 + masks.g×290) × (18,15,10.5) × sat(lum − 0.1));
  - `outcolor = half4(diff,1)·cDiffuse + half4(spec,0)`, then `PS_Fog_Bloom_Tone(outcolor × m_params[0].y)`.
- The final alpha is `cDiffuse.a`.

## Data checks (offline)

- **Hair materials.** Every hair material has, after the two metadata channels, `diffuse, alpha, normal`: `character.hair` ×104, `character.default_hair` ×36, `character.hair_ropa` ×12 (`createacharacter.big`).
- **Hair vertices.** Every Hair-folder vertex descriptor has TEXCOORD1 as FLOAT2, which the port already decodes.
- **tutorial_male.** Its Hair material is `character.default_hair`, with alpha GUID 0x52e103e387e6. `tutorial_male.recipe` lists that GUID, so the texture is resident.
- **Texture formats.** Character textures are DXT5 ×2812, DXT1 ×45 and A8R8G8B8 ×2. The uploader supports all of them.

## Changes

1. **`gpu_renderer/raster_state.rs` (new).**
   - `CullMode` and `RasterState`: FRONT maps to (`Ccw`, cull `Back`), BACK to (`Ccw`, cull `Front`), NONE to no culling.
   - Bias constant = round(DEPTHBIAS × 2^24). Depth32Float's resolvable step is 2^(e−23), which is 2^-24 for every depth in [0.5, 1). The slope is as authored, with no clamp.
2. **World effects.**
   - All six ported effects cull FRONT.
   - `decalzbias` is ported as `EnvironmentEffect::DecalZBias`:
     - channels: diffuse, specular, normal, lightmap, chromaticity;
     - inputs: `lm_norm` plus TANGENT;
     - states: the manifest's blend, bias and write mask, with the alpha test in the program.
   - Shared WGSL moved without change: `phong_specular` and `tangent_light` go into `COMMON`, and the baseenvironment VS into `ENVIRONMENT_VS`.
3. **Diagnostic map path.**
   - Every (manifest, technique) carries its pass `RasterState`. Pipelines are cached by (pixel program, state).
   - A multi-pass technique whose passes differ keeps the pass-reset state: one draw cannot reproduce them.
4. **Characters.**
   - The default and ghost passes cull FRONT.
   - defaulthair draws its three passes with the manifest states. `fs_hair` alpha = `i_alpha.r` (UV1) × `i_params[0].x`, over the diagnostic colour.
   - `i_alpha` uses the reset sampler.
   - Core `CharacterMaterialView.alpha` resolves the hair's "alpha" channel texture, and is required for defaulthair.
5. **Draw order.** Single-pass parts draw first, then every multi-pass part pass-major, as in `RenderMeshes`. This now covers any pass count.

## What to check on the next run

- **Coincident surfaces.** Back-to-back coincident surfaces, such as thin walls and signs, should stop z-fighting: only the side facing the camera draws.
- **Hair.** Hair cards lose their transparent area and blend. If the scalp showing through the hat was the opaque hair cards, it is gone.
- **Decals.** `original environment shading draws=` should now include `DecalZBias`. Decals should be lit, alpha-tested, blended and pulled toward the camera.
- **Inside-out scene.** If the whole scene looks inside-out (only far faces visible), the cull mapping in `raster_state.rs` `primitive()` is inverted. Report it; it is a two-line swap.
- **Sky.** `Sky` should appear in the draws line, drawn from inside the dome with FRONT culling as in the original.

## Deviations and limits

- **Stencil.** There is no stencil attachment, so the hair and ghost STENCILPASS REPLACE 255 writes are not reproduced. Nothing reads them yet.
- **Bias range.** The constant bias is exact only for depths of 0.5 and up, i.e. farther than twice the near plane. Nearer primitives get a proportionally smaller constant.
- **Slope scale.** The slope term is per pixel at the port's resolution, as the hardware defines it. It is smaller than at 720p.
- **Hair shading.** Hair colour is still the diagnostic unlit diffuse. Only its coverage and pass states are original.
- **Unproven root cause.** The scalp/hat root cause is not proven. Two non-original behaviours were removed: no culling, and opaque single-pass hair. If the artifact remains, the next step is to identify the mesh involved.
