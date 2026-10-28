# IK World line geometry and native traversal

Read-only connected-IDA author audit, 2026-09-08. This extends
[the dispatch packet](skeleton-ik-line-query-source.md) through the zero-fatness
clustered-mesh geometry used by the actual World query. It does not establish
live Ground/DMO publication or vehicle-query support. No Rust was changed and
no Cargo command was run for this packet.

## Authority and confidence

All intervals below are half-open native function intervals. Raw words, true
VMX128 operand extensions, scalar branch bits, and PDB layouts were checked.
In particular, printed VMX128 operands such as `v94` at `82aced04` actually
select VA62. Scalar BO/BI bits, not the convenient `ble/bge` spelling, decide
unordered behavior.

| Function | Interval | SHA-256 |
|---|---|---|
| `rw::collision::ThinTriangleLineSegIntersect` | `82acecf0..82acee64` | `8a517217ff2c22d15ef90bef474cec02e5413c2f93cf792d0e057bbdfd45587c` |
| `rw::collision::TriangleLineSegIntersect` | `82adf658..82adf804` | `4a0732cc1624cd8f8517f88772c64498e9fee495bd77d5dd047908f8526016ce` |
| Mesh `LineTest` specialization | `827c01a0..827c0428` | `78ca5b0163cfc53b4acfade7041d27ff449949cd696504388151f8f6ef4897d9` |
| Collection `LineTest` specialization | `827c1300..827c1544` | `50dec8cc0160fb7a85c43d55368c519bd6730ec78effc0a51841ee9d6742143a` |
| `LocalClusteredMeshLineTest` | `827c19c8..827c1b38` | `7ab2d9523b7902c6bc4d157d6a17f0f67a2daa2c942ae518f856299b309f907e` |
| `ClusteredMeshLineQueryIterator` constructor | `827bd740..827bd7c0` | `ead52a5bf6360408c2991c974fe7684ad1608059aed1bbda1e27672d466616e6` |
| Octree `LineQuery` constructor | `82adc128..82adc2cc` | `754b61e182c240ede366bdaee9d9ff5ebc51f7f633dbfe2e57252c867ad3da46` |
| Octree `LineQuery::ProcessNode` | `82adc2d0..82adc9c8` | `a764d12e2ea64829c645e7f33e1d55789c43c0f428e430e49cefd5c1ba0d0626` |
| Octree `LineQuery::GetNextInIntersectedNodes` | `827bb4a8..827bb560` | `54c7df36cc2b689301c7ab043998a421da5ef9d038e57b7c47f0689288eae879` |
| Octree `LineQuery::GetNext` | `827bd7c0..827bd8f0` | `33682bc1cb8145968cfb5f1d95a2d737b3e59c4fab2081f57624b795b53ab398` |
| `AABBoxTemplate::Transform` | `825e9480..825e9558` | `64e31963ea12949c319567726342523d55ad9f2782ebe3ab7940a84bf07fcb27` |
| Inbox-vector dynamic initializer | `82f7a710..82f7a748` | `36e5d1b4d3d18406c47596325354599b3f308f2117b886e270493bb30ca6bc34` |
| Child-mask dynamic initializer | `82f7a820..82f7a938` | `fb966b85e90fb31469a97db2f6b44333152536c2b3c22fcb33999dcd7a9eb553` |

KD constructor, `QueryRange[827bb9d8,827bbc74)`, `AALineClipper::Init`, and
full modern unit/edge construction reuse the accepted raw packets in
[kd-line-query.md](kd-line-query.md) and
[assembly-ground-unit-gp-construction.md](assembly-ground-unit-gp-construction.md).
Their ownership and immediate call sites were rechecked here. The modern
unit body is `82acfdd0..82ad0cc0`, hash
`38b4e0777f0aaa31e8953f2e35ece5fe16c1ca3085d49fec377fb32faf56ea6c`.

Confidence is **high** for the complete control, arithmetic, traversal and
publication graphs below. Exact native reciprocal/root estimate bits, VMX
reduction rounding, exceptional min/max selection, and fused-negative NaN /
signed-zero payload behavior remain explicit numeric backend boundaries.
There is no new runtime-Xenon line trace or original-world query corpus result.

## Thin intersection, outputs and full-four-lane arithmetic

The arguments after the output are start `S`, delta `D`, and ordered triangle
vertices `V0,V1,V2`; `D` is not an endpoint. All are native 16-byte vectors.
Define `mul/sub/add` as separately rounded f32 operations, `fma(a,b,c)` as
one fused operation, and `nmsub(a,b,c)` as the native fused `c-a*b` graph.
For full-four-lane cross, let `cycle([x,y,z,w])=[y,z,x,w]`:

```text
cross4(a,b) = cycle(nmsub(cycle(a), b, mul(a,cycle(b))))
```

This produces the familiar XYZ cross but W is
`nmsub(a.w,b.w,mul(a.w,b.w))`, a rounding residual or exceptional result,
not a justified canonical zero. Unknown input W remains unknown here.

The thin graph is:

```text
edge1 = V1 - V0
edge2 = V2 - V0
perpendicular = cross4(D, edge2)
determinant = dot3(edge1, perpendicular)
if !(determinant > bits(322bcc77)): miss

from_vertex = S - V0
lower = determinant * bits(b727c5ac)
u = dot3(from_vertex, perpendicular)
upper = determinant - lower
if u < lower or u > upper: miss

other_perpendicular = cross4(from_vertex, edge1)
v = dot3(D, other_perpendicular)
if v < lower: miss
if (v + u) > upper: miss

numerator = dot3(edge2, other_perpendicular)
output.line_parameter = numerator
if numerator < lower or numerator > upper: miss

inverse = f32(1 / determinant)       // scalar fdivs, NOT vrefp/Newton
parameter = numerator * inverse
output.line_parameter = parameter
u_scaled = u * inverse
v_scaled = v * inverse
output.position = fma(D, parameter, S)
output.volume_parameter = [u_scaled,v_scaled,+0,+0]
hit
```

`322bcc77` is the positive determinant tolerance; `b727c5ac` is the negative
edge/segment tolerance. Winding is one-sided: reversing vertex order reverses
the positive-determinant acceptance. The determinant gate at `82aced44` is
raw `40990114`, NOT_GT, so NaN is rejected. Subsequent rejection branches are
ordered LT/GT and let unordered numerators pass. Do not rewrite those gates
as an all-ordered interval containment predicate.

The numerator store at `82acedd4` precedes both segment-bound rejections.
Earlier misses leave all result fields unchanged. Late misses change only
line parameter. Success stores final line parameter at `82acee18`, position
at `82acee48`, then volume parameter at `82acee4c`. The latter is two scaled
coordinates and two explicit positive zeros, not three barycentric weights.
Neither volume pointer nor volume reference is initialized by this helper.

`TriangleLineSegIntersect` adds the two fatness arguments with scalar f32
addition at `82adf684`; the ordered zero-sum arm calls the thin helper.
Actual IK supplies positive-zero line fatness and generated GP fatness is
positive zero. The nonzero/unordered sum arm is a different fat-triangle
dependency and is outside this bounded zero-fatness implementation.

On thin success, the wrapper computes
`raw_normal=cross4(V0-V1,V0-V2)`; replacing both subtractions by their negated
counterparts may change exceptional and rounding behavior. It publishes this
raw full normal at `82adf6dc`, then calls dot3 on it, then a reciprocal-root
estimate. For each of two refinements:

```text
square = estimate * estimate
half = estimate * 0.5
residual = nmsub(squared_length, square, 1)
estimate = fma(half, residual, estimate)
```

The full normal is multiplied by the refined estimate and stored at
`82adf714`. There is no zero-length fallback. A checked numeric refusal must
preserve the preceding thin writes and raw-normal publication at its true
stage. Successful thin testing uses four dot3 services; successful wrapper
normalization adds one dot3 and one reciprocal-root service.

## Actual GP input, unit cursor and mesh nearest result

`ClusteredMeshData` is `0xc0`: volume pointer +0, mesh pointer +4, forward
matrix +10, inverse +50, local root AABB +90, owner +b0. A `GPTriangle` is
also `0xc0`, not the decompiler's misleading smaller stride. Its vertices
are +0/+20/+30, precomputed normal +10, user tag +88. The latter is
`(surface<<16)|group`; `lhz` at +88 on this big-endian target reads surface.

The mesh call has start/end/delta/model-query-box as distinct arguments and
the mesh occurrence supplies both placement matrices. It transforms start
and end with the inverse in translation/X/Y/Z FMA order for KD traversal.
Triangle tests still use the original WORLD start and WORLD delta. The modern
unit builder receives the MODEL query box and mandatory FORWARD matrix.

Every yielded KD range is consumed; this specialization never calls
`ClipEnd`, sorts ranges, or stops at a nearer hit. `QueryRange` returns the
current range again without traversal if processed count differs from total.
For a new range, encoded cluster/byte offset are split at 16 bits when maximum
cluster count <=65536, otherwise 20 bits. KD branch/leaf order is the already
accepted LIFO graph; an entire leaf count becomes one unit range.

At `827c02d4` the mesh calls the NEW member unit builder. It uses the returned
ENCODED UNIT BYTE SIZE, even for a culled or zero-primitive unit. Primitive
ordinals are then visited ascending, stride `0xc0`. After each unit:

```text
processed += 1
chunk_count += 1
next_byte = byte_offset + encoded_unit_size
negative = signed32(unit_data_size_u16 - next_byte - 1) < 0
if negative: byte_offset=0; cluster_index+=1
else: byte_offset=next_byte
```

The source loop compares `chunk_count < total - processed` using the UPDATED
processed value. It therefore publishes cursor state partway through large
ranges and re-enters `QueryRange`; for N initially remaining it processes
ceil(N/2) units in that chunk. Publication order is cluster index `827c03f4`,
byte offset `03f8`, processed count `03fc`, then `QueryRange` at `0404`.
Crucially, a range can continue into the NEXT CLUSTER. The existing formats
`visit_leaf_range_units` accepts only a single cluster and is insufficient.

Modern unit topology and complete numeric construction remain mandatory:
triangle/list-count-one yields `[p0,p1,p2]`; quad/list-count-two yields this
and `[p3,p2,p1]`; old-type-zero/list-count-zero produces none. List counts
above two publish a count without constructing native records and require a
typed host refusal. Local AABB culling precedes point transforms and numeric
work. Quad normal dots occur BOTH before BOTH root estimates, then their
refinements; explicit diagonal edge-cos follows both normals, then complete
T0 and T1 edge construction. Those services cannot be skipped merely because
the final line test reads only vertices and face normal.

For each wrapper hit the mesh clamps parameter using the actual fsel graph:

```text
lower = fsel(-parameter, +0, parameter)
clamped = fsel(1-lower, lower, 1)
```

`fsel(test,a,b)` selects a iff test>=0; unordered chooses b. Thus a NaN raw
parameter becomes clamped 1. A nearest replacement requires ordered
`clamped < nearest.parameter`. Ties keep the FIRST encountered primitive.
Replacement stores position at `827c0380`, GP normal `0384`, surface `0388`,
clamped parameter `038c`, owner `0390`. Position is copied from the thin
scratch with its UNCLAMPED parameter; never recompute it at clamped parameter.
The normal is the PRECOMPUTED GP normal, not the wrapper normal. Wrapper
normalization still executes before this choice. Every hit increments the
u32 count at `039c`, including non-nearest and tied hits.

## Collection routes, bounds and local-job publication

The collection stores ordered mesh records +0, count +4, optional octree +8.
It rejects a segment unless at least one ordered abs(end-start).XYZ exceeds
`SMALL_FLOAT_28`, bits `37800000` (2^-16). W is excluded by native lane
replacement. NaN in one axis does not pass, but another axis may pass.
The world query box is VMX min/max of endpoints expanded by `[r,r,r,+0]`.

Without an octree, records are scanned in ascending collection slot order.
For each, transform the query AABB by its inverse, then reject on any ordered
strict XYZ separation from the record's local AABB. Touching bounds survive;
unordered comparisons do not constitute separation. With an octree, visit its
returned entry order, transform the query box by that record's inverse, and
call mesh LineTest WITHOUT this additional record-AABB cull.

Non-null AABB transform `825e9480` must execute even for identity matrices:
`half=(max-min)*.5`, `center=(max+min)*.5`; transformed center is X FMA into P,
then Y FMA, then Z FMA. Half extent is separately rounded `abs(Y)*half.y`,
then `fma(abs(X),half.x,prior)`, then `fma(abs(Z),half.z,prior)`. Finally
store center-minus-extent then center-plus-extent, all four lanes. Input W is
not consumed; output W follows matrix lanes. Identity shortcuts change bits.
The null-matrix branch alone copies min then max verbatim.

`LocalClusteredMeshLineTest` atomically claims index old counter (incremented
value minus one), validates the result pointer, and constructs a LOCAL result:
position all +0, normal `[0,1,0,0]`, parameter MAX_FINITE `7f7fffff`, surface
0, owner 0, count 0. Job data holds octree/count/mesh-array/test-array at
+0/+4/+8/+c; test stride is `0x30` with start/end/radius/result pointer at
+0/+10/+20/+24. It calls collection LineTest using this local nearest state.
Only normal return publishes to result storage, in order position `1ae4`,
normal `1ae8`, surface `1aec`, parameter `1af0`, owner `1af4`, count `1af8`.
A geometric miss still publishes defaults and zero count; GetResult returns
null for zero count. A checked traversal/numeric failure is not a miss and
must not publish the incomplete local nearest as a successful batch result.

## Conditional octree line traversal, including initializer authority

This is the collection's generic non-null-spatial branch, not established
DMO activation. Parent's concurrent actual-owner audit finds DynamicPool
constructor `827caf20` explicitly stores null to collection.spacial at
`827caffc`, and BeginUpdate refills the ordered array. Unless a later writer
establishes a non-null tree, DMO/vehicle collections use the same no-octree
scan as Ground. The following complete dependency proof can remain deferred;
it must not force implementation of an unused path before live query closure.

The existing `LocalOctree` exposes actual extent, active/free node slots,
entries and bboxes. Those owners suffice for the traversal; a second spatial
tree is unnecessary. Native `LineQuery` layout is size `0x2880`: octree +0,
clipper +10, reciprocal padding +50, swap mask +60, 211 stack entries +70,
stack top +2800, nine results +2804, current result +2870, next entry +2874.
Stack entry size48 contains full parameter-space min/max, interval [lo,hi],
and node id. Result size12 contains interval and linked-list head.

Constructor padding is not just fatness: each lane is
`fma(max(abs(S),abs(E)),bits(358637bd),[r,r,r,+0])`, then the accepted
`AALineClipper::Init` graph runs with actual octree extent. Reciprocal padding
is `abs(clipper.padding*clipper.reciprocal)`. Direction swap bits are X=1,
Y=2,Z=4 for strict ordered negative delta. Root near/far vectors are VMX
min/max of `(extent.min-origin)*reciprocal` and
`(extent.max-origin)*reciprocal`. Root interval is [0,1], node0, stack top1;
current result=-1,next entry=ffff. There is no additional constructor root
clip comparable to the KD constructor's root rejection.

IDA static storage for the following globals is ZERO before dynamic
initialization; those zero bytes are not their live values. The audited
initializer `82f7a710` sets `rwc_OctreeInboxGenerationVector@8308d4f0` to
`[bits(becccccd),bits(becccccd),bits(becccccd),+0]`, i.e. XYZ -0.4.
Initializer `82f7a820` sets `kChildBBoxControl@8308d590` to eight bit masks:
mask i has all bits set in X/Y/Z iff i has bit1/2/4 respectively; W is zero.

ProcessNode pops the stack and resets current result to -1. If the node's
own stuck-entry head is non-ffff, it first appends that head with the parent's
interval. For parameter-space near N and far F:

```text
amount = (F-N) * inbox_vector
inner_near = N - amount
inner_far = F + amount
a = inner_near - reciprocal_padding
b = inner_far + reciprocal_padding
```

Define source scalar operations (with separately rounded subtraction):
`mn(x,y)=fsel(x-y,y,x)` and `mx(x,y)=fsel(x-y,x,y)`. For parent interval
[L,U], the eight logical child intervals are exactly:

| Logical child | Lower | Upper |
|---|---|---|
| 0 | L | mn(mn(mn(U,b.x),b.y),b.z) |
| 1 | mx(L,a.x) | mn(mn(U,b.y),b.z) |
| 2 | mx(L,a.y) | mn(mn(U,b.z),b.x) |
| 3 | mx(mx(L,a.x),a.y) | mn(U,b.z) |
| 4 | mx(L,a.z) | mn(mn(U,b.x),b.y) |
| 5 | mx(mx(L,a.z),a.x) | mn(U,b.y) |
| 6 | mx(mx(L,a.y),a.z) | mn(U,b.x) |
| 7 | mx(mx(mx(L,a.x),a.y),a.z) | U |

Logical children are considered ascending 0..7. Reject ONLY ordered lo>hi;
raw `82adc4c0=41990124` and its three unrolled copies are ordered GT.
Equality and NaN intervals are admitted at this stage. Physical child is
logical XOR swap mask. For a leaf child, append its non-ffff linked-list head
and interval to results. For a branch, push bounds obtained by bit-selecting
`[N,inner_near]` and `[inner_far,F]` using the LOGICAL child's mask, interval,
and physical node id. Results and branch stack are consumed LIFO, without
sorting. Node-stuck entries were appended first, so they are consumed after
that node's appended leaf lists but before later popped branch nodes.

GetNextInIntersectedNodes writes the selected entry id before checking the
ffff sentinel. It advances the actual entry.next link on each success. When
a list ends it decrements result index; when results end it pops/processes
stack nodes until a list exists or stack is empty. It does not deduplicate.

GetNext then clips each entry's ACTUAL bbox, expanded by clipper padding:
compute reciprocal-space min/max vectors; reduce far with mn(X,Y),then mn(Z),
near with mx(X,Y),then mx(Z); upper=mn(result.hi,far),
lower=mx(result.lo,near). Accept ONLY ordered lower<upper at `827bd8c0`
(`4198...`). Equality/NaN now reject. This is deliberately different from
ProcessNode's coarse admission predicate. Entry-list advancement already
happened even when precise entry clipping rejects.

## Minimal implementation split and real-owner requirements

1. Promote/extend the existing MODERN GP builder and selected-unit decoder;
   preserve the accepted graph, numeric batching and provenance, but retain
   full point/normal W. Local decoded XYZ suffices: forward transformation
   consumes only those XYZ and derives each world W from matrix columns.
   GP cross/normal W can then be derived from those actual transformed W.
   Do not make a second normalizer call to reconstruct it. Keep untouched
   source padding unrepresented. Add a range cursor that crosses cluster
   boundaries in the actual byte-size order and decodes only visited data.
2. Implement the thin helper and zero-fatness wrapper against mutable typed
   result scratch. Reuse `SkeletonRootVector` for known/unknown W or promote
   the existing semantic vector type; do not create guest registers. Expose
   dot3 and root estimate refusal at true stages. Scalar division is f32
   division, not a substituted reciprocal service. Keep wrapper normal work
   even though mesh nearest uses GP normal.
3. Implement this mesh specialization on borrowed occurrence data and mutable
   nearest/count, then ordered collection scan on the ACTUAL retained records.
   Add the conditional octree route only if a live publication writer enables
   it; an unexpected non-null tree can otherwise remain an explicit capability
   failure. Use owned query cursors, not copied world meshes.
   A candidate/result must carry occurrence slot, cluster index, unit byte
   offset, primitive ordinal, source owner and surface; pool generation or
   existing handles must prevent resolving a stale DMO occurrence.
4. Connect actual Ground and DMO published collections to the already proven
   batch acquire/aggregate/release path. Parent is separately auditing pool
   publication. Preserve Ground→DMO→Vehicle aggregation and first ties.
   Missing required collection/lifecycle/vehicle support remains a capability
   failure, never an empty collection. Geometry alone cannot close default7.

Current mismatches are concrete: formats `thin_segment_triangle_intersection`
uses a simplified arithmetic/normal graph, a determinant `<=` rejection that
differs for NaN, and no late numerator prefix. `segment_intersections` sorts.
The existing single-cluster range visitor cannot wrap. Modern GP construction
is private and XYZ-only; legacy trajectory helpers are not exact substitutes.
`LocalOctree` contains lifecycle/storage but no source line traversal yet.
Its public read views already expose the necessary authoritative data.

The smallest coherent deliverable is exact Ground plus exact DynamicPool
geometry over those actual published owners, followed by real World batch
integration. The current constructor evidence makes both ordered-array scans;
the generic octree graph need not delay this live scope. An arbitrary
triangle-list raycast adapter would omit observable source work.

## Focused validation required before acceptance

- Back/front winding; determinant threshold bits/tie/NaN; tolerant u/v/end
  acceptance; each early and late miss prefix; successful position/full W and
  `[u,v,0,0]`; numeric refusal after raw-normal publication.
- Frozen FMA discriminators for cross and position; noncanonical/unknown W;
  no degenerate wrapper-normal shortcut; wrapper normal different from GP
  normal; unclamped position retained with clamped parameter including NaN.
- Modern triangle/quad/list dispatch and full service order; cull before
  callbacks; cross-cluster range continuation; resumed chunk order and late
  malformed-unit refusal; duplicate ranges and ties with full provenance.
- Identity AABB center/half rounding; strict touching cull; short segment
  exact threshold; slot ordering; every-hit count versus nearest replacement.
- Octree initialized -0.4/mask table, all direction swaps, leaf/stuck/branch
  order, coarse equality/NaN admission versus precise strict rejection,
  linked-list advancement on rejection, and actual lifecycle-created trees.
- Local job defaults/no-hit publication, failure preventing final result
  publication, and Ground/DMO aggregate tie ownership. Then original mounted
  world queries must exercise real pool records and decoded selected units.

Host checks for invalid indices, malformed byte ranges, cycles, free nodes,
or the 211/9 capacities must refuse at the corresponding read/write boundary.
These are safe-host limits, not claims that unchecked native memory faults
produce a geometric miss. Exact exceptional VMX backend behavior still needs
reference traces; do not silently describe host approximations as exact.
