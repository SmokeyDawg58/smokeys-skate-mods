# Main DMO hull lifecycle

## Scope

This record reconstructs the collection-level lifetime around the main DMO
bank: `cDMOBankManager`, its embedded pathed hull loader, physical collection
completion, Simulation/Presentation notification, two shared-latch
acknowledgements, loaded exposure, and the symmetric unload path. It does not
claim complete manager effects, thread latency, or Presentation behavior.

The evidence comes from two external IDA packets,
`world_probe_temp/dmo_bank.txt` and `world_probe_temp/hull_dereg.txt`; neither
packet nor the database is a runtime dependency or repository input.

## Ownership and layouts

`cDMOBankManager` embeds `cPathedHullLoader` at `+0x04` and stores the main-bank
hull ID at `+0x72c0`. The pathed loader embeds `cHullLoader` at `+0x04` and owns
its normalized 256-byte base path at `+0x71bc`.

The exact `cHullLoader` fields used here are:

| Offset | Field |
|---:|---|
| `+0x04` | 32-node fixed map of `tHullLoadInfo` values |
| `+0x7090` / `+0x7094` | active load / unload info pointers |
| `+0x71a8` | `cAssetStreamManager*` |
| `+0x71ac` | hash callback |
| `+0x71b0` | stream-load flags |
| `+0x71b4` | filename clamp |

Each fixed-map node is `0x368` bytes and each owned `tHullLoadInfo` is `0x350`
bytes. Relevant load-info fields are:

| Offset | Field |
|---:|---|
| `+0x000` | raw state `0..16` |
| `+0x004` / `+0x005` | load / unload failure bytes |
| `+0x006` / `+0x007` | register-on-load / unload-requested bytes |
| `+0x008` / `+0x108` | owned 256-byte base path / filename |
| `+0x208` / `+0x20c` | optional load / unload callbacks |
| `+0x210` | high-priority byte |
| `+0x214` | raw asset flags / userdata |
| `+0x218` | memory category |
| `+0x220` / `+0x270` / `+0x2c0` | Simulation / Presentation / Texture exists operations |
| `+0x310` / `+0x318` / `+0x320` | stream IDs |
| `+0x328` / `+0x32c` / `+0x330` | total collection counts |
| `+0x334` / `+0x338` / `+0x33c` | physical load/unload counters |
| `+0x340` / `+0x344` / `+0x348` | registration/unregistration latches |

These offsets are **exact** from constructor and direct load/store
instructions.

## Main-bank setup and load

`cDMOBankManager::LoadMainDMOBank` (`0x82e0e628`) requests `DMO` under
`data\content\world\dmo`, with register-on-load true, no hull callback, DMO
memory category, normal priority, and asset flags 8. `cHullLoader::LoadHull`
(`0x82eeedc8`) ORs bit 1 at `0x82eeee68`, retaining exact userdata `0x0a`.

`cHullLoader::LoadHull` formats those two strings as
`data\content\world\dmo\DMO` with `%s\%s` at `0x82eeee28`, then calls the
constructor-supplied `GetHashValue32` callback at `0x82eeee38`. That wrapper
(`0x82a74490`) calls `tHashName32::getHashValue` (`0x82a74438`), whose body
shifts the accumulator left by four, adds each sign-extended non-NUL byte, and
folds any high nibble with `hash ^= (high >> 23) ^ high`. The exact resulting
`MainBankHullID`, stored at manager `+0x72c0`, is `0x048f3c1f`.
`cHullLoader::GenerateUniqueID` (`0x82eecbe0`) independently repeats the same
format and callback for later regeneration.

The supplied bank has no Simulation manifest and exactly one Presentation
collection. One `cHullLoader::Update` case runs per update:

1. The missing Simulation stream advances to raw state 7.
2. State 7 calls `StartStreamCollectionLoad` (`0x82eed1b0`) for
   `cPres_Global`, then stores total Presentation collections 1, physical-load
   counter 1, registration latch 2, and raw state 8.
3. Physical completion calls `cHullCollectionLoadingCallback::OnComplete`
   (`0x82eed410`), decrements the physical counter `1 -> 0`, observes
   register-on-load, and invokes `InitiateCollectionRegistration`
   (`0x82eecd50`) with `0x0a` and `forceImmediate=false`.
4. That routine creates two callbacks sharing the same latch and calls
   `NotifyCollectionReg` (`0x82d5c8c0`). It invokes
   `NotifyCollectionSimReg` (`0x82d5c468`) before
   `NotifyCollectionPresReg` (`0x82d5c680`). Each increments the collection's
   loaded hold before either retaining a deferred registration record or
   enqueuing its registration and callback messages.
5. Each completion callback at `0x82eed548` decrements the shared latch and
   deletes itself. The shared latch is order-insensitive; cross-thread
   acknowledgement arrival order remains open, and the two completions reduce
   it `2 -> 1 -> 0`.
6. State 8 waits for latch zero, then advances to 9. The next update advances
   `9 -> 10`. `IsHullLoaded` (`0x82eed968`) is true in raw state 10 or 11. A
   following state-10 update would invoke the optional callback before setting
   11; the main DMO bank has no callback.

The native code submits the physical request before initializing the counters.
That relies on asynchronous completion and is **not** a safe ordering to copy
into synchronous Rust.

Both subsystem registration paths have a live deferral branch. They may defer
when the relevant controls allow it, `forceImmediate` is false, and unsigned
stream size is greater than `0x7d000`. This bank exceeds that threshold.
Runtime control values and precise cross-thread latency remain open.

## Simulation collection receiver

`SimThreadMsgMan::ReceiveRegCollectionMsg` (`0x8277e988`) receives the full
collection ID at message `+0x10`, userdata at `+0x18`, and activation byte at
`+0x1c`. When activation is nonzero it walks the collection's arena vector
forward, stopping at the first null. For each visited arena it calls
`Simulation::RegArena` then `LocationManager::RegArena`. It restarts at the
beginning for a second forward `Simulation::RegDMOArena` walk with the same
first-null bound.

The supplied bank contains 125 assets, and the successful host composition
validates all 125 as non-null arenas. Treating all of them as one collection
registration followed by one Simulation acknowledgement is high-confidence
composition; the native receiver's forward first-null control flow is exact.
The existing aggregate kind-11/16 adapter covers only a validated subsequence
and cannot truthfully emit the complete native Simulation acknowledgement.

## Unload and deregistration

Raw state 12 calls `StartStreamCollectionUnload` (`0x82eed290`) in Simulation,
Presentation, Texture order and walks each collection vector forward. Main DMO
has only the Presentation collection.

Physical unload completion at `0x82eed498` increments its counter, samples the
registration latch, and calls `InitiateCollectionUnregistration`
(`0x82eece08`) because the registered bank's latch is zero rather than `2N`.
`NotifyCollectionDReg` (`0x82d59b10`) notifies Simulation before Presentation.
Simulation enqueues deregistration then callback; Presentation does likewise.

`SimThreadMsgMan::ReceiveDRegCollectionMsg` (`0x8277ebb8`) first walks the
arena vector forward through `Simulation::DRegDMOArena`, bounded by the first
null, restarts at the beginning, then calls `Simulation::DRegArena` and
`LocationManager::DRegArena` per visited arena with the same bound.

Each unregistration callback (`0x82eed580`) calls
`DownCollectionLoadedCount` **before** incrementing the shared latch. State 13
waits until the latch equals `2 * TotalCollections`, then requests stream
unload and advances to state 14. State 14 polls `IsStreamUnloaded`. State 15
regenerates the 32-bit hull ID from base path and filename, passes it to the
optional unload callback, selects `OnFail` when byte `+0x005` is 1 and
`OnComplete` otherwise, writes state 16, and falls directly into node erasure
during the same update; an independently entered state 16 also erases the hull
map node.
For register-on-load false, the load phase leaves the latch at `2N`, so unload
skips collection deregistration.

## Failure and reconstruction boundary

Notification functions and receiver effects are `void`; no rollback or
recoverable manager status exists. A missing registered collection or missing
collection asset-list state reaches `FailAssert`/`DebugBreak`. A null
asset-vector slot terminates that receiver walk; a non-null asset whose
arena-data pointer is null also reaches `FailAssert`/`DebugBreak`. Callback
allocation failure is forwarded as a null callback without counter
compensation, which is high-confidence evidence for a permanently incomplete
latch but has not been observed in a live trace. Physical-load failure callback
behavior is still open.

A truthful native coordinator may model the exact one-collection/two-ack
latches and retain the complete collection owner, while keeping Simulation and
Presentation as opaque collaborators. Simulation may acknowledge only after
its normal, Location, and separate DMO passes are complete; therefore the
current kind-11 diagnostic subset must not synthesize that acknowledgement.
The coordinator must not request stream/manifest unload until both
deregistration callbacks have released their holds. Any later host catalog
release remains an explicitly separate composition boundary.

## Native coordinator milestone

`dmo_hull_acknowledgement` now implements this bounded one-collection path as
an owner-thread Rust state machine with opaque complete-subsystem ports. It
starts at raw state 8 with physical count 1 and registration latch 2, preserves
Simulation-before-Presentation hold/dispatch order, accepts the two distinct
non-cloneable completion capabilities in either arrival order, and advances at
most one recovered state case per `update`. Loaded exposure remains restricted
to raw 10/11.

The unload half retains an explicit request in state 11 until the next update,
submits the supplied bank's sole Presentation collection from state 12 before
publishing state 13, and dispatches Simulation `DReg` before Presentation.
Each `DReg` capability is paired with its own occurrence-bound resident hold.
The coordinator accepts only the sealed real `CollectionResidentStreamer`
release authority, so a caller cannot increment the latch without executing
the resident Down. A successful Down precedes its latch increment; a teardown
fault after decrement consumes both capabilities and permanently faults the
coordinator. State 13 rejects stream unload at latch values 0 and 1, requests
only Presentation at 2, and state 14 polls Simulation, Presentation, then
Texture even when an earlier result is false.

The optional state-15 callback is exposed with the caller-supplied 32-bit hull
ID and explicit Complete/Failed selection. This ID must come from the proven
path/filename hash boundary; the coordinator does not guess or recompute it.
Whether the still-unlocated main-bank unload caller supplies the callback
remains open; selecting no callback is a labeled host composition, not a claim
about that caller. State 16 and map-entry erasure occur in the same completion
call. External-effect reentry is guarded, panic prefixes retain their precise
dispatch stage (including each individual state-14 poll), and callback domain
plus checked generations are high-confidence host safety rather than native
record fields.

The native physical callback does not inspect the resident reference count.
Requiring a freshly published count of 1 before taking the two registration
holds is therefore explicitly a high-confidence integration precondition for
this frozen main-bank composition. It prevents the currently unmodeled
already-resident/cache-hit case from being mislabeled as recovered behavior.

Fourteen focused tests drive an actual `CollectionResidentStreamer` through the
complete load and unload path. They cover the exact request words and dispatch
order, `1 -> 3` holds, reverse-order acknowledgements, separate state polls,
base unload `3 -> 2`, two hold releases through destruction, latch gating,
three-poll order, hull-ID-bearing Complete/Failed/absent notification and
same-call erasure, synchronous state-12 completion, foreign/drop behavior,
consumed teardown faults, generation exhaustion, fresh-resident precondition
rejection, recursive-dispatch rejection, and precise panic prefixes. This is
deterministic host integration validation; a reference runtime trace of
callback latency and the complete Simulation, Location, separate DMO, and
Presentation receivers remain required before end-to-end registration parity
can be claimed.

The resident callback also has a narrow mapped-dispatch boundary for an
application-owned callback enum. It forwards the outer streamer's genuine
active callback context rather than reconstructing one, so both holds remain
bound to the correct streamer domain and resident occurrence. No queue or
release authority is exposed. A thirteenth focused test drives both load and
unload through that outer enum and reaches exact final resident destruction.
This closes a host composition seam only; it does not make either opaque
subsystem receiver complete.

## Application real-corpus prefix

The optimized `--inventory-bam-sim` gate now drives a second physical load of
the supplied `cPres_Global` through `CollectionResidentStreamer` and the mapped
coordinator callback. The real phase proves 125 assets in physical XST order,
93 represented kind-11 operations, wrapper-create then wrapper-publish then
Simulation/Presentation dispatch, exact collection/userdata/force-immediate
request fields, resident count `1 -> 3`, raw state 8, physical counter zero,
registration latch 2, and a released physical request-ID lease. An extra
coordinator update cannot advance without either capability.

The port only captures those two opaque capabilities; it executes no
Simulation, Location, separate DMO, or Presentation effects. After the raw-8
observations are fixed, an explicitly named synthetic mechanical cleanup
returns the capabilities in reverse order, advances the state machine, and
uses the real streamer for base Down plus the two occurrence-hold releases.
That cleanup validates `3 -> 2 -> 1 -> 0`, 125 ordered garbage callbacks, exact
wrapper removal, and detached weak ownership. It connects the sole
Presentation stream-unload request to the actual host manifest unload. The
absent Simulation/Texture polls return true only as labeled host cleanup
policy, and state 15 selects the documented no-callback composition.

This gate proves the real-data host resident-wrapper publication/hold prefix
and clean ownership mechanics. Its synthetic returns are not native subsystem
acknowledgements, raw states 9 through erasure are not evidence of complete
receivers, and host manifest completion is not a trace of native cross-thread
latency.

## Provenance hashes

- load/state packet: `973DC02215376DA0ADD3893C29F5A0C9E5E6FE25D9BD786C30E6785AD0013B6C`
  from IDA-database snapshot
  `100602C06679BC32FD53C437E0A6EA25DB1ED1421A52227CC7E4891082A8CC7A`
- current hull/receiver/unload packet:
  `301DBD5BDE74FCB2D49177DC9069431673E547BA68B99D2D795F2DDDCB4C8F26`
  from IDA-database snapshot
  `E362FD6EDF4480A6CFAD62C0D9578C75651BB72ACC3883FC438F3851877A76D9`
  with extraction script
  `548C3CC7CF1D10E424DC726D12FB2B01729575209586D20C502D2F217F6512C1`
- XSM: `08532DAC62666E4885802BF1DA4200C482021FB3859FB27C9E4C2458D5344EC2`
- XST: `830A99A4361BB09EF42CF134C6F14813426594E41773C5536AEFCDAD8BAE9EB0`
- XSF: `477D443547F9F5DC6C6459AD0B00812D17A2DC15F57E64952ADA68F87E32B957`
