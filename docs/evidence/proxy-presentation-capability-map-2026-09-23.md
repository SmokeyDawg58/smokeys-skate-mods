# Minimum Presentation proxy boot capability map

Read-only follow-up to `proxy-world-boot-order-2026-09-23.md`. No implementation,
Cargo, or replacement streaming owner was introduced. Default proxy gate stays
true; disabled boot is not proposed as a shortcut.

## Existing reusable pieces

- `world_stream_adapter::WorldStreamAdapter` already implements
  `WorldStreamBackend::begin_stream_file/poll_stream_file`; the mounted source,
  XSM/XST parsing and `WorldStreamCatalog` registrations are reusable for the
  Presentation PROXY stream metadata. Do not invoke the entire normal-world
  XML/high-effects pipeline as a substitute for native proxy stream loading.
- `skate2_formats::stream::CollectionSpace::parse` supplies XSS rows. Existing
  `collection_stream_order` is a candidate helper for spatial sorting, but its
  exact arithmetic/tie handling must be compared with proxy `824df878..dfa14`
  before reuse is asserted. Stable equal-order behavior must be preserved.
- `CatalogResidentCollectionLoadFactory`, `MountedCollectionStreamSnapshot`,
  `MountedResidentCollectionLoadDriver`, `CollectionResidentStreamer::update`,
  and `OwnedResidentCollectionStorage` already provide actual mounted ranges,
  FIFO asset work, retained collection assets and completion callbacks.
- `DecodedArenaAssetBuilder::build` implements the common format2 compressed
  arena path with XST identity/type/resource-size checks. It can back a narrow
  Presentation builder session; `CollectionAssetBuilderSessionFactory` is the
  existing session seam. No second decompressor, arena parser, or load pump is
  needed. Actual PROXY format/type admission must be checked, not assumed from
  the validated BAM census.
- `mounted_character_scene::own_presentation_arena` already decodes materials,
  meshes, models and instances into owned geometry. This is reusable geometry
  conversion, **not** native processed-asset registration or activation.
- `ArenaRegistrationRefCounter::add_asset` implements existing first-use gates.
  `SimulationResidentRegistry` has generic retained Collection/outer-Arena
  occurrence lookup; its selected inner namespace is Simulation-specific.
  Reuse/refactor only the common container identity layer if needed rather than
  publishing Presentation proxy data as Simulation registrations.
- `skate2_formats::hash::rw_hash64_string` is the exact existing high-to-proxy
  name hashing helper.

## Actual activation consumer

`SyncActivateProxyWorld824d9ef8` enqueues registration then callback per sorted
collection ID. `PresThreadMsgMan::ReceiveRegCollectionMsg82780958` first checks
activation, then resolves the actual collection from the container registry at
`827809e4`, iterates its actual arena list, calls `AddAsset` at `82780a3c`, and
calls `Presentation::RegArena827f1430` only for first-use arenas at `82780ab0`.
The proxy sender uses userdata0 and activation1.

`RegArena` dispatches the processed record kinds: texture0, material2,
environment-instance10, DMO11 only if userdata bit8, visual-instance17/18/19,
loose-world-instance20, waypoint29, lighting34 and antifrustum35. Other kinds
take the native default branch. This list is source capability inventory, not
a request to implement unrelated handlers. Actual PROXY record census must
determine the required subset.

The second message reaches `ReceiveCallbackMsg8277be10`, which invokes the
supplied nonnull callback at `8277be2c`. Actual
`cProxyWorldActCallback::OnComplete824d7d38` increments the retained manager's
activated count atomically and destroys that callback. Geometry decode or a
queued registration is therefore not an activation acknowledgment.

## Existing consumers that cannot stand in for this path

`PresentationDmoArenaRegistrationAdapter::register_collection` already mirrors
first-use collection traversal, but its prepared input supports the processed
material/DMO subsequence and explicitly excludes proxy collections. Its
deferred producer is the main-DMO-bank protocol, not the synchronous proxy
registration/callback pair. Preserve it; extract shared traversal only if the
concrete proxy implementation demonstrates useful common code.

The app's `load_presentation_collection` in `apps/skate2-pc/src/main.rs` reads
hard-coded BAM XSF paths and calls `own_presentation_arena`. It does not retain
the native load callback, processed registry, registration refcounter or
activation callback. It is not a boot-completion producer.

## Bounded implementation proposal

1. Produce a mounted PROXY census using the existing metadata/arena decoder,
   retaining the same actual decoded assets that will be registered. Establish
   actual formats, processed record order/kinds, cross-arena references and
   counts. This is the indispensable input for bounding the registration work.
2. Add a retained proxy boot owner over the existing catalog/load machinery:
   invalid initial stream ID, real metadata load, exact XSS ordering, map insert
   before each request, actual load callbacks and retained sorted IDs. Keep load
   and activation counters distinct. No fake completed return after XSS.
3. Connect only the observed proxy registration kinds to actual retained
   Presentation services, through an ordered registration/callback consumer.
   Missing services are typed pending boundaries; do not acknowledge activation
   after an unsupported record or manufacture a zero-kind census.
4. Feed that same owner's `AddHighCollection` map update into the existing
   `WorldStreamEffects` after real activation completion. Root's startup remains
   the caller; no second session facade or general DMO/LivingWorld expansion.

Likely owned files are a narrow `presentation_proxy_boot` owner and tests plus
one Presentation asset-builder/registration adapter if the census requires it.
Any shared extraction is limited to existing container-registry identity or
first-use traversal. No files are reserved or implemented by this audit.

Unknowns before a complete packet can be promised: actual PROXY arena census;
which processed-asset constructor/registry paths already cover those record
types; required concrete texture/environment/loose-instance consumers; and the
host scheduling interface that delivers registration before its callback. The
generic loader is available; those activation producers are the real gap.
