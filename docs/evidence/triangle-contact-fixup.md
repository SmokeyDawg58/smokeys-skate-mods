# Triangle contact fix-up

This checkpoint reconstructs the complete private
`rw::collision::FixUpTriangleResult` decision and mutation graph at
`0x82acb5d0..0x82acc050`, plus its three behaviorally relevant helpers. It
consumes the preceding private `Sk8ComputeContactPoints` candidate but still
does not publish a ground `Contact`. A later audited checkpoint now connects
this mutation to final `GPInstance::ContactPoints` packing; live Contact
composition remains separate.

## Authority

- Xbox 360 executable SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`
- full PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`
- linker MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`
- complete FixUp proof packet SHA-256:
  `4CCD7C7903ED82A87AE125F06E23516B1DB470313C832562FC262CAFE5A6B478`
- corrected top-level VMX decode SHA-256:
  `414D80C192E90870FEE52E509D0882BD501644AFE35B974582F29EDD74AF332E`
- helper raw/decode SHA-256:
  `969935CDD4D65396A3C7F391AB2C826C6454F90776EB0F26B8BEAF15FE8A6AD6`
  and
  `9F7DFE0C7891DF226B3B717741ABCD3D874242D28DE57FE87A2EB07C38C774C7`
- audited `narrow_phase.rs` source SHA-256:
  `755CE8E132FC31220EF8F3BA299D3749AA80475168365951698DBAB32194752E`

The PDB names the consumed flags:

| mask | name |
|---:|---|
| `0x010` | `FLAG_TRIANGLEONESIDED` |
| `0x020` | `FLAG_TRIANGLEEDGE0CONVEX` |
| `0x040` | `FLAG_TRIANGLEEDGE1CONVEX` |
| `0x080` | `FLAG_TRIANGLEEDGE2CONVEX` |
| `0x100` | `FLAG_TRIANGLEUSEEDGECOS` |
| `0x200` | `FLAG_TRIANGLEVERT0DISABLE` |
| `0x400` | `FLAG_TRIANGLEVERT1DISABLE` |
| `0x800` | `FLAG_TRIANGLEVERT2DISABLE` |

The native `FixupReason*` parameter is overwritten before use and is never
read or written. The semantic Rust function therefore omits it rather than
inventing an observable reason.

## Reconstructed behavior

The implementation snapshots the oriented contact normal and triangle face
normal, performs the exact `0x3f7ff62b` strict alignment gate, and only then
normalizes and classifies the tangential direction. Region values retain the
native bit composition: face `0`, edges `1/2/4`, and vertices `3/5/6`.

All three native policy tables are explicit:

- one-sided with edge cosines uses strict face rejection, the one-sided edge
  bending helper, ordered adjacent-edge tests, and the peculiar `isObject`
  bypass of a disabled vertex;
- two-sided with edge cosines selects `d` or bit-negated `d` for convex or
  concave edges, rejects disabled vertices unconditionally, and uses the full
  ordered edge test;
- without edge cosines it preserves the strict face/culling gates and returns
  raw edge masks `0x20`, `0x40`, or `0x80`, not normalized booleans.

The semantic edge mapping is deliberately separate from storage order:

| edge | direction slot | cosine slot | convex mask |
|---|---:|---:|---:|
| 01 | 2 | 0 | `0x20` |
| 12 | 1 | 1 | `0x40` |
| 20 | 0 | 2 | `0x80` |

Vertex tests short-circuit in native order: vertex 1 uses 01 then 12, vertex
0 uses 20 then 01, and vertex 2 uses 12 then 20. Their directions are
bit-negated before either edge helper.

`EdgeCosTest` and `EdgeCosTestFull` preserve the source-shaped fused edge
cross, strict side test, separately rounded projection subtraction, estimate
normalization without a zero guard, inclusive cosine comparison, and the
two-sided nonconvex comparison against bit-negated face normal.

`FixUpOneSidedTriangleEdge` preserves the direct convex/object route, the
strict binary64 `1e-5` object tolerance, the exact minimum clipping constant
`0x3a83126f`, and strict bend acceptance. On its sole mutating path it reduces
the contact/face dot before issuing the root estimate, writes the result
normal, then rewrites point pairs in ascending order with fused multiply-adds.
Reverse mode changes the second point array; forward mode changes the first.
Count, prism normal, and normal-override remain untouched.

The Rust typed prism has sixteen pairs. A malformed positive count above
sixteen returns `None` before mutation instead of reproducing native
out-of-bounds access. Valid internal prism builders cannot reach this explicit
host-safety deviation. The native full 128-bit W lanes are not represented by
the current `[f32; 3]` points, so only XYZ behavior is claimed here.

## Numeric boundary and tests

The private `TriangleFixupNumeric` boundary injects every `vmsum3fp128`
reduction and `vrsqrtefp128` seed. Two fused Newton rounds and all surrounding
scalar/vector operation order are represented directly. The normal host
backend remains an explicit ordinary-finite policy and does not claim Xenon
estimate bits, denormal/exception behavior, NaN payloads, or FPSCR effects.

Focused tests freeze:

- all 28 edge-cos/sidedness/region cells, plus raw-zero/raw-mask returns and
  each disabled vertex;
- strict alignment-cutoff entry at equality and adjacent ULPs, including the
  exact reduction/estimate/classifier chronology;
- distinct edge-cos slots across all three edge regions in both mode tables;
- object-only disabled-vertex bypass in the one-sided table;
- strict edge side, nonconvex `EdgeCosTest` rejection, full nonconvex testing,
  and estimate execution for a zero projected vector;
- the two f32 values straddling the binary64 object tolerance, bend-threshold
  equality, and the inclusive minimum-angle rejection;
- a two-pair reverse concave correction with signed-zero result-normal bits,
  untouched metadata/unused slots, exact point-array ownership, and dot-before-
  estimate event order;
- count zero, negative count, and the over-capacity host safety route;
- an owning acceptance seam that consumes rejected candidates instead of
  allowing incomplete contact geometry to escape.

## Remaining contact boundary

This completes the mandatory triangle fix-up itself at exact/high confidence
for its branch graph, flags, ordinary-finite XYZ operation order, integer
returns, and mutation ownership. It is not yet an executable ground producer.
Clustered-mesh unit topology, quad siblings, flags/edge cosines, cache/rip, and
the final ContactPoints pack are now reconstructed by their downstream private
checkpoints. The remaining integration must preserve the complete
`RunIntersectionTests` graph and four occurrence-qualified body roles before
Contact append authority is granted.
