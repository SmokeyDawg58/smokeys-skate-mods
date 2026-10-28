# Environment shading parity audit, WorldPainter fog/sky, sky dome (2026-09-25)

Status: source written; rustfmt parse PASS on every touched file. NOT compiled,
NOT tested, NOT run. This follows
`render-environment-shading-2026-09-25.md`, which was built and run:
lightmaps looked right, but distant buildings rendered as flat blue shapes and
there was no sky.

## Root causes found in IDA (debug `sk82_na_m.xex`)

1. **Fog source.** `RenderMan::UpdateRenderParams` 0x82852780 packs
   `g_FogK1`/`g_FogColour` from `m_FogAttribs.m_output_collection`. That is a
   WorldPainter blend, not the location fog phase 1 used.
   - `RenderMan::UpdateWorldPainterAttribs` 0x82848ba8 takes
     `VD_WorldPainterInfo` keys (fog, colour cube, sky, exposure, bloom,
     district) whenever the VisualDirector supplied them and the location
     layout byte 0 is set.
   - `kbEnableWPRender` defaults to 1.
   - Without WorldPainter keys it falls back to the location's
     `material_fog` / `material_sky` / `auto_exposure` / `colour_cube` /
     `bloom` references (32-bit keys 0x83f23940 / 0xaa55d29f / 0xb190ce8d /
     0x0b263bce / 0x14304f99).
   - The city never sends `cMsgSetWorldRenderLocation`. `GameModule::GameLoad`
     0x8273be50 sends it only for an additional world location. So the
     location is always the default 0xec763034105b866b.
2. **WorldPainter keys.** `VisualDirector::VDWriter::_PrepareWorldPainterData`
   0x827fd200 samples the game camera position permuted `<0,6,0,0>`, i.e.
   (x, z). It calls `LayerMan::GetAttribData` 0x82ceaf50 for layers eFog 15,
   eColorCube 11, eSky 16, eExposure 17, 18 (bloom) and eDistricts 4. The
   chain is:
   - `GetTileHandle` 0x82cead18: 128-unit tiles via `GenTileId`.
   - Layer data at tile +32 + 4*layer: `tWorldPainterLayerData`, with quad tree
     +0 and dictionary +4 as arena IDs (`Fixup` 0x829c7858).
   - `DoQuadTreeLookup` 0x82cea8b8:
     - root centre +0, half width +16, `m_uiNumNodes` +32, root node offset
       +36 (`Fixup` 0x829c6408);
     - 10-byte nodes: four signed children, then the lookup word;
     - children[0] == 0xFFFF marks a leaf;
     - inclusive `vcmpbfp` box tests;
     - child centres = half width × `quad` [(-1,-1),(-1,1),(1,-1),(1,1)] +
       centre, with `half` (0.5, 0.5, 0, 0);
     - `InBounds4Way` 0x82cea420 picks the first containing child in order.
   - The dictionary check is index < `m_NumTotalEntries` (+0). The region word
     is the first word of the entry pointed to by the 8-byte slot at
     `m_pEntryTable` (+12), both relative to the dictionary (`Fixup`
     0x829c63a0).
   - `WorldPainter::Palette::kaTOC` 0x82312480 has 363 entries of 24 bytes and
     is shared by all 19 `LayerMan::sLayerData` 0x83024690 handles. The
     comparison is on the GUID's high word only.
   - The result is `Attrib::StringToKey` (= `StringHash64`) of the region
     name. Layers 5 and 13 lower-case the name first (LayerHandle byte +1).
     A miss returns key 0.
3. **Blending.** `RenderMan::RenderMan` 0x82854268 starts both fog and sky on
   the `default` collection 0xd7edbd362d7d2152 with a 5 s blend. Auto-exposure
   (key 0x2b00cf9bb190ce8d) blends over 5 s and bloom over 2 s.
   - Per frame, `BlendAttribCollection<T>::Update` 0x828445f8 / 0x82844718: a
     changed key calls `ChangeCollection` 0x8283d7b8 / 0x8283c038, and a
     missing collection targets the initial defaults. Then `UpdateCollection`
     0x8283d9a0 / 0x8283c0d8 runs.
   - `BlendAttrib`, per value:
     - lerp' = `fmadds(invBlend, dt, lerp)`, clamped to exactly 1 by `fsel`;
     - output = `fmadds(1 - lerp, begin, end*lerp)`;
     - `Change` restarts from the current output with lerp 0.
   - The fog blends colour, far (+28), max/strength (+24), near/start (+20)
     and power (+16).
   - The sky blends `m_params[0..2]`, writing into `material_sky/default`'s
     `m_params` storage.
   - Vault: 22 `material_fog` collections (fog_financial, fog_oldtown, …) and
     `material_sky` collections with different values from the blue default.
4. **Sky.** `WorldPresentation::AddRenderInstance` 0x827e4f00 builds a
   `SkyPresEntity` 0x827d4258 for a new-format instance whose bone-name list
   starts with "skybox". The entity marks every island visible, and `Update`
   0x827d4480 re-centres it on GfxMan+0x1B260 (`mGlobalData`+0xE0 =
   `g_vViewPos`).
   - The shipped `sky.fx` (`default` technique): `defaultVS` offsets
     model-space positions by `g_vViewPos`.
   - `defaultPS`:
     - gradient desaturated by `m_params[1].w` and tinted by `.xyz`;
     - clouds (`i_detail`) added, minus 0.5;
     - squared;
     - sun glow from `tex1D(i_specular, sin(angle to g_vLightDir) /
       m_params[0].x)`, squared, divided by saturate(a + 0.01);
     - × `m_params[0].y`, then `PS_Bloom_Tone` with `g_envattributes[2].x`;
     - alpha 1.
   - sky.xml samplers: diffuse/detail WRAP-U CLAMP-V anisotropic,
     specular CLAMP.
   - Render states: CULLMODE FRONT only; depth on.
   - The port had routed the sky mesh as an ordinary diagnostic draw at its
     authored transform, far from the camera, so it never appeared.
5. **Occlusion cubes.** In the same function, "testCu…" models become
   `OccTestPresEntity` and are never added to a render view. The port was
   drawing them.

## Audit of phase-1 assumptions

- **Technique:** confirmed. `PresentationEntity::BindConstants` 0x827fb168
  resets every entity to `szdefaultTechnique` ("default") each frame.
  `baseenvironment`'s lod/optimized programs are not used by world entities.
- **HEND3N tangents:** no CPU decoder exists in the xex. It is a fixed Xenos
  vertex fetch format (11/11/10 signed-normalized, x in the low bits), which
  is what `skate2-formats` decodes.
- **CSM empty atlas:** `RenderShadows_CSM` 0x82855888 only clears
  depth/stencil and resolves the atlas from the shadow pass. Regions without
  casters must evaluate lit (otherwise the whole 360 world would be dark), so
  the 1x1 (1, 0) atlas is the equivalent state.
- **Shader maths:** re-checked line by line against the `.updb` HLSL.
  - `ApplyOverlay` returns the darken branch.
  - The reflection vector is `reflect(E, N)` with xy negated.
  - Phong uses `-reflect(L, N)`.
  - Fresnel is `tex1D(.).g * 5`.
- **Sun direction map:** real and on by default. `kEnableSunDirectionMap`
  defaults to 1, and layout byte 3 is set.
  - `SunLightMan::GetSunPosition` 0x82851f20 samples the map at the skater's
    hips via `SampleBilinear` 0x82826fc0.
  - The map is a texture named by a `rendering` attribute (key 0x44e84dd8),
    loaded through `SetTexturePointers` 0x8284e3f0, `LoadWorldMapAsset`
    0x82851d10 and `LoadWorldMapImage` 0x8284f968.
  - **Deferred:** the location sun vector is still used.
- **Auto-exposure:** `PostFX::Tonemap` 0x82828f68 reads the downsampled
  bloom buffer back each frame and rewrites `g_envattributes[2].x` =
  clamp(((target − avgLum)·damping + 1)·exposure, min, max). **Deferred** to
  the PostFX batch (bloom chain); exposure stays at the Vault 1.0.
- **Texture fetch-constant swizzle/sign:** not retained by the decoder. The
  supported DXT1/DXT5/A8R8G8B8 formats carry the standard swizzle, and the
  diagnostic colours confirm the channel order. A per-texture audit needs the
  header retained (12 `NativeTexture` constructors); **open**.
- **Scalar channel names** (macroOverlayUVScale / detailNormalUVScale /
  macroOverlayOpacity): arena payloads are chunk-compressed. **Unconfirmed**;
  the fallback-reason log names them if they are wrong.

## Files

- `crates/skate2-core/src/world/world_painter_palette.rs` (new, generated from IDA):
  the `kaTOC` table.
- `crates/skate2-core/src/world/world_painter_layer_asset.rs`: dictionary object,
  `quad_tree_lookup`, `dictionary_region_word`; tests in
  `world_painter_layer_asset_tests.rs`.
- `crates/skate2-core/src/world/world_painter_layer.rs`: `try_attrib_key`
  (`GetAttribData`) and the lower-case layers.
- `crates/skate2-core/src/session/selected_session_public_facade.rs` + `lib.rs`:
  `WorldPainterKeys`, and `SelectedVisualSnapshot::world_painter` sampled at
  the camera (x, z).
- `apps/skate2-pc/src/render_environment.rs`:
  - all `material_fog` / `material_sky` collections;
  - location fog/sky references;
  - `m_params` arrays;
  - which materials share `material_sky/default`'s `m_params` storage.
- `apps/skate2-pc/src/render_world_painter.rs` (new): `BlendAttrib`,
  fog/sky `BlendAttribCollection`, and the location fallback.
- `apps/skate2-pc/src/gpu_renderer/environment_shading.rs`:
  - `EnvironmentEffect::Sky`, the sky layout/sampler/pipeline (no lighting
    stream) and the `SKY` WGSL;
  - globals grow to 240 bytes with the blended sky `m_params`.
- `apps/skate2-pc/src/gpu_renderer/map_draw.rs`:
  - `FrameEnvironment` holds per-frame fog and sky;
  - `push_sky` is never culled;
  - `InstanceMaterial::sky`, `set_view(clip, eye, frame)`.
- `apps/skate2-pc/src/render_inspection.rs`:
  - per-frame blend with presentation dt;
  - sky routing;
  - occlusion-cube skip;
  - per-effect diagnostic counts.
- `apps/skate2-pc/src/live_skating_view.rs`: passes the WorldPainter keys.

## What to look for on the next run

- **Fog:** distant buildings take the district's fog (grey in the financial
  area, warm in old town) and ease between regions over 5 s. They should no
  longer be flat blue everywhere.
- **Sky:** a sky dome behind everything. The log shows `Sky: n` in
  `original environment shading draws=`. If it is missing, the
  `ported-effect draws kept diagnostic by reason=` entry names the missing
  channel. For example, a 1D sun texture the decoder cannot load would show
  up there.
- **Unported draws:** `diagnostic draws by effect={...}` names the effects
  of the white window towers, so they can be ported next.
- **Lookup failures:** `WorldPainter key lookup failed (...)` is printed once
  if the lookup errors; fog/sky then use the location references.
