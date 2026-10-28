# Dynamic DMO/Vehicle trajectory providers

## Scope

This evidence slice establishes the native provider selection, dynamic
clustered-mesh storage, and provider-local trajectory execution needed to add
DMO and Vehicle collision to the readable predictor. It also records the
source-level aggregate wrapper committed as
`fea95b30c30a8a39b90306fc4f17079a0388b491`, the finite-input transform
arithmetic correction committed as `8b5ac39`, and the arena-resident
structural template committed as `fb971d8`. Application ownership is committed
as `46255e1`, the host-bounded Reset inverse as `2067242`, the bounded affine
orthonormalizer as `5f201ee`, and the Part attachment seed as `5f60888`.

The wrapper preserves independently completed provider results and their
strict aggregate ordering. The template and application catalog now retain
streamed clustered-mesh allocations, assembly-part metadata, and native
`AddVolume` occurrence order without flattening them. They do not yet
materialize a live DMO or Vehicle `DynamicPool` snapshot or connect live part
transforms. Static Ground is the only concrete gameplay trajectory provider
currently connected.

## Native anchors

| Symbol | VA | Proven role |
|---|---:|---|
| `World::AcquireLineBatch` | `0x827ab918` | forwards caller flags, or substitutes `7` for the visual-island path |
| `Island::AcquireLineBatch` | `0x827b8728` | appends Ground, DMO, then conditional Vehicle line batches |
| `Island::AcquireBoxBatch` | `0x827b8860` | appends the same providers in the same order for box tests |
| `Island::AcquireTrajectoryBatch` | `0x827b8998` | appends the same providers in the same order for complete trajectory tests |
| `AggregateTrajectoryBatch::AddTest` | `0x827bc050` | submits the complete trajectory test independently to every child provider |
| `AggregateTrajectoryBatch::GetResult` | `0x827bc0f0` | selects a nonnegative completed child result and replaces only for strictly lower time |
| `ClusteredMeshTrajectoryBatch::AddTest` | `0x827bdcc0` | allocates and initializes one provider-local trajectory test/result |
| `ClusteredMeshTrajectoryBatch::GetResult` | `0x827bdf68` | returns only provider results with nonnegative collision time |
| `LocalClusteredMeshTrajectoryTest` | `0x827c2270` | constructs a walk against that child batch's collection and writes its complete result |
| `TrajectoryWalkFunctor::operator()` | `0x827c1c78` | performs the provider-local line hit, optional box cache, and ground-normal calculation |
| collection line specialization | `0x827c1300` | traverses one `ClusteredMeshCollection` for a trajectory segment |
| inner mesh line specialization | `0x827c01a0` | traverses KD ranges/units/GP triangles and applies strict-nearest replacement |
| collection trajectory-box specialization | `0x827c1548` | traverses the same provider collection around that provider's hit |
| inner trajectory-box specialization | `0x827c0428` | admits world-space overlapping triangles to the fixed cache |
| `CalculateGroundNormal<Triangle,64>` | `0x827bf5a0` | calculates the provider result's landing normal from its local cache |
| `DynamicPool::AddVolume` | `0x827c86d0` | filters aggregate volumes and flattens clustered meshes/SMA children into the provider array |
| `DynamicPool::Remove` | `0x827c92b0` | synchronizes all batch helpers, removes the assembly, and swap-removes its mesh records |
| `DynamicPool::BeginUpdate` | `0x827c93f0` | rebuilds mesh records from current assemblies and live part transforms |
| `ClusteredMeshData::Reset` | `0x827c8378` | stores volume, mesh, transforms, KD bounds, and owner |
| `rw::physics::Part::GetTransform` | `0x826c3480` | derives the live part transform consumed by `BeginUpdate` |
| `Island::AddVehicleBody` | `0x827b6840` | inserts a Vehicle assembly and establishes its Island/body identity |
| `Island::CleanUpVehicleBody` | `0x827b6a98` | clears contacts and removes the Vehicle assembly from its pool |
| `Island::InitWorld` | `0x827b42c0` | constructs the 50-entry DMO and conditional 24-entry Vehicle pools |

The three Island acquisition functions use the same exact Vehicle predicate.
They load `Island::mFlags` at `+0x08`, compare it with integer `3`, and skip
Vehicle unless it is equal; they then test bit `4` in `BatchFlags`. The
condition is therefore:

```text
Island.mFlags == 3 && (batch_flags & 4) != 0
```

It is not a low-bit mask test on `mFlags`. The PDB enum assigns Ground `1`, DMO
`2`, Vehicle `4`, GroundDmo `3`, Default `7`, and All `255`. Provider insertion
order is Ground, DMO, Vehicle. `LineCollisionTestWithVehicles` at `0x827b2538`
is a tail call into the ordinary World line path; its name is not evidence for
bypassing this Island predicate.

## Dynamic-pool storage and rebuild order

DMO and Vehicle both use `Sk8::Physics::DynamicPool`; Vehicle is not a separate
geometry algorithm. The PDB fixes the relevant layouts:

| Type | Size and fields used here |
|---|---|
| `DynamicPool` | size `0x17f0`; `AssemblyPool` base through `+0x41f`, allocator/scheduler `+0x1760/+0x1764`, capacity `+0x1768`, collection `+0x1770`, mesh allocation `+0x1780`, line/box/trajectory/step helpers `+0x1784/+0x1790/+0x179c/+0x17a8` |
| `DynamicPoolParams` | size `0x34`; capacity `+0x10`, line/box/step/trajectory/test counts `+0x14/+0x18/+0x1c/+0x20/+0x24` |
| `ClusteredMeshCollection` | size `0x10`; mesh records `+0x00`, count `+0x04`, outer spatial pointer `+0x08` |
| `ClusteredMeshData` | size `0xc0`; volume `+0x00`, mesh `+0x04`, forward/inverse matrices `+0x10/+0x50`, local KD bbox snapshot `+0x90`, owner `+0xb0` |
| `rw::physics::Assembly` | size `0x30`; part list `+0x08`, part count `+0x1c`, user data `+0x28` |
| `rw::physics::Part` | size `0x60`; part matrix `+0x00`, volume `+0x40`, optional inverse-body matrix `+0x44`, rigid body `+0x4c` |
| `ClusteredMeshTrajectoryTest` | size `0x50`; trajectory `+0x00`, radius/errors `+0x40/+0x44/+0x48`, result pointer `+0x4c` |
| `TrajectoryResult` | size `0x50`; position/normal/ground normal/time `+0x00/+0x10/+0x20/+0x30`, frame `+0x40`, surface `+0x44` |
| `TrajectoryBoxTestFunctor` | size `0x1010`; 64 `Triangle` records at `+0x0000`, count `+0x1000` |
| `Triangle` | size `0x40`; p0/p1/p2 `+0x00/+0x10/+0x20`, surface `+0x30` |

`InitWorld` constructs the DMO `DynamicPool` with both assembly and mesh
capacity 50. It constructs Vehicle with both capacities 24 when the simulation
configuration admits that pool, otherwise zero. These are distinct from the
64-entry trajectory ground-normal cache.

`BeginUpdate` first synchronizes line, box, trajectory, and step-selector
batches, resets the mesh count, and visits the `AssemblyPool` array in its
current order. It visits each assembly's part list in index order, calls
`Part::GetTransform`, and passes that live matrix and the part volume to
`AddVolume`. `AddVolume` ignores a disabled volume unless the native
ignore-enable cvar is set, ignores non-aggregate volumes, emits a
`ClusteredMeshData` record for a ClusteredMesh aggregate, and recursively
visits SimpleMappedArray inline volumes in ascending index order. Other
aggregate object types do not emit a record.

The same part transform is forwarded through SimpleMappedArray recursion.
Child `Volume::transform` matrices are not composed into the mesh placement by
this function. The emitted record separately retains the exact leaf volume.
The 50th DMO or 24th Vehicle mesh is stored, after which `AddVolume` returns
false and the rebuild stops; no 51st or 25th record is requested on that walk.
The native assertion path if a caller nevertheless enters with an already-full
array is not modeled as recoverable behavior.

`ClusteredMeshData::Reset` records the forward placement, computes its inverse,
and snapshots the ClusteredMesh KD-tree bbox. The dynamic collection's outer
spatial pointer is initialized null, so the gameplay line and box collection
specializations use mesh-record order before entering each mesh's local KD
tree. General line results retain the assembly pointer at `owner`; the
80-byte trajectory result deliberately does not retain owner.

Removal synchronizes the four helpers before mutation. It removes all records
whose owner pointer is the target assembly and fills a removed slot from the
last live record. That swap removal can change subsequent traversal and exact-
tie order. The executable does not prove a lock inside `DynamicPool::Remove`;
caller/thread discipline remains outside this slice.

## Provider-local trajectory execution

The aggregate does not choose one global line winner and then ask only that
provider for triangles. `AggregateTrajectoryBatch::AddTest` forwards the full
test to every enabled child. Each `ClusteredMeshTrajectoryBatch` worker builds
a `TrajectoryWalkFunctor` with its own collection, walks the complete
trajectory, and copies one complete `TrajectoryResult` to that child batch.

For every trajectory segment, `TrajectoryWalkFunctor::operator()` runs the
line specialization against that same collection. On a collision it stores
that provider's position, face normal, time, frame, and surface. When ground-
normal calculation is enabled it centers an AABB at that provider's collision
point, traverses that same provider again with the trajectory-box functor, and
calculates the ground normal from the resulting provider-local cache.

Only triangles passing the final world-space SAT reach the capacity check. A
prior count of 63 admits and stores the 64th triangle. The next qualifying
triangle observes a full cache, is not stored, and stops the remainder of that
provider's traversal. Each provider owns an independent 64-record cache and
can calculate a different collision point and ground normal.

After the child jobs are complete, `AggregateTrajectoryBatch::GetResult`
walks the provider array in insertion order. It ignores results whose time is
negative and replaces the current result only when the current time is
strictly greater than the candidate time. Exact ties therefore retain Ground
over DMO over Vehicle. The ordinary finite interpretation also rejects a NaN
candidate; exceptional VMX behavior is bounded below.

The mesh line functor likewise replaces only on a strictly lower clamped line
parameter. It copies the world `GPTriangle` face normal and the unsigned
16-bit surface from the high half of its big-endian user tag. It copies the
mesh record owner into `VolumeLineResult`, but only surface—not owner—is
propagated into `TrajectoryResult`.

## Readable aggregate wrapper

Commit `fea95b30c30a8a39b90306fc4f17079a0388b491` adds
`predict_ballistic_trajectory_across_providers`. The caller supplies a mutable
provider slice in Ground, DMO, Vehicle order. The wrapper executes the existing
complete readable predictor separately against every provider, retaining each
provider's line/box state and 64-entry cache behind its own
`TrajectoryCollisionWorld`. It then applies the recovered nonnegative-time and
strictly-earlier aggregate selection to each complete result. Equal times,
including either signed zero, keep the first complete record.

The wrapper does not discover providers, apply Island flags, construct dynamic
mesh records, or attach live bodies. It returns the first host geometry error
without querying later providers; native batches expose no corresponding
recoverable parse error, so this is an explicit safe-host policy. Tests cover
empty/negative/NaN results, signed-zero ties, complete-record replacement,
independent provider sweeps/caches, and provider ordering. A focused provider-
error short-circuit test has not yet been added.

## Required dynamic representation

The current application DMO inventory temporarily owns all required source
data in `CollisionArena`: aggregate roots, ordered SimpleMappedArray volumes,
parsed ClusteredMesh headers, and their original object bytes containing KD,
cluster, unit, and vertex allocations. `inventory_dmo_collision_template`
then calls `dynamic_collision_part` and
`collect_dynamic_collision_aggregate`, retaining only flattened primitive
volumes and expanded `MeshTriangle` values. The owning `CollisionArena` and
the following information are then lost:

- original ClusteredMesh/KD/unit backing and mesh dictionary identity;
- the leaf aggregate `Volume`, SimpleMappedArray ancestry, enable state, and
  exact depth-first leaf order;
- `AssemblyPartDefinition` inverse-body/static metadata;
- live `Part::GetTransform` values, provider kind, pool order, and an
  ABA-resistant assembly-owner identity.

Flattened triangles cannot reconstruct those values. The narrow additive path
is to keep each existing `CollisionArena` behind shared immutable ownership,
retain model templates as ordered assembly-part root indexes, and build a
provider snapshot containing mesh indexes, exact leaf volumes, current forward
and validated inverse transforms, local KD bbox snapshots, and generation-safe
owner identities. Existing flattened DMO overlap and grind APIs can remain
unchanged. Building a new immutable snapshot before a short atomic publication
and pinning that snapshot for each query gives the bytes and owner metadata a
safe lifetime equivalent to the native batch-sync boundary without retaining
raw entity pointers.

The current `RuntimeDmoCatalog` has the locator transform and model bounds;
`RuntimeDmoCensus` obtains an object ID and stable live/cull order. It does not
yet instantiate native-equivalent `rw::physics::Part` state. The serialized
root transform currently used by the flattened host body is not proof of the
live `Part::GetTransform` result, which can depend on the optional inverse-body
matrix and live rigid-body state. A native-shaped snapshot builder must accept
explicit current part-to-world matrices until that lifecycle is reconstructed.

There is no production Vehicle collision catalog/materializer yet. Existing
Vehicle inputs are synthetic or caller-provided flattened
`OwnedDynamicCollisionBody` values and do not contain original ClusteredMesh
backing. The reference Vehicle asset/recipe-to-assembly construction and live
transform/update/removal lifecycle must be recovered before its provider is
claimed connected.

## Readable retained structural template

Commit `fb971d8` adds `DynamicClusteredMeshTemplate`. Each template owns one
`Arc<CollisionArena>`, so the parsed `ClusteredMesh` headers and their original
KD/cluster/unit/vertex object bytes remain at stable allocations without a
second raw-byte copy. Ordered part descriptors retain the resolved root index
and the serialized 16-float inverse-body transform plus raw
`has_inverse_body_transform` and `is_static_body` words.

The builder samples the recovered enable policy, skips non-aggregate and
unsupported aggregate kinds, and walks SimpleMappedArray children in forward
depth-first order. Every ClusteredMesh occurrence is retained separately with
its exact leaf `Volume`; aliases are not deduplicated and child volume
transforms are not composed. An explicit heap continuation stack preserves
that order without a host call-stack depth limit. Active-path cycle detection,
unique dictionary-target validation, and typed malformed-resource errors are
safe-host policy rather than native recovery behavior. Changing the debug
ignore-enable policy requires rebuilding the template because native
`AddVolume` samples it on each rebuild.

Twelve focused tests cover part/SMA order, disabled parent/intermediate/leaf
handling, enable-before-target resolution, unsupported aggregates, aliases,
release-mode active-path cleanup, malformed local links, root-link
consistency, shared byte-allocation identity, and `Send + Sync + 'static`.
The real dedicated-DMO corpus independently establishes 93 local collision
models, 93 local Assembly targets, and 93 ordered parts with no ownership
errors.

## Application ownership and corpus gate

Commit `46255e1` moves each decoded collision arena into one `Arc`, constructs
one complete template per assembly-backed model, and stores the resulting
`Arc<DynamicClusteredMeshTemplate>` in the application DMO catalog. Runtime
locators clone the model template `Arc`, so 249 Rob's Park locator instances do
not copy the retained KD/cluster/unit/vertex bytes. Publication requires a
unique local-GUID collision owner, a valid local collision-model link, and
matching runtime Assembly, serialized AssemblyDefinition, and decoded vector
part counts. Missing, duplicate, nonlocal, ambiguous, or inconsistent
provenance is reported instead of selecting a first match or fabricating
metadata.

The release corpus gate requires every one of the 93 model templates and every
one of the 249 resident references to contain exactly one authored part and one
mesh leaf. It simultaneously freezes the pre-existing flattened totals at
14,451 model primitives, 13,865 triangles, and 35,626 resident primitives,
with no ownership errors or missing model IDs. The flattened path remains the
active legacy query seam; this slice only establishes stable structural
ownership and does not infer a live part transform from the locator matrix.

## Finite transform arithmetic correction

Commit `8b5ac39` corrects the readable affine helpers to the recovered VMX
operation graphs. `compose_collision_affine` uses one multiply followed by two
fused multiply-adds for each basis column and three fused multiply-adds
beginning with the outer translation for the translation column. A
noncommuting fixture proves the native `BodyPose * inverseBodyLTM` order:
translation `(8,21,33)` rather than the reversed `(11,-28,23)`.

`transform_collision_aabb` now starts each extent with the Y-axis multiply,
then accumulates X and Z with fused multiply-adds. Fixed finite discriminators
freeze composition lanes that differed from the prior helper and AABB outputs
that differed by one ULP.

Commit `2067242` implements the affine inverse written by
`ClusteredMeshData::Reset` (`0x827c83e4..0x827c84ac`). It preserves the cyclic
ordinary-multiply/fused-negative-multiply-subtract cofactors, native transpose
and duplicated cofactor W lanes, the two fused Newton refinements, bitwise
translation negation, and multiply/FMA/FMA inverse translation. It has no
invented singularity branch. The deterministic host seed is `f32::recip`, and
the determinant uses the documented nested XYZ host model. Exact Xenon
`vrefp` seed, `vmsum3fp` reduction, denormal, NaN-payload, and signed-zero bit
parity remain explicit boundaries. A reciprocal fixture also discriminates a
fused refinement (`0x32800001`) from the defused alternative (`0x32800000`).

## Proven live DMO placement boundary

The retained catalog matrix is not sufficient to publish a native live DMO
provider. `LWDynamicObjectLocatorMan::AddLocatorsForStreamedLocatorChunk`
(`0x82d3be90`) initializes current and initial placement from the streamed DMO,
but persistence may replace it. `CensusMan::SpawnDynamicObject` (`0x82726c08`)
passes the live locator transform into `LWDynamicObjectCensusMan::Spawn`
(`0x82d31ba0`), which initializes models, sets the target, and adds them to the
world.

`FillCollisionPartTargetTransforms` (`0x82e5f8d8`) starts from that target,
calls the DMO-specific collision-body shift when not pipe-originated, and then
adds velocity times `0.016666668`. The concrete DMO collision vtable at
`0x8232cd88` maps slot `+0x58` to `0x82d2ba40`; that function returns an
all-positive-zero vector with no field or vault read, branch, or fallback.
Consequently the call has no displacement effect for this concrete type.
`cLivingWorldCollisionBody::AddToWorld`
(`0x82e50cb0`) calls `Body::SetPosition` (`0x82de8090`) for parts in order
before `Island::AddDMOBody` (`0x827b6798`). At this point `Part.m_body` is null,
so `Part::SetTransform` (`0x82b25480`) copies each explicit matrix into
`m_partTM`.

`AssemblyPool::Insert` (`0x82e0dbb0`) calls `Assembly::AddToSimulation`
(`0x82ae0df0`) before serializing the assembly. `Part::AddToSimulation`
(`0x82ae0aa0`) uses `m_partTM` directly when there is no inverse-body matrix;
otherwise it computes `m_partTM * rigid_inverse(m_inverseBodyLTM)`. That inverse
is the native unchecked rigid transpose, not the general Reset inverse: it
transposes XYZ, creates positive-zero basis W lanes, negates translation with
arithmetic `+0 - value`, and reduces the new translation Z then fused Y then
fused X. The following composition evaluates all four lanes and preserves
noncanonical W results. Requested body state `4` becomes `1` only when Part
internal-flag bit zero is set. `Simulation::AddRigidBody`
(`0x82ae6e00`) orthonormalizes that seed and binds the resulting live body.
Thereafter `Part::GetTransform` reads the body pose, composing
`BodyPose * m_inverseBodyLTM` when present, and dynamic simulation—not the
catalog locator—is authoritative.

`OrthoNormalize3x3` (`0x82ae33b8..0x82ae3750`) has now been recovered at the
ordinary-finite source level. It independently normalizes all three axes with
`vrsqrtefp` and two Newton refinements. Independent duplicate scalar-length
chains use two refinements for X/Y but only one for Z, and exact-zero squared
lengths select positive zero only after estimate work. The function measures
all three absolute pairwise correlations and uses strict comparisons to retain
the cyclic pair with the smallest correlation. An all-equal tie retains X/Y;
`qX == qY < qZ` retains Z/X; every minimum tie involving qZ retains X/Y. It
recomputes the missing axis as normalized `A x B`, then recomputes B as
normalized `M x A`. The native fused cross can retain a W residual, all four
axis lanes are scaled, and all four translation bits are copied unchanged.
There is no epsilon or finite guard. Commit `5f201ee` ports this graph with
explicit host replacements for the unrecovered Xenon `vrsqrtefp` seed and
`vmsum3fp` reduction. Denormals, NaN/infinity behavior, signed zero, and exact
Xenon-dependent W bits remain boundaries.

Commit `5f60888` ports the pure Part attachment transform and state override.
The later runtime transaction first adds Parts, then Joints, then Drives. A
body failure rolls prior Parts back in reverse; a joint failure rolls prior
Joints back in reverse and then Parts forward; a drive failure rolls prior
Drives back in reverse, Parts forward, then Joints forward. Every Part
copy-backs `GetTransform` before removal and clears its body pointer afterward.
Normal removal is Parts, Joints, Drives, all forward. Its Part portion now uses
a private reusable positional attachment owner, allowing a future serialized
DMO bridge to share the exact transaction without manufacturing a clustered
mesh template. That refactor still does not claim a live DMO attachment.

## Evidence artifacts

| Artifact | SHA-256 |
|---|---|
| `sk82_na_m.xex` (debug-symbol set) | `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2` |
| `sk82_na_m.pdb` | `0b060cec3caa2f174999ebfdec1d7c90cbcb212815e7933e68d273d687d56ef0` |
| `sk82_na_m.map` | `51e60f0993c40ff1e473f3d28e1e2f954dfec605a785ca2ee02320c8b8997e88` |
| `dump_vehicle_provider.py` | `e507081f420ebdca337226beca9be26f8df6725e493fd01629d0a35da580f6d3` |
| `vehicle_provider_raw.txt` | `3fb68bc3f540f1ba6eae3ead7a577111a3386c2ab6cc252e3bf3378f40f2c772` |
| `dump_dmo_provider.py` | `b41993f874667ea0543878a8aa3717aab5960678cf22985e3971f0f19e0cf652` |
| `dmo_provider_raw.txt` | `fbccc7ae83588a0e3c21c0664d664dbe6e67f9666c63c85626746880d7328c81` |
| `dump_trajectory_orchestration.py` | `18b982e8cfce3a5a02882c59a94ba82eb2b448f805b1605e35c87df730039b50` |
| `trajectory_orchestration_raw.txt` | `9114ee56690df3ab2b3f455675fe426f8fcb86ddc55509b0a4f896d28ee11659` |
| `dump_outer_collision_line_followup.py` | `e165689f63f44f612ea0d750a5d9537fdab86cff4373a4f02ba52dd6493f9717` |
| `outer_collision_line_followup_raw.txt` | `a57ded62314111b33a8e570fe6ce328198bafbf8d852ce4a49aa720704e456df` |
| `dump_dynamic_transform.py` | `7164aeab2b98b02cb0a26bcf13578694a01b1a52a2e8ea47106e96e528f402e3` |
| `dynamic_transform_raw.txt` | `a0b0264cbbe7f52d432be303effa0d2edc4bbfa5addf2ba639e27b741e9ef895` |
| `dynamic_transform_evidence.md` | `a527df52e1c8336507f0cd75df125973113f512a61cd2fe801e21ca6a74927c8` |
| `dynamic_transform_discriminator.rs` | `0df57690949fb5bfec0a27d1a8c95341cf3cabea23d837c50335db26659a6569` |
| `dynamic_transform_discriminator.txt` | `bb0519c37eeee946dbb06a3a5b4d2131de2b66db1dcd84f00fea29c14b0bd635` |
| `reset_inverse_discriminator.rs` | `81d9bc14a01adef8cdf5bc9e8d11fda13b9b184b76a83bdd1d457678aaa43abe` |
| `dump_dmo_live_placement.py` | `98ae340e428e2cdbcfcd51cde61fb93564a1778fd571326feff683f90ba57abd` |
| `dmo_live_placement_raw.txt` | `0ff8604d86bdd60dfd12b5452e7ea3a17d0d6bd1fb2cb9d921340be8d87362d8` |
| `dump_dmo_body_binding.py` | `8c4a896fc9a2e26613a20c6f2ba07aece979fd6a0a50c1a2aa07e969f213808f` |
| `dmo_body_binding_raw.txt` | `8ba5d8692b87b688693b0e9e0e19a8354da7c4012416ce654a5a5458118f4e8d` |
| `dump_dmo_shift_orthonormalize.py` | `df15bd5fd457c2bba07cca1b6f231bb07802225958be794eb2356acd15f916a7` |
| `dmo_shift_orthonormalize_raw.txt` | `590bb7c9cd8307ff55c27c3ef2d52286ce5cc5caa9147a6b70d552b2530f3c48` |
| `orthonormalize3x3_evidence.md` | `12d4b3e8db1f6d09e7bc56f2a9f14b87876351812da829c002c40f9307cf2d92` |
| `orthonormalize3x3_discriminator.rs` | `ee217b71042671335696763c08ebf075af1de4fd5bddef132787ae9c3d59b2de` |
| `orthonormalize3x3_discriminator.txt` | `5559cb1f3a6b68a4e2b457de09b1bd933286029fec28a993de942a37cd3aeb18` |
| `dump_part_add_remove_inverse.py` | `60b8d458b06b65cc710a012c5949d85ca3eb60629ba4be1a879cb682bb0f9a00` |
| `part_add_remove_inverse_raw.txt` | `224245a66f24c938e26e67fb436fff8515be9b64ca96069289980bcca1f3e95f` |
| `part_attachment_discriminator.rs` | `52baaddae34ca6731744cdd702d0d5a6354b99340db91061940fc918474f8dc7` |
| `part_attachment_discriminator_output.txt` | `cb63183670a778cfde1e1a4f29ea32a4e3c4f9ee3a7ed75f104743d70e17275b` |
| `dump_assembly_transaction.py` | `850242f479555d812e4e61e51f3cd16c46aa916ad44ea37dca77e273a42a42e3` |
| `assembly_transaction_raw.txt` | `71d2d6f7f938d69b15d1e4ae510a3004ad3e3d606934264d458489b89ba2255e` |

The extraction scripts and raw reports are temporary research artifacts under
`world_probe_temp`; they are not shipped runtime dependencies.

## Boundary and confidence

Confidence is exact for named symbol addresses, PDB sizes/offsets, the integer
Vehicle gate, provider order, ordinary control-flow order, same-transform SMA
recursion, fixed capacities, provider-local line/box/cache execution, result
field propagation, and strict aggregate replacement.

The readable claim is source-level for ordinary finite inputs. Exact Xenon
matrix inversion, `vrefp`/`vrsqrte`, fused VMX arithmetic, `vminfp`/`vmaxfp`
ties, NaN payloads, denormals, exceptional signed-zero behavior, job
interleavings, allocation/assert behavior, corrupt-resource behavior, and
native error recovery remain outside this result. Safe Rust snapshot
publication, generation identities, typed errors, and first-error short circuit
are host policies, not claims about native recovery.
