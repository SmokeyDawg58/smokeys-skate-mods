# Fisheye final scene pass

Status at author handoff: implemented and integrated by root; release Cargo
check passed. Shader/runtime validation and final release build remain with
root. This note does not claim full postFX or reference-runtime pixel parity.

## Inventory and ownership

The app already owns the world/camera renderer, original texture decoder and
uploader, MSAA resolve, APT geometry and HUD draw lists. No fisheye/final scene
pass existed. New `gpu_renderer/final_pass.rs` and `final_pass.wgsl` consume
the existing scene; they do not duplicate the camera, APT, marker, or GPU
texture systems. Root retains `vignette_gradient.Texture` in the existing
boot presentation arena and passes its existing cached `GpuTexture` view.
The peer's `respawn_noise` module owns native snow generation/phase math.

## Native path and coefficients (high confidence)

* `RenderMan::RenderModeScene` 0x82857a78: FE Render3D 0x82857cf8 runs
  before PostFX Apply 0x82857dec; FE Render2D 0x82857ea4 runs afterwards.
  CORRECTION after tracing the callee: Render3D draws world-menu quads,
  NOT the type5 HUD. AptRenderingIntegration::Render2D82631a98 explicitly
  draws AptAnimLevel5 at82631bd0 AFTER postFX. Score and phone HUD retain
  their 3D projection but bypass scene fisheye/vignette/noise. App FPS is
  also after the effect. See score-render-order-2026-09-30.txt.
* `kbEnableFishEye` initializer 0x82f70528 enables it by default.
  `SetUberParams` 0x8282422c..2444 selects identity `(0,1,0,0)` when
  eFishEye bit8 is absent. No timed introduction exists in ordinary freeroam.
* PostFX ctor 0x82839390 / raw 0x828394b8..94f0 binds `postfx` class
  **46f43159bc8272ab**, collection **316a03931530b5e6**. Decompiler
  64-bit keys are incomplete; the raw `insrdi` construction was inspected.
* `SetParameters` 0x8284bc44..bd24 reads z,y,x in that order and publishes
  `(x,y,z,0)` at PostFX+0x4f0. Actual shipped VLT/BIN values:

| Lane | Attribute | Actual value |
| --- | --- | --- |
| x | 03d419b25e35e90c | 0.72 (3f3851ec) |
| y | c83e225b09fc3d00 | 0.9 (3f666666) |
| z | d35e9a015df6c989 | 1 (3f800000) |

`FisheyeParameters::load` uses existing `with_vault` and inheritance/default
resolution. Values are read from the mounted immutable Vault at view startup,
not hardcoded. A live editor mutating this Vault is not currently supported.

* `VisualFX_Animation` 0x82847610 initializes vignette offset `(0,.25,0,0)`.
  Challenge failure can change y; that presentation path is not represented
  here. `Blit_ToDeviceRT` 0x82831d5c sets w to positive bloomCoefficient[0],
  else zero. Native ordinary bloom collection `post_fx/3c71d6b414304f99`
  has threshold .5; `SetBloomCoeefs` 0x828233f8 / 0x82823428 leaves
  coefficient[0] zero for nonnegative threshold. Thus ordinary freeroam w0
  is supported by source plus corpus, not a neutral-mask invention.
* `SetTexturePointers` 0x828287cc hashes `vignette_gradient.Texture`
  (string 0x821316d8) with `RwHash64String` and stores the registered asset
  at 0x8282883c. Final blit samples this original gradient; no generated
  radial darkening texture replaces it.
* PostFX ctor raw 0x828399c4..99d8 sets `mWhiteNoiseIntensity` (+0x9ec)
  to literal zero. It is not a normal freeroam Vault coefficient. OOB record
  presence or positive white-noise intensity selects the snow variant.

## Final shipped shaders (high confidence)

The executable embeds `basictex_fisheye_PS` **0x8212cda8, 584 bytes** and
`basictex_fisheye_noise_PS` **0x8212cff0, 896 bytes**. These were extracted
only to external temporary files and decoded with existing
`tools/xenos_disasm.py`. CTAB plus final instructions take precedence over
older source/UPDB. The shaders use c0=warp and the actual vignette texture.

For p=UV-(.5,.5), r2=p.x²+p.y², the scene sample is:

    p * ((r2 * warp.x) * r2 + warp.y) + (.5,.5)

This is genuine image resampling. Its radius is deliberately NOT multiplied
by screen aspect. Only the separate vignette radius uses native constants
16/9 and -8/9:

    radius = sqrt((UV.x * 1.777777791 - .888888896)² + (UV.y-.5)²)
    vignetteUV = (1 + offset.x - radius * warp.z, offset.y)
    output = sceneSample * saturate(vignetteSample + offset.w)

The standard shader uses MAD for vignette X; the noise shader has separate
MUL/ADD. Independent peer review identified this distinction; the two
fragment entries now retain their respective operation ordering.

The noise shader shares scene warp/vignette, adds original two-layer binary
RGBA snow, and changes all four output channels. Its native dot operand
permutations and repeated dot evaluation are retained. Full peer evidence
is in the respawn-noise packet. It uses wrap/linear snow sampling and
clamp/linear scene/vignette sampling. The current renderer supplies its
existing gamma-encoded non-sRGB scene target; this pass does not add another
gamma conversion. Host pixel centres already supply the D3D9 full-quad
half-texel correction. Non-widescreen native UV crop (.75,1)+(.125,0) is
available, while the current live renderer requests its existing widescreen
camera mode.

## Validation and limits

Raw source capture: `artifacts/fisheye-ida-20260930.txt`.
Final program CTAB/disassembly: `artifacts/fisheye-programs-20260930.txt`.
Corpus: original mounted Jan17 `data/db/skater.vlt` + `skater.bin`, inspected
with the existing `tools/vault_dump.py`; no original binaries are committed.
Both owned Rust/WGSL files are source-inspected; rustfmt parsed the Rust.
`original_fisheye_and_noise_shaders_compile` uses wgpu/Naga/backend validation
for both entries without consuming the shared gameplay RNG. Root runs it.

No reference executable frame was captured, so no exact GPU rounding,
texture filtering, or whole-image comparison is claimed. Xenos and host
floating-point/backend behavior may differ. Existing absent bloom,
auto-exposure, grading, replay/ICE, and challenge effects remain absent;
they are not approximated by this final-blit addition. The current live
game has no pause menu, so root supplies the native unpaused state; the
noise API accepts paused state for that future owner.
# Integrated verification

Root integrated both agents' effects and built the release application.
`artifacts/fisheye-shader-test-20260930.log`: actual wgpu shader test passes.
Scripted marker return exits successfully at424 and600 simulation ticks;
screenshots `fisheye-marker-return-ramp-20260930.png` and
`fisheye-marker-return-cleared-20260930.png` show noise during the return and
its removal afterward, respectively. Scene distortion is active; original3D
phone HUD was incorrectly included before the final pass in these earlier
captures; the September30 follow-up corrects that ordering. Normal `play-quiet.bat` uses this
build without diagnostic inputs. No complete postprocessing parity claim.
