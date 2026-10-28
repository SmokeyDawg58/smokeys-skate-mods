# Collection stream scheduler

This record covers the readable outer `cAssetStreamSystem` scheduler that turns
per-focus collection requests into one globally serialized collection
load/unload operation. It ends at explicit operation and lifecycle
collaborators. It does not implement XSF I/O, arena/resource dispatch, or the
asset manager's registration callbacks.

## Evidence anchors

| Address | Behavior |
|---:|---|
| `0x824d6d80` | busy-activating predicate |
| `0x824d6de0` | busy-deactivating predicate |
| `0x824d6e40` | global collection-active predicate |
| `0x824d7608` | outer and inner pending-request query |
| `0x824dbcc8` | strict-distance `LoadBestCollection` |
| `0x824dbe78` | front `LoadNextCollection` |
| `0x824dbfc8` | FIFO `UnloadNextCollection` |
| `0x824dc208` | loaded-count/activation `LoadComplete` boundary |
| `0x824dc3e8` | delay/deactivation `UnloadComplete` boundary |
| `0x824dd1e0` | complete outer scheduler update |
| `0x824ddce0` | load callback completion enqueue |
| `0x824ddd78` | unload callback completion enqueue |
| `0x82d61318` | inner `cAssetStreamManager::Update` |

PDB `cAssetStreamSystem` is `0x11950` bytes. Its three dynamic load vectors
begin at `+0x58`, unload vectors at `+0x88`, 32-entry non-overflowing load and
unload completion vectors at `+0x11550/+0x11660`, active operation ID at
`+0x118f0`, activating/deactivating IDs at `+0x118f8/+0x11900`, lifecycle
targets at `+0x11908/+0x1190c`, raw defer/FE controls at
`+0x11920/+0x11924`, persistent load delay at `+0x1192c`, signed per-kind
loaded counts at `+0x11930`, and update interlock at `+0x1193c`.

## Outer update order

Raw `mLoading == 1` suppresses an update; other values proceed. After acquiring
the update interlock, the reference:

1. reconciles deactivation and then activation against the processed record's
   zero-extended activation byte;
2. admits at most the front unload completion and then at most the front load
   completion, without scanning past a blocked front;
3. calls the inner manager update once;
4. stops if a collection I/O operation remains active;
5. otherwise samples the live FE control and attempts at most one priority
   branch, falling through to a separately sampled live defer control only when
   the FE pass attempted nothing;
6. dispatches at most one operation outside queue ownership and immediately
   calls the inner manager update a second time only after dispatch;
7. releases the update interlock.

A completion produced by either manager poll appends its full `u64` ID before
clearing the active operation. Since the completion-admission phase has already
passed, that entry cannot be processed until the next outer update. A
completion from the first poll may nevertheless clear the slot soon enough for
a different operation to dispatch in the same update.

Completion capacity is exactly 32 in each direction. Native behavior on entry
33 reaches assertion/allocator territory. Rust returns a typed terminal error
without clearing the active operation and permanently replays the first fault;
that is a high-confidence safe host policy, not recovered native recovery.

## Selection policies

The FE pass runs only for raw `mFECollectionOrder == 1`:

1. unload Texture;
2. unload Presentation;
3. unload Simulation;
4. best Simulation load when its signed loaded count is below one;
5. best Presentation load under the same signed gate;
6. best Texture load under the same signed gate.

If no FE rule is eligible, the regular pass runs. With nonzero defer its order
is Presentation unload, Simulation unload, best Simulation load, best
Presentation load, Texture unload, best Texture load. With zero defer it is
Presentation unload, Simulation unload, Texture unload, next Simulation load,
next Presentation load, then Texture load; Texture uses best selection while
its signed loaded count is below one and next selection otherwise.

Choosing a nonempty priority branch ends selection for that update even when
its helper refuses the candidate. A busy request remains in place and no lower
priority is attempted.

Both load selectors refuse while an unload completion exists, deactivation is
active, or persistent `mLoadDelay` is greater than the live garbage-release
index under an unsigned `u32` comparison. Equality permits loading. Best load
retains the first ordered strict minimum below `1.0e9f`; next load uses the
front stream-order entry. Unload uses only the front FIFO entry and the global
active/busy predicate.

`UnloadComplete` samples the manager garbage-head word into persistent load
delay before later same-update selection. The Rust lifecycle result is
direction-specific so only unload completion can refresh this value. The
broader processed-record counts, activation targets, proxy lookup, and two
registration callbacks remain owned by the result-bearing lifecycle
collaborator.

## Native Rust boundary and confidence

`CollectionStreamScheduler` composes the existing handle-corrected request
queues, `WorldStreamCatalog` metadata authority, one opaque active operation,
two bounded completion FIFOs, signed loaded-count snapshots, persistent delay,
and opaque activation/deactivation progress. Loads receive the exact catalog
path, offset, size, format, asset count, and parent snapshot. Unloads retain the
ID-only manager boundary.

Valid-domain priorities, read timing, guards, FIFO/front behavior, full-ID
comparisons, queue mutation points, manager poll order, and completion timing
are **exact**. Result-bearing collaborators, permanent error latching, checked
completion overflow, and Rust exclusive borrowing in place of the atomic
update lock are **high-confidence safe policies**. Callback thread affinity,
manager failure recovery, allocation exhaustion, and resource dispatch below
the collaborators remain unknown.

The first frozen implementation passed its tests but failed two independent
reviews because it froze load delay, FE order, and defer at update entry. The
corrected implementation stores delay persistently, refreshes it from same-tick
unload completion, samples FE only after the first manager poll/active check,
and samples defer only on regular-pass fallthrough. A subsequent targeted
review rejected an over-broad lifecycle result that could replace the completed
ID with a proxy/group ID; native proxy lookup affects only later notification
arguments. The final result returns only a target count and the scheduler pairs
it with the exact dequeued ID. Two independent frozen reviews then passed the
final source.

Twenty-one focused tests cover all policies and fallthrough, refusal without
fallback, signed/unsigned edges, live read timing, persistent delay in both
directions, one-active ownership, manager poll order, next-update completion
admission, lifecycle equality/missing/front blocking, simultaneous opposite
lifecycle phases, both 32-entry caps, loading/update-lock gates,
`PendingRequests`, and first-fault replay.
