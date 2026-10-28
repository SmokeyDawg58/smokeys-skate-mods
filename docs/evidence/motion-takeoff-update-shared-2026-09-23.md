# Shared Motion Takeoff update reached after SequenceAnim

After SequenceAnim was connected, the authored Ollie facade stopped on the
same tick at `Motion(PendingStateUpdate(2190422384))`, or `0x828f3170`.
The log in `artifacts/public-facade-ollie-sequence-2026-09-23.log` contains
the address but not the selected state index.

IDA labels `0x828f3170` as `StateUpdate_Takeoff_1074`, yet generated Motion
states 792, 799, 871, 878, 1074, 1082, 1090 and 1098 all share it. The
native thunk calls `In_565` at `0x828f318c`, then its leaf PlayAnimation
update at `0x828f31a0`. `In_565` expands `Trick_19`, `OnBoard_3` and
`Motion_0`, so the selected callback order is slots 0 through 27 exactly
once. The leaf and inherited callback implementations already exist in Rust.

Guarded dispatch rows were added for exactly those eight state/address pairs
with their generated active behavior IDs. Existing active-set validation
remains before the callback loop. This adds no second PlayAnimation or
state-update implementation. The original-data facade now completes the
authored Ollie sequence with a visual frame on every selected tick in
`artifacts/public-facade-ollie-takeoff-2026-09-23.log`. This test establishes
the reached callback path, not the native leaf identity or pose parity.
