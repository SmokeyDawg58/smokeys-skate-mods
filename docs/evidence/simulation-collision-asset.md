# Simulation collision asset projection

## Scope

`simulation_collision_asset.rs` moves the previously app-local RW4 collision
and grind projection into the core resident-asset builder boundary. Each
`SimulationCollisionAsset` owns the complete `DecodedArenaAsset` plus the
derived `CollisionArena`. Unsupported dictionary objects and all five resource
buffers therefore remain available instead of disappearing when collision
types are selected.

The projection is not named or treated as native registration. It does not
implement the resource allocator, TOC publication, `Simulation::RegArena`,
WorldPainter/SpatialMap processing, deregistration, or destruction behavior.

## Native anchors

| Symbol | Address | Role beyond this projection |
|---|---:|---|
| `cAssetManager::AddCompressedArenaAsset` | `0x82d63a10` | raw stream format 2 arena construction |
| `cProcessArenaAsset::ProcessArena` | `0x82d5fee0` | native arena object processing |
| `cProcessArenaAsset::ProcessSplineList` | `0x82d5f180` | spline/grind processing |
| `cProcessArenaAsset::ProcessInstanceList` | `0x82d5ec38` | instance processing |
| `Simulation::RegArena` | `0x828a01e0` | later Simulation registration |
| `cSk8AssetManager::SendSimRegArenaMsg` | `0x82d58318` | Simulation registration message |
| `ArenaSystem::RegisterArenaTypes` | `0x828b46e0` | base RW4 type callbacks |
| `Volume::RegisterArenaReadCallbacks` | `0x82ae0860` | collision volume callbacks |
| `KDTreeMappedArray::Fixup` | `0x828b45a0` | mapped-array fixup |
| `ClusteredMesh::Fixup` | `0x828b4698` | clustered-mesh fixup |

These anchors establish the native types and later stages; they do not prove
that an owned Rust projection reproduces their side effects.

## Builder and projection order

The catalog-checked resident builder is created only by
`SimulationCollisionAssetBuilderSessions`. It re-resolves the registered
collection by full ID, requires the complete supplied snapshot to match that
canonical catalog record, resolves the owning catalog file, and rejects
missing, Presentation, or Texture owners before decoding. The builder first
delegates envelope validation, all five segment decodes, XST resource-size
checks, and RW4 parsing to `DecodedArenaAssetBuilder`.
The resulting wrapper exposes the same full outer XST ID through
`ResidentAssetIdentity`, allowing the non-owning resident writer to preserve
collection membership without substituting the 32-bit RW4 arena ID.

Projection then preserves the existing application order:

1. spline objects in dictionary order;
2. collision geometry in dictionary order;
3. root linking after all geometry indexes exist;
4. collision models in dictionary/record order;
5. instances in dictionary/record order.

Local section-zero references resolve to stable indexes in the owned vectors.
Nonlocal and missing local targets are distinct typed errors. Assembly-backed
collision models retain their raw object ID instead of inventing assembly
construction. Instance transforms and AABB lanes are copied without numeric
normalization. The diagnostic grind projection still labels sources with the
RW4 arena ID plus dictionary index. This is deliberately not presented as the
native registry identity: `cProcessArenaAsset::ProcessSplineList`
(`0x82d5f180`) passes the enclosing TOC entry's full 64-bit GUID to
`AddSimRegistryAsset`, and `Simulation::RegArena` (`0x828a01e0`) forwards that
same asset ID to `Physics::GrindData::AddSplineData`.

## Real-data validation

An external frozen harness drove the actual decoded builder over all 3,281
selected Rob's Park Presentation, Simulation, and Texture assets, then applied
this projection to all 68 Simulation arenas. The Simulation asset-order FNV
signature remained `0x93cebde307b9f54e`. All 68 succeeded and retained 954 RW4
dictionary objects, including 674 entries outside the six directly consumed
type families.

The exact resulting projection is:

- 68 arenas;
- 71 aggregate roots;
- 45 primitive sets with 451 inline primitives;
- 26 clustered meshes with 354,382 triangles;
- 26 collision models;
- 477 instances, 26 linked to collision models;
- 42 grind assets with 10,314 grind primitives.

Harness SHA-256 is
`726C51A7724B972293838C059B5EF0E7DAC8167EF53D61C22CD65B70C49F6D41`;
output SHA-256 is
`FAC1D675363750CB30663D82B1EDF25FAB5008E24DFC6551E465E871B4C7649A`.

Eight focused tests cover complete opaque resource retention, full outer ID
versus independent RW4 arena ID, typed local-reference failures, assembly-link
preservation, unusual transform bit preservation, and Simulation-only session
selection including a retargeted-snapshot rejection. The corpus counts and
extraction of the current app behavior are
**exact**. The Rust ownership/session boundary is **high**. Native allocation,
registration, publication, cross-asset sharing, and teardown remain unknown.
