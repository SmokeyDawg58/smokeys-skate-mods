# World line-query collection ownership and publication

Root read-only IDA audit, 2026-09-08. This complements the line-query dispatch
and geometry packets. No new runtime collection is implemented by this packet.
Confidence high for the direct source lifecycle below; external mutation and
complete native scheduling still require integration evidence.

## Actual selected owners

Island PDB fields `mDmoPool` (+6cc8) and `mVehiclePool` (+6ccc) both hold
`eastl::scoped_ptr<Sk8::Physics::DynamicPool,...>`. `mSkaterPool` (+6cc4),
pedestrian and hook pools instead hold `EA::Physics::AssemblyPool`.
The IK default line batch selects the world's GroundPool and the selected
simulation island's DMO pool, then vehicle pool only for FLAG_Both. Skater
and hook primitives are not additional line-batch children.

`DynamicPool::DynamicPool`827caf20 explicitly sets collection meshes/count
to null/zero ataff4/aff8 and `spacial=null` ataffc. The collection is at+1770,
with meshes+0, count+4, spatial pointer+8. GroundPool ctor827c8c68 likewise
sets `spacial=null` at8cd4. DynamicPool BeginUpdate and Remove do not create
an octree. A scan of direct stores to offset1778 in827c0000..827d0000 finds
only the constructor store. This does not prove the absence of arbitrary
aliased writes, but no later spatial-index producer has been identified.
Therefore the generic Collection LineTest octree branch must not be described
as an already-established actual DMO traversal. Preserve the recovered octree
packet as a distinct optional collection path.

## Dynamic publication ordering

`?BeginUpdate@DynamicPool@Physics@Sk8@@UAAXXZ` [827c93f0,827c94bc), SHA256
`65dd3c036fd7b4b9bcac4c7a7e8c31d6926021f3b779c024e60181f4acfb44b3`:

1. Sync line, box, trajectory, step-selector batches at9404/0c/14/1c.
2. Clear collection count at9428, then capture current assembly count942c.
3. Visit current assembly array in ascending order; capture each part count.
4. For each Part, load its volume pointer9464 before actual Part.GetTransform
   9468. Invoke AddVolume with that assembly identity and returned transform.
   A part with no supported mesh still performs this getter.
5. A false AddVolume immediately returns, preserving completed mesh entries
   and skipping the base AssemblyPool.BeginUpdate. Only full traversal (also
   empty traversal) reaches base BeginUpdate at94b0.

`?AddVolume@DynamicPool@Physics@Sk8@@AAA_NPAXPBVVolume@collision@rw@@ABVMatrix44Affine@vpu@math@6@@Z`
[827c86d0,827c8824), SHA256
`4585f89e0a9ef3afff04b6dbbf6f5f4d33a33dbdc9758c0db6bd5823caeb373e`:

Only a disabled volume reads the live IgnoreVolumeEnableFlag cvar at8708.
Enabled volumes bypass that read. Rejected disabled volumes and non-aggregate
volumes return true. SimpleMappedArray children recurse ascending, retaining
the SAME assembly transform and owner; parent volume transforms are not
composed into that argument. Other aggregate types are ignored except meshes.

A mesh checks count < capacity at8750, otherwise native assertion/DebugBreak.
It increments count at8784 BEFORE resetting the selected entry879c. Reset
failure in a fallible host implementation must retain this admitted prefix;
it is not an all-or-nothing rebuild. After Reset it returns count < capacity.
Consequently the mesh that fills capacity is retained and returns false,
stopping all later siblings/parts/assemblies and the base BeginUpdate tail.
Native inline child indexing narrows to unsigned16; ordinary assets lie in
that range, while wider malformed counts require explicit host policy.

`DynamicPool::Remove`827c92b0..93f0 synchronizes those four batches, invokes
base AssemblyPool.Remove, then removes EVERY matching owner entry. Each hole
is filled from the last mesh (full pointers, transforms, bounds, owner), count
decrements, and the same index is examined again. This affects later tie order.

## Mesh record Reset

The real function span is [827c8378,827c84dc), not the old internal arithmetic
subspan83e4..84ac used by existing math comments. Decorated symbol:
`?Reset@ClusteredMeshData@Physics@Sk8@@QAAXPBVVolume@collision@rw@@PBVClusteredMesh@56@ABVMatrix44Affine@vpu@math@6@PAX@Z`.
SHA256 `863c06f03ea0a4bb7e4afbd23486db442956069be5476b43d58e7ccc132563d0`.

Stores owner+B0, volume+0, mesh+4, then the supplied full transform X/Y/Z/P
to+10. Volume's own transform is not composed here. The inverse at+50 is a
cofactor inverse, with one XYZ determinant, reciprocal estimate and two fused
refinements. Inverse column W duplicates its cofactor Y lane; position uses
bitwise-negated input position XYZ and multiply/FMA/FMA. There is no singular
guard. Only after all inverse stores does non-null mesh copy both full KD bbox
vectors to+90. Null mesh leaves previous bbox storage untouched.

Existing `world::inverse_collision_affine_reset` implements this numerical
graph with declared host dot/seed limits. The newly accepted typed two-bone
cofactor inverse has the same adjugate/merge graph and injected numerics; reuse
requires checking operand order, without substituting a rigid inverse.

## Existing Rust owner audit and next integration

`StaticGroundPool` is already the published ground authority with retained
original arena owners and swap-with-last removal order. Its
`assembly_ground_meshes()` borrow exposes current mesh order and lazy retained
KD-bound decoding errors. Actual ground frames borrow this pool separately
from `SimulationAssemblyConstraintSession`.

`DynamicAssemblySession` retains actual admitted Part getters and an Arc runtime
template. `DynamicClusteredMeshTemplate` retains original ordered mesh leaf
occurrences and backing arena bytes, but explicitly samples enable policy at
construction. It is not DynamicPool.BeginUpdate: it has no batch sync, live
disabled-node cvar sampling, mesh capacity, live Part transforms, Reset prefix,
base BeginUpdate tail or live mesh collection. A new live publisher must use
the actual assembly membership and retained arena/volume graph and may reuse
the validated structural relationships; it cannot relabel that template a
completed dynamic query collection or fabricate an empty DMO/vehicle pool.

The constraint session currently owns body/contact/hook/skater pools and
physical-player authority, but not the complete world's Ground/DMO/vehicle
line-query collections. Integration must connect these actual owners and the
World simulation-island selection before claiming default-query completion.
This packet has no original runtime line trace or independent review yet.
