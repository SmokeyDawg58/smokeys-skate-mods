# Supported normal Simulation arena plan

## Scope and provenance

`SimulationNormalArenaPlan` is the successful, dense processed-asset
projection for the currently supported normal `Simulation::RegArena`
subsequence. It merges the existing kind-6 grind and kind-10 Environment
actions with pointer-backed kinds 11 and 16 and the recovered ID-dispatched
kinds 22, 23, 25, 27, 28, and 32 in authored TOC order. It does not claim
complete arena registration: `LocationManager` kind 21, the separate DMO
kind-31 pass, and the manager implementations below this plan remain external.

Reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- external processor packet SHA-256
  `C4C11506EA241F207998C3ED78E6E68E339DCFCDE2A726C22D31CBA04B8566C2`;
- external dispatch/callee packet SHA-256
  `3C3CDA39C3BAED4BE2EA0F79C3E627786376DBCC6F627CB7F6B761A889B323CF`;
- external runtime-adapter packet SHA-256
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`;
- external processed-Arena-cost packet SHA-256
  `6711C4CD9835EA86C27714AE77AB83D14326566630C4C5EA5D307C925DB694AD`.

The packets remain outside the repository under
`world_probe_temp`; no executable, symbols, or proprietary game
payload is committed.

## Recovered construction and dispatch

`cProcessArenaAsset::ProcessArena` (`0x82d5fee0`) selects the first TOC and
walks its 24-byte entries forward. Each processor appends its products before
the next TOC entry, so multi-record list products remain adjacent and forward.
On their nominal non-null path, the simple processors each issue one
`AddSimRegistryAsset` call from the full TOC GUID and resolved object pointer,
then append its result once:

| Raw TOC type | Processor | Processed kind | Runtime route |
|---:|---|---:|---|
| `0x00eb000d` + inner `Instance_DMO` | `ProcessInstanceList` `0x82d5ec38` | 11 | model manager then DMO manager |
| `0x00eb0019` | `ProcessTriggerInstanceList` `0x82d5f078` | 16 | trigger-volume manager |
| `0x00eb000f/10/11` | `ProcessWorldPainter` `0x82d5f1d8` | 22 | streamed tile register/unregister |
| `0x00eb0022` | `ProcessNavMesh2Data` `0x82d5f230` | 23 | nav-mesh add/remove |
| `0x00eb0013` | `ProcessRAINData` `0x82d5f338` | 25 | RAIN add/remove |
| `0x00eb0014` | `ProcessAIPathData` `0x82d5f498` | 27 | AI-path add/remove |
| `0x00eb001a` | `ProcessWaypointData` `0x82d5f390` | 28 | waypoint register/deregister |
| `0x00eb0020` | `ProcessSpatialMapData` `0x82d5f548` | 32 | spatial-map add/remove |

Kinds 11 and 16 expand their inline arrays forward with exact strides `0xa0`
and `0xf0`. Kind 11 is emitted only when the container discriminator equals
`Instance_DMO = 0x06d051d1`; its processed ID is the primary GUID at record
`+0x60`. Kind 16 has no inner discriminator gate and its processed ID is the
local GUID at record `+0xb8`, not the primary GUID at `+0xb0`. Both processed
assets forward the exact inline record pointer. These construction details are
**exact**.

`Simulation::RegArena` (`0x828a01e0`) and `DRegArena` (`0x828a05f0`) load the
full `u64` ID for all eight represented kinds and traverse forward. The
waypoint polarity is
literal true on registration and false on deregistration. Every callee returns
`void`; there is no outer rollback channel. These construction mappings,
identities, and call directions are **exact**. Deep manager behavior is not
represented by this plan.

The same retained operation list now reproduces processed
`cArena::mCost`. `ProcessArena` clears the unsigned field on every constructed
path. `ProcessInstanceList` increments only its admitted-Environment local at
`0x82d5ef38` and wrapping-adds it into the field at
`0x82d5ef50..0x82d5ef58`; the other instance subtypes add zero. Because the
plan's Environment operation is governed by that exact admission predicate,
wrapping-counting those operations is the original field construction rather
than a host heuristic.

Kind 11 samples independently supplied userdata bit 3 during both registration
and deregistration.
When set, registration calls `ModelMan::AddDMOModel(primary_id)` and then the
DMO manager with `(record pointer, primary_id)`; deregistration calls the two
removal paths in the same subsystem order. The DMO manager internally keys by
the record's local GUID at `+0x68`, stores the exact record pointer, preserves
the earlier pointer on duplicate local IDs, and removes only when both local ID
and pointer match. Kind 16 is not userdata-gated: it calls trigger add/remove
with the same record pointer. Those runtime semantics are consumed by the
unified adapter documented in
[`simulation-normal-arena-adapter.md`](simulation-normal-arena-adapter.md);
this plan preserves its inputs and identities.

The owner retains an `Arc<SimulationStaticArenaPlan>`, which in turn owns the
complete decoded arena and all five resource buffers. Static operations are
owner-relative indexes. Kind-11/16 operations keep a private resource index
and absolute record span parallel to the public descriptor. `record_source`
returns a short-lived owner-bound byte view; pointer-equivalent identity is
defined by `Arc::ptr_eq` on the immutable arena owner plus the same resource and
record start. Record length remains a validated view invariant, not part of
pointer equality. `record_handle` provides the same identity as a cloneable
owning authority for managers that retain the native record beyond one call;
it shares the backing arena and never copies the record. Thus repeated TOC
aliases compare as the same backing record while byte-identical records at
different offsets or in different owners do not. No raw host pointer is stored
or exposed as runtime authority.

The new Pegasus record-table views validate only the 20-byte container header
and counted fixed-stride record array. They do not decode unrelated instance or
trigger strings that these two native processors do not read, so the normal
plan adds no such rejection itself. However, the current
`from_collision_asset` composition first builds the existing collision/static
projection, whose full `InstanceData` parser still validates strings. End-to-end
construction is therefore deliberately stricter than the native DMO processor
on malformed instance strings; removing that predecessor preflight is an open
architecture boundary, not claimed parity. Construction
uses one forward TOC scan and a monotonic static cursor; it never sorts, groups
by kind, or deduplicates repeated records. Before publication it validates
local section zero, dictionary bounds, exact raw type, and retained resource
range. These typed malformed-input errors and `Arc` ownership are
**high-confidence safe host policy**, not recovered native error handling.

The plan is explicitly nominal. Native processor allocation failure enters
assert-oriented behavior and can create a null list terminator. In addition,
`PreventNavmeshLoadingHack` can suppress the whole arena when enabled and the
first TOC entry is raw type `0x00eb0022`. That external flag/bypass must be
handled before treating this nominal plan as live output.

## Rob's Park real-data gate

The optimized `--probe-robs-park-resident` path builds and retains this plan
for all 68 selected Simulation arenas in both direct and parent-expanded runs.
It independently re-resolves every TOC/dictionary source before hashing the
descriptor stream. The result is:

| Kind | Count |
|---:|---:|
| 6 grind | 42 |
| 10 Environment | 26 |
| 11 DMO instance | 0 |
| 16 trigger | 0 |
| 22 WorldPainter | 145 |
| 23 NavMesh | 0 |
| 25 RAIN | 4 |
| 27 AI path | 4 |
| 28 waypoint | 3 |
| 32 SpatialMap | 45 |
| **Total** | **269** |

The frozen big-endian FNV-1a descriptor signature is
`415b55e790aabfb0`. The record stream includes collection name, outer asset and
RW4 arena IDs, processed kind, full processed asset ID, TOC and dictionary
indexes, inline index, raw TOC type, and raw `ObjectId`. `u32::MAX` encodes “no
inline index” only in this host test schema. The inputs and ordering are
**exact** for the selected corpus; FNV and this descriptor encoding are host
regression policy.

The earlier content-rich static signature `39413e2d647f8631` remains an
independent 68-operation gate. The new probe proves construction, ordering,
identity, and lifetime only. It does not call the six newly represented
manager domains or claim visible world activation.

## Whole-BAM pointer-record gate

The optimized `--inventory-bam-sim` path now constructs this plan for all 284
BAM Simulation collections and all 2,135 arenas. It independently re-resolves
each pointer operation against the first TOC, dictionary object, inline offset,
and retained record bytes. A separate narrow dictionary walk finds zero raw
DMO containers/records and nine raw trigger containers containing 10 records;
the gate requires that raw container map to equal the processed plan's map, so
orphaned or omitted containers fail independently of the operation signature.
The 62,810-entry scope therefore contains no kind-11 `Instance_DMO` records and
exactly 10 kind-16 trigger records across nine containers. The absence is
specific to BAM Simulation; the separate dedicated
DMO stream contains 93 `Instance_DMO` records. A later audit proved that bank
does reach normal `Simulation::RegArena` with userdata bit 3 set, despite its
Presentation-named stream; it is distinct from the kind-31 DMO pass.

The frozen host regression schema is
`C(name,NUL); A(outer u64, arena u32); O(kind, dispatch, primary, local, TOC,
dictionary, inline, offset, raw type, raw ObjectId)`, with every numeric field
big-endian. It produced scope signature `d53ab86208c6d53a`, pointer-operation
signature `8cd308157ff70b30`, and unified signature `f9ea70696c82a99b`, matching
the independent external harness byte-for-byte. Corpus inputs and order are
**exact**; FNV-1a and the descriptor encoding are host regression policy.

## Pointer-operation runtime adapter

`PreparedSimulationPointerArena` prevalidates every kind-11/16 record shape,
kind, and primary/local ID lane before effects. Its registration adapter owns
the complete plan through each explicit occurrence and walks the pointer-backed
subsequence forward. With bit 3 set, kind 11 calls model add before DMO register
and model remove before DMO deregister. Kind 16 always calls trigger add/remove.
Deregistration accepts a fresh userdata word, so the native asymmetric cases
remain possible: clear/set emits removal without a prior add, while set/clear
skips removal. In the latter case the adapter conservatively retains an orphan
source owner until its dispatcher is destroyed because the DMO manager may
still hold the record pointer.

The dispatch seam uses the cloneable owning record handle, a non-cloneable
same-domain occurrence token prevents accidental duplicate teardown, repeated
registrations and aliases remain distinct calls, and token `Drop` performs no
implicit cleanup. Storage is reserved before observable calls. The recovered
manager calls are `void`, so the seam deliberately does not invent a
recoverable effect-error or rollback protocol. Host allocation and authority
errors are typed policy.

This adapter is explicitly only the kind-11/16 subsequence. Running it as a
separate pass cannot preserve global interleaving with kinds 6, 10, 22, and the
other normal operations, and the dense plan cannot represent the native
first-null early termination after a processor allocation failure. The unified
normal-arena adapter now closes the interleaving boundary; first-null behavior
still prevents a complete `RegArena` claim.

An aggregate `PreparedSimulationPointerArenaBank` now retains a caller-ordered
set of already prepared arenas and its total pointer-operation count. The
adapter's bank route preallocates and installs one strong live-bank record,
checks the shared registration-ID sequence, and constructs its one opaque
non-cloneable token before the first dispatch. It then walks arenas forward and
uses the unchanged per-arena forward operation loop. Teardown validates the
whole token and reserves a whole-bank orphan hold before its first call; fresh
userdata is sampled independently, and a set/clear DMO asymmetry retains the
entire bank until adapter destruction. Token `Drop` remains inert.

This is a **high-confidence safe host preflight**, not a recovered native bank
object or transaction. It removes adapter-owned late allocation/ID failures
from the effectful prefix, but the `void` dispatch interface cannot preflight
manager-internal allocation or prevent panics. No rollback result is invented.

The optimized BAM gate now prepares, registers, and deregisters all 2,135
plans. It observes exactly 10 trigger adds followed by the same 10 exact-source
forward removes within their arena occurrences, zero DMO calls, zero identity
mismatches, one adjacent distinct-record
pair, no live/orphan ledger entries, and immediate plan-owner release after
matched teardown. This is an **exact corpus/order/identity gate** for BAM's
kind-16 records and a host ownership test.

## ID-dispatched runtime adapter

`PreparedSimulationRegistryArena` extracts kinds 22, 23, 25, 27, 28, and 32
without sorting or deduplication and retains the complete normal-plan owner.
`SimulationRegistryArenaRegistrationAdapter` walks that filtered subsequence
forward in both directions, forwards each full `u64` ID, and supplies the exact
waypoint boolean (`true` on registration, `false` on deregistration). One
non-cloneable occurrence token controls teardown; foreign or unknown tokens do
nothing, and `Drop` is inert. Checked host allocation/ID failures occur before
manager calls. The native calls are `void`, so the adapter does not invent a
recoverable manager-error or rollback result.

The whole-BAM route records 7,426 operations in each direction with per-kind
counts `{22:6122,23:0,25:75,27:101,28:95,32:1033}` and signatures
`757b00e47ce5f273` / `b55ae55479c67d26`. These counts independently reconcile
with the raw TOC corpus. See
[`simulation-registry-arena-adapter.md`](simulation-registry-arena-adapter.md)
for the exact hash protocol and dispatch addresses.

This remains a labeled subsequence. The static, pointer, and ID-only adapters
cannot be invoked sequentially to reproduce `Simulation::RegArena`, because
that would group operations by adapter rather than preserve the normal plan's
global authored order. The unified adapter now consumes their shared global
plan instead. Native first-null slots remain absent from the dense successful
plan.

## Dedicated DMO-bank kind-11 gate

The strict app-local DMO gate separately parses `DMO_Pres.xsm`,
`DMO_Pres.xst`, and `cPres_Global.xsf`; it does not reuse the older tolerant
diagnostic inventory. The manifest identity is version 3, stamp
`0000000094d14dc9`, format 2, one `cPres_Global` collection, 125 unique XST
assets, and 125 XSF blocks in identical full-ID order. Every asset has type
`ab329a6a`; stored ranges, all five decoded resource sizes, and RW4 arenas are
validated before any runtime effect. The full physical asset-ID signature is
`9a861845e82ee292`.

The mounted input hashes are XSM
`08532DAC62666E4885802BF1DA4200C482021FB3859FB27C9E4C2458D5344EC2`, XST
`830A99A4361BB09EF42CF134C6F14813426594E41773C5536AEFCDAD8BAE9EB0`, and XSF
`477D443547F9F5DC6C6459AD0B00812D17A2DC15F57E64952ADA68F87E32B957`.
A formats-only independent raw-corpus harness (source
`E6FEC1829CE97ABE9C2A2E4E157BBF60D06F94925D02138AF89803599EAF9386`, output
`FBD4876E94844CBD70336A36445FF0B8B60AFFCB146A870B619C7325315458E5`)
reproduces all descriptor signatures below without importing `skate2-core`.

All 125 arenas successfully traverse the strict decoded-envelope/RW4 checks
and the existing public collision/static and normal-plan constructors. Their
287 first-TOC entries
produce exactly `{11: 93}` represented normal operations, with one raw DMO
container holding all 93 records and no triggers. The unchanged descriptor
schema yields scope signature `3142cd8e74991ba3`, operation signature
`c769707cfc80f14f`, and unified signature `4151adc680021965`. The raw-container
map must equal the processed map, independently of those hashes.

The complete bank is prepared before calls and retained by one aggregate
authority. Registration then walks all 125 arena occurrences in physical XSF
order with raw userdata `0x0a`, producing 93
model-add/DMO-register pairs. Deregistration consumes the one aggregate token
and walks its 125 arenas in the same forward order with a fresh `0x0a`, producing 93
model-remove/DMO-deregister pairs. The gate verifies immediate within-pair
order, full primary IDs, 160-byte sources, exact register/deregister backing
identity, every callback's primary/local pair against the frozen plan order,
zero trigger calls, empty ledgers, and owner release after matched teardown. A
separate raw-`2` pass proves that clearing bit 3 produces no DMO effects.

The raw `0x0a` value is **exact**: `LoadMainDMOBank` supplies register-on-load
asset flag 8 and `cHullLoader::LoadHull` ORs bit 1 before the value is retained
and forwarded through `NotifyCollectionSimReg`. Raw `8` would choose the same
kind-11 branch but would not reproduce the message value. The clear-bit pass,
FNV schemas, preallocation, typed failures, and owner checks are host regression
policy.

The native forwarding/call order and shipped serialized corpus are **exact**.
That a successful reference load presents all 125 arenas as non-null processed
assets is **high confidence** from the load chain plus their valid shipped
decodes; it has not been separately observed in a live native trace. Native
malformed/allocation-failure consequences remain unknown.

This proves the real kind-11 pointer-operation boundary, not concrete Living
World or DMO-map mutation, cross-kind interleaving, Location or kind-31 DMO
registration, first-null allocation behavior, asynchronous message timing, or
complete `RegArena`.
