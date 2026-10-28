# Assembly/ground clustered-unit GP construction

This checkpoint reconstructs the private semantic record built by
`rw::collision::ClusteredMeshCluster::UnitGetOverlappingGPInstances`. It sits
between the selected-unit visitor and the twenty-triangle tester cache. It is
not yet a public collision query or a Contact producer.

## Authority

- complete native function: `0x82acfdd0..0x82ad0cc0` (`0xef0` bytes);
- `rw::collision::ComputeEdgeCos`: `0x827c81c0..0x827c8378`;
- executable/PDB/MAP SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`,
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`,
  and `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- complete GP-construction proof packet SHA-256:
  `7E56FBA59B0E24A69D8B696B51A3A2C44B5A2B6F73E5A33360F9BE00D3F77FE5`;
- edge-cos/packing proof packet SHA-256:
  `62F32FC7DC7229C8DA566CA9D45C23B13ECC2DC397225F5ADDFA879AAC9F9262`;
- full native body blob SHA-256:
  `38B4E0777F0AAA31E8953F2E35ECE5FE16C1CA3085D49FEC377FB32FAF56EA6C`;
- implementation SHA-256:
  `BE05D5A364F596B0C00FF32A0E78F3075ADE3E190A3180883B98DEE230D358F6`.

This current source hash includes the later old-overload split and the
serialized-order diagonal-call correction. The preceding checkpoint hash
`FC5E47D184A76EDE96302BDCAB8BAAD7DE830CB165A079279311BB6E92D47E7E`
is historical and contains the now-corrected GP-slot naming mistake.

The raw construction text hashes to
`C4E68318C6BCD4B289E2BF42661343FA64EB617DA115717563806992EE23A934`.
The complementary raw edge text hashes to
`BAD645E9D4000B33EEB7DB5A276ED2F86542A109772E58BAF97AA68A52EFFA99`.

## Typed boundary and chronology

The private builder accepts already-decompressed local XYZ vertices, a
mandatory forward transform, the model-space query box, the parsed raw unit,
and the cluster flags. Keeping decompression outside this function avoids
assigning meaning to native vector W lanes; the formats layer already preserves
the three proven compression modes and saturating unsigned-16 origin addition.

For a supported nonempty unit the builder validates the safe-host input,
resolves every referenced index, performs the inclusive local AABB test, and
returns an empty batch on strict separation before transform or numeric work.
Accepted points use the native fused order: translation plus X, then Y, then Z.

The construction order is:

1. T0 face normal; for a quad, T1 face normal;
2. for an explicit-edge quad only, the shared
   `ComputeEdgeCos(p0,p1,p2,p3)` call in serialized index order;
3. external edge decoding and exact flag packing;
4. complete T0 cyclic edge frame; then complete T1; and
5. return the fully initialized fixed batch, the typed counterpart of native
   count-last publication.

The source wrapper preserves cluster index, unit byte offset, primitive ordinal,
zero volume tag, and `(surface << 16) | group` user tag. `GpPrimitive` alone
cannot preserve those occurrence-qualified identities.

## Numeric graph

The face normal uses the separately rounded multiply/fused-subtract cross,
one three-product reduction, one reciprocal-root estimate, and exactly two
Newton refinements. It deliberately has no degenerate shortcut. Cyclic GP edge
slots are `normalize(p2-p0)`, `normalize(p1-p2)`, and `normalize(p0-p1)`.
Each uses one reciprocal-root estimate and one refinement; its corresponding
length uses a reciprocal estimate of that refined inverse length and two
reciprocal refinements.

`ComputeEdgeCos` writes convexity before its two minimum-normal guards. Equality
at squared length `0x00800000` returns `1.0`; NaN follows the estimate path.
The numerator is reduced only after both guards, both estimates are injected,
two Newton rounds are applied to each, and the final numerator products remain
two separately rounded multiplies.

The host backend uses ordinary `sqrt().recip()`/`recip()` as estimate seeds.
Reductions and estimate seeds remain an injected private boundary so reference
Xenon traces can later supply exact seed/result bits. The surrounding formulas,
branch directions, operation counts, and ordering are frozen; exceptional
zero/NaN/infinity payload parity is not claimed.

## Unit topology, flags, and safe-host limits

Modern triangle and type-3 count one emit `[p0,p1,p2]`. Quad and type-3 count
two emit T0 `[p0,p1,p2]` and T1 `[p3,p2,p1]`. Ordinary old-triangle type zero
and type-3 count zero emit nothing. Type-3 counts above two publish a raw count
but construct no records in native code, so the PC path returns a labeled
unsupported error rather than exposing uninitialized records.

Explicit serialized edges use the signed 32-bit `slw/extsw/fcfid/frsp`
denominator and `K=0x411de9e7`. The `n=28` negative-denominator seam is
preserved. Triangle stores natural external slots. Quad shares the computed
diagonal in slot one and uses the proven opposite-external vertex-disable bits.
Only cluster bit `0x10` is inherited. Without explicit edges, both flags are the
literal `0x1e0`, edge data are positive zero, and cluster one-sidedness is
ignored.

Malformed vector lengths, edge lengths, indices, and a missing explicit-quad
diagonal are typed host refusals. These checks are safety policy, not claims
about native invalid-memory behavior. Likewise, serialized exponents 29..31,
native untouched padding, W lanes, and the four method-pointer addresses are
not represented as portable semantic fields.

## Verification

Focused tests freeze:

- exact external scalar bits including denominators at exponents 27/28/29;
- every triangle and quad one-hot convex/vertex-disable mapping;
- the shared-diagonal slot and both synthetic-middle flag equations;
- literal no-edge flags, cluster masking, absent/half/full user tags, and
  source ordinals;
- old/list dispatch including type-3 count-one/count-two equivalence and the
  count-above-two refusal;
- convex orientation, degenerate and exact-threshold guard chronology, and
  estimate call counts for `ComputeEdgeCos`;
- fused affine order, inclusive cull with no rejected-path numeric work;
- T0/T1 face normals, cyclic directions, dimensions, counts, zero fatness,
  and both-normal-before-diagonal ordering.

The remaining integration boundary is feeding decompressed selected-cluster
vertices into this builder, then routing the completed records through the
twenty-triangle cache and the already reconstructed triangle fix-up/contact
append path. Final `ContactPoints` serialization and live body-role ownership
remain separate work.
