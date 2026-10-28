# Career visual indicators (October 9)

The world indicators the career draws between and during missions: sign-up
location ribbons, objective "go to" ribbons and the follow trail. They run from
the challenge system's `cVisualIndicatorManager` through the VisualDirector
records to `VIPresentation` and the `spline` effect.

## Native chain

### Simulation side (VDWriter)

- `VDManager::DoSimSubmit` 0x82800120 → `VDWriter::WriteData` 0x827ff8c8 →
  `VDWriter::_PrepareChallengeVisualIndicators` 0x827ff138, for the local
  skater's `Actor::GetPosition`.
- `cVisualIndicatorManager::GetVisible*Indicators` walk the groups under the
  main challenge's unique ID (-1 without one), then every other group
  (`GetVisibleIndicators<T>` 0x82db2210, 0x82da8a20, 0x82da8b68, 0x82da8cb0).
  - Locations: `GetVisibleLocationIndicators` 0x82db2568 with
    `AddIndicatorsFromGroup<tLocationIndicator>` 0x82db0c30. Enabled
    locations within `kMaxDistanceToLocationDataType` (cvar 0x82f6ddf8,
    default 500) of the skater are sorted nearest first with EASTL
    `quick_sort` (comparator `<=`), up to 22.
  - Splines: `GetVisibleSplineIndicators` 0x82da93f8 (16).
  - Follow targets: `GetVisibleFollowIndicators` 0x82da9460 (4).
  - Triggers: `GetVisibleTriggerIndicators` 0x82da94c8 (types 1 to 8).
- `AddLocationRibbon` 0x827ff050 writes a location only when both its
  `ribbon_indicator` and `ribbon_spline_colour` keys are in the
  `VDUtils::BuildIndexMap` maps. The record holds the translation and
  `RawDiscretize::QuatCompress` 0x825e9b00 of the orientation
  (`Sk8::CompressedFromMatrix44Affine` 0x825e9ff0).
- `AddRaceGate` 0x827fda90 and `AddBoxInfo` 0x827fef18 write race gates,
  obstacles, Create-a-Spot volumes and boundaries from the trigger instances'
  volumes.

### Records restored (VDReader)

- `VDReader::ReadData` 0x827fdd78; the location ribbon block is
  0x827fdfdc..0x827fe1c4.
  - `Sk8::Matrix44FromCompressed` 0x827fd8c0 restores the matrix
    (`QuatDecompress` 0x825e9e68, `AnimSinCos2` 0x825e9a30, `gSqrt2s` basis,
    W lanes masked).
  - The ribbon's `Offset` is added along the rows.
  - The record keeps the ribbon's spline key (layout +8), the colour key and
    `RenderSettings`.

### Presentation (VIPresentation)

- Constructor 0x828044c8 and `Init` 0x82805030.
- Ribbon resource: `GameModule::OneOffLoadTexture` 0x82735730 loads
  `data\content\world\models\visual_indicators_sk82.rx2`, then
  `AddInstancedSplineInstance` 0x828050a8. The `tVisualIndicatorData` fixup is
  0x829c5f20. The shipped file holds 36 indicators with 1769 segments.
- `SplineContainer` constructor 0x82804348 and `InsertSpline` 0x82804200.
- `Spline::InitFromSplineHeader` 0x82801d00 builds 142 control points (two
  edges of 71).
- `Update` 0x828055e0:
  - `UpdateLocationSplines` 0x82803528 runs the 22 location instances.
  - `UpdateGateSplines` 0x828037e0 runs the 32 race gate instances.
  - `UpdateDynamicIndicator` 0x82803fa0 samples the first follow target's hips
    every `kDynamicIndicatorPointTime` (0.05 s).
  - `UpdateCustomizedSplines` 0x828033a0 and `UpdateCreateASpotIndicator`
    0x82805328.
- `SplineIndicator`:
  - constructor 0x82804cf0 and `Init` 0x828023c0;
  - `CalculateBounds` 0x828010c8 and `SetLocalToWorldMatrix` 0x82800548;
  - `Cull` 0x82801278: billboarding about Y, then `Frustum::TestAABB_VMX`
    0x82825830 against `ComputeFrustum` 0x8282bd28;
  - `Update` 0x828013f8: camera-distance width, jitter and fades, with the
    layers rebuilt by `cParticleRandomManager::Build` 0x828009a8;
  - `Render` 0x82802940.
- `DynamicIndicator`: `AddPoint` 0x82803df0 (`kUpdateDistance` 0.25),
  `Update` 0x82801770 and `Render` 0x82802c00.
- Settings come from the `visualindicators` collections (constructor
  0x828007f0): the ribbon's `RenderSettings`, `default`, and `key_trail` for
  the trail.
- Textures: `cTexturePackManager::LoadTextures` 0x8282ad58 loads
  `vitextures.rx2`. `GetBackTextureAsset` is 0x82800868.

### Rendering

- `RenderMan::RenderModeScene` 0x82857a78 calls `VIPresentation::Render`
  0x82803a18 at 0x82857d2c. That is after `FrontEndManager::Render3D` and
  `ParticlePresentation::Draw`, before `PostFX::End`.
- The `spline` effect (`data\render\shaders\final\spline.xml`), shipped
  microcode checked with `tools/xenos_disasm.py`:
  - First, every indicator draws in `darken` with its back texture: blend
    SRCALPHA/INVSRCALPHA.
  - Then every indicator draws again in `default` with its front texture:
    blend ONE/ONE.
  - Each control-point layer is one triangle strip.
  - Depth test on, no depth write, no cull.
  - The vertex shader evaluates the uniform cubic B-spline. It fades by
    clip-space distance through `i_ClipValues`.

### Location resolution

- `cVisualIndicatorGroup::AddLocationIndicator` 0x82db25d0 copies
  `cLocationManager::GetLocationOrientation` 0x82a8e3b0 at add time. The hash
  form is 0x82a8de48, through `GetLocationInfo` 0x82a8dc40.
- An unknown name gets `DefaultLocationOrientation`: identity at the origin
  (constructor 0x82a8ef20).
- Objectives add their `VisualIndicators` from three places:
  - `cChallengeObjective::SetObjectiveStatus` 0x82d4bac8;
  - `ChallengeAssetLoadComplete` 0x82d42fa0;
  - `OverrideVisualIndicatorStatus` 0x82d42d80.
- m_1a_01's ribbon locators (`m_1a_01_goto_slappys_new_vi01`..`032`,
  `m_1a_01_goto_slappys_vi05`..`019`) are not in `BAM_Sim.loc`, the global
  locators or the BAM streams. The mission's own stream holds them:
  `missions.big` `data\content\missions\m_1a_01_goto_slappys\cSim_Global.xsf`,
  47 locations. For example, `new_vi01` is at (1510.41, 110.78, -717.74).
- That collection registers through `SimThreadMsgMan::ReceiveRegCollectionMsg`
  0x8277e988 → `cLocationManager::RegArena`.

## Ported

- `crates/skate2-formats/src/pegasus_visual_indicator.rs` parses
  `tVisualIndicatorData`.
- `crates/skate2-core/src/challenge/indicators.rs`:
  - adds `visible_locations`, `visible_splines`, `visible_follows` and
    `visible_triggers`;
  - `location_orientation` asks the host's `cLocationManager` first (new
    defaulted `Host::location_orientation`).
- `crates/skate2-core/src/challenge/visual_director.rs`:
  - `prepare`, the writer and reader together, including the QuatCompress
    round trip;
  - the attribute layouts.
- `crates/skate2-core/src/challenge/vi_presentation.rs`: `ViPresentation`, the
  ribbon resource, the location instances, the trail, culling and `Render`'s
  draw list.
- Session wiring:
  - `CareerSession::visual_indicators`.
  - `SelectedVisualSnapshot::visual_indicators`, with the first follow
    target's published hips.
  - `CareerHost` answers `location_orientation` from the career's
    `LocationManager`. That manager holds the global, streamed and hull
    arenas; `LocationManager::orientation` is new.
- PC:
  - `apps/skate2-pc/src/visual_indicator_view.rs` loads the assets once, then
    runs Update and Render per frame at the shown tick. Nothing updates in an
    NIS. A scene's hidden indicators clear the location ribbons.
  - `gpu_renderer/visual_indicator.rs` and `.wgsl` hold the two pipelines.
  - The draw sits after the world UI in the scene pass.

## Not ported

- Race gates, obstacle boxes, Create-a-Spot volumes and boundaries are not
  gathered: the challenge host does not expose trigger volume geometry.
  `UpdateGateSplines` therefore turns every gate off.
- Streamed world spline indicators (`UpdateCustomizedSplines`) are not drawn.
  Their `VD_SplineInfo` records are gathered.
- The Create-a-Spot mesh indicator is not ported.
- The sign-up post-FX treatment is not ported.
- `ChaseArrowMan` (front-end chase arrows) is not ported.
- `VIPresentation::Flush` on game unload is not modelled.

## Host differences

- The jitter RNG has its own LCG stream with the particle manager's seed
  (0x12345678 + 42). Natively the stream is shared with particles, so jitter
  phases differ.
- `i_diffuse` is anisotropic with a POINT mip natively. wgpu cannot combine
  those, so it samples LINEAR min/mag with the nearest mip.
- The frustum uses the window's projection, not the console's 16:9.
- The trail samples the published tick hips, not interpolated ones.
- QuatCompress uses the host's `vrsqrtefp`/`vrefp` estimates.
- The follow trail binds no texture natively (the effect keeps the last one);
  the port skips it until a texture was bound.

## Seeing it in game

- **Sign-up ribbons (between missions):** career free roam with a sign-up
  available. Example: m_1a_03 at `m_1a_03_tutorial_fliptricks_challengelocator01`
  (1640.4, 100.6, -522.4).
  - The ribbon is the `tutorial` `ribbon_indicator`: spline asset
    0x00019af403e38707, billboarded, 2 m above the locator (`Offset`).
  - It uses the `default` colour, which is the orange texture.
  - It fades out from 90 to 100 m (its `RenderSettings` 0x8b41719b1f829fec,
    params [6] and [7]).
- **m_1a_01 objectives:** after the mission's hull loads, the `push` objective
  shows groups of three orange ribbons.
  - The ribbons use spline asset 0x0001ae2403e38707 and are not billboarded.
  - The first group is around (1510, 111, -718), the next around
    (1564, 110, -696), and so on.
  - `meetslappy` puts a billboard ribbon 1 m above `m_1a_01_goto_slappys_vi012`
    (1650.3, 102.7, -519.4).
  - These ribbons also fade out from 90 to 100 m.

## Tests

- core `challenge::visual_director`:
  - QuatCompress codes and round trips;
  - spline kinds;
  - game-data tests (`SKATE2_GAME_ROOT`): sign-up ribbon style and range,
    nearest-22 order, and a challenge stream locator placing its ribbon.
- core `challenge::vi_presentation`:
  - control points, vertices, LCG, trail spacing, frustum cull;
  - game data: the shipped resource and a billboard ribbon's layers.
- formats: `pegasus::visual_indicator`.
- pc: `gpu_renderer::visual_indicator` (WGSL parse, record size).
