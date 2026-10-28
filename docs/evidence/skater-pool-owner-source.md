# Source-owned board membership in SkaterPool

2026-09-07, following the independently accepted 2384 authored-ground checkpoint.
Read-only recovery by mounted_probe; separate live source verification by
remapped_source_audit. Root read the full packet and live Insert (58 instructions),
Remove (52), BeginUpdate (159), and EndUpdate (80). Confidence: **high** for this
bounded source graph. The owner is now implemented and independently reviewed;
the later integration section distinguishes source facts, host policy and tests.

## Outcome and ownership

The next connected change is a genuine manager-bound AssemblyPool owner for
the already reconstructed board session and its collision publication. It must
admit a detached board before the real hook, retaining the actual session,
publication, separate bounds and an opaque membership capability. Do not consume
an index retrospectively for an already-attached board or create a dummy member.
The managed ground frame should borrow its board/publication from this owner,
not accept independently supplied pairs.

Keep this separate from full Island::AddSkaterBody, which also sets the Body's
Island pointer, allocates a distinct BodyID and inserts into Body lists. The
physical skater counterpart, those lists and the native pair producer remain open.

## Insert and three distinct IDs

`EA::Physics::AssemblyPool::Insert`, `0x82e0dbb0`:

1. Load manager `mNextObjectID` at manager+0x50; increment its unsigned 32-bit
   value with wrapping arithmetic (`0x82e0dbd0..0x82e0dbd8`).
2. Store the old value into Assembly `mWorldIndex`, Assembly+0x14 (`dbdc`).
3. Call Assembly::AddToSimulation `0x82ae0df0` at `dbe8`. Ignore its return.
4. Store the Assembly pointer into members[count] at `dbf8`.
5. For serialized storage, call SerialisedAssembly::Create `0x82e0c850` at
   `dc1c`; copy its Header bounds into the separate pool bounds (`dc48/dc50`).
   Nonserialized storage instead receives zero bounds.
6. Increment member count.

There is no native capacity, duplicate, null or Add-failure recovery branch.
Native raw member-pointer storage precedes Create, while committed count follows.

The manager counter starts at 0 (`0x82e0c324`). With a pristine component manager,
actual board insertion followed by actual hook insertion therefore yields 0
then 1. These are test expectations, not hardcoded runtime IDs or a promise that
no other native pool inserts first. The current direct-board corpus's hook ID 0
must change when it is migrated to actual board membership.

Part world-index is separate: initialized to 0 at `0x82ae901c`; these paths do
not propagate the Assembly index into each Part. Island BodyID is separate again:
Body+0x28, allocated from Island+0x4c, initialized to 1 at `0x827babb0`. Static
Bodies receive BodyID 0. Do not combine either counter or infer ID from a slot.

## Configuration and larger callers

Island::InitWorld `0x827b42c0` supplies serialized SkaterPool storage, capacity
16 when flags&1 and otherwise 2 (`0x827b4370/4380`), one assembly per update job,
and the current parallel cvar (`0x827b4354/435c`). Constructor call `0x827b4488`
uses the actual Island manager. The cvar initializer defaults false; that is not
permission to hardcode an immutable runtime setting.

AssemblyPool constructor `0x82e0d9c0` allocates member, bounds, serialized, job
input and active-list storage. A Rust owner may combine their lifetimes, not
claim allocator-layout or allocation-failure parity.

Island::AddSkaterBody `0x827b66d0`, when its Body Assembly is nonnull, selects
bending threshold 0.99900001 or -1 (`0x827b6718/6724`), calls Insert at
`0x827b6748`, then sets Body.mIsland (`0x827b674c`) and performs BodyID/list work.
CleanUpSkaterBody `0x827b6988` removes per-Part SpyTargets and the Body contact-map
entry before pool Remove (`0x827b6a78`). A pool owner is not those full callers.

## Remove and stable ownership, not stable dense order

AssemblyPool::Remove `0x82e0c1c8` calls RemoveFromSimulation at `0x82e0c1e8`
BEFORE searching the dense member array. If the removed slot is not last, it
copies the last Assembly pointer into the hole, assigns the last serialized
record with its assignment operator (`0x82e0c258`), and copies the last pool
bounds. It then decrements count. This is swap-last, not a stable suffix erase.

The moved Assembly keeps its world-index. No counter refund, removed-index
clearing or immediate active-list rebuild occurs. Rust should swap-remove the
corresponding session/publication/bounds owner together. A membership capability
must survive another member's relocation in the dense array without becoming
an ordinal-as-identity token. Missing-member/empty native removal is unsafe;
checked host lookup before detach is an explicit safety divergence.

## Refresh and ground iteration

For the **nonparallel** current setting:

1. BeginUpdate `0x82e0dc98` updates every serialized membership in dense order
   (`0x82e0de6c`), then queues bounds work over the complete range.
2. EndUpdate `0x82e0bb10` waits (`bb40`), copies all separate pool bounds into
   their serialized Headers (`bb7c..bb88`), then resets/rebuilds the active list.
3. Active-list reconstruction scans every live Part state bit2, in member/Part
   order. Not-all-frozen means active. Static-only nonfrozen assemblies are
   active; all-frozen and zero-Part assemblies are not.

Do not loop the current complete single-board try_refresh to implement this:
that would interleave Update/bounds/publication across members. Split internal
publication phases and preserve all-updates, all-bounds, all-publication, then
active-list work. Keep body ownership immutable until the exclusive collision
phase begins. Preserve explicit freshness invalidation on checked host failures.

The parallel branch batches distinct update/bounds jobs and EndUpdate skips
the nonparallel Header-copy path. Supporting only the false setting must be
explicit; a true current setting cannot silently be treated as false.

Ground Pipeline::Run `0x827cab00` consumes pool Begin() and **mNumAssemblies**
(`0x827cab34/74/80`), not the active list. Ground iteration therefore follows
the current full dense member order, including swap-last effects. Keep existing
job-range all-static gating (`0x827cacb0..d4`) and CPU per-assembly frozen/static
gates downstream. Do not introduce active-list prefiltering. Actual job grouping
must also follow that full-member order.

## Proposed checked-host seams and tests

Use a private SimulationSkaterAssemblyPool child beside the existing HookPool
owner. Bind it to the same contact-manager and rigid-body-pool authorities and
reuse the existing shared index allocator. Retain private session, publication,
separate bounds, world-index and opaque membership capability per member.
Expose only narrow handle-based operations for truck-drive admission, hook
connection, live spawn, inspection and explicit cleanup. No unrestricted mutable
session escape may detach beneath membership.

Admission failure must return the candidate and disclose its real state:

- Authority/detached/capacity/storage preflight failure: no index consumed.
- Failure after real index allocation: retain the issued index; never refund.
- Publication failure after attachment: return the attached candidate retaining
  its actual bodies and index. Do not silently detach/drop or claim a native
  transactional rollback. Staging ownership until Create succeeds differs from
  native early raw pointer publication and must be documented.

Tests should cover actual board-before-hook insertion, shared counter behavior
across owners, full/cross-authority/failure ownership, swap-last membership and
unchanged moved indices, multi-member phase order and full ground iteration,
active-list state distinctions, retained original geometry/constraints, source
spawn and explicit drive/hook/board teardown. Re-run the existing mounted ground
frames through the owner and keep the original serialized/image path unchanged.

## Full flags3 remains a separate requirement

Step_Collision1 `0x827b3818` ends SkaterPool update at `0x827b3838`. For flags==3
it asserts even membership and processes adjacent pairs (`0x827b38c4..3998`),
ordering each pair by native Assembly pointers before manager pair publication.
One genuine board member is useful but does not satisfy that branch. Recover
the actual humanoid/skater Assembly and pair producer; do not add a phantom
counterpart, invent native addresses or claim this restricted ground component
is the complete offline Tick.

## Implemented retained owner and managed frame

`dynamic_assembly_skater_pool.rs` now owns actual board candidates, admitted
sessions, exact collision publications, separate bounds and opaque membership
handles in one dense collection. It shares the actual manager index allocator
with HookPool. Native pre-Create raw pointer visibility is replaced by checked
host staging with returned candidate ownership; no ID refund or implicit cleanup.
Its source capacity derives from supplied world flags. A sampled true parallel
setting returns UnsupportedParallel before membership/ID effects.

Member operations expose immutable session/publication inspection and narrow
truck-drive, cross-hook, original spawn-transform and explicit cleanup routes.
No unrestricted mutable session or publication escapes. Hook operations check
the same manager and backend; spawn invalidates publication freshness before
any potentially partial source writes. Removal retains original detach guards
and swaps the entire member/publication/bounds owner. Active diagnostics become
unavailable until refresh, rather than silently publishing a new active list.

The publication's internal phases are now separate: invalidate, Update,
stage bounds, publish bounds, and rescan live frozen state. Pool refresh runs
all members through each phase before beginning the next. Bounds staging uses
updated stored record transforms with current live motion. A later checked
failure invalidates all freshness stamps without undoing completed data writes.
All-static/nonfrozen membership remains active; ground never filters by Active.

`skateboard_pool_session.rs` gives the existing managed owner these operations.
SkaterPool is explicitly initialized with the caller's current setting; an
uninitialized ground call fails before frame effects, and reinitialization
cannot replace retained ownership. Both HookPool and SkaterPool use the same
world flags. The existing component constructor still selects flags3; the new
explicit-flags constructor does not claim the corresponding full Tick.

`skateboard_ground_frame.rs` now obtains the full publication iterator from
retained members after whole-pool refresh, then runs the existing isolated CPU
job and same Contact/Joint/Drive Build/Horus/integration path. Caller-supplied
board/publication pairs were removed. This is still one isolated serial CPU job,
not Pipeline::Run worker batching, pair construction, live cvars or full Island.
The original serialized/image path is unchanged.

### Verification

All managed mounted board runs now admit the detached original board into the
pool before creating truck drives and HookPool membership. Their actual shared
indices are board0/hook1 in the pristine component manager. Original source
geometry/material/culling, seven body occurrences, six joints, three drives,
live relocations and teardown are preserved. Four Rob's Park ground frames
retain the previous counts6/6/6/4 and the previous reported deck poses. The
diagnostic extra publication used for foreign/stale refresh checks is dropped
before runtime; actual collision consumes the retained member publication.

Eight new ordinary tests cover empty real-manager pool capacities, parallel
refusal without ID consumption, foreign authority at equal numerical frames,
empty active publication, unissued-handle refusal, shared world flags,
reinitialization refusal and missing-owner failure before frame mutation.
No fabricated board geometry or publication supplies these tests.

Three new mounted-template scenarios cover nonempty ownership:

- Capacity2: two actual boards admitted, third Full returns detached candidate
  with initial ffff index and no body effects. Removing the first swaps the
  second into its position while preserving world-index1, exact observations,
  geometry records and bounds. The retained refused candidate then receives2.
- Body capacity8: manager plus one board fills body storage while membership
  has space. A second Add fails at Part0 after consuming ID1; the actual returned
  owner survives foreign backend/manager refusal. Removing the first permits
  retry with ID2. No hidden repair, index refund or substituted backend.
- Three real boards with controlled states4/2/1: current live state, all19
  records each, active ordinals[0,2] and full publication range3 are checked.
  The actual ground CPU job first rejects stale member0 bounds; after refresh
  it processes1/skips2. Thus frozen/static filtering occurs downstream, not via
  Active. Initial poses are far from the real resident mesh, so this selection
  test claims no generated contacts. All actual members are explicitly removed.

These tests are not injected allocator-failure or post-Create-failure traces.
Later checked refresh failure retains source-prefix/host-invalidation policy by
code review; there is no native malformed-memory recovery equivalence claim.
Independent source/production review passes; final regression results are in
STATUS and the latest RESEARCH_LOG handoff.
