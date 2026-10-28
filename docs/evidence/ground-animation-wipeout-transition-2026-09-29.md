# Ground-animation to wipeout transition

Reported: `SystemLogic(Selection(PendingTransition(300)))`, tick 13628.

The existing `select_ground_animation` returns 300 when the processed wipeout
flag is set. `selected_transition` omitted GroundAnimation -> Wipeout, despite
both exit and entry implementations already being present. Its old test even
asserted rejection of this pair. Among the implemented selector states this
was the missing bail route; the error itself does not record its predecessor.

## IDA verification (2026-09-29)

- `CalcSuggestedState` GroundAnimation case `82ddc368`: `rlwinm r11,r9,0,13,13`
  checks mask `0x40000`; `82ddc370` branches to `82ddc104`, which returns
  `0x12c` (300). This precedes the boneless and offboard requests.
- `SetPhysicsState82dff180`: controller notification; timer/last/current/next
  fields; old state's Exit at `82dff218`; select WipeoutGround at `82dff748`;
  new state's Enter at `82dffd1c`.
- `GroundAnimation.Exit82d83ed0`: zero linear drag, restore wheel material,
  and, only if its retained jump info requests a jump, apply the existing
  gravity/timestep and jump-velocity clamp. It does not skip this on bail.
  The existing `try_exit_pair_ground_animation` already implements this path.

The fix admits this explicit predecessor/destination pair and reuses the
existing transition driver, GroundAnimation exit, and WipeoutGround entry.
Updated the existing regression's expectation; no duplicate bail logic or
blanket transition acceptance added.

Validation: scoped Rust parser/format check only.
No Cargo or game launched. Subsequent work also connected the
skitch physics104 flow; see skitch-physics-104-2026-09-29.md for that separate
implementation and its uncompiled/unrun validation status.
