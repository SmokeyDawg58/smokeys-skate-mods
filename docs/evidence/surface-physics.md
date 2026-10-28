# Collision surfaces and board response

Last updated: 2026-09-04

## Serialized surface ID

Collision units carry a 16-bit `pegasus::tSurfaceID`. The executable extracts
the physics field with `(surfaceID >> 7) & 0x1f`; debug enums and the remaining
bit widths establish the complete packed representation:

| Bits | Width | Meaning |
|---:|---:|---|
| 0..6 | 7 | `tAudioSurfaceType` |
| 7..11 | 5 | `tPhysicsSurfaceType` |
| 12..15 | 4 | `tSurfacePatternType` |

`tPhysicsSurfaceType` is exact: undefined `0`, smooth `1`, rough `2`, slow
`3`, slippery `4`, very slow `5`, unrideable `6`, do-not-align `7`, stair `8`,
and wipeout `9`. The source parser retains unknown numeric values rather than
silently substituting a meaning.

## Runtime selection

Direct executable evidence:

| Address | Function | Recovered behavior |
|---:|---|---|
| `0x824e67c0` | `AudioSystem::GetAudioMaterial` | undefined maps to the maximum/sentinel; otherwise a valid surface maps to `surface - 1` |
| `0x824e6858` | `SurfacePatternToKey` | maps all serialized pattern values to attribute keys |
| `0x82b24878` | `SkateboardBody::UpdatePostPhysics_ChoosePhysicsSurfaceType` | votes across the four wheel contacts; an explicit forced-surface value wins |
| `0x82b29fb8` | `Skateboard::SetWheelMaterialForSurface` | installs the selected material on all four assembly parts |
| `0x82dd6060` | `SurfacePhysics::SurfacePhysics` | constructs per-surface wheel materials and loads friction/restitution attributes |
| `0x82dd63e8` | `SurfacePhysics::Dispatch` | chooses the gameplay attribute collection and wheel material |
| `0x82dda0b0` | `SurfacePhysics::Update` | dispatches from the collision result's physics-surface field |

Dispatch has dedicated attribute/material pairs for smooth, rough, slow,
slippery, and very slow. Unrideable deliberately uses the slow pair; an
unrecognized or undefined value uses smooth. The high-LOD player layer adds
semantic consequences: unrideable requests a teleport and wipeout requests a
wipeout. Do-not-align and stair remain distinct contact classifications even
where they use a fallback wheel material.

The wheel materials begin with reference defaults of static friction `0.5`,
dynamic friction `0.3`, and restitution `0.4`, then receive the configured
surface-specific values. This is the boundary the Rust skater physics layer
will reproduce; it does not require an Xbox physics or emulator subsystem.

For the recovered static-ground trajectory path, `GPTriangle::mUserTag`
(`GPInstance` user tag at `+0x88`) packs surface in the high 16 bits and group
in the low 16 bits. Xenon is big-endian, so the halfword load at `+0x88`
selects the surface ID. The readable direct-unit path preserves an explicit
two-byte serialized surface
tag and maps a missing optional tag to zero; it does not substitute the
clustered mesh's default surface. This result plumbing establishes which
packed ID reaches later surface dispatch, not the still-unrecovered complete
contact-to-wheel state machine.

## Native primitive collision

The serialized `rw::collision::Volume` transform and active union member now
drive readable Rust thin-segment intersections for spheres, oriented boxes,
and finite cylinders. These are traced to:

| Address | Function |
|---:|---|
| `0x82ad7170` | `BoxVolume::LineSegIntersect` |
| `0x82ad8ce8` | `CylinderVolume::ThinLineSegIntersect` |
| `0x82ad9500` | `SphereVolume::LineSegIntersect` |
| `0x82ae0778` | `CylinderVolume::LineSegIntersect` wrapper |

The box union stores three half-extents. The cylinder uses the union's
half-height and radius fields; a sphere uses `Volume::radius`. A complete BAM
simulation pass casts an axis segment through every inline primitive using its
own serialized transform and dimensions. All 14,932 primitives hit: 401
spheres, 13,911 boxes, and 620 cylinders, with zero invalid parameters or
misses. This validates both the union interpretation and transform convention
against the supplied game corpus.

## Trajectory result normal and ordering

The static-ground trajectory functor consumes values emitted by
`ClusteredMeshCluster::UnitGetOverlappingGPInstances` at `0x82acfdd0`.
`ClusteredMeshCollection::LineTest<TrajectoryLineTestFunctor>` at `0x827c1300`
and its inner mesh routine at `0x827c01a0` copy
`GPTriangle::mFaceNormals[0]` into `VolumeLineResult`; this is the triangle face
normal, not a sphere-contact radial normal. The readable implementation
recomputes that face normal from the serialized vertices. It matches the
source-level geometric source for ordinary finite triangles but is not claimed
bit-identical to Xenon reciprocal-square-root/refinement arithmetic or for
degenerate and exceptional-float inputs.

Candidate order is ground-pool slot, KD leaf range, serialized unit, then GP
triangle. Within that sequence, only a strictly smaller clamped line parameter
replaces the result, so exact ties keep the first. At the provider level,
`Island::AcquireLineBatch` (`0x827b8728`) orders Ground, DMO, then conditional
Vehicle work; `AggregateLineBatch::GetResult` (`0x827bbec8`) also replaces only
on strict improvement, preserving the earlier provider on equality. The
current host milestone implements the static Ground provider, not DMO or
Vehicle collision.

The gameplay collection path exhausts its KD range iterator without
hit-driven `ClipEnd`. Generic RenderWare line-query ownership is separate:
`KDTreeMappedArray::LineIntersectionQueryThis` (`0x82ad9d78`) clips at entry,
`ClusteredMesh::LineIntersectionQueryThis` (`0x82adf808`) clips at entry and
after nearer triangles, and `KDTreeLineQuery::ClipEnd` is `0x82ac8b70`. Those
generic calls must not be projected onto the gameplay trajectory callback.

## Remaining parity work

The dedicated stateful KD line clipper now supplies clustered-mesh thin and
radius-fat broad phases; see [`kd-line-query.md`](kd-line-query.md). The static
Ground path, provider order, strict nearest/tie behavior, face-normal source,
and surface-tag source are recovered at source level. Dynamic DMO/Vehicle
providers, the complete downstream gameplay contact consumers, and
instruction-bit-exact radius-fat contact remain separate work.

Exact Xenon `vrefp`/`vrsqrte` refinement, fused arithmetic, VMX min/max ties,
NaN payloads, denormals, and exceptional signed-zero behavior still require a
hardware oracle. Typed parse errors, eager host validation, allocation
failures, and failure-atomic `Result` returns are safe-host boundaries rather
than claims about native failure behavior.
