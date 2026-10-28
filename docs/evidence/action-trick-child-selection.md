# Released Trick child selection

Confidence: high, source-reviewed draft; unregistered and not compiled or run.
This does not establish recognizer-to-Trick execution or a complete skating frame.

The actual loaded Trick20 has Square52/expr91, Tail53/expr92, Nose54/expr93.
Fresh Pre828ff990 and Sustain82906048 both evaluate:

| Expression | Ordered formula |
|---|---|
| 91 | 108 OR (109 AND (110 OR 36)) |
| 92 | 111 OR (109 AND (110 OR 36)) |
| 93 | 112 OR (109 AND (110 OR 36)) |

108/111/112 are HasGestureIntent Square/Tail/Nose. 109 is real AG
HoldPattern presence. 110 is InParentStateForTime, less 0.4. 36 is the existing
PhysFilteredState Air condition. Every AND/OR is short-circuiting. In particular,
the Air fallback is required when HoldPattern exists but the timer fails. This
is not a timer-only hold rule. A gesture match skips all three fallback reads.

HasGestureIntent Activation82cc1870 resolves/requires the actual IActionGraph,
gets its retained intent map, then requires gpSkaterManager and obtains its
same GestureTrickMapping member. It calls GetGestureIntents82cc11b0 with NULL
pair output. The new actor_action_gesture_condition.rs is a proposed child of
actor_action_gesture_runtime.rs and reuses its captured identity checks and
shared services. It does not resolve Motion or Animation. The mapping lookup
retains native bucket/chain order, reads live cvar124 only on a match, and uses
the same application-global debug yPos and overlay holder. NULL pair allocation
is not dereferenced merely to answer presence with debug disabled; enabling
debug still requires the pair at the existing source use. Existing mapping
tests already discriminate this optional-output path and late failures.

InParentStateForTime Activation82c8c950 calls Base.GetParentState82ceecb0.
For the actual generated factory's cast-parent tag this returns its just-stored
state tag, not that state's declaration parent. TemplateHC.GetTimeInState
82929860 decodes the tag, permits FFFF NULL, validates other tags against 224,
then calls retained time-tree query82931c40. Missing time is -1.0. The callback
at c9b0/c9b4 rejects only ordered negative values; NaN proceeds to existing
EvalCompare82ceeea0. The new helper reads the existing controller BTreeMap and
does not relock the controller or sample dt. Selection precedes timer increment.

Integration proposal (root owns registered changes): register the condition
child beneath gesture_runtime and the selection child beneath generated Action
runtime. Forward otherwise-pending expressions91..93 to its expression helper.
Add HasGestureIntent/Gesture payload dispatch through the real optional mapping
service; propagate GestureRuntimeError through an explicit runtime error arm.
Add InParentStateForTime/Comparison dispatch reading condition.base.parent after
the existing wrapper store, then pass self.time_in_state and self.states.len()
to parent_time_condition. Existing condition109 and36 paths are reused.
Do not reinterpret the state declaration parent or advance time early.

Four new focused selection tests cover all three groups and both masks,
gesture-success and absent-Hold laziness, conditional Air fallback, selected
tag versus a different active state's age, missing/invalid tags, and unordered
versus negative age. They are draft tests, not executed. Root must compile,
independently review and run actual released-input corpus after integration.

Root registered this batch and independently reread Pre/Sustain cases91..93,
including their shared HoldPattern/timer/Air labels. Raw82cc1988 passes NULL
output, and raw82c8c9b4 rejects ordered-negative time before EvalCompare.
Actual wrapper still publishes the parent tag before activation and caches its
result afterward. Independent peer reviewed this integration with no defect
found. Clippy71 passes; focused/workspace/original corpus runs remain pending.
