# Owning MotionGraph dynamic session

Date: 2026-09-07. This checkpoint places the recovered generic dynamic
MotionGraph controller, one collaborator facade value, and one
PlayAnimation-instance allocator value under a single private owner. The owner
runs the complete immutable leaf-capability preflight before controller
construction and creates only short-lived borrowed adapters internally.

## Construction gate

Construction consumes an already built MotionGraph runtime and performs this
host-safety order:

1. Require completed structural `Process` and scan every arena leaf through
   `preflight_motion_graph_capabilities`.
2. If coverage is incomplete, return the complete ordered diagnostic report;
   do not construct a controller or invoke facade/allocator methods.
3. If coverage is complete, run the generic dynamic controller's structural
   validation and preserve its exact typed invariant error.
4. Own the controller, facade, and allocator in private fields.

The preflight remains a coverage snapshot rather than proof that callbacks
will succeed. Every callback still revalidates exact name, payload variant,
and callback family in `BorrowedMotionGraphBackend`. Mandatory preflight here
prevents a supported prefix of the mounted graph from running before its first
opaque leaf is encountered. Low-level controller/backend APIs remain available
for explicit synthetic research, but this safer owner cannot partially enter
an unsupported graph.

For the mounted onboard MotionGraph, construction must retain and return all
2,902 unsupported-leaf diagnostics before any callback. No mounted frame is
attempted.

## Ownership and update boundary

Each `update` or explicit `end_all_behaviours` call mutably borrows the owner's
three disjoint fields, creates an ephemeral `BorrowedMotionGraphBackend` from
the stored facade and allocator, and synchronously delegates to the stored
controller. The API accepts no external backend and exposes no mutable
controller, facade, allocator, replacement setter, or non-consuming split.
Read-only controller observations are forwarded.

This statically prevents callers replacing those whole stored Rust values
between owner-mediated ticks. It does not prove a stable memory address when
the owner moves, prevent interior mutability inside a generic facade, or prove
that the facade cannot internally retarget another entity. Strict native Actor
identity needs a later sealed authority-backed facade and Context token.

The existing callback and poison contracts remain unchanged. Allocation false
can retain a null active instance and poison at Begin; subsequent owner update
and EndAll calls return `Poisoned` before facade or allocator effects. EndAll is
explicit. Ordinary Rust drop does not fabricate an implicit native End pass or
swallow a fallible cleanup result.

## Verification

Synthetic tests cover construction-error precedence, full unsupported-report
retention with zero collaborator/allocator calls, one owned facade/allocator
identity across owner movement and multiple ticks, one Create with retained
per-activation state, explicit effect-free EndAll, null-instance poison and
no-work retry/EndAll, and the absence of an implicit backend/End callback when
a live session is dropped. Generic Rust field destructors remain outside that
claim. The unsupported fixture places a supported HasIntent prefix, two opaque
leaves, and a supported PlayAnimation suffix in one processed arena; both
opaque diagnostics return in `LeafId` order and no supported callback runs.

Six focused tests, three independent post-implementation audits, the 1,900-test
core suite and two compile-fail doc tests, formatting, and strict all-target
core clippy pass. Final reviewed source hashes are:

| Source | SHA-256 |
|---|---|
| `motion_graph_dynamic_session.rs` | `532FF4B57312BACF1916BF65B9F2A8A0B325D85813425380E9DB1C4CF5407D77` |
| `motion_graph_dynamic_backend.rs` | `34258AED6084E01BF9228062E1F1CF2AC77BEE0D8569047BC8D871E4C16A049E` |
| `motion_graph_capability_preflight.rs` | `321832F1B3EDA22CF3ECD0500EEE5DA47DA9EDA46AF25FACBFEFE0C3ADAF88EB` |
| `dynamic_state_graph_controller.rs` | `E0BD869E8AD3AAD46094C4233B550CB28756635467303E010D5A0A15AD152F7F` |
| `lib.rs` | `EB5D2EF71CBC08BC941190B8B17853449B262986EB01DB9BC25AD556C70ADEFA` |

## Nonclaims

- This owner uses controller Context `()` and does not yet represent the native
  six-pointer `StateGraph::Context`.
- The generic facade and allocator are stable stored values, not proven native
  Actor or allocator identities.
- No UCOM lookup, entity discovery, live animation backend, TransitionHook, or
  opaque leaf body is reconstructed here.
- The generic dynamic fallback is not the matched generated onboard default.
- No mounted graph, animation playback, input sample, physics step, or gameplay
  frame is executed.
