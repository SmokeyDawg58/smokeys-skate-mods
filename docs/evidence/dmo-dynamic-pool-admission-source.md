# Generic DMO DynamicPool admission and publication

2026-09-08, bounded read-only source and implementation packet.
No Rust edits or Cargo. Inherited Insert/Remove bytes and actual DMO/vehicle
outer admission/DMO cleanup were read directly; Begin/End and Create/Update
reuse independently accepted source packets and their existing implementations.
This packet coordinates with the separate same-manager source-arena backend
bridge. It does not introduce a competing pool or complete living-world spawn.

## Source identities and actual dispatch

| Function | Half-open range | SHA256 |
|---|---|---|
| AssemblyPool::Insert | 82e0dbb0..82e0dc98 | 5597112ff6c0264eaf69ef3c289855e188c8165662509a82b659141f7e87ee61 |
| AssemblyPool::Remove | 82e0c1c8..82e0c298 | 0bbcfa07c1097bae6be9df0f1d37126908fde882525f1d7b787ad73f50695e1c |
| AssemblyPool::BeginUpdate | 82e0dc98..82e0df14 | e3a3fc3a03c111c0bb47bea3ef6b36444f9bd9df8377c245d7a94de9f6754274 |
| DynamicPool::Remove | 827c92b0..827c93f0 | c5c945fbe3afd7180257711b3427dc8698dbf373410fca8adbf7e56e570c8ea1 |
| Island::AddDMOBody | 827b6798..827b6840 | 16d67bdd097423f45921881d4227d007a7d4cbd06954fafec97f5fd43a8bae50 |
| Island::AddVehicleBody | 827b6840..827b68e8 | 4ab7b65c26a35e74ea7f11e66c7f3c96dbc92e57676d6711ee87e3e24102816a |
| Island::CleanUpDMOBody | 827b6b90..827b6c88 | ef34a3a6695fee17d376f4fd6d562b743b394a067298260acd7a4e1a4ad23722 |
| Island::RemoveDMOBody | 827b7530..827b75b8 | c5d806c2ab9104492157e540ea69139931cabfe10082af86db99209421fdfdc9 |

DynamicPool vtable at 8210aa94 proves the inheritance: +4 points to overridden
BeginUpdate827c93f0, +8 to inherited EndUpdate82e0bb10, +c to inherited
Insert82e0dbb0, +10 to overridden Remove827c92b0. Insert has no mesh-publication
override. Mesh records are created by the subsequent DynamicPool BeginUpdate.

Existing reviewed packets:
[generic source ordering](skater-pool-owner-source.md),
[serialized refresh](assembly-pool-refresh.md),
[dynamic mesh publication](skeleton-ik-world-line-owners-source.md), and
[current runtime gaps](world-dynamic-line-runtime-integration-plan.md).

## Insert: committed count is the last step

The inherited pool fields used here are manager+30, count+10, member array+1c,
separate bounds+20 and optional serialized array+24. The native sequence is:

1. Read manager's next object ID+50 at dbd0, wrapping increment/store at
   dbd4/d8, store OLD ID to actual Assembly.worldIndex+14 at dbdc.
2. Fetch actual manager Simulation and call Assembly::AddToSimulation at
   dbe8 with the requested state. The native caller ignores the return.
3. Publish the Assembly pointer to members[count] at dbf8. Count has not
   advanced yet.
4. If serialized storage exists, call SerialisedAssembly::Create at dc1c
   on selected slot `count*0x1780`, using the passed one-word Params. Then
   copy Header bounds into separate pool bounds at dc48/dc50. The null
   serialized branch writes both full bounds vectors to positive zero.
5. Increment committed count at dc5c (serialized) or dc8c (nonserialized).

There is no native capacity/duplicate/Add-failure/null recovery branch.
Therefore host refusal cannot be described as native rollback. Reuse the
existing SkaterPool ownership policy explicitly: authority, detached-state,
capacity and host-storage checks can precede effects; once issued, a world ID
is never refunded. Add failure returns the candidate with its actual attachment
transaction result. Create failure returns an attached candidate and issued
ID, rather than dropping live bodies. Host staging until Create succeeds
differs from native early raw pointer publication; retain that documented
boundary instead of publishing a fake complete member.

The model ObjectId, manager Assembly world index, dense pool ordinal and later
Island BodyID are four distinct identities. Use an occurrence capability for
membership so swap-last does not change identity. Do not reuse the skater
MemberContent enum, catalog model IDs or line-result owner IDs as substitutes.

## Actual outer caller and its additional responsibilities

AddDMOBody enters Island's critical section+6d10 at 67bc and reads
Body.assembly+18 at 67c0. With a nonnull Assembly it loads ACTUAL DMO pool+6cc8,
constructs fresh SerialisedAssembly::Params at 67d8, calls pool virtual Insert
at 67f8, then stores Body.island+24 at 67fc. Params constructor82e0b680 supplies
the source threshold bits3f7fbe77; this caller does not select the skater's -1
alternative or read a bend cvar.

After the optional insertion, exact requested state==1 selects BodyID0 and
Island static-body list+2c. Every other raw state reads Island.nextBodyID+4c,
increments it, stores the OLD ID to Body+28 and appends to dynamic list+3c.
The list push occurs at 682c, followed by critical-section release. Even the
null-Assembly arm reaches this ID/list suffix. A bounded Assembly-backed DMO
host path can require the actual Assembly, but must not claim that requirement
is native null handling. Exclusive Rust access may represent the lock, while
checked list allocation failure must retain preceding insertion/ID effects.

AddVehicleBody has the same graph, using vehicle pool+6ccc and call68a0.
No vehicle loader/session owner is supplied by this equivalence. Constructor
capacity/parallel/batch inputs remain as recorded in the runtime integration
plan: DMO50; vehicle24 when island flags mask1 is set, otherwise0. Distinct
actual pools are retained even when one has constructor-proven zero capacity.

The existing [physical Island registry](../../crates/skate2-core/src/physics/player/interaction/physical_player_island.rs)
already illustrates separate BodyIDs, lists and cleanup over actual skater
membership. Generalize the native responsibility as needed; do not manufacture
a PhysicalPlayerAssemblyOwner or skater kind for a DMO.

## Minimum retained generic member

Proposed names below describe responsibilities, not mandated shared APIs.

| Owner | Retained responsibilities |
|---|---|
| GenericDmoCandidate | Exact resident source/template association; actual detached/body-only session; current live Assembly/Part metadata; assigned-world-index and requested-state progress. Failure returns this same object, possibly attached. |
| GenericSerializedMember | Private occurrence handle + candidate/session + sole PreparedSerialisedAssembly + separate bounds. Member and publication cannot be supplied independently. |
| SimulationDynamicAssemblyPool | Actual manager and body-pool authorities; native capacity/configuration; dense member array; active list/stamp; its own retained DynamicPool mesh records/batches. DMO and vehicle are separate instances. |
| Island DMO Body owner | Actual Body-to-Assembly/member association, Island identity and BodyID/list progress. This is the outer caller; generic pool insertion alone does not complete it. |

The existing [DMO source owner](../../crates/skate2-core/src/physics/assembly/simulation_dmo_assembly_part_session.rs#L175)
already retains asset, template ordinal, Assembly ObjectId, attachment batch
and backend authority. Keep that identity chain. Its current session/backend
is staged and body-only; only tests attach it. Initially admit only actually
supported body-only original definitions, preserving explicit refusals for
nonzero definition Joints/Drives instead of silently omitting them.

The coordinated backend direction is one shared SimulationAssemblyBackend
with a private source-routed inertia variant and retained arena route/outcome
directory, using the SAME existing body pool and image domain. The directory
must resolve occurrence-to-route for source-aware removal and Static outcomes;
current state or semantic inertia presence cannot reconstruct that identity.
This is the separate bridge author's responsibility, not another pool here.

The generic member must borrow a native Part getter from that bridge: actual
body observation → existing packed_body_transform/image/outcome provider →
optional inverse-body composition. The canonical-W host getter used by the
current board helper does not recover the native packed W consumed here.
Keep private handle ownership and expose only owner-qualified operations.

Create inputs are also borrowed from the SAME retained session, rather than
recreated cached poses: definition-ordered body observations, live Part cache,
material/user data, current Assembly group/culling and source Params. The
source-aware provider reads the live body transform itself. Current template
fields supply actual immutable source references; any overwritten live
group/user-data/volume flags need their actual constructor/setter producer.
An arbitrary caller scalar or convenient zero is not a recovered runtime value.

## Serialized and mesh publications have different consumers

[try_prepare_serialised_assembly_create_from_xenon](../../crates/skate2-core/src/physics/assembly/serialised_assembly_create.rs#L711)
already accepts template, actual body pool, source inertia owner, image domain,
Part inputs, live Assembly inputs, Params and numeric service. It returns
PreparedSerialisedAssembly retaining original owner, culling source, body
occurrences, VolumeData, Header and frame stamps. Its staged host publication
does not reproduce native invalid-input partial writes; do not overstate it.

The generic admission bridge should make this existing entry reachable and
retain its result. Subsequent Update must reuse its body occurrence array and
owner, not recreate Create per frame. Existing Update preserves counts,
identity, Params and body table while updating the represented VolumeData and
late Header fields. Native body reads and current field sampling must remain
owner-qualified; no proxy snapshot or copied external part list becomes the
authority.

DynamicPool BeginUpdate first completes four batch syncs, clears mesh count,
and visits the FULL dense member/Part array for AddVolume. It reads each live
Part volume before its native GetTransform. The mesh graph is the actual root
and SMA structure; it is neither flattened Create VolumeData nor the
construction-time enable-filtered DynamicClusteredMeshTemplate leaf list.
Retain exact Part transform/owner through recursive AddVolume and execute its
disabled-only cvar reads and count-before-Reset stores.

Only complete mesh traversal reaches inherited BeginUpdate. Filling the
final mesh slot returns false and stops this tail with that mesh retained;
this is not a decode failure or rollback. Represent progress such as
`base_begin_ran=false` separately from the valid admitted mesh prefix. Do not
automatically invent a successful serialized refresh when that tail was skipped.

The inherited nonparallel path updates ALL serialized members, then computes
ALL separate bounds. EndUpdate waits, copies ALL bounds into Headers and only
then rebuilds the active list from actual Part frozen bit2. Full member order
is retained; zero-Part/all-frozen members are not active, all-static alone is
not exclusion. The existing
[try_update_serialised_assembly_pool](../../crates/skate2-core/src/physics/assembly/serialised_assembly_pool_update.rs#L87)
preserves this order with explicit host failure/freshness rules. Split its
phases if required by the real Begin/End schedule, not by running a complete
single-member refresh repeatedly.

Ground/assembly collision consumes serialized Headers/VolumeData and actual
body tables over the FULL pool count, with downstream frozen/static gates.
World line queries instead consume DynamicPool's mesh collection in published
order, and keep duplicate occurrences/ties. Actual IK default routing selects
the Ground and DMO owners, then vehicle for Both. These two publications share
membership/body authority but cannot replace each other.

## Removal prefix and lifetime

Inherited Remove calls Assembly::RemoveFromSimulation at c1e8 BEFORE searching
members. It then copies the last member pointer into the hole, assigns the
last serialized record at c258, copies both bounds vectors and decrements
count at c28c. No ID refund/index clearing or immediate active-list rebuild
occurs. Safe-host handle validation before detach is the existing explicit
divergence for unsafe missing-member removal.

DynamicPool::Remove synchronizes four batches BEFORE that base Remove, then
removes every mesh record with the removed Assembly owner via repeated
swap-last. Keep the removed source owner alive until this suffix completes.
Moved records retain their identities; line candidates must not resolve a
stale/reused dense ordinal as a new owner.

The larger Island RemoveDMOBody first asserts Body.island matches at
7550..7570, then CleanUpDMOBody. Cleanup, when Assembly is present, visits all
Part SpyTargets at +4c in ascending 0x60-byte Part order, removes the Body
contact-map entry if present, then calls actual DynamicPool.Remove at6c74 and
clears Body.island at6c7c. RemoveDMOBody subsequently removes the Body from
both static and dynamic lists and clears island again. These effects precede
or follow generic removal exactly as stated; do not reduce whole DMO cleanup
to `members.swap_remove` or silently skip required spy/contact ownership.

## Useful next acceptance story

Reuse the original DMO asset/arena/inertia and repeated-ground corpus in
[simulation_ground_contact_corpus](../../crates/skate2-core/src/physics/collision/simulation_ground_contact_corpus.rs#L265).
It already proves actual source Create/image/inertia and repeated refresh,
but is a controlled corpus, not generic pool admission. Move that same body
path through real candidate→shared ID→attachment→Create→member ownership,
then connect DynamicPool mesh publication on an original mesh-bearing model.
Keep native field producers and controlled placement scope explicit.

Two real admitted occurrences should verify issued IDs, dense order, moved
member identity, source mesh ownership, changed actual Part transforms on a
second BeginUpdate, line-hit placement, and removal tie order. Synthetic
tests isolate pre-ID failures, post-ID attachment failures, Create refusal with
attached candidate retained, no native Add-return recovery claim, source mesh
capacity stop before base Begin, all-member phase order and cleanup prefixes.

Required dependencies remain concrete: same-manager source backend/Part
access, live candidate metadata/placement, generic admission/publication,
selected Island Body membership/cleanup, and real line-batch lifecycle.
Vehicle admission and full living-world spawn scheduling remain separate
uncompleted owners. No absent route may be advertised as an empty pool.

Confidence: exact for the newly read dispatch/call/store facts; high for
reused reviewed source order and existing implementation mapping. Proposed
owner factoring is a design. No new original corpus execution, Rust changes,
Cargo result or deterministic native frame trace is claimed.
