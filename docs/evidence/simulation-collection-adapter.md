# Active and deferred Simulation collection receivers

## Scope and provenance

`SimulationCollectionRegistrationAdapter` composes the three already bounded
arena adapters in the orders used by the active outer Simulation collection
receiver and by an already-admitted deferred producer. It covers successfully
resolved collection/arena handles whose vector has already been truncated at
the native first null slot. It does not decide whether a collection defers,
model native update batches, implement registry lookup or native null-slot
production, emit callbacks or hull acknowledgements, or claim
render/GPU completion.

Reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- Simulation dispatch packet SHA-256
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`;
- DMO load/state packet SHA-256
  `973DC02215376DA0ADD3893C29F5A0C9E5E6FE25D9BD786C30E6785AD0013B6C`;
- hull deregistration packet SHA-256
  `301DBD5BDE74FCB2D49177DC9069431673E547BA68B99D2D795F2DDDCB4C8F26`;
- independent formats-only outer-oracle source SHA-256
  `490AD3A0848ACED8BEDAFC3F966BDA0BADDCBA51F175B7C93738DF30048EE384`;
- independent deferred/activation-zero oracle source SHA-256
  `2DD5FDBA1DD1106024E455F2762469F87BD06D612C4F727BC113B4352CA7C076`.

The external packets, oracle, executable, symbols, and decompiler database
remain outside the repository under `world_probe_temp`.

## Recovered receiver order

`SimThreadMsgMan::ReceiveRegCollectionMsg` at
`0x8277e988..0x8277ebb4` reads the collection ID at message `+0x10`, userdata at
`+0x18`, and the activation byte at `+0x1c`. When activation is nonzero, it
walks the arena vector forward to its first null pointer. For each non-null
arena it calls `Simulation::RegArena` at `0x8277eab8`, then
`cLocationManager::RegArena` at `0x8277eaf4` for that same arena. It restarts
from the vector beginning and calls `Simulation::RegDMOArena` at
`0x8277eb80` once per arena. When activation is zero, the branch at
`0x8277ea34..0x8277ea3c` jumps directly to the restarted DMO pass at
`0x8277eb10`. The normal and Location loop is not entered, but each DMO arena
still receives the message userdata reloaded at `0x8277eb58`. Empty vectors
and a null first slot produce no nested calls.

`ReceiveDRegCollectionMsg` at `0x8277ebb8..0x8277ede4` is unconditional. It
first walks the complete forward arena prefix through
`Simulation::DRegDMOArena` at `0x8277ecd4`, restarts at the beginning, and then
calls `Simulation::DRegArena` at `0x8277ed74` followed by
`cLocationManager::DRegArena` at `0x8277edb0` for each arena. Neither outer
direction reverses arena order. The message's immutable userdata word is
reloaded for each Simulation call; the Rust boundary preserves the observable
value by supplying the same `u32` to every nested Simulation call in one outer
operation and accepts a fresh value for deregistration.

## Deferred producer sequence

`cAssetStreamManager::NotifyCollectionSimReg` at
`0x82d5c468..0x82d5c67c` retains the caller's userdata once. For stream types
zero and one it admits deferral only when the live deferral flag is enabled,
`forceImmediate` is false, and collection size is greater than `0x7d000`.
Other stream types skip collection work but still enqueue their callback.
Admission inputs remain outside this adapter.

`UpdateSimCollectionDefer` at `0x82d5b378..0x82d5b5f0` emits individual arena
messages in authored order. Their receiver calls normal Simulation at
`0x8277e844` and Location at `0x8277e884`. Once its active index reaches the
prefix end, the producer enqueues a collection message carrying activation
zero at `0x82d5b580`, then the separate callback message at `0x82d5b5bc`.
Every individual message and the terminal collection message receives the same
userdata snapshot stored by the notifier. The number emitted in one native
update is now frozen by the separate exact
`simulation-deferred-batch-policy.md` boundary; this adapter remains agnostic
to update grouping.

Flattened manager effects are therefore exactly normal, Location per arena,
then a restarted complete DMO pass. The host API exposes one explicit step per
individual arena message, but callers may group those steps into any update
cadence. `finish_deferred` is only the activation-zero DMO receiver returning;
it neither sends nor delivers the later callback.

The native calls return `void`, expose no rollback channel, and stop rather
than skip at the first null arena slot. A non-null collection asset containing
a null arena pointer enters an assertion/debug-break path. The readable plan
therefore accepts only the already validated dense prefix and deliberately
does not fabricate native lookup or null-slot behavior.

## Ownership and safe-host policy

`PreparedSimulationCollection` derives and prepares normal, Location, and DMO
views from each same `Arc<SimulationNormalArenaPlan>` before any manager call.
The outer adapter owns the three child ports before its live owner ledger and
retains one collection owner plus private child authorities until explicit
teardown. Its collection token is non-cloneable and domain-bound. Foreign,
unknown, and already-faulted deregistration attempts return that token without
effects; `Drop` performs no hidden cleanup.

Deferred registration retains the original userdata and issues a non-cloneable
continuation capability. Each arena step consumes it and reissues it only after
normal and Location both succeed. `finish_deferred` accepts it only after every
arena step and converts it to complete collection authority only after every
DMO call succeeds. A dropped continuation is inert and leaves the reached
prefix retained; it cannot fabricate completion. Completed, incomplete,
actively registering, and deregistering rows remain distinct so diagnostic
counts cannot be mistaken for callback or acknowledgement eligibility.

Nested failures record phase, pass, arena index, and whether rejection occurred
before effects, after entry, or through an unwind. The outer boundary catches
every generic child-pass unwind because a collaborator can panic after effects
but before returning its token. A returned pre-effect child authority is put
back into its exact slot; an unwind's moved authority is conservatively treated
as consumed. Any nested failure permanently faults the outer domain and
preserves the reached prefix without rollback. Checked allocation,
registration IDs, authority tracking, unwind containment, and sticky faults are
safe-host policy rather than claims about native exception handling.

Seventeen focused tests freeze active and deferred successful registration and
deregistration streams, a mid-registration Location fault prefix, a DMO
deregistration fault that prevents entry into the later normal pass, and inert
foreign/unknown/empty-collection behavior. They also cover immutable deferred
userdata, incomplete and extra steps, distinct repeated occurrences, inert
continuation drop, false-completion prevention, a registration
unwind after a wrapper effect, deregistration unwinds before and after a child
effect, same-arena Location suppression, exact restoration of a child token
returned on pre-effect rejection, and exact plan-owner identity/order comparison
that cannot be reduced to equal IDs or values.

## Exact pass-boundary composition for the main DMO bank

Core commit `b180f44` factors the existing public receiver loops into private
pass-boundary operations without changing their successful call order. The
specialized main-bank coordinator uses those same operations to place the
already recovered kind-11 model controller at the only chronology compatible
with the debug-XEX receiver:

- active registration walks all normal/Location arenas with one model-register
  controller, proves that controller complete, then enters the restarted full
  DMO pass;
- deferred registration retains one controller across all individual arena
  messages, proves it complete after arena 125, then delivers the
  activation-zero full DMO pass;
- deregistration completes the full DMO pass first, then walks all
  normal/Location arenas with one fresh model-deregister controller, proves it
  complete, and only then removes the outer live occurrence.

The active ordering follows calls at `0x8277eab8`, `0x8277eaf4`, restart
`0x8277eb10`, and DMO call `0x8277eb80`. The deregistration ordering follows
DMO call `0x8277ecd4`, restart `0x8277ed08`, normal call `0x8277ed74`, and
Location call `0x8277edb0`. The deferred individual receiver calls are at
`0x8277e844` and `0x8277e884`; the later activation-zero collection message
uses the same DMO-only branch at `0x8277ea34..0x8277ea3c`. All walks remain
forward and operate on the already validated dense first-null prefix.

Native Simulation calls reload the same message userdata at each call site.
The coordinator captures and supplies one equal value for each Rust occurrence;
it does not claim native volatile load timing. For this pinned main-bank load,
the value is `0x0a`, derived from the established load chain (asset flag 8 plus
the hull path's bit 1), not from a hardcoded receiver literal.

The split continuation capabilities carry a private adapter domain, monotonic
occurrence identity, captured userdata, and phase. Every later operation
re-resolves the live row and completion additionally proves the required stage
and that all normal, Location, and DMO child slots are empty. Cross-domain or
stale capabilities cannot select another live row. A controlled model refusal
latches the same outer adapter after the reached prefix, returns the exact
controller cause internally, and performs no rollback. These capabilities,
typed failures, completeness checks, sticky faults, and stranded-phase policy
are Rust host-safety mechanisms; the native calls are `void` and do not prove
those objects or failure semantics.

The focused suites now contain 17 collection-adapter tests and 12 sealed
main-bank binding/coordinator tests. They include exact 125-arena/93-model
active, deferred, and deregistration traces; early deferred finish; foreign
authority recovery; controller refusal after an already-applied Add or Remove;
resident expiry; repeated collection occurrences; equal model keys from
distinct source rows; and cross-domain capability rejection. The strict
workspace gate passes 1,631 core tests, 173 format tests, 19 application tests,
three symbol tests, two compile-fail doctests, formatting, diff checks, and
warning-denying Clippy.

| Source | SHA-256 |
|---|---|
| `simulation_collection_adapter.rs` | `63DA541B96525E30822A569A5E5D699167A9DC5BEBCAFBC95EF0F653327D5A22` |
| `simulation_dmo_bank_collection_binding.rs` | `17C6206042FAB05B73476F925D7F6B1A28BFA59F118CCF14D5542E62FDB3C4FA` |
| `simulation_dmo_bank_collection_coordinator.rs` | `89A7C227C2BDFD693CB5A40FF7F01E348B3D323424308630B52905C3DA26E5F4` |

This checkpoint does not select the live active/deferred route, enqueue or
dequeue native messages, invoke the callback, acknowledge the hull, perform a
fresh receiver-time registry lookup, integrate native first-null production,
register locator records, execute `SpawnStatic`, or construct a DMO Body.

## Independent whole-BAM gate

The external formats-only V1 oracle parses XSM/XSF/RW4 data without importing
`skate2-core`. It covers 284 nonempty collections and 2,135 arenas, with one to
31 arenas per collection and 2,135 distinct `(outer asset ID, inner arena ID)`
pairs. It defines the complete outer event streams independently:

- registration: normal, Location per arena, then the restarted DMO pass;
- deregistration: the complete DMO pass, then normal, Location per arena.

Its frozen scope/registration/deregistration/unified signatures are
`13a98da6233ecb1d`, `26dc539be7eeaabc`, `350c8e486be71e16`, and
`04978e44d60002fd`.

The optimized in-repository `--inventory-bam-sim` gate constructs actual
`PreparedSimulationCollection` values and drives the concrete normal,
Location, and DMO arena adapters through the deferred message FIFO. Each direction produces
2,135 calls of every pass type, including empty per-arena child plans. All
6,405 calls per direction match the next oracle identity and ordering record.
All seven frozen signatures match their independent protocols, and sequence
mismatches remain zero. Teardown ends
with no live outer or child registrations, no normal/DMO orphaned source, no
Location entry/source, no published Environment ground volume, and no sticky
fault.

Flattened deferred effects intentionally reproduce the active registration
signature `26dc539be7eeaabc`. Two independent protocols prevent that equality
from hiding an immediate-path substitution: the DMO-only activation-zero
stream has 2,135 DMO calls, zero normal/Location calls, and signature
`42f92d5d50c74ec9`; the staged stream records every individual arena message,
one terminal activation-zero marker, DMO pass, and later callback marker in
producer/receiver order and has signature `8b3a8a23042ff350`. The scope-plus-zero
signature is `b1afe10e126ba608`. The callback marker validates producer order
through callback-node dequeue, not callback invocation. The gate's one-Arena
round-trip cadence is deterministic host probe policy; integration of the
recovered batch count, unrelated-message interleaving, and acknowledgement
timing remain open.

The corpus result establishes exact successful dense-prefix composition for
the supplied data. It does not establish behavior for allocation failure,
native null slots, malformed collection pointers, deferral admission,
recovered batch-count integration, unrelated-producer interleaving,
asynchronous scheduling, or acknowledgement timing.

## Remaining boundaries

The producer-side pending-to-active collection lookup and exact batch count are
now proven. The outer notification router has an additive resident-backed seam
for caller-invoked producer activation and a fresh terminal Collection lookup,
but stateful pending-manager composition remains open. Deferred Arena messages
now freeze their full ID and perform a fresh global receiver-time lookup at the
FIFO front; the early-bound prepared-plan identity check is an explicit
safe-host boundary. Their producer's unchecked null-slot dereference remains
unmodeled. The Collection receiver's separate first-null traversal is also not
reconstructed.
The FIFO callback must separately be connected to the existing hull
acknowledgement coordinator.
An acknowledgement can only mean that this CPU-side receiver returned; it must
not be presented as renderer or GPU completion. Concrete subsystem managers
behind the three child dispatch seams remain later work.
See `simulation-deferred-message-fifo.md` for the exact queue evidence and
safe-host deviations, and `simulation-deferred-batch-policy.md` for activation
and cadence evidence.
