# Queued device input into the retained selected player tick

## Scope and owners

The patch adds an explicitly admitted queued entry to the existing
`SelectedPlayerTick`. It reuses the same `GameInputFrameSession.commands` for
delivery and `.gameplay` for recognition, followed by the unchanged selected
Actor/physical body. It does not construct another history, dispatcher, map,
player, or physics owner. The immediate controlled entry remains available.

The original-data driver carries `SelectedPlayerStartupOwners` through the
ground fixture. Its original mounted map Arc, retained loader and backend
controller selection feed `SelectedDeviceInputRuntime`, ordinary player sample,
the existing codec, retained offline mailbox, native RNG, and the already
constructed frame session. The same live `PhysicalGameInfo` supplies network and
sync reads. Poll, SIM cache, PRES cache, and SIM consumption are explicit events.
Hardware and remap replies reuse the existing `Platform`/`Remap` test fixtures;
these are controlled test collaborators, not recovered platform boot policy.
The test holds focus and refuses any unexpected missing PauseMan query.

Exact fixture input: `Platform::new` reports success, ordinary pad subtype 0,
device type 1, wired capability, packet number 7, button word `0x0001`, both
triggers zero and all four signed stick axes zero on every poll. Its existing
`Remap` requires handle 42/player 0 and overwrites converted channel 0 with
`0.75`; other converted channels remain zero. Thus this is controlled non-neutral
input. Native conversion `82989448` maps button bit 0 to channel 0. The inspected
original input configuration maps `DPadU`/`GP_UDPad` (ID 84) to that channel
(see `input-value-runtime-plan.md`); `InputPadButton` treats values above `0.5`
as held. The driver supplies held D-pad-up, with the first recognized edge and
later effects determined by the existing pad/map/recognizer/Action state, not by
an injected gesture or an asserted gameplay outcome. It does not exercise
stick flicks, grabs, release edges, or a varied interactive controller trace.

## Native chronology

* `PlayerUI::Update` at `828af9f8`, ordinary payload slice `828afc1c..828afcc0`:
  network before raw device line, focus handling, lazy active-simulation/proxy
  query, then loader-index receiver. The previously reviewed sample/queue bridge
  is reused without another immediate dispatch.
* `SimController::Update1` at `828a74d8..828a7690`: ghost and normal command
  stages precede simulation. The existing queue preserves actual shared visual
  and simulation counters, normal-command RNG advancement and CRC behavior.
  The branch at `828a767c` skips `SimUpdate1` for mask zero; otherwise the mask is
  stored at `828a7680` and passed into the call at `828a7690`.
* `Simulation::SimUpdate1` at `8289f410`: overlay requirement and thread monitor
  acknowledgment precede visual-tick and tick stores (`8289f474`, `8289f488`).
  `GameInputManager::Update` receives the same mask at `8289f4b8`. Later selected
  animation/physics stages include animation 1/2/3 (`8289f618`, `8289f674`,
  `8289f6e4`), physics (`8289f71c`), and World Update1 (`8289f758`, same mask in
  r5). Predictor harvest, StepSelector, locomotion and World Update2 follow at
  `8289f8f0`, `8289f908`, `8289f934`, `8289f970`.

The admitted queued entry consumes the real batch first. Mask zero returns its
actual counts before Actor/physical owner reads or observer calls. Mask three
reaches recognition unchanged, with no repack or second delivery. Visual-only
mask two and other partial masks report the unsupported selected schedule after
retaining the already completed command prefix. An accepted empty offline batch
is valid and yields mask three; it is different from no batch.

This composition starts at the admitted command slice and selected input/Actor
body. The full GameModule load/focus/pause/session-marker gates, native complete
Simulation tick counters, thread monitor, manager phases and whole scheduler
are not claimed. Owner lookups for the admitted selected Actor remain existing
host preconditions. This packet does not fabricate completion of those native
caller stages or install a production window loop.

## Validation

`selected_player_queued_tick_tests.rs` exercises the actual empty mailbox and
accepted-empty-batch distinction and preserves reports for unsupported masks.
The original-data test
`original_selected_device_queue_reaches_same_mounted_player_tick` first checks
the real empty queue against mounted owners (no observer calls or RNG advance),
then executes up to 120 queued ticks. That count is explicit harness policy;
native dt and tuning are unchanged. Immediate budgets remain unchanged (three
ordinary ticks and the existing wipeout budget). Each accepted input asserts
one actual delivery, zero missing receivers, mask three, and one RNG advance.
Existing finite, predictor, state and presentation probes remain in the shared
loop; errors stop at the actual failing frame, without skipping or fallback.

The generator formats the shared patch and `git apply --check` passes against
the current registered sources. The author ran no Cargo commands. Root owns
independent source/code review, compilation and original-data acceptance; the
120-tick result is not asserted successful by this evidence.

Root accepted source/code composition. Gate queued120-2 passes format, strict
Clippy (38.28s), 4675 unit tests and six docs. First gate caught two test-only
lint issues, fixed without behavioral change. Original-data aggregate18/21
(6.44s): new queued mode completes frames0..2 with same owner, one delivery and
one RNG advance per tick, then stops at frame3 Motion Pre expression5. Existing
bail test advances to frame3 expression270; collision test stops on six zero
Skeleton contact impulses. These later boundaries are preserved, not ignored.

Packet: `artifacts/selected-queued-tick-2026-09-22.patch`, generated by
`artifacts/build-selected-queued-tick-2026-09-22.py`. New Rust files are the two
test files named above; registration is included in the patch. The camera/RAIN
agent's separate future ground-corpus entity-initialization argument change
does not overlap these handoff and mode changes.
