# All gestures available (2026-10-12)

## Requested parity exception

Every gesture is always available, a PC quality-of-life
change, including choices originally gated by DLC or career progression. PC only.
This supersedes the unlock filtering introduced earlier today; see
[the original catalog audit](edit-skater-gestures-2026-10-12.md).

## Implementation

`EditSkaterFrontend::new` now fills the available list with every native gesture
ID (0 through 34). All four direction slots share those 35 choices for existing
profiles and new careers. The list is rebuilt whenever the editor opens, so no
save migration or unlocking action is needed.

The native catalog order, labels, preview keys, cycling, equipped slot assignment,
and profile serialization remain the existing paths. The constructor no longer
takes an unlock callback; its two callers are updated. The session still supplies
unlock state to the clothing shop. This exception operates on gesture selection,
without changing the profile's general progression or DLC unlock records.

## Reference and review

The full 35-gesture catalog and IDs were confirmed against live IDA in the preceding
task: `FrontEndState_CaC::SetScreenState` at `0x82684b40`, catalog branch
`0x82685138..0x82685f30`. Its category 14 `IsUnlocked(..., includeDLC=true)` gates
are deliberately bypassed here. The earlier raw reference packet remains at
`.scratch/gesture-catalog-20261012/`.

Both changed Rust files parse with `rustfmt --emit stdout`; scoped diff and both
constructor callers reviewed. No tests added/run, compilation, or gameplay run.
The executable must be rebuilt to include this change. The existing fixture call
was adapted to the constructor signature, with no new assertions.

Backups and this task's diff: `.scratch/all-gestures-20261012/`.
