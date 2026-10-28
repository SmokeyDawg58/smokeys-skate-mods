# MotionGraph Entity authority and bound session

## Scope

This checkpoint connects the recovered semantic six-slot
`StateGraph::Context` to a production-facing dynamic MotionGraph session. It
adds one privately issued Entity authority, one separately issued Motion
controller token, authority-parameterized runtime services, and a private
adapter that validates callback identity and Context binding before supported
Context-bearing callback effects. Context-free CreateInstance is excluded from
that binding check.

It does not make the mounted MotionGraph runnable. The generic controller is
still the fallback selected only when no generated controller is available,
and only `HasIntent`, `WillExpire`, and `PlayAnimation` have recovered bodies.

## Native ownership and Context evidence

PDB ordinal 17267 fixes `StateGraph::Context` at 0x18 bytes with controller,
Entity, Instance, CameraMan, BehaviourSource, and BehaviourSink at
`+0/+4/+8/+c/+10/+14`. Ordinal 17280 fixes the containing
`HierarchicalController` at 0x20 bytes: inline Context, root hash at `+18`, and
inner controller pointer at `+1c`.

`Actor::Actor` at `0x825fe850` passes the address of its contained
`Sk8::Entity` subobject to both separately allocated graph objects:

- ActionGraph allocation/construction/interface installation occurs at
  `0x825fec68`, `0x825fec84`, and `0x825fec90`.
- SkaterMotionGraph allocation/construction/interface installation occurs at
  `0x825fec9c`, `0x825fecb4`, and `0x825fecc8`.

MotionGraph construction at `0x82cea060` seeds
`[null, Entity, null, null, null, null]`, copies those six words at
`0x82cea124..0x82cea134`, and writes the newly allocated wrapper's own identity
to slot zero at `0x82cea138`. ActionGraph performs the same sequence at
`0x82cea340` and `0x82cea3e0..0x82cea3f4`. Thus the two wrappers share one
Entity pointer and have distinct controller identities.

The dynamic inner constructor stores the address of the wrapper's inline
Context at `0x82cf6a30`. `HierarchicalController::Load` at `0x82cef180`
replaces only the inner controller and reuses that same Context. The base
Instance, CameraMan, source, and sink slots are exactly null at Motion/Action
construction.

Generic Begin, Update, exit End, and EndAll End shallow-copy all six words and
replace only the copied Instance slot. The relevant copy/overlay regions are
`0x82cf520c..0x82cf522c`, `0x82cef8ec..0x82cef908`,
`0x82cf33cc..0x82cf33f4`, and `0x82cf3220..0x82cf3244`. The active
PlayAnimation instance therefore remains the existing separate callback
overlay rather than base Context state.

The Actor destructor calls the Action deleting destructor before Motion at
`0x825f9800..0x825f9844`, while the Actor/Entity still exists. Motion and
Action graph destructors delete their inner controllers and free their wrappers
at `0x82ce9f80` and `0x82cea220`; they do not destroy or retain/release Entity.
Dynamic controller destruction at `0x82cf63b8` does not call EndAll. Explicit
EndAll remains separate.

## Host boundary

`MotionGraphEntityAuthority<K>` is an opaque issuance backed by a private
identity allocation containing `K`. Equality is allocation/issuance identity,
not key-value equality. Its constructor is crate-private and it has no `Clone`;
only the explicit `StateGraphContextHandle::shallow_copy` operation duplicates
the handle. `MotionGraphControllerToken` follows the same rules and is issued
independently for every bound session.

`ActorGraphAuthority<K>` is the only public authority issuer. Multiple graph
facades created by one owner receive shallow handles to the same issuance.
`MotionGraphRuntimeFacade<K, R>` owns one such handle and its runtime and
exposes neither runtime mutation/replacement nor an authority retarget
operation. Its short-lived internal adapter passes the same immutable authority
to every method of the three authority-aware runtime traits. Implementations
remain trusted to apply effects to that supplied authority; Rust cannot prove
the behavior of arbitrary service code.

`BoundMotionGraphContext<K>` is exactly the semantic six-slot type with a
Motion controller token and Entity authority in the first two slots. Four
uninhabited handle types force the base Instance, CameraMan, source, and sink
slots to remain `None` in this narrow slice.

`AuthorityBoundMotionGraphSession::try_new` performs complete immutable arena
preflight before issuing a controller token or invoking runtime/allocator
methods. It constructs both binding and Context internally. For a supported
Condition or Behaviour callback the private adapter performs, in order:

1. exact registered-name/kind/payload identity validation;
2. exact controller and Entity issuance validation;
3. delegation through the authority-aware runtime adapter.

This applies to supported Context-bearing Condition and Behaviour callbacks,
preserves identity-error precedence, and prevents their runtime effects from
preceding a binding error. CreateInstance has no native Context parameter;
it uses only the privately stored allocator after session construction.
Transition hooks remain unsupported and return their exact identity error
without a runtime effect. Callback-time leaf identity validation remains in
the established low-level backend as defense in depth.

All session fields are private. There is no controller/context/runtime/allocator
mutable accessor, external-backend update, `Pin`, self-reference, or raw pointer.
Moving the session preserves semantic issuance identity. Drop still does not
synthesize EndAll.

## Verification

Focused tests cover independent issuance despite equal keys, one Actor owner
sharing Entity issuance across sessions while controllers remain distinct,
explicit shallow copies, exact constructor slot population, all four
missing/foreign binding errors, session movement across two ticks and EndAll,
authority observation on every runtime service call, and preflight before
effects. Adversarial controller tests prove malformed leaf identity precedes a
bad Context and that a valid leaf with a missing Entity is rejected before any
entity-runtime or allocator effect. A malformed CreateInstance identity is
also rejected in the outer identity category before allocation. Compile-fail
documentation tests reject external Entity/controller issuance and runtime
mutation through the facade.

Twelve focused unit tests, three new compile-fail API checks, three independent
post-implementation audits, the 1,917-test core suite, six core doctests,
formatting, and strict all-target Clippy pass. Final source hashes and the
complete workspace gate are recorded in the research log for this checkpoint.

## Nonclaims

- Authority and controller tokens are not native pointer values, wrapper
  addresses, ABI fields, UCOM handles, refcounts, or proof of liveness.
- Retarget prevention is a checked host API policy, not a proven native
  prohibition. A runtime implementation can ignore its argument or mutate
  externally shared state; such behavior is outside this boundary's claim.
- The opaque `K` is not yet a world slot/generation. Existing living-world DMO
  IDs and character asset IDs are not relabeled as the Actor Entity.
- The token allocation may outlive the runtime; it does not keep an Actor or
  native Entity alive. The Context remains semantically non-owning.
- The four null slots are established only for this constructor/three-leaf
  slice. Generated Action/Motion updates can directly install a base Instance,
  and recovered source/sink setters require later integration.
- Native aliasing, deletion, thread behavior, CameraMan behavior, cross-graph
  source/sink sharing, allocator authority, and Actor-level Action/Motion
  orchestration remain open.
- No opaque leaf, generated controller, mounted graph, animation system, input,
  physics, or gameplay frame is executed by this checkpoint.

## Evidence hashes

| Evidence | SHA-256 |
|---|---|
| XEX | `3706CEDA...FE2` |
| PDB | `0B060CEC...EF0` |
| MAP | `51E60F09...E88` |
| PDB symbol/type extraction | `62516420D7009BEAC3BA7D2FDBBB080DA725E2C3421A5D3A008C3351FF092D9F` |
| controller proof/raw | `12A5DB655DDF3FED35C8BDF244E58650B9A20C6AF05F17EE25AB129A53914CB4` / `E5E4AE52A71B682660FE2C74A0618531DBFEC2CD14DC54E49517F8EB42ABB218` |
| controller/owner batches | `B2B61749EC0493FD464338E4DA8D1727BE6B44FD5CB5EF350514FEA929909AD4` / `A83FD9360BC04274B6E4AAAC25299A4C33E80FB7B033751104CFD021168DFE04` |
| lifecycle proof/raw | `A563AE6A8DB5E666D17C94277C0071AE98C879C3CEA703C7ED1AF25362AE4AD1` / `D08A8AB0E285816637BB75C3C65951B7AA502966B0B9F670403C401824AD20E5` |
| generated proof/raw | `465E0612BA72F5192ED47CB8BBD826F8691A5BDAB6F2CA74EA034DAC90A6043B` / `3A5899FEAA53E59A9A4F9C7E79A65845FBDC372F06CE99F8288D87EEBAA7BB81` |
