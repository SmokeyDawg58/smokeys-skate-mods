# Deferred collection activation and batch policy

## Scope and provenance

`DeferredCollectionBatchControls` reconstructs the pure arena-count decision in
`cAssetStreamManager::GetActiveCollectionRegIndex`. The same extraction also
closes the previously unknown pending-to-active transition immediately before
that calculation. A separate private main-bank producer now applies that
decision to the already-sealed fixed dense 125-Arena Pending/Ready owner. It
does not own the manager's pending vector, read live cVars itself, send through
the global postman, or dispatch a queued message.

Reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- deferred setup/batch extraction SHA-256
  `848A059F6CD02F85E40DCE97FE4160F112D693DE70D634A49F63205C03FBBE3C`;
- Presentation deferred updater extraction SHA-256
  `8648F8278D0FA78B18D892BCF9EEA188590F3FD5D63AD447ECB6C9FCF96CFA18`;
- deferred updater caller extraction SHA-256
  `6D3BEB8B8E403BFA7C10FD25A90B0FBCAA52919076AD1B61490632363D2D763F`;
- PDB type-layout packet SHA-256
  `4718F3FC499112AD38D4387037B553573FE54BAFE258F764200571E3EF4CE28F`;
- processed-Arena-cost packet SHA-256
  `6711C4CD9835EA86C27714AE77AB83D14326566630C4C5EA5D307C925DB694AD`.

The executable, symbols, decompiler database, and extraction packet remain
outside the repository under `world_probe_temp`.

The broader producer/control-flow extraction is `sim_defer.txt`, SHA-256
`252943E74DABA53E2C1C6BFF8855386B6090238C10DBFD303AB9D6A2749F56A9`.

## Pending-to-active transition

The MAP identifies `cAssetStreamManager::SetupNextActiveCollectionReg` at
`0x82d5b268..0x82d5b374`. It runs only when the prior active index is at least
its maximum. Under the pending-vector mutex it copies exactly 48 bytes (six
eight-byte moves) from the first pending `tCollectionReg` to the active record
at manager offset `+0x2a800`, erases that first vector entry, and unlocks.
This proves oldest-pending selection and destructive FIFO removal.

When a row was found, the function calls `cAssetRegistry::GetRegAsset` with
the active record's full collection ID. It takes `RegAsset::mAssetData` at
`+0x0c` as the `cCollection*`, resets the active index to zero, stores that
pointer, and derives maximum index as `(mpEnd - mpBegin) >> 2`. A missing
registered asset becomes null and is then unconditionally dereferenced. There
is no recovery branch in this function. A readable host manager must therefore
retain a typed terminal/invariant failure without claiming native recovery.

This producer-side collection lookup is separate from every deferred Arena
receiver's later full-ID lookup and the activation-zero Collection receiver's
later lookup. The outer notification router now supplies the terminal
Collection re-resolution and each Arena FIFO node performs its own fresh
full-ID global lookup. Both receiver-time seams remain outside this pure
arithmetic module.

## Exact arena-count calculation

The MAP identifies `GetActiveCollectionRegIndex` at
`0x82d54230..0x82d542c4`. It first computes `remaining = mMaxIndex - mIndex`.
It chooses a maximum-tick cVar from raw stream type: one selects Simulation,
zero selects Presentation, and every other value selects Texture. It then
performs an unsigned divide of total `mMaxIndex` by that live value. PPC
`cmpw` instructions clamp the quotient upward to the live minimum-arenas cVar
and clamp that result downward to `remaining` using signed comparisons.

For ordinary nonnegative collection-vector counts the calculation is:

```text
remaining = totalArenaCount - activeIndex
target = max(totalArenaCount / maxTicksForStream, minArenasPerTick)
arenasThisUpdate = min(remaining, target)
```

The division deliberately uses total arena count on every update, not the
shrinking remainder. The Rust boundary preserves unsigned division and signed
PPC clamp comparisons, including their high-bit behavior, while rejecting an
index beyond the maximum because native callers enter only while
`mIndex < mMaxIndex`. Equality returns zero as an explicit quiescent safe-host
extension; native Update does not call the function in that state.

## cVar identities and defaults

The four initializer bodies pass these exact defaults and ranges to the cVar
constructor in group `Stream`:

| MAP identity | Address | Default | Range |
| --- | ---: | ---: | ---: |
| `kMinDeferredArenasPerTick` | `0x830a7adc` | 10 | 1..=100 |
| `kMaxDeferredSimCollectionTicks` | `0x830a7ba0` | 20 | 1..=600 |
| `kMaxDeferredPresCollectionTicks` | `0x830a7b58` | 10 | 1..=600 |
| `kMaxDeferredTextureCollectionTicks` | `0x830a7b2c` | 5 | 1..=600 |

`DeferredCollectionBatchControls::new` eagerly validates all four registered
ranges. This is explicit safe-host input validation; one native call reads only
the selected tick value plus the minimum, then sequences `divwu` before an
explicit logical/unsigned `<= 0` trap check on the selected divisor. With an
immediate of zero that catches zero only; the registered upper bound separately
excludes high-bit values.
`NATIVE_DEFAULT` freezes the retail initializer values, while callers may
provide a freshly sampled valid snapshot when runtime cVar control is wired.

## Fixed main-bank producer integration

Core commit `5a87edd` owns one already-admitted main-bank occurrence through
private Pending, Active, and Terminal states. Pending calls Setup as the final
substantive action of its update and does not sample controls or append an
Arena. Each Active update samples its caller-supplied snapshot exactly once,
uses raw stream type zero (`Presentation`), and appends the resulting authored
ordinal range. With native defaults, 125 Arenas produce ten batches of 12 and
one final batch of five. The completing update appends those five Arenas,
Finish, and `RegistrationCallback` in that order. A Terminal update is an
explicit safe-host no-op and does not sample again.

This boundary only appends to the private FIFO. It performs no Collection or
Arena lookup and no Normal, Location, DMO, or callback effect; those remain at
the separately recovered receiver fronts. Rust pre-reserves a complete local
update before consuming its producer capability. Typed allocation refusal,
exact owner retention, retry, and all-or-nothing local append are safe-host
policy rather than native postman behavior. Tests explicitly prove reservation
sizes 12 for a nonterminal default batch and seven for the completing five
Arenas plus two terminal markers.

Native does not scan or truncate at a null Arena slot. Setup stores the full
vector count and Update advances its index and unconditionally dereferences the
selected pointer. The fixed dense 125 path is therefore a validated shipped-
corpus specialization; upstream null-slot construction remains an invariant
boundary rather than a shorter-prefix policy.

## Presentation following-Arena boundary

`UpdatePresCollectionDefer` at `0x82d5b5f0..0x82d5b8a0` calls the same live
batch helper for a type-two active row. It interprets the returned word as a
signed count: zero or a high-bit value takes the `cmpwi`/`ble` path at
`0x82d5b6a4..0x82d5b6a8` and selects nothing. For a positive count, the Arena
at the current index is always selected and the index is advanced before its
send at `0x82d5b6ac..0x82d5b704`; this first path does not read asset data or
the later Arena cost.

The PDB type packet proves `assman::cAsset` is `0x10` bytes with `mAssetData`
at `+0x0c`, and `cArena` is `0x18` bytes with unsigned `mCost` at `+0x14`.
Only additional quota positions load the candidate `cAsset::mAssetData` and
then `cArena::mCost` (`0x82d5b710..0x82d5b74c`). A zero cost admits and
advances that Arena. Any nonzero cost ends the current invocation before
the candidate is sent or the index advances, leaving the same ordinal to be
the next invocation's unconditional first Arena. The field identity, unsigned
type, and zero/nonzero use are exact. Its producer is now also recovered:
`ProcessArena` clears the field, and `ProcessInstanceList` wrapping-increments
it once for each admitted Environment record. No time or byte unit is implied.
A null data pointer
asserts/debug-breaks and is then dereferenced; the Rust provider error is
explicit safe-host policy.

Core commit `4ab8f3b` implements only this pure, lazy prefix selection. Its
`completed_this_update` result is true only when at least one Arena selected by
that invocation advances exactly to the maximum. Equality on entry is a
quiescent safe-host input with no terminal authority because native jumps
straight to Setup rather than replaying Collection/proxy/callback messages.
The generic selector does not bind a live `cAsset`/`cArena` owner or produce
any message. The prepared main-bank Presentation collection now binds it to
retained per-Arena costs. On the real 125-Arena DMO corpus every cost is zero,
so native defaults reproduce ten batches of 12 and one batch of five. This
owner-bound composition still selects only; it does not append or dispatch a
message.

The Presentation completion sequence recovered in the same packet is final
Arena, primary activation-zero Collection, optional proxy Collection when the
full 64-bit proxy ID differs from `assman::kInvalidCollectionID`, callback,
then Setup. The following fixed-bank owner now stages that local message
sequence; Setup's manager-wide mutation remains external.

## Fixed main-bank Presentation producer integration

`MainDmoPresentationDeferredProducer` now owns one exact prepared main-bank
occurrence through Pending, Active, and Terminal states. Pending performs only
the Setup transition and does not sample controls or stage a marker. Every
Active update samples one caller-supplied control snapshot, invokes the owner-
bound selector, and stages the selected full-ID Arena markers in authored
order. Actual completion stages the primary activation-zero Collection,
optional proxy **DRegCollection**, and callback in that order before entering
Terminal. Terminal replay is an explicit host no-op and samples nothing.

The main DMO bank's full parent ID is
`0xe171e1abfedb69ed`, exactly `assman::kInvalidCollectionID`, so its real path
stages no proxy marker. Native defaults produce 125 Arena markers plus the
primary and callback, 127 total. A non-sentinel test proves the proxy marker is
between primary and callback without treating zero as invalid. Complete-update
pre-reservation and an owning retry after refusal are safe-host policy; the
native producer sends messages individually and exposes no recoverable error.

Arena fronts can now be dequeued only through the checked Presentation
receiver boundary. That path validates the message ordinal/full ID, performs
`AddAsset` first, skips lookup on retained use, and performs fresh resident
Arena lookup plus exact-owner validation before first-use dispatch. A
successful receiver return removes exactly one FIFO front. A post-add failure
retains the front diagnostically while the adapter's sticky fault prevents a
second increment. The queue still invokes no callback and does not consume the
proxy front. Activation zero is accepted only at the FIFO front after all
Arenas, removes only that marker, and performs no registry/refcount/manager
effect. The following callback can then be extracted as an opaque capability;
the real-corpus composition passes that exact capability to the existing hull
acknowledgement boundary, while native callback scheduling remains external.
The receiver packet
`presentation_receiver_fronts.txt`, SHA-256
`0DB5B22BF66436E3DFCC4F42594F21AE934B2AEB4785E24E20CCEB67A7204F14`,
proves the ordering: `ReceiveRegArenaMsg` calls
`AddAsset` before lookup and skips lookup on non-first-use, while
`ReceiveRegCollectionMsg` returns immediately when activation is zero.

The real-corpus gate now drains all 125 Arena markers through the shared
published registry, produces 125 first-use owners and the unchanged 187
registration events, accepts and removes activation zero, extracts the
callback, performs one fresh Collection lookup, and then exercises ordinary
teardown. Five producer tests plus deferred-adapter tests raise the complete
core count to 1,711. The workspace
also passes 173
format, 21 application, three symbol, and two compile-fail doctests,
formatting, diff checks, and warning-denying Clippy. The optimized real-corpus
gate exits zero with the coupled producer/receiver path active.

Both updaters share the gate at manager `+0x2a838`, active mutex at
`+0x2a1d0`, active row at `+0x2a800`, mixed pending vector, and Setup tail.
Simulation processes only row type one; Presentation only type two. The Sim
caller is `cAssetStreamManager::Update` at `0x82d61364`, immediately after
`cAssetStreamer::Update`. The Pres caller is non-exiting
`GameModule::Render` at `0x8273216c`, immediately before the following Pres
queue Dispatch at `0x827321a4`. This proves each local call order, not their
cross-thread/frame order or that Dispatch drains or acknowledges the queue.

## Shared mixed-row manager

`DeferredCollectionRegManager` now owns the manager-wide scheduling state
above the two subsystem-specific producers. Its row owner retains the seven
native admission field classes: full Collection ID, full proxy ID, callback/
owner payload, userdata, registration type, stream size, and raw stream type.
Index, maximum, and Collection owner are absent until Setup, matching native
admission clearing index/maximum and storing a null Collection pointer.

The pending owner has the evidenced fixed capacity 16 from
`fixed_vector_allocator<48,16,8,...>` and preserves mixed insertion order.
The no-proxy combined `NotifyCollectionReg` wrapper calls Simulation at
`0x82d5c8e8` before Presentation at `0x82d5c914`; tests therefore freeze a
Simulation row followed by a Presentation row without claiming those two
separate notifier locks form one atomic insertion.

Both update entry points share one atomic exchange-like gate. A busy caller
returns without active work or Setup and does not clear the holder's gate.
After successful entry, only a matching unfinished row advances. An
unfinished row of the other registration type remains active, but the common
Setup tail is still reached and observes it as incomplete. Setup removes at
most the oldest one pending row, and its fallible host initializer is invoked
after that destructive selection, matching the native lookup order.

The active-record persistence is also exact: when the pending vector is empty,
Setup does not clear a completed record. That record remains installed until a
later Setup finds a pending row and overwrites it. The displaced host payload
is returned so its already-staged subsystem messages may remain undrained while
the next row is active. This returned ownership is host policy, not a native
manager queue. Setup and impossible-progress failures latch their exact owners
instead of inventing native recovery.

The optimized dedicated-DMO composition now routes the actual Presentation
producer through one type-two row in this owner. It carries the catalog's real
stream size plus the pinned full IDs, userdata, and raw Presentation stream
type. Manager Setup performs the producer's Setup-only transition, every later
matching call advances both indices by the same selected count, and the final
five-Arena update leaves the completed active row installed because pending is
empty. Its 125 receiver-time Arena lookups, 187 registration effects,
activation-zero transition, callback handoff, and Collection-lookup teardown
remain unchanged. This is real Presentation coupling only; the exact private
Simulation producer has not yet joined the same runtime manager instance.

## Exact producer handoff compatibility

One core integration test now places the private
`MainDmoBankDenseDeferredProducer` in the first type-one row and the actual
`MainDmoPresentationDeferredProducer` state machine in the following type-two
row of one `DeferredCollectionRegManager`. It freezes both mismatched-updater
no-ops, ten 12-Arena batches plus the final five for each producer, and the
common Setup call that retires Simulation and activates Presentation on the
same entered Simulation update.

The returned Simulation payload remains alive until after Presentation has
also completed. Its queue still contains the complete 125-Arena prefix,
`DeferredFinish`, and registration callback, while the manager retains the
completed Presentation row at index/maximum 125 with its independent
125-Arena, activation-zero, and callback queue. This directly checks the
ownership consequence of the recovered Setup order without claiming that the
two subsystem queues are globally serialized or drained by the manager.

The Simulation unit fixture has the real 125 Arena IDs and exact private
Simulation producer path, but intentionally omits the shipped bank's final
Presentation material/DMO action layout. The test therefore uses the
Presentation module's explicitly test-only prepared-collection constructor;
the optimized original-data probe remains the separate proof that the public
Presentation constructor accepts the shipped corpus and that the production
Presentation manager row reaches receiver/callback/teardown. That unit
checkpoint is a core compatibility proof; the original-data application
wiring is covered independently below.

## Public opaque Simulation producer surface

`MainDmoSimulationDeferredProducer` is now the deliberately narrow production
surface over the exact private dense producer. Its constructor accepts the
published Simulation resident lookup, the existing normal/Environment,
Location, and separate-DMO collaborators, plus the non-cloneable hull
registration callback. Construction remains effect-free. The first update
still performs Setup without sampling controls, each Active update samples
once, and Terminal remains inert.

The public update enum exposes only Setup, queued-Arena count, terminal-Arena
count, and idle classes. An owning failure exposes a sanitized Setup,
caller-controls, progress, or queue class and supports exact retry, while the
private binding, coordinator, marker FIFO, producer permits, and callback never
escape. A focused test proves a failed live controls sample can be inspected
and retried into the first exact 12-Arena batch with no registration effect.
This façade does not expose or skip terminal marker dispatch: the next boundary
is now supplied by the sealed driver below, which delivers all Arena fronts,
Finish, and callback
through the existing receiver FIFO before the real-corpus application can
acknowledge Simulation.

`MainDmoSimulationDeferredRegistrationDriver` now supplies that boundary. Only
a Terminal producer can yield it; earlier-stage conversion returns the exact
unchanged producer. The driver owns the already-staged 127-marker FIFO and maps
private marker identities to a sanitized public Arena ordinal, Finish, or
callback result. Dispatch still calls the mature private receiver and hull
coordinator. Successful callback acknowledgement is reported without exposing
the resulting deregistration-begin permit, which remains sealed inside the
driver.

The focused driver test first proves Active conversion is effect-free and
recoverable. After completing the producer, it dispatches ordinals zero through
124 in order, then deferred Finish, then the registration callback, and finally
observes an idle FIFO. All per-Arena and Finish-time resident revalidation stays
on the existing receiver path, 343 reconstructed Simulation registration
effects are emitted, and acknowledgement authority appears only after the
callback front. The following checkpoint extends the same sealed owner through
deregistration without exposing those capabilities.

## Original-data mixed-manager registration gate

The optimized `--inventory-bam-sim` route now consumes both hull registration
callbacks after the established raw-state-8 resident checkpoint. It puts an
opaque `MainDmoSimulationDeferredProducer` in a type-one row, followed by the
real 125-Arena Presentation producer in a type-two row of the same
`DeferredCollectionRegManager`. Simulation Setup is controls-free, the wrong
type-two updater leaves that active row untouched, and ten 12-Arena advances
plus the final five-Arena advance reach 125. That final update returns the
complete Simulation row while the common Setup tail activates Presentation at
index zero. Presentation independently repeats `10 * 12 + 5` and remains the
completed active row.

The probe intentionally leaves Simulation's 127 marker nodes inside the
retired owner while producing Presentation, then converts the retired owner to
the sealed driver. It dispatches Arena ordinals zero through 124, Finish, and
the Simulation hull callback, followed by one Idle observation. This ordering
is a host probe schedule chosen to stress retained-owner coexistence. It is not
evidence that the native postman drains Simulation after producing all
Presentation messages, nor a claim about native worker or frame timing.

The receiver effects are checked against an expected stream freshly derived
from the published original-data resident plans: each authored kind-11 model
add remains in its Arena-relative position, every Arena performs the separate
Location registration, and Finish performs all 125 separate DMO passes. The
result is exactly 343 Simulation events (93 model adds, 125 Location passes,
125 zero-operation DMO passes). Unexpected grind, Environment, trigger, or
ID-registry routes fault the gate. The independent Presentation checks remain
125 zero-cost Arenas, one material plus 93 DMO actions, and 187 events. The
optimized corpus command exits zero and mechanical cleanup retains no callback
capability.

Simulation deregistration now follows the same sealed owner. The driver
consumes its acknowledgement-produced begin authority together with the real
hull DReg callback, reserves the two-node suffix as safe-host recovery policy,
then stages `DRegCollection` followed by the callback. The collection front
uses the established fresh resident lookup and exact reverse subsystem walk;
the callback front alone performs the resident Down-before-latch operation.
The original-data oracle consequently extends to 686 Simulation events: the
343 registration effects followed by 125 DMO-pass deregistrations, the 93
model removals in exact Arena-relative order, and 125 Location-pass
deregistrations. Presentation then releases the final resident hold. This
proves the lifecycle effects and capability order, but the surrounding cleanup
still does not claim native cross-thread or per-frame timing.

## Verification and remaining boundaries

Eight original pure-policy tests freeze all defaults, typed and raw stream
selection, total-versus-remaining behavior, final partial updates, complete
and invalid progress, all four cVar range boundaries, sampled control changes,
and the unsigned-divide/signed-clamp distinction. Five producer tests freeze
Setup-only timing, one live sample per Active update, raw-Presentation
selection, `10 * 12 + 5`, exact terminal suffix order, Terminal no-op behavior,
and allocation-refusal preservation. The complete workspace gate passes 1,690
core tests. Eight additional Presentation-selector tests freeze the signed
quota gate, unconditional first ordinal, lazy following reads, nonzero stop
without advance, exact provider-error ordinal, raw control routing,
unsigned-bound defense, and actual-progress completion. The complete workspace
gate now passes 1,698 core tests, 173 format tests, 21 application tests, three
symbol tests, and two compile-fail doctests, plus formatting, diff checks, and
warning-denying workspace Clippy.

One owner-bound adapter test adds the retained collection composition, and the
real dedicated-DMO gate checks all 125 costs plus the exact default batch
sequence. The complete workspace gate now passes 1,699 core tests, 173 format
tests, 21 application tests, three symbol tests, and two compile-fail doctests.

Still open are exposing and coupling the shared manager to the exact private
main-bank Simulation producer in the real-corpus composition, an actual live cVar
provider, callback invocation timing, proxy reception, the Presentation singleton-null branch, the unrelated
individual `DRegArena` route, global postman capacity/interleaving, and
frame/update scheduling. Main-bank teardown now performs one fresh Collection
lookup and exact owner-sequence validation before its forward `RemAsset` walk.
Per-Arena and terminal Simulation receiver-time
lookups are implemented at their separate fronts; the Presentation FIFO now
invokes its own registration lookup only through the exact `AddAsset` gate.
The safe-host FIFO does not claim native allocation failure or message-layout
behavior.
