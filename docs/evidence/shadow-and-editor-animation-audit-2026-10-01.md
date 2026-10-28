# Dynamic shadows and editor animation audit — October 1, 2026

Reports: small bright holes in skater shadows, unstable editor shadow edges,
and editor animation stepping. This audit uses the connected debug XEX in IDA,
raw PowerPC argument setup, PDB structure offsets, and shipped shader microcode.
It does not establish that every reported pixel has the same cause.

## Confirmed corrections

### Editor animation sampling

`CASCharacterAnimator::Init` **82597140**, especially **82597370..82597464**,
constructs an `ACSParams` at stack+0x100. PDB offsets: mode+52,
fetchmode+56, category+60. The stores are:

- **82597390**: mode=0 (RAW).
- **825973ec**: fetchmode=1 (FETCHBLEND), from r21 initialized to1.
- **825973f0**: category=0.
- Truncation offset remains .5 (**82597450**).

The struct is passed partly in registers and partly on the stack. Interpreting
the flattened Hex-Rays argument list as separate scalar arguments had given
the wrong default fetch mode. `EvalCurrAnim` **8258f178**, call setup
**8258f1c8**, requests USEDEFAULT(2), so it resolves to FETCHBLEND(1), not FETCH(0).

`ui/edit_skater_presentation.rs` now passes the correct ACS default to the
existing shared `BufferedFetchTiming` and `frame_pair`/Andale sampler. This
restores interpolation between adjacent clip frames without inventing a new
animation clock or smoothing layer. Existing AddTime and gesture-switch timing
was checked against `CASCharacterEntity::Update` **8259f230** and retained.
Optional `SKATE2_TRACE_CAS_FETCH` logs decoded times, frame pairs and weights.

This supersedes earlier editor notes/comments claiming ACS default FETCH(0).

### Native shadow texture precision

`PostFX::Shadows_CSM_Init` **8283ec80**, including **8283ecf8..8283ed44**,
allocates the HBLUR/ATLAS/WORLD targets with G16R16_EDRAM buffers resolved to
G16R16 textures. PDB distinguishes these from G16R16F:

| Format | Native value |
| --- | --- |
| G16R16 | 0x2d200199 |
| G16R16_EDRAM | 0x2d20ab8d |
| G16R16F | 0x2d22ab9f |
| D24S8 | 0x2d200196 |

The host previously used Rg32Float for HBLUR and Rg16Float for ATLAS/WORLD.
These are different precision/rounding representations. On supported adapters,
all three now use **Rg16Unorm**. The device requests the two wgpu0.19 native
extensions only after checking actual render-attachment, texture-binding and
filterability capabilities. Unsupported adapters retain the earlier float
approximation; that fallback is explicitly not precision parity.

This matches the resolved native texture representation; it does not emulate
every Xenos EDRAM operation. Host Depth24PlusStencil8 is still backend-selected,
and its depth/stencil aspects replace the native resolved byte decoding.

### Ordinary cloth shadow vertex layout

Ordinary full-material ROPA vertices contain position/UV/weights/indices plus
tangent/binormal: **52 bytes**. The shadow selector omitted ROPA and read that
buffer through a **28-byte** pipeline. The selector now uses the same 52-byte
layout as the visible surface and CPU cloth updates. Editor CAS remains60,
full hair60, plain hair36, and original plain character28. This is a host
stream mismatch, not a change to native cloth simulation or shadow bias.

## Other shadow flow reviewed

- `RenderShadows_CSM` **82855888**: two512² dynamic cascades,1024x512 atlas;
  light basis, native scale/offset/depth attributes, floor(v*256)/256 snapping,
  gameplay skater top+2, and separate frontend camera CSM attributes retained.
- `ShadowView::Get_BBOX_Height` **82829de8**: native15% min/max padding agrees
  with the editor's single-character caster height calculation.
- `ShadowView::Cull` **82850ac0**, `IsInLightView` **82829b88**: distance²1300,
  light-space bounds and first-cascade containment behavior retained.
- Caster state: depth LessEqual, clear1, stencil clear0/replacement255,
  back-face culling and the existing posed/skinned geometry.
- Fresh dumps of H5/V5/H3/V3 at **8212f4e0**, **8212f748**, **8212f998**,
  **8212fb88**: whole-texel offsets, depth selection and tent coverage agree
  with the current WGSL. The disassembler already applies the encoded
  half-texel-unit conversion; do not halve its printed offsets again.
- Receiver review: first cascade wins overlap, native atlas Y mapping,
  environment coverage/depth test, character9-tap self-shadow sampling and
  authored material bias remain unchanged. No arbitrary shadow resolution,
  bias or temporal blur was added to conceal the symptoms.
- World-shadow reuse retains exact projection/command identity and publishes
  its cache only after submission; dynamic caster passes still run per frame.

## Validation and artifacts

- `artifacts/check-shadow-filters-20261001.rs` is a standalone GPU regression
  that extracts the **current production WGSL**, renders a depth/stencil
  rectangle and checks all262144 output pixels per kernel. Both native kernels
  PASS on Vulkan: exact coverage agreement with the quantized tent
  oracle, depth within1/65535, and no missing interior coverage. Log:
  `artifacts/shadow-filter-gpu-regression-20261001.log`.
- Raw ACS setup: `artifacts/cas-fetch-init-raw-20261001.txt`.
- Full native decompilations: `artifacts/shadow-cas-audit-0x*-20261001.txt`;
  additional bounds source: `artifacts/shadow-bounds-native-20261001.txt`.
- Fresh filter disassembly: `artifacts/shadow-audit-filters-20261001.txt`.

- Release build PASS (6m29s), `artifacts/shadow-cas-parity-build-20261001.log`.
- Standing editor, couch editor and gameplay runs all exit0. Captures inspected:
  `artifacts/shadow-cas-standing-20261001.png`,
  `artifacts/shadow-cas-couch-20261001.png`,
  `artifacts/shadow-gameplay-20261001.png`; matching `.log` files record native
  Rg16Unorm selection on the test GPU. No GPU validation errors reported.
- Standing trace:878 fetches,877 fractional pairs. Couch run:2224 sitting
  fetches, all fractional, plus1119 standing fetches (1118 fractional). Every
  fetch uses FETCHBLEND and valid adjacent/end frame pairs. The first standing
  sample starts at exactly0. This verifies interpolation is actually exercised,
  not merely enabled in configuration.
- Seven existing projection/culling tests PASS using the previously built test
  executable; those functions were unchanged by this patch. Their log is
  `artifacts/shadow-projection-regression-20261001.log`. The standalone GPU
  filter test above reads current source, not the previous executable.
- Only `artifacts/edit-skater-diagnostic-saves` was selected for runtime tests;
  normal player saves were not used. No compiler or game left running.

A same-location comparison remains needed to establish whether the reported
bright pixels and edge crackle are completely eliminated. Native512² shadows
also have inherent finite-resolution aliasing; do not claim pixel-perfect
parity from a clean boot or one screenshot.

## Follow-up: remaining ground specks and wheel/deck edge

Both reproduced after the first corrections. Neither is declared
fixed by this follow-up. Re-read native `AddDefaultSamplerStates` **82e18990**:
`ShadowTextureSampler` is linear min/mag, border-zero U/V, no mip, anisotropy1.
The existing receiver samplers agree. Source saved to
`artifacts/shadow-sampler-native-20261001.txt`. Re-read skater material binding
**827dcf50** into `artifacts/shadow-receiver-bind-native-20261001.txt`.

Fresh final-program disassembly:

- `baseenvironment_defaultPS` pc39 calculates normal-map world normal dot light;
  pc49..52 calculates CSM then chooses it only for nDotL>0. A normal facing away
  from the light receives1 here. The port's existing normal-dependent bypass
  is in the shipped program, not an invented optimization. This is a candidate
  explanation for specks, not a demonstrated cause at the reported location.
- `defaultcharacter_defaultPS` pc32 uses g_charattributes[0].z times
  (1/512,1/1024); pc39..47 fetches9 taps; pc48..67 counts depth comparisons gated
  by nonzero coverage. These are binary taps with a1/9 average, not a smooth
  analytic penumbra. January17 original skater.vlt/bin authored first row is
  (.003,.0095,.8,0). Existing cloth/skin/face/ordinary character values match.
  Native finite resolution can produce steps, but an equivalent original-game
  close-up is still needed to classify the reported wheel edge.
- A **separate confirmed mismatch**: `cacstamp_shift_defaultPS` uses fixed
  +/-one-texel offsets (pc36..53) and depth+1 (pc22/54), not character attributes.
  Its embedded HLSL explicitly passes scalar-1 for the bias vector. Reused the
  existing PCF helper with parameterized bias/radius and selects(-1,-1) for
  mode3 only. Cloth/skin/face and ordinary characters retain authored settings.
  Current isolated editor recipe contains cloth/defaultcharacter/skin/face/hair,
  not shift: do NOT claim this change fixes that recipe's board edge.

Disassemblies: `artifacts/shadow-receiver-{base,cas,cacstamp_cloth,cacstamp_skin,
cacstamp_face}-20261001.txt`. Existing original shader extraction reused from
TEMP/skate2-material-audit-20260929 and TEMP/skate2-character-shading-20260930.

### Diagnostic build, not a claimed fix for the remaining reports

`SKATE2_SHADOW_RECEIVER_VIEW=raw|normal` changes supported ground-material output
only. Raw: black=shadow, white=lit. Normal: additionally marks red where the
original nDotL bypass exempts a pixel inside a valid shadow. Covered effects:
default/alphatest default, base/decal environment, base/decal terrain. Other
effects/actors remain normally shaded; this is deliberately not a global
material replacement. Compile-time shader constant is0 and folded out for
ordinary play. No new renderer/atlas ownership, logs or per-frame readbacks.

`play-shadow-debug.bat` selects normal mode; optional argument `raw` selects raw.
It scopes the variable to its process and invokes the existing quiet launcher.
Closing it and starting play-quiet.bat restores ordinary rendering.

Release build PASS2m13s, `artifacts/shadow-receiver-build-20261001.log`.
Three550-tick offboard-street-look runs at Waterfront passed/exit0 in normal
diagnostic, raw diagnostic, and ordinary rendering. Captures visually inspected:
`artifacts/shadow-receiver-{normal,raw,lit}-20261001.png` and matching logs.
At THIS test location the raw silhouette has no interior holes and the
normal diagnostic shows no red bypass within it. Do not extrapolate to the
cropped pavement screenshot; location/full view not yet established.

Extended the standalone actual-WGSL GPU harness to exercise both shared ordinary
and shift receiver parameters against a known depth/coverage texture. Both
receiver results and both filter kernels PASS across262144 pixels each:
`artifacts/shadow-receiver-gpu-regression-20261001.log`.
Only isolated diagnostic saves used. No compiler/game left running.

Outstanding: get the same-location diagnostic result (specks white, red, or
absent) and whether they remain attached to pavement or move with the camera.
The latter was asked asynchronously; no answer received at handoff. Compare
an equivalent native wheel/deck view before changing the native PCF/radius.
