# Remove postfx lens reflections - October 12, 2026

## Requested visual change

After a review of the rebuilt PC image, the task changed: remove the colored screen lens reflections altogether, while
retaining the illuminated sign's ordinary bloom and the other postfx. This is
an explicit exception to original-game visual parity, like the separately
requested Hall of Meat full-speed change. PC only; Android remains on hold.

The preceding sampling corrections did not fully eliminate the reported
PC/RPCS3 difference. Do not describe that original discrepancy as diagnosed or
fixed by an exact parity change. The requested removal supersedes further
investigation of the reflection strength; an RSX capture is no longer required
for this task.

## Original boundary checked

Live IDA `Sk8::Render::PostFX::GenerateBloomBuffers` **0x82832c68** first
calls `Blur9_combined3` **0x82829520**, then selects the reflection shader at
**0x82833268..280**. It binds the combined bloom texture at
**0x82833294..2b8**, draws a quad with no explicit UV shift at
**0x828332bc..2c4**, and resolves its output at **0x828332c8..2d8**.

The actual Xbox pixel program **0x82130038** contains:

- Slot 8: the untransformed base bloom texture sample.
- Slots 9..14: six scaled/reflected texture samples.
- Slots 15..32: RGB intensity, authored color, alpha mask and accumulation.
- Slot 33: base bloom plus the accumulated reflections.

The prior artifact `.scratch/postfx-brightness-20261012/shaders.txt` records
the disassembly. `SetBloomCoeefs` **0x828233b0** and the shipped flare Vault
collection supplied the colors/scales. The follow-up also checked the PS3
executable's corresponding producer **0x425a48**: its six colors use the same
`1 - m_RadialBlur` factor at **0x4261bc..2e4**. The live Xbox reference remains
the reconstruction baseline; these checks do not establish matching PS3
runtime texture bindings for the screenshot.

## Implementation

- Replaced the GPU `ghost` entry point with `bloom_copy`. It keeps only the
  native base sample, with the same UV phase, bilinear filtering, quarter-size
  intermediate and UNORM storage. Keeping this resample avoids shifting or
  sharpening ordinary bloom as a side effect of removing the reflections.
- Removed the six mirrored/scaled samples and all their color accumulation.
  There is no reflection-strength tuning value or gameplay-only exception.
- Removed the unused reflection Vault loads, asset fields, constant uploads
  and trace output. Updated the two existing asset fixture initializers.
- Both normal `grade` and cinematic `visual` still consume target 7. Replay,
  ICE and Hall of Meat therefore use the same reflection-free bloom. The
  diagnostic bloom view also reads that target. Stage indices did not change.
- Extraction, fine/medium bloom, coarse exposure readback, thresholds,
  saturation, radial blur, color cubes, material lighting, vignette, HoM
  treatment, and simulation/physics were not changed by this patch.

## Review and status

Three production files changed: `gpu_renderer/postfx.rs`,
`gpu_renderer/postfx.wgsl`, and `render_postfx.rs`. Two existing fixture
initializers changed; no test cases were added or run. Reviewed the complete
before/after diff and normal/cinematic/diagnostic bindings. Rust source syntax
parsed successfully with rustfmt using stdout only; no formatting rewrite.

**Source only: not compiled or checked in gameplay.** The existing release
executable still contains the reflections until rebuilt. The earlier build
success applies only to the earlier sampling/HoM changes.

Confidence: **exact** for the removed original shader contribution and retained
source sample; **high** for the shared host-path wiring. Runtime appearance is
**unverified**. This change intentionally sacrifices reflection parity and is
not a claim of complete original-game postfx parity.

Backups and the scoped diff are in
`.scratch/postfx-lens-reflections-off-20261012/`. Original shader/game data stay
external or in ignored research. The scratch PS3 executable copy used for
reference inspection is under `.scratch/postfx-brightness-20261012/ps3-cpu/`.
