# Processed arena registration

This record separates three stages that earlier diagnostics intentionally
collapsed:

1. an RW4 arena is decoded;
2. `cProcessArenaAsset::ProcessArena` expands authored TOC records into an
   ordered `cArena::cAssetList`;
3. Simulation and Presentation independently consume that typed list and
   publish its assets to subsystem managers.

Decoded dictionary geometry is therefore not automatically live world or
render state.

## Evidence anchors

| Symbol | VA | Proven role |
|---|---:|---|
| `cProcessArenaAsset::ProcessArena` | `0x82d5fee0` | finds the first used-dictionary `0x00eb000b` TOC and invokes registered processors in inline record order |
| `ProcessInstanceList` | `0x82d5ec38` | expands instance records in forward array order |
| `ProcessSplineList` | `0x82d5f180` | creates type-6 Simulation asset under the TOC entry's full GUID |
| `ProcessDMOData` | `0x82d5f4f0` | creates one kind-31 Simulation asset for the whole first-TOC `tDMOData` object |
| `Simulation::RegArena` / `DRegArena` | `0x828a01e0` / `0x828a05f0` | forward typed-asset dispatch |
| `Simulation::RegDMOArena` / `DRegDMOArena` | `0x828a0420` / `0x828a0828` | separate forward kind-31 dispatch with asymmetric registration/deregistration routing |
| `Presentation::RegArena` / `DRegArena` | `0x827f1430` / `0x827f1680` | forward typed-asset dispatch |
| Simulation collection receivers | `0x8277e988` / `0x8277ebb8` | exact multi-arena normal/DMO pass ordering; no arena refcounter |
| Presentation collection receivers | `0x82780958` / `0x8277fff8` | per-arena first-use/last-use dispatch through its refcounter |
| `NotifyCollectionSimReg` | `0x82d5c468` | loaded hold, optional defer, registration message, callback message |
| `NotifyCollectionPresReg` | `0x82d5c680` | loaded hold, optional defer/proxy transition, callback message |
| `NotifyCollectionPresDReg` | `0x82d580d0` | deregistration/proxy transition, callback message |

The PDB fixes `assman::cAssetID` at eight bytes, `assman::cAsset` at 16 bytes
(ID `+0`, kind `+8`, data `+0xc`), `cArena` at 24 bytes (asset vector `+0`, RW
arena `+0x10`, cost `+0x14`), and `pegasus::tTOCEntry` at 24 bytes (name `+0`,
GUID `+8`, raw type `+0x10`, object `+0x14`). Raw `ld` instructions confirm
that the apparent split-word expressions in some decompiler output are
artifacts; identities remain full `u64` values.

The processed-cost extraction packet `presentation_arena_cost.txt` has
SHA-256
`6711C4CD9835EA86C27714AE77AB83D14326566630C4C5EA5D307C925DB694AD`.
It remains outside the repository under `world_probe_temp`.
The receiver-front packet `presentation_receiver_fronts.txt` has SHA-256
`0DB5B22BF66436E3DFCC4F42594F21AE934B2AEB4785E24E20CCEB67A7204F14`.

## TOC processing order

`ProcessArena` calls the named exported-object accessor across used dictionary
indexes and stops at the first TOC object; the accessor directly indexes the
used local dictionary rather than a separate export-mark list. It then walks
the TOC's inline entries from zero to count with a 24-byte stride. For
each entry it immediately looks up the raw type in its processor table. A
missing processor is ignored. A found processor runs before the next TOC
entry, and processors append zero or more typed assets to the `cArena` list.

Consequently, native typed-asset order is authored TOC order with each list
processor's products expanded in place. It is not RW4 dictionary order, kind
order, or a later material/model/instance pass order.

The constructor at `0x82d604a0` registers processors for texture hashes and
raw types `0x00eb0004`, `0005`, `0009`, `000d`, `000f`, `0010`, `0011`,
`0013`, `0014`, `0016`, `0017`, `0019`, `001a`, `001c`, `001d`, `0020`,
`0021`, `0022`, `0024`, and `0025`. TOC subreference records such as
`0x00eb0064` and `0x00eb0069` do not imply a processor invocation merely by
appearing in the TOC.

`ProcessSplineList` passes the TOC entry GUID directly to
`AddSimRegistryAsset` as kind 6. The former diagnostic arena-ID plus dictionary
index label is useful ownership metadata but is not native grind-registry
identity.

`ProcessInstanceList` branches on its container subtype and walks all inline
160-byte instances forward. It emits Marquee 7, Create-a-Character 8,
LivingWorld 9, Environment 10, DMO 11, and Trigger 15. Environment skips an
instance only when both conditions are true:

- its collision model is non-null and has a nonzero mesh count;
- its render model is non-null and has a nonzero mesh count.

Assembly-form collision mesh count `-1` is nonzero. Every emitted instance ID
is the full GUID at instance offset `+0x60`. The processor evaluates both
model/count predicates in collision-then-render order before the final `C && R`
exclusion branch. Each count field is loaded only when its model pointer is
non-null. Raw zero is null only for the instance's optional model fields; a TOC
`ObjectId(0)` resolves local dictionary entry zero.

## Processed Presentation cost

`ProcessArena` initializes the unsigned `cArena::mCost` field to zero on its
texture-special path (`0x82d60098`), ordinary path (`0x82d60328`), and empty
fallback (`0x82d60424`). `ProcessInstanceList` initializes its local addition
to zero at `0x82d5ec50`. Only an Environment record that survives the exact
collision-model/render-model exclusion increments it, at `0x82d5ef38`.
The processor then load/add/stores that local count into `mCost` at
`0x82d5ef50..0x82d5ef58`. Every other supported instance subtype reaches its
exit with the unchanged zero addition.

Consequently, processed cost is the wrapping `u32` count of admitted
Environment operations across `ProcessInstanceList` callbacks. Its exact
zero/nonzero meaning in Presentation deferral is established; no time or byte
unit is implied. `SimulationNormalArenaPlan::presentation_arena_cost`
reproduces the value from the same admitted Environment operation stream, and
the prepared Presentation owner retains it beside the Arena. The complete
dedicated DMO bank contains only kind-11 DMO instance-list output: its 125 real
arenas all validate at cost zero.

## Simulation dispatch

`Simulation::RegArena` and `DRegArena` both walk the typed list forward and
stop at the end or first null pointer. Deregistration does not reverse the
order.

| Kind | Registration action |
|---:|---|
| 6 | GrindData spline add/remove under the full TOC GUID |
| 10 | unlockable environment instance add/remove |
| 11 | LivingWorld then DMO model add/remove when userdata bit 3 is set |
| 16 | trigger-volume add/remove |
| 22 | WorldPainter streamed tile register/deregister |
| 23 | LivingWorld nav-mesh add/remove |
| 25 | LivingWorld RAIN add/remove |
| 27 | AI path add/remove |
| 28 | waypoint register/deregister |
| 32 | spatial-instance add/remove |

Static collision publication enters through kind-10 environment instances,
not by registering every collision-shaped dictionary object. The recovered
path is `UnlockableLocationMan` to `SimUnlockableLocationMan`,
`WorldStreamerView`, and `GroundPool`. It walks the collision-model mesh table
forward and uses identity transform at this boundary. Unlock/challenge state
may suppress immediate world insertion.

Simulation collection registration performs a complete forward normal pass
(`Simulation::RegArena`, then `LocationManager::RegArena` per arena) when
activate is true, followed by a second complete forward `RegDMOArena` pass.
Deregistration performs the complete `DRegDMOArena` pass first, followed by a
forward normal pass (`Simulation::DRegArena`, then LocationManager per arena).

### Separate kind-31 DMO pass

`ProcessDMOData` selects raw TOC type `0x00eb001d` and appends exactly one
kind-31 asset carrying the TOC entry's full GUID and the complete fixed-up
`tDMOData*`. It does not expand the object's 128-byte locator records into the
processed list. `tDMOData::Fixup` (`0x829c5670`) clears each record's serialized
runtime-instance word at `+0x78` and fixes its object-relative name pointer at
`+0x7c`.

`RegDMOArena` walks the processed list forward and samples the live
`kLoadDmoLocatorFromStream` value independently for each kind-31 asset. The
registered default is true, but the runtime value remains configurable. With
the sampled value true, userdata mask `0x4` selects Challenge add and its absence
selects Living World add; independently, low three userdata bits equal to 7
invoke `Dmo::Registration::RegDmoLocator`. With the sampled value false, only
userdata mask `0x2` invokes Living World add and no DMO-registry call occurs.

`DRegDMOArena` deliberately does not sample that flag. Userdata mask `0x4` selects
Challenge remove and its absence selects Living World remove; independently,
low three bits equal to 7 invoke `DRegDmoLocator`. Route removal precedes the
registry call, just as route addition precedes registration. `RegDmoLocator`
(`0x825eac28`) keys its 50-entry map by the full processed container GUID,
stores the exact whole-object pointer, and silently keeps the first duplicate.
`DRegDmoLocator` (`0x825ea6b8`) erases by full GUID alone, ignores pointer
identity, and no-ops when absent.

## Presentation dispatch

Presentation registration and deregistration also walk forward, stop at the
first null, ignore kinds above 35, and no-op for unlisted kinds in that range.

| Kind | Presentation action |
|---:|---|
| 0 | texture add/remove |
| 2 | material add/remove |
| 10 | unlockable environment instance add/remove |
| 11 | LivingWorld then DMO presentation model add/remove when userdata bit 3 is set |
| 17 | streamed visual-indicator spline add/remove |
| 18 | instanced visual-indicator spline add/remove |
| 19 | visual-indicator mesh add/remove |
| 20 | loose-world render instance add/remove |
| 29 | emitter waypoint add/remove |
| 34 | hull light probes add/remove |
| 35 | anti-frustums add/remove |

The kind-11 arm samples `(userdata & 8) == 8` independently in registration
at `0x827f1598..0x827f15c8` and deregistration at
`0x827f17e8..0x827f1818`. Registration calls
`LivingWorldPresentation::AddDMOModel` (`0x828112a0`) before the DMO
registration vcall; deregistration calls
`LivingWorldPresentation::RemoveDMOModel` (`0x8280dfd8`) before the DMO
deregistration vcall. Both DMO calls receive the exact `tInstance*` and the
full processed primary ID, although the recovered base registration map uses
the pointer's full local GUID at `+0x68` as its key and ignores the primary ID.

`Dmo::Registration::RegDmoModel` (`0x825eab98`) stores the raw instance pointer
only for an absent local-GUID key; a duplicate key is a silent no-op.
`Dmo::Registration::DRegDmoModel` (`0x825ea628`) erases only when both the local
GUID and stored pointer match the supplied instance. Missing keys and pointer
mismatches are silent no-ops. `Dmo::PresManager::DRegDmoModel` (`0x825ea958`)
then scans its entity map and removes every node whose entity instance pointer
matches. Source allocation identity must consequently remain stable through
last-use deregistration; full GUID equality or equal record bytes are not a
substitute for pointer identity.

Living World's underlying add/remove functions are `0x828107d8` and
`0x8280dd68`. The add route retains a raw Presentation-registry data pointer
and normally derives its map key by resolving the full primary ID to the
instance local GUID. The controlling flag defaults true at
`0x82f8d368..0x82f8d394` but is configurable. Remove erases a present key and
calls `NotifyOfAssetRemoval` even when no entry existed. The current Rust
adapter deliberately leaves these manager and registry effects behind a
void-returning dispatch seam.

`LooseWorldPresentation::AddRenderInstance` (`0x827e10b0`) ignores a null
model or zero mesh count; otherwise it allocates an entity, indexes it by the
instance GUID, and adds it to all render views. This is direct CPU-side render
publication evidence. It is not proof of GPU upload completion or a displayed
frame.

## First use, last use, and completion

`PresThreadMsgMan` owns an `assman::cAssetRefCounter<4096, 2048>`.
`AddAsset` inserts count one and returns true only for an absent full-width
arena ID; existing counts wrap-increment and return false. `RemAsset`
wrap-decrements an existing count, erases and returns true only when its old
count was one, and asserts when the ID is absent. Presentation arena
registration thus occurs only when an absent entry is inserted with count one;
an existing wrapped zero incrementing to one remains retained. Deregistration
occurs only when the old stored count is one and that entry is erased.
`cAssetActivationManager` owns the other recovered
`cAssetRefCounter<1024, 512>` specialization. `SimThreadMsgMan` is only
`0x60` bytes, has no refcounter field, and its reviewed arena and collection
receivers dispatch without this coalescing step. All of these are distinct
from collection loaded-count holds.

Registration/deregistration and callback are separate FIFO messages. Completion
means all preceding CPU-side handler calls returned; it carries no manager
result and proves neither transaction rollback, GPU fences, draw visibility,
nor a completed frame.

The individual Presentation Arena receivers make refcount transition first.
`ReceiveRegArenaMsg` (`0x82780890`) calls `AddAsset` at `0x827808a8`; a
non-first-use return exits without registry lookup. First use then reads the
message's full Arena ID, resolves it through `GetRegAsset` at `0x827808f4`,
loads `mAssetData`, and—only when the Presentation singleton is non-null—calls
`Presentation::RegArena` with the full ID, arena pointer, and message userdata.
`ReceiveDRegArenaMsg` (`0x8277ff38`) symmetrically calls `RemAsset` first at
`0x8277ff50`; non-last-use exits without lookup, while last use resolves the
full ID and calls `DRegArena` with the arena pointer and userdata.

The collection receiver is materially different. `ReceiveRegCollectionMsg`
tests activation at `0x8278096c..0x82780974` and returns immediately when it is
zero, before Presentation-singleton access, collection lookup, iteration, or
refcount mutation. This is the terminal message staged by deferred
registration. Activation true resolves the collection once, then walks its
Arena pointer vector forward and performs the same first-use gate without a
fresh global lookup for each Arena.

Simulation notification precedes Presentation for the parent path, and each
takes an independent loaded-count hold. Those holds remain until the matching
deregistration callbacks release them. Backing decoded storage must therefore
survive through both acknowledgements.

Both subsystems can defer large type-0/1 collection registration when the live
deferral flag admits it, immediate is false, and stream size is strictly above
`0x7d000`. The flag is configurable and must be sampled rather than assumed.
The 1,298,304-byte `cPres_Global` parent is in the size domain where deferral is
possible.

Presentation proxy transitions are asymmetric and material: registration
queues primary RegCollection, optional secondary **DRegCollection**, then the
callback; deregistration queues primary DRegCollection, optional secondary
**RegCollection**, then callback. Deferred completion ends with a no-op
RegCollection carrying activate false, then the optional proxy removal and
callback.

## Reconstruction boundary

The ordering, full-width identities, dispatch tables, owner-specific
refcounter decisions, and FIFO completion position above are **exact**. The
current Rust decision helper deliberately uses an unbounded map instead of the
two recovered fixed EASTL specializations; native capacity, overflow allocation,
and post-assert behavior are not claimed by that host storage policy.

Still open are complete owned processing for every raw TOC class, the live
manager implementations behind several dispatch kinds, failure behavior after
partial side effects, and GPU/backend visibility. Until those are reconstructed,
the PC runtime must call its ordered subsets by narrow names rather than claim
a complete native `RegArena` implementation. The current Simulation-static
plan additionally leaves `ProcessArena`'s single-Texture fast path, the live
`PreventNavmeshLoadingHack` early release, external arena-group references,
unlock/challenge activation, and assembly-backed environment publication open.
The bounded Simulation Environment router now reconstructs admission and
context mutation ordering when the fixed-map/status input is explicitly
provided. The producer of that input and real SimReg/DReg message timing
remain open; the diagnostic resident probe now uses this route only at its
explicit build/`Drop` lifecycle boundary.

The narrower `SimulationStaticArenaRegistrationAdapter` now consumes the
plan's exact `{kind 6, kind 10}` subsequence in forward order for both
registration and deregistration. Its owner-bound live ledger retains each
Environment source while an explicit, non-cloneable complete token authorizes
normal teardown, and it does not coalesce repeated arena registrations. A
safe-host routing failure returns a token that identifies the retained
processed prefix at the faulted adapter-abandonment boundary; it cannot
authorize cleanup. Such a failure permanently faults the adapter because an
unconditional pointer-based remove could consume another live occurrence. It
  is not the complete native list: the selected corpus also predicts
  248 direct processed records outside the plan, including kinds 22, 25, 27,
  28, 31, and 32 plus kind 21 for `LocationManager`.

The owner-bound `LocationArenaPlan` now reconstructs the producer/lifetime side
of that separate kind-21 Location pass. It scans only the first used-dictionary
TOC for local raw `0x00eb0009`, validates each complete `tLocationDescData`,
preserves full TOC GUID and authored order, and retains exact whole-object and
128-byte record identity through the immutable arena owner. Its independent
whole-BAM gate covers 107 containers and 53 locations with signature
`024de4660f5fc535`. It does not yet implement `cLocationManager`'s duplicate-key
multimap, high-priority snapshot, exact-pointer removal, or outer collection
acknowledgement. See [`location-arena-plan.md`](location-arena-plan.md).

The bounded `LocationArenaRegistrationAdapter` now consumes that plan through a
fresh full-ID registry-resolution seam on every Add and Remove. It hashes exact
raw name bytes, skips only null name pointers, samples raw high priority once
per inserted descriptor, appends after equal hashes, and removes the first
equal-hash entry with the exact fresh descriptor identity. Each entry retains
its record owner; occurrence tokens authorize the arena call but deliberately
do not tag individual native-equivalent nodes. Whole-BAM runtime signatures
`2c83b407a844d816` / `8b32aa3e9943123b` cover 2,135 arena occurrences, 107
container calls in each direction, 53 inserts/removes, and zero final live
state. Sorted-vector storage, checked allocation, and sticky faults are labeled
host safety. Nonstreamed debug seeds, manager queries, and the outer receiver
remain outside this slice.

The bounded `SimulationRegistryArenaRegistrationAdapter` covers the six
ID-only normal routes. It retains the complete normal plan per occurrence,
walks kinds 22/23/25/27/28/32 forward in both directions, forwards full `u64`
IDs, and fixes waypoint polarity to literal registration `true` and
deregistration `false`. The whole-BAM gate drives 7,426 calls per direction
with signatures `757b00e47ce5f273` / `b55ae55479c67d26` and exact counts
`{22:6122,23:0,25:75,27:101,28:95,32:1033}`. This remains an explicitly
filtered subsequence: it neither implements those six managers nor preserves
their global interleaving with static and pointer-backed operations. See
[`simulation-registry-arena-adapter.md`](simulation-registry-arena-adapter.md).

The owner-bound `SimulationDmoArenaPlan` now covers the kind-31 producer side
of the separate pass. It scans only the first used-dictionary TOC, retains the
complete decoded arena, validates each local raw-`0x00eb001d` object and span,
and emits one operation plus a cloneable whole-object authority per container
in authored order. Its immutable bytes are provenance for a future writable
fixed-up representation; they do not claim that serialized pointer words are
live host state. The concrete Challenge/LivingWorld/DMO routing, live flag
sampling, 50-entry registry behavior, and collection callback are still
outside this plan.

`SimulationDmoArenaRegistrationAdapter` now consumes that plan as a distinct
per-arena pass. It samples a typed live policy once immediately before each
operation, passes raw userdata and full IDs unchanged, preserves route-before-
registry ordering, and never samples policy during deregistration. A
non-cloneable occurrence token and pre-effect owner ledger retain every whole
object. Mismatched route or registry teardown conservatively moves the owner to
host orphan storage; collaborator unwinds latch their exact phase, operation,
and stage, preserve the reached call prefix, and invent no rollback. This does not
implement the concrete managers, their fixed maps, or the outer collection
pass/acknowledgement.

The bank-scoped `MainDmoBankPresentationRegistrationAdapter` now covers the
complete processed Presentation action list proven for `cPres_Global`, but is
not a generic Presentation receiver. Its single refcounter/token/fault domain
applies the exact outer first-use and last-use decisions to all 125 arenas,
including 124 zero-action arenas. The final arena dispatches one kind-2
material followed by 93 kind-11 DMO records, and both registration and
deregistration walk that mixed list forward. Material is unconditional;
kind 11 independently samples fresh userdata and orders Living World before
DMO.

The material operation retains the complete 29,907-byte serialized
`tRMaterialData` object, not one inline 12-byte `tRMaterial`. Its diagnostic TOC
GUID is not passed to `TextureMan`: `AddMaterial`/`RemMaterial` receive only the
data pointer. Preparation parses the object as a safe-host preflight, while the
handle remains immutable provenance rather than pretending to be the native
fixed-up, writable object. Concrete texture lookup and `m_pStream` mutation are
still external.

Prepared DMO operations validate the 160-byte record plus primary/local GUID
lanes and retain exact record identity. Live collection tokens, first-use
owners, and conservative orphan owners are host safety: if last use supplies a
different material or DMO backing allocation, or fresh deregistration userdata
disables a previously enabled DMO call, the first-use arena owner survives
until dispatcher destruction. Dispatcher unwinds and private-ledger
contradictions latch a permanent host fault, preserve the exact applied prefix
and every source owner, consume the in-flight authority, and perform no
rollback.

The adapter now also owns the deferred individual-Arena receiver front. A
non-cloneable deferred occurrence fixes authored ordinal and full Arena ID
before effects. Reception then calls the same wrapping `AddAsset` boundary
first. Retained use advances without touching the resident lookup. First use
performs a fresh full-ID lookup against the shared published Arena namespace,
requires the exact retained processed owner, and only then dispatches the
prepared Arena. Missing, detached, wrong-kind, expired, or replaced lookup
winners are safe-host fail-stop faults after the already-applied increment;
replay is forbidden because it would add twice. The producer removes its FIFO
front only after successful receiver return.

After all retained Arenas arrive, the activation-zero Collection is modeled as
an effect-free capability transition to ordinary deregistration authority,
matching the receiver's early return. The current queue consumes that marker
only at the FIFO front, then separately extracts its following callback
capability without invoking it. Proxy effects, callback invocation, the
Presentation-singleton-null branch, individual `DRegArena` reception, other
Presentation kinds, native fixed-map allocation failures, and global
callback/postman timing remain outside this slice.

Main-bank teardown reaches `ReceiveDRegCollectionMsg`, not 125 individual
`DRegArena` messages. Its non-null Presentation path resolves the full
Collection ID once before the first `RemAsset`, then walks the Collection's
Arena pointer vector and directly uses the last-use pointer for `DRegArena`.
The adapter's receiver entry now performs exactly one fresh Collection lookup,
verifies the exact retained owner sequence as safe-host preflight, and only
then enters the existing forward removal walk. Lookup/liveness/owner refusal
therefore returns the live registration before any removal. Once removal
begins, the established prefix-preserving failure behavior still applies. The
independent individual `ReceiveDRegArenaMsg` lookup path remains unimplemented
because it is not the dense main-bank teardown route.
