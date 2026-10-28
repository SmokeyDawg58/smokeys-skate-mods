# Road decals, transparent overlays, loose sky arenas (2026-09-26)

Status: source written; rustfmt parse PASS on every touched file. NOT compiled,
NOT tested, NOT run. Follows
`render-worldpainter-sky-parity-2026-09-25.md`.

## What the run showed

The run showed half the road in a different colour from the middle, and
baked tree shadows cutting off at seams. The run log (`render-log.txt`, 09-25)
explains both, and also the missing sky:

- `diagnostic draws by effect` had `decalenvironment` at 62-83 draws. Those
  road pieces drew unlit raw diffuse, with no lightmap, fog or tonemap. The
  `baseenvironment` road next to them was lit. That produced the colour
  seam, and the baked shadows stopped at the decal pieces.
- `transparentenvironment` (136-177 draws: road markings, grime, overlays)
  also drew opaque and unlit.
- `original environment shading draws` had no `Sky` entry, so no sky
  instance ever reached the queue.

## decalenvironment (ported)

Source: `decalenvironment.fx` from `render.big` UPDB, `default` technique.

- **VS:** identical to baseenvironment, except `VS_IN.uv_decal` is `float4
  TEXCOORD0` and zw is copied to `Out.Decal`.
- **PS:** baseenvironment's PS, except for these points:
  - `cDecal = GetDecalTexture(i_decal, Decal)`: rgb squared, alpha kept
    (`environment_textures.fxh:32`).
  - `cDiffuseColour = ApplyOverlay(cOverlay, ApplyDecal(cDecal,
    cDiffuse.rgb))`, where `ApplyDecal = lerp(base, decal.rgb, decal.a)`
    (`:37`).
  - `returncolour.a = 0`.
  - The remaining differences are inert:
    - `ApplyOverlay(half3, half4)` and `GetDiffuseTerm(half4)` truncate to
      the only (`half3`) overloads.
    - `TransformToWorldFromTangent` has the same body as `_normalize`.
- **Samplers / states:**
  - `i_decal` is CLAMP/CLAMP, ANISOTROPIC min/mag, LINEAR mip (the existing
    clamp sampler).
  - The pass sets only CULLMODE=FRONT, so it is opaque with depth write.
- **Port:**
  - `EnvironmentEffect::Decal` uses baseenvironment's channels plus `decal`,
    and the same scalars.
  - The lighting vertex stream grows from 28 to 36 bytes to carry
    TEXCOORD0.zw at location 14. That is zero for two-lane UV0;
    `GpuMesh.has_decal_uv` gates Decal admission.
  - The baseenvironment WGSL became `environment_vs` + `environment_tone(surface)`,
    shared by `defaultPS` and `decalPS`. The math is unchanged.

## transparentenvironment (ported)

Source: `transparentenvironment.fx`, `default` technique.

- **VS:** baseenvironment's `lm_norm` normal decode, with no TANGENT and a
  `float2` UV.
- **PS:**
  - diffuse (rgb squared) and specular masks;
  - the *unnormalized* interpolated normal;
  - `NdotL > 0 ? CSM : 1`;
  - `GetShadowedLightMap` with the same constants;
  - `diffTerm = GetDiffuseTerm(cLightMap, 1)`;
  - `GetSpecularTerm` with the same compiled arguments;
  - `outcolor = (half4(diffTerm,1)*cDiffuse)*cDiffuse.a + half4(specTerm,0)`,
    so the alpha is diffuse.a squared;
  - then `PS_Fog_Bloom_Tone`, and `returncolour.a = outcolor.a`.
- **States (transparentenvironment.xml):**
  - ALPHABLEND SRCALPHA/INVSRCALPHA ADD, alpha the same;
  - ALPHATEST GREATEREQUAL 16, done in the program after every fetch;
  - ZENABLE and ZWRITEENABLE true;
  - COLORWRITE=RGB;
  - CULLMODE FRONT.
- **Port:**
  - `EnvironmentEffect::Transparent` with channels diffuse, specular,
    environment (cube), lightmap, chromaticity;
  - `EffectTarget.blend`;
  - `GetSpecularTerm` moved to a shared `SPECULAR` WGSL snippet, which
    base/decal/transparent all include.
- The unused `sh` technique (spherical-harmonic irradiance) is not used by
  world entities: `BindConstants` forces "default".

## Loose sky arenas (loaded)

The loading path in IDA:

1. `GameModule::OneOffLoadTexture` 0x82735730 adds, in this order:
   - `data\content/world/models/DIST_skybox_Textures.rx2`
   - `DIST_skybox.rx2`
   - `DIST_Water_Textures.rx2`
   - `DIST_Water.rx2`
   - `material_test.rx2`
   - (`DIST_ProxyWorld*` only when `kEnableProxyWorldStream` is 0)
2. `LooseAssetHelper::LoadAssets` 0x8272cdb8 turns '/' into '\', loads each
   file as an arena (`kAssetType_Arena`) and sends `SendPresRegArenaMsg`. That
   is the same Presentation registration stream arenas get.
3. `WorldPresentation::AddRenderInstance` 0x827e4f00 then makes the
   "skybox"-named model a `SkyPresEntity`.

What the port had missed: it never loaded these files. The contents
(checked offline with `extract_big.py` plus a dictionary/fetch-constant
reader that mirrors `rw4.rs`/`material.rs`):

- **`DIST_skybox.rx2`:**
  - one InstanceData, 2 models, 5 optimeshes, 1 material table;
  - model `skyboxSphere`;
  - materials:
    - `proxymoutains` (`environmentsimple.background`, with lightmap and
      chromaticity);
    - two ad signs (`environmentsimple.default`);
    - `skybox1` (`sky.default`: sky_gradient / sky_detail / sun_gradient);
  - bounds: the dome is ±7451 wide and -285..4472 high; the mountains reach
    about ±5000.
- **`DIST_skybox_Textures.rx2`:** 8 textures, all 2D DXT1 or A8R8G8B8, so all
  decodable, including every channel above.
- **Vault:** bindings exist for `material_environmentsimple`
  {default, background, alphatest} and `material_sky/default`, and every
  effect string in the Vault names a shipped manifest.
- **Far plane:** 7700 (`rendering` attribute; corpus test). That is greater
  than the dome radius, and the dome is centred on the camera, so it sits
  inside the clip volume as in the original.

How the port loads them:

- `PresentationScene.loose` holds these arenas. It is resident for the
  session, outside stream focus, and chained last in `resident_collections`
  and `resident_collection(index)`.
- `load_runtime_presentation_scene` fills it with the textures arena, then
  the model arena, as one collection.
- **Host robustness (not game behaviour):** if the arenas cannot be owned, or
  a material has no Vault binding, it logs `loose presentation assets not
  loaded (...)` and continues without them. The original loads them
  unconditionally.
- The runtime-scene probe now collects manifests from every resident
  collection, which `NativeRenderQueue::build` requires.

**Deferred, and documented in `LOOSE_PRESENTATION_ASSETS`:**

- The `DIST_Water` pair (`oceanPlane`, `ocean.default`).
  - `ocean.fx` needs `g_fPcaMean` and `g_fPcaWeights{R,G,B}_{0,1}` from
    `cPCAWaterAnimationData` (`Update` is called from
    `WorldPresentation::Update`).
  - Drawn through the diagnostic path, it would be a flat placeholder plane.
- `material_test.rx2`. Its only instance is the "testCube1"
  `OccTestPresEntity`, which is never rendered.

## g_fAnimationTime writer (found; not yet consumed)

- `GraphicsMan::Init` registers `g_fAnimationTime` as `mGlobalData` +0x150
  (GfxMan +0x1B2D0). No store uses that offset directly.
- The writer is `WorldPresentation::Update` 0x827d3af8. It uses the
  `DataSystem` time controller tick count `{int frame, float fraction}`:
  - In an NIS it uses the ScenePlayer's `GetTimeDelta`.
  - Otherwise, dT = `fsel`-abs((fraction - mPrevDelta + (frame - mPrevFrame))
    * 1/60).
  - dT is 0 when |frame delta| >= 15 (`CLOTH_MAX_FRAME_DELTA` 15.0, and -15.0).
  - It stores `g_fAnimationTime = (old.x + dT, old.y, old.z, old.w)`.
  - It then calls `cPCAWaterAnimationData::Update(dT)`.
- Consumers are the unported `treeanimate`, `vertexanimate` and
  `scrollincandescent` effects.

## What to look for on the next run

- **Road:**
  - `original environment shading draws=` now lists `Decal` and `Transparent`.
  - The road reads as one lit surface.
  - Baked shadows continue across decal pieces.
  - Markings and overlays blend instead of drawing as opaque unlit quads.
- **Sky:**
  - `Sky: 1` or more in the same line.
  - A gradient dome with clouds and a sun glow, plus distant proxy mountains
    lit by their own lightmap.
  - If the sky is still missing, look for a `loose presentation assets not
    loaded (...)` line, or a `Sky: ...` entry in `ported-effect draws kept
    diagnostic by reason=`.
- **Still diagnostic:** `advertisement` (no UPDB source), traffic lights
  (need `g_TrafficLightsStatus`), the incandescent family,
  `building_ceiling_worldspace`, `treeanimate`/`vertexanimate`,
  `baseterrain`, `water`/`flowingwater`, and the ocean.
