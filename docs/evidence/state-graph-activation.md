# StateGraph generic activation

Date: 2026-09-07. This checkpoint reconstructs the matched debug build's
generic Condition-mask and ExpressionOperator activation layer over the typed
StateGraph runtime. Concrete gameplay Condition booleans remain an explicit
backend boundary.

## Evidence identity

- Matched debug XEX SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.
- PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
- MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
- Activation/lifecycle proof SHA-256:
  `A563AE6A8DB5E666D17C94277C0071AE98C879C3CEA703C7ED1AF25362AE4AD1`.
- Raw assembly/pseudocode/PDB packet SHA-256:
  `D08A8AB0E285816637BB75C3C65951B7AA502966B0B9F670403C401824AD20E5`.
- Extraction script SHA-256:
  `C48AD0C59B59CA1B2FD967E843F29334848F4C13CA87E2C13C211EBEC4A5F851`.
- Frozen IDA copy SHA-256:
  `F818425AD983C1B789392BEC02B55249811E49D4B679516729F2817059A5663F`.

The primary code authorities are `Condition::GetActivationMasked` at
`0x82CEF0F8`, `ExpressionOperator::GetActivationMasked` at `0x82CEF5E0`,
`Node::GetActivation` at `0x82CEF9B0`, and
`State::GetActivationParent` at `0x82CEFC70`. Claims were checked against PPC
instructions and PDB declarations; decompiler output is explanatory only.

## Reconstructed semantics

The exact `CondMask` values are Pre=1, Sustain=2, Always=3, and Post=4.
Always is the bitwise union of Pre and Sustain. A Condition calls its concrete
`ActivationFunc` only when its inherited Base enabled byte is nonzero and its
stored mask intersects the requested mask. Disabled or nonintersecting
Conditions return true with `wasMasked=true` and make no concrete callback.

Expression children are evaluated in stored order:

- NONE and AND share true identity, ignore masked children, and stop at the
  first unmasked false.
- OR starts false, ignores masked children, stops at the first unmasked true,
  and returns true/masked when every child is masked.
- NOT requires exactly one child, propagates a masked child as true/masked, and
  negates an unmasked child.
- Empty NONE, AND, and OR all return true/masked.

A Node with no expression, or with a wholly masked expression, activates.
State ancestor activation evaluates the supplied State first and then parents,
short-circuiting on the first false. The linked weak/editor function address is
present in this matched XEX, so expressions are evaluated; it is not exposed as
a mutable host switch.

The Rust API keeps concrete leaves separate through
`StateGraphConditionActivationBackend`. It supplies the typed leaf ID,
canonical registered name, mutable factory payload, and caller-owned context.
The core owns masking, combination, ordering, and short-circuit behavior.
Checked invalid-ID, wrong-kind, missing-mask/payload, backend, and NOT-arity
errors contain malformed host inputs instead of reproducing assertion
continuation.

Sixteen focused StateGraph tests include the full 4x4 stored/requested mask
matrix, zero-call masked paths, empty/all-masked identities, mixed and nested
short-circuit order, State/Transition no-expression behavior, self-before-
ancestor order, and every checked error boundary. Independent source audit and
strict core clippy pass. `state_graph_runtime.rs` SHA-256 is
`05F0B2D5732FC34EFF5356E101DCABFED8D14F1F0BBD2CB1C9274FB5B471D893`.

## Controller boundary established for the next slice

The same evidence pass bounded the native dynamic fallback controller without
implementing it yet. Its exact tick is: select next State and build hierarchy
lists; add single-precision `dt` to retained State timers; reverse-End exiting
Behaviour instances; run transition hooks in source order; zero all entering
State timers in a complete first pass; root-to-leaf Create/Begin Behaviours;
then forward-Update every current Behaviour, including newly begun instances.
Each built hierarchy slice accepts at most 16 pointers.

Controller-boundary proof SHA-256 is
`12A5DB655DDF3FED35C8BDF244E58650B9A20C6AF05F17EE25AB129A53914CB4`;
its raw packet is
`E5E4AE52A71B682660FE2C74A0618531DBFEC2CD14DC54E49517F8EB42ABB218`.

This is a native fallback, not the matched onboard default. The exact cVar
initializer sets `kbUseGeneratedStateGraph=true`; ordinary onboard player load
enables the generated ActionGraph and MotionGraph delegates. Those generated
controllers bypass the decoded graph and remain a separate parity target. A
dynamic-fallback controller is still the narrowest native path for exercising
the reconstructed graphs, but any result must be labeled as that fallback.

The onboard corpus ranks MotionGraph `HasIntent`, `WillExpire`, and
`PlayAnimation` as the smallest high-leverage research cluster: 2,402
occurrences, with all 165 Params belonging to PlayAnimation. This is
centrality evidence, not proof that those three concrete classes alone can
produce a native-faithful tick. The corpus proof SHA-256 is
`B2DC4A8B98055BFBB765895FFEFB37938D0B988E47DC5B53013F9CB7EFD7A1BE`.

## Nonclaims

- No concrete Condition result, Behaviour instance/side effect, or
  TransitionHook side effect is implemented here.
- No native initial Action/Motion State path can be claimed without concrete
  condition outcomes. A deterministic stub can prove topology only.
- The generated default controller path is not substituted or claimed equal to
  the dynamic fallback.
- PostCond is represented exactly, but the recovered base controller does not
  request it and the shipped StateGraph corpus contains no PostCond leaf.
- Callback-driven mutation/reentrancy, native allocation/refcount timing, and
  assertion continuation remain outside the safe-host boundary.
