# Shared PostFX restoration — October 2, 2026

Requested after the frontend/world appeared dull. This change mounts the original
exposure feedback, bloom and colour-cube stages. It is not a new brightness or
saturation preset. The normal display gain is now1.0 (the original1.188 compensation was removed
in the follow-up below). Explicit `SKATE2_BRIGHTNESS` remains a separate override.

## Evidence checked in the live debug IDB

- `PostFX::Apply` 8284c7a0: scene processing precedes the existing final blit;
  ordinary gameplay uses FastUber, ICE uses VisualFX followed by Uber. CAS does
  not select the 30-Hz cinematic branch. This does not impose a PC frame cap.
- `SetParameters` 8284bbf0, `SetBloomCoeefs` 828233b0, original `post_fx` vault
  records, `UpdateWorldPainterAttribs` 82848ba8 and constructor 82854268:
  bloom/exposure/location cube inputs, bloom 2-second and exposure 5-second
  parameter blends. The threshold range<.1 branch really loads 10, not .1.
- `GenerateBloomBuffers` 82832c68, `Blur9` 82830a58 and
  `GetSampleOffsets_Bloom` 82827b00: quarter/sixteenth/32x16 surfaces, nine
  bilinear Gaussian fetches, original multiplication/order, combined bloom and
  six ghost reflections. Vertical Blur9 deliberately uses source width; the
  combined pass uses height. No guessed extra coarse contribution: the shipped
  combined3 program samples only two textures despite the C++ third binding.
- Shipped pixel programs at 8212b250 (extract), 8212f2e0 (downsample), 8212b748
  (blur), 82130478 (combine), 82130038 (ghost), 8212df38 (FastUber), 8212e570
  (VisualFX), 8212dc28 (Uber). Disassembly, rather than CPU pseudocode alone,
  establishes square-before-bloom extraction, scene gains 1.41 / 1.41412354,
  colour-matrix operations and colour-cube coordinates.
- `Tonemap` 82828f68, including raw PPC: reads **alpha**, not RGB, from delayed
  32x16 bloom; original x/y weights, target/min/max/damping and multiplicative
  exposure update. Publishes environment[2].x and character[2].z. Cars use the
  shared live exposure instead of retaining their construction-time value.
- `ColourCubeCache::ASyncLoad` 8284d968, `ColourCubeManager::BlendBegin`
  8282cc50, `Blender::SetColour` 82b22548 / VMXBlend2 82b21468, and
  `XGRAPHICS::TileVolume` 82c30c48. Original RGB files retain Xbox volume tiling
  even though alpha is stripped. Decode to PC-linear 32x32x32 RGBA; preserve
  byte-domain blend truncation before hardware trilinear filtering.
- Cube sampler 82828918 plus `Device::SetState` 82bcbbd0 and
  `Xbox2SetGpuTextureFetchRegister` 82bcb440: the srgbtexture field in the
  generic sampler does **not** enable an extra gamma conversion on Xbox here.
  Texture initialization 8284b950 uses format 18280186 (A8R8G8B8). Keep the
  PC cube UNORM, not sRGB.
- `ICEUserChannelBloom::UpdateFrameEnd` 826c88e0: element112 enables element37
  threshold / element36 intensity. Otherwise `PostfxPresentation::SetDefaults`
  827eff00 supplies 1.4/.5. Existing frontend ICE sampler supplies these values;
  no second animation owner. ICE threshold halves; fine/medium intensity *4.

Captured decompilations, raw PPC, original pixel programs, disassembly and small
extracted cube assets are local ignored diagnostics under
`artifacts/frontend-source/postfx*` and `artifacts/frontend-source/cubes`.

## Integration and PC adaptations

`render_postfx` loads inputs via the existing mounted Vault/BIG readers.
`render_world_painter` retains parameter blends. The existing renderer/final-pass
owner renders into a retained scene surface, performs PostFX, then runs its
existing fisheye/noise/vignette path. The 2D HUD remains after these stages;
projected wall UI participates in the scene as before.

Readback uses three asynchronously mapped buffers; ordinary frames never wait
for the GPU. Feedback uses the latest completed eligible buffer. The native
exposure recurrence is stepped at 60 Hz (cinematic30) in elapsed presentation
time to avoid high PC render rates accelerating adaptation. Cube blending uses
the native 1/360 progression normalized to a six-second gameplay duration.
This is a documented PC scheduling adaptation, not a claim of identical GPU
queue latency to Xbox. Surface resize preserves accumulated exposure.

The small shipped cube inventory is preloaded/deduplicated through the existing
mount instead of doing disk reads during a location transition. Unshipped cube
references (for example cc_xgames.rgb) do not fail unrelated world loading;
the current cube is retained, as with an incomplete native cube load. Other
file/data errors remain errors.

## Validation and limits

- Scalar threshold/matrix/feedback tests pass.
- Tiling bijection and all **32,768** texels of the original `cc_neutral.rgb`
  match independently expected R=(31-y)/31, G=x/31, B=z/31 byte values.
- Production GPU stages compile and execute; readback verifies separate RGB
  channels through both FastUber and the quantized cinematic intermediate.
  A small isolated harness imports the actual production modules/tests, with
  only the asset-reader owner replaced by its source-extracted data types.
- Release build PASS (`postfx-build3.log`, 1m40s app rebuild; preceding core
  rebuild8m13s). Native shader validation caught a WGSL runtime array-indexing
  restriction; corrected and retested before the successful captures.
- Main-menu camera completion PASS, exit0 (`postfx-runtime3.log`,
  `postfx-menu.bmp`). Visually inspected: original LUT tint/contrast, bloom,
  green foliage, readable wall text and unaffected screen HUD.
- Main-menu -> gameplay -> Quit -> menu roundtrip PASS, exit0,213 gameplay
  ticks (`postfx-roundtrip.log` / `.bmp`).
- Outdoor Waterfront offboard-street-look PASS, exit0,550 ticks,
  9.166667 simulated seconds /9.154705 wall seconds (`postfx-gameplay.log`
  / `.bmp`). Captures used display gain1.0; the ordinary launcher still used
  1.188 at this checkpoint. That validation gap caused the overbrightness
  follow-up below. This is a functional capture, not a GPU benchmark.
- Only isolated diagnostic saves were used. No game process left running.

This is the shared exposure/bloom/grading path, **not complete PostFX parity**.
Depth-dependent DOF/motion blur, authored non-default cinematic colour/effect
channels, special HoM/success/failure colour animations, online menu exposure
override and stereo/screenshot branches are not newly claimed. The current
frontend takes have no populated near/far colour channels; their matrices use
the original identity defaults. GPU floating-point/filtering differences remain
possible even with the original constants. Exact screenshot parity still needs
same-scene original-console captures.

## Overbrightness correction following a screenshot review

The old display compensation (1.08*1.10 = 1.188) remained after the newly
mounted LUT. This clipped all graded channels above approximately0.84175,
destroying highlight detail independently of the native exposure and bloom.
Changed the normal default to1.0; preserve explicit `SKATE2_BRIGHTNESS` only
as an optional user override. This supersedes the earlier brightness change
because the resulting image was overbright. Native material
tone mapping, bloom coefficients and the FastUber1.41 gain are unchanged.

Fresh IDA raw SetBloomCoeefs828233b0/ApplyFX_NEW82823f08 review did not justify
retuning the saturation matrices (native bloom uses1+saturation). The material
tone map's final0.5 and subsequent square root already precede FastUber;
removing its native1.41 gain would introduce another mismatch.

Release build passed (`postfx-highlight-fix-build.log`). Same static550-tick
`--live-y-demo` Waterfront captures with isolated saves both exited0:
`postfx-highlight-before.bmp` explicitly1.188 and `postfx-highlight-after.bmp`
with the environment override absent, exercising the new default. In matching
sky ROI x180..929,y40..239 (150,000 pixels), all RGB>=250 fell from84,862 to
36,013; pixels with any channel255 fell from89,060 to22,924. Visually the sky
gradient and sunlit pavement retain more detail. These are PC comparisons,
not proof of original-console image parity or elimination of all bright sky.

Expanded production GPU regression beyond zero bloom: constant scene
RGB[.2,.4,.6], threshold.02, medium.5/fine.25, neutral LUT, zero ghosts.
Extraction/blur/composition independently predict approximately[75,171,255];
both gameplay and cinematic paths pass within2 bytes. All four isolated
production-source tests pass (original-asset cube test remains separately
opt-in). No new shader retuning or normal save changes in this correction.

## Presentation clock correction and deferral

Repeated redraws were consuming the same published frame.dt multiple times.
PostFX now records frame.updated_at and advances exposure/cube blend time only
once for that presentation update. No native tuning constants changed. A GPU
regression repeats the same frame and verifies exposure/cube blend state stays
fixed, then advances a new timestamp and verifies adaptation resumes. Four
production-source GPU tests pass; release build passed and the1100-tick sun-look
diagnostic completed without an error. These captures did not reproduce the
extreme blue/red report, and do not establish visual parity.

The sky still looks overbright and clothing wet; appearance work is
deferred. Do not mark PostFX appearance fixed.
The next requested work is PC Display Settings with separate saved PostFX/
fisheye switches, documented in pc-display-settings-2026-10-02.md.
