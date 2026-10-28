# Selected session input identity corpus (2026-09-23)

Registered test file: `crates/skate2-core/src/session/selected_session_input_identity_corpus.rs`.
The review patch remains at `artifacts/selected-session-input-identity-corpus-20260923.patch`.

The ignored original-data test uses the existing pad `Platform` and the existing
Juice boot test `Platform` with its explicit no-network result. It reads the
external `Juice_m.ini`, constructs one `SelectedSession`, and runs its registered
asset/input/Juice/world prefix before the controlled player constructor. It
then asserts `Arc::ptr_eq` for the parsed `input.cfg` map in bootstrap and
player startup, and for the Juice state in bootstrap, player owners, and the
Actor's retained `ActorInputRuntimeServices`. Because the test lives under the
existing Juice test module, it can inspect the private state Arc without a
production or test-only accessor.

The same test consumes that bootstrap through `try_bind_player`, polls the
existing production device runtime once, enqueues focus plus selected player
input through `try_preupdate_enqueue`, and dispatches the resulting command
batch through the same selected player's `GameInputFrameSession`. It checks two
delivered commands, no missing receiver, and mask 3. It does not claim a full
Actor/physics frame: the registered boot owner still names terrain, camera and
frame services as its next gate, and this test stops at queued command delivery.
No corpus mapper, second device manager or second Juice owner is constructed.

Validation: independent review approved the test, and the original-data run
passes in `artifacts/original-production-input-identity-2026-09-23.log`.
Root then added an assertion that the retained Register request matches the
loader's pending phase; that incremental assertion still needs a focused rerun.
