# Actual first Action Update

2026-09-12 source packet, new `generated_action_runtime.rs` and
`actor_action_runtime_session.rs`. Companion callback proof is
`actor-action-motion-intent-bridge.md`. Root independently reviews and gates
integration. No Cargo was run by the implementation agent.

The runtime acts on the original retained generated Action controller, statics,
condition cache, time map, current/last state and 42 packed instance slots.
The Actor session clones the actual retained controller Arc under a short
prefix lock, releases the lock and invokes Update through that same Mutex
owner. It never takes the controller out of its wrapper. Before running and
when writing back Context, it checks controller Arc and wrapper identity.
The original Context's reached instance slot is written back after success
or checked failure; the other five original slots survive. Begin/End only
alter shallow callback copies. Generated StateUpdate alters the original
Context, matching its source signature.

## Implemented selection domain

Update82955c78 first clears five condition-active words and stores dt.
UpdateLists82952068 sets allowEvaluations=true82952090 then calls
GetNextState829520ac; the clearing store is829520b8. Raw Update calls
UpdateLists at82955cc4 (the decompiler associates its return with5cc8).
The supported initial branch is currentFFFF -> Evaluate82929c08(root0,
PreCond). ExpressionFFFF returns true; every other expression is evaluated
through the actual supported table expression and concrete Actor queries.
Active children are considered in descriptor order, recursively; a parent
whose children all fail does not become a selected leaf. Unsupported expression
or condition returns a precise boundary instead of false or a canned state.

Implemented fresh `EvaluateExpression<1>`828ff990 cases:

| Expression | Source condition calls, in lazy order |
| --- | --- |
| 3 | 6 |
| 6 | 36 |
| 27 | 126 |
| 28 | 127 |
| 29 | 128 AND NOT127 AND NOT96 |
| 31 | 70 AND NOT127 AND NOT96 AND NOT130 |
| 37 | 0 AND NOT1 |
| 38 | 2 AND NOT1 |
| 56 | 74 AND NOT75 |

Condition cache bits use native condition index/32 and index%32. Cache hits
read retained results and perform no callback/parent mutation. A miss first
sets the shared static condition's actual Base.parent Mutex cell to the typed
state index. Raw IndexToState829282c0 uses `not r3,r31`8292831c for valid
indices and zero forFFFF: this is a complemented-index tag, not a heap state
object pointer. Then Activation runs; active and value stores only follow
successful return. A callback failure therefore retains the assigned parent
and earlier cached conditions but not that condition's result.

Concrete callbacks implemented: HasAGIntent (actual Action map presence and
existing source comparison), TimeSinceLastInput (cached same-Actor IPhysical
output+38 Animation.time_since_last_input), PhysFilteredState (output+40
Filtered.filtered_physics_state), IsGrinding and PhysicsRequestsDismount.
All use the actual full-HiLOD cached accessor; registered-only allocation is
insufficient. Condition6 IsGrinding82caa310 requires actual ISkaterAnim
occurrence first, then IPhysical, then Filtered.is_grinding at+50. A specific
grind name performs a second GetPhysOut and compares Filtered+8
skating_grind_name. It does not read similarly named State or Grinds flags.
PhysicsRequestsDismount82c98c70 raw82c98d4c/50 loads output+1c State+43,
the existing physics_requests_dismount field. Unknown96/130 remain explicit
callbacks when those lazy branches are reached.

If GetNextState errors, allowEvaluations remains true: the source clearing
store has not executed. Successful selection clears it; absent selected leaf
then reports the source assertion boundary. Initial transitionFFFF means no
transition hooks. Selected/current ancestors use original parent links with
source depth8 bound (SetAncestors829291d8). Existing state times increment by
f32 dt; last=current/current=selected publish before Exit/Enter. No state or
input value is forced to obtain the supported leaf.

## Retained lifecycle and direct calls

Enter829527f8 inserts state time0, then enabled behaviour descriptors in order.
It invokes actual CreateInstance, publishes the behaviour/instance pair in the
sorted CurrentBehaviours/packed mInstances before optional Begin, and creates
instances even when callBehaviours=false. Transfer and Juice instances are
their source constructors; PrintText2D binds shared null constructor82bb3da8
(`li r11,0; stw r11,0(r3); blr`). That successful null remains None, not a
placeholder instance. Unrecovered constructors stop before pretending to
allocate/construct their payload. Already entered state times, published
instances and earlier Begin side effects survive.

Exit82929a08 traverses active packed behaviour pairs in reverse, calls End on
those whose descriptor parent is exiting, then removes/releases the pair.
Only afterward does it erase each exiting state's timer, requiring membership.
The initial source branch has an empty exit set; the implementation does not
claim the later transition-selection path is complete.

UpdateBehaviours829290d0 first validates every packed behaviour identity
against its original static object. With calls enabled, only independently
read StateUpdate_Default_3 at828ecf30 is currently dispatched. It calls
StateUpdate_Action_0 at828ecf4c, then overlays packed slot11 and calls shared
no-op825187c0 at828ecf60. The root function executes Juice0 then transfers1
through10 with an original Context instance overlay before each. The checked
dispatch requires the actual descriptor layout [0..10,38]; it never selects
Default merely to make this layout available.

Default3 is a real leaf of root0. Constructor/Reset Filtered state is Invalid,
Animation.time_since_last_input is0 and State.physics_requests_dismount is
false; these facts may allow actual evaluation to reach Default depending on
the real Action inputs. A physical state enumGround does not imply a filtered
Ground output. Other selected leaves retain their prefix and report the first
unrecovered constructor or selected Update address. Any subsequent Update
reaches PendingSearchTransitions after source condition-clear/dt/allow prefix;
there is no false claim of a continuous Action graph yet.

Transfer null-instance fault timing is preserved explicitly: Begin first
creates/erases its actual destination key then faults at byte8=true; onUpdate
true Update likewise creates before null suffix byte8=false, while onUpdate
false faults reading byte8 before Remove. End does not dereference instance
and still removes the key. Null Juice Update validates/gets actual Motion
before the invalid trick-name read; its Begin/End are true source no-ops.

Pending validation: root workspace/strict gate and original same-Actor first
Update corpus after actual cached HiLOD publication. Direct callback tests
and constructor evidence alone are not a completed gameplay frame.
