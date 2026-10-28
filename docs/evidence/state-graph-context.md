# `StateGraph::Context` semantic slots

Date: 2026-09-07. This checkpoint records the exact native six-pointer Context
layout and reconstructs its shallow-copy boundary as typed semantic host slots.
It deliberately does not claim that the Rust generic record has the native ABI,
owns any pointee, or supplies a stable Actor authority.

## Evidence identity and confidence

The PDB layout and listed PPC constructor/copy/overlay operations are exact for
the matched image. Generated-controller evidence is exact only for the listed
template Begin/End paths; the complete generated state-specific Update corpus
is not claimed.

- Matched debug XEX SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.
- PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
- MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
- PDB/type raw packet:
  `62516420D7009BEAC3BA7D2FDBBB080DA725E2C3421A5D3A008C3351FF092D9F`.
- Controller-boundary proof/raw:
  `12A5DB655DDF3FED35C8BDF244E58650B9A20C6AF05F17EE25AB129A53914CB4` /
  `E5E4AE52A71B682660FE2C74A0618531DBFEC2CD14DC54E49517F8EB42ABB218`.
- Base-lifecycle proof/raw:
  `A563AE6A8DB5E666D17C94277C0071AE98C879C3CEA703C7ED1AF25362AE4AD1` /
  `D08A8AB0E285816637BB75C3C65951B7AA502966B0B9F670403C401824AD20E5`.
- Generated-controller seam proof/raw:
  `465E0612BA72F5192ED47CB8BBD826F8691A5BDAB6F2CA74EA034DAC90A6043B` /
  `3A5899FEAA53E59A9A4F9C7E79A65845FBDC372F06CE99F8288D87EEBAA7BB81`.

## Exact native layout

PDB type ordinal 17267 is 24 bytes. Every field is one 32-bit pointer:

| Offset | Native field | Native type |
|---:|---|---|
| `+0x00` | `mController` | `StateGraph::HierarchicalController*` |
| `+0x04` | `mEntity` | `Sk8::Entity*` |
| `+0x08` | `mInstance` | `StateGraph::Instance*` |
| `+0x0c` | `mCameraMan` | `Iris::CameraMan*` |
| `+0x10` | `mBehaviourSource` | `Sk8::LivingWorld::IBehaviourSource*` |
| `+0x14` | `mBehaviourSink` | `Sk8::LivingWorld::IBehaviourSink*` |

`HierarchicalController` (PDB ordinal 17280) is 0x20 bytes: the inline Context
at `+0x00`, root-name hash at `+0x18`, and inner controller pointer at `+0x1c`.

MotionGraph construction at `0x82CEA060` seeds
`[null, entity, null, null, null, null]`, allocates the 0x20-byte wrapper,
copies all six words at `0x82CEA124..0x82CEA134`, and installs the wrapper's own
address in slot zero at `0x82CEA138`. ActionGraph construction at `0x82CEA340`
does the same through `0x82CEA3E0..0x82CEA3F4`. Actor construction passes the
same Entity pointer to both, but their controller wrappers remain distinct.

TransferableBehaviour `Process` at `0x82CF0A40` starts from six null slots,
copies them at `0x82CF0AC0..0x82CF0AD0`, then installs the new wrapper identity
at `0x82CF0AD4`. Its copy path at `0x82CF3B38` creates another independent
wrapper and later transfers only BehaviourSource from the original.

## Callback copy and instance overlay

The generic dynamic controller gives Conditions its original Context as a
const reference (`0x82CEF0F8`) and gives transition Hooks the original mutable
Context (`0x82CEF938`). Hooks can therefore affect later callbacks.

Each Behaviour callback instead receives a fresh six-word stack copy followed
by replacement of copied `+0x08` with that active Behaviour's paired Instance:

| Callback path | Six-word copy | Instance overlay/call |
|---|---|---|
| Update | `0x82CEF8EC..0x82CEF8FC` | `0x82CEF900..0x82CEF91C` |
| EndAll End | `0x82CF3220..0x82CF3230` | `0x82CF3234..0x82CF3254` |
| Exit End | `0x82CF33CC..0x82CF33DC` | `0x82CF33E0..0x82CF3404` |
| Begin | `0x82CF520C..0x82CF521C` | `0x82CF5220..0x82CF523C` |

CreateInstance receives no Context. A null paired instance is copied through;
native entry does not reject it. Direct slot writes made by a Behaviour die
with its local copy, while writes through the copied pointer referents remain
visible. The two recovered direct mutation APIs write BehaviourSource at
`+0x10` (`0x82CEED30`) and current BehaviourSink at `+0x14` (`0x826FC770`).

Matched generated Action/Motion Begin and End template paths reproduce the
same six-copy/instance-overlay shape. Generated state-specific Update functions
remain outside the extracted proof packet, so no whole-generated-controller
copy claim is made.

## Semantic Rust boundary

The Rust record stores six distinct nullable generic handle types in native
logical order. `None` denotes a null native slot. A handle implements an
explicit shallow-copy contract meaning that the result denotes the same
referent or identity without gameplay effects; plain `Copy` tokens receive a
blanket implementation. The Context itself deliberately does not implement a
blanket `Clone` or `Copy`.

`DynamicStateGraphContext::copy_for_behaviour` explicitly shallow-copies each
slot. The active runtime instance remains the separate
`DynamicBehaviourCallbackRequest::instance` borrow, which semantically shadows
the copied base instance slot. No borrowed instance is stored in the owned
Context and no self-reference is formed.

Only the two recovered source/sink mutators are public. Controller, Entity,
base Instance, and CameraMan slots can be seeded at construction and observed,
but no unsupported mutation API is invented.

## Verification and nonclaims

Focused tests must freeze the native constant/slot table, all-null and mixed
records, six-slot shallow identity, discarded direct mutations to a Behaviour
copy, persistent shared-referent mutation, source/sink original mutation, and
base-instance preservation. Five focused tests, one compile-fail API test,
three independent post-implementation audits, the 1,905-test core suite and
three doc tests, formatting, and strict all-target core clippy pass.

| Source | SHA-256 |
|---|---|
| `state_graph_context.rs` | `53CE60BBA9A9CCF981349B2355E4198D2219301BD0AD4A9A1258FD1D36DF40F1` |
| `dynamic_state_graph_controller.rs` | `E0BD869E8AD3AAD46094C4233B550CB28756635467303E010D5A0A15AD152F7F` |
| `lib.rs` | `D6F590AA4CE0BF003B280EF28F6FC6CCCE56516AC58F374395E1D4F88D9E7402` |

- The Rust generic record is not `repr(C)` and has no asserted native size,
  pointer width, alignment, or bit-pattern equivalence.
- Handle validity, liveness, ownership, refcounting, thread behavior, and
  pointee APIs remain external.
- A controller handle is a semantic token, not a Rust self-reference or native
  address. Stable Actor/entity authority is not reconstructed here.
- This type is not yet installed in the owning MotionGraph session, which still
  uses `()` Context.
- No opaque leaf, generated state-specific Update, mounted graph, animation,
  input, physics, or gameplay frame is executed.
