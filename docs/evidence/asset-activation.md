# Asset activation and global-location loading

## Scope and confidence

This checkpoint reconstructs the exact scheduling and the reachable
global-location filename-load states of `cAssetActivationManager`. It does not
claim the native worker-thread timing, postman timing, or the shared unload
states as implemented. The generic scheduler and successful load-state order
are **exact** from executable/PDB evidence. Checked allocation, callback
validation, and fatal-error propagation are explicit safe-host policies.

Primary executable authorities are:

- `cAssetActivationManager::Update`, `0x82d5d8e0..0x82d5dab4`;
- `cAssetActivationManager::UpdateRequests`,
  `0x82d5d468..0x82d5d8dc`;
- `cAssetActivationManager::IsPendingRequests`,
  `0x82d54870..0x82d548f8`;
- filename `LoadArenaAsset`, `0x82d5bf40..0x82d5bfe8`;
- `FreeActiveRequest`, `0x82d5d400..0x82d5d468`;
- `AddAsset`, `0x82d5bd80..0x82d5be28`, and `RemAsset`,
  `0x82d5a0d0..0x82d5a184`;
- load-success/failure and registration callbacks at `0x82d548f8`,
  `0x82d54920`, and `0x82d54970`;
- `cSimAssetLoader::AssetAct`, `0x82d578c8..0x82d579a4`.

The PDB fixes `tRequest` at `0x118` bytes and `tActiveRequest` at `0x238`
bytes. The manager owns eight inline active records, an active pointer vector,
a queued request vector, and a free pointer vector. Its constructor argument
four reserves queued storage; it is not a four-request semantic limit.

## Exact scheduling

Free slots begin in order `[0,1,2,3,4,5,6,7]`. Admission pops the back, so
fresh assignments are `7,6,...,0`. Retirement appends the freed slot and thus
preserves LIFO reuse.

An admission pass scans the queue forward while fewer than eight requests are
active. An ID already present in the active vector leaves that queued request
in place, but scanning continues, so a later distinct ID may be admitted. The
comparison is the complete `u64`. Accepted requests are stably removed and the
shifted successor is inspected at the same index.

The active vector is also scanned forward. Retired entries are stably removed.
Public `Update` repeats admission and the complete scan only when at least one
request retired. A callback state write by itself does not cause another pass.
This detail is important for synchronous host backends: `4 -> 5` and `6 -> 7`
remain visible until later public updates unless another retirement causes the
native outer loop to repeat.

## Global-location load path

The filename overload hashes the original case-sensitive path before slash
conversion. The successful state path is exactly:

```text
0 Load
  AddAsset; on first use store 4 and begin Arena load
4 LoadingArenaAsset
  wait for callback
5 LoadingArenaAssetComplete
  store 6 and begin Simulation registration
6 ActArenaAsset
  wait for callback
7 ActArenaAssetComplete
  AssetLoaded, optional request callback, AssetAdded, retire
```

An already retained asset completes directly from state zero after `AddAsset`
increments the reference: optional request callback, `AssetAdded`, retire. It
does not repeat Arena load, registration, or `AssetLoaded`. Load failure writes
raw state three; the next scan performs `RemAsset` and `AssetMissing`.
Global-location requests have a null per-request callback, so the narrow Rust
owner omits that collaborator.

Native callbacks retain fixed inline-record addresses and unconditionally
store their next raw state. Rust completion handles instead retain a unique
per-admission `Arc` signal. They are one-shot and phase-typed; retirement marks
the signal dead, preventing an old callback from affecting a reused slot. The
stronger checked transition and `SeqCst` atomics are host safety choices, not
claims about native callback validation or identical CPU memory ordering.

The native failure path reaches a debug-fatal `AssetMissing`. The Rust owner
preserves `RemAsset` and `AssetMissing`, then latches one replayable error and
stops with the failed slot retained. This deterministic abort policy is an
explicit host deviation; continued execution after the native debug break is
not treated as a recoverable contract.

## Shipped corpus

The mounted application enumerates named direct
`data\content\global_locators\*.rx2` entries in mounted archive/table order.
That order, case-insensitive extension matching, and synchronous mounted
read/completion are host policies; native wildcard case rules, multi-device
directory ordering, and worker interleaving are not claimed. Within the fresh
corpus manager, each unseen full-path ID is queued before its unloaded record
is inserted, matching the recovered successful manager order. Persistent
cross-call manager reuse is not represented by this diagnostic corpus owner.

The optimized supplied-data route sends the real 18 entries through the
eight-slot owner, arena decode, Location plan, registration adapter, and final
loaded record update. It produces:

- 18 files and 18 Location containers;
- 744 locations and 2,419 sublocations;
- 744 registered Location entries and 18 live registrations;
- 18 `AssetLoaded` allocations;
- zero ordinary Simulation operations;
- empty queued and active ownership at completion.

Seven concrete state-machine tests distinguish asynchronous and synchronous
callback cadence, eight-slot admission, duplicate bypass, retained-use refill,
load failure, full path hashing, and checked path hazards. Five independent
generic scheduler tests freeze the corresponding queue/active behavior. The
complete workspace gate passes 1,755 core tests, 173 format tests, 25
application tests, three symbol tests, and two compile-fail doctests.

## Evidence hashes

- `global_location_activation_state_machine_design.md`:
  `15051725FA3170E2078226648E2DF68C598A1AC9C2A8E44C35EF41D854A7EDBF`;
- `global_locations.txt`:
  `8E23F6863CDB4BD82468B7BE9B66FF588C904BB554DC22F08E7A657B219A4731`;
- `asset_activation_types.txt`:
  `447D390C34E0911758845E1E1E144269476A6E34F5E26C2D214435A86E6495B5`.

## Remaining boundary

States 1, 2, and 8 through 14 belong to the shared activation manager but are
outside the current global filename-load owner. The complete unload,
header-first load, garbage-stamp, and actual asynchronous simulation-message
backends remain separate evidence/implementation checkpoints.
