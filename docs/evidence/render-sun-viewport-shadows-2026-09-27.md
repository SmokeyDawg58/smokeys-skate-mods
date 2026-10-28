# Sun direction, widescreen viewport and dynamic shadows (2026-09-27)

Status: source written and statically reviewed; rustfmt PASS on every touched
file. NOT compiled, NOT tested, NOT run.

Reference: debug `sk82_na_m.xex` in IDA (the only truth for addresses and
math), the shipped `render.big` shader containers (`.fpo`/`.vpo` microcode and
`.xml` manifests), and `skater.vlt`/`skater.bin`.

## 1. Sun direction per frame (SunLightMan)

- `RenderMan::UpdateRenderParams` 0x82852780 reads the location sun vector,
  then (`kEnableSunDirectionMap` = 1, location layout byte 3 set) overwrites it
  with `SunLightMan::GetSunPosition` 0x82851f20 before the normalize into
  `g_vLightDir` (GlobalShaderData +0xC0). The sample point is
  `SkaterPresentation::GetHipsPos` 0x827e3ef0 for the local skater, the camera
  without one.
- Map: `rendering` collection 0x1c70490308c534f2; texture name attribute
  0xe1d4c5c744e84dd8 = `sun_bam`; world rectangle min 0xfb5b9bfea84b1155 =
  (-2840, -2462), max 0x71b0a3a189b8bd3f = (1774, 1345).
- Texture: `miscboot.big` `data\content\textures\globaltextures.rx2`, TOC
  entry `sun_bam.Texture` (GUID `RwHash64String`), 64x64
  `PIXELFORMAT_LIN_A8R8G8B8` (405274758), untiled; texel bytes [A, R, G, B].
- `SampleBilinear` 0x82826fc0 with its FMA order, then the decode
  `c * (2/255) - 1` (`fmsubs`, operand 0x3c008081).
- Hips: `GetHipsPos` accumulates the DataPostBlend root rows scaled by the
  saved Hips translation row with `vmaddfp`, row 0 first. The core publishes
  it with the selected frame (`SelectedVisualSnapshot.skater_hips_position`).
- Port: `render_environment.rs` `SunDirectionMap`, sampled every frame in
  `render_inspection.rs`.

## 2. Widescreen viewport

- `ViewCamera::SetPerspectiveMatrixRadians` 0x82840770 hard-codes aspect
  1.777...; the original always renders one 16:9 target.
- Port (`live_skating_view.rs` `window_clip_from_world`): the core projection
  stays 16:9. A wider window scales clip x by (16/9)/aspect (the vertical field
  is the original's, the horizontal field widens); a narrower window scales
  clip y by aspect/(16/9). Only clip x or y changes, so depth is untouched and
  16:9 windows are bit-exact. Pixels stay square at any window shape.
- This is a deliberate enhancement: the original has no other aspect.

## 3. Dynamic shadows (CSM)

### 3.1 Where it runs

`RenderMan::RenderModeScene` 0x82857a78 calls `UpdateRenderParams`, then
`PostFX::RenderShadows_CSM` 0x82855888 (PostFX at RenderMan +0x6E0) when
`kRenderShadows`, before the scene. `Shadows_CSM_Init` 0x8283ec80 creates:

| Target | Size | Format | Role |
|---|---|---|---|
| `m_pShadowMap_Z` (+0xA58) | 512x512 | D24S8, readable as texture | caster depth + stencil |
| `m_pShadowMap_HBLUR` (+0xA5C) | 512x512 | G16R16 (EDRAM G16R16), LINEAR | horizontal filter |
| `m_pShadowMap_ATLAS` (+0xA60) | 1024x512 | G16R16 | two cascades side by side |
| `m_pWorldShadowMap` (+0xA64) | 512x512 | G16R16 | world onto characters |

`RenderMan::RenderMan` 0x82854800 binds `shadowTexture_CSM0` to the ATLAS
texture (+0x1F8) and `shadowTexture_World` to the world map.

### 3.2 Cascade matrices (gameplay path)

Decoded with a symbolic PPC/VMX128 walker over the function (IDA displays the
VMX128 VA register with its two high bits swapped; the walker decodes the raw
fields). Gameplay path = not `m_isNIS` (RenderMan +0x2B4B0), no front-end
CSM attributes (+0xB80), not `m_bInLooseWorld` (+0x2B4CD), a local skater:

1. Light basis from `g_vLightDir` (GfxMan +0x1B240): `f = normalize(-L)`,
   `s = normalize(-f.z, 0, f.x)`, `u = normalize(f x s)` (rsqrte + two
   Newton steps each).
2. Cascade `c` in {0, 1}: `S = ScaleLevel[c] * A`, `ScaleLevel` 0x82ff023c =
   {1, 4, 16, 16} (only read), `A` = `m_CSMAttribs` attribute
   0xc1e51168dd06bd38.
3. Centre = `fma(camZ * S, D, camW)` per lane, from
   `CameraPresentation::GetMatrix` rows 2 and 3; `D` = attribute
   0x62c03852c5f4d2ab.
4. `Inv = rw::math::vpu::Inverse([s; u; f; centre])`; x and y columns scaled
   by `1/S` (`fdivs`); the translation lanes snapped
   `floor(v * 256) * (1/256)` (one texel of a 512 map = 2/512).
5. The depth column becomes `(0, -1/R, 0, snap(maxH/R))`: `maxH` =
   `SkaterPresEntity->m_BB.m_Max.y + 2.0` (0x82855cd4), `R` = layout +0x24.
6. `ShadowView::Cull(c, VP)` 0x82850ac0, then `SetViewMatrix(identity)`,
   `SetProjectionMatrix(VP)`, `ShadowView::Render(c)` 0x82854d28.
7. The VP columns x, y, z are stored as `CSM_Mat_Row0/1/2[c]`
   (PostFX +0xB50/+0xB54/+0xB58 point into GlobalShaderData +0x180..).
8. After both cascades (0x828578a4..7994): `B = Inverse([Row0[0]; Row1[0];
   Row2[0]; (0,0,0,1)])`, then `Row0[1] = (C0.B[:,0], C1.B[:,1], C0.B[:,3],
   C1.B[:,3])` with `C` = cascade 1's rows: the cascade 0 -> 1 scale (xy) and
   offset (zw) the receivers use.

Vault (`rendering` collection 0xb9a92367e7ac68b9, bound by `PostFX::PostFX`
0x82839748, no parent):

| Key | Value | Use |
|---|---|---|
| 0xc1e51168dd06bd38 (inline) | 1.8 | cascade 0 half-extent: cascades span 3.6 m and 14.4 m |
| 0x62c03852c5f4d2ab | (1, 1, 1) | centres S metres along the view axis |
| layout +0x24 `nis_depth_precision` | 5.1 | depth range |
| 0x9c8706768cfcbb4a / 0x0fb8d6f785615ebe | 1.5 / (0, 0, 0) | world-shadow pass only |

`m_localBB`: `PresentationEntity::CalculateBoundsInfo` 0x827f2708 unions the
LOD 0 model boxes; the `SkaterPresEntity` constructor 0x827e5a30..5b2c lowers
both y lanes by 1.0 (`gSwizzleStoreConstants[5]` = replace lane y) and copies
the box to `m_localBB`. `SceneRenderView::Cull` 0x828501c0 sets each frame
`m_BB = m_localBB + m_MatLtoW row 3`; `DoSimRestore` 0x827e7160..7248 gives the
skater `m_MatLtoW = mHipsTransform x root`, whose row 3 is the hips position.

### 3.3 Casters

- `SkaterPresentation::Init`, `Dmo::PresManager::Init`, `CASPresManager`,
  `cLivingWorldPresEntityManager`, `MVManager` and `SceneCharacterManager`
  register with `eShadowView` (3); `VIPresentation` (vehicles) does not;
  `WorldPresentation` registers with `eWorldShadowView` (4).
- `ShadowView::Sort` 0x8284ce50 copies the entity map each frame. `Cull` keeps
  an entity for cascade `c` when `IsInLightView` 0x82829b88 (eight `m_BB`
  corners, x/y against +-1; outside when all past one edge, inside when none
  past any) is not outside and `|GetPosition - camera|^2 < 1300`
  (`maxShadowDistSquared`); `GetPosition` 0x827d79c0 is `GetHipsPos`. An
  entity completely inside cascade 0 is erased before cascade 1.
- `Render` sets each entity's technique to `shadow`. defaultcharacter,
  defaultcharacter_ropa and defaulthair: `simpleVS` (the default skinning into
  `g_matVP`) + `shadowPS`; states STENCILENABLE, STENCILREF 255, STENCILPASS
  REPLACE (and CCW_), STENCILMASK/WRITEMASK 255, COLORWRITE 0, CULLMODE BACK,
  over the pass reset (ZENABLE, ZWRITE, LESSEQUAL).
- The Z target clears with `GfxMan->mClearDepthStencilParams` (depth 1.0 at
  0x8282c6d4; stencil 0).

### 3.4 Filters (Xenos microcode)

The PS containers (0x8212f4e0 `Variance_HBlur_PS` 0x268 B, `Variance_VBlur_PS`,
`_3_` and `_0_` variants, `depthsampler_PS`) hold `ps_3_0` microcode at
container +0x118 with four literal vec4s (c252-c255) before it. Disassembled
with a Xenos decoder (Xenia `ucode.h` field layout; scalar two-operand ops take
src3 swizzle components (w-slot, x-slot); `mulsc/addsc` take the constant from
the src3 register field). Cascade 0 uses `[0]`, cascade 1 `[1]`
(PostFX +0xA70/+0xA7C arrays); `_0_` is not used here.

- Taps: `tfetch2D` offsets in half texels: x = -2, -1, 0, +1, +2 (5-tap), y
  likewise for the vertical pass; 3-tap variants use -1, 0, +1. The samplers
  are the HBLUR target's (CLAMP) at texel centres (`RenderQuad` uvOffset =
  0.5/size).
- Depth decode: `dp3(t.ywx, (255/2^24, 255/256, 255/65536))` of the D24S8
  texture = the 24-bit depth; coverage = the stencil byte (t.z) / 255.
- `Variance_HBlur_PS`: depth = the centre's when its coverage > 0
  (`setp_gt` + predicated move), else `sum(D_i*C_i) * rcp(sum C_i)` over the
  four neighbours; coverage = (C(+2)/3 + 2C(+1)/3 + C(-2)/3 + 2C(-1)/3 + C0)
  / 3, i.e. the tent (1 2 3 2 1)/9.
  Out = (depth, coverage).
- `Variance_HBlur_3_PS`: two neighbours; coverage (3,8,3)/14 (c254.w =
  0.0714285746).
- `Variance_VBlur_PS`: neighbours count when `saturate(c * 9999999)`;
  coverage (2c-2 + 4c+1 + 6c0 + 4c-1 + 2c+2) / 18.
- `Variance_VBlur_3_PS`: (3,8,3)/14.
- With no counted neighbour `rcp(0) = inf` and the SM3 multiply gives 0, so
  the depth is 0 (lit for any receiver).
- `quad_transform_VS` 0x8212b0c0: `oPos = pos * posScale(c0) + posTrans(c1)`,
  uv = uv * uvScale(c3) + uvTrans(c2). PfxHelper constants: posScale
  (0.5, 1, 1, 1), posTrans (c - 0.5, 0, 0, 0), uvScale 1, uvTrans
  (0.5/512, 0.5/512). The ATLAS resolve rects are x [c*512, c*512+512),
  y [0, 512) to (c*512, 0).

### 3.5 Receivers

- The shipped `defaultenvironment_defaultPS.fpo` (and alphatest, base, decal,
  transparent) compute the atlas UV as cascade 0 `(x*0.25 + 0.25,
  y*-0.5 + 0.5)`, cascade 1 `(x*0.25 + 0.75, y*-0.5 + 0.5)`; literal
  c253 = (0.25, 0.5, -0.5, 4.5). The `.updb` HLSL text (`(-0.25, 0.75)`) is not
  what shipped; it would sample only the bottom half of a 1024x512 atlas. The
  port follows the microcode.
- Shadow term (pc 17-21): `saturate(step(z + 0.005, m.x) + (1 - m.y))`.
- Sampler: `cFxParser::EndElement("Pass")` 0x82e1f8e8..f954 binds the named
  `ShadowTextureSampler` (`AddDefaultSamplerStates` 0x82e18cbc..8d08): BORDER
  u/v, zero border, WRAP w, LINEAR min/mag, no mip.
- Every other sampler without a manifest block gets the pass's reset
  definition (0x82e1fb60..fbc4: WRAP, LINEAR min/mag, POINT mip). The port's
  `g_fresnelLookupTexture` sampler was POINT ("D3D defaults"); now LINEAR.

## 4. Port

| File | Change |
|---|---|
| `apps/skate2-pc/src/render_shadows.rs` (new) | Cascade rows, cascade 0 -> 1 transform, `IsInLightView` + distance culling, general 4x4 inverse; unit tests |
| `apps/skate2-pc/src/gpu_renderer/shadow_map.rs` (new) | Z (Depth24PlusStencil8), HBLUR (Rg32Float), ATLAS (Rg16Float); caster pipelines (stride 28/36), the four filters as WGSL, per-frame recording |
| `apps/skate2-pc/src/gpu_renderer/character.rs` | Skinning split into shared `SKINNING` WGSL; `row_layout()` |
| `apps/skate2-pc/src/gpu_renderer/environment_shading.rs` | Receiver UV fix, `ShadowTextureSampler`, fresnel pass-default sampler, live atlas binding |
| `apps/skate2-pc/src/gpu_renderer/map_draw.rs` | `FrameEnvironment.shadows`; CSM rows packed into the globals |
| `apps/skate2-pc/src/gpu_renderer.rs` | Shadow passes recorded before the main pass; clamp-to-border feature requested when present |
| `apps/skate2-pc/src/render_environment.rs` | `CsmAttributes` from the Vault |
| `apps/skate2-pc/src/render_inspection.rs`, `live_skating_view.rs` | Camera forward/position, hips and skater bounds into the cascades |
| `crates/skate2-core/src/assets/character.rs` | `CharacterScene::level_bounds` |
| `crates/skate2-core/src/rendering/skater/skater_presentation_selected_runtime.rs` | `m_localBB` at construction, world `m_BB` per frame |
| `crates/skate2-core/src/session/selected_session_public_facade.rs` | `skater_world_bounds`, `camera_forward` on the snapshot |

Casters in the port: every character part of the local skater (the only
entity the port renders). DMOs, pedestrians and other skaters are not rendered
yet, so they cannot cast; they need their `shadow` techniques and culling
when they arrive.

## 5. Deviations and limits

- Formats: G16R16 becomes Rg32Float (blur) and Rg16Float (atlas; filtered by
  the receivers). The Xenos EDRAM 16-bit precision was not modelled.
- The D24S8 target is read by aspect (depth + stencil) instead of decoding
  its bytes from an 8888 view; same values up to depth quantization.
- `Inverse` is a textbook cofactor inverse; `rsqrte` + Newton steps are
  `1/sqrt`. Differences are at the ULP level.
- Camera: the port's provisional `CameraMan` producer matrix stands in for
  `CameraPresentation::GetMatrix` (as for `g_vViewPos`).
- Adapters without clamp-to-border clamp to the edge texel.
- The stencil is read through a stencil-aspect view. Vulkan (the
  adapter in every saved log) returns it in `.r`. wgpu-hal 0.19.5's
  DX12 backend creates that view as `X24_TYPELESS_G8_UINT` with the default
  component mapping, where D3D12 returns the stencil in `.g`; on DX12 the
  coverage would read 0 and no shadow would show (no crash). Unverified.
- Not ported: the world-shadow pass (`m_pWorldShadowMap`,
  `CalculateWorldShadow`) and character receivers (`CSM_ShadowMap_Self_PS`),
  because character lighting is not ported; the NIS / front-end /
  loose-world variants. `PostFX::RenderAOBoxes` 0x828465b0 (the deck AO box
  under each skater, a post-FX pass: multiplicative `m_pTex_DeckAO` projected
  by `CalculateDeckShadowMatrix`), separate from the sun shadows, was ported
  later (2026-10-21, docs/GRAPHICS.md "Deck AO").

## 6. Tests added

- `render_shadows`: inverse round trip; cascade extent maps to +-1 within one
  texel; depth row = height below `maxH` over `R`; `Row0[1]` reproduces
  cascade 1; a skater inside cascade 0 is dropped from cascade 1 and a far
  one from both; `IsInLightView` outside/clipping/inside.
- `shadow_map`: caster clip-from-world layout and the globals order.
- `map_draw_tests`: globals packing with the CSM rows.

Not run (no cargo in this workflow). The `render_shadows` assertions were
checked with a float32 NumPy mirror of `cascades`; all hold (the tightest,
cascade 1's snapped origin, by 0.00018 against 2/512). The wgpu 0.19.4 calls
and validation rules (aspect views, sample types, formats, layouts) were
reviewed against the pinned wgpu/wgpu-hal/naga sources.
