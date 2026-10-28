# Assembly old free-overload unit GP construction

This checkpoint separates the clustered-unit GP builder actually called by
the Assembly/ground ripper from the newer member overload reconstructed in the
preceding checkpoint. The two bodies share semantic triangle construction, but
their dispatch, culling boundary, vertex source, volume input, and one scalar
chronology are not interchangeable.

## Authority

- Assembly caller: `0x827cd7c8..0x827cd960`;
- old free `UnitGetOverlappingGPInstances` overload:
  `0x827c98f0..0x827ca7b8` (`0xec8` bytes);
- newer member overload retained for comparison:
  `0x82acfdd0..0x82ad0cc0`;
- old raw extraction SHA-256:
  `CD4AF5ED53374ABD55EFB2CB045EC901063A70A7C88C4762095DFDE09BFDD9B8`;
- old native body SHA-256:
  `0FBC00D2320B9D2217D9314923C6CEC25399D3B11A6F6FC50DA756F6FBEA122D`;
- executable/PDB/MAP SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`,
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`,
  and `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- implementation SHA-256:
  `BE05D5A364F596B0C00FF32A0E78F3075ADE3E190A3180883B98DEE230D358F6`.

The PDB signature supplies the old body's explicit inputs: cluster, unit byte
offset, transform, destination records, output count, group/surface field
sizes, compression granularity, cluster flags, and `volumeRef`. Assembly passes
zero for `volumeRef`, but the private typed builder preserves the parameter and
copies it into every emitted sibling.

## Proven differences from the newer overload

The old body has no model-space query box and therefore performs no local
whole-unit pre-cull. It reads only vertices referenced by the current unit, in
serialized index order, expands their cluster compression, and then applies
the mandatory affine transform. Compression mode 1 widens unsigned 16-bit
components, performs signed saturating origin addition, converts signed integer
to float, and multiplies by granularity. Mode 2 converts signed 32-bit absolute
quantized values and multiplies by granularity without origin addition. Mode 0
loads uncompressed XYZ. The formats parser's refusal of other mode values is
explicit host safety policy.

Dispatch computes the native primitive count as zero, one, two, or the type-3
list count. Only counts one and two enter initialized construction. Type zero,
type-3 count zero, and type-3 counts above two reach the native
FailAssert/DebugBreak boundary. The PC builder returns a typed
`AssemblyNativeAssertedPrimitiveCount` refusal before vertex or numeric work;
it does not expose the apparent but unreachable old-triangle arm or assume that
native execution resumes after the break. Type-3 counts one and two share the
triangle and quad arms respectively.

Both overloads retain triangle `[p0,p1,p2]` and quad siblings
`[p0,p1,p2]` / `[p3,p2,p1]`, identical final edge slots and flags, packed
`(surface << 16) | group`, and count-last publication. Their explicit quad
diagonal calls both pass serialized order `p0,p1,p2,p3`; the earlier
`p0,p2,p1,p3` notation mixed GP-output slots with serialized-index names and
has been corrected. Their explicit quad
external-edge division calls differ: the newer member body requests
`e3,e0,e2,e1`, while the old Assembly body requests `e2,e3,e0,e1`. Stores put
the decoded results back into their natural geometric slots.

An unknown raw unit type above three takes a distinct earlier native assertion
route. `UnitType` cannot represent it because the formats parser validates the
two-bit field before this typed seam. Native treats any group/surface ID size
other than two as a one-byte field and any compression mode other than one or
two as uncompressed; the formats layer deliberately admits only ID sizes one
or two and compression modes zero/one/two. Those refusals are host safety
policy, not recovered native error behavior.

## Numeric chronology

For a quad, both face reductions and both reciprocal-root seeds precede the
optional shared-diagonal computation. Each completed record then owns its edge
frame. T0 reduces and seeds cyclic slots `0,1,2`, then requests dimension
reciprocal seeds in order `1,0,2`. T1 reduces and seeds cyclic slots `0,2,1`,
then requests dimension reciprocal seeds in order `0,1,2`. The T1 reduction
order is shared by both native overloads and corrects an under-discriminated
assumption in the preceding implementation.

The injected numeric boundary still represents Xenon three-product reductions
and reciprocal/reciprocal-root estimate seeds. Host nested FMA,
`sqrt().recip()`, and `recip()` are ordinary-finite execution policy. The
surrounding fused affine/cross/Newton formulas and callback order are frozen;
exceptional Xenon/FPSCR bit parity remains unclaimed until trace evidence is
available.

## Verification

Twenty-nine focused topology tests pass. New discriminators freeze:

- native-asserted type-zero, empty-list, and long-list paths before any numeric
  callback;
- direct/list count-one and count-two semantic equivalence;
- absence of a local query cull using far-transformed geometry;
- `volumeRef`, packed user tag, unit byte offset, cluster identity, and both
  quad ordinals;
- old `e2,e3,e0,e1` versus new `e3,e0,e2,e1` scalar-call order while proving
  identical final slots and their position after the diagonal calculation but
  before either GP edge frame;
- serialized `p0,p1,p2,p3` diagonal-call order in both overloads using a
  noncoplanar convexity/flag discriminator;
- exact cluster-one-sided flag inheritance and permuted referenced-index
  routing;
- unsigned-16 saturating-origin and signed-32 expansion followed by affine
  transform; and
- a non-symmetric T1 whose reduction vectors and seed values distinguish
  cyclic order `0,2,1` from `0,1,2`.

This remains a private builder, not a Contact producer. The next boundary owns
the selected-range rip loop: visit units, call this old overload, execute the
world-space triangle/AABB predicate before the spew read, feed accepted records
through one persistent twenty-triangle cache, advance native unit size, and
perform one consuming terminal drain. That rip owner and final ContactPoints
packing are now reconstructed downstream; live body-role composition remains
separate work.

Native Rip ignores the builder's scalar encoded-size return and calls
`DoSizeCalculation` only after the emitted-primitive loop. The Rust selected
visitor advances by the parsed unit size after its callback, preserving that
observable ordering without treating the builder return as traversal authority.
