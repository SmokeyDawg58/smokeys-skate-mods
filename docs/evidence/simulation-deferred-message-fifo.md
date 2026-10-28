# Deferred Simulation message FIFO

## Scope and provenance

`DeferredSimulationMessageFifo` models the proven message-class order between
an already-admitted deferred Simulation collection producer and the existing
collection receiver. It preserves separate Arena, activation-zero Collection,
and Callback sends, front-only FIFO execution, and completion authority only
at the callback position. It does not choose the deferred route, reproduce the
global EPostman graph, invoke callbacks, issue hull acknowledgements, or assign
messages to rendered frames.

Primary reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- Simulation dispatch packet SHA-256
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`;
- deferred producer packet SHA-256
  `252943E74DABA53E2C1C6BFF8855386B6090238C10DBFD303AB9D6A2749F56A9`.

The executable, symbols, decompiler database, and extracted packets remain
outside the repository under `world_probe_temp`.

## Native messages and routing

All four relevant messages are `0x20` bytes. Their common Hermes header is
kind at `+0x00`, port at `+0x04`, size at `+0x08`, and message ID at `+0x0c`.
The producer initializes port to zero; `Message::Enqueue` at `0x829832f8`
replaces it with the broadcaster port before copying the message.

| Message | Kind | Payload |
| --- | ---: | --- |
| `cMsgAssetSimRegArena` | `0x401fb250` | `+0x10 u64 ArenaID`, `+0x18 u32 userdata` |
| `cMsgAssetSimRegCollection` | `0xf8e29b80` | `+0x10 u64 CollectionID`, `+0x18 u32 userdata`, `+0x1c u8 activate` |
| `cMsgAssetSimDRegCollection` | `0x84403b90` | `+0x10 u64 CollectionID`, `+0x18 u32 userdata` |
| `cMsgAssetSimCallback` | `0x9eb56f8a` | `+0x10 u64 AssetID`, `+0x18 raw callback pointer` |

`SimThreadMsgMan::SimThreadMsgMan` at `0x82780160` registers these receivers
on queue 2 of `SimThreadBroadcaster`. The name hashes with seed `0xabcdef00`
to `0x1e309919`. The relevant handler constructors are RegArena
`0x8277e0e8`, RegCollection `0x8277e208`, DRegCollection `0x8277e298`, and
Callback `0x8277e3b8`. Table dispatch resolves `(port, kind)` and then visits
every matching handler; this is not reconstructed as a global event bus here.

## Queue and producer order

`EPostman::Create` at `0x82983200` appends a `0x10`-byte event owner plus an
aligned copy of the complete message. Each relevant message therefore consumes
`0x30` bytes of native queue 2's `0x6000`-byte linear buffer. The copy owns its
32 payload bytes but does not retain pointed-to callback state.

`Queue::Pop` at `0x829827b8` calls the handler before advancing the deletion
pointer. `EventSys::Flush(2)` at `0x82982968` repeats until empty.
`ThreadSafeSinglePortBroadcaster::Dispatch` at `0x824eccf8` holds its mutex
across that complete flush. Consequently a native-parity host must drain the
modeled queue to idle within its own exclusive broadcaster boundary; the
public one-front operation is a readable `Queue::Pop` seam, not evidence for
one message per frame.

`SendMsg` at `0x82d56d98` locks, appends one message, and unlocks. Deferred
production at `0x82d5b378` is therefore:

```text
RegArena[0], ..., RegArena[n-1], RegCollection(activate = 0), Callback
```

Each arrow is a separate send, so dispatch or unrelated messages may intervene
between sends. There is no atomic batch. The native deferred producer operates
one active collection occurrence at a time; the Rust type can retain multiple
host sessions, which is a labeled generalization rather than proven producer
behavior.

The callback receiver at `0x8277be10` consumes null as a no-op. Otherwise it
invokes virtual slot 1 with the full ID synchronously before `Queue::Pop`
advances. Rust instead returns a host-owned callback capability plus collection
registration after dequeue. Invocation can therefore occur later than native,
and dropping the generic payload runs `C::drop`; neither is callback-pointer
ownership parity. The callback position is not registration status, a hull
acknowledgement, renderer completion, or GPU completion.

## Readable host boundary

The FIFO enforces non-cloneable producer phases for exactly one Arena append at
a time, then Finish, then Callback. Every append reserves storage before
mutation and returns exact authority/payload on rejection. The queue retains a
private collection continuation and dispatches Arena as normal then Location;
Finish performs only the activation-zero DMO pass. A complete registration can
leave the session only when its Callback node reaches the front.

Native messages store raw IDs and resolve manager state at dispatch. The Rust
FIFO session instead keys a continuation by private session ID, while the
external collection receiver ledger strongly retains already-prepared plans.
It therefore reconstructs successful message-class and manager call ordering,
not message bytes, stale-ID lookup, null registry entries, or native ownership.

Wrong receiver-domain dispatch is the sole retryable receiver rejection and
restores the exact continuation/front. A matching receiver's missing state,
existing fault, nested fault, or unwind permanently faults the FIFO. Later
enqueues return their authorities and callback payloads unchanged. The failed
front and later nodes remain retained. This typed fail-stop behavior,
`catch_unwind`, checked allocation, and unbounded `VecDeque` storage are safe
host policy: native asserts/traps or stalls on a non-returning handler and its
fixed queue has no safe overflow result. Partial nested effects can remain, and
no rollback is invented.

Drop performs no explicit dispatch, callback invocation, acknowledgement, or
registration teardown, but normal Rust destruction still drops owned callback
payloads. Struct hosts declare the FIFO field before its receiver field so it
is destroyed first. Local variables have reverse destruction order and must be
arranged separately. Weak domain identity makes the relationship memory-safe;
the ordering is a semantic lifetime policy.

## Verification

Fifteen focused tests cover exact Arena/Finish/Callback order, null and
late-enqueued callbacks, one-front dispatch, host-generalized global FIFO
ordering, foreign receiver retry, exact foreign producer/finish/callback
ownership return, Normal/Location/DMO partial-effect faults, a pre-faulted
matching receiver, post-fault enqueue rejection, dropped producer authorities,
session-ID exhaustion, and inert receiver teardown.

The optimized `--inventory-bam-sim` gate routes every one of 2,135 Arena
occurrences in 284 real collections through the FIFO, then routes each
activation-zero and Callback node before explicit deregistration. Its
deterministic diagnostic cadence performs one Arena enqueue/dispatch round
trip at a time and enqueues Finish plus Callback before dispatching those two;
that cadence is not a native scheduling claim. All 6,405 registration and
6,405 deregistration manager calls match the independent identities and order,
with zero mismatches and fully quiescent FIFO/adapter/child state.

The frozen signatures remain:

- scope `13a98da6233ecb1d`;
- flattened registration `26dc539be7eeaabc`;
- activation-zero DMO-only `42f92d5d50c74ec9`;
- staged message/effect stream `8b3a8a23042ff350`;
- deregistration `350c8e486be71e16`;
- unified `04978e44d60002fd`;
- scope plus zero-registration `b1afe10e126ba608`.

Deferral admission and the pure exact batch calculation are now separate
evidence-backed boundaries. Open work is the stateful pending/active manager,
producer null-slot behavior, the Collection receiver's first-null traversal,
integration of the batch count into production, the global queue-2 topology,
callback invocation lifetime, and production hull acknowledgement. Fresh
per-Arena receiver-time registry lookup is supplied by the outer resident
notification router, outside this lower FIFO; this FIFO deliberately retains
its prepared compatibility path. The outer router also supplies the additive
resident-backed producer-activation and terminal Collection lookup path. See
`simulation-deferred-batch-policy.md` for the newly recovered cadence.
