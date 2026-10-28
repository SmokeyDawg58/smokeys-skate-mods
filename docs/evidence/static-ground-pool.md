# Static Environment ground pool and trajectory query

## Scope

This evidence slice covers storage identity and ordering for direct kind-10
Environment collision roots after processed-arena construction, plus the
ordinary finite-input control flow of the static-ground trajectory fat-line
and fixed-64 ground-triangle box queries. It does not claim complete
`Simulation::RegArena`, career/challenge
state, assembly collision, asynchronous job/batch behavior, live DMO or
vehicle providers, instruction-bit-exact fat contact, or native allocation
failure behavior.

## Native anchors

| Symbol | VA | Proven role |
|---|---:|---|
| `Simulation::RegArena` | `0x828a01e0` | kind 10 forwards the retained `tInstance*` |
| `UnlockableLocationMan::AddEnviroInstance` | `0x828b02d8`, `0x828b0068` | local-GUID context lookup and admission |
| `SimUnlockableLocationMan::AddToWorld` | `0x828a1670` | obtains the physics world-streamer view |
| `World::World` | `0x827b15f0` | supplies a ground-pool capacity of 128 |
| `GroundPool::GroundPool` | `0x827c8c68` | allocates 0xC0-byte mesh records and initializes query batches |
| `WorldStreamerView::AddVolume` | `0x827c8578` | walks direct CModel roots forward with identity placement |
| `GroundPool::AddVolume` | `0x827c84e0` | appends clustered-mesh state |
| `ClusteredMeshData::Reset` | `0x827c8378` | stores volume/mesh/transforms/owner and snapshots KD bounds |
| `ClusteredMeshCollection::LineTest<TrajectoryLineTestFunctor>` | `0x827c1300` | degenerate gate, transformed query bounds, spatial or linear candidate walk |
| inner `LineTest<TrajectoryLineTestFunctor>` | `0x827c01a0` | KD ranges, serialized units, GP triangles, strict-nearest update |
| `ClusteredMeshCollection::BoxTest<TrajectoryBoxTestFunctor>` | `0x827c1548` | transforms the query box and walks the outer spatial index or linear ground slots |
| inner `BoxTest<TrajectoryBoxTestFunctor>` | `0x827c0428` | KD ranges, unit expansion, final 13-axis triangle SAT, and fixed-cache admission |
| `ClusteredMeshCluster::UnitGetOverlappingGPInstances` | `0x82acfdd0` | constructs the `GPTriangle` values consumed by the functor |
| `AABoxTriangleOverlapTestOpt` | `0x827ace68` | final 13-axis world-box versus emitted-GP-triangle overlap test |
| `AABBoxTemplate::Transform` | `0x825e9480` | transforms the query AABB through the stored inverse transform |
| `Island::AcquireLineBatch` | `0x827b8728` | dispatches Ground, then DMO, then conditional Vehicle providers |
| `AggregateLineBatch::GetResult` | `0x827bbec8` | selects a strictly nearer provider result; ties retain the earlier provider |
| `Simulation::DRegArena` | `0x828a05f0` | kind 10 forwards removal in arena order |
| `WorldStreamerView::RemoveVolume` | `0x827c9088` | walks the same direct roots forward |
| `GroundPool::RemoveVolume` | `0x827c8e60` | synchronizes batches, first-match search, swap-last removal |

`WorldStreamerView` returns for a null instance, null CModel, or zero CModel
count. Add asserts on a nonempty RModel but continues. Direct CModel records
have eight-byte entries; only the first word, the collision `Volume*`, is
forwarded. The serialized instance transform, bounds, GUIDs, and second table
word do not reach `GroundPool`. RenderWare's affine identity encoding is the
bit-exact four-vector sequence `[I, J, K, zero]`; its final lane is positive
zero rather than the `1.0` used by a conventional homogeneous 4-by-4 matrix.

The PDB fixes the relevant native layouts:

| Type | Size and fields used here |
|---|---|
| `GroundPool` | size `0x90`; allocator `+0x00`, scheduler `+0x04`, capacity `+0x08`, collection `+0x10`, mesh storage `+0x20`, world bounds `+0x30`, line/box/trajectory/step batches `+0x50/+0x5c/+0x68/+0x74`, mode `+0x80`, collision group `+0x84` |
| `GroundPoolParams` | size `0x24` |
| `ClusteredMeshCollection` | size `0x10`; mesh records `+0x00`, count `+0x04`, spatial index `+0x08` |
| `ClusteredMeshData` | size `0xC0`; volume `+0x00`, mesh `+0x04`, forward matrix `+0x10`, inverse matrix `+0x50`, KD/root bounds `+0x90`, owner `+0xb0` |
| `VolumeLineResult` | size `0x30`; position `+0x00`, normal `+0x10`, line parameter `+0x20`, surface ID `+0x24`, owner `+0x28` |
| `TrajectoryLineTestFunctor` | size `0x40`; nearest result `+0x00..+0x2f`, accepted-triangle count `+0x30` |
| `TrajectoryBoxTestFunctor` | size `0x1010`; `Triangle[64]` cache `+0x0000`, cache count `+0x1000` |
| `Sk8::Physics::Triangle` | size `0x40`; `p0/p1/p2` at `+0x00/+0x10/+0x20`, surface ID `+0x30` |
| `GPTriangle` / `GPInstance` | size `0xC0`; position `+0x00`, face normals `+0x10`, user tag `+0x88` |

The box-query extraction is reproducible against these fixed artifacts:

| Artifact | SHA-256 |
|---|---|
| `sk82_na_m.xex` (debug-symbol set) | `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2` |
| `sk82_na_m.pdb` | `0b060cec3caa2f174999ebfdec1d7c90cbcb212815e7933e68d273d687d56ef0` |
| `sk82_na_m.map` | `51e60f0993c40ff1e473f3d28e1e2f954dfec605a785ca2ee02320c8b8997e88` |
| `dump_ground_box_symbols.py` | `841a1c240962f2cb162930e3153a2062a17407513fdae6389470fb046168b576` |
| `ground_box_symbols.txt` | `d285e29e8f4d177632bf215790152373ed4ce96de8162872e40f2d16ccea8714` |
| `dump_aabox_triangle_opt.py` | `e18b489d008c61dca7d1a973a14e5e2baaa8dec7488f6d27c89cd745bfcd8145` |
| `aabox_triangle_opt_raw.txt` | `c70c72d15b3ebf89d6847586e794fa297a65b97e50072243a56e13d4e3b8b664` |
| `fuzz_aabox_triangle_order.py` (historical non-fused exploratory search) | `3e52363529e9a89a0f9c40529c4840ee49fb9584585c1be6ce4d91d59aedd33c` |
| `search_sat_discriminator.rs` (native-shaped host-FMA model, not a Xenon oracle) | `03d55c20c0d1a448c5918db8bf7ef17f92cd0c4aed1a2f86c3d34dc03eb04ff8` |
| `search_sat_discriminator_output.txt` | `149bf8e2afe723f2b18d41a4ee690491a9aa371343b10c2f77d54cd383369fe5` |
| `search_normal_discriminator.rs` (host `vmulfp`/`vnmsubfp` model, not a Xenon oracle) | `0d755417f9c789581064e7df6cccf3d7794a2c5355aec6c2ec8cda31061d7271` |
| `search_normal_discriminator_output.txt` | `56cc6b82a1f51acf51da3d7e04a1d1f4e3f90352c449e0e9386bda98c3dbd4e5` |

`WorldStreamerView::AddVolume` supplies identity placement for each direct
root. `GroundPool::AddVolume` passes the aggregate volume as the clustered
mesh and a null owner to `ClusteredMeshData::Reset`. Reset stores both forward
and computed inverse transforms and snapshots `mesh->mKDTree->m_bbox`; the
readable slot likewise snapshots the decoded KD bounds at add time. A malformed
stored KD root is retained as a typed snapshot error and is reported if that
slot is later observed by a nondegenerate query.

Removal is source-based rather than paired-token based. The native unlock
manager calls `RemFromWorld` even if no context exists or the corresponding add
was suppressed. `GroundPool` then removes the first pointer-identical volume or
does nothing when absent.

## Readable ownership

`StaticEnvironmentSource` retains `Arc<SimulationStaticArenaPlan>` and stable
operation/root/mesh indexes. Construction rejects malformed indexes, assembly
models, and non-mesh roots before lifecycle mutation. Missing CModel and
ordinary zero-root models are valid no-op sources.

Each `StaticGroundPool` slot retains its own plan hold. Pointer identity is
modeled by `Arc::ptr_eq(owner)` plus collision-root index; GUIDs and structural
equality are deliberately not removal identity. Adds retain duplicates and
root order. Removes walk source roots forward, choose the first match, and use
`swap_remove` to reproduce fill-from-last compaction. That compaction also
changes subsequent candidate and exact-tie order in the same way as the native
array.

The production native constructor receives capacity 128 from `World::World`.
The readable pool accepts an explicit capacity so tests can exercise smaller
bounds; upfront Rust allocation, typed overflow, and whole-batch preflight are
host safety policies. Native Add has no visible capacity guard and its overflow
behavior is not known. Native remove synchronizes four query batches before
searching; synchronous Rust mutation is currently exclusive, while an
asynchronous backend will require an equivalent per-root fence.

## Static trajectory query

`ClusteredMeshCollection::LineTest<TrajectoryLineTestFunctor>` first compares
the absolute x/y/z line deltas with `SMALL_FLOAT_28`. The executable constant
at `0x821043a8` is big-endian `0x37800000`, or exactly `2^-16`
(`1.52587890625e-5`). At least one ordered component comparison must be
strictly greater; equality and an all-short line return before observing any
slot or stored KD snapshot.

For a nondegenerate query, the world query bounds are
`min(start,end)-radius` through `max(start,end)+radius`. Each record always
transforms that box through its stored inverse matrix, including the native
ground path whose matrix is identity. `AABBoxTemplate::Transform` reconstructs
center and half extent, transforms the center affinely, accumulates extents
through the absolute matrix axes, and returns transformed center plus/minus
extent. The readable implementation preserves that operation order before
culling.

When the collection spatial index is null, the native fallback scans record
slots from zero through `meshCount-1`. Its per-axis closed overlap predicate is
equivalent to:

```text
!(record_min > query_max || query_min > record_max)
```

Equality therefore overlaps. The underlying ordered `vcmpgtfp` comparisons
are false when unordered, so a NaN in an axis can admit rather than reject the
record. Accepted records are visited in slot order, then KD leaf-range order,
serialized unit order, and emitted GP-triangle order; this gameplay adapter
does not sort them.

The functor clamps the raw intersection parameter with the ordered scalar
sequence:

```text
lower = (-raw >= 0.0) ? +0.0 : raw
t = (1.0 - lower >= 0.0) ? lower : 1.0
```

Thus both signed zeros become positive zero and, under the recovered ordered
comparison model, NaN becomes `1.0`. A candidate replaces the current result
only when its clamped parameter is strictly smaller; exact ties retain the
earlier triangle, record, or provider. Position is copied from the
intersection output computed with the raw parameter and is not recomputed from
the clamp.

The gameplay collection call at `0x827c1300` exhausts its KD iterator and does
not call `ClipEnd`. This must remain distinct from generic RenderWare queries:
`KDTreeMappedArray::LineIntersectionQueryThis` at `0x82ad9d78` clips at entry
with its query-owned end value, `ClusteredMesh::LineIntersectionQueryThis` at
`0x82adf808` clips at entry and after nearer triangles, and
`KDTreeLineQuery::ClipEnd` is at `0x82ac8b70`.

## Static trajectory ground-box query

The trajectory box path at `0x827c1548` uses the same null-outer-octree ground
collection and therefore visits records in current slot order. For every
record it transforms the original world AABB through the stored inverse
matrix. Direct static ground stores identity, but the native center/half-extent
reconstruction is still performed and can round the resulting local/model
bounds. Those transformed bounds drive the closed record-bounds cull, the KD
AABB traversal, and `UnitGetOverlappingGPInstances`. The final
`AABoxTriangleOverlapTestOpt` at `0x827c04e8` instead receives the original
world AABB and the forward-transformed `GPTriangle`. The readable provider
keeps that world-versus-model distinction rather than reusing either box for
both stages.

An admitted record is expanded in slot, KD-range, serialized-unit, and emitted
GP-triangle order. Only a final SAT overlap reaches the cache-capacity check.
Counts below 64 copy the three positions, load the surface ID, increment the
count, and continue traversal. Consequently the 64th accepted triangle is
stored. A later rejected triangle does not stop a full cache; the first later
accepted triangle (the would-be 65th) observes `count >= 64`, is not stored,
and terminates the remaining record and collection traversal. No sorting or
replacement occurs.

The unit user tag contains surface in the high 16 bits and group in the low 16
bits. On the big-endian Xenon, the `lhz` at `0x827c0528` reads the high halfword
of `GPTriangle + 0x88`; the host exposes it as a zero-extended `u32`, and maps an
absent optional unit surface tag to zero.

The host API returns an owned vector capped at 64 and is failure-atomic. Its
safe decoder first exhausts the visited KD traversal, then eagerly validates
all serialized clusters and units before expanding the selected ranges. It can
therefore report a malformed later KD node, cluster, or unit before an earlier
would-be 65th accepted triangle could stop native traversal. Typed parse
errors, allocation behavior, and that failure timing are host policy rather
than claims about native recovery. The host SAT preserves the recovered nine
specialized edge-pair tests in edge/X/Y/Z order, followed by the box-axis and
triangle-plane stages, with strict contact inclusion. `f32::mul_add` models
the `vmaddfp`/`vnmsubfp` operation graph; nested FMA remains only a host
approximation of `vmsum3fp`, so Xenon fused/VMX exceptional behavior, NaN
payloads, denormals, and signed-zero details are not asserted bit-identical.

## Result semantics and host boundary

Native output copies the intersection position, the GP triangle's first face
normal, the clamped parameter, and the unit-derived surface ID. The user tag
packs surface in its high 16 bits and group in its low 16 bits; because Xenon
memory is big-endian, the halfword load at `GPTriangle + 0x88` reads the
surface. The record owner is copied separately. The functor also increments
its count for every accepted
triangle. The current `TrajectorySweepHit` host seam exposes only the selected
position, normal, and surface ID; it does not yet expose the native line
parameter, owner, or count. If an optional direct-unit tag is absent, the host
maps the surface to zero rather than substituting a mesh default.

The host reconstructs the same face-normal source from triangle vertices, but
its scalar cross product and square root are not asserted bit-identical to the
Xenon `vrsqrte` refinement path. The radius-fat triangle solver is likewise a
source-level finite-input implementation, not an instruction-level native
claim. Xenon reciprocal/reciprocal-square-root estimates, fused arithmetic,
VMX min/max ties, NaN payloads, denormals, and exceptional signed-zero behavior
remain hardware-oracle boundaries.

Typed parse/capacity failures, failure-atomic return without a partial hit,
Rust allocations, and eager validation that can report malformed unvisited
cluster/unit data are deliberate safe-host behavior. They do not describe
native failure or retry semantics.

## Corpus gate

The selected Rob's Park corpus contributes 26 accepted Environment operations,
one direct clustered-mesh root each. As a lower-seam diagnostic, the
in-repository optimized resident probe inserts all 26 during asset build and
must observe 26 slots while the four Simulation collections are resident, then
zero after host `Drop` teardown in both direct and parent-expanded
compositions. That timing bypasses unlock admission and is not asserted as
native SimReg/DReg timing. It retains the prior ordered static signature
`39413e2d647f8631`.

The independent corpus harness is
`env-kind10-audit\src\main.rs`, SHA-256
`B59FA8849208363B5FDA132B734C5B28741C4C695B1B00E2ADE9E6DB91BA6DA9`.
Its detailed ordered output SHA-256 is
`D1B5CD4F187F7B435B3F9597DA9E40294F402E89B4570485BFC522CA1F3626E4`.
Two executions reproduced the output hash.

The upper fixed context map and its physically 64-bit `bitset<16>` status
remain unseeded. Raw PPC proves local-GUID lookup and the admission predicate
`mask_bit == !is_unlocked_env_instance`, but the ordinary boot/career producers
for those values have not yet been recovered.
