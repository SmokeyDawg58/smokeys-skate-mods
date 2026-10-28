# Assembly/ground Run velocity-padding value graph

This checkpoint begins the private
`AssemblyTriangleIntersectionTester::RunIntersectionTests` owner with its
effective Contact-padding computation at `0x827cd444..0x827cd4cc`. It freezes
the exact effective-padding scalar passed to `Sk8ComputeContactPoints`,
including the represented reduction inputs, without claiming the surrounding
live volume traversal, stack writes, argument setup, Contact reservation, or
append effects.

## Authority

- Xbox 360 executable SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`
- full PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`
- linker MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`
- complete Run raw packet SHA-256:
  `B4FC2DF4223001B753A1B702BDB9478DF281C4CD9108B1C925A40273018C3A8E`
- full `0x827cd1e0..0x827cd7c8` native span SHA-256:
  `81C7E79EA8038EC1A61F45F03A3DC203A18B7742EFCA214EBA493C83B9FE4983`
- audited `simulation_assembly_ground_run.rs` source SHA-256:
  `0826C8EF587313EECD8F3CAEF8EF5267589E695D080EB962322FB985864F2936`

## Exact value chronology

Native loads the literal timestep `0x3c888889`. It bitwise-negates the cached
triangle face normal first at `0x827cd498`, then separately rounds every
represented component of `volume_velocity * timestep` at `0x827cd4ac`.
`vmsum3fp128` receives those two operands in this order at `0x827cd4b0`:

```text
bit_negated_face_normal
rounded_volume_velocity_times_0x3c888889
```

The three-component reduction is injected as a numeric backend. The normal
host path does not claim Xenon reduction bits, and the current typed vectors do
not represent native W lanes. Returning an error and no value is an explicit
host/test seam; native numeric execution has no analogous failure route.

The scalar tail is preserved literally rather than simplified to a clamp:

```text
negated_closing = bitwise_sign_negate(closing)
positive_closing = fsel(negated_closing, +0.0, closing)
remaining = generator_padding - positive_closing
limited = fsel(remaining, positive_closing, generator_padding)
effective = limited + volume_padding
```

For the operand ordering above, `fsel` chooses its nonnegative value for either
sign of zero and its other value for a negative or unordered selector. Thus
ordinary finite `closing` and finite positive generator padding are
algebraically
`volume_padding + min(max(closing, 0), generator_padding)`, but that shorthand
is not the parity contract. The bitwise `fneg`, separate subtraction, both
selection stages, and final standalone addition preserve signed-zero and NaN
choice behavior that host `min`, `max`, or `clamp` could change.

## Discriminating tests

Three focused tests freeze:

- literal timestep bits, normal sign-bit XOR including signed zero and a quiet
  NaN, componentwise pre-scaling, callback operand order, and the
  `0x38fdb969` pre-scale value that rejects reassociation;
- both signed-zero closing inputs, negative/positive routes, cap equality and
  adjacent ULPs, both NaN signs, a NaN generator-padding payload, and a
  negative-zero generator-padding case;
- the finite positive/negative orientation examples and cap-before-final-add
  rule, including `.75 + .25 == 1.0` rather than clamping the combined value.

The focused suite also passes in an optimized release build. Exact Xenon
`vmsum3fp128` results, vector W lanes, denormal/exception behavior, NaN payload
arithmetic outside the explicit selects, and FPSCR remain open.

## Remaining Run boundary

This module is private and imports no body, Contact, Sk8, reservation, or
append capability. The remaining owner must still preserve native triangle-
major/volume-minor/point-minor iteration, lazy eligibility/culling, live body
and table occurrence binding, the call into the complete Sk8 path, reserve-
before-composition/null-abort behavior, and final temporary-buffer publication.
The reserve-first write capability, pure sixteen-row compositor, and complete
Sk8 result are separate proven but unconnected input boundaries. See
`docs/evidence/assembly-ground-contact-write-port.md`.
