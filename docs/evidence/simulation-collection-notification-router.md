# Simulation collection notification router

## Scope and provenance

`SimulationCollectionNotificationRouter` composes the already-proven
registration admission, immediate collection receiver, deferred message FIFO,
deregistration receiver, and callback positions into one ordering domain. It
models one outer FIFO for immediate `RegCollection`, deferred `RegArena` and
activation-zero `RegCollection`, both callback classes, and
`DRegCollection`.

Primary references are the debug/devkit XEX, PDB, and MAP with SHA-256 values:

- XEX `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- Simulation receiver packet
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`;
- deferred setup/batch packet
  `848A059F6CD02F85E40DCE97FE4160F112D693DE70D634A49F63205C03FBBE3C`.

The executable, symbols, decompiler database, and extracted instruction
packets remain outside the repository under
`world_probe_temp`.

## Proven native sequence

`cAssetStreamManager::NotifyCollectionSimReg` at `0x82d5c468` increments the
loaded count, resolves the current stream type, and selects one of these
routes:

```text
supported immediate: RegCollection(activate=1), Callback
supported deferred:  RegArena*, RegCollection(activate=0), Callback
unsupported type:    Callback
```

`NotifyCollectionSimDReg` at `0x82d59a00` performs a fresh type lookup with
fresh userdata and independently selects:

```text
supported type:   DRegCollection, Callback
unsupported type: Callback
```

Every entry above is a separate `0x20`-byte enqueue/send. Immediate, terminal,
callback, and DReg messages use `SendMsg`; each deferred Arena instead performs
the equivalent explicit lock, `Message::Enqueue`, and unlock. Each operation
has its own lock interval, so unrelated producers may interleave between them;
there is no atomic notification batch. Immediate registration increments
loaded count before the lookup, while deregistration does not modify it. A
missing type-map entry takes the native assertion path rather than the
callback-only path.

Callbacks are sampled at notifier entry. A deferred notifier retains that
pointer until its later Callback send. The callback receiver treats null as a
no-op and otherwise invokes the pointer synchronously while the native queue
front is still current.

`SetupNextActiveCollectionReg` at `0x82d5b268` destructively moves the oldest
pending 48-byte row to the active slot, then resolves its full collection ID
through `cAssetRegistry::GetRegAsset`. This happens during deferred producer
activation, not at Notify admission. Each later Arena message contains its own
full arena ID, and `ReceiveRegArenaMsg` independently resolves that ID at
`0x8277e7fc`. After the Arena prefix, the activation-zero Collection receiver
again resolves the full collection ID at `0x8277ea08` before reading the
activation byte and entering its DMO-only pass. Producer setup and terminal
receiver lookup are therefore separate collection lookups.

## Readable safe-host model

The router freezes an opaque nullable callback capability at each notification
entry but appends and dequeues its callback as a distinct FIFO node. Deferred
Arena, Finish, and Callback nodes use reserved outer markers over the existing
private deferred FIFO, so they cannot bypass immediate or callback-only nodes.
Registration occurrence, deferred producer, and callback permits are
non-cloneable and tied to one weak domain identity. Deregistration consumes the
delivered occurrence and samples a fresh userdata word.

The router constructs its collection adapter internally from fresh child
passes; an adapter with untracked live registrations or a prior fault cannot
be adopted. Direct immediate and deregistration calls are unwind-contained.
The first entered receiver fault is sticky, preserves the front and later
nodes, and causes later enqueue attempts to return exact caller authority.
Deferred child faults retain their typed source. Drop performs no implicit
dispatch, callback invocation, acknowledgement, rollback, or teardown.

The resident-backed immediate path now installs one fixed nonowning Simulation
resident lookup domain in the router and enqueues only the full `u64`
collection ID and userdata. When that node reaches the front, the receiver
performs the lookup, consumes the exact ID/token occurrence into a dense
prepared bundle, and only then enters the collection adapter. The bundle stays
bound to the session across the separate callback. A supported-class DReg node
performs a second fresh lookup and dense preparation before extracting the old
adapter registration. The old registration is used only when registry-domain,
full ID, occurrence token, and exact ordered immutable plan-owner snapshot all
match. The snapshot check closes a host ABA-like hole where an append can keep
the same occurrence token while changing the resident sequence.

This ordering follows the independently observed receiver lookups at
`SimThreadMsgMan::ReceiveRegCollectionMsg` `0x8277ea08` and
`ReceiveDRegCollectionMsg` `0x8277ec38`. A missing collection is represented as
a sticky recovered assertion boundary with no manager effect; detached or
poisoned host registry state remains a distinct typed infrastructure fault.
Preparation failure retains its exact non-cloneable occurrence for inspection.
A same-ID reload between Reg and DReg is currently a typed unsupported identity
boundary: native code would operate on the newly resolved pointer, while the
current child adapters cannot use an old registration ledger to tear down a
different prepared owner. Occurrence and plan-owner comparisons are safe-host
stability policy, not native pointer comparisons.

The resident-backed deferred begin API represents caller-invoked
`SetupNextActiveCollectionReg` timing after a pending row has been selected. It
looks up the full collection ID, performs dense all-view preparation, and
retains that exact occurrence across its Arena producer sequence. An unresolved
ID, exact preparation failure, or already-prepared bundle is returned with the
frozen userdata and callback when host setup rejects before a session exists.
This typed recovery is host policy: the stateful native pending vector has
already removed its front before lookup, and missing data is subsequently
dereferenced.

Each resident-backed Arena producer step freezes the current prepared slot's
full `u64` outer asset ID into its distinct outer FIFO marker. When that marker
reaches the front, the router performs a fresh lookup in the registry's shared
Collection/Arena occupancy domain before touching the child FIFO continuation.
The selected weak asset is upgraded under the registry lock, then held strongly
only across normal Simulation and Location dispatch. Missing, detached,
poisoned, expired, and wrong-class lookups become distinct sticky pre-effect
boundaries. Because the existing child ledger was prepared at Setup, the safe
host also requires the freshly resolved asset's immutable plan allocation to be
pointer-identical to that slot's early-bound plan. A different current registry
winner faults before manager effects; native code instead uses the pointer
returned by its live lookup and does not perform this Rust pointer comparison.

Arena candidates are snapshotted as ordered weak handles when the resident
wrapper is created. Publication installs only the first active candidate for a
full ID, and exact collection removal erases only rows backed by that
collection's weak asset allocations. This wrapper-time candidate snapshot and
first-published-winner rule are explicit safe-host policy around the recovered
lookup seam, not a claim about native registry layout, refcounting, or duplicate
promotion.

Native collection wrappers and outer Arena assets occupy the same
`cRegisteredAssetContainer` key domain, so a full-ID collision across those two
classes leaves the first inserted pointer as the row seen by either lookup.
The host registry likewise gives Collection and Arena rows one occupancy
domain, but records their class and reports `PublishedKindMismatch` rather than
interpreting a wrong-class pointer. That typed failure is a safe-host boundary,
not native recovery. This model is deliberately narrower than the complete
native container registry: other registered asset classes, including processed
inner assets, are not represented here.

When a resident-backed activation-zero marker reaches the outer FIFO front,
the router performs a second full-ID collection lookup and preparation before
allowing the child FIFO to extract its continuation or enter DMO. Missing,
lookup, preparation, occurrence mismatch, and ordered-plan snapshot mismatch
are distinct sticky faults retaining the Finish/Callback fronts and all owned
state. On success the fresh terminal bundle replaces the setup bundle and is
retained through callback and the independent DReg lookup. A terminal same-ID
replacement would be accepted as a new pointer by native code; rejecting it is
an explicit host boundary required by the existing cached child ledger.

This is not the complete native notifier. Registration starts require the
caller to run the admission selector after loaded-count mutation and locked
type lookup. DReg starts separately require the independent fresh locked lookup
and DReg classifier, with no loaded-count mutation. The prepared immediate/DReg
entry points replace those receiver lookups, while the prepared deferred entry
point replaces producer Setup and terminal Collection lookup; all remain
explicit compatibility/probe APIs. The resident path fixes collection lookup
positions for immediate registration, caller-invoked deferred producer
activation, the terminal activation-zero Collection receiver, and stable
supported-class deregistration. It also fixes each deferred Arena message's
full-ID lookup position at its FIFO front under the stable-plan restriction
above. Pending-vector selection/ownership and producer null-slot dereference
remain open. The current adapter supports only a stable supported/unsupported
stream class between registration and deregistration;
native code does not impose that restriction. A changed class returns a typed
`StreamClassMismatch` with the occurrence and frozen callback rather than
inventing manager behavior.

Owned Rust callback capabilities also differ from nullable non-owning native
pointers: they are returned, never invoked, and ordinary Rust destruction may
drop them. Queue capacity and mutex topology, loaded holds, map assertions,
global EPostman scheduling, callback invocation timing, and production hull
acknowledgement remain outside this layer.

## Verification

Thirty-four focused router tests plus the registry tests cover:

- separate immediate registration and deregistration callback nodes;
- callback-only registration and deregistration;
- callbacks sent after their primary receiver returns while retaining the
  entry-time payload;
- empty and nonempty deferred sequences;
- a deferred Arena interleaved with an immediate collection in one FIFO;
- explicit stable-class rejection with exact occurrence/callback return;
- foreign authority rejection;
- a sticky exact nested registration-fault prefix with a later callback
  retained;
- session-ID exhaustion before effects;
- independent registration and fresh deregistration userdata snapshots;
- resident publication after enqueue but before immediate receiver dispatch;
- sticky zero-effect removal before the immediate receiver front;
- same-ID reload before immediate dispatch selecting the replacement;
- independent DReg lookup rejecting an ABA replacement while retaining the
  old live registration and later callback;
- DReg removal producing the recovered missing/assert boundary before token
  extraction;
- one real resident arena crossing preparation and observing independent Reg
  and DReg userdata;
- detached resident preparation retaining its exact nested failure;
- child registration fault retention after successful resident preparation;
- sticky-fault precedence over the unavailable-resident-lookup enqueue class;
- resident deferred setup missing/unavailable/detached-registry/detached-asset
  recovery with full ID, userdata, callback, occurrence, or prepared-bundle
  ownership as applicable;
- outer-ID exhaustion and child-begin rejection after resident preparation;
- a successful resident Arena/Finish/Callback/DReg flow with distinct userdata;
- resident Arena lookup occurring only when its outer FIFO marker reaches the
  front, with removal producing a sticky missing boundary before child effects;
- a same-ID shared-domain first-winner plan mismatch faulting before deferred
  child
  progress;
- terminal removal, detached registry, detached preparation, same-ID
  replacement, same-occurrence append, and DMO child-fault retention before
  false completion; and
- same-occurrence append rejection before DReg with the old registration and
  FIFO front retained.

The optimized `--inventory-bam-sim` gate runs a second isolated adapter/router
over all 284 real BAM Simulation collections. With raw type `1`, enabled defer
controls, false `forceImmediate`, and userdata `10`, it observes:

- 205 immediate collections containing 743 arenas;
- 79 deferred collections containing 1,392 arenas;
- 205 immediate collection receiver nodes;
- 1,392 deferred Arena nodes and 79 activation-zero Finish nodes;
- 284 registration Callback nodes;
- 284 `DRegCollection` nodes and 284 deregistration Callback nodes; and
- 2,528 total nodes, with no identity/order mismatch and quiescent router,
  collection adapter, observer, and Location source state.

An independent oracle at
`world_probe_temp\notification_router_oracle\src\main.rs`
(SHA-256
`C1AFE892BE1A0A305F0D3B5608D4513768B46BB447D69705E01A292FBE9689F0`)
defines the V1 protocol and predicts signature `84efde6b9be3a067`. The
in-repository corpus pass reproduces it. Its sibling forced-deferred pass is
unchanged and still reproduces all seven prior signatures:

`13a98da6233ecb1d`, `26dc539be7eeaabc`, `42f92d5d50c74ec9`,
`8b3a8a23042ff350`, `350c8e486be71e16`, `04978e44d60002fd`, and
`b1afe10e126ba608`.

That V1 gate still uses the compatibility prepared-plan route. The separate
opt-in `--inventory-bam-sim-resident` gate now composes the mounted
production-code loader, owned residents, safe-host weak registry, loaded-count hold, and this router over
all 79 deferred-eligible collections/1,392 arenas. Its three collection
resolution points perform 79 Setup, 79 terminal, and 79 DReg lookups and
preparations. The 1,392 individual Arena FIFO fronts additionally perform
1,392 successful fresh full-ID lookups before normal/Location effects. The
notification protocol reproduces the independently frozen V2 signature
`92164ff9ad83bb59` over 277,522 bytes, including 1,708 outer nodes and 1,550
child-FIFO nodes.
The formats-only V2 source is
`world_probe_temp\arena_id_census\src\bin\resident_deferred_v2.rs`
(SHA-256
`8B79751A0AABFFE23CF29A18AA4AF5E626AC791975A2EEBF4B9026FE6EA41C20`).
It inserts `K` between each authored `A` and its `N,L` receiver effects and
freezes 1,392 attempts, 1,392 hits, zero misses, and zero identity mismatches.
Separately, the in-repository gate asserts 79 physical loads and 79 exact
unloads. See
[`simulation-resident-notification-corpus.md`](simulation-resident-notification-corpus.md).

That production-code diagnostic corpus is deliberately one collection at a
time and dense.
It does not establish native pending-vector/batch interleaving, null-slot
termination, current-winner replacement behavior beyond the explicit host
fault, callback invocation, or acknowledgement.

## Private main-DMO caller-asserted receiver boundary

Core commit `7194982` adds a private session for the exact main DMO bank beneath
the sealed source-binding hierarchy. It does not replace or claim integration
with `SimulationCollectionNotificationRouter`: it owns no FIFO and accepts the
caller's assertion that a particular message has reached the front. Its
effectful boundaries reproduce the three proven same-producer sequences:

```text
immediate: Collection(activate=1), Callback
deferred:  Arena*, Collection(activate=0), Callback
teardown:  DRegCollection, Callback
```

The star is the native forward first-null prefix, whose construction remains
open. This pinned dense binding instantiates it as exactly 125 caller-asserted
Arena fronts.

The session withholds the opaque registration callback until the complete
active receiver, or all 125 deferred Arena receivers plus the activation-zero
receiver, succeeds. It retains the complete registration internally until a
later staged `DRegCollection`; the deregistration callback is withheld until
that receiver succeeds. Returning a callback capability is not invoking it and
does not acknowledge the hull.

Immediate versus deferred is represented by distinct caller-selected
constructors. The session does not read the live global or manager defer
controls, query size, choose admission, assign batch cadence, allocate/send/
dequeue a message, or model EPostman locking and interleaving. Constructors and
deregistration begin only stage state; each effectful receive method requires a
caller-asserted front. This is intentionally narrower than claiming the
existing generic router can consume the main-bank binding.

Wrong-stage rejection restores the exact state. A deferred-begin refusal owns
the callback and coordinator and permits only same-route retry. Reached
receiver failure withholds the callback and retains the no-rollback prefix;
Drop performs no implicit receiver dispatch, callback invocation, hull
acknowledgement, rollback, or native teardown. Ordinary Rust destruction of
owned fields still occurs. Typed stages, retry, sticky faults, and ownership
rules are safe-host policy. The native notifier and receiver interfaces are
`void` and establish no recoverable error contract.

Five tests raise the core inventory to 1,636 and cover separate immediate/
DReg callback fronts, all 125 deferred Arena boundaries before activation-zero
DMO and callback, exact active and DReg failure prefixes, callback withholding,
wrong-stage restoration, retry ownership, and no implicit protocol work on
Drop. The strict workspace gate also passes 173 format tests, 19 application
tests, three symbol tests,
two compile-fail doctests, formatting, diff checks, and warning-denying Clippy.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_notification_receiver.rs` | `28A1C1769FBADD3434D60AB692130AC12909C25F186689243BEB3585F22B2D69` |
| `simulation_dmo_bank_collection_binding.rs` | `3FEC94646511B2CAFE587149FA7DE17B896355331AB0A5432B5D5D4F410247E5` |

An actual FIFO permit, native receiver-time registry/first-null behavior,
callback invocation, hull acknowledgement, Presentation completion, and live
defer-control trace remain separate work.

## Pinned main-bank kind-31 absence gate

Core commit `167b62c` closes an ambiguity before the caller-asserted receiver
can be given FIFO or hull authority. The native processor table binds raw type
`0x00eb001d` to `cProcessArenaAsset::ProcessDMOData` at
`0x82d609cc..0x82d60a08`; `ProcessDMOData` at `0x82d5f4f0` emits one kind-31
operation for each matching first-TOC row. The frozen processor packet is
`world_probe_temp\remaining_processors.txt`, SHA-256
`C4C11506EA241F207998C3ED78E6E68E339DCFCDE2A726C22D31CBA04B8566C2`.

The supplied dedicated `DMO_Pres/cPres_Global` census in
`world_probe_temp\dmo_processed_factory_output.txt`, SHA-256
`7C765C278490E233726E975DBD54429A17689E99935D4205663E2F16D205A1EF`,
parses all 125 arenas with no read failures. Its complete dictionary-type map
contains no decimal `15400989` (`0x00eb001d`), while the same output records
815 such containers in the separate world/BAM Simulation corpus. Therefore
the pinned shipped main model bank has zero kind-31 container operations. This
is a corpus fact, not a native format restriction or a claim about other
collection banks.

The sealed binding now sums the operation counts of its already-prepared
kind-31 arena views with checked host arithmetic after the existing 125-arena,
93-kind-11, ordered-ID, and plan-preparation gates. A nonzero total is rejected
before the binding is published. The new adversarial fixture retains the exact
125/93 shape but inserts one native-selected `tDMOData` container containing
two 128-byte locator records; the binding reports one operation, proving that
the gate counts containers rather than contained locator records. A 92-row
poison case freezes the earlier kind-11 validation precedence.

One focused test raises the core inventory to 1,637. Independent code and
evidence audits pass, as do the full workspace tests, formatting, diff checks,
and warning-denying Clippy.

| Source | SHA-256 |
|---|---|
| `simulation_collection_adapter.rs` | `D434ABAFCC480D478086BFB039F221AAB7F2B54A8CF58A66CC75384E3AE56FFE` |
| `simulation_dmo_bank_collection_binding.rs` | `35E7E0F0722EFA0D07D5DC8BA555C87B4A0B772F992A18AE53794A293CBD3D1A` |

## Exact-main-bank occurrence marker FIFO

Core commit `1d4873e` places a private occurrence-local marker FIFO around the
exact main-bank receiver. Producers append the proven same-producer sequences
one marker at a time:

```text
immediate: Collection(activate=1), Callback
deferred:  Arena[0]..Arena[124], Collection(activate=0), Callback
teardown:  DRegCollection, Callback
```

Each append reserves one host slot before changing its non-Clone, Weak-domain
producer permit. The 125 ordinals are the sealed dense shipped-bank instance;
they do not claim native first-null vector construction. Separate append calls
preserve the fact that native `SendMsg` operations lock, append, and unlock
independently. A focused test dispatches the immediate Collection to an empty
FIFO before the later Callback is appended, so the Rust layer does not require
an invented atomic batch.

`dispatch_one` peeks one front and removes a Collection/Arena marker only after
the corresponding receiver call succeeds. A reached receiver refusal latches
the first host fault, keeps the failed marker and suffix queued, withholds its
callback, and preserves the already-applied prefix without rollback. Domain
permits, typed stages/allocation failures, sticky faults, and retry behavior are
safe-host policy; native receiver and queue calls are `void`.

The registration callback has the stronger native ordering. `Queue::Pop` at
`0x829827b8` runs the handler while the front is still current; the callback
receiver at `0x8277be10` synchronously calls the non-null callback; the main
hull callback at `0x82eed548` atomically decrements its latch before deleting
itself. Accordingly, the FIFO extracts the opaque capability only at the
Callback front, calls the real hull acknowledgement while that marker remains
front, and pops only after success. A wrong hull returns the exact capability
to FIFO-owned in-flight storage and blocks the suffix; retry against the right
hull neither reruns the collection receiver nor decrements either wrong-domain
latch. That recoverable refusal is host policy, not a native error path.

The first FIFO checkpoint deliberately stopped one boundary earlier at the
DReg callback front. Native callback `0x82eed580` performs
`DownCollectionLoadedCount` at `0x82eed598`, atomically increments the shared
unregistration latch at `0x82eed59c..0x82eed5b8`, then deletes itself before
queue advancement. Core commit `0211c36` closes that boundary without adding a
callback mint: the FIFO validates its sole non-Clone domain-bound front, keeps
the marker current, and passes its retained opaque typed callback to the real
hull coordinator together with a sealed `MainDmoResidentHoldReleaser`.

The hull coordinator owns the occurrence-bound hold acquired by the physical
load callback. A successful acknowledgement therefore executes the streamer's
resident Down and zero-count teardown first, advances the hull latch second,
and only then returns to the FIFO for its infallible marker pop. The returned
boolean is only the safe-host streamer's resident-wrapper-destroyed
observation; native `OnComplete` and queue functions are `void`. The complete
ordering is:

```text
resident Down / optional zero-count teardown
hull unregistration latch increment
FIFO callback-marker pop
```

Wrong-hull, foreign-streamer, and pre-existing unavailable-streamer refusals
restore the exact callback to FIFO-owned in-flight storage and return the same
front capability for retry. These typed refusals are fail-closed Rust policy,
not native callback return paths. Once a valid hold reaches resident Down, a
teardown error or panic has consumed both hold and callback authority: the FIFO
keeps the callback marker current, latches a sticky no-rollback fault, and does
not invent a retry.

The module neither owns global `EPostman` nor claims queue capacity, unrelated-
producer interleaving, drain cadence, reentrancy, or a frame boundary. Its
`dispatch_one` is an integration primitive, not a claim that native Flush
processes only one node. Test-only hull capability issuers are compiled solely
under `cfg(test)`; they add no production minting API. Drop performs no
implicit protocol operation, although ordinary Rust destruction of owned
fields still occurs.

Nine tests raise the core inventory to 1,646. They cover late callback enqueue,
the exact 125-marker deferred route, wrong-hull retry with both latches
unchanged, foreign producer recovery, sticky receiver failure and inert Drop,
and DReg collection-before-callback with an unadvanced callback front. The
three completion tests use the real physical load/unload callbacks and real
`CollectionResidentStreamer` holds rather than a fake releaser. They prove
wrong-hull and foreign-streamer retry, successful Down-before-latch-before-pop,
a reached teardown error, and the distinct caught-panic path. Independent
evidence and implementation audits pass, as do all strict workspace gates.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_marker_fifo.rs` | `327527A2193062DFB9B91D1B641CD256E2968C0664370B49E1694F3E86C8BF46` |
| `simulation_dmo_bank_collection_binding.rs` | `700765F1206FDF0B6ED6E68CEF9EF8381A239DFE10171B6FE247621CE00E19CF` |
| `dmo_hull_acknowledgement.rs` | `4F831EC8FB12F70B8AB6C22B868E38692DA974DFFD65AEC35BE6D3C261BD8DD2` |

## Exact published-occurrence main-bank acquisition

Core commit `e4de462` closes the next ownership boundary without inventing a
second resident registry. The native notification entry at
`0x82d5c468..0x82d5c67c` performs the Simulation resident Up at
`0x82d5c48c..0x82d5c494` before route selection. The later consumers perform
fresh lookups: deferred Setup at `0x82d5b32c` within
`0x82d5b268..0x82d5b374`, terminal/immediate registration at `0x8277ea08`,
and deregistration at `0x8277ec38`. The direct production-chain and deferred-
setup evidence packets remain SHA-256
`3F17155988DACEB338E4B6CFF7DC70AD31B25FA7A74F3D41796CA3E42DA7D028`
and
`848A059F6CD02F85E40DCE97FE4160F112D693DE70D634A49F63205C03FBBE3C`.

The staged Rust acquisition consumes one resolved
`SimulationResidentOccurrence` specialized to the processed main DMO bank. It
first revalidates the exact active registry domain, full collection ID, and
monotonic occurrence token. While that registry row remains protected, the
resident collection takes one lock and verifies that its live asset vector is
still the wrapper-time sequence by exact count, authored order, full 64-bit
asset ID, and outer `Arc` allocation identity. It reserves and clones all
owners under that same lock; identity mismatch or allocation failure publishes
no partial owner.

The result is an opaque, non-Clone, non-Debug snapshot. It has no asset getter
or generic strong-snapshot conversion. A private-constructor consumer owned by
the main-bank model subtree is the sole handoff that can join its exact owners
with the occurrence liveness token. Preparation failure reconstructs the whole
opaque snapshot; later collection-gate failure retains the whole model batch.
The outward owning failure borrows only typed diagnostics and supports
consume-to-retry, so no occurrence, snapshot, asset owner, liveness token, or
model-batch half can be independently extracted and cross-paired.

Every published model callback issue resamples both exact occurrence identity
and resident collection liveness. The retained strong assets keep validated
record storage safe but do not imply residency. Tokens, outer-pointer checks,
strong retention, retry ownership, typed allocation/poisoning failures, and
the sealed consumer are safe-host Rust strengthening. They are not attributed
to native resident-hold, occurrence-lifetime, callback-return, or retry
semantics. Direct construction from an arbitrary resident collection remains
available only under `cfg(test)`.

The production-shaped test publishes the complete pinned bank and derives one
inseparable binding containing the same 125 ordered processed arenas, 93
kind-11 rows, and zero kind-31 operations. It issues one validated model
callback while live, removes the exact publication, and observes a stale
refusal on the next pass. Additional tests reject wrapper-time append, dead
resident state, wrong collection, stale same-ID reload, equal IDs backed by
foreign outer allocations, a one-slot foreign-owner substitution, equal low
32-bit IDs with different high words, malformed model sources, and wrong
corpus order. Both fallible binding cases retain opaque retry authority.

Nine tests raise the core inventory from 1,646 to 1,655. Three independent
implementation, ownership, and evidence audits pass. The complete workspace
gate passes 1,655 core tests, 173 format tests, 19 application tests, three
symbol tests, two compile-fail doctests, formatting, diff checks, and warning-
denying workspace Clippy.

| Source | SHA-256 |
|---|---|
| `owned_resident_collection.rs` | `CCFAEB0601CB14903ACE1DA5DB5F3C6DCD4C16D6635104B85B85D29F90D2FC31` |
| `simulation_resident_registry.rs` | `1956A5B1170ECBBA49E59AC98783395CD3E142A849C4E76F9B456237A921577A` |
| `simulation_dmo_bank_model_registration_batch.rs` | `1B71BE1DBD4FD60BBA7F1976B04396D1183C1231AA8B26922A58E158170AF3E8` |
| `simulation_dmo_bank_collection_binding.rs` | `E1CDE0672A0C50EC2F0EDA86620E43FD4DD11B01F40E1510355B876DFB4A6A88` |
| `simulation_dmo_assembly_part_session.rs` | `734FE7FC02FB2194A38B8B0F216BA91E19210A4134FF970F99939DC3D89855DC` |

At this checkpoint the seam was not yet invoked by application production
flow because the app probe still used a separate registry. The following
checkpoint pairs publication with the core registry, but sealed binding
acquisition remains deliberately uninvoked until the correct fresh receiver
front. Live defer values, global queue ownership/interleaving/Flush cadence,
Presentation completion, native first-null construction, locator registration,
`SpawnStatic`, and DMO Body construction remain separate.

## Paired production probe and registry-domain authority

Application/core commit `526c311` replaces the DMO resident probe's separate
weak-view publication with a composite that owns the existing telemetry and
the real `SimulationResidentRegistry<DmoBankProcessedArenaAsset, ...>`. The
core registry is the sole published-occurrence authority. The app row and its
fixed diagnostic value `1` remain probe telemetry; they are not modeled as a
second native registry or an independently meaningful occurrence identity.

After the real load callback has run and both subsystem holds exist, the app
uses the matching non-owning core lookup only to validate the mounted corpus.
The observed occurrence contains the same 125 authored IDs in order and 93
kind-11 DMO operations. It remains visible after the base resident decrement
at count two and after the first subsystem release at count one, then is absent
after the final release removes the wrapper. This is a read-only probe. It does
not consume the sealed main-bank snapshot, build the model binding, dispatch a
marker/receiver, or place native `GetRegAsset` at this observation point.

Adversarial review also found that the reusable core token previously carried
only a per-registry `u64` serial. Since two fresh registry instances both begin
at zero, a foreign same-generation token could match another registry's row.
The fixed token is now the pair of a serial and a `Weak` reference to one
private per-registry domain. Equality requires both the serial and
`Weak::ptr_eq`; no process-global counter or copied host address is used. The
registry alone holds the strong domain owner, so wrappers and retained tokens
cannot extend registry lifetime. Keeping the stale `Weak` also prevents an
allocator reuse from making a later domain compare equal.

No public error ordering changes. `publish` still locks first and maps a
foreign token to `UnknownPendingToken`; `remove_exact` still checks the active
full ID before token equality and maps a foreign token to `TokenMismatch`.
Poisoning therefore precedes token validation, while a missing removal ID
precedes mismatch. The app composite performs its old telemetry prechecks,
then its own composite-domain check, then the core effect, and commits telemetry
only after success. Its outer domain prevents a token assembled by another
composite from pairing diagnostic and core halves.

All domain identity, non-owning token, ABA refusal, sanitized Debug, typed
failure, and telemetry rules are safe-host Rust policy. They are not attributed
to a native token layout or recoverable native API. The recovered native timing
facts remain the earlier Simulation Up at `0x82d5c48c..0x82d5c494` and fresh
lookups at deferred Setup `0x82d5b32c`, terminal/immediate registration
`0x8277ea08`, and DReg `0x8277ec38`.

Four new core tests prove equal-serial foreign publication refusal, foreign
removal refusal with wrong-ID precedence, non-owning stale-domain/new-registry
refusal, and poisoned-lock precedence. Two application tests prove that the
outer composite rejects foreign publish/remove without changing telemetry or
core publication. The complete gate passes 1,659 core tests, 173 format tests,
21 application tests, three symbol tests, and two compile-fail doctests, plus
formatting, diff checks, and warning-denying workspace Clippy. The real
`--inventory-bam-sim-resident` probe also exits successfully against the
mounted supplied game corpus; `colourcubes.big` remains the one known absent
archive in that data set.

| Source | SHA-256 |
|---|---|
| `simulation_resident_registry.rs` | `C056C813DDCBE7A5443332FA957FDDF98AA3CF603E1D22D36511E8FA4EE4D1CA` |
| `dmo_hull_resident_probe.rs` | `4C08F61AB0BF362616877F94645EDE17D460AC9A5E9395E14BA34A7161743B1B` |

The next boundary is deliberately later than the subsystem-port call. The
safe-host callback sequence invokes that port after acquiring the Simulation
resident hold, modeling the recovered native void-notification chronology.
The port may perform the proven route selection and retain or enqueue an
unbound request, callback, or front marker. Forbidding lookup, snapshot, and
binding construction there is host policy. The proposed safe-host binding
acquisition belongs at the front corresponding to native `GetRegAsset` at
`0x8277ea08`. Deferred setup should acquire at the boundary corresponding to
its proven Setup lookup and re-resolve at Finish; DReg should re-resolve before
its removal effects. The current marker FIFO takes a completed binding at
construction, so a private pending/front owner should delay that construction
without repeating the resident Up or exposing independently pairable
authority.

## Lazy exact-main-bank binding acquisition

Core commit `cc912f8` adds a private effect-free acquisition owner inside the
main-bank binding subtree. Construction retains only the paired non-owning
lookup and internally fixes `DMO_BANK_COLLECTION_ID`; explicit resolution
orders `lookup -> exact occurrence snapshot -> sealed binding`.

Failures expose only sanitized classifications. Lookup/missing failures may
retry the same lookup domain. Once snapshot or binding authority exists, retry
reuses that exact opaque occurrence/snapshot-derived state and cannot select a
same-ID reload. Success yields one non-Clone/non-Debug owner containing lookup
and binding, with no parts getter or public re-export.

Fresh revalidation resolves the fixed ID, compares registry identity and the
domain-qualified token, re-snapshots wrapper-time count/order/full IDs/outer
allocations, and compares fresh owners to the retained batch by ordered
`Arc::ptr_eq`. It builds no second batch or prepared collection. This is a
point-in-time safe-host pre-effect check, not a resident hold or native token,
pointer, lifetime, or retry contract.

Seven tests cover lazy detached/missing resolution, success, same-ID reload,
same-occurrence append, foreign domain, exact order and outer allocation with
an identical inner plan, and binding retry that cannot hop to a valid reload.
The full workspace gate passes 1,666 core, 173 format, 21 application, and
three symbol tests plus two compile-fail doctests, formatting, diff checks, and
warning-denying Clippy.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_collection_binding.rs` | `9CBAA98439B6FC3EF19118F291903BB7F0D2F5923FFB4175F67F0684336A4667` |
| `simulation_dmo_bank_model_registration_batch.rs` | `A66C33F7A5756E0BBC72F550989C72BD511FDEECA017B628D932804B1EA89AE4` |
| `simulation_resident_registry.rs` | `2CCC69D9FE4C0B2AE5D85AF282D5C4A76CD5B8FC5D79D74ED010258F7DAE4959` |

This remains private prerequisite infrastructure. Fallible DMO runtime and
coordinator initialization must next retain all inputs on failure. The marker
FIFO can then gain private Pending/Ready receiver ownership: Immediate resolves
at its Collection front, deferred resolves at Setup and revalidates at Finish,
and DReg revalidates before its first deregistration effect.

## Ownership-preserving coordinator initialization

Core commit `1186caf` makes production coordinator construction consume the
entire published binding owner instead of its prepared binding alone. The
coordinator therefore retains the exact occurrence and the lookup needed for
later revalidation as one capability. Only isolated `cfg(test)` fixtures may
construct a coordinator from a synthetic prepared binding.

If runtime construction refuses, the returned opaque owner keeps that complete
published owner together with the Normal, Location, and DMO collaborators.
Retry consumes and reuses the same inputs; abandonment is inert. No raw
binding, occurrence, lookup, dispatcher, callback, or partial manager escapes.

Native construction creates model storage at `0x825eb05c` before locator
storage at `0x825eb074` within `0x825eb020..0x825eb094`. Native destruction
releases locator then model state at `0x825eafd4..0x825eafec`. The Rust runtime
keeps these capacities and owner order, while recoverable allocation errors,
retries, deterministic refusal injection, and drop behavior remain explicitly
safe-host contracts rather than native behavior claims.

Tests cover model allocation refusal at 256 entries, repeated locator refusal
at 50 entries, zero dispatch effects, same-input recovery, same-ID reload
rejection, and inert failure abandonment with exact owner liveness. All strict
workspace gates pass with 1,668 core tests.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_collection_coordinator.rs` | `7A21F920D0006D84159397C72A55C1A0CFA597A666EC63FB44266BE59E243B8E` |
| `simulation_dmo_bank_collection_binding.rs` | `10788780AD696357C7D9A8FBB99B2C43D8B3670B863A4B6820C35A0450EF01C7` |

This commit still performs no port/FIFO integration. The next owner must keep
the registration callback beside the lazy acquisition and any initialization
failure until the correct Immediate front or deferred Setup boundary is
reached, without another resident Up.

## Immediate-front Pending/Ready owner

Core commit `4bdf4c3` places the first production binding acquisition at the
Immediate Collection marker front. Pending construction reserves and queues
that one marker while retaining the callback, lookup, and coordinator
collaborators, but performs no lookup or registration effect. At dispatch it
resolves exactly once, corresponding to the native `GetRegAsset` call at
`0x8277ea08`; a success proceeds directly through runtime construction and the
Immediate receiver without an artificial second lookup. The pre-existing Up
at `0x82d5c48c..0x82d5c494` is neither owned nor replayable here.

Marker removal remains effect-ordered: 93 model adds, 125 location registers,
and 125 DMO registers complete before the sole front is popped. The successful
Ready value keeps the mature FIFO and next registration producer inseparable,
and production code has no parts getter. An acquisition-stage or receiver
refusal at the reached front is sticky and retains the marker, callback, and
complete stage-specific recovery; once acquired, the exact published owner
remains inseparable. Later same-ID publication cannot make it hop. Rust recoverable runtime
allocation is the explicitly non-native exception. It retries initialization
first from the retained owner, does no lookup while allocation still refuses,
and performs one fresh exact-occurrence revalidation immediately before the
effect after recovery.

Tests discriminate these placements by attached-registry lookup counts. The
lazy/current-winner path observes `0 -> 1`; a repeated allocation refusal
remains at `1`; successful allocation recovery followed by a same-ID reload
revalidation reaches `2` and becomes sticky with zero effects. A missing
actual-front acquisition remains missing after later publication and registry
detach. The unchanged recovery path emits exactly 343 effects and pops once.
The counter and deterministic failure staging are `cfg(test)` only.

The workspace gate passes 1,672 core, 173 format, 21 application, and three
symbol tests plus two compile-fail doctests, formatting, diff checks, and
warning-denying Clippy. The checkpoint hashes are:

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_marker_fifo.rs` | `ED354D6F98547AA3144374ED892764143707F6B773E965601CABB6C6E3833BA0` |
| `simulation_dmo_bank_collection_binding.rs` | `C75D60D7657F8CAF6692F2F00BF13CC1AD58FEE58F18A0E1813CD71CD92D1DE8` |
| `simulation_dmo_bank_notification_receiver.rs` | `4B6908A359FEB6E3CAC3BD6E9968129E087073CB033A32AE07C880539FF286EB` |
| `simulation_resident_registry.rs` | `056EA87C85AF43F01BBEA540FFD769CF1DE25B339FDF84DDF358690B88C822D8` |

The implementation is still private core infrastructure. It does not yet
claim subsystem-port/app wiring, deferred Setup at `0x82d5b32c`, Finish/DReg
revalidation, the per-Arena lookup at `0x8277e7fc`, live route controls,
first-null construction, or global queue cadence/capacity/interleaving.

## Deferred Setup Pending/Ready owner

Core commit `32fa1d9` implements the separately proven deferred Setup boundary.
Before Setup, the sealed owner retains the registration callback,
Normal/Location/DMO collaborators, and lazy fixed-ID acquisition only. It owns
no FIFO, domain, marker, producer permit, binding, runtime, or deferred
continuation and performs no lookup or child effect.

At `try_setup`, one lookup corresponds to native `GetRegAsset` at
`0x82d5b32c`. The native producer has already copied and erased the first
pending row at `0x82d5b2d4..0x82d5b2fc` and unlocked at
`0x82d5b304..0x82d5b308`. The safe host consequently makes an acquisition
refusal sticky; this typed fault and retained ownership are not native error
behavior. Success initializes the coordinator and begins only the effect-free
deferred continuation, then mints an empty mature FIFO and first Arena producer
inside one opaque Ready owner.

Safe-host runtime allocation recovery retries the retained initializer before
revalidation. Safe-host deferred-begin recovery freshly revalidates before
each retry. Initial synchronous success performs no redundant second lookup;
post-delay success checks the exact retained occurrence immediately before
begin, and a mismatch becomes sticky without model, Location, or DMO effects.

Six tests freeze lookup counts and authority timing: construction is zero,
initial Setup is one, a repeated runtime refusal remains one, and successful
runtime or begin recovery revalidation reaches two. Same-ID replacement is
rejected at either delayed boundary; missing Setup remains sticky after later
publication. Ready starts with an empty queue, ordinal zero, the matching weak
producer domain, and no registered model/effect.

Setup is the final substantive call of native `UpdateSimCollectionDefer` at
`0x82d5b5c4`; the new row's first Arena therefore belongs to a later update.
The old row's terminal Collection/Callback were already sent at `0x82d5b580`
and `0x82d5b5bc`, but evidence does not establish that the global queue drained
before next-row Setup.

The workspace gate passes 1,678 core, 173 format, 21 application, and three
symbol tests plus two compile-fail doctests, formatting, diff checks, and
warning-denying Clippy. Checkpoint hashes are:

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_marker_fifo.rs` | `0C2B5F78F8CFCECAADFCEA5881B62320B3847CD5E02CF930E72C8C9577D2F4BD` |
| `simulation_dmo_bank_collection_binding.rs` | `73E85F9BB40A6D7149A9C4E78E039DC685C423C6557D42C826E23F43F77159F8` |
| `simulation_dmo_bank_notification_receiver.rs` | `59A16C5357795DC3BFC9C8001AAD016C0116B36D2F70B6BA9B40CE9F0BB07535` |

Still open: subsystem-port/application wiring, live route controls and
first-null construction, per-Arena receiver lookup at `0x8277e7fc`, mature
Finish revalidation at `0x8277ea08`, DReg revalidation at `0x8277ec38`, and
global EPostman capacity/cadence/interleaving.

## Terminal collection-front revalidation

Core commit `cd68453` adds fresh safe-host published-occurrence checks inside
the mature receiver at the exact terminal Finish and DRegCollection calls.
The host wrong-stage guard remains first and lookup-free. At a valid front,
revalidation completes before receiver-state extraction, controller completion,
the terminal DMO pass, or any teardown operation.

This placement follows the native receiver order. Terminal
`ReceiveRegCollectionMsg` calls `GetRegAsset` at `0x8277ea08`, reads the
activation byte at `0x8277ea34`, and reaches its first activation-zero DMO
effect at `0x8277eb80`. `ReceiveDRegCollectionMsg` calls `GetRegAsset` at
`0x8277ec38` before DMO teardown `0x8277ecd4`, restarted normal removal
`0x8277ed74`, and Location removal `0x8277edb0`.

The host retains Setup-built dense controller/ledger state, so it rejects a
different same-ID occurrence rather than hopping to it. On refusal, the exact
typed error remains nested in the existing FIFO receiver fault, the front and
callback suffix stay queued, and replay performs no lookup or effect. Native
has no occurrence token, Arc/ordered-owner comparison, typed retry, sticky
fault, or rollback contract; these are explicitly safe-host strengthening.

Five tests freeze the full collection-level lookup chronology as
`Setup 1 -> Finish 2 -> DReg 3`, with no collection revalidation during 125
Arena fronts, callbacks, or DReg staging. Finish reload preserves the completed
218-effect Arena prefix and blocks the terminal 125-DMO suffix. Deferred- and
Immediate-origin DReg reloads block all 343 teardown effects while retaining
one live registration. Replays after detach keep the stored error and lookup
count. Published wrong-stage probes prove stage refusal precedes lookup and
state mutation.

The workspace gate passes 1,683 core, 173 format, 21 application, and three
symbol tests plus two compile-fail doctests, formatting, diff checks, and
warning-denying Clippy. Checkpoint hashes are:

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_marker_fifo.rs` | `0D433FBC5CD00C94B8418A38465B2DC2CE05065BD66A235BE9E4735411D8BF2A` |
| `simulation_dmo_bank_collection_binding.rs` | `260FAEC9E6BFE5A638D32CE6CA45002AEE8FBE2047CAE090A4A2042A52A48AEF` |
| `simulation_dmo_bank_notification_receiver.rs` | `40FBB02FFE5888525E0DE0EAB328ECA6B20EFB55D79268B94E9736EA5BA012E2` |

The independent per-Arena receiver lookup at `0x8277e7fc` remains open and
must not be inferred from the Setup/Finish collection checks. Live producer
controls, first-null construction, port/app wiring, and global EPostman
capacity/cadence/interleaving also remain outside this checkpoint.

## Per-Arena receiver-front resolution

Core commit `87174d7` closes that independent lookup without treating the
Setup snapshot as a receiver-time substitute. Each queued deferred Arena
marker carries its authored ordinal into the receiver. Wrong-stage refusal is
first and lookup-free. A valid front derives the Setup plan's outer full-`u64`
ID for that ordinal and performs exactly one current-winner Arena lookup before
the deferred continuation or any child effect is consumed.

This preserves the native order in `ReceiveRegArenaMsg`: message `+0x10` full
ID load at `0x8277e7c8`, `GetRegAsset` at `0x8277e7fc`, asset-data load at
`0x8277e808`, then the Simulation singleton check at
`0x8277e830..0x8277e838`. Only the non-null branch sets the arena, reads
message `+0x18` userdata at `0x8277e840`, and calls Simulation `RegArena` at
`0x8277e844`; Location `RegArena` at `0x8277e884` follows on either branch.
The host uses its sealed Setup ordinal to represent the authored message ID and
its pinned main-bank route value for userdata; it does not claim native message
storage or a volatile per-message userdata reread.

The current registry winner is temporarily held alive across the unlocked
synchronous effect visitor. Before that visitor, Rust requires both exact
outer asset allocation identity and exact plan identity with the Setup-retained
ledger. On refusal, the untouched deferred continuation returns to the
receiver, and the FIFO stores the typed fault at the exact marker. Replay does
no lookup, pop, callback release, or effect. These checks and recovery behavior
are safe-host policy: native only uses the fresh raw pointer, asserts on
missing/null data, and exposes no occurrence token or recoverable retry.

Lookup-count tests prove zero Arena resolution at Setup and dense enqueue, one
resolution per actual Arena front, and none at Finish, callbacks, DReg staging,
or DRegCollection. A same-plan foreign outer-owner collision is rejected before
the first effect. A separate ordinal-62 missing-winner fixture preserves the
exact first 62-Arena/124-event prefix and latches before the target effect,
distinguishing correct ordinal routing from a hard-coded ordinal-zero lookup.
Both faults replay without a second lookup. The successful collection-level
chronology remains `Setup 1 -> Finish 2 -> DReg 3` independently of the 125
Arena lookups.

The workspace gate passes 1,685 core, 173 format, 21 application, and three
symbol tests plus two compile-fail doctests, formatting, diff checks, and
warning-denying Clippy. Checkpoint hashes are:

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_collection_binding.rs` | `B50C81F02E728B3AFCDC0350AD7FCF6303CFB6C1870BF64CF3E08D1D362EF802` |
| `simulation_dmo_bank_collection_coordinator.rs` | `66806A2352281D5FD19498FDCC0FE41E27B90F72B4D886B173BE891B2209BEAC` |
| `simulation_dmo_bank_marker_fifo.rs` | `406AA85FE2F539050B9732FB4EB942F28C70150A5D349809726F033B8A165B96` |
| `simulation_dmo_bank_model_registration_batch.rs` | `8799EEDF65F8A70B3589C19FE92C669CD78F0AD4AA6F4C62B0E354EAD789D826` |
| `simulation_dmo_bank_notification_receiver.rs` | `86E7CAF1344E38C91092D871CB918B538B32D2CC8103E4609D2C89DC67616B62` |
| `simulation_resident_registry.rs` | `C3F713EB8A1A3D89B5BF1667F6B8798DE171E3B1AC6B6407FC8B6D224E4EBE7D` |

Still open: the live outer producer's active-row batching and control inputs,
upstream processed-vector/null-slot construction, subsystem-port/application
wiring, and global EPostman capacity, cadence, and interleaving. Focused review
of `UpdateSimCollectionDefer` confirms it does not first-null select: Setup
stores the full vector length, and the producer unconditionally dereferences
each chosen slot at `0x82d5b4ac..0x82d5b4b4`. A null is an invalid route, not a
shorter Arena sequence. Per-Arena DReg at `0x8277e8ec` belongs to a distinct
route; the dense main-bank teardown remains the separately recovered
DRegCollection path and does not infer 125 Arena-DReg messages.

## Fixed dense outer-producer cadence

Core commit `5a87edd` adds a private owner for one already-admitted shipped
main-bank occurrence. Its Pending update performs Setup only; because native
Setup is the final substantive call at `0x82d5b5c4`, it neither samples batch
controls nor appends the first Arena. A later Active update samples one
caller-provided control snapshot and applies the recovered total-based batch
formula with this bank's raw stream type zero (`Presentation`).

Native defaults yield ten 12-Arena updates and one five-Arena update. The last
update appends the final five Arena markers, terminal Finish, and
`RegistrationCallback` in exact local order. Enqueue itself performs no
resident lookup and no receiver effect. Rust pre-reserves the complete local
batch before consuming its producer permit; typed refusal/retry and the inert
Terminal state are explicitly safe-host policy, not claims about global
EPostman allocation or draining.

Five focused tests distinguish raw Presentation from Simulation/Texture,
full-total division from remaining-count division, stale from freshly supplied
controls, and exact `10 * 12 + 5` cadence. Refusal tests prove 12-slot ordinary
and seven-slot completing reservations, unchanged ordinals/marker prefixes,
and exact retry. Core reaches 1,690 tests and all strict workspace gates pass.

The native producer never performs a first-null scan: it uses the full vector
length and unconditionally dereferences indexed Arena pointers. Dense 125 is a
validated shipped-corpus safe-host specialization. Still open are the real
pending/active multi-row manager, live cVar source, atomic reentry gate, global
postman scheduling/capacity/interleaving, and subsystem-port/application
dispatch.

## Shared Presentation updater boundary

The companion packet `pres_defer.txt`, SHA-256
`8648F8278D0FA78B18D892BCF9EEA188590F3FD5D63AD447ECB6C9FCF96CFA18`,
proves that Presentation is not an independent deferred manager. Its updater
shares Simulation's gate, active mutex, active 48-byte row, mixed pending
vector, and final Setup call, but advances only `mRegType == 2`. Simulation
advances only type one.

Presentation always sends a signed-positive batch's first Arena without
reading asset data. It then reads each following candidate's unsigned
`cArena::mCost` through `cAsset::mAssetData`; nonzero stops before that
candidate without advancing it. The PDB type-layout packet SHA-256
`4718F3FC499112AD38D4387037B553573FE54BAFE258F764200571E3EF4CE28F`
proves that field identity at `+0x14`. Core commit `4ab8f3b` models only this
pure lazy selection. The later processed-cost packet
`presentation_arena_cost.txt`, SHA-256
`6711C4CD9835EA86C27714AE77AB83D14326566630C4C5EA5D307C925DB694AD`,
proves that `ProcessArena` clears the field and `ProcessInstanceList`
wrapping-increments it per admitted Environment. The prepared main-bank owner
now binds those costs to selection. The later Presentation-specific producer
owns the exact main-bank marker sequence, and its Arena fronts can now be
delivered through the distinct `AddAsset`-first receiver gate; this shared
Simulation-router document does not treat that as a Simulation receiver.
Terminal order is
final Arena, primary activation-zero Collection, optional full-ID proxy
Collection, callback, then Setup; an already-complete row skips the suffix and
goes directly to Setup.

Caller packet `defer_callers.txt`, SHA-256
`6D3BEB8B8E403BFA7C10FD25A90B0FBCAA52919076AD1B61490632363D2D763F`,
places Simulation deferral in `cAssetStreamManager::Update` at `0x82d61364`
after `cAssetStreamer::Update`, and Presentation deferral in non-exiting
`GameModule::Render` at `0x8273216c` immediately before the following
Presentation Dispatch. This does not prove cross-thread/frame order or that
Dispatch drains or acknowledges every queued message.
