# Actual Island Body membership and contact-spy registration

2026-09-07. Next-source packet. High confidence; complete registration is NOT
implemented at checkpoint2554. The rigid-body state-store prefixes and solved
Spy export now exist, not the actual Body/Entity target map or contact list.
Assembly/SkaterPool membership alone does not implement Island
Body lists, Body IDs or contact feedback. Independently audit and recover the
bounded contact-list/callback consumer before claiming that connected path.

## Exact missing constructor sequence

SkeletonBody::Init after CreateAssembly82b620d8:

1. Assembly group1 and ascending Part userdata1.
2. Entity::GetPhysicsIsland82b62120.
3. IIsland virtual+4 AddSkaterBody(body,Active4,false)82b6213c.
4. Reload Body.mIsland; virtual+2c SyncCheckBody(body)82b62154.
5. Parts0..23 ascending, stride60: actual Part.m_body+4c, virtual+34
   RegisterSpyTarget(rigid_body,body,part_index)82b62188.
6. Actual Joint spy+1c=1 ascending82b621c4.
7. Three-zero material, select for root82b621e4.
8. SetUpNormal82b621ec.

Existing pre-normal properties implement6/7, not3..5. Existing actual Assembly
pool insertion is reusable inside3 but is not the entire method.

## Island::AddSkaterBody827b66d0..827b6794

Nonnull Body.Assembly: construct serializer parameters; bendingtrue selects
source0.99900001, false selects-1.0. Actual SkaterPool Insert827b6748, THEN
Body.mIsland=this827b674c. Requested state EXACTLY1 selects Body.ID0 and appends
static list. Otherwise read old Island.nextID, wrapping-u32 increment/store,
assign old ID to Body, then append dynamic list. Classification is requested
state, not individual effective Part states. ID/list work also happens without
an Assembly; current actual assembly owners need only nonnull branch—do not
invent null-Assembly support.

| Owner | Offset | Field |
|---|---|---|
| Body | 18 | Assembly pointer |
| Body | 20 | Entity owner |
| Body | 24 | Island pointer |
| Body | 28 | u32 Body ID |
| Island | 2c | Static Body list |
| Island | 3c | Dynamic Body list |
| Island | 4c | u32 next Body ID |
| Island | 50 | RigidBody -> RigidBodyInfo map |
| Island | 6c | Body -> ContactInfo-vector map |
| Island | 6cc4 | SkaterPool |

Island constructor827bab18 initializes empty lists and nextID1 at827babb0.
This counter is NOT the manager Assembly world-index counter or rigid-body
slot/RF identity. In pristine membership, board then skeleton receive BodyIDs1/2,
not0/1. List push_back827b6298 allocates12-byte node from category41, links at
tail, increments size. No fixed16-member list limit:16/2 is SkaterPool's limit.
Typed host allocation refusal must preserve/disclose preceding issued IDs and
membership effects; native allocation fault is not transactional success/failure.

## SyncCheckBody and actual registration

SyncCheckBody827b2da8..827b2de0 only enters/leaves Island.mLock at2dc0/2dc8
in this build. It neither logs nor produces a registry. Do not add an artificial
sync collaborator or hidden state machine for this lock-only operation.

RegisterSpyTarget827b7830..827b7894, under Island lock:

1. Read actual RigidBody.state+8c, store state|8 at7874.
2. Obtain/create rigid-body target map entry7878.
3. Store BOTH Body reference and original unsigned Part tag7880.
4. Release lock.

RigidBodyInfo is8bytes, Body reference+0/tagu32+4. Existing registration replaces
both words. Tag is not derived from host slot or BodyID. Actual rigid pool
recognizes states9/10/12 and removes using low state bits. Checkpoint2554 adds
owned-capability register/remove state-prefix methods; the actual target map
still does not exist. Earlier coverage had only mutated private fixture state.
Mutate actual live slot state without moving rings/replacing allocation or
changing occurrence, inertia, motion or forces.

## Removal chronology

RemoveSkaterBody827b74a8 asserts Island identity, calls CleanUpSkaterBody827b6988,
removes matching Body from static list, then dynamic list, then clears Island
pointer. BodyID is neither refunded nor reset.

Cleanup under Island lock: actual Parts ascending RemoveSpyTarget, remove Body's
contact-vector entry, actual SkaterPool Remove, clear Body Island pointer.
RemoveSpyTarget827b5608 asserts nonnull body, stores state&7 at827b5660, then
removes mapping if present. It is NOT merely state&!8: all higher bits clear.
Base-state list is preserved. Actual spy removal must precede body occurrence
invalidation. Existing hook/bone teardown guards still apply before detachment.

## Smallest connected actual-owner route

Reuse the current manager/backend, mixed SkaterPool and retained whole board/
skeleton owners. Add a source-scoped Island projection containing separate BodyID
counter, insertion-ordered static/dynamic membership, retained-member metadata,
private occurrence-qualified rigid-body -> member/original-Part-tag mapping,
and recoverable registration progress. No fabricated Body object, native address
image or mutable session/body-handle export is needed.
SimulationRigidBodyObservation already provides authority+occurrence equality/hash:
use sealed lookup identity, NOT a claim of native pointer order. Host map iteration
is not native pointer ordering. Actual state-bit mutation belongs behind existing
backend/session private actual handles. Avoid creating a second body manager.

Targets feed CreateContactSpyList827ba710. Later World::Update3 invokes
ResolveContactSpyList827b3a90 for nonempty vectors. Independent follow-up found
that BOTH board and skeleton Body::GetContacts callbacks are native no-ops:
Body vtable slot+8 at8225fdc8/82268b68 contains825187c0, whose complete body
is4e800020 (blr). The displayed unrelated function name is a folded alias.
Do not implement callback copying, forwarding or SkeletonCollisionData writes.

The real skeleton consumer is PULL-BASED: UpdatePostPhysics82b575d0 updates
physics records at82b576d8, calls Island::GetCollisions at82b57714, constructs
current collision-update inputs, then calls SkeletonCollision::Update at
82b57888. GetCollisions827ba6b0 locks and obtains/creates the SAME retained
Body contact vector; it does not copy or drain it. Collision Update reads the
retained ContactSpy forces, live bodies and authoritative physics history.
The meaningful next path remains actual registration, solved-spy export and
Island-owned contact vectors connected to that consumer. None is implemented
at checkpoint2541. See `contact-spy-feedback-source.md` for the recovered route.

This is more directly useful than isolated final IK/proxy-tail flags while no
real SkeletonIK owner exists. Root separately read raw Init82b62acc..82b62b28:
four ascending IK store groups at+1d4/+1f4/+1e4, doIK0 then1, proxyEnabled0,
a shared-symbol callee at82b62b14 (semantics not yet independently checked),
root Volume.enabled-bit clear and root disable counter0. That tail and
the actual IK owner remain necessary, but guessed objects/standalone flags do not
complete native initialization.

## SHA-256 function hashes

```text
AddSkaterBody aa045875ac51988ddde9dd7dc4afa59ec5803439151d9cf9cbfd7bb8b947ddc7
Island ctor ba7aa28bdb4ce1d2134439a387404e2a5fca54aa2fb2683043955c172036d490
SyncCheckBody 461d0af1bd9d4e6898b9729eb5235394baeaba946a0dd9b73f071f95cfcda83a
RegisterSpyTarget 95aa265711eaed71071c1f5b9b51b8a2ba312e28a2ba3df7de8541ce1322f3f5
RemoveSpyTarget 64031ccd22d4989ef048aec053eff8011ec2dff86bf04ea1458203b1250cff36
RemoveSkaterBody e4a50455c74713e0bb35a941f52cbc168dea4ff55336489c2c6faeca23e5bf35
CleanUpSkaterBody 4b638e8ddd02bf80bb88fce3c72c47978d8c870a1faf068a84c457c890a44f8a
ResolveContactSpyList 096f75180f00ee297f3e66fc550cf45e40826a9890b3a567b10bfe37acfae570
```

## Implemented paired-owner membership and target path (after2578)

The real source-ordered physical-player assembly producer now supplies this
Island projection. See physical-player-owner-next-source.md for full constructor,
raw routing correction and declared scope; final strict gate is in RESEARCH_LOG.
Independent source/implementation/test review PASS, high structural confidence.

`physical_player_island.rs` is owned by the EXISTING constraint world, over its
existing mixed SkaterPool/backend. Members retain actual component handles and
the same meaningful paired-player owner, not new empty Entity/Body tokens.
Legacy component-only owners are unknown and refused, not treated as null.
Constructed candidates reject a foreign manager before actual Insert mutations.
Membership checks the actual Insert-stamped requested state (skeleton fixed4)
before issuing its separate BodyID; state==1 alone selects static ID0/list.
All other requested states issue the wrapping nextID from1 and append dynamic.
Effective Part state and Assembly world indices are not classification inputs.
Both real constructors retain their issued BodyID through removal. Static-ID0
and wrapping-overflow branches are source reviewed but not exercised by the
new paired mounted path, which uses the native fixed active constructor state.

Host member storage reservation precedes issuance; source-list reservation
follows it. A list allocation error preserves the issued ID/actual member and
reports incomplete list publication. No native allocator-fault equivalence or
refund is claimed. The native lock-only SyncCheckBody requires no fake service;
the host's exclusive selected-world borrow serializes construction.

Narrow real Part operations resolve attached authority, backend, ordinal,
actual handle and current occurrence before applying state|8 or state&7.
Registration walks actual7/24 Parts ascending and stores actual occurrence keys,
component handles and ORIGINAL unsigned Part tags AFTER the state prefix.
Host map reservation is an explicit preflight. Re-registration replaces both
fields, never mints identities, and does not imply native pointer iteration order.
Completion marking/diagnostics now revalidate exact live Part/member/tag/occurrence
coverage and spy bits, not merely a historical success boolean. Per-map-operation
retained-entry counts and separate cleanup_completed_parts expose partial work.
Readonly target queries reject stale or replaced actual occurrences.

The existing typed board/skeleton Remove wrappers apply ALL detach guards,
including wrong-kind, skeleton drives and incoming/null post-drive guards,
BEFORE any spy/map mutation. Actual ascending Part cleanup clears state&7 then
erases each target, actual pool detach/swap-last follows, then all static and
dynamic list matches are removed. Component BodyID is neither reset nor refunded.
No contact vectors exist yet, so their native pre-pool-removal cleanup site is
explicitly inert, not an invented container/callback implementation.

Validation: four real-pool Part tests check state-only operations, ordered
preconditions, capacity-null drive guards and remove/reuse occurrence refusal.
Mounted actual paired construction verifies BodyIDs1/2 versus Assembly0/2,
31 target Parts with state12, distinct component handles sharing one player owner,
and non-target Hook/StaticWorld bodies retaining state1. Private corruption
checks remove a target, change its tag, or clear its actual spy bit, require
diagnostic/completion refusal, then restore original map/state. Actual cleanup
progress and re-registration are checked without exposing mutable capabilities.
Foreign-manager and contradictory requested-state refusals preserve prior IDs;
both guarded manual cleanup and full explicit paired teardown are exercised.
This is actual construction/registration, not contact-list consumption or skating.

NEXT: connect solved ContactSpy export/reset/freezing to source
CreateContactSpyList827ba710 using THESE current targets and owner relations,
then GetCollisions/SkeletonCollision::Update. No-op Body callbacks remain no-ops.
Never equate an unregistered body with an unknown-owner legacy member when
implementing native null-target behavior; recover that boundary explicitly.
