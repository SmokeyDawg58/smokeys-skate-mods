# Native KD AABB and line traversal

This note records the bounded `rw::collision::KDTree` query milestone. The
implementation is based on the supplied Skate 2 debug/devkit executable and
its matching PDB/MAP, then checked independently across every BAM Simulation
collision tree. It proves ordinary finite-input structure and ordering; it is
not a bit-exact Xenon exceptional-floating-point claim.

## Native inputs

| Input | SHA-256 |
|---|---|
| `sk82_na_m.xex` | `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2` |
| matching PDB | `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0` |
| matching MAP | `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88` |

The final native evidence packet is
`world_probe_temp\kd_line_clipper_evidence.md`, SHA-256
`B026B5F98880B83EB64B02C0CE18912172DBC97B2EB04B8353BBDE2B4459DC25`.
Its raw PPC/PDB/MAP extraction has SHA-256
`847CFEA35CAEE0FB556B7E9DE28320781CE8F0C80C5713024B278B4B07C38982`;
the extraction script has SHA-256
`D3E217D617353E14B4A51292AFBD78098232202FB283C67381CD6EB7F75353A8`.
The arithmetic was recovered from raw instructions because IDA misrenders
some VX128 high-register operands.

The follow-up extraction that attributes the gameplay collection provider,
per-mesh query, primitive normal, and all native `ClipEnd` callers is
`world_probe_temp\outer_collision_line_followup_raw.txt`,
SHA-256
`A57DED62314111B33A8E570FE6CE328198BAFBF8D852CE4A49AA720704E456DF`.
Its extraction script is
`world_probe_temp\dump_outer_collision_line_followup.py`,
SHA-256
`E165689F63F44F612EA0D750A5D9537FDAB86CFF4373A4F02BA52DD6493F9717`.

## Functions and layouts

| Address | Function / role |
|---:|---|
| `0x827ccc48` | `ClusteredMeshBBoxQuery::ProcessBranchNode`, AABB child tests |
| `0x827ccd80` | `ClusteredMeshBBoxQuery::GetNext`, stateful AABB traversal |
| `0x827bb850` | `ClusteredMeshBBoxQueryIterator::QueryRange`, encoded AABB leaf ranges |
| `0x827bd340..0x827bd480` | `AALineClipper::Init` |
| `0x827bd480..0x827bd650` | `KDTreeLineQuery` constructor and root clip |
| `0x827bd740` | `ClusteredMeshLineQueryIterator` constructor; calls the KD query at `0x827bd768` |
| `0x827bb9d8..0x827bbc74` | `ClusteredMeshLineQueryIterator::QueryRange`, inlined line traversal |
| `0x82ac8b70..0x82ac8be0` | `KDTreeLineQuery::ClipEnd` |
| `0x82ad9d78..0x82ada140` | `KDTreeMappedArray::LineIntersectionQueryThis` |
| `0x82adf808..0x82ae0058` | `ClusteredMesh::LineIntersectionQueryThis` and hit-driven clipping |
| `0x827b15f0`, `0x827b19b0` | `World::World`; ground-pool construction and capacity 128 |
| `0x827c8c68..0x827c8e5c` | `GroundPool::GroundPool`; collection initialization and `0xc0 * capacity` allocation |
| `0x827c8378..0x827c84dc` | `ClusteredMeshData::Reset`; transforms, inverse, bounds, and owner |
| `0x827c84e0..0x827c8574` | `GroundPool::AddVolume`; identity placement and slot append |
| `0x825e9480..0x825e9558` | `AABBoxTemplate::Transform`; center/half AABB transform |
| `0x827ab338..0x827ab448` | `World::LineCollisionTest`; batch acquisition, execution, and result copy-out |
| `0x827b2538..0x827b2540` | `Island::LineCollisionTestWithVehicles`; tail-call to the owning `World` |
| `0x827c19c8`, call at `0x827c1abc` | `LocalClusteredMeshLineTest`; job worker dispatch to the collection query |
| `0x827c1300..0x827c1544` | gameplay `LineTest<TrajectoryLineTestFunctor>` over a `ClusteredMeshCollection` |
| `0x827c01a0..0x827c0428` | gameplay per-mesh line query, primitive expansion, and nearest-result update |
| `0x82acfdd0..0x82ad0cc0` | `UnitGetOverlappingGPInstances`; transformed vertices and face normals |

The PDB fixes `AALineClipper` at `0x40` bytes: origin, delta, reciprocal, and
padding are four consecutive 16-byte vectors. `KDTreeLineQuery` is `0x260`
bytes: the tree pointer is at `+0x000`, clipper at `+0x010`, 32
`StackElement` records at `+0x050`, top at `+0x250`, leaf count at `+0x254`,
and next entry at `+0x258`. Each stack record is `{NodeRef, pa, pb}` and is
`0x10` bytes. `NodeRef.content == 0xffffffff` denotes a branch; otherwise it
is a leaf count and `NodeRef.index` is the first encoded entry.

## Recovered behavior

The AABB iterator applies the native high-child test and then low-child test.
Its LIFO branch stack and same-node sibling-leaf coalescing are preserved.
Malformed axes and child indices are reported as safe-host errors; the native
routine assumes a valid serialized tree.

For line construction, with `S=start`, `E=end`, and componentwise operations:

```text
raw_delta = E - S
half_delta = raw_delta * 0.5
scale = vmax(vmax(abs(Bmin), abs(Bmax)), abs(S))
extension_magnitude = vmax(abs(half_delta), scale * 0.000001)
                    - abs(half_delta)
extension = fsel(half_delta, +1, -1) * extension_magnitude
origin = S - extension
delta = (E + extension) - origin
recip = vrefp(delta), followed by exactly two Newton/FMA refinements
padding = fma(vmax(abs(S), abs(E)), 0.000001, fatness)
        + extension_magnitude
```

Raw verification, 2026-09-24: `0x827bd514 = 0x116d602e` decodes standard
VMX fields VD=11, VA=13, VB=12, VC=0, XO=46. `vmaddfp` computes
VA*VC+VB: v0 holds epsilon and v12 holds fatness. IDA displays operands
in B-before-C order; the earlier fatness/epsilon formula was reversed.

The padded tree AABB is clipped against `[0,1]`. Axis entry and exit values
are reduced x/y first and then z with scalar `fsub` plus `fsel`, so the
operation order and zero ties matter. The root is accepted only by the ordered
strict comparison `pa < pb`; an equality-only or unordered interval is
rejected.

For a branch axis `a`, interval `[pa,pb]`, and split extents:

```text
t[0] = ((extent[0] + padding[a]) - origin[a]) * recip[a]
t[1] = ((extent[1] - padding[a]) - origin[a]) * recip[a]
positive = ordered(recip[a] > 0) ? 1 : 0

if ordered(pb > t[positive]): push far child first
other = positive ^ 1
if ordered(pa < t[other]):    push near child second
```

LIFO consumption therefore emits the geometrically near child first. Both
split comparisons are strict, and zero-count leaves are skipped. The formats
API returns a complete native leaf span and advances by its full count; callers
expand that span if they need per-entry behavior.

`ClipEnd(end)` scans the pending stack in existing order, retains entries for
which `pa > end` is not ordered true, and replaces each retained `pb` with the
native `fsub`/`fsel` minimum of old `pb` and `end`. It does not alter an active
leaf. Its native ownership is the generic RenderWare `VolumeLineQuery` path:
`KDTreeMappedArray::LineIntersectionQueryThis` calls it at `0x82ad9e84`,
`ClusteredMesh::LineIntersectionQueryThis` calls it at `0x82adf970`,
`0x82adfd74`, and `0x82adff08`, and
`TriangleKDTreeProcedural::LineIntersectionQueryThis` calls it at
`0x82ae0178` and `0x82ae0510`.

The gameplay `ClusteredMeshCollection` route at `0x827c1300`, including its
per-mesh worker at `0x827c01a0`, contains no `ClipEnd` call. It repeatedly
calls `ClusteredMeshLineQueryIterator::QueryRange` through exhaustion, even
after finding a nearer primitive. The public formats all-hit APIs are another
deliberate mode: they retain every accepted hit and stable-sort them. That
public stable sort must not be described as either native gameplay traversal
order or generic RenderWare nearest-query clipping.

## Gameplay collection provider

The PDB fixes `ClusteredMeshCollection` at `0x10` bytes: `meshes` is at `+0`,
`meshCount` at `+4`, and optional `spacial` octree at `+8`.
`ClusteredMeshData` is `0xc0` bytes: volume and mesh pointers at `+0/+4`, the
forward matrix at `+0x10`, inverse matrix at `+0x50`, AABB at `+0x90`, and
owner at `+0xb0`. `GroundPool` is `0x90` bytes and places capacity at `+8`,
the collection at `+0x10`, and its record allocation at `+0x20`. The normal
world constructor supplies capacity 128 at `0x827b19b0`.
`ClusteredMeshLineBatchJobData` is `0x10` bytes with outer octree, mesh count,
mesh array, and tests at `+0/+4/+8/+0xc`. The result/functor is `0x40` bytes:
its `0x30`-byte nearest record begins at `+0` and hit count is at `+0x30`.

`GroundPool::GroundPool` clears collection `spacial` and allocates exactly
`0xc0 * capacity` bytes. `GroundPool::AddVolume` appends the next slot and
increments `meshCount`; the native routine assumes that configured capacity
is respected. It constructs an identity placement rather than consuming its
matrix argument. `ClusteredMeshData::Reset` copies that forward matrix,
computes the inverse into `+0x50`, and copies the mesh KD root AABB verbatim
into `+0x90`. Consequently the ordinary ground collection reaches the
no-octree slot loop in ascending slot order.

Before either outer route, `0x827c1350..0x827c1378` computes the absolute XYZ
line delta and compares it, strictly and ordered, with `SMALL_FLOAT_28` at
`0x821043a8`, whose raw bits are `0x37800000` (`2^-16`). The raw VX128 words
at `0x827c135c` and `0x827c1360` decode as `vandc v56,v60,v59` followed by
`vor v55,v56,v56`: the latter is a copy, not an OR with epsilon bits. A query
therefore proceeds only if at least one XYZ component is strictly greater
than `2^-16`; equality, all signed zeros, and all sub-threshold components are
rejected. A NaN component is unordered and does not satisfy the comparison,
although another finite component may still pass. W is excluded by the native
lane reduction.

The no-octree broad phase at `0x827c14c0..0x827c14f0` forms, lane by lane,

```text
separated = (mesh_min > query_max) | (query_min > mesh_max)
```

and rejects if any XYZ lane is separated. Both comparisons are strict, so
touching bounds survive; W is discarded by `vsldoi ... 12`. A faithful host
predicate is the negation of those ordered `>` tests, not a rewrite using
`<=`, because those forms differ for NaN.

The preceding `AABBoxTemplate::Transform` at `0x825e9480` is not an identity
copy when its matrix pointer is non-null. It computes `half=(max-min)*0.5` and
`center=(max+min)*0.5`, then evaluates the absolute-axis extent in Y, X, Z
VMADD order and the transformed center in translation, X, Y, Z VMADD order.
Even Ground's identity inverse can therefore alter endpoint bits through
center/half rounding. Input AABB W is not consumed; output W is derived only
from matrix lanes and is ignored by the outer cull.

Within each accepted slot, `0x827c01a0` consumes every KD range and unit in
native iterator order. Each accepted triangle increments the result count.
The nearest record is replaced only when the clamped line parameter is
strictly less than the current parameter, so equal-distance ownership remains
with the first triangle encountered; mesh-slot order is therefore observable.
There is no callback-driven early termination on this specialization.

`UnitGetOverlappingGPInstances` does not source gameplay normals from the
serialized cluster normal table. For a triangle it transforms the three
decoded vertices first, computes `(p1-p0) x (p2-p0)` at
`0x82ad0160..0x82ad0180`, normalizes it with `vrsqrte` plus two Newton/FMA
refinements through `0x82ad01b4`, and stores it at `GPTriangle +0x10` at
`0x82ad02f0`. Quad output performs the corresponding two normalizations at
`0x82ad071c..0x82ad07fc`. Gameplay loads that stored normal at `0x827c035c`
and copies it into the nearest result at `0x827c0384`; it does not use the
normal temporary produced by `TriangleLineSegIntersect`.

The PDB-proven native stack capacity is 32. Overflow, invalid axes, and invalid
branch references are typed safe-host faults. A query must be discarded after
any error; deterministic replay of the overflow fault is an extra host safety
property, not a native recovery claim.

## Corpus gates

The independent formats-only full trace produced these frozen totals:

| Measurement | Count |
|---|---:|
| collections / Arenas | 284 / 2,135 |
| trees / mapped / mesh / branchless | 1,825 / 1,033 / 792 / 212 |
| branches / entries | 1,238,312 / 4,643,391 |
| probes / accepted roots / rejected roots | 12,788 / 12,785 / 3 |
| branch visits / leaf emissions / empty-leaf skips | 1,369,922 / 886,111 / 384,423 |
| unique wide-probe branches / missing reachable branches | 1,238,312 / 0 |
| maximum stack / synthetic cases / `ClipEnd` survivors | 30 / 14 / 2 |

Its exact 145,503,393-byte trace has signature `d096b1b5e5bfee79`.

The separately frozen public projection and the production comparator match
exactly at 70,139,064 bytes and signature `57c03f40e465e5f7`. Both report 284
collections, 2,135 Arenas, 1,825 trees, 12,775 real probes, 16 synthetic
probes, 886,115 leaf ranges, 4,804,738 leaf units, and two `ClipEnd` calls.
The production formats APIs now use the line traversal for mapped-array line
candidates, thin clustered-mesh segments, and radius-fat clustered-mesh
segments. Range collection is failure-atomic, and all-hit paths do not shorten
the traversal with `ClipEnd`.

With `SKATE2_GAME_ROOT` pointing at the supplied corpus, the repository-native
production gate is:

```text
cargo run --release -p skate2-pc -- --inventory-bam-sim-kd-line
```

It rejects any parse/query error, counter drift, protocol-length drift, or
signature drift.

Reproducibility hashes:

| Artifact | SHA-256 |
|---|---|
| independent full-trace harness | `5212B64DFA9BC7662E65EC83D5C4B7B6C0503FFA93BBEBDD529D93FC87648357` |
| public-projection harness | `6A5A44B86CC0D4105BECF2012C0BB7398B9BF337F367BE28A90CEA01DD7579C9` |
| production comparator harness | `2D954D556024D692B7C9F5629B9CA522B58C5099A6E27DFD2C2E4F6754B92F5E` |
| independent/public-oracle workspace manifest | `DAA459C65FBABD5B94FAE67D95B1B46446376F28C2AE88BBCD46CC41E48F3C3E` |
| frozen `43a690a` production-comparator manifest | `640EB3A43AE31882A6373B330A3062842123B02A2719088A0C224AC257983D14` |
| frozen `skate2-recomp-43a690a.zip` archive | `011283FF2876782C00FC2F1966CAC612252AABB9C8875DBA1F1F04A0D84BA062` |
| repository-native production gate module | `3C9CC7796CBA10862D84571C7D4725996CB1D940305DB52C6D2A8127F6956A73` |

## Explicit boundary

The scalar host implementation seeds the proven two reciprocal refinements
with host division. Static evidence cannot reproduce Xenon's exact `vrefp`
estimate bits, exceptional `vmaxfp`/`vminfp` behavior, fused rounding, NaN
payloads, denormal modes, or every signed-zero result. The gates therefore
prove ordinary finite-input structure, comparisons, leaf spans, and ordering,
not bit-exact exceptional VMX parity.

The gameplay ground provider, slot order, KD exhaustion, strict nearest
replacement, and absence of `ClipEnd` are now directly attributed. Generic
RenderWare `VolumeLineQuery` remains a separate API with its own result modes,
buffering, and the six `ClipEnd` call sites listed above. Exact parity outside
ordinary finite inputs still requires a Xenon oracle for reciprocal estimates,
fused rounding, denormal handling, NaN payloads, and signed-zero details.
