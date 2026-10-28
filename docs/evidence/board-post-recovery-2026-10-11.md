# Board post-physics recovery — 2026-10-11

Report: after24,339 completed frames,
`Tick(After(BoardPost(InitializeTransformsPending(StrangeAcceleration))))`.
The report does not identify which Actor hit the guard. Solo implementation,
live Jan17 rich-symbol IDA read-only. Scratch: `.scratch/board-post-recovery-20261011`.

## Cause and native behavior

The outer board post-physics function still had two explicit fatal placeholders
from the early reconstruction. The board's complete teleport reset had since
been implemented for ordinary player respawns, but these callers were never
connected to it. This report reached the acceleration placeholder.

| Native reference | Recovered branch/order |
| --- | --- |
| `Skateboard.UpdatePostPhysics` `0x82b37d20` | EndWheelLineTest, bad-data predicate, acceleration predicate and animated gate, then the selected branch. Every completed branch clears queued board forces. |
| `IsBadData` `0x82b2b398` | NaN in any XYZ lane of the four live deck Part transform columns. W is ignored; infinity passes. Existing predicate retained. |
| Raw `0x82b37d60..80` | Set mWantsRespawn, InitializeTransforms, TeleportReset with Processed deck transform+0x40, return2. |
| `HasAcceleratedStrangely` `0x82b26070` | Live deck-body velocity Y > live kMinBadVelY, then live kMaxGoodVelY > previous Processed velocity Y. Both strict/ordered. Original defaults25 and5 retained; initializers `0x82f7b028`/`0x82f7b088` checked live. No threshold change. |
| Raw `0x82b37d9c..e18` | Only nonanimated boards recover. InitializeTransforms, TeleportReset with Processed effective transform+0x80, SetVelocity(+0) for all Parts, then deck-body omega XYZ=+0 preserving packed W, return1. |
| `Skateboard.TeleportReset` `0x82b360b8` | Empty AIProcess.OnEnd, another InitializeTransforms, SetEffectiveTransform, Body.TeleportReset, deck/turn/steering/animation/hook dynamics reset, queue clear, target speed0 and jump normal up. Both initializer calls are preserved. |
| `SetEffectiveTransform` `0x82b35ae0` | Live Processed flag0x00100000 flips all lanes of X/Z before Body.SetTransform. |
| `SkateboardBody.TeleportReset` `0x82b2e7f0` | Zero velocities/forces, turn fields, collision reset, grip flags/cosine/vector reset. Existing implementation reused. |
| `SkateboardBody.SetVelocity` `0x82b244d8` | Increasing Part index; each attached body resolved at its store, XYZ replaced with input, packed W retained. |
| `HiLOD.Update4_PostEngineAdjust` `0x82e01c78` | Result1 requests Wipeout22 (BadSkateboard), value0, only when Processed category is not500/offboard. Then Skeleton post-physics, current state post-physics and wipeout checking continue. Existing caller already implements this. |

`try_teleport_reset_board_live` extracts the existing board-only operation
sequence from `try_begin_pair_teleport_player_now`. Ordinary teleporting still
calls that same sequence at its original position. Post-physics recovery calls
only the board reset: it must not run the surrounding HiLOD/Skeleton teleport
resets or change the player physics state itself.

The initializer writes attached Parts6,0..5 and reads board attributes; it does
not mutate Processed. The selected Processed matrix is copied after the first
initializer; the shared reset samples the live board-flip flag after its own
initializer. Owner locks are released before entering the shared reset.

## Connected host scheduling and output

Native recovery retains the current solve's contact records for the subsequent
Skeleton post-physics call. The host's ordinary pre-solve velocity setters
invalidate their solved-frame certificate. Using them here would lead directly
to a ContactSpy FrameMismatch after the reset. The recovery therefore uses the
existing post-solve linear store and a matching post-solve angular store. Both
validate the actual body occurrence, preserve native links/W and retain the
same solved frame. No frame is fabricated or reissued, and no contact or
Skeleton checks are skipped. Transform/zero-motion helpers likewise retain
these body occurrences and do not invalidate the solved frame.

Bad-data recovery leaves mWantsRespawn set through the board reset. The existing
board FillPhysOut tail publishes physics_requested_teleport and clears the
request. Acceleration recovery returns1 to the existing category-gated wipeout
request; it does not set mWantsRespawn. Animated boards take the ordinary path.

Optional `SKATE2_TRACE_PHYSICS_RECOVERY` logs completed exceptional resets,
reason, return code and reset position; it does not alter branch decisions.

## Validation and limits

- `cargo check --release -p skate2-pc -j 1` passed in1m25s (`check1.log`).
- Final `cargo build --release -p skate2-pc -j 1 --timings` passed in12m00s
  (`build1.log`); updated `target/release/skate2-pc.exe` is ready.
- No unit tests added or run. No gameplay diagnostic was run this turn;
  neither exceptional branch has runtime coverage here. The exact
  gameplay replay remains the next check. No compiler/game left running.
- Current report proves the previously fatal guard was reached. It does not
  establish why the solver produced the upward velocity or identify the Actor.
  This change completes the original recovery rather than changing that guard.
- Real save SHA256, confirmed unchanged after the build:
  `F6F2672FCC050D766F1CC179F6FED92863B17063518D00D50D7B7BD1D208F0E5`.
- Confidence: **high** for recovered control flow and existing-owner reuse;
  the raw offsets/call ordering are **exact** observations. Existing host VMX
  numeric limits and unspecified native W lanes in shared helpers remain;
  numerical/instruction-level parity is not established by this change.
