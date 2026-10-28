# Dynamic StateGraph fallback controller

Date: 2026-09-07. This checkpoint reconstructs the matched debug build's
generic `DynamicHierarchicalController` selection and lifecycle boundary. It is
the native decoded-graph fallback, not the generated ActionGraph/MotionGraph
controller selected by default for the onboard player.

## Evidence identity

- Matched debug XEX SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.
- PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
- MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
- Corrected controller proof SHA-256:
  `12A5DB655DDF3FED35C8BDF244E58650B9A20C6AF05F17EE25AB129A53914CB4`.
- Controller raw packet SHA-256:
  `E5E4AE52A71B682660FE2C74A0618531DBFEC2CD14DC54E49517F8EB42ABB218`.
- Corrected base-lifecycle proof SHA-256:
  `A563AE6A8DB5E666D17C94277C0071AE98C879C3CEA703C7ED1AF25362AE4AD1`.
- Lifecycle raw packet SHA-256:
  `D08A8AB0E285816637BB75C3C65951B7AA502966B0B9F670403C401824AD20E5`.

Primary code authorities are `DynamicHierarchicalController::Update` at
`0x82CF7EC0`, `UpdateLists` at `0x82CF7598`, `GetNextState` at `0x82CF6AE0`,
`EvaluateByPriority` at `0x82CF04D8`, transition search at `0x82CF5B80` and
`0x82CF6560`, `ValidTarget` at `0x82CF5588`, and `EvaluateState` at
`0x82CF1AD8`. Lifecycle authorities are time update `0x82CF0478`, exit
`0x82CF3310`, hooks `0x82CEF938`, two-pass entry `0x82CF4FE8`, forward update
`0x82CEF870`, and reverse EndAll `0x82CF3198`.

## Reconstructed selection

The first update evaluates the supplied root with PreCond. The supplied State
is not screened by its own enabled or active fields; only recursively traversed
child edges require both. A State expression must activate, and only a State
with no children is returned. Children are tried in source order.

Later updates search transitions by urgent, high, medium, then low priority;
within a priority they search current State to ancestors, preserving each
State's transition order. Candidate gates are transition enabled, target
enabled, direct PreCond activation, `ValidTarget`, and a nonnull target-subtree
result. Target active is not a transition gate. Repeated activation between
`ValidTarget` and target evaluation is preserved rather than memoized.

All three interrupt policies are represented. Sustain is used on the current
hierarchy, while alternative branches receive PreCond. Explicit transitions to
self or an ancestor compute the common boundary against the target's parent,
forcing the target subtree to exit and re-enter.

Every actually constructed `StateHierarchyList` slice is limited to 16 State
pointers. The active timer map has an independent 16-key limit. Checked host
errors contain assertion continuation and poison a partially evaluated update;
they are not claimed native success behavior.

## Reconstructed frame lifecycle

After selection and list construction succeed, the controller performs this
order:

1. Assign old current to last and the selected State to current.
2. Add the exact `f32 dt` once to every pre-existing time-map entry, including
   States that will exit later in the tick.
3. Traverse active Behaviour/Instance pairs backward, End matching exits, then
   remove their pairs and exit timers.
4. Invoke every selected TransitionHook in attachment order without checking
   the Hook enabled byte.
5. Reset every entering State timer to `+0.0f` in a complete root-to-leaf pass.
6. In a second root-to-leaf pass, visit Behaviours in attachment order, skip
   those disabled at entry, CreateInstance, append the pair, then Begin.
7. Update all retained and newly created Behaviours in forward insertion order.

Null Behaviour instances are retained. An enabled byte is sampled only at
entry; a retained Behaviour still receives Update and End. Missing State-time
queries return exactly `-1.0f`. EndAll runs backward and does not clear timers.

## Checked host failure containment

The recovered native callbacks do not expose a recoverable error return. The
Rust collaborator boundary is nevertheless fallible so an unavailable or
incomplete concrete reconstruction cannot be treated as a successful gameplay
tick. This is host-side containment, not a claim about native recovery:

- Condition, CreateInstance, Begin, Update, End, and TransitionHook failures
  poison the controller immediately. A later Update or EndAll returns
  `Poisoned` before changing `dt`, state, timers, payloads, instances, Context,
  or invoking another callback.
- Lifecycle errors retain the exact State/leaf or Transition/hook identity and
  standard error-source chain. Activation backend failures now retain their
  backend source through the same chain.
- CreateInstance failure appends no active pair. Begin failure occurs after
  append, so the pair and optional instance remain retained. Update failure
  removes nothing.
- Exit End walks in reverse. Successfully returned suffix entries are removed,
  while the failing entry and untouched prefix remain retained. No exit timer
  is erased until the complete End pass succeeds.
- A hook failure occurs after all successful exit removal/timer erasure and
  before the complete enter-timer pass. EndAll likewise removes only the
  reverse suffix whose End calls returned success and never clears timers.
- The controller does not roll back payload, instance, shared-pointee, or
  original Hook-Context effects performed before a backend returns `Err`.
  Direct writes to a Behaviour's shallow Context copy are discarded at the
  normal copy boundary. Backend effects beyond these ownership facts remain
  backend-defined and are not characterized as native behavior.

Panics still unwind normally and are not converted into the poisoned Result
path. The successful native callback and mutation order is unchanged.

The controller consumes and owns a successfully processed graph and a stable
base Context, preventing arena, payload type, or Context swapping. A successful
Process latch is set only after the complete structural traversal returns.
CreateInstance receives the mutable Behaviour object without Context. Hook
callbacks receive the mutable original Context. Each Behaviour callback gets a
fresh shallow six-slot-equivalent Context copy; its direct slot writes are
discarded, while writes through shared pointees remain visible. The separately
supplied paired instance represents replacement of native Context `mInstance`
at `+0x08`. `DynamicStateGraphContext` makes this shallow-copy contract
explicit and deliberately has no blanket `Clone` implementation.

Fourteen focused controller tests cover initial selection gates, priority and
source ordering, all interrupt modes, mixed Sustain/PreCond routing, sibling
and forced-self transitions, exact callback/timer traces, null instances,
mutable Context and payload boundaries, successful-Process enforcement,
EndAll, 16/17 hierarchy capacity, timer-union overflow, and poisoning. The
failure corpus injects errors in the middle of three-entry Create, Begin,
Update, exit-End, and Hook sequences, checks EndAll's partial reverse prefix,
proves no retry work, and freezes pre-`Err` ownership effects. Two independent
source audits passed. Source SHA-256 values at this checkpoint are:

| Source | SHA-256 |
|---|---|
| `dynamic_state_graph_controller.rs` | `E0BD869E8AD3AAD46094C4233B550CB28756635467303E010D5A0A15AD152F7F` |
| `state_graph_runtime.rs` | `6AF512D03DD8D653C1C80D313ED8BD0666767B52BD4424688C619BD141548721` |
| `lib.rs` | `361808408A70F689C63A69014D95288FC6A7B5CB0E94235B3D28ADC97F1FB04E` |

## Generated onboard boundary

Separate research proves the matched default remains generated. The smallest
leaf-free native seam is read-only generated `StateDesc` access: Action
`HCDelegate_ActionGraph_OnBoard::State` at `0x82928268` has 224 entries at
`0x821766B0`; Motion's accessor at `0x8292A538` has 1,354 entries at
`0x82195FA0`. Each PDB-backed descriptor is `0x20` bytes.

Generated index order diverges from decoded dynamic wire order at index 2 in
both graphs, so IDs must never alias. Path-keyed topology nevertheless matches
all 1,578 State paths for name, active byte, interrupt metadata/target path,
and direct Behaviour/Transition counts. This is structural evidence only.

A generated tick is not leaf-free: its constructors instantiate concrete
Behaviour, Condition, Hook, and Param-backed objects, and baked expression
evaluators call concrete condition helpers directly. Generated entry also
zeros and begins one State at a time, unlike the dynamic fallback's complete
timer-reset pass. The generated seam proof SHA-256 is
`465E0612BA72F5192ED47CB8BBD826F8691A5BDAB6F2CA74EA034DAC90A6043B`.

## Nonclaims

- Concrete gameplay Condition, Behaviour, Instance, and TransitionHook effects
  are still opaque callbacks.
- Fallible Result containment is a checked host boundary; native callback
  recovery, rollback, and retry semantics are not claimed.
- No native initial onboard State path is claimed without concrete conditions.
- This dynamic fallback is not substituted for, or claimed equivalent to, the
  generated onboard default.
- Actor intent generation, animation, skater physics, rendering, and UI do not
  become complete through this checkpoint.
- Callback reentrancy, native allocation/refcount timing, assertion
  continuation, active-controller Reload cleanup, and generated update
  functions remain outside this boundary.
