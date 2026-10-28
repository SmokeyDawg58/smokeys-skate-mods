# Shared PROXY kind-0 processed product draft (2026-09-23)

Unregistered producer/cache revision. `Test-Path -LiteralPath` found this evidence path absent before creation. Only the existing unregistered `proxy_kind0_resident_builder_draft_20260923.rs`, `proxy_shared_loaded_arena_draft_20260923.rs`, its test draft, and this note changed. No registered module, parser, uploader, or Cargo command changed. Confidence is high for the owner shape and exact for the cited native lookup/producer branches; this packet has not compiled or uploaded mounted textures.

## Corpus and existing producer audit

The earlier read-only `artifacts/proxy-parent-census-2026-09-23.log` comes from the existing `DecodedArenaAssetBuilder` and retained mounted collection loader. A fresh count over that log gives 93 parent asset rows, 327 arena occurrences, and 180 `ac462e4a` Texture TOC rows: 93 parent and 87 spatial. Those 180 occurrences represent 145 distinct full outer arena IDs; the spatial subset has 87 occurrences, 52 IDs, and 35 repeated IDs. The parent and spatial ID sets do not overlap, as established in `proxy-parent-asset-producer-audit-2026-09-23.md`. For example, arena `b048de236c3f0656` occurs in spatial collections `da213d1ef597c698` and `8795aaf645c44c66`. These counts are corpus facts from the prior diagnostic, not evidence that the new producer has run. The mounted root exists at `<game folder>` when checked with `Test-Path -LiteralPath`; no new mounted decode or GPU test was run in this delegated revision.

Native `ProcessArena` `0x82d60058..0x82d601cc` takes the single-Texture alias branch for the observed parent and spatial Texture arenas. `RegisterTexture` `0x82d5ea00` creates the processed kind-0 wrapper, records the full TOC ID, and appends it before the collection callback. `cAssetManager::AddCompressedArenaAsset` `0x82d63a70..0x82d63bd0` reuses the one processed allocation on a full outer arena-ID loaded-map hit; `DownLoadedCount` `0x82d62edc..0x82d62fac` unprocesses it at the final physical use. The separate Presentation first/last-use gate then inserts/removes the same alias identity through `RegArena`/`DRegArena` `0x827f155c..0x827f17bc`.

The existing `DecodedArenaAssetBuilder` owns XST decode and RW4 parse. `own_texture_arena` resolves Texture and GPU-buffer dictionary objects and decodes actual texture levels. The unregistered `GpuTextureUploader` patch delegates the app's `GpuRenderer::upload_native_texture` through the same device and queue; it is still unapplied. The kind-0 producer already checks the one-row Texture TOC, exact Texture/GPU-buffer dictionary indexes and resource ranges, source Texture parameters, full IDs, and real upload. Its Box gives a distinct stable alias identity while the decoded source lives beside it. There is no second texture parser, uploader, or scene inventory in this revision.

The direct one-row TOC and Texture metadata checks are repeated inside
`own_texture_arena`; this reuses the same parser implementation but parses
those small records twice. That order preserves native branch admission
before texture-level decoding. `NativeTexture` currently omits source
`Texture::data` and fetch words; removing the second Texture parse would
require a bounded typed-output change. Arena decoding, level decoding, and
GPU upload each happen once per cache miss.

## Revised allocation boundary

The producer's value is now explicitly `Kind0ProcessedArena`: alias Box first, `DecodedArenaAsset` second, so alias/GPU teardown precedes decoded-source teardown. Its existing `Kind0ResidentArenaAsset` name is an alias for compatibility. `process_decoded_kind0(decoded, uploader)` extracts the existing validation/upload body after the decoder, allowing a future unified factory to classify one already decoded parent **or spatial** arena and call it without a second decode. The current `Kind0ResidentBuilderSessions` still admits only the selected parent ID and rejects zero-asset/non-Presentation sessions. Calling that parent-only factory for all spatial collections would fail; no unified factory is claimed here.

The reviewed `SharedLoadedArenaCache<Kind0ProcessedArena>` wraps the physical builder and supplies the cloneable `Arc<Kind0ProcessedArena>` on a full-ID cache hit. Each collection receives a separate noncloneable occurrence handle, while `retained_product()` clones the exact processed allocation for a future Presentation first-use owner. Thus the alias address, decoded source, locators, and GPU object belong to one canonical product allocation across the 35 repeated spatial Texture IDs. The revised focused cache test proves two collection occurrences expose pointer-equal retained products and releases those clones before final collection destruction. It does not instantiate a `GpuTexture` or test the kind-0 builder.

There is a necessary lifetime contract: the Presentation owner must retain the collection's physical resident hold until its last-use removal completes, then drop all retained product `Arc`s before that collection's final physical down. If an external product clone outlives every physical occurrence, the cache row can be erased while that clone remains alive, allowing a second allocation for the same ID. The draft API documents this precondition but cannot enforce it before a unified Presentation caller and hold owner exist. The prior final-teardown serialization test applies when no external strong clone violates this contract. A finished owner must prove this ordering, or strengthen the cache lease API before registration.

The native branch chooses the first **exported** TOC; the current parser exposes dictionary entries. This producer deliberately admits exactly one dictionary TOC and one Texture row. The mounted proof must show all 180 observed Texture occurrences pass that narrower admission; the historical TOC-row count alone does not establish exported-object equivalence for arbitrary arenas. The source's serialized fetch words are not the fixed-up Xbox alias address, so the product carries a checked source locator and host GPU object rather than fabricating that pointer.

Root temporarily compiled the revised shared-cache module through an app
test-only hook, ran its five focused tests, and removed the hook. The result
is `artifacts/proxy-shared-kind0-retained-handle-focused-2026-09-23.log`.
This validates retained shared-handle identity and physical cache teardown;
the kind-0 GPU producer itself remains uncompiled and unrun.

## Focused mounted proof before integration

Root can temporarily register this draft with the existing resident factory and unapplied uploader patch under an available wgpu adapter, using **one** cache across the accepted parent and all spatial sessions. A unified factory must decode each miss once, inspect the source TOC, dispatch the observed kind-0 arenas through `process_decoded_kind0`, and carry material/Environment arenas through their own existing or future producers. For the 180 Texture occurrences, record source-order outer IDs and full TOC IDs, both checked locators/ranges, one alias identity and one real GPU upload per distinct full outer ID, and pointer-equal retained products for each of the 35 repeated spatial IDs. Expected observed counts are 93 parent + 87 spatial Texture occurrences and 145 unique Texture products/uploads while all relevant collections remain resident. Fail explicitly if the adapter lacks the required BC support or any source check fails; preserve the completed prefix and withhold the corresponding callback.

After the unified Presentation owner exists, test first-use AddTexture and last-use RemTexture against that same product identity, with the resident hold and all retained `Arc`s released only after removal. Test callback order and a forced upload/registration failure. Kind-2 mutable material binding, admitted kind-10 Environment effects, processed-container lookup, and activation callbacks remain absent. This packet does not register kind 0 or advance enabled PROXY boot.
