# Motion expression 410, scripted frame 13

Confidence: **exact** for the native evaluator control flow and condition activation; **high** for the loaded graph mapping and existing filtered intent owner. This packet covers the observed expression 410 boundary only.

The mounted raw-pad replay `artifacts/motion-expression180-scripted-2026-09-23.log` advances through expression 180, then stops at `PendingExpression(410)` in frame 13's Actor Motion prefix. The loaded graph assigns expression 410 to priority-zero transition 1375, target state 21. The active transition index list contains 1375 at position 995, inside state 687 `Idle`'s transition section (positions 994–996). The expression's conditions are:

| Index | Loaded class | Serialized attributes |
| --- | --- | --- |
| 259 | `HasFilteredMotionGraphIntent` | `abs=true`, `intent=BoardAdjustAngle`, `less=0.4` |
| 260 | `HasFilteredMotionGraphIntent` | `abs=true`, `intent=BoardAdjustMag`, `less=0.4` |
| 268 | `HasIntent` | `intent=BoardAdjustDown` |

In debug executable `sk82_na_m.xex`, `HCDelegate_MotionGraph_OnBoard::EvaluateExpression<1>` at `0x8290aa50` dispatches `jpt_8290AA8C[410]` to `0x8291a23c`. At `0x8291a260`, it reads condition 259 through `StateConditionDirect_HasFilteredMotionGraphIntent` using `sConditions+0x40c` (`259*4`). If false, it returns false at `0x8291a350`. Only when true does `0x8291a2b8` read condition 260 at `+0x410`; that must also be true. Then `0x8291a310` reads condition 268 via `StateConditionDirect_HasIntent` at `+0x430`. The final branch at `0x8291a340..0x8291a354` returns the negation of 268. Each read checks its own active bit in the delegate's `+0x20` word, takes a cached value from `+0x54` on a hit, and publishes the direct result on a miss. The exact typed expression is `c(259) && c(260) && !c(268)`, with lazy reads in that order.

`EvaluateExpression<2>` at `0x8291e218` dispatches `jpt_8291E250[410]` to `0x829221c8`, which returns one without condition reads. Native `GetTransitionActivation<1>` at `0x8292baf8` calls the Pre evaluator; state activation callers are `0x8292b8b8` for Pre and `0x8292b808` for Sustain.

The direct filtered condition wrapper at `0x82cc2910` tags the evaluated parent state and calls `HasFilteredMotionGraphIntent::ActivationFunc` at `0x82cc25c8`. Its `0x82cc25d4..0x82cc26a4` prefix resolves and checks the Actor's `ISkaterMotionGraph`, then calls the filtered intent getter through vtable slot `+0xc`. It calls `Intents::HasIntent` at `0x82cc26b0`; missing presence returns false at `0x82cc26c0`. When present, nonzero comparison operator invokes `ComparisonCondition::EvalCompare` at `0x82cc26e0`; operator zero returns true at `0x82cc26ec`. The existing generated constructor already parses this condition into `ConditionPayload::HasFilteredMotionGraphIntent` with comparison and name. The existing Actor method `motion_runtime_filtered_intent_value` reads the retained `filtered_intents` map; the existing shared `compare` handles `abs` and the native comparison ordering. The Motion runtime's condition dispatcher currently lacks this payload arm, so adding only expression 410 would immediately stop at `PendingCondition(259)`.

The unregistered `artifacts/motion-expression410-filtered-condition-2026-09-23-draft.patch` adds the exact Pre/Sustain arms, routes the parsed filtered condition through the retained filtered map with presence before comparison, and tests lazy condition order, both results, error propagation, and constant Sustain. The native Boolean path has no Rust `Result` error values; injected errors test the existing host failure prefix. No registered source was edited here. Root integration, Cargo gates and replay remain pending.
