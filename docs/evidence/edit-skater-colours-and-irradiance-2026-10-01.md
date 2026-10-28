# Edit Skater controls, color publication and skater irradiance

Fresh IDA queries
use the Skate 2 debug IDB. This is a focused parity audit, not a claim that the
whole renderer/editor is pixel-identical to the original.

## Controls and customizations

The original `highlight_morph.SetState` APT at 002194..0021ff changes BOTH the
row state and `mKnob` timeline. Previously the port left each knob in its initial
lit state. The shared morph control now selects the knob's `on`/`off` label;
this covers body, ethnicity, eyes, eyebrows, nose, mouth and jaw controls.
The original asset regression exercises repeated selection changes and ticks.

RawPartDataV6 starts its 43 string fields at byte28. PDB `pEyeColour` is byte48
(index10), not index8 (eyebrow color). Deck color is byte38 (index5), not index4
(costume ID). `GetAlternateEyeColourPartID82b66510` keeps the current part when
its color already matches, otherwise selects the first original part matching
gender/color. `GetAlternateBoardColourPartID82b66fa8` selects the first
non-gameplay logoable deck with the requested color. These now change the
assembled recipe's actual material IDs rather than only the HUD selection.

`SetColourForZone82599438` sets custom-color bindings to the CURRENT installed
hat/hair/organ/outer torso/sock. Stale garment IDs could silently disconnect a
new color from a newly worn item. Existing assembler retargeting remains the
owner of the skin/rostral/required-part relationships. The original OnKey color
strip clamps at its endpoints; it does not wrap like a style selector.

Fresh `CASColourHelper::GetColourSettings82589fb0` reads baseRGB at32 and
highlightRGB at0 unchanged; no invented gamma conversion is applied.
`CASMorphParams::Init825916c8`, `CACRecord::SetMorphZone82b63720`, and
`CACPresEntity::CreateBlendJob827e89d0` were rechecked: existing morph defaults,
authored limits, sorted target buffers and contiguous multipliers are retained.

## Lighting ownership and native evidence

Source capture: `artifacts/edit-skater-sh-native-20261001.txt`.

- `SHLightingMan::UpdateDefaultLightProbe82827860`: nine float3 coefficients
  from render_locations; `use_world_sh_probes` is layout byte1.
- `tIrradianceData::Fixup829c7bb0`: count at0, object-relative array pointer at4;
  each probe160 bytes (nine float4 coefficients followed by float4 position).
  RW4 dictionary type00eb0024, registered asset typeea34.
- `AddHullLightProbes8284ea88`: up to12 resident grids, first1681 probes,
  41x41 XZ grid, last authored probe wins when cells coincide. Grid dimensions
  use exact scalar bits3f831f38; expanded 3D query bounds are +/-15.
- `GetClosestLightProbe8284e6f0`: ordered asset-ID grids; rectangle +/-15 in
  XZ, at most512 descriptors INCLUDING empty cells, X-outer/Z-inner traversal,
  nearest squared3D distance with strict comparison. Default if none.
- `SHLightProbeCache::FindLightProbe8284a6c0`: six copied probes, strict
  distance-squared<6.25, promote selected entry to MRU. RemHull8284fcd8 removes
  grids but deliberately retains copied cache values.
- `kbRenderEnableWorldSHProbes82f6f840` defaults true. The host retains that
  native default; the original debug CVar is not yet user-exposed here.
- `SkaterPresEntity` constructor827e5be4 initializes coefficients from default.
  Update827e8568 samples GetPosition, adds topLightHack to row2 using bits
  3d4ccccd/3d4ccccd/3d54fdf4/0, then old*.98 + target*.02. BindConstants827dcf50
  publishes nine rows. CACPresEntity::Update827e8fc8 calls this same update.
- `SkaterPresEntity::GetPosition827d79c0` calls the presentation callback's
  `GetHipsPos`; the existing published hips position is the correct sample
  point, not the camera or a guessed model origin. Fresh capture:
  `artifacts/edit-skater-lighting-position-20261001.txt`.
- Pedestrian8280b5d0 and vehicle8280b8f8 BindConstants explicitly use the
  DEFAULT probe. They must not inherit the skater's position lookup by guess.
  Their existing ambient bindings remain unchanged. Generic livingworld
  BindConstants8280b4c8 binds material LOD/instance/vertex parameters; no SH
  lookup occurs there. DMO rendering retains its existing material/world-shadow
  path; no second prop shader or lighting registry is introduced.

## Host changes

Streamed presentation collections now retain decoded hull probes with their
original asset IDs. Grid publication/removal follows completed map generations;
pending background map builds retain the prior grids. Grid construction occurs
only on scene publication, not each rendered frame. No separate asset loader,
texture registry or chunk streaming policy was introduced.

The selected skater owns smoothed coefficients, passed to the existing CAS
and ordinary character material uniform. Repeated renders of a completed tick
do not accelerate interpolation. Editor presentation uses its existing elapsed
time with a60Hz accumulator. This is the host presentation cadence adapter,
not a reconstruction of the original Xbox thread scheduler.

Ordinary defaultcharacter/defaulthair/defaultcharacter_ropa previously stayed
on the diagnostic diffuse pipeline. They now reuse the already implemented
character_surface normal/SH/key/rim/shadow path (also used for pedestrians).
Normal channels are admitted through the existing material owner. CPU cloth
keeps its prepared rigid-root validation and matching52/60-byte lit streams.

CAS final shader ambient floor and lighting/tone-map structure are retained;
older UPDB source alone is insufficient to remove the floor present in final
microcode. The prior PC world brightness multiplier is not changed.
Complete native tone mapping/auto-exposure is still a separate known gap.

The material regression suite also exposed a prior header-admission regression:
`try_construct_from_header` truncated peer IDs before the cloth-disable decision.
Fresh raw UpdatePresEntity827e6298..62a8 is `ld` then `cmpdi -1`, despite
Hex-Rays displaying LODWORD. Restored the full64-bit comparison. The existing
header regression intentionally distinguishes 00000000ffffffff from -1 and
must NOT be weakened to accept the decompiler's truncation.

## Verification

Original male/female eye and deck palette-to-recipe check passed, covering
every authored palette entry and endpoint clamping. The shared original APT
regression passed, including repeated morph-knob selection/unselection across
120 timeline ticks per change. Both SH unit checks passed (nearest/cache/default
behavior and repeated-render interpolation guard). Original mounted Waterfront
assets yielded nine registered grids and 7266 sampled positions with nondefault
irradiance. Authored world probes are enabled for gameplay and disabled for CAS;
the CAS default coefficients are deliberately different from the world default.

Release PC test target compiled successfully. After the full64 peer correction,
all six existing material-admission regressions passed; the original palette
test was rerun successfully against that rebuilt core. Core regression build
log: `artifacts/edit-skater-material-recheck-20261001.log` (12m48s).
Final release executable compiled successfully (7m06s), log
`artifacts/edit-skater-irradiance-final-build-20261001.log`.
The standalone irradiance format test passed, including relative pointer,
160-byte stride and truncated-buffer rejection.

Two live raw-controller runs traversed Start -> Edit Skater -> Body Mods ->
Face Modification -> Eyes and selected opposite palette endpoints. Both exited0;
captures `artifacts/edit-skater-eye-first-20261001.png` and
`artifacts/edit-skater-eye-last-20261001.png` were visually inspected: the iris
changes green/brown, Color alone is highlighted, and Rotation/Width/Height knobs
remain dim without their selection outlines. Corresponding `.log` files retain
the selected palette indices0 and4. These runs use the ordinary renderer/material
publication, not test-only recoloring. Saves isolated under
`artifacts/edit-skater-diagnostic-saves`; no real save was touched.

Final editor Finish/save -> gameplay run passed (exit0,622 completed live ticks),
log/capture `artifacts/edit-skater-irradiance-roundtrip-20261001.*`. The returned
Waterfront skater rendered successfully using the updated material/lighting
path; capture inspected. No game or compiler remains running. `play-quiet.bat`
can launch the built release executable. The targeted fixes and asset flows
are verified; this is not complete editor/renderer pixel parity. In particular,
full native room tone mapping/auto-exposure remains a known separate gap.
