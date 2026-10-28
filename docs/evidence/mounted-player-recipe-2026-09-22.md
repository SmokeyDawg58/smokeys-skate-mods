# Mounted local marquee completion

Confidence: **high** for the selected native ordering and identities described
below, pending original-data acceptance. Synchronous mounted IO remains an
explicit host service; native asynchronous scheduling is outside this claim.

This packet replaces the original-data startup's direct
`character_loaded(1, schema_hash, 2)` call with retained request, recipe, scene
and face completion owners. It uses the same supplied `PlayerLoaderSlot`,
backend settings/Vault and `CharacterFaceManager`. The completed scene Arc is
retained in selected startup and passed into the existing presentation runtime;
the frame probe no longer decodes a second character scene.

## Native identity and callbacks

`SimCharacterManager::SimCharacterManager8281cf70` sets next request ID to zero
at `8281cfa8`, allocates 256 request records (0xa400 bytes), and constructs them.
`Load8281a1f0` and `Register82817a08` choose the first free record. Both retain
the callback and request type/status before storing/incrementing the shared
counter; increment skips 0xffffffff by replacing it with zero. No synthetic
per-player request counter is used.

`SimCharacterRequest::OnLoaded82815418` calls the real callback at `8281544c`
before clearing request/category/model/status/callback (`82815458..88`). This
means the load request still occupies a slot while its callback allocates the
registration request. The new selected request-field owner keeps this order.
It is a projection of those fields, not a claim that the complete native manager
and broadcaster have been constructed. The retained callback occurrence uses
an actual host Arc allocation; its address is only an opaque host callback
identity and is not interpreted as a guest address.

`PlayerLoaderImpl::OnSimCharacterLoaded82817f28` stores the character ID before
primary attributes, optional ghost attributes, network and fresh ghost reads,
then calls Register. The shared patch extracts the existing attribute preparation
body unchanged, so real registration allocation occurs after those reads. The
old controlled public protocol remains for isolated tests. The mounted path
returns a real pending Register action; it never acknowledges roster/camera or
presentation registration as complete.

## Selected recipe path

The entry admits an already resolved first-local-marquee collection, the native
factory entry after `GetIDForName82cfe828`. It checks the supplied name against
that actual cached Vault name and does not invent a second marquee name table.
Full message-name lookup and manager dispatch scheduling remain caller work.

`RecipeFactory::CreateMarqueeLoader82814998` uses the actual character/face
managers. Its folded policy function at `82784530` returns zero. The factory
therefore samples generated cached layout byte8, falling back to the native
default marquee ID only when false; then `GetSkaterName82cfe920` samples the
effective collection. Local=true selects PlayerType0 at `82814a48..58`.
`MarqueeLoader` construction `82d0b130..158` produces clone policy3. Binary
`Unpack82d11f04..18` reads the serialized clone field but replaces the constructor
field only if the latter is undefined: clone3 survives this path.

`MarqueeLoader::LoadImpl82d09ef0` → `LoadSchemaFromFile82d09bd0` publishes Loading
and supplies an assets-loaded callback, with no schema callback. Asset manager
`LoadSchemaFromFile82d09720` lowercases the request name before FNV-1-64 hashing;
`UpdateRequest82d15120` assigns that ID to the decoded schema. It is therefore
the schema hash for this admitted path, but completion is no longer inferred
merely from knowing that hash. Full64 ID is retained and transported unchanged
at loader `82d0a1ec` and presentation status `82818f04` (raw `std`). Misleading
decompiler high-half assignments in these functions are not used.

The caller constructs and retains `MountedRecipeRegistry` once in selected
startup, separately from each load. It represents the selected recipe portion
of the shared presentation namespace, with full-u64 keys and explicit unsupported
other-asset occupants. `cAssetRegistry82d65428` constructs the empty asset map;
`CharacterAssetManager82d14308..14` constructs the separate loaded-schema vector.
Actual lookup precedes the binary cvar. A recipe hit reports a pending cache-hit
continuation; another asset type reports a typed collision boundary. Neither is
silently treated as absent. Parsed recipe publication precedes tracking and asset
IO (`82d15184`, `82d15190`); `TrackLoadedRecipe82d07278` deduplicates the actual
pointer. The registry keeps that same recipe Arc and later the same decoded
scene Arc, including across partial failures. This is not the full native asset
registry or manager. Existing presentation DMO adapters resolve outer collections
through `SimulationResidentRegistry`; `RenderScopeRegistry` assigns material and
effect scopes, and animation mount's `Registry` registers graph interfaces.
None can store this recipe namespace without changing their contracts.

The selected load samples
live `kSchemaLoadFromBinary` at the native `82d14c68` boundary, then fulfills
binary IO with the supplied MountSet and existing recipe parser. False requests
an explicit unsupported XML path. Missing IO/parse/asset data never reports a
successful callback. Native XML/graveyard fallback, cache hits, concurrent shared
loads and general activation scheduling are outside this selected initial path.

Mounted geometry/texture IO and decoding reuse `own_character_scene`; they are
host asset fulfillment, not a reconstructed asynchronous activation scheduler.
This existing host helper admits complete decoded buffers before exposing them,
so its partial IO failure does not claim native per-arena publication parity.
The recipe and native-stage prefix remain retained. Runtime category is set to
Marquee, as native `UpdateRequest82d15108` does after unpacking.

## Instance finalization and face dependency

Fresh `AddtInstance82d048c8` matches the full source instance asset ID, then its
clone0/clone3 branch stores the same registry instance pointer at `82d04994`.
No material clone or geometry clone occurs on the selected clone3 path.
`RebindComponentToMaterial82d08fd8..82d08fe4` enters only for clone1 or clone2;
clone3 returns without material stores. This is a proven native no-op, not an
unimplemented material callback answered with success.

The runtime recipe Arc receives requested category0 and constructor-preserved
clone3 before registry publication, so serialized category/clone values cannot
leak into later consumers. A discriminating test starts with category9/clone2.
Other parser metadata remains decoded file metadata; this host asset service does
not model the native allocator's memory-category accounting.

`BuildTInstanceArray82d088a8` destroys the old arrays, determines LOD count,
counts and publishes instance handles in component/model/LOD map order, excludes
Beard when Rostral exists, and requires a nonempty LOD0. Component comparator
`82590cc0` orders unsigned pooled-string hashes; model IDs and LOD bytes have
their native ascending order. Finalization retains indexes into the same scene
Arc and resolves exact instance GUIDs, never the first convenient render model.
Unsupported ambiguous host registry identities/map collisions fail explicitly.
Focused tests verify pointer provenance, native ordering, exclusion and late
failure with retained LOD prefix. Existing palette model admission remains its
previously documented host policy; this packet does not establish all native
model/cloth behavior or change that policy.

After assets and finalization, `CharacterRecipeLoader::OnComplete82d0a140`
stores recipe/asset and calls actual `LoadFaceData82d04cf0` for Marquee category.
Already loaded faces complete immediately. Otherwise the owner enters
WaitingFace, actual IO completes the existing face holder, and its queued
callback must match the retained loader occurrence and full asset ID. Only that
callback publishes Loaded. The marquee vtable at `82129e88+0x10` points to the
folded empty `825187c0`; `LoadCompleteAction82cfd528` then stores State_Loaded.
`PresCharacterManager::Update82818d50` admits a loaded managed request and sends
the retained ID/status before clearing its pending request. This packet performs
the selected equivalent callback continuation after actual completion, without
claiming native inter-thread message cadence.

## Cvar and validation

Reserved index293 is `kSchemaLoadFromBinary`, initializer `82f8b110`, table
`82fd5398`, reference `830a6c54`. Raw initializer loads default1 at `82f8b138`,
null update callback at `82f8b134`, section `Debug_CharacterAssetManager`, title
`Load Binary Schema Files`. It is inserted before the next selected native
initializer `kMaxVechileObjectSize` (`82fd53ec`), while the title-order test places
it by section/title. Tests verify unlinked refusal and live deserialized changes.

New tests also distinguish no face event from real face completion and reject
foreign/duplicate callbacks; request tests verify nested allocation, full capacity
and sentinel wrap. Registry tests cover full-width collision distinction, retained
parsed state before any scene, foreign recipe allocation rejection, and exact
scene allocation sharing. Original startup now asserts the returned registration request
and exact finalized instance provenance before constructing Actor animation.
Existing repeated tick and presentation probes consume the same resulting scene.

Author validation: rustfmt and shared patch `git apply --check`; no Cargo.
Independent source/code review approved the complete packet, including registry
ownership and pre-publication runtime category/clone overrides. Root original-data
and Cargo acceptance remain pending.
Packet: `artifacts/mounted-player-recipe-2026-09-22.patch`; generator with the
corresponding `build-` prefix. All registered edits remain exclusively root-owned.

Root acceptance action7-recipe-2: fmt and strict Clippy53.26s pass; workspace
4685units plus six docs pass. Original19/21 in6.83s, with mounted recipe,
finalized native instance provenance and same scene sharing passing in the
selected startup/ticks. Remaining failures are later Action42 and wheel-contact
fixture admission. Initial gate found two style lints only; root inverted the
already-loaded face branch and used explicit raw pointer in provenance test.
No native asynchronous cadence or complete application startup claim.
