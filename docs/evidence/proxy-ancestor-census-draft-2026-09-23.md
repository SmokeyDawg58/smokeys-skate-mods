# PROXY XSM ancestor census draft

The reviewed draft was integrated into the existing
`crates/skate2-core/src/streaming/proxy_asset_census.rs` diagnostic. This keeps one census
owner; the extended original-data run is recorded separately below.

## Source and typed semantics

`cGameStream::Update82d5db60` registers XSM descriptors and their parent
relations before XST registration. `SyncLoadProxyWorldStream824df648` requests
the spatial XSS rows. `cAssetStreamManager::LoadCollection82d57f58` plans each
request's ancestors first, attaching `cLoadParentCollectionRequestCallback` to
ancestors and the supplied callback to the requested row. The existing
`CollectionParentManager::add_parent` / `plan_load` encodes that relation and
uses the exact invalid-parent sentinel `e171e1abfedb69ed`. Confidence: high
for the source plan; exact for parsed XSM/XSS/XST fields and the previously
observed XSS-only decoder output. Extended parent output requires its own run.

The draft prints each XSM ID, parent ID, registered path, and declared asset
count. It computes `plan_load` for every XSS row in authored diagnostic order,
then decodes the first-seen union of required IDs with the existing mounted
resident driver and `DecodedArenaAssetBuilder`. It reports the ancestor-only
collection IDs, any XSM IDs outside that union, each decoded asset ID/type,
unique required asset count, and differences against XST IDs. Parent-collection
asset IDs/types are printed separately. The `CensusOnlyParentEffects` boundary
cannot deliver registration effects because this diagnostic invokes only
`plan_load`.

The union order is a diagnostic traversal in authored XSS order. The native
proxy routine sorts spatial rows and its queued resident requests may complete
in another order. These results identify required assets; they do not establish
parent callback completion, Presentation/Simulation registration, texture
activation, or world rendering. The earlier XSS-only census decoded 234 arenas
from 147 spatial collections; XSM has 148 descriptors and XST has 292 entries.
The earlier XSS-only result left the remaining parent collection unmeasured;
the integrated result below closes that diagnostic gap. No inference that its
assets are textures is made.

Focused assertions in the draft check that each plan ends with the requested
row, preceding entries use parent callbacks, every planned ID is registered in
XSM, decoded collection counts match XSM, each decoded asset ID has XST
metadata, and all planned collections were retained. `rustfmt` on the draft
passed. The root owns Cargo and the real-data probe after integration.

## Integrated original-data result

`artifacts/proxy-parent-census-2026-09-23.log` passes the one ignored mounted
test. The existing `plan_load` closure requires all 148 XSM collections for the
147 XSS requests. Its one parent-only collection is
`f691a7720d232c78`, with invalid-parent sentinel
`e171e1abfedb69ed` and 93 declared assets. All 93 decoded assets in this
collection report type `ab329a6a`; their Presentation producer is still under
source audit. The 148 retained collections contain 327 decoded arena
occurrences but 292 unique asset IDs. All 292 XST IDs occur in the required
closure, and no decoded required ID is absent from XST. The 35 occurrence
difference is repeated IDs, not an inferred missing collection. This proves
diagnostic decode and ancestor coverage; native parent callbacks and
Presentation activation remain unimplemented.
