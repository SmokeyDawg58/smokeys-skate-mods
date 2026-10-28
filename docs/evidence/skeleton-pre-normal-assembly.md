# Actual pre-normal Skeleton Part/Joint admission

2026-09-07. Confidence **high** for recovered control flow and ordinary host
arithmetic; **exact** for reference stores, raw constants and pointer routing.
This extends `skeleton-normal-limits-and-collision-data.md`. It is a connected
construction/admission boundary, not complete SkeletonBody initialization,
SkaterPool publication, a humanoid physics frame or playable skating.

Later checkpoint2501 connects actual live normal setup, full packed inertia and
runtime material/Volume state; see `skeleton-live-normal-setup.md`. The remaining
authority list below records the earlier checkpoint, not today's code boundary.

## One backend, retained immutable and mutable inputs

`SimulationAssemblyBackend` now uses opaque `SimulationAssemblyInertia` and
`SimulationAssemblyJointLimits<L>` payloads. Existing immutable board inputs
and accepted mutable skeleton owners use the SAME rigid-body, Joint and Drive
pools. Conversion moves/clones ownership without reading a mutable value.
Each add dispatches directly into the already accepted pool method; no new
transaction, numeric kernel, borrowed backend adapter or second world exists.

Board definition producers still return their original immutable Arc payloads.
The binding boundary wraps them; hook inertia retains its exact original Arc.
Source Joint frame/limit identities are checked in the mounted board corpus.
The separate serialized arena/image-routed backend is unchanged.

Empty body capacity still precedes inertia inspection; Static admission skips
it. Joint capacity precedes parent/child validation, which precedes mutable
ownership preflight. A native-shaped empty pool returns None without a
diagnostic; a host refusal returns None with the existing typed diagnostic.
Session rollback and successful body-first teardown remain unchanged.

Host policy: first mutable admission permanently binds an owner to one pool.
Rollback/removal does not release that binding. Retrying in the same backend is
valid; another backend refuses the bound owner. No writable guard, independent
owner mutation, fabricated body capability or pool replacement is exposed.
Public `as_immutable` explicitly returns None for mutable inputs; `try_read`
returns a fallible current Copy value, never an immutable-looking stale view.

## Original constructor placement and metadata

`Sk8::Physics::SkeletonBody::Init` (`82b61738`) creates its definition at
`82b61878`, then the Assembly at `82b620d8`. The latter calls
`PhysicsUtility::CreateAssembly` (`82e4a540`) -> `Assembly::Initialize`
(`82ae1338`) -> `InitializeComponentsFromDefinition` (`82ae0bf8`). Parts are
initialized ascending, then Joints, then Drives, then the culling pointer.
No placement call intervenes before island admission at `82b6213c`.

The definition creation path `82e4a4d8` -> `82ae13f0` initializes each
PartDefinition via `82ae9058`. It explicitly writes hasInverseBodyLTM+8c=0
at `82ae90b0` and isStaticBody+90=0 at `82ae90d8`. Inverse matrix storage+40
itself is unwritten while absent. There is no SkeletonBody static override.
Source constructor material is static .5, dynamic .3, restitution .4; later
body definition construction supplies its actual retained material and mass.

`Part::Initialize` wrapper `82ae9820` binds definition volume+0, inertia+10,
material+80 and static+90. It supplies inverseFrame=definition+40 iff +8c is
nonzero. Actual initializer `82ae8fc8` stores I at `82ae9020`, zero position
at `82ae9024`, J at `82ae9028` and K at `82ae902c`. Constants:

- I `8227e8f0`: `[3f800000,0,0,0]`;
- J `8227e900`: `[0,3f800000,0,0]`;
- K `8227e910`: `[0,0,3f800000,0]`;
- position: four +zero lanes, including W.

These stored Part matrices are NOT SkeletonData physics-pose placement.
`Part::AddToSimulation` (`82ae0aa0`) still applies the optional inverse mass
frame: stored Part matrix times the source rigid transpose-inverse. It passes
that result to AddRigidBody at `82ae0bd8`. Requested Active4 becomes Static1
only when the native static bit is set. The bridge reuses the existing
`part_attachment_transform` and `effective_part_body_state` kernels; identity
Part placement is not assumed to imply identity final rigid-body placement.

Raw range hashes (SHA256, end exclusive), retained from the source proof packet:

| Range | SHA256 |
| --- | --- |
| 82e4a4d8..82e4a53c | edd3106657a2d91c81c7405405c52c8b669e86529a8943f8a63c931fdfc6b873 |
| 82ae13f0..82ae14f0 | cbe1208f54e1f45aea3ead4925d784d6c46aac953096830aa52588677d302008 |
| 82ae9058..82ae90ec | e1fb5bebae72b2ea11ea5112b62983902814f69ea47cf52c26c6f94e9f30bb94 |
| 82e4a540..82e4a5b8 | d97ebcb5f2a3c322c6b1bfeafd348df7da36f56fc855f0ff1770d00feb0fc8c3 |
| 82ae1338..82ae13ec | 0aaa886f0912d51660720587470c3f9b12fd9c169d7d6f66391073c88e3ce3fc |
| 82ae0bf8..82ae0df0 | 007c0a98a60d5a7c3535621ca32e78e94a46087b292a1a4b77a6c477307cba51 |
| 82ae9820..82ae9874 | e713562c048f22ec1969fd3da02a584ed230ccd3873c7a0593eb12c11d6d8e33 |
| 82ae8fc8..82ae9058 | 32096d0f7dbc1bb379cd9f18c10767366ae824495516c9366690877d48371c6f |
| 82ae0aa0..82ae0bf8 | a004d5ccb70a1e15ebe5f0748cc17255c3d5db87458628b7e1ad19bf1835dad7 |

## Concrete Skeleton binding

`try_create_pre_normal_skeleton_assembly` retains the real body geometry owner,
24 mutable inertia cells initialized from CURRENT source-produced mass state
after debug adjustment, dense mutable Joint limits and actual immutable frames.
It retains both saved mass caches and full packed inertia W in the original
body definition; those are not substitutes for current pool inertia. Body and
Joint inputs must be produced from the same retained SkeletonData/attributes;
this is a caller invariant, not a provenance check invented by the bridge.

The actual normal culling resource is retained from the caller's existing
inventory, with exact Arc identity. Source creates normal at `82b620b4`, ragdoll
at `82b620bc`, then installs normal resource+8 into definition+28 at `82b620d0`;
Components copies it to Assembly at `82ae0de4`. Binding does not construct a
replacement inventory or infer a table. A wrong resource key is a host refusal.
This retains the initial table; live collision publication/current-mode
switching remains outside this partial owner.

The session starts detached with 24 source constructor matrices and zero
definition Drives. It refuses mismatched declared joint counts, allocated but
uninitialized joint tails, or a present inverse-frame flag without initialized
storage. These are explicit host preflights, not invented native recovery or
permission to shrink original counts. Native constructor partial publication
and allocator failure semantics are not claimed.

Remaining integration is substantial: actual Body/State relationships, mutable
Part material and Volume group/enable/userdata authority, full-four-lane mutable
inertia ownership, island membership/body-owner registration, Joint **spy**=1
(Joint+1c, not tag+18), root zero material and remaining SetUpNormal chronology,
TeleportReset, actual two-part Skeleton hook/drives and SkaterPool membership.
Outer Skeleton::Set placement, original player input/animation physics, world
pairs, camera and renderer remain on the freeroam critical path.

## Validation scope

New backend fixtures check immutable Arc identity, live mutable storage, mixed
sessions in one authority, rollback/removal binding retention, body Static and
capacity ordering, and Joint capacity/endpoint/owner preflight ordering.
Skeleton fixtures distinguish supplied pose from constructor placement and
current post-debug inertia from deliberately changed synthetic decoy caches
(native saves its animated tensor AFTER debug adjustment), then exercise actual
24-body/22-joint admission, lifetime/cleanup, both capacity rollback stages and
malformed-definition refusals.

The existing ignored original-body corpus now also admits all hats 0/4/1 using
actual constructed geometry/current mass and original 22 frame/limit records.
It checks actual pool basis/COM/derived inertia and all 20 frame/16 limit words,
ordered endpoints, retained culling/geometry ownership and explicit cleanup.
Expected transforms and tensors route through already independently tested
source kernels: this is an integration oracle, not new captured Xenon floats.
The original RAW pose fixture is still controlled, not live animation provenance.
The older normal-limit diagnostic remains separate and explicitly labeled.

Final strict checkpoint **68527 EXIT0**: formatting check; **2258 core +187
formats +38 app +3 symbols =2486 unit tests**, six doc tests; all-target Clippy
with `-D warnings`; **all seven original core corpus checks**; git diff check.
App ignored probe was not rerun. Board Rob's Park C/J/D remains6/6/3,6/6/3,
6/6/3,4/6/3 with all four prior centers unchanged. DMO eight-frame final center
remains `[588.8619,-54.40114,996.5048]` and contact hash `0e7c704e6a10f08a`.
Independent source/code/tests/corpus/document review passes.

Ten new unit tests: five shared-payload/backend tests and five actual Skeleton
binding tests. Initial gate74887 caught the new fixture's incorrect assumption
that animated cache tensor preceded debug adjustment; accepted production code
already preserves the real later cache store. Corrected only the test to use
explicit synthetic decoy caches. Reviewer independently rechecked debug calls
at6201c/62060, current tensor load62068, cache store62074, destructor62078.
Focused85591 passed all five Skeleton tests then reported doc/API lint issues;
those were corrected before final full gate. No native arithmetic was altered.

## Next connected authority step

Read-only next-boundary audit recommends a skeleton-specific child module of
`dynamic_assembly_session`, not another backend. It can reach the existing
private Part/joint handles and attached authority, then use narrow backend
operations to call the accepted normal-limit setter and inertia stores. Do not
export handles/pools merely to invoke these routines.

Three ownership changes must feed their actual consumers:

- Part material selection needs one runtime Definition/Zero/Normal selector
  retaining actual values. Changing a Part pointer is not overwriting its
  definition material. M can remain AssemblyPartMaterial; the backend does not
  consume it during body Add. Live publication must resolve this selection.
- Each top-level Volume needs one authoritative mutable group/enable header;
  hat edits affect the outer Aggregate, not its children. A retained constructor
  Volume snapshot must be explicitly initial evidence if a separate live header
  is chosen. No duplicate writable header states.
- Extend the SAME private mutable inertia cell with explicit optional packed W
  from BodyMassState. Packed writes update XYZ/W under one lock; scalar/drag
  writes preserve W. Do not invent W for existing generic immutable owners or
  refresh derived body tensors/RF when source merely edits referenced inertia.

Existing `skateboard_collision_publication::read_live_parts` samples immutable
template material, board-specific userdata and immutable geometry. It must not
be repurposed by changing seven to24. A skeleton producer must read its actual
live material/header ownership and primitive/hat topology, then reuse accepted
Create/Update and ground consumers. Serialized Update already rewrites enabled,
material, transform, group and frozen while retaining Create-only inline data.

Normal setup order remains: actual limits; normal culling; assembly group6/all
Part userdata6; Collision.Reset; all angular drags then all linear drags;
literal Parts1..23 normal material, animated inverse mass/full4tensor/spherical,
raw compliance then priority; root compliance1; fresh local Skeleton byte
043f9227035de245 (missing0); NormalCollision or NoCollision(true); ragdollfalse.
Root zero material was selected before this call at82b621e4. Distinguish
Assembly group, Part userdata/serialized-record group and Volume group_id.

Initial disable counters1..23 remain unproven before first normal setup.
NormalCollision conditionally enables/group0 only when the signed counter is
zero; it does not reset that counter. NoCollision(true) resets it. Later
TeleportReset proves all24 zero. Do not silently initialize unknown bytes or
invent missing Body/State references to make this step appear complete.
