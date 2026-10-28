# Live Skeleton normal setup and reset

Follow-on integration: [Skeleton collision and mixed pool](skeleton-collision-and-mixed-pool.md)
connects the current materials/headers to actual publication and shared ground
consumption. The implementation boundary below records the earlier checkpoint.

2026-09-07. Confidence **high** for recovered control flow/ordinary host
arithmetic, **exact** for copied raw words, offsets and field selection.
Extends `skeleton-pre-normal-assembly.md`. The same admitted 24-body/22-joint
assembly now receives normal setup, not a second diagnostic body pool.
This is not full physical-player construction or a humanoid physics frame.

## Actual retained owner

`dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly` owns the
existing session and one `SkeletonRuntimeState`. Its factory consumes actual
source body/joints, the existing normal culling resource and completed
collision data. The state retains the same binding. No mutable session/pool or
body capability is exposed. Detached/foreign operations refuse before effects;
AlreadyAttached refuses before a repeated group1 write. Existing source
attachment/rollback/body-first removal remain unchanged.

The wrapper records group1/all Part userdata1 before Assembly admission. A
separate property slice writes actual Joint spy=1, then selects root zero
material. It explicitly does NOT stand in for the preceding island/body-owner
and per-Part registration calls; those still require their real runtime owner.
There is no fake native Body/State reference in the collision-data projection.

Each Part has one live material selector: Definition, Body-owned Zero, or
Body-owned Normal. All normal-selected Parts resolve the same retained
material address. Selection does not overwrite definition material, and the
initial template's copied material is not presented as current.

One live header per top-level Volume owns current flags/group. Real geometry
allocations retain their initial header evidence and immutable shape data.
Hat changes address the outer Aggregate only. Every future humanoid collision
publication must use these current headers/material selectors; that consumer
is not yet implemented. Flattened hat children are the important exception:
their own flags remain authoritative, with no outer-header propagation (see
the next-publication packet below). Assembly group, Part userdata and Volume group remain
three distinct fields.

## Full inertia storage, unchanged solver caches

`SimulationMutableInertia` now retains an optional W lane in its SAME private
locked storage. `new_packed` receives the producer's explicit W; generic XYZ
owners retain None. Skeleton binding initializes actual packed owners from
current `BodyMassState`, not a saved mass cache. Public read-only payload
`try_read_packed` returns None where no producer supplied W.

Existing scalar/XYZ writes preserve W. Checked packed writes require a live
same-pool occurrence, pointer-identical owner, existing known W, and unpoisoned
storage. Value/W update under one lock. Missing W returns
PackedInertiaUnavailable without promoting or changing the owner. Authority
remains permanent after removal. No writable guard/callback is exposed.

Normal setup uses staged writes for inverse mass, full tensor, intermediate
spherical minXY, then final reciprocal minZ. Body cached inverse mass/rotated
tensors, pose, velocity, force, solver deltas, rings and occurrences are NOT
refreshed by these referenced-inertia edits. Scalar/whole-payload atomic host
writes omit native concurrent intermediate observations; callback-visible
source stages and failure prefixes remain distinct.

## Source chronology

Root and independent reviewer read the live symbolized IDB. Existing raw range
hashes remain in `skeleton-joints-and-normal-authority.md`:

- `SkeletonBody::SetUpNormal` 82b50508..82b50720, SHA256
  6877eab50f42157bdf44711a1d3fdc349ac726eed9cc8d448def066c71650557.
- `SetUpNormalCollision` 82b50400..82b50508,
  4dc1b4933aec1c7bf249e8dd2d0d288f5fbf8e1412732c9df579a9fa782165e0.
- `SetUpNoCollision` 82b47640..82b476ec,
  29a965fa5ebb9196db29a502ed6c06b730fcdbe164b3b5f1914121f8564c7da8.
- `TeleportReset` 82b3bf88..82b3bffc,
  7d402450fbac36c254b3627f6c16d78f3723e54478077066305c8095c89b6ab8.
- `Body::SetAngularDrag` 82de7f38..82de8090,
  acdb4602e11b58de08904a2706193cc642f78a733699b31067bbfc24f4fb7b5c;
  `SetLinearDrag` 82de7de0..82de7f38,
  900ccf4b68c98689631e9e91d41c196e4b480bde7c8beada594432b51e67deac.

Represented SetUpNormal order:

1. Actual dense normal-limit writes through existing admitted Joint owners.
2. Select retained normal culling, Assembly group6, ascending all-Part userdata6.
3. Selective Collision.Reset.
4. Angular drag0/-1, then linear drag0/-1. Both compute0*426fffff; six groups
   of four pointers/ascending stores. No body-state filter. Host preservation
   copies are sampled before each group's stores, with explicit lock errors.
5. Literal Parts1..23: Normal material pointer; cached animated inverse mass
   at505f8; full tensor at50614; spherical minXY interim store5062c then
   reciprocal minZ50648; raw compliance5065c then priority5066c.
6. Root compliance1 at506c0.
7. Fresh LOCAL Skeleton byte043f9227035de245 at506d0, missing default byte0.
   Nonzero selects NormalCollision; zero selects NoCollision(true).
8. isRagdoll=false at5070c, then folded OnEnd, verified BLR.

Both minima keep the left operand ONLY for strict `<`; ties and unordered
select the right. W is copied without arithmetic. Existing immutable Vault
provider supplies the original final byte; the raw original VLT node5dd50 is
`043F9227035DE2450100000000034000`. A corpus oracle read does not replace the
production method's later fetch. Native reload/mutable Vault lifecycle remains
outside this provider's accepted scope.

NormalCollision first repeats group6/all userdata6, sets root Volume group4,
clears root enable bit and sets root counter0. Parts1..23 with counter0 enable
their Volume and set group0; nonzero counters, including negative, preserve
the header. EVERY loop Part writes userdata6 again after that branch. Other
flag bits remain unchanged. The branch-only OnEnd is also verified BLR.

NoCollision always makes root group4/disabled/count0, disables Parts1..23 and
resets their counters only when its argument is true. It changes neither other
groups, Part userdata, culling/material selection, collision data nor the
disable-count-active byte.

## Known and unknown constructor fields

PDB: disable counters are signed int[24] at Body+1e00; active bool+1e60;
isRagdoll bool+1e61. Live source confirms ctor82b62590 stores isRagdoll=false,
before InitAttribData625b8/Init625c4. Constructor SHA256:
8dd1b36ab7ee3920c57f3e88de88025327b2a2c59c068575607c239793881e73.
The nearby22-element clears end before+1e00; counters and active remain unknown.

Counter None is not silently zero. For NormalCollision, an unknown counter
can proceed only if enabled=1 and group=0 already makes BOTH branches have the
same resulting header. None is retained; no native counter value or exact
counter-read/conditional-store trace is claimed. A divergent unknown branch
returns UnknownDisableCount with completed earlier effects retained. Actual
original fresh variants satisfy this bounded semantic projection.

Later constructor TeleportReset at625cc is separate: isRagdoll=false,
Collision.Reset, all24 counters0, active=false. It does not change live volume
headers/materials/culling/group or compliance. In particular it clears normal
collision priorities just installed while retaining compliance and configured
specific-body parts.

## Validation

Final **68077 EXIT0**: formatting check; **2273 core +187 formats +38 app +3
symbols =2501 unit tests**, six doc tests; all-target Clippy `-D warnings`;
ALL SEVEN original core corpus checks; git diff check. App ignored probe was
not rerun. Existing board Rob's Park C/J/D and all four centers, plus the DMO
eight-frame result, are unchanged from checkpoint2486.

Fifteen new unit tests: four packed-inertia, five runtime-state and six actual
normal-owner tests. They cover raw W -0/NaN, identity/authority/stale/poison
refusals, no cache/RF refresh, material reference identity, negative/unknown
counters, flag preservation, selective reset, early cosine and late attribute
failures, detached/foreign/AlreadyAttached checks and spherical right-tie/
unordered behavior. Existing limit-service tests additionally check staged
cosine-visible limit prefixes.

Original hats0/4/1 each construct ONE fresh normal owner, admit its actual24
source bodies/22 joints, then run properties/normal/reset before teardown.
The test checks44 cosine arguments, all16 actual current limit words, full20
frame words, all24 packed inertia owners/root preservation/23 saved animated
values, unchanged body caches/topology, source materials/headers/compliance/
priority and unchanged hat children. Collision-data clones are explicitly
independent variant fixtures, not cloning a live mutable runtime owner.

Independent source/code/test/corpus review passes. Original data plus accepted
host kernels prove routing/integration, not captured native-tick float parity.
Next: actual humanoid collision publication and real shared SkaterPool/world
membership, while retaining this owner and its current state. Registration,
SkeletonState/Body relationships, Skeleton hook/drives, physical-player/input/
animation scheduling, world pairs, camera and renderer remain incomplete.

## Next publication/membership source packet

Read-only source/caller audit after the gate selects a skeleton-specific
publication followed by a correlated mixed owner in the EXISTING SkaterPool.
Keep the entire `SkeletonNormalAssembly`, not just its session. Publication
must happen BEFORE normal setup: fresh group1/Active admission -> initial
Create -> spy/root material/normal properties -> invalidation and refresh.
Do not insert an already-normalized body and call that native construction.

Native authored selection emits24 records for no hat,25 with hat, from24 actual
body observations. Hat is a real SimpleMappedArray2 created82b4fbc0 and wrapped
as Aggregate82b4ff98. Create82e0ca10..ca9c/Update82e0d050..d0bc flatten Cylinder
then Sphere, both retaining the same Part/body. No wrapper record is emitted.
Counterintuitively, each child supplies its OWN enable flag; outer Aggregate
enable is NOT tested or ANDed. Normal/NoCollision outer-header changes must not
be propagated into serialized child enables. Add a direct discriminator.

Primitive Create copies actual geometry with current top-level header; hat
Create copies actual unchanged child Volume. Material resolves current Part
selection; record group is Part userdata, not Volume group. Part GetTransform
includes retained inverse-frame metadata; do not apply hat outer transform a
second time. Update rewrites enabled/material/transform/group/frozen only,
preserving Create inline Volume snapshots. Assembly header Update samples
current group/culling at82e0d2d0/d2d8. Bounds retain existing0.05 padding and
primitive/swept kernels, including disabled/frozen/static selections.

Concrete restrictions to remove coherently: board-only candidate/member in
`dynamic_assembly_skater_pool`; board publication iterator in
`simulation_assembly_ground_job`; board-specific Authored publication reference
in `simulation_ground_contact`; board refresh/accessors in ground/pool session.
A closed board/skeleton read-only publication reference can reuse the SAME
ground traversal/contact compiler, without weakening arena-image provenance.
Correlate each complete owner with its publication in one member variant;
shared handle/index/bounds may remain outside. Board-only truck/hook methods
must reject a skeleton member. Removal returns the entire normal wrapper.

Pool semantics remain index -> attach -> Create -> commit, failure preserves
the real candidate/index/attached state, then source detach/swap-last removal.
Refresh all invalidate -> all Update -> all bounds -> all publish -> active
scan; ground consumes FULL dense membership. Invalidate before any normal
mutation, even one that later fails in the SAME frame. Shared manager owns
index allocation: native constructor ordering board/hook/skeleton yields
board0/hook1/skeleton2 in the pristine tested manager; never hardcode indices.

Next source ranges SHA256: Create82e0c850..82e0cf2c
c29dbd5dc8928a10c82c41e4fbffcf340cc6554689703342db074b35280283e2;
Update82e0cf30..82e0d35c
113a7064f503b753d0f45bd36d4102a000b2730381c1a9862b4873121f64aa91;
CreateHatVolume82b4fae8..82b4ffd4
fd6a093fda92509d7a1c09d8f9e8e134edcdbe70e5e103a48436cd48945831cb.
This packet is next-work evidence, not implemented publication/membership.
The independent reviewer also confirmed the child-only flag reads directly:
Create ca8c -> enabled ca9c; Update d0a4 -> enabled d0bc. Final documentation
scope review passes.
