# Video screens — October 19, 2026

The stadium's two video screens show the game's own view. The port drew them with the
material's diffuse texture. Native addresses are the debug `sk82_na_m.xex` (Jan-17-2009).

## Native

- **The binding.** `cModelInstance::Resolve` (0x82853de8) builds each mesh's effect list
  (`CreateEffectListFromPegasusMaterialUnSkinned` 0x82e20948 and its skinned and
  multi-parted siblings, the only callers of `AddPegasusMaterialToEffectList`). For every
  scope in the list whose effect scope is named `videoscreen_default` (the pooled string
  `k_videoscreen`, 0x83055c40), it sets `i_diffuse` to
  `RenderMan::m_PostFX.m_mainRt->m_colorTargets[0].m_pTexture` (0x82853f90..0x82854028),
  after the material's own channels.
- **The name.** `Technique::Technique` (0x82e1cfb8) names a technique's scope
  `<fx>_<technique>`, so the name is the `videoscreen` fx's `default` technique, the one
  `PresentationEntity::BindConstants` selects for world entities.
- **The meshes.** `cModelInstance`s are made only by
  `PresentationEntity::AddLODModelInstances` (0x827fa1c0). That covers world entities
  (`WorldPresEntity` 0x827d3e90), DMOs, visual indicators, living-world entities and
  characters.
- **The target.**
  - `PostFX::Begin` (0x82823188) starts the scene in `m_mainRt`, and `PostFX::End`
    (0x828289e8) resolves it.
  - In a 60 Hz frame, `ApplyFastUber` (0x828469b8) then renders the graded frame (colour
    cube, bloom, vignette and the AO boxes) back into `m_mainRt` and resolves it again
    (0x82846ca4).
  - `Blit_ToDeviceRT` (0x82831c18) copies that to the device. The HUD (`Render2D`)
    follows on the device target.
  - When the next frame's scene draws, the main target therefore holds the previous
    frame, graded, without the HUD.
  - `InitBuffers_Xbox360` (0x8283ea58) sizes `m_mainRt` by `m_screenWidth` and
    `m_screenHeight`, which `PostFX::Init` sets to 1152 by 640 (`li r5, 0x480` and
    `li r3, 0x280` at 0x8284a908, stored at 0x8284a938 and 0x8284a944).
- **The shader.** The `videoscreen` effect's shipped programs and manifest states are
  baseincandescent's (world-material-audit-2026-09-30.md).

## Data

The BAM presentation stream has two `incandescent.videoscreen` materials, whose manifest
is `videoscreen`, with diffuse, lightmap and chromaticity channels. They are in
`cPres_-100_-2100_high`, which lies inside the stadium interior box that
`cWorldParticleController::Update` keeps the ambient effect out of. The source is
`--material-census`.

## Port

- `apps/skate2-pc/src/gpu_renderer/main_target.rs` holds a 1152 by 640 texture in the
  surface format. Each frame, after `PostFX` (or after the scene when `PostFX` is off), the
  final pass's scene texture is copied into it, in the place of the resolve at 0x82846ca4.
- The copy takes the scene's central 16:9 view, the view the original draws, which other
  window shapes widen around. It box-filters that view with four bilinear taps a texel.
- `render_inspection.rs` gives a `videoscreen` draw's `default` technique that texture as
  its diffuse (`TextureIdentity::MainTarget`), with the material's own lightmap and
  chromaticity, through the baseincandescent pipeline.

## Checks

- The shader parses and validates with naga 30.0.1.
- Clippy reports no new warnings.
- `the_copy_keeps_the_central_sixteen_by_nine_view` passes from a standalone copy.
- In game, spawned at `onlinefreskate_stadium`:
  - the build before the change shows the screen's own texture ("JUMBO TRON TEMP");
  - the build after shows the skater on the ramp from the game camera.
