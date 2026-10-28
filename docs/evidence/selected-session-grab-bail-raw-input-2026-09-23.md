# Selected public-session raw grab and bail probes, 2026-09-23

The new ignored original-data probes live in `selected_session_grab_bail_corpus.rs`, a child of the existing public-facade corpus. They reuse its booted `SelectedLiveSession`, real mounted input map, controller-zero scripted pad, selected command queue, Actor, Motion, physical pair, and completed-frame checks. They do not add an input mapper or gameplay behavior.

## Source packet and branch evidence

- `cXenonPadDevice::MapStateToButtons` `0x82989448`: PPC `lhz` at `0x82989494`, followed by successive bit extracts and stores, places XInput button bits 6/7 in output channels 6/7 (`0x8298954c..0x82989588`). The bytes at `XINPUT_STATE` offsets 6/7 become analog left/right triggers scaled by the source `0x3b808081` constant. Mounted `input.cfg` maps stick-click controls 76/79 to channels 6/7 and trigger controls 80/81 to channels 10/11 (existing `input-value-runtime-plan.md`). Confidence: high for ordinary finite controller packets.
- `ActionGraphInputListener::Fill` `0x825fb4c8`: the PPC region `0x825fbbc0..0x825fbc18` distinguishes a full trigger equal to 1.0 from a trigger greater than zero. Its intent insertion stream at `0x825fcb10..` writes `LeftGroundGrab` only for full left trigger and `LeftAirGrab` for any positive left trigger. The first probe supplies raw left-trigger byte 128 after a real Ollie reaches physical Air, then observes both intents, Motion, and physical state. Confidence: high for source predicate and intent write; runtime branch awaits the corpus run.
- The same Fill at `0x825fbdf4..0x825fbef4` reads `kUseTriggersForExplicitWipeout`, requires both full triggers and both current stick-click bits, then accepts a fresh edge on either trigger or either click. The original selected cvar constructor default is true (`0x82f57230`; `action-input-continuous-fill.md`). The second probe supplies raw button mask `0x00c0` and both trigger bytes 255 after neutral frames, then observes `WipeOutRequest`, Motion, and physical state through subsequent ticks. Confidence: high for the selected branch; loaded startup settings and downstream tick outcome await the corpus run.

No selected test here claims full native grab animation, physical board grab, ragdoll completion, or a deterministic Xenon reference trace. The assertions establish the actual first reached producer boundary; a failing tick will report its stage and the completed trace.

## Mounted corpus result

With `SKATE2_GAME_ROOT` on the original external build, the focused ignored grab test passed. The raw Ollie entered Action52/Motion789 at step9, physical GroundAnimation at step16, and physical Air at step22. A raw left-trigger byte128 on step23 produced Action `LeftAirGrab=1` and no `LeftGroundGrab`; physical state stayed Air. Motion remained state516 with no `WantsLeftAirGrab` intent on that first trigger tick. This proves the input/Action predicate in a complete selected frame; grab Motion selection and animation still need a held sequence and source trace. Log: `artifacts/selected-raw-air-grab-2026-09-23.log`.

The raw bail packet at step8 produced Action `WipeOutRequest=1`, then selected Action state80 `DBLGrab` stopped before a completed frame with `PendingStateUpdate(0x828ed830)`. Motion remained state1170 and physical Ground because this pending Action update precedes them. The revised probe explicitly recognizes this typed boundary, asserts the retained request, and passes; it also permits a later completed frame if that update is recovered. Logs: initial failure `artifacts/selected-raw-explicit-bail-2026-09-23.log`, bounded passing probe `artifacts/selected-raw-explicit-bail-boundary-2026-09-23.log`.

IDA names `0x828ed830` `StateUpdate_DBLGrab_80`: it first calls `StateUpdate_OnGround_19` at `0x828ed84c`, then updates two `CreateConstMGIntent` behaviour instances 31 and 32 in order at `0x828ed860` and `0x828ed874`. `StateUpdate_OnGround_19` (`0x828ed1f0`) calls `StateUpdate_OnBoard_6` and then inherited `CreateMGIntentFromAGIntent` instances 19 through 30 in ascending order. This is the exact next source implementation boundary; it must reuse the existing behaviour/runtime owners and retain ordered failure prefixes. Confidence: high for branch and callback order, medium for end-to-end selected bail until that update and downstream Motion/physics finish.

The guarded Action implementation now checks state80/address `0x828ed830`
and the exact 33 active behavior IDs `0..10, 42..49, 68..79, 118, 148`,
derived from the generated parent chain Action0/OnBoardGeneral1/OnBoard6/
OnGround19/Grabbing45/DBLGrab80. It invokes each existing callback once in
source order. The first completed explicit-bail frame now has Action80,
Motion99 with `WipeOutRequest=1`, and physical WipeoutGround. The next frame
hit Action Pre expression101. IDA jump-table arm `0x82904eb8` lazily reads
conditions45 and44, then the physical/intent alternatives
`(74 && 15 && 16) || (36 && 38 && 39)`; this exact ordered Pre expression
was added without adding a second condition implementation. The replay then
completed 70 frames; Action Wipeout23 starts at step9 and Motion recovery30
first appears at step76 in the longer 120-frame probe. Physical state remains
WipeoutGround through step119. Step120 of a 180-frame exploratory replay
reached CameraMan `PendingActiveAvoidance`, before a completed visual frame.
This is physical wipeout entry and Motion recovery-state selection, not proof
of full physical remount/recovery or animation parity. Logs:
`artifacts/selected-raw-explicit-bail-after-pre101-2026-09-23.log`,
`artifacts/selected-raw-explicit-bail-70-2026-09-23.log`, and
`artifacts/selected-raw-explicit-bail-180-2026-09-23.log`.
The revised ignored original-data regression asserts WipeoutGround step8,
Action Wipeout23 step9, Motion recovery30 step76, and a queued visual on all
120 completed frames; it passes in
`artifacts/selected-raw-explicit-bail-120-asserted-2026-09-23.log`.

## Physical recovery timing and next boundary

Fresh source review confirms Motion state30 is an animation recovery, while
physical `CalcSuggestedState` `0x82ddc558` sends WipeoutGround300 to the
unchanged-current return at `0x82ddd294`. Its earlier priority check reads
Processed `+0x8c4` bit `4` or `+0x8c8` bit `0x80000` and returns
Teleporting702 at `0x82ddbf50`. Physical `WipeoutGround.Update`
`0x82d9aff8` increments its time at `0x82d9b100` and calls `ManageRecovery`
at `0x82d9b318`. `ShouldAutoTeleport` `0x82d8f390` reads the maximum
duration first, or the minimum duration followed by settled time;
`ShouldRequestTeleport` `0x82d8f4e0` arms and counts down the fade before
setting a request. `FillPhysOut` publishes that request at `0x82d84f98`,
which the next Processed input maps to `+0x8c8` bit `0x80000`.

Read-only `skate2-formats` inspection of the mounted Xbox
`data/db/db.big:data\\db\\skater.vlt`, at VLT bytes `0x5f560..0x5f5bf`,
confirms the four `physics_wipeout/default` values: auto fade key
`b565383e5da5382b` = `3f000000` (`0.5s`), maximum auto time key
`74ca33aefd03ca71` = `41a00000` (`20s`), minimum settled time key
`b3cfa69bb284bbaf` = `40200000` (`2.5s`), and minimum auto time key
`823dd222bb311d13` = `40600000` (`3.5s`). Thus 180 replay frames after
step8 are too early to prove an automatic exit, even if the skater settles.
The Rust recovery timer, post-physics rest test, and PhysOut request publisher
already exist. Its state-change coordinator still returns typed
`PendingTransition(702)` for Teleporting, so a later request is an explicit
integration boundary before remount.

The existing ignored bail corpus now records the retained physical recovery
timers and flags at steps 8, 76, and 179, plus changes to auto-pending and
request flags. This observation does not alter its assertions or claim a
completed remount. The amended 180-frame corpus passed in
`artifacts/post-reboot-raw-bail-180-snapshot-2026-09-23.log`. At step179 it
records `time_in_state=2.8666646`, `time_settled=1.7333324`, no auto pending,
and no teleport request. This is consistent with the original timing limits;
the first automatic exit and physical remount remain unproven.
