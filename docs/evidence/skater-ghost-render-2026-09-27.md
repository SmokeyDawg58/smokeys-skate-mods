# Skater ghost technique in the renderer (2026-09-27)

**Status:** rustfmt PASS on every new hunk. The WGSL was reviewed by hand line by line; it is validated only at runtime. NOT compiled or tested. The core half (the opacity itself) is in `skater-respawn-opacity-2026-09-27.md`.

## Native contract

1. **Technique selection.** `SkaterPresEntity::Update` 827e8568 runs from `SceneRenderView::Cull` 828500a8 (vtable+0x10) after this frame's DoSimRestore. At 827e8730 it calls `SetTechnique("ghost")` (`RenderMan::GetGhostEffectHash`) when `mIsGhosted`, else its current technique (+0x154).
   - `cEffectList::SetCurrentEffect` 82e1c210 falls back to the `default` effect when the list has no such technique.
2. **Manifests** (`data/render/shaders/final/*.xml`).
   - `defaultcharacter` and `defaultcharacter_ropa`: `ghost` has two passes with vertex program 0 and pixel program 1 (the default programs).
     - Pass 0: CULLMODE FRONT, COLORWRITE 0.
     - Pass 1: ALPHABLEND, SRCBLEND/SRCBLENDALPHA SRCALPHA, DESTBLEND/DESTBLENDALPHA INVSRCALPHA, BLENDOP/BLENDOPALPHA ADD. ALPHATESTENABLE with ALPHAFUNC GREATEREQUAL and ALPHAREF 1. COLORWRITE RGB. Stencil REPLACE 255. CULLMODE FRONT (the same as `default`).
   - `defaulthair`: **no** `ghost` technique, so it keeps `default`. That technique has three passes:
     - pass 0: blended, no Z write, CULLMODE BACK;
     - pass 1: blended, no Z write, CULLMODE FRONT;
     - pass 2: COLORWRITE 0 depth pass.
3. **Pixel alpha.**
   - `defaultcharacter.fx:72`, `defaultcharacter_ropa.fx:84` and `cacstamp_skin.fx:149`: `outcolor.a = i_params[0].x`.
   - `defaulthair.fx:61-64`: `outcolor.a = tex2D(i_alpha, UV1) * i_params[0].x`.
   - `i_params` (+0x870) is bound by pointer to every part, so the draw reads this frame's DoSimRestore value.
   - In `default`, `i_params[0].x` is always exactly 1.0, because ghosted is defined as final < 1.0. So the existing opaque draw is unchanged when not ghosted.
4. **Multi-pass order.** `SceneRenderView::RenderMeshes` 82850670 draws single-pass meshes in sort order, collecting every mesh with more than one pass. It then draws those pass-major: for each pass index, every multi-pass mesh that still has passes, in collection order.
   - So all of the skater's ghost pass-0 depth draws come before any pass-1 blend.
   - Pass 1 therefore blends only the nearest skater surface, one layer, with no see-through limbs.

## Port

### `gpu_renderer/character.rs`

- `CharacterShaderFamily::has_ghost_technique`: Character and Ropa have it; Hair does not.
- `CharacterGhostState { opacity, ghosted }` is carried on `CharacterPoseRows`. The Object uniform colour alpha holds `i_params[0].x`; `fs_main` ignores the colour.
- Pipelines:
  - `ghost_depth` = vs_main + fs_main, write mask empty.
  - `ghost_blend` = vs_main + `fs_ghost`: `SOURCE_ALPHA_BLEND` for colour and alpha (shared with `transparentenvironment`), write mask COLOR, and `discard` when alpha < 1/255 (ALPHAREF 1, the port's existing alpha-test convention).
  - Culling stays off and depth stays LESSEQUAL with writes, as for every ported effect (GRAPHICS.md deviation). There is no stencil attachment (Depth32Float), as for `default`.
- WGSL:
  - `Output.position` is `@invariant`, so pass 1's LESSEQUAL test sees exactly pass 0's depth. naga 0.19.2 emits `precise` (HLSL) or `Invariant` (SPIR-V).
  - The fragment entries now take only `@location(0) uv`, so no invariant builtin appears in a pixel-shader input. naga sorts interface members location-first, so the D3D signatures stay compatible.

### `gpu_renderer.rs` draw loop

- Single-pass objects draw first, in order, as before.
- Then, for pass 0 then 1, every object that `CharacterPipelines::ghost` selects (ghosted, and its family has the technique) is drawn with that pass's pipeline.
- `draw_object` holds the shared bind/draw code.

### Plumbing

`SelectedVisualSnapshot::{skater_opacity, skater_ghosted}` flows through `live_skating_view.rs`, then `selected_character_material::{create_objects, update_object}`, then `character_palette::{create_selected_character_object, update_selected_character_binding}`, then `pose_rows`.

## Known gaps

- **Hair.** Native hair fades during the ghost through its always-blended `default` technique. The port's hair is still the opaque diagnostic draw: the `i_alpha` channel on UV1 and the three blended passes are unported. So the hair stays opaque while the body fades. The next renderer step is porting `defaulthair` `default`.
- **Colour.** The pixel colour is still the diagnostic unlit diffuse, for both default and ghost.
- **Stencil** REPLACE 255 has no port counterpart for any character pass.
- **Draw order.** Native defers multi-pass meshes to the end of the `RenderMeshes` range. The port draws its ghost passes after the map and all single-pass objects, which is the same relative order within the one range the port has. The `RenderMeshes` call ranges and caller were not traced.
