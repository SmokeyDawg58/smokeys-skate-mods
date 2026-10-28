# RestAndRecover state 30 Update source trace

Read-only independent check of the live IDA `sk82_na_m.xex` database and generated Motion Rust on 2026-09-23. Confidence: **exact** for the native call and slot order; **high** for the current Rust wiring, which was inspected as source rather than exercised in a runtime trace.

`StateGraph::Gen::HCDelegate_MotionGraph_OnBoard::StateUpdate_RestAndRecover_30` at `0x828EE960` calls `StateUpdate_WipeOut_4` (`0x828EE0B8`) at `0x828EE97C`, then loads instance slot 7 from `instances + 0x3C` into `Context::mInstance` (`Context + 8`) and calls the slot's update target at `0x828EE990`. The target is `0x825187C0`, a single `blr`; IDA calls this folded address `Sk8::AI::AIProcess::OnEnd`, but the `PlayWipeoutRecoverAnim` vtable's Update word at `0x823004A4` also points there. Thus the final slot is an update no-op, not an `OnEnd` phase. The same vtable has CreateInstance `0x82BB3DA8`, Begin `0x82CBC4C0`, and End `0x825187C0` at `0x8230049C/A0/A8`.

The WipeOut thunk calls `StateUpdate_Motion_0` (`0x828EDD40`) at `0x828EE0D4`, then dispatches its three own slots in order. Motion dispatches four slots. Each dispatch sets `Context::mInstance` from the corresponding eight-byte `BehaviourInstance` pair before the direct Update call. No ancestor thunk is called again after the recovery slot. The resulting complete order is:

| Packed slot | Global behaviour index | Update callback |
| ---: | ---: | --- |
| 0 | 0 | `CharacterGesture` (`0x828EDD6C`) |
| 1 | 1 | `UpdateTimeSinceTeleport` (`0x828EDD80`) |
| 2 | 2 | `UpdateRidingFakie` (`0x828EDD94`) |
| 3 | 3 | `FakieHeadChannel` (`0x828EDDA8`) |
| 4 | 33 | `AttachIntent` (`0x828EE0E8`) |
| 5 | 34 | `SetScoreAugmentation` (`0x828EE0FC`) |
| 6 | 35 | `CreateAttribute` (`0x828EE110`) |
| 7 | 107 | `PlayWipeoutRecoverAnim` (`0x828EE990`, folded no-op body `0x825187C0`) |

The generated declarations agree: Motion state 0 owns indices 0–3, WipeOut state 4 owns 33–35 and has parent 0, and RestAndRecover state 30 owns 107 and has parent 4 (`generated_motion_load_data.rs`). In `generated_motion_runtime_lifecycle.rs`, `enter` inserts active behaviours in ascending global index and has a null instance branch for `PlayWipeoutRecoverAnim`; `update_behaviours` matches `(30, 0x828E_E960)` to exactly `[0, 1, 2, 3, 33, 34, 35, 107]`, checks this against the active set, and calls `callback` once per position in that order. Its recovery update arm returns `Ok(())` when `begin == false`, consistent with the native no-op. The `begin == true` arm invokes `motion_runtime_play_wipeout_recover_anim`, whose effects are outside this update-only audit.

**Discrepancy found:** none in the state 30 update slot list, call order, or recovery update no-op. This check does not establish full per-tick parity for the seven ancestor callbacks or validate the recovery Begin side effects.
