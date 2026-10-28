# Kickflip first-contact runout probe, 2026-09-24

The ignored original-data `original_selected_live_facade_kickflip_long_hold_route`
replay uses the authored Kickflip right-stick points at steps 6 and 7, holds
the second point, and releases at step 38. The retained Motion graph reaches
Kickflip Cyc1 state 793 at step 27 and Out2 state 1080 at step 39. At completed
step 42, PhysOut runout is false and the Wipeout request ledger is empty. At
step 43, Air.Post records exactly one request, DeckAcceleration (request 2),
and PhysOut asks for a runout. The next frame reaches source Action31 and
Motion16 HippyRunout. Its OBAir Begin behavior 58 now sets the retained
SkaterAnim landing bit; replay continues to the next typed boundary,
SystemLogic transition 502.

`Wipeout::CheckForDeckAccelerationWipeout` at `0x82dd91a0` is called from
native Air wipeout `0x82de10b8`. Its board-contact gate reads the number of
parts in contact. It projects `CollisionInfo.maxContactVel` through the
Skeleton world-to-animation transform, then requests type 2 if planar speed
exceeds the chosen XZ threshold or absolute local Y exceeds the Y threshold.
The native Air caller selects the thresholds from the original Wipeout Vault
attributes using wheels, trick/jump-frame and contact flags. The Rust path
uses the retained selected board, contact, Skeleton, Processed and Vault
owners; the diagnostic at `physical_player_ground_wipeout_leaves.rs` is
test-only and does not change the comparison.

At this request, the trace reports `parts=1`, world contact velocity
`[0, -3.1606693, 0]`, local vertical magnitude `3.1606693`, planar length
`0`, XZ threshold `0.1000000015`, and Y threshold `0.6000000238`. The local
vertical comparison is therefore the observed request source. The relevant
bit patterns are `0x404a4868 > 0x3f19999a`. This explains the native-branch
decision in the current Rust run. It does **not** establish whether the
contact velocity, board admission, selected Vault branch, or exact timing
matches the Xbox executable on the same input; no per-tick reference trace
has been captured. In particular, this is not evidence for suppressing the
runout to make the demo look cleaner.

Validation: `cargo check -p skate2-pc` passed; `cargo test -p skate2-core
--lib` passed 4,837 nonignored tests. The ignored probe with
`SKATE2_GAME_ROOT` set, `SKATE2_KICKFLIP_HOLD_END_STEP=38`, and
`SKATE2_TRACE_DECK_ACCELERATION=1` reached the described request and then
stopped at typed pending transition 502 after the OBAir mount. The same
pre-transition Processed snapshot has `+0x8cc=0x80100` and
`+0x8d0=0x800`; the 502 selector bit is present, and the offboard
controller's `0xc00` LetGo gate is also true. Its exact same-owner path is
under source review before mounting 502. Confidence: high for the source
branch and current Rust producer chain, medium for matching Xbox numeric
outcome until a native tick trace exists.
