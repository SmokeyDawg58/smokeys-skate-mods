# Missing sky: `material_sky`'s inline `effect` node (2026-09-27)

**Status:** fix written, rustfmt PASS on every new hunk. NOT compiled or tested.

## Evidence (playtest run, `render-log.txt`, UTF-16)

- Every `original environment shading draws` line has no `Sky` key. There is no `loose presentation assets not loaded` line, so the loose skybox pair loaded without error.
- The first build accounts for every queue submission: 415 original plus 252 diagnostic = 667, equal to `mesh_uploads=667`. The loose skybox arena therefore contributed no counted submission.
- `DIST_skybox.rx2`, decoded offline with the same layouts as `rw4.rs` / `pegasus.rs` / `render.rs`:
  - InstanceData has 3 instances.
  - `skyboxSphere_1000...` has render model entry 13 (1 OptiMesh, bone `skyboxSphere`).
  - `DIST_skybox_Fuse_Seed_0_Split_1` has entry 42 (4 OptiMeshes).
  - The materials are `environmentsimple.background`, `environmentsimple.default` and `sky.default`.
  - So the queue does build submissions for it. The only silent exit in `prepare_mesh_submission` is **Deferred**: a binding whose manifest is the literal `"default"` becomes a deferred submission, which the live map build never draws or counts.

## Root cause

- **Vault data** (`skater.vlt/.bin`, offline):
  - `material_environmentsimple/{background,default}` resolve through the collection layout pointer to `defaultenvironment` (ported Default).
  - `material_sky/default` holds `effect` as an explicit **inline** node: node flags 0x40, slot 0x58fe0. The slot's own `PtrN` relocation points at the string `"sky"` (bin 0xc3e2).
- **Port bug.** `resolve_collection_entry_location` (`apps/skate2-pc/src/main.rs`) always treated an explicit node's slot as a stored pointer and returned its relocation target.
  - For an inline node, that target is the string itself.
  - `resolve_material_manifest` then looked for a pointer at the string bytes, found none, and returned `"default"`.
  - The sky became Deferred.
- **Native reference.** `Node::GetPointer` precedence is inline, layout, static, then stored. The core `AttributeRuntime` already implements this: its 0x40 branch reads the slot itself, and it uses `node_flags` (`entry_flags` is not consumed).

## Fix

- `resolve_collection_entry_location` returns `(0, slot)` for a node with `INLINE_COLLECTION_VALUE_FLAG`, and dereferences only stored-pointer nodes.
- **Other callers are unaffected.** `resolve_attribute_value` handles inline values before calling it; DrawOrder references (16 bytes) cannot be inline; and the render-environment float reads are all larger than 4 bytes.
- **Diagnostic.** The per-build line now ends with `deferred default-effect submissions=N`.

## Expected side effects

These inline `effect` nodes were ALL mis-resolved to `"default"` and Deferred:

| Collection | Effect |
|---|---|
| `material_sky` (23 collections) | `sky` |
| `material_tree/default` | `tree` |
| `material_decal/default` | `decalzbias` |
| `material_glare/default` | `glare` |
| `material_basic/default` | `baseincandescent` |
| `material_visualindicator/default` | `visualindicator` |
| `material_ocean/default` | `ocean` (loose water pair not loaded) |
| `material_proxyworld/default` | `proxyworld` |

Every one of these manifests ships in `render.big`, so none can fail the manifest load.
- World materials in those families that were previously invisible can now draw. `sky` uses the ported sky effect; the rest use the diagnostic path until ported.
- Native routing of visual indicators (`VIPresentation::Render`) and glare was not re-checked. If such meshes appear where the original hides them, that is the next item to trace.
