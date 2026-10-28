# RigidBody DynamicUpdate

Date: 2026-09-05 (Europe/Helsinki)

This checkpoint reconstructs the ordinary-finite host graph of
`rw::physics::RigidBody::DynamicUpdate` at `0x82ae4488..0x82ae4904` and wires
it to Active bodies in the semantic Simulation pool. It is body integration,
not a complete constrained Simulation tick: later checkpoints have reconstructed
bounded Contact/Joint/Drive Jacobian paths and one runtime-counted prepared
island interleave, but cross-island ownership, the all-island barrier, phase
consumption, non-network Contact production, and manager job scheduling remain
separate.

## Provenance

Reference artifacts remain the matching debug XEX/PDB/MAP set recorded in the
project research log. Reproducible packet artifacts are in
`world_probe_temp`:

| Artifact | SHA-256 |
| --- | --- |
| `dynamic_update_evidence.md` | `11B7B9BB2E0FF205BE7D21C687D3D94DD2CD3C4915162F1A17A07C7192881861` |
| `dynamic_update_raw.txt` | `5AB40B3AB38CE40FC6F7C9046C8FFA609E7F6510ACC47CDA6836A35F8CC6D592` |
| `dynamic_update_decode.txt` | `B55F250F1B7A7CCA1D1CB61DD72F0ECA9E196EEFC8DD1672D5CBCD461C54B7A0` |
| `dynamic_update_oracle.rs` | `13238E404EA3A3F275546F9BBA21D69A3DCA31FB6AA7CA87147E4847872E0E1B` |
| `dynamic_update_oracle.txt` | `994DC6B7C247BA7DA31D90D6EF5A7AC04CEFD13C12B31C89511F5FCC3DA35D7A` |
| `dynamic_update_preflight_consume_safety_api.md` | `68EBDAE27730F269E85388D8DF077350227D7C86039C6EDEDDC830DB9E16DF6E` |

IDA 9.3 misdecodes the VMX128 `VA` extension bits in this image. The corrected
decoder matches the raw encoding and Xenia's VX128 format. In particular,
`0x82ae474c` is `frequency - angular_drag`, not an unexplained high-register
operand. Independent raw review also corrected the basis permutation at
`0x82ae45d8/0x82ae4634` to full-vector `YZXX`; its fourth lane is discarded,
but the source and evidence no longer make a false `YZXW` claim.

## Recovered graph

For RF vectors `d0..d3`, native dependency roles are exact:

- `d0` contributes to the integrated linear step, COM, and persisted velocity;
- `d1` is a direct COM-only correction;
- `d2` contributes to orientation and persisted omega;
- `d3` is an orientation-only correction.

The function then performs the recovered quaternion step and unguarded
two-round reciprocal-square-root normalization, rebuilds Ri/Up/At with the
native permutation/FMA graph, and recomputes the rotated inverse-inertia words.
The host reuses only the already-proven rotated-inertia helper; it does not
substitute AddRigidBody's unrelated matrix-to-quaternion conversion.

Angular drag is read before linear drag. Each persisted candidate is multiplied
by `max(frequency - stored_drag, +0)` using the native ordered strict compare.
Angular speed is clamped before linear speed, and only when squared magnitude
is strictly greater than the corresponding squared maximum. The native
ratio/estimate/two-Newton-round calculation occurs before its exact-zero scale
selection; the host source preserves that dataflow.

Kinematic value is the fused combination of post-clamp angular and linear
squared magnitudes. The cooldown graph preserves wrapping increment and
unsigned cap. Raw branch bits also establish unordered routing: unordered
speed comparisons skip clamping, unordered energy-versus-minimum resets the
cooldown to zero, and unordered energy-versus-old-energy increments if the
low-energy path was reached.

Observable terminal order is velocity, omega, RF clears `d3,d2,d1,d0`, current
Simulation gravity restoration, then torque clearing. The semantic host layout
keeps native packed fourth words as typed adjacent state, so RF association,
links, Simulation/inertia ownership, tag, inverse mass, state, kinematic value,
and cooldown retain the same overwrite/preservation roles without pretending
they are floating vector W lanes.

## Host policy and remaining boundary

Native DynamicUpdate unconditionally dereferences Inertia. The host prepares a
non-cloneable crate-private phase that exclusively borrows the body pool,
validates the complete retained structure, reserves the whole Active plan, and
copies each Active occurrence and its Inertia in exact right-link order. It also
copies the current timestep, frequency, gravity, cooldown, and minimum-energy
configuration. Reservation failure and later invalid Inertia therefore precede
every RF or body effect. This whole-ring failure atomicity is deliberate
safe-host policy; native would already have mutated earlier bodies before a
later invalid pointer faulted.

While the phase exists, a solver can acquire only its restricted RF-delta
transaction; no pool reference, row index, or general body mutation capability
escapes. Consuming the phase integrates the frozen plan and configuration,
returns no error, and performs no validation, allocation, or locking. Dropping
an unconsumed phase is inert. The existing one-shot pool entry point is a
prepare-then-consume compatibility wrapper. Later constraint checkpoints borrow
this phase, and one bounded checkpoint now reconstructs a prepared island's
runtime-counted Contact -> Joint -> Drive schedule. The all-island owner and
this phase's final consumption remain open.

`SimulationInertia` now carries all seven consumed semantic values. It remains
an ordinary Rust value, not a `repr(C)` byte-layout claim. The current retained
`Arc` is immutable; live post-attachment drag writers will require a controlled
shared-mutable owner in a later slice, without exposing arbitrary writes that
could desynchronize cached inverse inertia.

The pool begins with deterministic zero timestep/frequency as an explicit
unstarted host policy. `begin_dynamic_update_frame` preserves StartFrame's
ordinary-finite store/divide/store graph; Initialize's exact minimum energy
`1.0`, maximum cooldown `30`, and gravity `[0,-600,0]` remain configurable at
their existing semantic seam.

Exact Xenon `vrsqrtefp128` seeds, `vmsum3fp128`/`vmsum4fp128` reduction
rounding, scalar exceptional division, denormal flushing, NaN payloads,
infinities, signed-zero details, and FPSCR effects remain hardware boundaries.
The Rust graph uses the project's explicit nested host reductions and
`sqrt().recip()` seed policy and therefore does not claim Xenon bit identity at
those boundaries.

## Verification

Direct tests freeze three complete ordinary-finite oracle outputs. The mixed
fixture discriminates defused FMA, swapped `d0/d1`, swapped `d2/d3`, textbook
quaternion convention, and textbook basis formulas. Separate tests freeze the
strict clamp predicate and both proven unordered cooldown routes.

Pool tests prove later-body missing-Inertia rejection has zero earlier-body or
RF effects; corrupt structure is rejected before reservation; injected
reservation failure and a false-success insufficient-capacity allocator are
effect-free; and remove/re-add order, Inertia, and frame configuration are
copied by preparation. A deliberately mutated private plan discriminates both
consume order and frozen configuration/Inertia from pool rereads. A stale body-B
observation at a later pair is rejected before every effect in that RF
transaction, and dropping a prepared phase neither integrates bodies nor clears
RF.

The integration fixtures additionally prove Active right-link traversal
excludes Static and Frozen; all four delta roles remain distinct; every RF lane
clears; current timestep/frequency, minimum energy, maximum cooldown, and
gravity are sampled; packed-adjacent semantic fields survive; and all five
retained Inertia controls preserve their exact bits without changing
AddRigidBody's tensor/mass-only arithmetic.
