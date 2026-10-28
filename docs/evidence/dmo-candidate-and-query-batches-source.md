# DMO candidate metadata and retained query batches

Read-only source/Rust audit, 2026-09-08. No Rust edits or Cargo. This packet
extends [generic admission](dmo-dynamic-pool-admission-source.md) and the
[shared backend bridge](dmo-shared-backend-bridge-source.md). Confidence is
**exact** for the cited raw stores, branches and concrete vtable entries;
**high** for the proposed Rust ownership mapping. No complete living-world
spawn, online `SpawnStatic`, job scheduler or box/trajectory/step geometry
implementation is claimed.

Implementation follow-up: `collision_query_batches.rs` retains the four
constructor-configured helpers and distinct leases/states/counts. Its line End
uses an exclusive owner-bound callback returning the actual local-job result.
It owns no geometry or scene copy. Host execution failures are explicit:
Running and successful earlier results survive, while SyncJobs/Sync/Release
refuse the known incomplete job and do not retry or manufacture a miss.
Non-line request acceptance is intentionally absent pending typed callers;
constructor-empty slots have their real lifecycle. This is the synchronous
local-job path, not an asynchronous scheduler implementation.

## Concrete candidate routes

There are two different native producers; they must not be conflated.

1. `Sk8::Dmo::Body::Body` at `825ea858..825ea954` retains an actual Definition,
   model, flags and DMO ID, creates a fresh Assembly from that Definition,
   and applies the SAME supplied full locator matrix to every Part in
   ascending order. It does not set Assembly group or Part user data:
   both retain their proven initializer value zero. `SimManager::SpawnStatic`
   calls `IIsland::AddDMOBody` directly, bypassing the group switch described
   below. Thus zero is a source-backed constructor value for this particular
   producer, not an arbitrary generic DMO default.
2. `LivingWorld::cLWDynamicObjectCollision::InitCollisionBodies`
   `82d3aa28..82d3ac3c` creates a one-Part collision body from the actual model's
   high-LOD world Definition. It selects group 16 when the retained
   characteristics layout scalar `+6c` is ordered less than 5.5, otherwise
   group 14. Equality, greater and unordered/NaN all select 14: raw `fcmpu`
   at `ab1c`, `blt` at `ab20`, assignment 14 at `ab24`. Both groups enter the
   passed Island's DMO pool through `AddToWorld`.

The second route is appropriate for the eventual normal offline living-world
spawn. The first route is a smaller, independently usable source-bound Body
constructor beneath the already retained locator/model association. Its
current corpus use must explicitly exclude native `SpawnStatic`'s online
enable cvar, network collision query, native hash traversal, paired simulation
and visual islands, ID/list ordering and spawn policy. It does not establish
that online spawning runs in offline freeroam.

### Dispatch proof: group is not an Island or pool ordinal

`cLivingWorldCollisionBody::AddToWorld`, `82e50cb0..82e50e60`, reads group,
subtracts 10 and checks unsigned result <=6. The seven raw words at `82e50d78`
are `82e50db4 82e50dd4 82e50dd4 82e50dd4 82e50d94 82e50d94 82e50d94`.
The concrete Island vtable at `820ffa10` resolves their indirect calls:

| Group | Call slot | Actual function |
|---|---|---|
| 10 | +14 | `Island::AddVehicleBody`, `827b6840` |
| 11, 12, 13 | +1c | `Island::AddPedestrianBody`, `827b68e8` |
| 14, 15, 16 | +0c | `Island::AddDMOBody`, `827b6798` |

The same table has `AddSkaterBody` at +4, `827b66d0`. Groups 14/16 do NOT
insert into SkaterPool. Earlier working messages incorrectly labeled those
slots; this concrete table corrects that interpretation. The caller supplies
the actual Island; `AddDMOBody` then uses that Island's `mDmoPool+6cc8` as
already proved in the admission packet.

## Fresh Assembly and Part fields

`PhysicsUtility::CreateAssembly`, `82e4a540..82e4a5b8`, obtains the descriptor
from Definition part/joint/drive counts, calls the actual allocator and then
`Assembly::Initialize`. It does not load a serialized Assembly header as the
live header and does not clear the returned allocation itself.

`Assembly::Initialize`, `82ae1338..82ae13ec`, stores in this order:

- group `+18=0` at `1370`; Part count `+1c` at `1378`, drive count `+24`
  at `1380`, joint count `+20` at `1388` from the actual Definition;
- flags `+2c=0` at `1390`, user flags `+2e=0` at `1398`;
- inline component pointers and Definition pointer `+4` at `13a8`;
- world index `+14=0x0000ffff` at `13bc`, not all-ones u32;
- component initialization at `13d0`.

`InitializeComponentsFromDefinition`, `82ae0bf8..82ae0df0`, initializes
Parts ascending (runtime stride `60`, Definition stride `a0`), then joints,
then drives. Its final `0ddc..0de4` loads Definition `+28` and stores Assembly
culling pointer `+0`. Neither initializer writes Assembly user data `+28`.
That word remains unknown for a fresh allocation; current CPU Create/Update
does not consume it, so the bounded candidate need not invent it.

`Part::Initialize(resource, Definition)`, `82ae9820..82ae9874`, passes actual
Definition volume `+0`, Inertia address `+10`, Material address `+80` and
static-body word `+90` to `82ae8fc8..82ae9058`. That callee stores:

| Part field | Source/store |
|---|---|
| Volume pointer +40 | actual Definition Volume, `8fe4`; no volume flags changed |
| Inertia pointer +48 | actual Definition Inertia, `8fec` |
| Material pointer +50 | actual Definition Material, `8ff4` |
| inverse-body pointer +44 | zero at `9008`, then Definition+40 iff Definition+8c nonzero at `985c` |
| attached body +4c | zero at `900c` |
| world index +54 | zero at `901c` |
| cached full matrix | X=`gI` at `9020`, P=all positive zero at `9024`, Y=`gJ` at `9028`, Z=`gK` at `902c` |
| internal flags +58 | 1 iff static-body argument nonzero; otherwise 0 |
| user flags +5a / user data +5c | zero; static arm stores these before internal flag, nonstatic arm after it |

In particular cache P.W is zero, not canonical affine 1. It is a full4 cache
store; subsequent actual placement preserves the supplied matrix W lanes.
Volume flags remain owned by the retained volume graph, not copied from Part
internal flags or inferred from body state.

`Physics::Body::Body`, `825ea770..825ea808`, initializes Assembly, Definition,
owner, Island pointers and Body ID to zero; remain-on-reset false; owns-volume
and owns-Definition true. `Dmo::Body` then retains flags/model/DMO ID before
its null-Definition assertion, retains Definition before the allocator
singleton read/assert, stores the new Assembly at `a908`, clears both owns
flags at `a90c/a910`, applies each Part position at `a92c`, and sets
remain-on-reset true last at `a948`. These are distinct from Part world index
and the Assembly index assigned later by Insert.

## Living-world metadata mutation and placement

The live dynamic-object initializer stores entity and actual characteristic
instance at `aa84/aa90`, calls model slot +48, asserts Definition Part count
equals one, then allocates the body. After group selection, the actual model
chain `model+74 -> +84 -> +4` is compared with -1. For -1, basic virtual Create
slot +1c executes. Otherwise characteristics `+74`, `+78`, `+5c` are loaded
in that order BEFORE `FillWorldCollisionMasses` at `ab78`, which appends actual
characteristic `+6c` (one mass). Rich Create slot +18 follows. The model body
association is committed after Create; owns-Definition false is last `ac24`.

Basic `cLivingWorldCollisionBody::Create`, `82e50b70..82e50c10`, retains the
Definition before allocator access, constructs Assembly, stores Assembly group
at `0bdc`, then stores that SAME group into each Part user data at `0bf0`.
Rich Create `82e5cff0..82e5d0c0` first calls SetMaterialProperties, then
SetMasses, then allocation/Initialize, group and per-Part user-data writes.
`SetMaterialProperties`, `82e4c818..82e4c8e4`, mutates the actual shared
Definition Part material fields `+80/+84/+88` ascending. `SetMasses`' complete
float graph is not recovered here; do not implement rich Create using copied
immutable template inertia or constant masses. Later `SetCollisionGroup`,
`82e4c9d8..82e4ca14`, writes Assembly group before each Part user-data word.
Known callers include DynamicObject SetAsHandProp/UnsetAsHandProp.

`FillCollisionPartTargetTransforms`, `82e5f8d8..82e5fa3c`, copies actual core
transform `+50`, optionally obtains the collision shift (non-pipe path), adds
the full shift to P, then FMA-adds core velocity `+90` times `kUpdateTime` to
P and appends one matrix. Even the concrete DMO zero shift executes the add.
This is separate from the direct Dmo::Body constructor's supplied locator
matrix; do not apply the predictive shift to that constructor.

`AddToWorld` returns immediately if Body already has an Island. Otherwise it
checks supplied matrix count >= actual Part count BEFORE placement, applies
all positions ascending, and ONLY THEN dispatches group (invalid group fails
after position writes). After insertion it registers each attached Part as
an actual Island spy target. `Body::SetPosition`, `82de8090..82de8124`, on an
already attached Body activates state2 and clears its body's cooldown before
Part.SetTransform. Detached construction skips that activation branch and
uses the actual Part cache. No alternate pose snapshot is required.

## Existing Rust inputs and minimum API

| Existing owner | Reusable source / precise gap |
|---|---|
| [SerialisedAssemblyTemplate](../../crates/skate2-core/src/physics/assembly/serialised_assembly_template.rs) | Retains exact arena, Assembly/Definition IDs, parsed Definition and ordered volume/inertia/material/inverse/static fields. Serialized Assembly group/world index are not fresh Initialize values. |
| `resolve_culling_table` in that file | Currently resolves parsed **Assembly** culling. Fresh Initialize instead copies **Definition** culling. Resolve the latter from the same arena, or explicitly validate equality before reusing the former; no assumed equality. Both fields are already parsed. |
| [SimulationDmoResolvedLocatorPartOwner](../../crates/skate2-core/src/physics/assembly/simulation_dmo_assembly_part_session.rs) | Retains exact locator occurrence and selected resident-model Part owner. Current `try_begin_session(Vec<Matrix>)` accepts arbitrary supplied matrices; add a source-specific constructor operation reading the retained locator and applying it ascending to all Parts. Preserve the existing explicit component seam as such. |
| [DynamicAssemblyPartRuntime](../../crates/skate2-core/src/physics/assembly/runtime/dynamic_assembly_session.rs) | Existing cache/body owner is the authority; extend it or its actual owner with narrow retained constructor metadata. Do not create a second mutable transform vector. |
| [SerialisedAssemblyCreatePartInput / LiveAssemblyInput](../../crates/skate2-core/src/physics/assembly/serialised_assembly_create.rs) | Need live body observation, actual Part transform, actual material, Part user data, Assembly group and culling. They do not need Assembly user data. Preserve same-owner lazy sampling, existing prepared payload and shared backend address image. |

Recommended source-specific `DmoBody` candidate construction has no caller
group/user-data scalar: fresh Initialize proves zero, followed by exact
retained locator placement. A separate living-world operation would take the
actual characteristic provider/model branch and perform the recovered
group/material/mass operations. It must not silently turn a serialized header
group into live state or use a free-standing group setter as a spawn producer.
Root should keep generic pool membership independent of these two producers.

For the first original fixture, use the already sealed locator/model/arena
association plus actual same-manager body session, verify constructor metadata,
then Insert/Create/BeginUpdate and selected query consumption. This proves a
controlled source-body constructor and pool pipeline, not `SpawnStatic` or
offline living-world scheduling. No original-corpus run was performed in this
read-only task. Live characteristic values and rich Create remain a later
producer requirement for the offline route.

## Batch lifetime needed before collection rebuild

`DynamicPool::BeginUpdate` first synchronizes four actual retained helpers:
line at `9404`, box `940c`, trajectory `9414`, step `941c`; mesh count is
cleared only at `9428`. Publication traversal then follows the accepted
owner packet. A failure in a later helper must preserve the earlier sync
effects and leave the old collection intact.

`BatchHelper<Line>::Sync`, `827c8110..827c8168`, reads its allocated batch
count `+4` and visits EVERY slot in increasing index, stride `3d0`; box,
trajectory and step use folded helper `827c8168..827c81c0`, stride `3c0`.
These call virtual slot **+24 = SyncJobs**, not public Sync at +0c. They do
not clear the batch state, lease, tests or count.

Concrete vtables `821044d0` (box), `82104500` (line), `82104568` (trajectory),
`8210459c` (step) all contain SyncJobs `827bb5e8` at +24 and an actually empty
folded function `825187c0` at +2c. Their lifecycle is:

- Base constructor `827bf238..827bf468` stores capacity `+398`, count
  `+39c=0`, state `+3a0=0` (Idle), lease `+3a4=0`, actual allocator/scheduler
  and job/test storage. This is a real initialized empty state.
- Acquire `827b2e48..827b2f84` asserts label nonnull, scans allocated slots
  ascending, atomically acquires first lease0->1, then stores label when the
  helper has its label array. Exhaustion asserts; it is not an empty result.
- Begin `827bc8f0..827bc958` asserts Idle, stores state1 (AddTests), then
  clears count. It is not the same operation as Acquire.
- AddTest uses the retained test/result slot and source capacity behavior.
  Line AddTest `827bd9a0..827bda68` returns false on count==capacity before
  calling Next, then resets the actual result and copies full start/end plus
  fatness. A batch owns real results; no fabricated miss on unavailable work.
- End stores state2 (Running) BEFORE preparing job data. Prepare binds the
  actual collection's spatial pointer, mesh-array pointer, mesh count and
  retained tests. Raw store order is spatial, meshes, count, tests. For line
  source collection is +3c0; for other three it is +3b0.
- Non-line End `827bc588..827bc6dc` supports actual local execution, ascending
  tests, or scheduler jobs. Line atomic End `827bee78..827bf084` additionally
  captures old ratio, resets ratio to1, clears atomic cursor, then runs jobs
  or the actual local line callback. A synchronous host can execute the
  recovered local callbacks without fabricating asynchronous scheduling.
- SyncJobs `827bb5e8..827bb6b0` returns if test count is zero. Otherwise it
  checks whether the job handle is valid; if valid, joins scheduler work until
  IsDone, then calls actual +2c postprocessing (empty for these four concrete
  classes). Without a valid handle it still reaches +2c. It never clears
  count/state/lease and does not enforce state Idle/Running.
- Public Sync `827bc6e0..827bc764` asserts state2 OR state0, calls SyncJobs,
  then sets Idle. Release `827bf088..827bf15c` asserts lease1, calls SyncJobs,
  sets Idle and atomically releases lease1->0. Release does not clear count.
- Line GetResult `827bda68..827bda9c` gets its actual test result and returns
  null only when its real result count is zero. GetTest/state validation is
  a separate helper and must not be elided by a generic result vector getter.

Additional raw recovery for implementation: `Next` at `827bc3e0..827bc480`
asserts state1, then count<capacity, then increments count before returning the
slot. Line AddTest's own equality-capacity return precedes that state check.
`GetTest` at `827bc480` asserts state0 before checking index<count, and never
calls SyncJobs. These checks explain why helper synchronization alone does
not permit GetResult. Release of an AddTests batch is not forbidden by a
source state check; it synchronizes and returns the slot to Idle without
clearing count. Reacquisition retains that count until Begin, so initialized
but unexecuted AddTest results can still be read through the native sequence.

Minimum useful synchronous design therefore retains four families of actual
allocated slots, lease/label/state/count/capacity, request/result buffers and
the collection owner. End performs the actual implemented geometry against
the bound collection; SyncJobs observes completion and preserves state;
Release performs its distinct state changes. Completed local work legitimately
makes later SyncJobs cheap, but four unconditional empty callbacks with no
owner/state cannot prove the boundary. Initially unused box/trajectory/step
slots may be genuinely constructor-empty. Once a request is accepted their
unsupported geometry must refuse at execution rather than report completion
or miss. Do not reject a leased AddTests slot merely because BeginUpdate's
SyncJobs is called: source has no such state preflight.

## Raw span fingerprints

SHA-256 over the actual IDA bytes, end exclusive:

| Span | SHA-256 |
|---|---|
| 82ae1338..82ae13ec | 0aaa886f0912d51660720587470c3f9b12fd9c169d7d6f66391073c88e3ce3fc |
| 82ae0bf8..82ae0df0 | 007c0a98a60d5a7c3535621ca32e78e94a46087b292a1a4b77a6c477307cba51 |
| 82ae8fc8..82ae9058 | 32096d0f7dbc1bb379cd9f18c10767366ae824495516c9366690877d48371c6f |
| 82ae9820..82ae9874 | e713562c048f22ec1969fd3da02a584ed230ccd3873c7a0593eb12c11d6d8e33 |
| 82e4a540..82e4a5b8 | d97ebcb5f2a3c322c6b1bfeafd348df7da36f56fc855f0ff1770d00feb0fc8c3 |
| 825ea858..825ea954 | 03c18f7c4597940d0d555ebd7fe47fc5b3f236c561a89b097f98622d65e714f6 |
| 82d3aa28..82d3ac3c | 44ddc26cfb9cbd445d5ef481666a332424849d2f668eb844e698b313bac6b920 |
| 82e50cb0..82e50e60 | 78799115b521ffea9d5494d8a37a03f2cb3e52f032c4de5bc79eb5e9768ce440 |
| 827c8110..827c8168 | c880f83c8e593c4ff45c97fbc24caa418ee6c974a1a5b52cfbed06dd40b27a81 |
| 827c8168..827c81c0 | b609ff4e3dda5031d01a9ad7e156f0538895f5687c4b031a896b3b67d53d399d |
| 827bb5e8..827bb6b0 | f707eec302d0d7bca8f37e2a794d6fa33feb2aa77f92862b17ac4692e4130049 |
| 827bc8f0..827bc958 | 35a84bdc3e16e31184511bafaea1c1223465562a173b6c7e315039fedbad43c5 |
| 827bc6e0..827bc764 | d8ac54f6e2a3c648bc99b8c464381818297619aeb036b5b74909f1ec63c46d4c |
| 827bf088..827bf15c | b439e9d159191f0e032bf0f6fd0a5a5f916eb2cbbb8f66f3043b8296e0574e51 |
| 827bee78..827bf084 | 854cb199230cee1b1694bbaf6a7df32d8c672b7d6a71055c10a3c06c5c5c63e9 |

Remaining implementation gates are the actual same-backend source owner and
Definition-culling binding, source-specific candidate metadata/placement,
generic pool admission/publication and actual retained batch lifecycle. The
full offline living-world route additionally needs the actual characteristic
instance/model selection and rich mass mutation. These gates do not justify
inventing empty dynamic collections for a requested World query.
