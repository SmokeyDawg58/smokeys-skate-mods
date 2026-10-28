# Physical WipeoutGround 300 to Teleporting 702, selected build

Read-only source packet, 2026-09-23. IDA database: original offline Xbox
`sk82_na_m.xex`. Confidence is **exact** for the listed instructions and virtual
slots, **high** for their composed scheduling, and **unproven** for a completed
selected-session remount. The mounted Xbox `physics_wipeout/default` values are
max auto time 20 s, min auto time 3.5 s, min settled time 2.5 s, and fade 0.5 s;
see `selected-session-grab-bail-raw-input-2026-09-23.md` for keys and corpus
provenance. These are runtime Vault values, not proposed hardcoded tuning.

## Selected request and transition

`PhysState_WipeoutGround::Update` `0x82d9aff8` adds Processed time delta to
`mTimeInState` at `0x82d9b100`, then reaches `ManageRecovery` at `0x82d9b318`
after control, steering, skeleton, and impulse work. `ManageRecovery`
`0x82d8f600` updates reset flags and settled time, calls
`ShouldRequestTeleport` at `0x82d8f6d0`, and sets its retained request byte at
`0x82d8f6e4` on success. `ShouldAutoTeleport` `0x82d8f390` checks max duration
first (`0x82d8f438`), then min duration and settled duration
(`0x82d8f4a8..0x82d8f4cc`). `ShouldRequestTeleport` `0x82d8f4e0` also has
impalement/manual branches; its auto branch sets the Vault fade duration and
pending bit at `0x82d8f5dc..0x82d8f5e0`, then decrements time delta on later
updates (`0x82d8f598..0x82d8f5b0`). WipeoutGround.FillPhysOut
`0x82d84e18`, called through Update5's state slot `0x82e05b74`, publishes
the request. The next processed input stores it as word at `+0x8c8` bit
`0x80000`.

`PhysicalPlayerHiLOD::Update2_SystemLogic` `0x82e01c40` calls
`CheckForStateChange` `0x82e01c54` before its request-ledger clear
`0x82e01c5c`. In `PhysicalPlayerStateChanger::CalcSuggestedState`
`0x82ddbd58`, the global early priority at `0x82ddbf1c..0x82ddbf5c`
chooses Teleporting 702 for Processed `+0x8c4` bit `4` (external player reset)
or `+0x8c8` bit `0x80000` (physics teleport request). This precedes the
WipeoutGround 300 same-state return in the later switch. `CheckForStateChange`
stores HiLOD `framesSinceJump=100` at `0x82e005f8` before calling
`SetPhysicsState` at `0x82e005fc`.

`SetPhysicsState` `0x82dff158` calls the existing SkateboardController
`EnterNewPhysicsState(702)` at `0x82dff1b0`, resets HiLOD state count and
timers and Processed timer at `0x82dff1c0..0x82dff1cc`, writes previous
state/category and next/current at `0x82dff1e8..0x82dff204`, and calls the
old state's virtual Exit at `0x82dff218`. `GetPhysicsCategory(300)` returns
Wipeout category 300 at `0x82d83c40..0x82d83c44`. The intervening Actor ID
queries/category telemetry and `mJPhy_State` update at
`0x82dff250..0x82dff318` still occur before destination selection.
WipeoutGround.Exit `0x82d84d70` calls `Skateboard::SetStandard` at
`0x82d84d88`, then clears `mPreventManualRespawn` at `0x82d84d90`.
`SetPhysicsState` selects the **already constructed** Teleporting object at
`0x82dffca8`, then invokes its virtual Enter at `0x82dffd1c`.

## Teleporting lifecycle and later remount

Teleporting's vtable starts at `0x8234a384`: Update `0x82d86940`, Enter
`0x82d86928`, Exit `0x825187c0` (empty inherited callback),
FillPhysOut `0x82d869f0`, and UpdatePostPhysics `0x825187c0` (empty).
Constructor `0x82d911b8..0x82d91210` retains the Processed input, board,
skeleton, PhysicsSystems members, and PhysOutConditioners, and initializes
the flag word. Enter `0x82d86934` clears the acquired/ready high bits and
sets on-board bit 29. No reset transform is invented on Enter.

On Update, an external-reset request in Processed `+0x8c4` bit `4` copies
its full 4x4 transform, marks data acquired, copies on-board, and marks
teleport ready (`0x82d86950..0x82d869ac`). Otherwise Update delegates to
`Skeleton::TeleportSomewhereAppropriate` at `0x82d869b8`, which delegates
through the actual Actor query at `0x82b3b860..0x82b3b870`. The Actor path
`0x825f0e18` chooses an AI respawn transform or suggested breadcrumb, then
`Actor::Teleport` `0x825ed1c8` publishes the transform/on-board external
reset and flag for a later input pass. These Actor/breadcrumb behaviors have
no equivalent selected Rust implementation yet.

Only if ready, Teleporting.FillPhysOut `0x82d869f0` publishes
`State.isTeleportFinished`, `State.teleportExitState` (Ground 100 if on-board,
OffBoard 500 otherwise), `SkateboardMotion.didTeleport`, and the copied
transform, in that order (`0x82d86a5c..0x82d86ae4`). In the next Update1,
`CheckForTeleport` `0x82e01db8` checks finished and asserts didTeleport before
calling `TeleportPlayerNow` at `0x82e01ebc` from the output transform;
Update1's call is at `0x82e02170`. `TeleportPlayerNow` `0x82e00618` resets
collision/controllers, board, systems, and skeleton, then invokes
`SetPhysicsState(Ground 100)` at `0x82e00994`, followed by board SetStandard
and skeleton SetUpNormal at `0x82e009ac/0x82e009c4`. `ResetSystems`
`0x82dfff20` includes state-changer TeleportReset `0x82dfff90`. That broad
reset is a later, separate fidelity boundary; choosing Motion recovery 30
alone does not perform it.

## Retained Rust owners and first implementation seam

`physical_player_remaining_states_fields.rs::Teleporting` and
`physical_player_remaining_states_simple.rs::Teleporting::construct` retain
the actual 112-byte state allocation, reset transform as uninitialized,
PhysOutConditioners link, and constructor flag values. Its published
`StatePayload::Teleporting` lives in the same
`PhysicalPlayerAssemblyOwner::remaining_states()` collection as the other
constructed state objects; the native constructor slot is `0x82e04928`.
`PhysicalPlayerPhysicsStateOwner::current` is a separate selector and has no
Teleporting variant. `physical_player_ground_to_air.rs` has the existing
selected transition runner and request-clear suffix; it currently admits
neither `(WipeoutGround, 702)` nor WipeoutGround as predecessor. The board
SetStandard path already exists as `PhysicalAssemblyWorld::try_set_board_standard`.
Wipeout fields and request clear already live on the same physical owner.
`physical_player_update1_prefix.rs::check_for_teleport` retains the later
output check and a typed `TeleportPlayerNowPending` boundary. PhysOut already
has teleport-finished, exit-state, did-teleport, and transform storage; there
is no Teleporting.Fill publication yet.

First implementation stage: admit only selected `WipeoutGround 300 ->
Teleporting 702` in that existing runner. Preserve the source's controller,
timer, previous/category/next/current, Wipeout Exit, Actor-query, selection,
Enter, and single success-only ledger-clear order. Validate the published
Teleporting object and same-owner board/skeleton links before using it; use
the existing board SetStandard call. Enter clears the two transient high bits
and sets on-board, leaving the transform untouched. Preserve failed
transition prefixes. A focused selected transition corpus should assert
state 702, Processed last=300/category=300/next=current=702, board standard,
cleared prevent-respawn, Teleporting flags, and exactly one ledger clear.

Subsequent stages: dispatch Teleporting.Update with the actual external-reset
or Actor/breadcrumb branch; publish its ready output through the existing
Update5 state slot; then complete Update1's `TeleportPlayerNow` reset and
Ground/OffBoard remount. None of those later callbacks should be represented
as completed by the first transition/Enter stage.

## Selected implementation checkpoint

The existing `ground_to_air` transition runner now admits only `(300,702)`
from WipeoutGround. It uses the same controller, Processed transition stores,
Actor queries, state selection, and success-only Wipeout ledger clear as the
already mounted transitions. WipeoutGround Exit calls the actual board
`SetStandard` and then clears its retained prevent-respawn flag. Teleporting
Enter uses the previously published `StateKind::Teleporting` object, checks its
same-owner physical and Processed links, sets on-board to 1 and clears
acquired/ready to 0. It does not touch the reset transform. The existing
Actor-query helper writes `mJPhy_State` only on its category telemetry branch;
the `(300,702)` path does not invent an unconditional update.

IDA `Teleporting.GetType` at `0x82d91218` is `li r3, 0x2be; blr`, so the
same-frame Update5 current-type query now returns 702 for the published
Teleporting object. `Teleporting.FillPhysOut` loads flags at `0x82d86a04`,
tests ready bit 31 at `0x82d86a08..10`, and branches directly to its epilogue
at `0x82d86b04` when clear. The selected Update5 state slot therefore has a
validated no-op only while ready is zero. Its ready publication, Update,
teleport reset, and remount remain explicit pending work.

The existing raw controller bail corpus has been extended to 300 frames, with
an assertion for the 300/300/702/702 Processed transition fields, cleared
prevent-respawn, Enter flags 1/0/0, and unwritten reset transform. The mounted
replay passed in `artifacts/post-reboot-raw-bail-300-teleport702-2026-09-23.log`:
automatic fade armed at step226, request published at step256, and 300→702
Enter occurred at step257. That frame then reached a separate Ground.Reckoning
`ForeignState` fallthrough, traced in
`teleporting-update-dispatch-boundary-2026-09-23.md`.
The same corpus samples the retained board `free_wheel_enabled` mode before
and after the transition, requiring true then false (the native
`SetWipeout`/`SetStandard` effects), and counts calls to the actual Wipeout
ledger clear in test builds, requiring exactly one during that frame. The
counter observes the production clear method; it does not participate in
state selection or alter the ledger. Both assertions passed in
`artifacts/post-reboot-raw-bail-300-exit-effects-2026-09-23.log`: free-wheel
mode changed true to false and the Wipeout ledger clear counter 258 to 259.
