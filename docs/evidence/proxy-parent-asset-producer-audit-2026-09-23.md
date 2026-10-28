# PROXY parent asset producer and duplicate identities

Read-only audit of `artifacts/proxy-parent-census-2026-09-23.log` and the live
Skate 2 IDA database. No registered source or Cargo command was changed.

## The outer XST type is an arena container

All 327 decoded PROXY arena occurrences have outer XST type `ab329a6a`.
`decoded-arena-asset.md` independently records that value for BAM Presentation,
Simulation, and Texture arenas. It identifies the compressed RW4 arena input to
`cProcessArenaAsset::ProcessArena` (`82d5fee0`), not a processed Presentation
kind. The decoded first TOC and its raw entry types determine the products.

The sole XSM ancestor outside XSS is `f691a7720d232c78`,
`cPres_Global_proxy.xsf`, parent sentinel `e171e1abfedb69ed`, declared 93
assets. All 93 decoded assets have four dictionary objects, one first-TOC row
with raw type `ac462e4a`, a `.Texture` name, and no `InstanceData`. Their 93
outer asset IDs and 93 TOC GUIDs are distinct. The first TOC GUID is
`0000755003e3870a` in outer arena `ed26bb11633758bf`. The raw type is the
registered `kAssetTocType_Texture` hash; it is not the outer `ab329a6a`.

Fresh `ProcessArena82d60058` checks whether the first TOC has exactly one row
and that row is `kAssetTocType_Texture`. Every parent arena meets that observed
predicate. This branch gets texture parameters (`82d600f8`), obtains aliased
parameters (`82d60108`), gets the resource descriptor (`82d60114`), allocates
the backing resource (`82d60168`), initializes the texture (`82d60180`), and
calls the common `RegisterTexture` helper at `82d601cc`. Thus the parent does
not take the ordinary TOC `ProcessTexture82d5ead8` callback, which zeroes the
resource descriptor before calling the same helper. `RegisterTexture82d5ea00`
constructs the texture asset wrapper, copies its five resource bases, calls
`AddPresRegistryAsset` at `82d5eabc`, and appends the product at `82d5eac8`.
The registered result is one processed Presentation **kind 0** per parent
arena. On first-use Presentation arena registration, `RegArena827f1430` calls
the shader texture manager's `AddTexture` at `827f156c` for kind 0.
Confidence is high for this producer/consumer chain and exact for the observed
corpus predicates/counts. The diagnostic has not executed these native
registration effects in Rust.

## Where the 35 repeated IDs actually occur

The census retained 148 collections, decoded 327 arena occurrences, and found
292 unique outer asset IDs. Log analysis by collection and arena ID gives:

| Group | Occurrences | Unique IDs | Repeated IDs |
| --- | ---: | ---: | ---: |
| Parent collection | 93 | 93 | 0 |
| 147 spatial collections | 234 | 199 | 35 |
| Full required closure | 327 | 292 | 35 |

Each of the 35 repeated identities occurs exactly twice, in two different
spatial collections. All 70 occurrences have four dictionary objects and a
single raw `ac462e4a` Texture TOC row; none has `InstanceData`. The parent IDs
have zero overlap with spatial IDs. Parent TOC GUIDs also have zero overlap
with the 52 unique spatial Texture TOC GUIDs. For example outer arena
`b048de236c3f0656` appears in spatial collections
`da213d1ef597c698` and `8795aaf645c44c66`. Therefore the 35-count
occurrence gap is spatial texture sharing, not repeated parent assets.

XSM gives 127 spatial children of the global parent and 20 terminal spatial
collections, plus the global parent itself. `plan_load` consequently includes
the parent in 127 spatial request plans, although the required collection union
contains it once. Native `OnLoadParentComplete82d5c978` increments its use
count on later completions but sends the first-use Simulation and Presentation
registrations only once; the existing `collection_parent_manager.rs` models
that rule. Physical cache hits and registration refcounts must be kept distinct
from the 93 parent asset identities and the 35 spatial duplicate identities.

The entire closure uses all 292 XST IDs. The 147 spatial ordinary arenas have
TOC material `00eb0005` and instance-list `00eb000d` rows (plus unprocessed
subreference rows); each instance list reports one Environment subtype
`acb31c9a`. Across all collections there are 180 Texture TOC occurrences:
93 parent plus 87 spatial; deduplication leaves 145 unique Texture arena IDs.
The 147 material products are kind 2. Environment instances are kind 10 only
when `ProcessInstanceList` admits them under its collision/render population
predicate; this census does not measure that predicate outcome.

## Smallest actual first-test producer path

The enabled default proxy boot needs one retained owner using the existing
mounted metadata/catalog, XSS parser, `CollectionParentManager`, resident
streamer/storage/driver, and decoded arena builder. It must preserve proxy
spatial sorting and enqueue parent requests before child requests, retain the
actual loaded callback counts, and process arenas into ordered Presentation
asset lists. The observed subset is parent and spatial Texture kind 0, spatial
Material kind 2, and conditionally Environment kind 10. All 180 observed
Texture TOC occurrences are single-row arenas, so they select the special
aliased-resource construction described above. Materials and admitted
environments need their respective source consumers. Presentation collection
registration must apply the
arena first-use counter and deliver registration before the separate activation
callback increments the proxy activated count. Only after that completion may
the same owner service `AddHighCollection`; normal world stream boot follows.

This path uses no DMO producer. Proxy activation sends userdata zero, so the
kind-11 RegArena arm is gated off even if another corpus later supplies one.
The current PROXY TOC/instance census contains no DMO subtype. Remaining
unproven pieces are the Environment admission outcomes, concrete texture
manager and material binding effects, Presentation environment unlock/render
publication, and the full callback schedule. Geometry decode or the 292-ID
census alone does not acknowledge activation.
