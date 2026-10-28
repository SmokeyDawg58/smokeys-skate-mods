# Complete private `Sk8ComputeContactPoints` path

This checkpoint reconstructs the accepted semantic path of
`rw::collision::Sk8ComputeContactPoints` at
`0x827cce40..0x827cd1e0` for represented primitive types. It joins the
previously proven shape-pair geometry prefix to mandatory triangle fix-up and
the final `GPInstance::ContactPoints` pack. The result remains private and is
not authority to construct or append a ground `Contact`.

## Authority

- Xbox 360 executable SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`
- full PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`
- linker MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`
- normalized/raw function packet SHA-256:
  `3238491393527F13F05C228C126815B1940D22FA670260FA7781949E5C449E50`
- decoded VMX operand packet SHA-256:
  `D1D22AA41812F9A7F7DA7DD22437223EE3765423AF67BB6DC5F19BB785E5CCBC`
- complete triangle-fixup proof packet SHA-256:
  `4CCD7C7903ED82A87AE125F06E23516B1DB470313C832562FC262CAFE5A6B478`
- audited `narrow_phase.rs` source SHA-256:
  `DD26ABCEB79C43D95CFDA676727D5A7BCB3AC3880919A43C4B85B6CEE8E029B2`

The PDB fixes `GPInstance::ContactPoints` at 560 bytes: four `u32` tag
fields, a `u32` point count, a 16-byte normal, and sixteen 32-byte point
pairs. The preceding `rwc_FeatureIntersectionPrism` remains the separately
proven 544-byte internal record.

## Reconstructed behavior

The implementation preserves this executable order:

1. select the complete shape-pair separating-direction routine;
2. compute the contact limit as
   `(second.fatness + padding) + first.fatness` and reject only when the
   separation is strictly greater;
3. build both maximum features and their ordered intersection prism;
4. apply the one-point normal replacement and interval-gap orientation graph;
5. when the second primitive is a triangle, always call
   `FixUpTriangleResult` with `reverse=true`, both thresholds, and `isObject`;
6. pack tags, count, the bitwise-negated final normal, and the active point
   prefix after any accepted fix-up mutation.

The packed tag order is first volume, second volume, first user, second user.
For each active pair, native performs separate vector multiply then add or
subtract operations:

```text
first  = prism.first[i]  + final_internal_normal * first.fatness
second = prism.second[i] - final_internal_normal * second.fatness
normal = bitwise_sign_negate(final_internal_normal)
```

The separate operation boundary matters: it is not a fused multiply-add.
The typed Rust output exposes only the active prefix and retains exact integer
words and XYZ values. Its initialized inactive array tail is host storage, not
a claim about native caller scratch.

`drawContacts` is present in the PDB signature but is unread in this retail
body. It remains an explicit dead input rather than being assigned invented
debug behavior.

## Rejection and ownership chronology

Early separation or prism failure returns zero before the native output pack.
The accepted-only Rust seam returns `None` and exposes no caller-scratch model
for those routes.

Triangle fix-up rejection is different. Native still packs the complete
post-fix-up scratch at `0x827cd104..0x827cd1ac`, then forces only the returned
count to zero at `0x827cd1ac..0x827cd1c0`. A private computation type retains
that distinction for tests. The sole sibling-visible seam filters on the
returned count, so rejected geometry cannot acquire Contact-production
authority.

Negative or greater-than-sixteen prism counts are typed host refusals before
serialization. Valid internal builders cannot produce them; reproducing the
native out-of-bounds malformed-memory path is deliberately outside scope.

## Numeric boundary and discriminating tests

The `vmsum3fp128` reductions and `vrsqrtefp128` seeds remain injected at the
private implementation boundary. The normal host backend uses nested f32 FMA
and `sqrt().recip()` as explicit ordinary-finite policies. This checkpoint
does not claim exact Xenon estimate bits, vector W lanes, exceptional values,
denormal handling, NaN payloads, or FPSCR behavior.

Focused tests freeze:

- strict rejection versus exact padding equality and the one-ULP native
  fatness association discriminator;
- feature order, reversed edge/face array ownership, exact-zero estimate
  execution, the fused two-round Newton discriminator, and every interval
  normal winner/tie/gate outcome;
- mandatory reverse triangle fix-up, including internally packed rejected
  scratch followed by a zero public result;
- an accepted concave reverse fix-up whose rewritten normal and second-side
  points must reach final fatness packing;
- exact tag order, bitwise normal negation including signed zero, each
  primitive's own fatness and side sign, and the dead draw flag;
- literal inputs where separate multiply/add yields `0x4311195f` but fused
  arithmetic yields `0x43111960`, and separate multiply/subtract yields
  `0x3e62c63e` rather than fused `0x3e62c63f`;
- malformed negative and seventeen-point host refusals.

## Remaining ground-Contact boundary

`AssemblyTriangleIntersectionTester::RunIntersectionTests` at
`0x827cd1e0..0x827cd7c8` consumes the returned count, normal, and point pairs.
It does not use the scratch tags or scratch count to form final Contact tags.
The raw Run packet hashes to
`B4FC2DF4223001B753A1B702BDB9478DF281C4CD9108B1C925A40273018C3A8E`.

That downstream owner still must preserve triangle-major/volume-minor/point-
minor order, eligibility and inclusive AABB gates, source-shaped velocity
padding, reserve-before-composition/null-abort behavior, material comparison
semantics, the full kinematic basis, and four independent occurrence roles.
In particular, the two PPU table identities and the two rigid-body RF-row
bindings must not be collapsed. Until that compositor is reconstructed and
audited, this checkpoint is not a ground Contact producer or a simulation-
frame producer.
