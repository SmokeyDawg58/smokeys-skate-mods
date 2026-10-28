# Normal skeleton limits and constructor-owned collision data

2026-09-07. Confidence **high** for source control flow and ordinary host
arithmetic; **exact** for raw words, named offsets and source pointer routing.
This extends `skeleton-joints-and-normal-authority.md`; it does not complete
SkeletonBody admission, the whole SetUpNormal routine, or playable skating.

## Connected joint-limit path

`SimulationJointPool` now retains either the previous immutable Arc or a sealed
`SimulationMutableJointLimits<L>` owner. Both use the original single Add/Remove
graph. Add locks mutable ownership only for host authority preflight; it does
NOT copy/read the source limit value as a physics operation. Empty capacity
returns before this inspection. First admission binds the owner permanently
to one actual Joint pool. All writes require exclusive pool access, a live
occurrence and pointer-identical retained owner. No independent write capability
or writable guard escapes; another pool cannot mutate an alias.

Existing generic immutable API signatures and Arc identities are preserved.
The borrowed immutable binding accessor explicitly refuses mutable storage;
`try_limits` reads current copied values. Pool snapshots remain topology-only,
so poisoned limit storage does not prevent snapshot or removal. Host checks do
not invent native failure recovery or pointer ownership.

The actual `simulation_joint_preparation.rs` consumer samples this owner at
its existing limit-read point, after B/A state filtering. Its prepared batch
retains `&JointPool` for its entire usable lifetime. Combined with sealed
`&mut`-pool-only writes and permanent pool binding, that prevents mutation
between preparation and solving without adding an unnecessary epoch. Horus
uses already compiled coefficients, not a live limit pointer.

Source lifetime chain independently rechecked:

- `Assembly::InitializeComponentsFromDefinition`, `82ae0bf8`, selects the actual
  definition row; `JointedPair::Initialize`, `82ae8f78`, retains frames at+8 and
  limits at+c (store at `82ae8fa4`).
- `Assembly::AddToSimulation`, `82ae0df0`, passes these same pointers to
  `Simulation::AddJoint`, `82ae4358..82ae43ec`; limits stored at Joint+4 by
  `82ae43c0`, with no value snapshot.
- `JointBatchBuild` reads B state then A and requires `(B|A)&4 != 0` before
  Build. `JointJacobian::Build` captures Joint+4 at `82ae5964` and reads selected
  fields through that pointer during compilation.

### SetNormalJointLimits

`skeleton_normal_joint_limits.rs` performs the actual live writes through those
pool owners. Reference `?SetNormalJointLimits@SkeletonBody@Physics@Sk8@@AAAXXZ`,
`82b3bb90..82b3bd04`, SHA256
`64e358ab6993f1b4b69e20964ad98c5b107008689f7238f318e39167b7d4adc3`.
Root and reviewer independently read its full raw body this checkpoint.

Loop24 Parts in order, skipping absent physics parents. Validated SkeletonData
cannot contain the additional native parent-bone -1. Each eligible Part consumes
the next dense attribute/limit row regardless of enable, BallJoint, earlier
emission-map or allocated-count values. Compute BOTH products before either
cosine: swing=row.swing_angle*bone.swing, twist=bone.twist*row.twist_angle.
These are the opposite operand orders from InitJoints.

Native re-resolves Body+1c -> AssemblyDefinition+c for each row and retains the
selected row across both calls. Preserve the callback-visible store sequence:

1. Select original swing product or epsilon `3c23d70a` using product-epsilon
   >=0, and store swing angle at+28. Unordered selects epsilon.
2. Call cosine.
3. Store swing cosine at+30, then selected twist angle at+2c.
4. Call cosine.
5. Store twist cosine at+34; advance dense64/attribute20.

All other fields are preserved. Host numeric failure retains completed writes,
including current-row partial state and earlier rows. Missing initialized rows
are refused at that row; no native unwritten tail is filled with invented data.
The two adjacent stores in step3 are a single locked Copy update in Rust;
callback-visible prefixes match, but concurrent observation between those two
native scalar stores is not claimed. Source array rebinding/reentrant physics
mutation is excluded by the retained immutable inputs and exclusive pool borrow.

Eight mutable-pool tests and six normal-setter tests cover storage identity,
generic Copy payloads, FIFO lifetime/reuse, capacity, poison, stale/foreign/wrong
owner refusal, exact callback-visible prefixes, numeric failures and untouched
fields. Two actual preparation tests additionally prove changed normal limits
change the actual compiled Joint record, match independently specified final
values through the accepted immutable Build path, and state-filtered poisoned
limits remain unread. Selected poisoned limits fail before numeric/RF effects.
This comparison proves new storage plumbing, not independently re-proving all
Build numerics by running the same algorithm twice.

## Collision data: genuine initialized fields, not fabricated references

`skeleton_collision_data.rs` implements constructor-owned semantic data plus
selective Reset. It is deliberately NOT the whole native reference-bearing
SkeletonCollision object. The eventual physical owner must retain its real
SkeletonBody and SkeletonState relationships. No integer IDs, `Arc<()>`, copied
addresses or placeholder body/state objects stand in for them here. Native
padding is not represented. Construction returns only complete data; native
partial initialization/publication and allocation/assert behavior remain outside
that boundary.

Source identities, independently checked by author/root/reviewer:

| Symbol (`Sk8::Physics::SkeletonCollision`) | Range | SHA256 |
|---|---|---|
| Constructor | `82b4c700..82b4c804` | `7660d2927a4d9496df447fc3249447334fcebe9c138e83edf8d683658e100452` |
| `InitSpecificBodyParts` | `82b3ec78..82b3eeac` | `997708acab24c99ad98648542ca276e618d18f85669386884360c94e6e8ffbd3` |
| `Reset` | `82b3acc0..82b3ae5c` | `929bece7c094c7f8333ec8268d3bb601adcabb4e556f65f3f8dbf7309ace9fb9` |

Constructor called at `82b62374`, before Body Init. It initializes20 NormalInfo
entries (normal full4 zero, PartIndex0),24 BoneData records, stores real State
then Body references at+f90/+f94, initializes specific parts, clears24 compliance
bytes, then calls Reset. These initial zeros are proven constructor stores,
not assumptions about allocator memory.

PDB sizefd0. NormalInfo[20] at0 stride32; times24 at280, priority24 at2e0,
justCollided24 at340, compliance24 at358. Eight-channel force/friction/slide/
audio arrays at370/390/3b0/3d0; normals8 at3f0; vertical/toe normals470/480;
contactBone8 at490. BoneData24 at4b0 stride112. SpecificBodyParts atf30 size96.
Named scalar/status suffix startsf98, numNormals atfc8.

BoneData contains five full4 vectors at0/10/20/30/40: contact normal, subpart
normal, slide vector, subpart slide vector, contact position. Forces50/54,
surfaceIDs58/5c, hitVehicle/Ped/Skater/World bytes60..63; padding64..6f omitted.
NormalInfo has normal0/PartIndex10 and12 omitted padding bytes.

SpecificBodyParts retains Parts[Hips23,Neck1], two full4 world positions, two
full4 offsets, two squared radii, and contact/recent-contact bytes. The last
four bytes have NO stores in constructor/helper/Reset. Fresh values are
`[None;2]` for each pair, not invented false. Later explicit raw byte values
are preserved by Reset.

### Original Vault fetching for specific parts

`skeleton_collision_attributes.rs` retains actual immutable global
`physics_collision.default`: class `7379c4467aa75df9`, collection
`d7edbd362d7d2152`. Generated constructor `828b1338` selects cached112-byte
layout or generated zero default. Layout offset0 is GroinLocalOffset, +10 is
FaceLocalOffset; remaining80 bytes are unused here. Two separate cached-pointer
reads copy ALL four lanes. No singleton replacement/editor reload claim.

For groin radius key `a945308cde002e9c`, look up first pointer, then second,
then LOAD SECOND followed by first, and multiply second*first in f32.
Repeat for face key `a8ed92d201bee7dd`. Each missing pointer independently
selects zero scalar default. Do not collapse duplicate reads into one squared
value; typed pointer/read operations preserve delayed sampling. Malformed
scalar widths are errors. Finally store two world positions full4 zero.

Independent original db.big oracle: VLT collection5c040, parent0; slot5c068,
PtrN b3798 under target dependency0 -> dependency1 BIN9b420. Raw pointer record
`0005c06800030001000000000009b420`. Full112 layout SHA256
`fe60a46766788b8a1911354e72da567ed959e54379dac3d4eb4d69e5583de3bb`.
Groin vector `[be19999a,0,0,0]`, face `[0,3d8f5c29,0,0]`.
Inline node5c140 has radius `3dcccccd`; node5c0e0 has `3e19999a`.
Independently rounded squares are `3c23d70b` and `3cb851ec`. Reviewer reread
the archive and recomputed them, not production printouts. Runtime values
remain fetched; these constants occur only in test oracles.

### Selective Reset

Clear hitVehicle/hitWorld/opposingContacts first. For24 Parts in order clear
time, priority, justCollided; five BoneData vectors; surfaceID, force,
subpartSurfaceID, subpartForce; four hit bytes. For8 channels in order clear
force, friction, slide, normal; contactBone=-1 (typed None), audioSurfaceType0.
The named suffix follows native scalar/byte order in the Rust method, including
the source's repeated hitVehicle/hitWorld/opposingContacts clears. NumNormals0;
vertical and toe normals become original full4 J `[0,1,0,0]` at8227e900.

Do NOT clear compliance, NormalInfo, SpecificBodyParts or retained references.
This preserves the known distinction between SetUpNormal's reset and the
constructor's later TeleportReset: the latter clears just-installed priority
but preserves compliance. Four unit tests cover exact10-service read order,
every failure prefix, exceptional/raw radius arithmetic, and every Reset target
seeded with nonzero/raw-NaN data plus preserved fields/raw bytes.

## Original-data and integration boundary

Existing body corpus checks the original collision constructor once, and normal
limits for all hat0/4/1 variants. Original22 frames/limits are bound to22 real
mutable Joint pool slots;24 body handles use explicitly DIAGNOSTIC identity
poses/default inertia solely to provide real endpoint capabilities. These are
not source SkeletonBody assembly admission/placement, collision publication,
SkaterPool membership or a physics tick. The test checks44 cosine arguments,
all16 limit words, current Active reads, frames/endpoints, unchanged topology/
body state and explicit body-before-joint cleanup. The known pose provenance
boundary from the preceding checkpoint remains unchanged.

Still required: integrate mutable inertia/limits into the ONE actual shared
SimulationAssemblyBackend and source Assembly transaction, retain mutable
material/volume group/enable/userdata and full4 inertia state, create24 actual
Parts, preserve group1/Active4/spies/rootZero material/SetUpNormal ordering,
then genuine Skeleton hook/drives and player/state/input relationships. Do not
create a second private physics world or install final values before insertion.
The collision data owner belongs embedded in that eventual concrete owner;
its existence is not evidence those missing references/callers are connected.

Root's final raw/PDB callsite check corrected older handoff terminology:
`82b621c4` writes Joint+1c = `m_spy` to1, not `m_tag` (+18). Joint PDB size32
and AddJoint's spy-zero store at `82ae43b8` confirm this. Future setup must
change actual spy storage, leaving tag untouched.

## Final validation

Gate77928 EXIT0: formatting check, workspace2248core+187formats+38app+3symbols
=2476 unit tests and6doctests, all-target warnings-denied Clippy, ALLSEVEN
ignored core original-data tests, and git whitespace check. App ignored corpus
not rerun.20 new unit tests:8pool,6normal setter,2actual preparation,4collision.
Focused82729 (6normal+8pool) and13844 (all33preparation) also pass. Initial
full test run35102 passed tests, then reported doc-markdown/float-test lint
issues; corrected only formatting/test annotations, not physics arithmetic.

Original board ground counts/centers remain unchanged (C/J/D6/6/3 for first
three frames,4/6/3 fourth). Separate8-frame DMO test retains contact hash
`0e7c704e6a10f08a` and final COM `[588.8619,-54.40114,996.5048]`.
Independent acceptance PASS for source/production/all focused tests/
corpus/evidence. Reviewer corrected one doc sentence to clarify that collision
construction is tested once, normal limits in all three body variants.
