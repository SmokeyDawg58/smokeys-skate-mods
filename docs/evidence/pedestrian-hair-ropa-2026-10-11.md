# Pedestrian `defaulthair_ropa` — 2026-10-11

## Request and status

Failure: `pedestrian effect defaulthair_ropa requires its original GPU
implementation`.

The pedestrian and colorized cutscene material dispatchers rejected this
effect. Other skater paths admitted it as ordinary hair, losing a real final
pixel-shader distinction. Added `CharacterShaderFamily::HairRopa` and its
production surface pipeline, connected all affected material/layout consumers.

**Confidence: high (source reconstruction); exact manifest values and final
program register/mask observations.** No compilation, GPU validation, tests
or gameplay run. This is not a pixel-identical
or runtime-verified claim.

## Original evidence

Original assets are external in
`<game folder>\data\big\render.big`, under
`data/render/shaders/final`. Extracted only to the task scratch directory for
inspection with the existing `extract_big.py`, `updb_source.py` and
`xenos_disasm.py` tools. No original binaries/assets are production additions.

| Asset | SHA-256 |
| --- | --- |
| `defaulthair.xml` | `fd5fe6d2baae3ad11b43c93d5753849789d138ea7c1db75eed9378452adb6d63` |
| `defaulthair_ropa.xml` | `efed6380ee59deea4848df9df341874a3e70975f30c2478806bfc07ed41429bb` |
| `defaulthair_defaultps.fpo` | `6011af05d4773c46ef03132c3fe5531fa9bae570436fea3ba16dd347c7281641` |
| `defaulthair_ropa_defaultps.fpo` | `5eec62a40a2cfd87c91cdbc1a5316fc3bd2eef705f0b4f442c11932a797fc260` |
| `defaulthair_ropa_defaultvs.vpo` | `5f2a6064976411a5c0068ff91adf92c1f57d7ab5a1251189fd1ab2aef55cca30` |
| `defaulthair_ropa_simplevs.vpo` | `e8182b354f1ab82c105991c4240284c50a57fcbcafb35d00d281a8c807fe6b03` |

### Binding ownership checked in live IDA

- `cLivingWorldPresEntity` constructor `0x82807a20`: load `1.0` at
  `0x82807aa4`, splat and insert X with `vrlimi128` mask 8 at `0x82807ae8`,
  store `i_vtx_params` at `this+0x220` at `0x82807af0`.
- `cLivingWorldPresEntity::BindConstants` `0x8280b4c8`: binds
  `i_material_lod`, instance `i_params`, and that `i_vtx_params` for every
  current-LOD model instance. The ropa X=1 pedestrian path is grounded in
  this owner, not inferred from the selected skater's cloth state.
- `cPresPedestrian::BindConstants` `0x8280b5d0`: calls the base binding,
  supplies colorization, SH and `i_doGPUSkin`. Pooled-name initializers
  `0x82f6ebf0` and `0x82f6ee00` establish that `i_vtx_params` and
  `i_doGPUSkin` are distinct parameters.
- `SkaterPresEntity::BindConstants` `0x827dcf50`: skater parameter ownership,
  including `i_vtx_params` at +0x910.
- `ColorizedSkaterPresEntity::BindConstants` `0x827dd1a0`: calls the skater
  base and binds its two colorization vectors to current-LOD models.
- Existing pass-major mesh ordering is retained through the shared renderer
  (`RenderMeshes` `0x82850670`).

### Manifest states

Both hair manifests' `default` techniques use these three passes:

| Pass | Cull | Color / alpha blend | Depth write | Color write | Stencil |
| --- | --- | --- | --- | --- | --- |
| 0 | BACK | SRCALPHA / INVSRCALPHA | false | RGB | REPLACE 255 |
| 1 | FRONT | SRCALPHA / INVSRCALPHA | false | RGB | REPLACE 255 |
| 2 | FRONT | ZERO / ONE | reset default true | 0 | REPLACE 255 |

No hair pass enables alpha testing. Diffuse and normal override sampling to
anisotropic 4 with linear mip. Alpha has no override: wrap, linear min/mag,
point mip from the existing pass-reset implementation. Neither manifest has
a ghost technique, so both retain their blended default passes with instance
opacity. Shadow passes have BACK culling, color write 0 and stencil replace
255. Their simple programs use the same geometry transform on the GPU arm.

Ordinary hair additionally declares crowd; ropa declares debug. Neither was
substituted for the production default/shadow techniques.

### Final shader programs

The UPDB contains embedded HLSL, but final compiled shader math is the
authority where they differ. Both default PS programs have the same CTAB and
literal constants and 960 bytes of microcode, with different allocation.

- Ropa default VS: `i_vtx_params` is c7; pc18 compares c7.x > 0. GPU branch
  pc20..50 uses the original four-bone skin rows, OffsetPos `(0, .8, 0, 2)`,
  tangent/binormal frame, UV0 and UV1. The other branch uses `i_matWorld`.
  Shadow VS likewise branches on its c4.x.
- Pixel programs: original normal-map decode, SH floor at pc55, saturated
  rim at pc51, specular clamp at pc67, fog/tone/exposure through pc78.
- Ordinary hair pc61 squares **RGBA** into r4; pc67 multiplies specular by
  **r4.w**, the squared diffuse alpha.
- Ropa pc61 squares **RGB only** into r2; pc67 multiplies specular by
  **r6.w**, the original diffuse alpha. Its dedicated WGSL entrypoint
  preserves this distinction.
- Both output `i_alpha`'s red channel sampled with UV1 times instance
  opacity. This alpha texture is separate from the diffuse specular mask.
- Shadow PS decoded operations agree; container metadata differences are
  not grounds for inventing a different shadow surface.

## Production changes

- `pedestrian_render.rs`: admit the actual effect for ambient pedestrians and
  colorized NIS characters, require its original diffuse/normal/alpha
  textures, seven material rows and original palette. Correct pedestrian
  X=1 documentation to its actual constructor.
- `gpu_renderer/character.rs`: explicit HairRopa identity; shared UV1/alpha
  admission; no ghost technique; ropa parameter guard; correct dynamic
  surface vertex layout for prepared CPU cloth updates.
- `gpu_renderer/character_surface.rs`: use the distinct unsquared specular
  mask, sharing the already recovered hair lighting/coverage program.
- `gpu_renderer/character_stamp.rs`: three HairRopa pipelines with the
  original hair pass states and the proper surface samplers.
- `gpu_renderer/shadow_map.rs`: use the matching 60-byte lit hair or 36-byte
  diagnostic hair layouts, retaining existing shadow ownership.
- `selected_character_material.rs`, `gpu_renderer/character_palette.rs`:
  route selected/AI skater `defaulthair_ropa` to this same family, with
  material-effect identity checking. `cac_hair(_ropa)` keeps its separate
  original CAS shader. Existing prepared CPU cloth/root-row adaptation is
  retained; missing CPU publication is still rejected.

## Inspection and limits

Static review covered all CharacterShaderFamily matches, material admissions,
alpha UV layouts, opacity/ghost selection, pipeline entries, sampler selection,
CPU stream refresh and shadow layout consumers. Diffs were compared against
pre-edit backups. Rust syntax was parsed with rustfmt (`--emit stdout`, output
discarded, children skipped); this does not check Rust types or WGSL validity.
No tests were added/run, no cargo commands or game launches were made, and no
save files were touched by this work.

The shared renderer remains a host GPU reconstruction: existing camera-relative
floating point and WGSL arithmetic/transcendental differences are not removed
by adding this effect. A build and a return to the failing pedestrian
area are still required to validate the actual GPU pipeline. Cutscene,
selected-skater CPU cloth and shadow variants also have source coverage only.

Scratch and backups: `.scratch/pedestrian-hair-ropa-20261011`; the two final
PS disassemblies are retained there beside the inspected assets.
