# Hair during the respawn fade: draw order (2026-09-27)

Symptom: the hair is invisible after a respawn; it must also take the
opacity/transparency of the skater's respawn fade.

Status: source written and statically reviewed; rustfmt PASS on every new
hunk. NOT compiled, NOT tested. Same batch: the `proxies` initializer missing
from `load_runtime_presentation_scene` (compile error E0063) is added.

## Native contract (IDA, debug `sk82_na_m.xex`)

- `SkaterPresEntity::Update` 0x827e8730 reads `mIsGhosted` (+0x61a) and calls
  `SetTechnique(RenderMan::GetGhostEffectHash())` for every part.
- `cEffectList::SetCurrentEffect` 0x82e1c210: when the list has no such
  effect it looks up `GetDefaultEffectHash` ("default") and returns 0. The
  hair (`defaulthair`) has no `ghost` technique, so it keeps `default`.
- `defaulthair.xml` `default`: pass 0 blended (SRCALPHA/INVSRCALPHA), no Z
  write, CULLMODE BACK; pass 1 the same with CULLMODE FRONT; pass 2 COLORWRITE
  0 with depth write. No STENCILFUNC and no ALPHATEST, so nothing masks it. Its
  pixel alpha is `i_alpha(UV1) * i_params[0].x`, so the hair fades with the
  skater (already ported, `fs_hair`).
- `defaultcharacter.xml` `ghost`: pass 0 depth only (COLORWRITE 0), pass 1
  SRCALPHA blend with Z write.
- `SceneRenderView::Sort` 0x82839b38 stable-sorts every mesh context by
  `InstanceSortKey`; `SceneRenderView::RenderMeshes` 0x82850670 draws
  single-pass meshes in that order, then the multi-pass ones pass-major in the
  same order.
- The key's first byte is the material's draw order:
  `cMaterialManager::CreateEffectListFromMaterialKey` 0x82e203f8 follows the
  `DrawOrder` RefSpec to a `pass` collection and reads its `GUID` (0x8d15d435bdd3dd6c,
  layout field 0) for every effect of the list, `ghost` included.

## Data (Vault, identical Nov-28-2008 and Jan-17-2009)

tutorial_male's materials: `character.default_cloth` (feet, hat, inner/outer
torso, organ, pants), `character.default_cloth_ropa` (outer torso),
`character.default_skin` (arm, rostral, skateboard), `character.default_hair`
(hair).

| Material | DrawOrder | pass GUID |
| --- | --- | --- |
| default_skin / default_cloth / default_cloth_ropa | `pass/0x4aa1f13c69033ff2` | 9 |
| default_hair | `pass/0xba85ec1ff1a585e5` | 10 |

## The defect

While ghosted, every body part is two-pass and the hair three-pass. The port
drew the multi-pass group in recipe part order. Where the hair precedes the
head or hat, their `ghost` pass-1 blend (drawn at their own pass-0 depth)
paints over the hair, which writes no depth; the higher the fade opacity, the
less hair shows. The original draws the hair (10) after every body part (9)
in each pass, so it blends over them and fades with them.

## Port

- `character_material_admission.rs` (core): each mesh entry keeps its
  material's Attribulator class/variant; `CharacterMaterialView::material_key`.
- `selected_character_material.rs`: resolves every key's `DrawOrder` through
  the app's `load_material_attribute_bindings` (the resolver the map uses:
  RefSpec -> `pass` `GUID`), as a sort-key byte; the core runtime-Vault
  getter rejects constructed RefSpec nodes, so it is not used here.
- `character_palette.rs` / `character.rs`: `CharacterMeshAdmission::draw_order`
  lands in `CharacterBinding::draw_order`.
- `gpu_renderer.rs`: the single-pass loop and the pass-major multi-pass loop
  run over the character objects stably sorted by draw order.
- `live_skating_view.rs` / `live_skating_client.rs`: the view keeps the mounts.
- Not modeled: the effect/material scope bytes of the key; equal draw orders
  keep part order (the skater parts are all 9).
