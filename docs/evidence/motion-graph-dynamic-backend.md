# Borrowed MotionGraph dynamic backend

Date: 2026-09-07. This checkpoint connects the three recovered MotionGraph
payload bodies to the already reconstructed fallible dynamic StateGraph
controller. It is a synthetic integration seam only. The mounted onboard graph
continues to be refused by the all-leaf capability preflight because 2,902 of
its leaves remain opaque.

## Evidence boundary

The callback bodies are not newly inferred here. Their native evidence and
checked-host boundaries remain frozen in:

- `motion-graph-conditions.md` for `HasIntent` and `WillExpire`;
- `play-animation-lifecycle.md` for `PlayAnimation` `CreateInstance`, `Begin`,
  `Update`, and no-op `End`;
- `dynamic-state-graph-controller.md` for selection, lifecycle ordering,
  partial-effect retention, and poison containment; and
- `motion-graph-capability-preflight.md` for complete-arena implementation
  coverage and mounted refusal.

The adapter consumes the exact canonical registration bytes and typed payload
variant again at every callback. Only these triples delegate:

| Callback family | Canonical registration | Payload variant |
|---|---|---|
| Condition | `HasIntent` | `HasIntent` |
| Condition | `WillExpire` | `WillExpire` |
| Behaviour | `PlayAnimation` | `PlayAnimation` |

Names are byte- and case-sensitive. A canonical name paired with the wrong
variant is a typed mismatch before callback-family classification. A matching
known tuple at the wrong callback family is a kind mismatch. Unknown typed
payloads are unsupported identities, while unknown opaque payloads are
registered-opaque failures. Every TransitionHook is rejected before
collaborator or allocator effects. Callback kind is derived from the callback
site; the adapter does not claim to read physical node kind. A prior successful
preflight is not accepted as an execution token.

## Borrow and lifecycle contract

`BorrowedMotionGraphBackend` holds one exclusive borrowed collaborator facade
and one separately borrowed instance allocator. During that adapter borrow,
both recovered Conditions and `PlayAnimation` `Begin`/`Update` use the same Rust
facade object. Only `CreateInstance` touches the allocator. `End` validates the
exact PlayAnimation identity and then performs the recovered no-op; a null
instance is valid there.

Each successful allocation produces an independently owned, non-cloneable
`PlayAnimationRuntimeInstance`. Allocation false remains successful
`None`. The controller appends that null pair before `Begin`; `Begin` then
returns the checked `MissingInstance` boundary before any facade operation.
The controller preserves the pair and becomes poisoned, exactly according to
the previously documented fallible-controller containment contract. No retry
or rollback behavior is invented.

Delegated failures retain callback-leaf identity and their concrete nested
error source. Identity failures retain the callback site, raw registered name,
raw `LeafId`, actual variant, and mismatch reason. The adapter does not erase
effects completed inside a collaborator before an error.

## Verification

Focused tests use only synthetic, successfully processed graphs and spies. They
cover exact identity acceptance/rejection, case and embedded-NUL rejection,
effect-free opaque/Hook refusal, both Condition call orders, PlayAnimation
allocation and independent first-update latches, integrated
Condition-to-Create-to-Begin-to-same-frame-Update order, null-allocation poison
and retained-pair behavior, no-op End, and nested backend-source preservation.
Three independent post-implementation audits passed. Focused tests, the full
core test suite, formatting, and strict all-target core clippy pass. The final
reviewed source snapshot is:

| Source | SHA-256 |
|---|---|
| `motion_graph_dynamic_backend.rs` | `34258AED6084E01BF9228062E1F1CF2AC77BEE0D8569047BC8D871E4C16A049E` |
| `motion_graph_conditions.rs` | `027391D2C296A50C61A4C125DFD1E8E69C629CDF617403143D7B430EACC2383F` |
| `lib.rs` | `8EC0555E612FCA152ED74EC5CB444BA63032E6B642889F3DD810EEBFBD9BA2B2` |

## Nonclaims

- The borrowed Rust facade is not proof of stable native Actor identity across
  controller ticks; a caller can drop it and provide another adapter later.
- The controller Context type is deliberately `()`. This checkpoint does not
  reconstruct the Actor constructor's native six-pointer Context or its entity,
  instance, camera, behaviour-source, and behaviour-sink slots.
- The facade is a typed collaborator contract, not reconstructed UCOM lookup,
  entity discovery, or a live animation backend.
- No opaque leaf, TransitionHook, generated onboard wrapper, mounted controller,
  animation playback, input sample, physics step, or gameplay frame runs here.
- The generic dynamic fallback remains distinct from the generated onboard
  MotionGraph controller used by the matched game.
