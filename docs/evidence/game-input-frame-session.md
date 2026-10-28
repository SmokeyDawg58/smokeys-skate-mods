# Retained delivery stage for one selected Actor tick

Confidence: high for extraction/order of existing recovered helpers. New file
`game_input_frame_session.rs` is UNREGISTERED, with no Cargo execution claim.
This is a production adapter; it imports no corpus/test type or constructed
replacement service. It accepts mapped `PlayerInputData`, not device input.

## Evidence and scope

Existing evidence `game-input-command-runtime-plan.md` establishes Pack82a8c070
and DoCommand825e5610; `game-input-manager-runtime.md` establishes manager
Update826e2578. Current `game_input_recognizer_corpus::step` composes these in
that order; `LoadedActionInputSession::step_loaded` calls it before observing
the registered listener, then separately invokes Fill and Action. All direct
codec/command call sites were inspected by repository search: outside tests and
the two original-input corpora there is no existing production frame composer.

The adapter calls those same implementations. Sparse input uses at most nine
bytes (mask plus eight fields); unfocused input uses zero bytes. It does not
clamp values, change focus, flip axes or bypass quantization. Missing receiver
remains the native successful dispatch outcome, returned to the caller; Update
still follows as in the existing composition. Pack failure stops before
command dispatch; command failure stops before manager Update; Update errors
retain already committed command, history, broadcaster and listener changes.
No per-frame manager Reset, CRC clear, pattern construction or listener rebinding.

This is the selected delivery/recognition stage, NOT GameModule::Update1 parity.
The earlier native scheduler gates pause/NIS/network eligibility, obtains the
actual simulation mask and advances Simulation RNG before each ordinary command.
Those responsibilities remain at its caller. The existing corpus selects mask1,
receiver10, addToCRC=true and controlled syncLevel2. The production helper accepts
all those policies explicitly and does not manufacture an offline sync service.
It is one sample followed by one manager update; a future multi-command batch
must dispatch its full batch before ONE Update rather than call this per command.

## Exact root mounting instructions

1. Register `game_input_frame_session` in the crate root beside input managers.
2. Replace ONLY the `LoadedActionInputSession` struct definition in
   `game_input_loaded_action_corpus.rs` with:
   `pub(crate) type LoadedActionInputSession = crate::game_input_frame_session::GameInputFrameSession<crate::actor_construction::ActorIdentity>;`
   Keep its existing inherent replay impl. The same-crate concrete alias can
   retain that impl and existing struct-literal constructor field names.
   Alternatively use the fully qualified production struct at the sole return
   literal in `game_input_recognizer_corpus::check`; move both managers unchanged.
3. In `game_input_recognizer_corpus::step`, preserve authored-point to sample
   conversion (including recognizer Y sign), then replace the local pack,
   try_do_command and try_update block with `try_deliver_frame(manager, gameplay,
   GameInputFrameRequest { input: &sample, receiver_id: receiver, update_mask: 1,
   add_to_crc: true }, &mut HostReckoningNumeric, services, runtime, overlay)`.
   Unwrap only in this corpus; retain its pending-route assertion. Remove now
   unused codec/EncodedGameInputCommand imports. This makes existing Ollie and
   loaded Kickflip replays exercise the production composition without changing
   their Action-only scope or observations.
4. Eventual retained tick owns `GameInputFrameSession<ActorIdentity>`, supplied by
   the existing startup move. Call `try_step` once with the actual mapped sample,
   live sync provider and `GameInputRuntimeServices` borrowing the retained cvars,
   Vault, overlay and available local-index capability. Do not call `step_loaded`
   here: it additionally invokes Fill and Action. Observe events if needed, then
   execute the Actor prefix's one Fill/Action followed by Motion/body/physics.
5. Keep startup's actual listener.player pointer identity assertion. Do not
   register a new listener or reconstruct the player/manager just for this type.

No new behavior kernel was introduced. Existing codec, dispatch and manager
tests remain applicable; the original retained Ollie/Kickflip corpus after step3
is the essential composition gate. Root owns Cargo, clippy and integration.
