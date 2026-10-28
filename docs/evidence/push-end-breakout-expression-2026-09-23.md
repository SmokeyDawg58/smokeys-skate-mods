# PushCycle to PushEnd condition and expression (2026-09-23)

The raw A replay reached Motion Pre expression 444 on the PushCycle to PushEnd transitions targeting states 1284/1285. The loaded graph has two transitions with expression 444; it does not imply PushEnd is selected until the expression and source owners evaluate.

## IDA source

- Generated Motion Pre evaluator `0x8290aa50`, switch arm `0x8291d938..0x8291db4c`: lazy OR of condition indices `368, 18, 288, 343, 287, 225`, in that order. The latter five are the same ordered tail already recovered for expression 438: DisablePushBrake, GroundBSGrab, GroundFSGrab, GroundDBLGrab, Coffin. Condition 368 is `BreakOutOfPush` and is sampled only for expression 444's reached branch. The controller's existing condition cache owns repeated reads.
- Generated Motion condition 368 declaration is `BreakOutOfPush`, with no attributes, factory `0x82c8fe48`, vtable `0x822ff75c`, activation `0x82c9baa8`.
- `BreakOutOfPush::ActivationFunc` resolves the same Actor's `IAnimatable` and `ISkaterMotionGraph`, asserting cached interface identity. Virtual `ISkaterMotionGraph+0x34` returns the retained `PushState` (`Sk8::Skater::PushState` size `0x24`); `mOutFactor` is at `+0x0`, `mContinuePush` is the top bit at `+0x20`. Virtual `IAnimatable+0x28` returns the base `Andale::Animation`, whose `GetLength` is virtual `+0x14` and `GetTime` is virtual `+0x18`.
- Raw `0x82c9bc18..0x82c9bc60` calls `GetLength`, multiplies the resulting `f32` by `mOutFactor`, calls `GetTime`, then uses `fcmpu`/`blt` for the threshold. A time strictly below `mOutFactor * length` returns false. Equality and unordered comparisons continue to the `mContinuePush` bit test; true returns false, clear returns true. This preserves the native floating point order and NaN branch.

## Retained implementation

The existing `ActorSkaterGraphState.push[0]` and `push_bits` own the two PushState fields; `PushCycle` and `ComputeRepushDeadline` already publish them. The existing `ActorAnimationConstruction::body_reference_clock` reads the retained base animation tree or transition TO child. Its visibility was widened for this condition; no second animation clock was created. The adapter verifies the published same-Actor animation interface and Motion graph, calls length before time, then performs the native threshold and delayed continue flag check. Missing owners remain explicit errors.

Expression 444 is added to the existing Pre evaluator; the shared condition dispatcher has one `BreakOutOfPush` arm. No generated lifecycle callback was changed by this work.

## Validation and limit

Focused tests check lazy expression read order, the strict threshold, `mContinuePush`, and unordered comparison. The raw A replay after this change is owned by the root integrator. Confidence: **high** for expression order, source field ownership, and arithmetic branch; full hands-on push behavior remains unproven until the replay and later runtime transitions pass.
