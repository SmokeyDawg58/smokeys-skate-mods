# Motion expression 411, frame 13 scripted raw pad

Confidence: **exact** for the native evaluator branches and operand order; **high** for their match to the loaded graph declaration. This packet proposes only expression 411. It does not claim Kickflip selection or a completed frame 13.

## Loaded graph and condition owners

`generated_motion_load_data.rs` declares state 1148, `Grabbing.Into`, active, parent 995 (`Grabbing`), expression 411, one behaviour at 2138 and transitions starting at 1480. Conditions 268, 271, and 269 are three separate `HasIntent` objects. Their sole serialized attributes are respectively `intent=BoardAdjustDown` (attribute 2616), `intent=NoseGrab` (2618), and `intent=RocketAir` (2617). In particular, the native expression order is **268, 271, 269**, despite the numeric ordering of the condition declarations.

## Debug executable, exact branches

IDA debug build `sk82_na_m.xex`:

- `HCDelegate_MotionGraph_OnBoard::EvaluateExpression<1>` at `0x8290aa50` dispatches expression 411 through `jpt_8290AA8C[411] = 0x8291a360` (table base `0x8290aa90`). At `0x8291a378..0x8291a3c0` it reads the shared cached condition 268 and returns false when that is false. If true, `0x8291a3d4..0x8291a418` reads condition 271. If 271 is false, `0x8291a42c..0x8291a474` reads condition 269. A true result from 271 or 269 reaches `0x8291a478` and returns true. The false branch at `0x82910254` returns zero. The typed expression is `c(268) && (c(271) || c(269))`, with lazy operands.
- The three direct calls at `0x8291a384`, `0x8291a3e0`, and `0x8291a438` invoke `StateConditionDirect_HasIntent` with `sConditions` slots `0x430/4 = 268`, `0x43c/4 = 271`, and `0x434/4 = 269`. Each operand checks its own active bit in controller word `+0x20`, obtains a cached value in `+0x54` on a hit, or calls then publishes both active and result bits on a miss. The first and second result gates precede the next operand. The direct wrapper at `0x82cc2908` assigns the passed state as the condition's parent, then calls `HasIntent::ActivationFunc`.
- `EvaluateExpression<2>` at `0x8291e218` dispatches `jpt_8291E250[411] = 0x829221c8` (table base `0x8291e254`). `0x829221c8` returns one and reads no condition. Thus Sustain 411 is constant true.
- Native `GetStateActivation<1>` at `0x8292b8b8` obtains the state's expression and calls the Pre evaluator at `0x8292b950`; `GetStateActivation<2>` at `0x8292b808` calls Sustain at `0x8292b8a0`. `GetTransitionActivation<1>` at `0x8292baf8` also calls the Pre evaluator at `0x8292bb94`. The state selector supplies the evaluated state as the condition parent.

## Current Rust owner and proposal

`generated_motion_runtime.rs` already resets the per-update condition active array before selection and evaluates each condition through one shared results cache. It already handles `MotionCondition::HasIntent` using the Actor's retained intent map, and tags the condition parent before dispatch. `generated_motion_runtime_selection.rs` and `_sustain.rs` lack 411 cases, so the frame 13 mounted probe stops at `PendingExpression(411)` before evaluating the first condition. The unregistered patch `artifacts/motion-expression411-pre-sustain-2026-09-23-draft.patch` adds only those two arms and a focused test for lazy order, both Boolean outcomes, error propagation, and constant Sustain. No registered source was edited by this packet.

The native Boolean evaluator has no recoverable `Result` error channel. The Rust test's injected errors verify that the existing runtime propagates a failed live condition read without sampling later operands; they are not a claim about native error values. Root integration and Cargo validation are pending. Replay should establish the next actual frame 13 boundary before expanding any adjacent Grab expressions.
