# PostFX highlight sampling - October 12, 2026

PC source changes only, solo. Playtesting showed excessive cyan sign bloom and colored
screen reflections beside the skater at the Fitness & Racquet building. Compare
the PC clipboard image with `realgamecomparison.png`,
captured from RPCS3. The requested baseline remains the live Xbox debug IDA.

## Confirmed mismatches and changes

### Bloom resolution and sample footprint

`Sk8::Render::PostFX::Init` at **0x8284a8b8** fixes the scene domain at
**1152x640**. Raw PPC at **0x8284a908/910** loads 0x480/0x280 and stores them
at PostFX+0x24/+0x28 at **0x8284a938/944**. The decompiler incorrectly prints
332 for the width, reusing an earlier r5 value; use the raw instructions.
`SetRects` **0x82823050** independently confirms 1152x640 for the 1x viewport
and the complete 2x/4x tiled rectangle coverage.

`InitBuffers_Xbox360` **0x8283ea58** derives the quarter and sixteenth bloom
surfaces from those members. Their native sizes are **288x160** and **72x40**;
the exposure surfaces are **32x16**. The device target separately takes the
front-buffer size. The port previously used PC render width/height divided by
4/16, so increasing display resolution reduced the normalized blur footprint.
The fine and medium passes consequently spread a highlight over a smaller
portion of the image. This is a definite source mismatch; attributing the full
reported visual difference to it still needs an updated scene capture.

Changed the retained bloom, combined, and reflection targets to their original
dimensions. Extraction offsets now use 1/1152 and 1/640, matching
`GenerateBloomBuffers` **0x82832c68** calling
`InitWeights_Blur16WithBilinear` **0x82822bb0** with main2's dimensions. The
existing Gaussian producer now naturally receives the original bloom dimensions.

The PC scene, depth/stencil, cinematic intermediate, final output and HUD still
use the selected render resolution. This retains the original effect footprint
over normalized screen coordinates while supporting a higher-resolution scene.
For ultrawide, the same effect domain covers the whole viewport; the original
game does not provide an ultrawide rendering baseline. No extra scene reduction
or guessed blur kernel was introduced.

### Quad sampling phase

`PfxHelper` constructor **0x8283e238** sets its quad rasterizer's
`halfpixeloffset=0` at **0x8283e348**. `RasterizerState::Initialize`
**0x82bccf58** and `D3DDevice_SetRenderState_HalfPixelOffset` **0x82c35cf0**
carry that into the hardware pixel-center bit. The fullscreen vertices come
from **0x82f6f8e0** and `CreateQuad` **0x82830400**; they span -1..1 without
a position bias. `quad_shiftuv_VS` **0x8212ade0** adds only the supplied UV
offset. `RenderQuad` **0x82827db0** supplies zero for a null argument.

The integer-center convention is independently documented by the primary
[Xenia Xenos definitions](https://github.com/xenia-project/xenia/blob/master/src/xenia/gpu/xenos.h)
(`PixelCenter::kD3DZero`) and its
[viewport conversion](https://github.com/xenia-project/xenia/blob/master/src/xenia/gpu/draw_util.cc).
WGPU fullscreen interpolants instead reach the first pixel at half a texel.

| Pass | Native explicit UV shift | Port correction relative to WGPU centers |
| --- | --- | --- |
| Initial extraction | Debug attribute / main2 dimensions | Attribute / reference dimensions minus half a destination texel |
| Both lower-level downsample passes | Half a destination texel | Zero; conventions cancel |
| Five Blur9 passes | Half a destination texel | Zero; conventions cancel |
| Combined bloom | Zero | Minus half a destination texel |
| Six-reflection pass | Zero | Minus half a destination texel |

The extraction attribute is class `postfx` **0x46f43159bc8272ab**, collection
`Debug` **0x550740b00aa8d4ac**, field **0x88a3da4a1d1d4179**. Constructor
**0x828394f4..52c** binds that collection; **0x82832ddc..eb8** fetches the
field and divides by main2 width/height. Both supplied datasets contain **0.0**.
The port now reads the field through its existing Vault reader instead of
substituting a hardcoded tuning value. GPU uniform slot24.zw carries the net
sampling phase to extraction, combined bloom and the reflection shader.

The existing Blur9 vertical pass deliberately uses source **width**; the
combined vertical pass uses **height**. Both match their native callers and
remain as written. FastUber/VisualFX retain the PC scene-resolution sampling;
this change does not impose the native full-scene rasterization on PC.

## PS3 versus Xbox investigation

Reference datasets:

- PS3: `PS3_GAME\USRDIR`.
- Xbox debug: `<game folder>`.

Compared all defined attributes in all **39 post_fx collections** with the
original Vault readers. These authored postfx values match across datasets,
including the six reflection colors/scales. Original Xbox shader bytes were
extracted from the live IDB and disassembled. Relevant programs:

- 0x8212b250 extraction; 0x8212f2e0 lower-level downsample.
- 0x8212b748 Blur9; 0x82130478 combined bloom; 0x82130038 reflections.
- 0x8212df38 FastUber; 0x8212ade0 quad vertex shader.

RPCS3's existing BLUS30253 shader cache also contains the six-reflection shader
(`7CF6D52E32FF078.fp`, older `FBBDD5ACDFB632C5.fp`), with the same six samples,
0.33 RGB sum, sampled alpha mask and authored color multiplication. It does
not establish the runtime bindings for the exact comparison screenshot.
The PS3 extraction shader `B40786BE7EC40271.fp` additionally multiplies its
squared inputs by 0.943359375, and PS3 FastUber `28C7E44D421CA93D.fp` has two
scene samples and a 1.1 power. These are genuine platform differences, not
justification to change the Xbox-reference constants. RSX instruction fields
were decoded against the primary
[RPCS3 program definitions](https://github.com/RPCS3/rpcs3/blob/master/rpcs3/Emu/RSX/Program/RSXFragmentProgram.h).

The original ghost shader does **not** use the CPU's registered
`g_Bloom_Thresh` name; that uniform was optimized away. Adding a new reflection
threshold or disabling reflections would not match the inspected Xbox shader.
Bloom weights, threshold/saturation, reflection strengths, scene gains, LUTs,
and exposure adaptation were not retuned. The base-incandescent material
shader was also compared with the port; that review did not justify changing
its emissive gain, fog/exposure or gamma arithmetic.

## Source review and limitations

- Reviewed production stage bindings, separate per-pass constants, texture
  dimensions/formats, resize reconstruction, exposure readback and the debug
  attribute's class/collection/field against the live IDA reference.
- Three production files changed: `gpu_renderer/postfx.rs`,
  `gpu_renderer/postfx.wgsl`, `render_postfx.rs`.
- Existing fixtures in `gpu_renderer/postfx_tests.rs` and
  `render_environment.rs` were updated for the added asset field. The latter
  also lacked the existing `hom` field and now initializes it. No test cases
  were added or executed. No build, game launch or screenshot replay was run.
- **Exact:** native sizes, callers, field keys, shipped attribute value and
  quad offset differences. **High:** normalized effect-footprint and sampling
  correction in the host shader. **Unverified:** amount of improvement at the
  pictured sign, disappearance of the reported ghosts, or pixel-identical
  output against RPCS3. Higher-resolution scene sampling, original platform
  shader differences and GPU filtering/precision remain relevant.
- The existing executable does not contain this source change. A fresh PC
  build and matching view are needed for visual acceptance. Do not describe
  the screenshot discrepancy or complete PostFX parity as verified fixed.
- Android, simulation/physics, saves and the prior HoM full-speed exception
  were not edited by this change.

Local ignored research: `.scratch/postfx-brightness-20261012/`, containing
before-file backups, `ida-reference-packet.json`, shader disassembly/bytes,
`vault-platform-comparison.json`, PS3 shader notes and `source-changes.diff`.
Original assets and shaders remain external or in ignored diagnostics.


## Requested PC release build - October 12, 2026

Compilation followed. The normal command
`cargo build --release -p skate2-pc -j 1 --timings` succeeded with exit 0 at
`2026-10-12T09:19:27.0025899Z` (814.079 seconds wall time).
The new `target/release/skate2-pc.exe` is 42,463,744 bytes, SHA256
`C74ABAAA5C3CEEDE0B78CEF6E4DA45D3006A31D4588D52808D91E6A21CEA9175`. It includes both the postfx sampling correction and the
HoM full-speed changes. No source fixes were needed during the build. Existing
warnings remain (435 core, 18 PC). No tests or gameplay runs were performed;
the result awaits a hands-on check. This supersedes earlier uncompiled status,
while the visual/runtime verification limitations still apply.

Log and manifest: `.scratch/postfx-brightness-20261012/pc-release-20261012-120552.log`
and `.json`. Previous executable backed up as
`.scratch/postfx-brightness-20261012/skate2-pc-before-20261012-120552.exe`.
