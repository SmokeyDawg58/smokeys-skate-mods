# Vehicle paint and lighting investigation — 2026-09-29

All cars rendered nearly black/dark blue. Work performed solo; existing
renderer diagnostic compilation/runtime runs apply. Preserve earlier
mesh-arena, draw batching, shadow reuse, physics and visibility fixes.

## Confirmed causes and changes

1. The selected core Vault did not load `render_locations` (class
   `6a2b5946514b8490`). `vehicle_visuals` therefore bound a null instance and
   silently published nine zero SH coefficients for every car. Add the class
   to the existing startup selection. Read the default probe through the
   existing attribute loader, once per visual snapshot with present vehicles,
   validating its native nine-element requirement. No new randomizer, material
   registry, lighting manager or texture loader was introduced.
2. The port followed older UPDB HLSL and omitted an ambient minimum present in
   the shipped final body/glass pixel programs. Restore RGB
   `(0.036, 0.040, 0.042)` before the material ambient multiplier, preserving
   larger SH values. This is a native shader constant, not brightness tuning.
3. `vehicle_glass.xml` disables depth writes and has no alpha test. The shared
   pipeline incorrectly wrote depth and the fragment entry discarded tiny
   opacity. Correct those two glass-only states; preserve blending/culling.

## Existing flow checked rather than replaced

- `Sk8::Vehicle::InitModels` **82d1d5f0**: two `rw::math::Random` selections
  modulo the authored paint-array lengths, saved as two color-index bytes.
- `CreatePresVehicle` **82810d40**: reads selected float4s from model attributes
  `df76d7d773857edb` and `12026e2eed18cc8d`.
- `cPresVehicle` constructor **8280fd48**: copies both colors directly.
- `cPresVehicle::BindConstants` **8280b8f8**: binds red/blue colors, light
  selectors and SHLightingMan's **default** probe, not a position-sampled probe.
- `SHLightingMan::UpdateDefaultLightProbe` **82827860**: asserts collection
  exists and attribute length is nine, reads float3s, writes W=0. Full attribute
  key **f45dfcab6ec195cd** confirmed in raw PPC at828278a4..c0 and82827910..2c;
  Hex-Rays' recovered high/low argument halves alone are insufficient.
- `RenderMan::UpdateRenderParams` **828527d8..82852844** changes the
  render-location instance and refreshes that default probe. This selected BAM
  renderer already uses world location **ec763034105b866b**. This change does
  not claim to implement arbitrary render-location switching.
- Existing diffuse mask decoding is correct: square texture RGBA; if squared
  green <0.001225, replace RGB with R*color0+B*color1; preserve squared alpha for
  specular. Actual muscle-car diffuse has a large blue mask, not blue paint.
- Existing normal-map `.ag`, tangent/binormal skinning, SH polynomial and
  material parameter publication were inspected. No speculative shadow or
  lighting-direction tuning was applied.

Raw IDA capture: `vehicle-material-lighting-2026-09-29-ida.txt` alongside this file.

## Final shader evidence

Read original `render.big` final `vehicle_defaultPS.fpo`,
`vehicle_glass_defaultPS.fpo`, glass VS and effect manifests using existing
asset tools. Final binaries take precedence over stale UPDB HLSL:

- Body c253=(.036,.042,.040,.001225); pc112 MAX followed by pc113 xzy swizzle
  gives RGB (.036,.040,.042).
- Glass c254=(.036,.040,.042,-1); pc97 MAX, pc98 multiply and pc104 swizzle
  give the same RGB minimum.
- Ambient multiplier is m_params[5].w, after the minimum, before diffuse.
- Glass manifest: FRONT cull, SRCALPHA/INVSRCALPHA, RGB writes,
  DEPTHWRITEENABLE FALSE. The depth-write alias is already established by
  cDepthStencilStateHandle::SetState82e19368 (see render-cull-bias evidence).
- Older glass HLSL mentions depth bias, but final VS outputs plain dp4 clip
  coordinates (pc22..25); no bias was invented from that stale source.

Disassemblies: `artifacts/vehicle-{body,glass}-ps-disasm-20260929.txt` and
`artifacts/vehicle-glass-vs-disasm-20260929.txt`. Extracted original programs,
UPDB sources, manifests and diagnostic texture dumps are under
`%TEMP%/skate2-vehicle-shaders-20260929`.

## Diagnostics and validation

Opt-in only (normal play writes no additional logs):

- `SKATE2_VEHICLE_MATERIAL_TRACE=1`: logs entity/LOD creation colors/probe and
  new GPU material bindings, not every frame.
- `SKATE2_VEHICLE_SHADE_DIAGNOSTIC=1/2/3`: paint-only / shadow-light components /
  ambient-only diagnostic outputs. Unset/default0 is normal rendering.
- Baseline trace `artifacts/vehicle-material-runtime-20260929.log` proved
  distinct authored colors/material values across muscle car, older sedan,
  minivan and sports car. `vehicle-shadow-components-runtime-20260929.log`
  proved every probe was zero. Diagnostic street image contains shadowed car
  surfaces; this alone is not evidence that the shared shadow projection is bad.
- Production WGSL GPU regression exercises real object uniform packing,
  distinct red/blue masks, untinted nonpaint pixels, zero/negative/higher SH and
  glass ambient multiplier. Mounted bootstrap regression additionally checks
  that original loaded probe is nonzero and W remains zero.

### Final verification

- `cargo test --release -j1 -p skate2-pc gpu_renderer:: -- --nocapture`:
  **37 passed, 0 failed, 1 ignored**. This includes GPU execution of the new
  production material helper and validation/drawing of the full effect pipelines.
  Log: `artifacts/vehicle-material-tests-20260929.log`.
- `cargo build --release -j1 -p skate2-pc`: **passed**. Log:
  `artifacts/vehicle-material-final-build-20260929.log`.
- Original-asset runtime, `offboard-street-look`, 5400 completed simulation
  ticks, normal shading, uncapped presentation: **exit0**. Final image
  `artifacts/vehicle-material-after-20260929.png` visibly contains a red vehicle
  and a light-colored vehicle; baseline image had near-black/blue appearance.
  Vehicle positions/random paint choices differ, so this is not a pixel-identical
  A/B scene or an FPS benchmark.
- Live trace now publishes the actual probe, starting with [.3,.3,.3,0],
  [.01,.01,.01,0], [.2,.2,.2,0], [.3,.3,.3,0], rather than all zeros.
  `artifacts/vehicle-material-final-runtime-20260929.log` records colors and
  probe values from the real simulation/presentation path.
- Independent exact-node-pointer read of original Vault agrees with all nine
  live coefficients. World collection ec763034105b866b inherits default
  d7edbd362d7d2152; key f45dfcab6ec195cd points to dependency1 offset699772
  (header), with data at+8. Source/output:
  `artifacts/vehicle-probe-pointer-20260929.{rs,txt}`. Prefer this exact-pointer
  probe over the earlier exploratory asset dump, which printed adjacent arrays
  too: its neighboring .05 coefficient array is NOT the selected key.
- Added mounted bootstrap regression is ignored by default and was not run as
  a separate core test binary; the real runtime exercised that same startup,
  probe reader and nine-coefficient validation successfully.

Ready: current `target/release/skate2-pc.exe`; normal `play-quiet.bat`.
Diagnostic environment switches were process-local; ordinary play remains
quiet. No compiler or game remains active at completion.

No full renderer parity claim: existing f32-vs-half and host rendering
differences remain; the changes above address proven publication/shader
mismatches without modifying shadow projection or gameplay.
