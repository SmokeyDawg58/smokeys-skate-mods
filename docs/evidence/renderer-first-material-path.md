# First original visual material path

2026-09-12, bounded read-only renderer/source audit. No backend was chosen or
installed; no shader or runtime code was added. Confidence is **high** for
original shader-source formulas and identified ownership; **medium** for CPU
producer summaries from decompilation pending raw arithmetic review. No GPU
or visual parity is claimed.

## Current executable and asset boundary

The default `apps/skate2-pc/src/main.rs` path reports that playable runtime is
unimplemented and exits. App dependencies are core/formats only. There is no
window, camera execution, GPU upload or draw consumer. The private probe path
`load_runtime_presentation_scene` -> `own_presentation_arena` produces owned
`PresentationScene`; `NativeRenderQueue::build` gathers/sorts CPU submissions.
`TextureScene` is independently loaded and bound by GUID. This is not the live
WorldPresentation registration path: resident probe `presentation_register`
only records an event, while `DecodedArenaAsset` explicitly stops before
subsystem registration. Actual processed registration/first-use/last-release
must feed the scene and GPU residency, rather than rebuilding from arbitrary
archive reads in each draw.

BAM_Pres visual optimized Pegasus meshes are distinct from BAM_Sim collision
meshes/StaticGroundPool. Existing visual decoders preserve positions, semantic
attributes, indices, base vertex/start index/count and triangle-list islands.
Documented Rob's Park focus: six cells, 2,173 visual meshes, 734,053 vertices,
1,696,275 indices, 4,281 draw packets. Queue: 2,085 resolved contexts and88
deferred contexts with missing `default` effect. These census numbers come
from the earlier corpus evidence, not a new full inventory run here.

The external root contains `data/big/render.big` (5,194,161 bytes),
`data/content/worldbam.big` (2,445,763,840), `worldproxy.big` (4,810,496).
`colourcubes.big` is absent. Texture decoding supplies linear mip/face layouts;
BC1/BC3 remain block-compressed. Existing documented bindings resolve5,704 of
15,950 sampled channels to685 resident textures; global/nonresident textures
remain explicit gaps. Collision geometry is not a substitute visual model.

## Newly found original source evidence

`render.big` includes the final shader **UPDB files with embedded preprocessed
HLSL**, not merely opaque compiled programs. The previous target packet had
not inspected these companion assets. Extraction used the repository BIG and
RefPack formats, kept originals outside the repository in
`%TEMP%/skate2-original-defaultenvironment`, and checked decompressed sizes.
XML/VS/PS hashes independently match the prior target packet.

All paths below have prefix `data/render/shaders/final/`:

| Asset | Bytes | SHA256 |
|---|---:|---|
| defaultenvironment.xml |1671|fa026da3b09d480a7242e4c2df7f37f3c33c9b4abccde91467206273f5070bc8|
| defaultenvironment_defaultVS.vpo |896|ecf885470d91bbd75bfe784765bc5926c7ea91de8095844d8ff89319d3400433|
| defaultenvironment_defaultPS.fpo |1220|4b2855c319b918b547e2e289bf919a5a6c4de47dcc3b64e50981731a958ff90d|
| defaultenvironment_defaultVS.updb |86438|1693ca3e176e4e5e818d3b3c22620b87d8b02328a26902d06bbbc094732f2c7a|
| defaultenvironment_defaultPS.updb |88109|560148ef42398b33cb050205876aede1ef6f9d888370788f82e4532ff53089d1|

Both UPDB files identify compiler7645 and define SHADER_FINAL/rwgDEVICE_XBOX2.
Their shader entries select `defaultVS`/vs_3_0 and `defaultPS`/ps_3_0. Both
contain the same67,251-character preprocessed source in files/file id0, with
original #line references under D:/sk82/main/code/ng/render/shaders/Source.
They also contain constant tables, source maps, literal tables and interpolator
metadata. This is original reference evidence, not emulator shader translation.
Source formulas still require comparison with compiled optimization/precision
and captured rendering before bitwise or visual parity claims.

## Readable formula plan

The names in this section identify original source operations; implementation
should use readable domain shader code and preserve the staged operations.

`defaultenvironment.fx:10` vertex input is POSITION float3, TEXCOORD0 float2
(diffuse UV), TEXCOORD1 float2 (lightmap UV). It forms P=(position,1), passes
P.xyz to the pixel stage, and computes row-vector P*`g_matVP`. It does **not**
use `i_matWorld`, although that variable is declared elsewhere in the shared
source. Do not automatically multiply the queue instance transform into this
effect: validate fused-world vertex-space provenance first. It packs the two
UV pairs, computes per-vertex fog and copies the lightmap selector to an
interpolator. Fog is interpolated; recomputing per-pixel distance changes it.

`postfx_functions.fxh:21`: distance=length(P.xyz-viewPosition.xyz);
f=pow(saturate(distance*FogK1.x+FogK1.y), FogK1.z).
FogRGBA=FogColour*f+(0,0,0,1). FogColour alpha is meaningful, not an assumed1.
The unused matV argument and unused texture dimension arguments disappear
from the selected compiled parameter table.

`defaultenvironment.fx:21` pixel operations, in source order:

1. Sample diffuse at UV0. Square RGB to enter linear space; preserve sample
   alpha (`common_textures.fxh:6`, `common_functions.fxh`). Do not also request
   hardware sRGB decoding and thereby linearize twice.
2. Compute CSM atlas lookup from interpolated world position. Let a=dot(Row0[0],P),
   b=dot(Row1[0],P), z=dot(Row2[0],P), and second=(a,b)*Row0[1].xy+Row0[1].zw.
   Start UV=(-1,-1). If max(abs(second))<0.99, choose
   second*(0.25,-0.25)+(0.75,0.75). Then if max(abs(a,b))<0.99, overwrite with
   (a,b)*(0.25,-0.25)+(0.25,0.75). The first cascade wins overlap because its
   assignment occurs last. No perspective divide occurs (`common_shadows.fxh:14`).
3. Sample shadow RG. The selected helper returns
   saturate(step(z-(-0.005), shadow.r)+(1-shadow.g)). Equality is in front.
   The other passed shadow-bias components10/25 are unused in this compiled
   source path; do not add variance equations suggested by the variable name
   `moments` (`common_shadows.fxh:41`). Call this result S.
4. Sample lightmap at UV1; L=dot(sampleRGBA,lightmapSelector), then L=L*L.
   Sample chromaticity at the same UV unchanged. Light RGB starts as
   L*chromaticity.rgb*4.5. Blend toward (0.07,0.09,0.1) with
   saturate((L-0.04)*50)*(1-S). Intermediate alpha includes chromaticity.a*1.5,
   but the helper overwrites it with lerp(min(L,0.04),L,S); its final alpha has
   no1.5 dependence. defaultPS consumes only its RGB
   (`environment_lighting.fxh:12/53/96`).
5. Form RGB=linearDiffuse.rgb*lightRGB, alpha=diffuse.a; multiply by
   `m_params[0].y`. Then X=(color*Fog.a+FogRGBA)*`g_envattributes[2].x`.
   Per channel tone mapping is
   0.5*(max(0.25*X+0.75,1)-saturate(1-X)^2), then square root.
   Finally overwrite output alpha with0. No generic Reinhard curve, extra
   saturation or conventional fog lerp is interchangeable here
   (`postfx_functions.fxh:51/77`).

The numbered list follows HLSL source dependencies, not compiled GPU scheduling.
The PS source map schedules chromaticity fetch at pc15, lightmap at pc16 and
shadow at pc17. Reflection active counts are Row0 c0/count2, Row1 c2/count1,
Row2 c3/count1 and environment c4/count3 despite declared array lengths3/3/3/7.

The PS literal table independently contains0.04,50,0.99,0.25,-0.25,4.5,0.5,
-0.99,+0.005,0.75,1,0.07,0.09,0.1. HLSL half/float types and compiler choices
must be considered during native GPU validation; they do not imply host f16
storage or a bitwise GPU arithmetic match without evidence.

## Parameters, ownership and next producer closures

| Input | Proven native owner/binding | Remaining work |
|---|---|---|
| VP/view position | GraphicsMan.Init8273f6e8 binds GlobalShaderData; SetCurrentCameraIndex8256b508 selects camera VP/V/P and copies globals; UpdateCamera82848978 consumes actual CameraPresentation | Native camera producer, projection/winding/clip conventions and exact matrix arithmetic; no arbitrary fly camera called gameplay parity |
| FogK1/FogColour | UpdateRenderParams82852780 consumes m_FogAttribs.m_output_collection; UpdateWorldPainterAttribs82848ba8 updates the blending collection first | Retain actual location/WorldPainter blend owner; independently review raw VMX fog packing before implementing |
| environment exposure | GraphicsMan.Init binds g_envattributes to generated material_envattributes layout array (length then layout+16), not a hardcoded exposure | Complete original Vault collection key/data verification and lifetime; read array[2].x live |
| material brightness | cMaterialScope.Resolve82e17a98 enumerates declared material parameters, hashes names into actual material class/variant Instance and binds returned data pointer | Read exact environmentsimple.default m_params via Vault, including inheritance; no guessed1 default |
| lightmap selector | AddPegasusMaterialToScope82e17bb0, imageChannel&3, static LightMapDot table | Preserve instance selection and source overwrite order, not a single global selector |
| three material samplers | AddPegasusMaterialToScope starts at channel2; scalar flag2 selects inline float4, otherwise actual stream pointer (or caller-provided missingTexture) | Actual streamed/global texture admission and original missingTexture owner; no fabricated white/black replacement |
| shadow rows and texture | PostFX.Shadows_CSM_Init8283ec80 constructs512x512 D24S8 depth,512x512 G16R16 blur and1024x512 G16R16 atlas; RenderShadows_CSM82855888 builds rows, renders/filters/resolves atlas | Actual shadow-view casters, camera transforms, blur programs, atlas-global pointer binding and sampler states; do not inject always-lit S=1 |

The selector table is source-proven: channel0=(1,0,0,0), channel1=(0,1,0,0),
channel2=(0,0,1,0), channel3=(0.33,0.34,0.33,0), with RGB bits
3ea8f5c3/3eae147b/3ea8f5c3. Native channel detection checks the first two
shader-input characters for `li`, and binding uses the instance hash path.
The current scene already retains channel strings, image-channel values and
scalar constants, but does not execute this parameter scope resolution.

Decompiled fog producer indicates layout+14 start, +1c end, +10 power,
+18 strength, +0 RGB. K1.x=1/(end-start), K1.y=(-1/(end-start))*start,
K1.z=power. This is a **medium-confidence producer summary** until raw VMX
lane stores and fog-colour alpha sign are independently reviewed; do not
silently convert it to a sanitized host fog model.

Manifest one default pass: CULLMODE FRONT. Prior source packet establishes
depth enable/write LESS_EQUAL, no alpha test/blend, RGBA writes. Lightmap and
chromaticity clamp U/V; diffuse wraps; anisotropic min/mag4, linear mip,
THREEEIGHTHS threshold. Shadow sampler is reflected at s0; lightmap s1,
chromaticity s2,diffuse s3. XML order is not register order. Final alpha0 must
remain even with blending disabled because later postprocessing can consume it.

## Bounded implementation order and validation

First close real presentation registration/residency -> original visual scene
and a material admission check that reports exactly which samplers/parameters
are unresolved. Recover the selected camera/global/material producers and CSM
producer needed by this formula. Then implement portable buffer/texture/draw
consumers and this readable supported shader subset, preserving unknown
materials as deferred. A useful earlier diagnostic view may be separate, but
is not this source material's parity implementation. Character live pose/GPU
skinning is a subsequent required playable presentation closure.

Backend-specific resource, clip-space, winding and synchronization operations
belong behind the presentation interface. Native File/Read/Seek archive access,
blocking probe loops and host input cadence need platform adapters for later
Linux/Android/WASM. BC1/BC3 support/fallback, texture residency memory and
browser file delivery require explicit device/platform validation. The renderer
direction remains provisional; this task makes no
backend selection and installs nothing.

Validation completed here: original archive extraction/decompression and
asset hashes; UPDB entrypoints, shared source, reflection and literal-table
cross-check; IDA CPU producer/caller inspection. No Cargo, GPU test, capture or
new visual census was run. Required later tests include UV/lightmap selector,
cascade overlap/outside behavior and exact shadow comparison boundary, fog
alpha, brightness/exposure sampling, output alpha0, and original-frame capture.

## Resident presentation registration continuation

Follow-up read-only audit, 2026-09-12. The smallest live link is an owner-bound
Presentation receiver and admitted scene publisher. Extracting the old scene
loader alone is insufficient: its dictionary inventory is not native admission.

The existing `robs_park_resident_probe.rs` builder retains complete
`DecodedArenaAsset` values in `ProbeResidentAsset::Presentation`, inside the
actual `OwnedResidentCollection`. Its `ProbeParentEffects::presentation_register`
and deregistration methods only record events. The separate `main.rs`
`load_presentation_collection` rereads XSF, decompresses again, and builds
`PresentationScene`; it is never reached through these effects. Its
`own_presentation_arena` groups dictionary material/mesh/model/instance passes,
and `own_presentation_instances` includes every InstanceData record regardless
of subtype or TOC admission. Moving the focus also distance-sorts collections.
Those are useful inventory operations, not reconstructed registration ordering.

Fresh IDA inspection confirms this source chain (decompiler argument recovery
is imperfect for full-width asset IDs; use established raw proofs in
`arena-registration.md` for those ABI details):

1. `NotifyCollectionPresReg` (`0x82d5c680`) takes the independent loaded hold
   before notification; `NotifyCollectionPresDReg` (`0x82d580d0`) queues removal
   and its completion. Parent Simulation and Presentation acknowledgments are
   separate. An asset destructor is not a substitute for this receiver point.
2. `PresThreadMsgMan::ReceiveRegCollectionMsg` (`0x82780958`) returns immediately
   for activation zero. Otherwise it resolves the current collection, walks
   its arena vector forward, and invokes `RegArena` only on first use of each
   full arena ID in its own `cAssetRefCounter<4096,2048>`. The collection route
   uses each retained arena pointer directly; it does not re-resolve every
   arena from the global map. The individual-arena route (`0x82780890`) first
   increments, then resolves the global arena only on first use. Deregistration
   uses the corresponding last-use gate (`0x8277fff8`, `0x8277ff38`).
3. `ProcessArena` (`0x82d5fee0`) selects the first USED TOC and executes its
   callbacks in TOC order. `ProcessInstanceList` (`0x82d5ec38`) expands recognized
   subtype records in forward order. Its Environment predicate really skips
   a record when BOTH collision-model and render-model nonzero mesh predicates
   are true; do not replace this unusual branch with an intuitive test.
4. `Presentation::RegArena` / `DRegArena` (`0x827f1430`, `0x827f1680`) traverse
   the processed asset list forward. Kind2 material operations and kind10
   Environment operations remain interleaved in this order. Kind10 enters
   `UnlockableLocationMan::AddEnviroInstance` (`0x828b02d8` -> `0x828b0068`),
   which resolves the local-GUID context, retains the source pointer and
   applies actual location-lock state. Only an admitted instance reaches
   `PresUnlockableLocationMan::AddToWorld` (`0x827e5360`) and
   `WorldPresentation::AddRenderInstance` (`0x827e4f00`).
5. `WorldPresentation` rejects bad render models, distinguishes legacy and
   optimized entities, and special-cases model-name prefixes `skybox` and
   the first six characters of `testCube1_1000`. The occlusion entity takes a
   separate scene-view route. Ordinary entities enter a GUID-keyed world map
   and render views; proxy-view selection depends on the first mesh material's
   sort order versus the actual world proxy-order value. Kind20 instead enters
   `LooseWorldPresentation::AddRenderInstance` (`0x827e10b0`), requiring a
   nonnull nonempty model and adding the entity to all render views. Kind19 is
   `VIPresentation::AddMeshInstance` (`0x828058d8`), a separate local-GUID owner
   with a fresh Presentation registry lookup, not an ordinary world instance.

Concrete implementation sequence proposed for integration:

- Extract pure geometry/material/model projection from app `main.rs` into a
  reusable presentation decoder. Give it the already-retained decoded arena;
  preserve source dictionary/object offsets alongside projected indices.
  Do not publish its full inventory of instances. A new processed presentation
  arena plan supplies source-ordered material and selected Environment handles,
  retaining the same decoded allocation via an owning handle. The resident
  asset variant should retain that plan rather than a second decoded copy.
- Add a separate Presentation registration owner with the shared per-arena
  first/last-use counter, current resident lookup, ordered operations, and
  callback/hold lifetime. `OwnedResidentAssetsView::try_for_each` already visits
  insertion order after releasing its mutex. Its visitor receives `&A`, so
  persistent record handles need a cloneable owner inside `A`; borrowed visitor
  references alone cannot back later presentation entities. Existing
  `presentation_dmo_arena_adapter` is a narrow DMO-bank slice and cannot be
  relabeled as the general BAM_Pres receiver.
- Feed kind10 through a real Presentation unlock owner, with the same sourced
  context/lock producers used by the shared base algorithm. The existing
  `simulation_environment_unlock_router` can inform that algorithm, but its
  Simulation dispatch and collision handles are not Presentation owners.
  Publish admitted ordinary world entities into the scene; retain explicit
  unresolved routes for sky, occlusion, proxy selection, VI and other encountered
  presentation kinds until implemented. Do not silently flatten them all.
- Build `NativeRenderQueue` from this registered scene at a coherent generation,
  after live material binding. Current `RenderSceneHandle` contains vector
  indices, not owning handles: queue consumption must retain that scene
  generation or complete before any collection compaction/reordering. On last
  deregistration, remove admitted entities/material registrations in source
  order before releasing the Presentation loaded hold. GPU completion will
  require its own eventual resource lifetime; CPU callback completion is not
  proof of GPU retirement.

Suggested exclusive implementation boundaries are new `presentation_arena_decode`
and `presentation_resident_registration` modules, their tests, and an app
resident-presentation child with narrow builder/effect mounting. Root should
own changes to the shared scene/queue interfaces and executable entrypoint.
Required first corpus gate is same-allocation BAM_Pres resident load -> actual
receiver -> admitted scene -> queue -> deregistration -> hold release, including
shared-arena double use and callback-only activation zero. Compare original
projection geometry and material identities, but do not assert the old
unfiltered inventory's draw count as native admission truth. No new source-kind
census, runtime registration test, GPU test or Cargo gate was run in this audit.
