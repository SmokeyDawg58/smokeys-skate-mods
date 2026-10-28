# Retained Action selection after the first Update

2026-09-12. Confidence **high**, awaiting root independent review and original
same-Actor repeated-Update validation. Files: `generated_action_runtime.rs`,
`generated_action_runtime_selection.rs`, its tests, and
`actor_action_runtime_session.rs`. This extends the retained initial controller;
it neither changes generated-controller selection nor constructs another owner.

The production caller is Actor::UpdatePhase1 `825f41a8`. Following opacity,
grab-type and conditional locomotion/body-dispatch work, it invokes Action
`825f4374`, clears Motion attributes `825f43a0`, then invokes Motion `825f43cc`.
The new Action method still requires the actual registered wrapper and uses
the same cached physical and animation queries as the initial implementation.
The complete outer Actor phase and generated Motion Update remain separate work.

## Fresh source

All functions below belong to
`StateGraph::GenUtil::TemplateHC<StateGraph::Gen::HCDelegate_ActionGraph_OnBoard>`
unless otherwise qualified. Full decorated names, source bytes and decompilation
are retained externally in `%TEMP%/skate2-action-continuation-source.json`.

| Address | Function and recovered semantics |
| --- | --- |
| `82951138` | GetNextState: initial currentFFFF evaluates root Pre; otherwise search transitions before EvaluateState. |
| `8292a020` | SearchTransitions wrapper: priority3,2,1,0, each restarted at current. |
| `82929e58` | SearchTransitions priority overload: descriptor index indirection, current-to-parent traversal, priority then activation then ValidTarget then Evaluate(target,Pre). A failed target leaf continues the search; only a returned leaf publishes outNext. |
| `82929d50` | EvaluateState: interrupt0 retains current only after child-to-root Sustain; interrupt1 reevaluates root; interrupt2 checks target ancestry then target subtree, falling back to root. Other values assert. |
| `82929c08` | Evaluate: current ancestry uses Sustain, every other candidate uses Pre. Active children retain descriptor order; an exhausted nonleaf returnsFFFF. |
| `829283b0` | StateHasDescendent walks the FIRST argument's parent chain, including itself. Native parameter names must not reverse this relation. |
| `82929618` | GetStateParentActivation2 visits child before parent and returns immediately on false. |
| `829296b0` | ValidTarget: target itself always uses Pre. A later shared ancestor switches to parent-inclusive Sustain; no shared ancestor requires Pre up to root. |
| `829297a8` | GetTransitionActivation1 uses transition expression and the state whose transition interval is being visited as condition parent. |
| `82952068` | UpdateLists: allows evaluation before GetNextState and clears only on return; resolves pivot, derives exit/enter, increments existing times, then publishes last/current. |
| `829292f0` | FindCommonAncestors constructs sorted ancestor IndexSets and derives uncommon suffixes using the common count. |
| `82928c30` | IndexSet8::intersect stops on mismatch or index greater than/equal to pivot. Pivot is an exclusive numeric limit. |
| `829291d8` | SetAncestors inserts each ancestor into the sorted capacity8 IndexSet. |
| `82928ff8` | UpdateTransition invokes hooks after Exit and before Enter, even independently of mCallBehaviours. All129 actual Action transitions have zero hooks. |

Raw UpdateLists `82952124..28` passes transition target as the first
StateHasDescendent argument. When target descends from current, including self,
`82952144` obtains target.parent as the pivot. Therefore an explicit self edge
can exit/reenter the target's parent as well as the target. This is preserved;
ordinary no-transition selection of the same leaf has no exit or enter states.
The later Insert into the local active set does not alter the already computed
exit/enter suffixes.

## Masked expressions and lifecycle

The earlier audited Pre expressions3,6,27,28,29,31,37,38,56 remain supported.
The separately read Sustain evaluator `82906048` has the same conditions for
3,6,37,38,56, but27/28 return true without a condition callback and29/31 call
only condition96 DisablePushBrake and negate it. It is incorrect to reuse the
entire Pre expression body for Sustain. Other expressions remain precise
masked-expression errors; condition96 remains its existing explicit missing
callback. No unknown predicate is converted to false.

The same condition cache is cleared each Update, with callback parent/cache
partial writes unchanged. A failed search retains allowEvaluations=true and
earlier effects, without publishing selected state or incrementing timers.
After successful selection the flag clears before no-leaf assertion or list
construction. Existing times advance in key order using dt+old f32. Last/current
publish before Exit. Exit retains reverse packed behaviour traversal and
releases only after End returns; transition boundary then precedes Enter.
An unchanged leaf therefore retains all occurrence tokens and state timers and
executes only its existing generated StateUpdate. The Actor wrapper writes the
reached Context instance back on both success and checked callback failure.

The original Action table's129 zero-hook declarations are checked by tests.
A nonempty hook declaration is rejected after the reached Exit prefix, before
Enter, instead of inventing a hook. Only the previously recovered Default3
generated callback body is executable; another selected leaf retains its exact
unrecovered callback/state-update boundary. This is continuous selection and
the supported retained lifecycle, not complete Action gameplay or Motion.

## Validation

Focused tests distinguish ancestor high-priority edges from child lower-priority
edges, indirect transition ordering, failed nonleaf targets, child-first Sustain,
Pre on siblings, explicit self-edge pivot behavior, mask-specific lazy condition
calls, and actual original Default3 re-selection without Enter/Exit. Synthetic
topologies are algorithm tests, not reference runtime traces. The original-table
test uses controlled predicate results and makes no Actor-input parity claim.

Initial author Cargo attempts were blocked by concurrent files outside this
ownership (missing animation publication tests and unfinished Skeleton API
integration). Root owns independent review, final compile/strict gating and the
published same-Actor multi-frame corpus. No earlier checkpoint validates this
batch. Full input/animation/physics scheduling and native per-tick traces remain
required before hands-on skating.
