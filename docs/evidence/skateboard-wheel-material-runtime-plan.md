# Retained wheel material and live Part slots

Read-only source/runtime design, 2026-09-08. No material implementation or Cargo run belongs to this packet.

## Source setter and caller order

PDB `Skateboard::SetWheelMaterialForSurface`, original `82b29fb8..82b29ff8`, SHA-256 `9d0fb9a07afafab1a83dba1ee6d7e4979fa45c07494b54bba0ad5fdf9a578ee5`:

| Address | Operation |
|---|---|
| `82b29fb8` | Load retained SkateboardBody from board `+0c`. |
| `82b29fbc` | Load processed pointer from board `+110`. |
| `82b29fc0` | Load body Assembly `+18`. |
| `82b29fc4` | Load selected material POINTER once from processed `+908`. |
| `82b29fc8 / 29fcc` | Load Assembly Part list `+08`; store pointer to Part 0 `+50`. |
| `82b29fd0 / 29fd4 / 29fd8` | Reload Assembly/list; store SAME pointer to Part 1 `+b0`. |
| `82b29fdc / 29fe0 / 29fe4` | Reload Assembly/list; store SAME pointer to Part 2 `+110`. |
| `82b29fe8 / 29fec / 29ff0` | Reload Assembly/list; store SAME pointer to Part 3 `+170`. |

Part stride is `0x60`. No material coefficients are copied or recomputed here. No write to truck/deck Parts 4, 5, 6 occurs. No whole-four-part preflight or rollback is present. A checked implementation must preserve already-written slots if a later source-stage resolution fails.

Direct callers are GroundAnimation `UpdateSkateboard` at `82d8a7e0`, SlideGround at `82d8e2d4`, and PhysicsGround at `82d9a12c`. GroundAnimation ordering is already documented in `skateboard-ground-animation-prefix-source.md`; this setter belongs in that recovered prefix, before continuing its subsequent source work. This packet does not recover additional state handlers.

The selected material is one of the five actual `SurfacePhysics` material slots constructed by `skateboard_processed_surface.rs`, selected by Dispatch after its incoming collision type publication and normalization. Its current `SkateboardSurfaceProjection::material` is a copied `AssemblyPartMaterial`; that value is sufficient for bounded coefficient tests, but does not by itself identify the source material pointer or owning SurfacePhysics occurrence.

## Where the material is consumed

The material pointer is a property of the live Assembly Part, not of a rigid body. `DynamicAssemblyPartPayload<I,M>` currently contains authored `material: M` in the immutable attachment template. `DynamicAssemblyPartRuntime<H>` currently stores only transform cache and body handle. `SimulationAssemblyBackend<M,JF,JL>` carries M in its generic relationship but the rigid-body add path passes only inertia and body state; installing material on the body pool would be the wrong ownership boundary.

Current concrete board collision publication makes this gap observable:

1. `skateboard_collision_publication.rs::matching_live_parts` resolves the actual attached Part body occurrence and live pose, but obtains material from `attachment.template.payloads[ordinal].material` (currently around line 443).
2. Its Create path copies that value into `SerialisedAssemblyVolumeDataState`. Its `try_update_volumes` refresh writes `record.material = part.material` alongside enabled, transform, group and frozen fields.
3. `simulation_ground_contact.rs::try_volume` reads the published `volume.material` around line 352, selects against the source ground material, and passes the result into the existing contact/Jacobian composition path. Material tie/unordered selection is already tested in `simulation_contact_jacobian.rs`.

Therefore changing only a processed projection or the backend's generic parameter cannot change actual contact response. The live Part resolver used by collision publication must resolve the newly selected material owner.

This copy at SERIALIZATION time is source-backed. Original `SerialisedAssembly::Update` at `82e0cf30` repeatedly reads the CURRENT Part `m_material` pointer and copies its three words into each volume record. Raw examples:

| Pointer load | Static word load/store | Dynamic word load/store | Restitution word load/store |
|---|---|---|---|
| `82e0d018` | `d01c / d020` | `d024 / d028` | `d02c / d030` |
| `82e0d0ec` | `d0f0 / d0f4` | `d0f8 / d0fc` | `d100 / d104` |
| `82e0d160` | `d164 / d168` | `d16c / d170` | `d174 / d178` |
| `82e0d228` | `d22c / d230` | `d234 / d238` | `d23c / d240` |

`SerialisedAssembly::Create` at `82e0c850` also copies the current Part material words. Preserve that existing value boundary: a published collision record is not a permanently live material pointer and must not be retroactively rewritten by the wheel setter.

The skeleton already supplies a useful ownership precedent: `skeleton_runtime_state.rs::material(part)` resolves the retained Definition/Zero/Normal selection, and `skeleton_collision_publication.rs` consults it at Create and Update. Reuse that principle for board Part slots, without sharing a material occurrence between unrelated skeleton/board owners.

## Minimal proposed mechanism

1. **Retain the five actual source material allocations.** Change the SurfacePhysics material storage to stable, reference-counted owners whose contents are mutable ONLY during the existing one-shot constructor. For example, `[Arc<AssemblyPartMaterial>; 5]` can be created once and updated through exclusive `Arc::get_mut` while constructor readiness forbids issuing tokens. This retains all source constructor partial writes and avoids moving/copying a different material into an owner after construction. The alternative single `Arc<[AssemblyPartMaterial; 5]>` plus slot index is equally valid if the generic Part resolver can retain an indexed owner.
2. **Issue a sealed selected token after readiness.** It contains the exact SurfacePhysics occurrence, selected slot ordinal, and immutable material owner. Dispatch publishes this token at the existing material-publication stage. The same numerical triple from a different SurfacePhysics instance must not silently satisfy owner matching. Keep the existing collection token separate; an attribute collection identity is not the material allocation identity. No lookup or coefficient reread occurs during the wheel setter.
3. **Put mutable bindings on the actual Part runtime.** A minimal generic change is `DynamicAssemblyPartRuntime<H,M>` with a material binding whose initial state resolves the retained template payload and whose runtime state retains an `Arc<M>` token. An optional runtime binding avoids requiring `M: Clone` and leaves all immutable definition/template material bytes unchanged. A single `current_part_material(ordinal) -> &M` resolver chooses template or installed owner. It must become the only live board material source for collision publication; a separate override list consulted only by the setter would duplicate authority.
4. **Bind installation to the actual pair/session.** Validate the selected token against the pair's retained SurfacePhysics occurrence, then execute the four source stores in order on that pair's real board attachment. Keep the source's initial Assembly load before selected-pointer capture, and resolve Assembly/Part membership afresh for each store. The safe-host selected profile may explicitly exclude concurrent native list replacement, as current attachment ownership does; it must still perform the four checked store stages and preserve a prefix on a later failure. Reused/freed/foreign session or body occurrences must not authorize installation.
5. **Keep token identity across the four slots.** All four refer to the same selected immutable owner. Do not create four new `Arc`s each holding a copied triple, reconstruct the material from a collection key, or compare floats to establish identity. Subsequent Dispatch can select another existing token; a later setter replaces the four Part bindings. Parts 4..6 continue resolving their retained authored binding unless their own source setter is implemented.
6. **Refresh consumers at the source publication boundary.** Board collision Create/Update must read `current_part_material`. The volume record retains a three-word snapshot, as original serialization does. Material replacement should invalidate the host freshness proof used to admit a NEW collision run until the required Update has occurred; body-frame identity alone cannot detect a material change in the same frame. Preserve old record bytes until that Update rather than mutating the serialized snapshot from the setter. An assembly material revision tied to successful individual slot stores can supplement the current frame stamp.

This proposal keeps backend `M` generic. Only the selected board specialization needs an `AssemblyPartMaterial` coefficient decoder. Generic Part mutation stores a retained reference; material arithmetic remains in the existing collision/Jacobian consumer. It also keeps DMO definition/template owners untouched.

## Necessary verification before calling the path integrated

- Actual constructor tokens retain the same material occurrence through Dispatch and all four slot stores; foreign same-value tokens fail.
- Four source stores execute in order; injected resolution failure at Part 2 leaves Parts 0/1 changed and Parts 2/3 unchanged. Parts 4..6 remain authored.
- Original immutable template materials do not change. Destroying an external selection handle cannot invalidate a Part's retained material.
- Collision records remain unchanged before Update, reject stale use under the selected lifecycle, and receive the selected raw coefficient bits after actual Update. The downstream ground contact material/Jacobian test must observe those bits rather than only asserting Part state.
- Repeated selection, reset/detach/re-attachment and generation checks follow recovered source lifetime rules; do not infer a material reset during body detach because current detach only copies pose/removes body. Recover any additional source Part reconstruction/reset boundary before adding one.

## HiLOD construction timing clarification

Fresh original raw inspection confirms HiLOD constructs PhysOut at `82e03f20` and stores it at `82e03f38`, BEFORE its Skateboard constructor call `82e0413c` and Skeleton constructor call `82e041c8`. The actual pair may retain a fresh unknown selected PhysOut projection after existing body admission as an explicitly bounded host integration step, but that placement must not be described as native constructor order. There is still one retained output owner and no source output-buffer swap in the inspected route.
