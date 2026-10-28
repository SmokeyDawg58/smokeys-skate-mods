# Active IK line-query dependency map

Root read-only IDA audit, 2026-09-08. The virtual dispatch, query ownership and
top-level batch selection below are identified. The lower geometric/traversal
graphs are dependencies, not yet implemented or fully audited. This packet
does not justify a generic raycast or an always-miss contact implementation.

## Actual dispatch and world ownership

`SkeletonIK::ContactLineTest [82b49760,82b4994c)` hashes
`94b2b2f4e4c6ce5f67477e72d9254903bf292be49064f10ef855286406ec0ecd`.
It loads DataIn island at49880 and calls virtual slot+44 at498e4/498ec.
Actual Island vtable820ffa10 has BOTH +44 and+48 equal827b2538, named
`Island::LineCollisionTestWithVehicles`. Thus the differently named virtual
entries share this body; do not infer different filtering from symbol spelling.
The body827b2538..827b2540 forwards through its actual mWorld to World query.
Hash: `8a663744e90103ee9703716ddcdcafd0523e99f70362ad7b46d7397d64beba25`.

`World::LineCollisionTest [827ab338,827ab448)` hashes
`733937eac5939116038bfa86ea9f32abd9650b1be8f6b15d511ddcdca9d69362`.
It acquires a line batch, calls Begin, AddTest, End, Sync, then GetResult(0).
It copies a non-null result into a non-null output in order: full position,
full normal, lineParam, surfaceId, owner. It then releases the batch and returns
whether a result existed. The AddTest boolean is not used as a miss result;
do not silently replace acquisition/job/capability failure with a geometric miss.

World's label-only AcquireLineBatch827b1a58 explicitly supplies flags7 at1a5c
(`BF_Default`). The flags overload827ab918 forwards to the WORLD'S retained
simulation island, resetting flags to default if visual-island mode is enabled.
It does not query only the island originally copied into IK DataIn.

`Island::AcquireLineBatch [827b8728,827b885c)` hashes
`637034e117d7e24758085d18d2fa16a7886889ac835601da8fdc4c8555ef65bb`.
It asserts nonzero flags, then acquires in order:

1. Ground collection batch when bit1 is set.
2. DMO collection batch when bit2 is set.
3. Vehicle collection batch when Island flags are FLAG_Both and bit4 is set.

One selected batch returns directly; otherwise an AggregateLineBatch retains
the ordered children. Missing source pools or host query services must remain
explicit failures, not fabricated empty dynamic collections.

Aggregate AddTest827bbe50 calls every child and ANDs their booleans without
short circuiting. GetResult[827bbec8,827bbf60) hashes
`80df1fa4959e1a6944c21d544df14b5650e8d682ad9da560d6dc28b2b31a399c`.
It visits children in retained order, checks index against each child's count,
and retains the first available result unless a later lineParam is strictly
ordered less. Raw827bbf40 word40980008 is LT-not-set, including unordered;
ties and unordered comparisons preserve the earlier result.

## IK query inputs and stores

The source uses processed EffectiveTransform+80, current part-to-world target
P, and late sideways-offset cvar. It projects the target-relative position off
effective X, adds a signed sideways offset (+ for foot0, - for foot1), then
constructs a segment using negative effective Y: start 0.3 above the point,
end 0.7 below it. These are source arithmetic graphs needing corrected VMX
operand auditing before implementation; the decompiler's register aliases are
not reliable equations. Debug-overlay availability is checked before query
when the draw cvar is enabled.

The actual call uses label `SkeletonIK::ContactLineTest`, result scratch, and
fatness positive zero. Hit stores full contact position at49914, then VALID
at4991c; miss writes NOT_VALID at49930 without overwriting contact position.
The caller UpdateContact separately clears contact status before this call;
that prior store must survive an unavailable query. DataIn merely retained
the island identity earlier and must not have refused for absent line support.

## Original lower-path survey (superseded by reviewed implementation)

2026-09-08: the table below records the initial survey, not current gaps.
Accepted47724 implements the actual static-owner child through collection,
KD/range cursor, modern GP and zero-fatness intersection. Contact segment
arithmetic was separately raw-audited and implemented; source end=start+down
rounding is preserved. See [integration](world-line-runtime-integration.md),
[geometry](skeleton-ik-world-line-geometry-source.md) and
[contact](skeleton-ik-contact-source.md). Full World ownership remains open.

ClusteredMeshLineBatch::AddTest[827bd9a0,827bda68) hashes
`ead6aebf2a92ade27a9596464993b095e8d5175e307beb479ff69a734d697008`.
It refuses exactly at capacity, obtains the next test, requires result storage,
initializes nearest lineParam to maximum finite f32, normal to J, position to
zero, surface/owner/count to zero, then copies full start/end and fatness.
GetResult[827bda68,827bda9c), hash
`04b641f5d1a7f81fec76e24e4cc2dd58bee55042d98c60d5709227e6e92f1770`,
returns null only when the stored hit count is zero.

| Dependency | Actual span / address | Recovery status |
|---|---|---|
| LocalClusteredMeshLineTest | 827c19c8..827c1b38 | Job-to-query/result copy identified; aliases need raw argument audit |
| Collection LineTest template | 827c1300..827c1544 | Octree query or ordered collection scan, transformed bounds |
| Mesh LineTest template | 827c01a0..827c0428 | Line iterator, original unit expansion, triangle intersection and nearest selection |
| ClusteredMeshLineQueryIterator ctor | 827bd740 | Needs source traversal audit |
| QueryRange | 827bb9d8 | Needs source range/resume audit |
| TriangleLineSegIntersect | 82adf658 | Needs actual zero-fatness branch and numeric dependency audit |

Hashes respectively for the first three table spans:
`7ab2d9523b7902c6bc4d157d6a17f0f67a2daa2c942ae518f856299b309f907e`,
`50dec8cc0160fb7a85c43d55368c519bd6730ec78effc0a51841ee9d6742143a`,
`78ca5b0163cfc53b4acfade7041d27ff449949cd696504388151f8f6ef4897d9`.

The existing Rust terrain/cluster decoding and original unit expansion may be
reused after matching their consumer contracts. No matching live line-query
adapter currently exists. A complete implementation must preserve actual
world/pool ownership, traversal and tie order, transformed triangle provenance,
surface/owner results, and late source failure boundaries. Original terrain
and eventual dynamic-object/vehicle collections require corpus checks; a
static-only service must not claim default-query completeness.

Confidence high for the verified virtual/owner/batch call graph and aggregate
selection, medium for this lower dependency survey. No code, Cargo or reference
runtime query trace was produced by this read-only audit.
