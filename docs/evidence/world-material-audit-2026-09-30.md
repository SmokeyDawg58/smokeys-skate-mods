# World material audit (in progress)

Scope: coverage of missing materials, channels and fog, checked
against the debug executable and shipped programs. Work is solo. Existing
material resolution, WorldPainter, sun map, shared geometry, shadow and texture
streaming owners are reused.

## Findings and current changes

* The Boneyard null-collection fallback is native behavior, already documented
  in `boneyard-material-binding-2026-09-29.md`. It must not be confused with the
  app's unlit diagnostic fallback for unported effects or missing channels.
* The original world path previously supported eight effects. Added source
  implementations for `transparentdefaultenvironment`, `tree`, `treeanimate`,
  `vertexanimate`, `baseincandescent`, `scrollincandescent`,
  `transparentincandescent`, `zbiasincandescent`, `zbiasincandescentbacklit`.
* Tree lighting requires `g_ViewDotLight`, previously never published by the
  host. `RenderMan::UpdateRenderParams` 82852780 normalizes camera matrix axis2,
  dots the authored rendering direction and packs four lanes:
  `[response.x + dot*response.y, minimum, backlit, 0]`.
  Keys in rendering/default:
  direction `d900aef7ea4d75a2`, response `222cba2d46ea6839`, minimum
  `0279cce002b46bc4`, backlit `9cdb53d164e5da30`.
* Verify raw VMX128 words, not IDA's printed VA: its high VA bits are wrong in
  this database. Decode VA as `((w>>16)&31)|(((w>>5)&1)<<5)|(((w>>10)&1)<<6)`.
  82852d94 dot uses v63/v125, 82852e00 response multiply-add, 82852e04/e08
  interleave response/minimum and backlit/zero. `RenderMan` constructor
  82854268 creates rendering/default.
* `WorldPresentation::Update` 827d3af8 advances `g_fAnimationTime.x` using
  DataSystem tick/fraction, not render wall time. Frame difference outside the
  open interval (-15,15) suppresses delta; otherwise
  `max(0,(fraction-prevFraction+frame-prevFrame)/60)`. Constructor 827e9d00
  sets prevFrame=-15, prevFraction=0. The selected visual facade publishes
  completed whole ticks, so its fractional presentation time remains zero.
  NIS time is a separate native branch and is not claimed implemented here.
* TreeAnimate/VertexAnimate share native wind displacement, use three authored
  material vectors and the final VPO sine range reduction. They differ in
  minimum light: TreeAnimate clamps; VertexAnimate does not. Neither applies
  static-tree m_params[0].y exposure. Their manifests have no culling, alpha
  GE30, depth writes and RGB writes. Static tree writes RGBA.
* Incandescent shaders average four half-texel-offset lightmap samples before
  squaring the selected channel, use single-tap chromaticity, and interpolate
  lightmap contribution using m_params[0].x. Diffuse sampler is the native
  pass reset (wrap, linear min/mag, point mip), not the ordinary environment
  sampler. Transparent/ZBias variants use additive ONE/ONE blending, **not**
  source alpha. Backlit ZBias is opaque with alpha GE16; both biased variants
  retain native -0.0001 depth bias and slope1. Scroll uses authored U/V speeds.

## Reference artifacts

All 440 final shader files extracted read-only from original render.big to
`%TEMP%/skate2-material-audit-20260929/data/render/shaders/final`. Extracted UPDB
source and index reside in the sibling `source` directory. Compare final VPO/FPO
with UPDB: the latter is stale for some shipped programs (vehicle fix proved it).
`artifacts/material-family-source-audit-20260929.txt`, manifests companion,
and effect-named vpo/fpo disassemblies record the reviewed shader paths.

## Validation / open work

### Latest follow-up: texture and vertex binding defaults

Two more original behaviors are now connected; this supersedes the20-fallback
count below. `TextureMan::AddMaterial8282c838` tests the full64-bit GUID,
looks for odd/even registered assets, then the configured missing texture.
It still consults the original GUID's shader-table slot, otherwise publishes
the resolved asset's direct pointer. `AddPegasusMaterialToScope82e17bb0` also
uses missingTexture for a null channel stream. Raw PPC/decompilation is saved
in `artifacts/material-missing-texture-ida-20260930.txt`.

The configured name `missingtexture.Texture` hashes to4e8aac7b89e95463. It is
the original128x128 DXT1 texture with8 mips in miscboot.big/globaltextures.rx2.
It now lives in the existing retained loose collection and uses the existing
texture upload cache. `own_texture_asset` shares the whole-arena decoder;
selecting this one TOC asset avoids decoding the CPU-only linear sun-map
surface as a GPU material. No generated replacement texture was introduced.
An entirely absent channel is still a separate unresolved case; it does not
silently become the missing-texture asset.

`D3D::PatchVertexShaderToMatchVertexDeclaration82c43db8` explicitly handles
missing Usage/UsageIndex. At82c43eb4 it installs0x9250; OR0xe plus the original
kPatchSwizzleMask table822c96b0 composes destination selectors4,4,4,5 =0001.
World meshes lacking TEXCOORD1 now receive that default. Existing decoded
values, including signed zero used by normal encoding, remain unchanged.
PRESENT unsupported or truncated streams are not converted to absent inputs.
Evidence: `artifacts/material-vertex-default-ida-20260930.txt`.

Validation: `material-binding-default-tests-20260930.log`:108 pass,3ignored,
including native0001 bytes, malformed-stream rejection, signed-zero retention
and actual GPU draws of incandescent/transparent effects without authoredUV1.
`material-binding-default-build-20260930.log`:release built successfully.
Three-frame inspection and1200 live Waterfront ticks exit0. Inspection now
reports6 diagnostic draws instead of20:1 absent specular channel,5 original
2D `arrow` textures assigned to environment-cube channels. Their native flow
requires further investigation; no guessed cube substitution is applied.

Read-only `tools/inspect_texture_bindings.rs` scans boot identities and optional
all-city BAM_Tex using production parsers. Reports `material-texture-identities`
and `material-texture-reflection-audit` show three absent reference GUIDs were
not found in the high-detail BAM_Tex corpus. This does NOT prove absence from
every archive/proxy pool. Stone's specular map has nonzero blue endpoints;
there is no proof that its environment lookup can be discarded.

Inspection-only --render-eye x,y,z and --render-target x,y,z allow reproducible
material views. `material-water-view-east-20260930.png` shows the Waterfront
ocean surface from(850,-30,940) toward(1000,-65,1100). The elevated diagnostic
view also reveals sky clipping; its far plane is2000, whereas live gameplay
uses the original camera producer's Vault plane. Cause/visual parity remains
unverified. These switches do not alter the controller camera or spawn.

### Latest checkpoint: street effects and original distant water

This supersedes the older build-in-progress paragraphs below. There are now
28 world-effect pipelines. TrafficLightOne/Two use the existing RAIN owner's
visual vehicle and pedestrian indexes, published through the completed frame.
DoSimSubmit827d3cd8 / DoSimRestore827da340 pack 2 into the current vehicle lane
and twice the pedestrian index into W; no second traffic timer was added.
The final shaders select UV1 when the indexed lane exceeds1, otherwise UV0.

Glare implements the original camera-axis billboard and biased clip depth,
GE1 alpha test, additive ONE/ONE blend and no depth writes. GodraySpotlight's
final VPO uses the original mesh normal and instance transform; source-level
billboard intermediates were optimized away. Its final FPO uses diffuse
squared times abs(dot(normal,view)), additive blending and no fog. The existing
lighting vertex slot carries NORMAL0 for normal-only meshes without adding
fake lightmap admission or increasing the stream stride.

Videoscreen aliases BaseIncandescent because its shipped instruction stream
and manifest states match (container trailing hashes differ). This does not
implement video decoding. OneOffLoadTexture827357f8/8273580c's DIST_Water texture
and geometry arenas now join the existing retained loose presentation owner.

Validation: material-spotlight-tests-20260930.log records107 passed,3ignored,
including GPU draws for all28 effects. material-street-build-20260930.log is a
successful release build. Three-frame inspection and1200 live Waterfront ticks
both exit0; material-street-inspection-20260930.png and
material-street-waterfront-20260930.png were inspected. These views do not yet
establish close-up water or traffic-cycle visual parity.

An opt-in SKATE2_MATERIAL_AUDIT reports remaining fallback channels. Normal quiet
play produces no such logging. Waterfront has2790 draws,20 diagnostic: missing
UV1 (five neon,one cloud), absent diffuse/specular/detail GUIDs (eight), one
missing specular channel, and five authored2D `arrow` environment textures.
The latter are not automatically evidence of a bad texture resolver. Native
TextureMan::AddMaterial8282c838 and AddPegasusMaterialToScope82e17bb0 use the
configured boot `missingtexture.Texture` for missing referenced assets; an
entirely absent channel is a separate case. Investigation continues.

Remaining: actual missing vertex/texture inputs, texture-dimension mismatch
semantics, full postFX bloom/exposure/colour grading and documented sampler
differences. Native anisotropy8 plus point-mip filtering cannot be represented
directly by wgpu's anisotropic sampler rules. No full-renderer parity claim.

### September 30 continuation: terrain, buildings, water

The world census completed all 285 presentation collections, recorded in
`artifacts/world-material-census-20260930.tsv`. The `ported` column reflects
the first 17-effect build, not the current source. Dynamic-object effects use
their existing separate renderer; their false world-enum entries do not mean
DMO materials are missing.

Seven further effects are now implemented: baseterrain, decalterrain,
advertisement, building_ceiling_worldspace, water, flowingwater and ocean.
Terrain preserves both normal/colour blend equations; decalterrain's FINAL
program samples i_specular for both layers despite the duplicate authored
channel. Building ceiling strips use original floorHeightMeters/floorCount
and world-XZ projection. Advertisement's final program has no fog or bright
tone branch and does not sample its authored normal channel.

All regular environment reflection/specular helpers now use final-program
analytic Fresnel: 1 + 4*(1-saturate(dot(E,N)))^8. The older UPDB calls a lookup
texture which these FINAL shaders do not bind. Production GPU readback checks
cover facing, grazing, negative-dot and calculated-specular values.

Water animation follows cPCAWaterAnimationData::Init827e9498 and Update827d02f0.
The original PCAMeanValues82fef1c8 (30x3) and PCAFrameWeightValues82fef330
(30x24) are retained as exact IEEE bits. Update adds WorldPresentation's
simulation delta, uses strict >1/30, subtracts ONCE per presentation call and
increments the u16 frame counter; reads frame modulo30 without interpolation.
Weights/mean are scaled by the native float reciprocal255. Clock updates now
occur once per presentation frame, including zero-delta frames, matching the
native catch-up behavior. NIS's alternate clock remains outside this path.

Final water/ocean FPOs remap reconstructed PCA RGB to [-1,1] before toning
down; old UPDB source omits that second remap. Ocean additionally normalizes
max(abs(normal),.001), uses Ward anisotropic specular and its own Fresnel.
Final ocean pc59 applies sqrt(abs(NL*NE)); pc75..83 uses abs for Fresnel's
power base. Flowing water instead sums two scrolling tangent-space normals.
All three use original material arrays, lightmaps, chromaticity and cubemaps,
preserving the reflection XY sign changes. No invented substitute textures.

Validation: `artifacts/material-water-tests-20260930.log`: 107 app tests pass,
3 ignored, including GPU pipeline/draw checks for all 24 world effects and
water timing/table regressions. A release build is in progress. Earlier
17-effect build passed 600 Waterfront ticks with a screenshot in
`artifacts/material-families-waterfront-20260930.png`. New water/terrain/
building changes are not yet visually verified.

Water cube sampler still uses linear min/mag, point mip, anisotropy1 rather
than native anisotropic8/point mip: wgpu disallows anisotropy with point mip.
This is a documented host limitation, not exact sampling parity. Distant
DIST_Water loose presentation asset is not yet connected. Traffic-light shader
publication is next: native DoSimSubmit827d3cd8 / DoSimRestore827da340 use the
EXISTING RAIN traffic cycles; do not implement a second timer.

104 app tests pass, 3 ignored, including native clock/view-light regressions
and GPU pipeline/draw validation for all seventeen world effects.
Release build and original-city census are in progress. The explicit
`--material-census` command reuses original collection decoders and Vault
binding resolution, releasing each collection before reading the next.

This is **not** complete renderer parity. The older validation paragraph above
describes the first batch. Traffic lights, video screens, glare/godrays and
remaining missing texture bindings still require implementation. Existing WorldPainter fog/sky is present; exposure, bloom,
colour grading and their associated render targets remain separate gaps.

WGSL lacks fractional texture-fetch offsets. Incandescent half-texel offsets
currently use level-zero dimensions; Xenos applies offsets at the fetched mip.
This is exact for a one-level lightmap, not a claim of identical minification
on mipped inputs. This gap is explicitly retained for further work. Also retain
existing documented host float/anisotropy/raster precision differences.
