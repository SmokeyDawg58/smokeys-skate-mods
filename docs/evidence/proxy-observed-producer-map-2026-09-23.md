# Observed PROXY producers and the missing ancestor census

Read-only source/caller audit. No implementation or Cargo run. Actual input is
`artifacts/proxy-census-wake-2026-09-23.log`: 148 XSM descriptors, 147 XSS rows,
292 XST asset identities; direct spatial enumeration retained 147 collections
and decoded 234 arenas. Hex counts `93`, `1e2`, `57` mean 147, 482, 87.

## Actual producer mapping

| Observed TOC type | Count | Native processing | Presentation registration |
| --- | ---: | --- | --- |
| `00eb0005` | 147 | `ProcessMaterial82d5ebd8`: same full TOC GUID/data pointer, Pres kind2 at `82d5ec10`, append immediately at `ec1c` | `RegArena827f1580` -> `TextureMan::AddMaterial8282c838` |
| `00eb000d` | 147 | `ProcessInstanceList82d5ec38`; only observed record subtype is `Instance_Environment=acb31c9a` | Admitted Environment becomes kind10; `RegArena827f1590` -> actual unlock router/world |
| `ac462e4a` | 87 | `Texture` hash; `ProcessTexture82d5ead8` -> `RegisterTexture82d5ea00` -> kind0 | `RegArena827f156c` -> texture manager |
| `00eb0066`, `00eb0069` | 482, 147 | No callback in `82d604a0` registry | No independent processed asset from these TOC rows |

`82f76168` hashes literal `Texture` with `rw::RwHash32String`, seed
`811c9dc5`, and stores the result at `82f7618c`. Applying the already implemented
`hash::rw_hash32_string` operation graph gives `ac462e4a`. Do not inspect the
uninitialized database global to infer its runtime value.

`ProcessArena82d5fee0` visits the first used-local TOC and its authored rows,
invoking the registered callback and appending its products immediately.
Preserve that order; do not group textures/materials/instances by kind.

Environment processing `82d5ee78..ef58` checks the collision model pointer and
raw mesh count first, then the render model pointer and u16 mesh count. It
skips the instance only when both tests say populated. Otherwise it creates
Pres kind10 at `ef1c`, adds the SAME wrapper to Sim at `ef28`, appends at
`ef34`, and increments arena cost. Existing
`SimulationStaticArenaPlan::environment_is_hybrid` already implements this
predicate; extract/reuse the predicate rather than instantiate a second
collision owner. The census reports record subtype, not this predicate's
outcome, so it does not yet prove 147 registered kind10 records.

Texture processing initializes an empty resource descriptor for ordinary TOC
textures (`82d5eb08`) and calls the common registration helper. That helper
allocates the real texture-asset wrapper, stores the texture pointer and copied
resource fields, then adds Pres kind0 (`82d5eabc`) and appends (`eac8`).
`own_texture_arena` already decodes retained texture data; it is not this
processed registry publication or TextureMan activation.

## The 148th collection is not covered by the current census

Fresh `cGameStream::Update82d5db60` registers all XSM collection descriptors
(`dce0`), stream types (`dd14`), and authored parent relations (`dd74`). It then
registers XST metadata and publishes Loaded at `dff4`. There is no global asset
preload hidden in this stream-loaded transition.

After that, proxy `SyncLoadProxyWorldStream824df648` reads/sorts XSS and calls
`cAssetStreamManager::LoadCollection` for each spatial row at `824dfa78`.
`LoadCollection82d57f58` resolves the authored parent, recursively queues that
ancestor with the manager's parent callback (`82d57ff8`), then queues the
requested collection with its supplied callback (`82d5800c`). Existing
`CollectionParentManager::add_parent` and `plan_load` implement this exact
ancestor-first request plan; retain the native invalid-parent sentinel.

The existing parent completion callback also matters: first-use count update,
active Up, Simulation registration, second active Up, Presentation registration
(`82d5ca78`, `82d5caa4`; implemented in `collection_parent_manager.rs`). It is
not equivalent to incrementing the proxy spatial-load counter. Subsequent uses
share the same resident parent and do not register it repeatedly.

Consequently the next necessary diagnostic is all XSM ID/parent/path/count
metadata plus the unique union of `plan_load` for every XSS row, decoded using
the existing resident driver. The 58-asset difference must not be called shared
textures until that actual collection is inspected. Census extension requested
from root; no new parser or parallel loader is proposed.

## Smallest coherent production packet

1. After ancestor census, retain the existing catalog, resident streamer/storage,
   `CatalogResidentCollectionLoadFactory`, `DecodedArenaAssetBuilder`, and
   `CollectionParentManager` in one proxy boot composition. Use actual callbacks
   and separate parent registration, proxy loaded, and proxy activated counts.
2. Add only the observed processed Pres variants (currently 0/2/10), preserving
   decoded allocation identity and authored product order. Reuse the common
   outer Collection/Arena occurrence layer of `SimulationResidentRegistry`
   through a shared extraction if necessary; do not publish Pres objects into a
   fake Sim namespace. Parent completion still needs the actual Sim consumer.
3. Reuse `ArenaRegistrationRefCounter` and the traversal pattern in
   `presentation_dmo_arena_adapter.rs`; the DMO adapter itself is not a proxy
   handler. Actual kind0/kind2 activation needs retained TextureMan bindings,
   including its source fallback lookup, not a copied `TextureScene` index.
4. Kind10 requires Presentation unlock admission and
   `WorldPresentation::AddRenderInstance827e4f00`. Existing Simulation unlock
   router supplies the shared admission graph, but its ground-pool effects are
   not Presentation effects. `own_presentation_arena` provides reusable geometry
   parsing. A vector of decoded instances is not completed AddRenderInstance.
5. Only after ordered registration/callback delivery may the same proxy owner
   provide `WorldStreamEffects::AddHighCollection`, using existing
   `rw_hash64_string` and actual loaded proxy membership. Keep the default gate
   true and normal world-stream startup after real activation.

Concrete unresolved bounds before promising a complete activation packet:
ancestor asset kinds; actual Environment hybrid outcomes; unlock context
population at this boot point; texture fallback producer; and which render
instance branch/material order the actual proxy models select. Fresh
`AddRenderInstance827e4f00` dispatches through `GetRenderInstanceType`, then
old-format/sky/occlusion/optimized entity construction and actual render-view
publication. It cannot be collapsed into unconditional optimized geometry.
No DMO or unrelated kind handler is justified by the present census.

Suggested exclusive files after these observations: a narrow
`presentation_proxy_boot` composition plus tests, a shared processed-Pres
asset plan, and the required texture/environment service adapters. Shared
changes should extract existing parent/registry/admission machinery, not create
another streamer, parser, session facade, or render owner. No files are reserved
by this read-only proposal.
