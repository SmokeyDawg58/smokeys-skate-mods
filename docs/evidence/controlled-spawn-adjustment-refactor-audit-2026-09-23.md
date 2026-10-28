# Controlled pre-Actor spawn adjustment: read-only implementation audit (2026-09-23)

This note supplements `controlled-first-skating-spawn-pose-audit-2026-09-23.md`.
No source modules were changed. Addresses below refer to the matched debug XEX
in the live IDA database. `exact` means a directly observed branch, field, or
call; it does not imply a native runtime outcome for this boot.

## Native cvar and command evidence (exact)

`Sk8::SkaterManager::OnCommand` command 3 calls `AdjustSpawnPosition`
(`0x82600998`) before the old Actor is destroyed and `SpawnPlayer` is called.
`AdjustSpawnPosition` (`0x825F1A20..0x825F1BB8`) copies all four matrix columns
first. It calls `INetworkGameQuery::IsNetworkGame` through the Simulation at
`SkaterManager+0x50` (`0x825F1A7C..0x825F1AE4`). When that returns false, it
loads `kbEnableAdjustSpawnPosition.mTarget` and tests the current boolean byte
at target+`0xCD` (`0x825F1AE8..0x825F1AFC`). If both are false, it returns the
copied transform without querying the World. If network game is true, the
boolean is not read and the line path runs. The controlled first test is
explicitly offline; its network answer must be supplied by that retained
session context, not inferred from the cvar default.

IDA disassembly of the cvar initializer at `0x82F569F8..0x82F56A30` passes
section `Physics_Debug`, label `kbEnableAdjustSpawnPosition`, title `Enable
Adjust Spawn Position`, default `1`, and null update callback to
`cVarBoolRef::cVarBoolRef` at `0x82F56A24`. The static initializer pointer is
at `0x82FD1044`, between `kRigTPose` (`0x82FD1040`) and
`kbUseGeneratedStateGraph` (`0x82FD1048`). The selected Rust cvar owner
`input_cvar_runtime.rs` currently contains 342 definitions and does not
register this one. Its `REGISTRATION_ORDER` begins `329, 298, 239, 51, 52,
20, 21, ...`; the narrow addition is a new appended definition index 342
inserted between 52 and 20 in that order (the neighboring native `kRigTPose`
is not selected). `InputCvarReferences::try_current_boolean` reads the linked
target at use time; `SelectedOriginalAssets::try_new` links then deserializes
the actual `data/startup.ini`. The mounted file has no match for this label,
but the source default is not proof of a native runtime sample. Add a named
getter and test unlinked, default true, startup override false, and live
relink/read behavior. A literal true in the facade would bypass the retained
cvar service and hide missing links.

On the enabled path, `0x825F1AA0..0x825F1ACC` constructs vertical offsets
`+0.75`, `-500`, and `+0.15000001` (the latter is bits `0x3E19999A`).
`0x825F1B18..0x825F1B70` queries the World from Simulation+`0xC8` with the
start at authored translation plus `[0, 0.75, 0, 0]`, end at that start
minus `[0, 500, 0, 0]`, zero fatness, and batch label
`SkaterManager::AdjustSpawnPosition`. Raw VMX128 word `0x171FC875` at
`0x825F1B24` decodes `vsubfp128` with source register 63, which the preceding
`vaddfp128` at `0x825F1B1C` just filled with the raised start. Therefore the
end Y is authored Y minus `499.25`, correcting the earlier read-only audit.
On a hit, `0x825F1B84..0x825F1B98` replaces only column 4 with the full
16-byte returned hit position plus `[0, 0.15000001, 0, 0]`; on a miss it
returns the original matrix. Raw `vaddfp128` word `0x16B7B035` at
`0x825F1B94` and the full-vector store at `0x825F1B98` confirm the hit W
lane is carried through rather than replaced by authored W. The selected
Rust `PhysicalAssemblyWorld::try_world_line_test` already
uses `World::LineCollisionTest` (`0x827AB338`, `BF_Default=7`) and returns the
nearest result from its existing ground/DMO/vehicle batches. This audit did
not independently reprove all native child-hit tie cases or whether its
interpolated hit W exactly matches the native query for this terrain; retain
the existing query semantics and test the matrix axes and W explicitly.

## Minimal staged refactor (design, not implemented)

1. In `SelectedLiveSession::try_begin_controlled`, run the existing boot
   prefix, select the authored or explicit host transform, and call the
   existing `SelectedSession::try_select_terrain` on its retained assets and
   `Arc<Mutex<PhysicalAssemblyWorld>>`. Retain the spawn, marquee, flags,
   capsule answer, thread callback, and later camera/frame policies in one
   pending boot state. Do not construct Actor, pair, or presentation yet.
2. Keep the current one-stream/one-residency polling sequence in
   `try_poll_controlled_boot`: `try_poll_stream`, `try_request`, `try_poll`.
   `try_request_after_bounds` already initializes the same World's line query
   owners; `try_poll` registers resident terrain exactly once. Only after that
   returns ready, borrow `ground_pool_for(&world.world)`, lock the same world,
   sample the live cvar after the offline network answer, and call
   `try_world_line_test` with the native start, end, label, and zero fatness.
   Preserve the matrix on a disabled branch or query miss; apply the hit
   offset once on a hit. Return a named boot-stage failure for cvar, terrain,
   world-lock, or line-query errors; do not silently use the authored matrix
   on an error.
3. Feed the resulting matrix to the existing
   `try_continue_controlled_player` (which records `SelectedSession::spawn`),
   then the existing `try_continue_physical` and
   `try_continue_controlled_presentation`; continue with camera/roster, input
   bind, and frame preparation. No second world, terrain stream, player
   constructor, query route, or post-construction teleport is needed.

The async facade requires one deliberate API change: `try_begin_controlled`
currently accepts the owned `thread: T` and borrowed `mapping`, `assertion`,
and `hat_enabled` callbacks, while `try_poll_controlled_boot` accepts none.
Borrowed callbacks cannot be retained through terrain polls. The smallest
clear contract is to retain the owned thread and other plain values in a
`PendingBoot<T>` and pass the three borrowed callbacks to an explicit
completion method once residency is ready (or to each poll call). The
completion method must be monotonic: after a reached source write fails,
retain the completed prefix and report `AlreadyFailed(stage)` on later calls.
Do not call the Actor/pair constructors on every ready poll. Update the
facade's host and original-data corpus callers with this staged signature;
other internal `SelectedSession` users can keep their existing diagnostic
order unless intentionally migrated.

## Focused checks and current limit

The current completed frame-1 trace reports four wheel distances and board
height `0.14285605`, zero contacting wheels, and live `CLEAR_HEIGHT=0.12`;
frame 2 takes the registered Ground-to-Air route. That establishes the
current Rust branch input, not the adjusted-spawn outcome. After the refactor,
an original-data test should assert one retained world and terrain occurrence,
no Actor/pair while terrain is pending, the exact cvar/line inputs and matrix
output on both hit and miss, and one Actor/pair construction after residency.
The first-frame neutral contact trace should then be rerun, including wheel
distances, height, contact count, state, and pose. A native per-tick spawn and
pose trace is still absent, so changed wheel contact or visual posture alone
cannot establish full native parity.
