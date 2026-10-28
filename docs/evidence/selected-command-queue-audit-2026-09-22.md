# Selected command producer/consumer audit

Source/caller audit and approved draft packet following the reviewed ordinary
PlayerUI sample. No full native scheduler or complete SimController constructor
is claimed. Root is sole integrator and Cargo runner.

## Identity and proxy admission

`PlayersData::Reset82561e68` writes controller index0 at82561e98. The signed
getter8255afe8 and setter8255af88 assert unsigned player index<=8, then access
the same row without clamping controller values. Existing settings owner already
retains these nine native rows; add the field there, not to a second backend.
`DoStartSinglePlayerGame8277b070` gets actual FE controller selection at8277b124,
performs existing create/apply sequence, and clamps negative selection to0 only
at8277b1fc..200 before calling the setter atb210. That FE producer is not yet a
retained production owner, so do not invent its selection.

`GameModule::ResetGame8273a680` samples controller getter at8273aab4, then supplies
actual player/controller independently to PlayerUI atab5c. Its proxy is retained
SimController, optionally wrapped by real GhostProxy and RecordedGameProxy per
native global state. Offline/network-player selection and replay flags cannot be
silently normalized to the ordinary proxy.

The actual SimController vtable8215901c maps Tick to folded empty825187c0;
GetNextGenFrame maps to folded82a73568 (`lwz r3,0x14(r3); blr`). This proves the
ordinary proxy's Tick is empty, not that ghost/replay wrappers are empty.

## Queues and mailbox

`SimController::DoCommand828a6470` copies a command into mCommands. Its native
fixed vector has inline capacity10 and permits overflow allocation; 10 is not
a gameplay cap. Existing `GameInputCommandManager::try_do_command` is the later
workspace dispatcher, and cannot be renamed into this queue operation.

`GenerateSyncCommand828a89a0` requires an actual network owner. Valid ghost work
short-circuits SendCommands. Accepted send precedes wrapping frame increment at
828a8a20, mCommands clear at8a28, ghost-to-exec copy at8a38..44, ghost clear8a54,
and validGhost=true8a58. This flag becomes true even for empty ghost commands.
Refusal retains queued commands and frame. Constructor828a4bd8 stores frame1 at
4c24 and empty queues; Reset828a2c68 clears all queues/validGhost, stores frame1,
then calls actual Simulation::Reset before resetting the network.

`ResetNetwork828a21d8` chooses real OfflineNetwork for NT_SP and NT_Ghost; online
game without playback selects OnlineNetwork. Same type calls OnReset; changed
type constructs and replaces the owner. Do not infer this admission from a new
host constant. OfflineNetwork ctor82793618 has empty commands/valid=false;
OnReset8278e0f0 repeats that. SendCommands827a0ef0 rejects when already valid,
otherwise copies then sets valid. GetCommands827a0f70 copies only if valid, then
clears valid. Both ignore their frame argument; Get also ignores visualUpdate.
HasUpdate82784580 returns the retained valid flag. Empty command batches are
valid batches, and this is a one-slot mailbox, not immediate delivery.

## Actual consuming order and missing schedule

`SimController::Update1828a72c8` requires network, resets current update mask,
samples actual pause, dispatches the simulation broadcaster, updates network and
latency, then updates NIS. NT_SP resamples pause after those effects. Only the
active branch proceeds through installer/streamer work to command consumption.
Valid exec ghost commands dispatch with CRC=false at828a7534, then clear and
reset validGhost. Normal GetCommands at75c4 requests tick+1 and current visual
bit; successful receipt sets simulation bit, unlocks global Random, advances
actual simulation RNG once per command at7614, then dispatches CRC=true7648.
Only after all commands does SimUpdate1 run with the resulting mask at7690.
Existing gameplay update/recognizer stage must run once at its proper later
stage, not once per queued command. Failure must retain earlier writes; retry
semantics cannot silently replay the whole batch.

`Update2828a0af8` runs SimUpdate2 only under cached active/nonzero mask, locks
Random, calls actual network OnSimulate if simulation bit set, then unconditionally
Simulation::PostUpdate and Juice logging. These are not part of a command slice.
`GameModule::DoGameUpdate82730418` invokes UI only under actual input-update,
load/network/pause/focus policy, and GenerateSyncCommand only under its source
branches. No generic per-physics-tick send/drain policy is established.

## Reusable random storage

Existing LivingWorld `ManagerRandom` already implements the same native
Random::Reset82a6dba8 and SetSeed82a6db50 six-word storage, including original
seeds. Extract/reuse that implementation rather than duplicate it. Simulation
ctor828a8724 calls Reset; Simulation::Reset8289eee8 calls SetSeed before input
manager reset. Fresh RandValue82a6dbc8 first asserts global !msLocked, then
updates six words with source wrapping sums/carry comparisons, increments the
last word and cascades overflow, and finally increments mIndex. RandValue has
no existing implementation in this repository. GetRandom8289e808 calls it then
performs floating range conversion; the command loop discards the float result.
An admitted command-consumer leaf can preserve exactly the RNG mutation without
claiming a general numeric GetRandom implementation or a full Simulation owner.

Recommended next packet is same-backend controller fields plus shared random
storage and retained offline queue stages feeding the existing workspace
command manager. Full pause/broadcaster/NIS/streamer/ghost/replay/FE scheduling
remains explicit caller admission and must not be replaced by success callbacks.

## Concrete packet and bounded connection

`players_backend_controller.rs` adds actual signed getter/setter to existing
settings. `BackendControllerSelection` captures same loader/controller once at
the native ResetGame boundary; later settings writes do not rewrite this copy.
Negative controller values retain their native bits until the manager's bounded
cache access. There is no fabricated FE selection fallback.

`native_skater_random.rs` extracts the already-used LivingWorld reset/seed
implementation. LivingWorld keeps its alias and unchanged source call ordering;
Actor graph keeps its existing three stored fields and shares only reset seed
values. Audit found no existing RandValue implementation. Shared gate is supplied
as one Arc per application; static msLocked8308c740 is verified0. Reset/SetSeed
never unlock it. Source-derived integer sequence/overflow tests cover the new
advance; this is not a general GetRandom floating-range parity claim.

`selected_sim_command_queue.rs` retains actual copied payloads and the offline
mailbox. Source Command holds256 payload bytes; unused host array tail remains
inaccessible to readers/serialization. Host vectors permit native overflow beyond
inline10 and return typed allocation errors. Queue consumption reaches the same
supplied workspace dispatcher; it never creates a registration map/history.
Consumer failures preserve already-committed RNG/CRC/player/mailbox state and
mark the host stage terminal, explicitly preventing replay of a failed prefix.
This host fault guard is not presented as a native field or native recovery.

Root's counter review identified source globals sSimCounter83058ea0 and
sVisCounter83058ea4, both static0; fresh IDA xrefs show Update1 is their sole
reader/writer. They are now a shared supplied Arc owner, not per-controller
defaults. Visual reset7554 follows exec-ghost clear and precedes validGhost reset;
no-ghost branch increments756c. Simulation reset/increment commits7674 only
after normal command completion/no-batch branch. Tests cover two-controller
sharing, wrap and retained counter prefix on late decode failure.

The actual selected device runtime now has sample→existing codec→queue entry,
using backend controller capture, actual same GameInfo and proxy generation.
Its test creates real reset/poll owners, shares one map with the registered player,
then explicitly sends and consumes into that same command manager. No periodic
poll scheduler or earlier/later PlayerUI stage is inferred.

`GameInputFrameSession::try_deliver_queued_commands` borrows its existing command
manager. A separate `try_recognize_delivered` borrows its existing gameplay owner
and calls the shared Update/Flush leaf without repacking/redispatch. The original
immediate delivery path reuses this recognition leaf. A test uses actual pattern
constructor/reset, same registered players, cvars and recognizer services, checks
CRC/RNG remain unchanged during recognition, and retains exact player Arc identity.
SelectedPlayerTick's original immediate corpus entry is unchanged; production
mounted scheduling must select the queued path rather than calling both entries.

Files are formatted; registration/extraction patch apply-check passed. Author did
not run Cargo. Focused source/code review and execution acceptance belong to root.
