# Skateboard wheel-line producer

2026-09-08, original debug XEX/PDB read-only audit. Confidence
**exact** for raw board caller/output ordering; **high** for existing-code reuse
and missing integrations. No implementation/Cargo or new live-query claim.

## Source identities and ownership

| Method | Half-open range | SHA256 |
|---|---|---|
| SkateboardBody::StartWheelLineTest(Vector3) | 82b24530..82b246fc | 3a5a762dfa832f2c21006dccb5618737cfe79675419fcd6e1130b54a81384829 |
| SkateboardBody::EndWheelLineTest() | 82b24700..82b24878 | f9dc220f8c2c0a55ae6ddfbfe1314b09ae2f4403947e786a28d4642bf7071d20 |
| Island::AcquireLineBatch(label,flags) | 827b8728..827b885c | 637034e117d7e24758085d18d2fa16a7886889ac835601da8fdc4c8555ef65bb |

PDB `Sk8::Physics::SkateboardBody`: actual assembly+18, CollisionInfo+40,
IIsland pointer+374, retained LineBatch pointer+378, deck LineTestInfo+1f10.
The constructor calls actual `Entity::GetPhysicsIsland825f1758` at382e8,
stores it at382f4 and clears batch at382fc. Entity obtains the actual physics
World from Simulation, checks the actual Entity's GetFlags() bit0 and dispatches World virtual+20
or+28 for the corresponding island. Do not independently choose a ground-only
owner or invent an island from the requested geometry.

Outer `Skateboard::UpdatePrePhysics82b37cb8` captures the full current
Reckoning up vector: outer+104, Reckoning+1b0 (`37cdc..37cec`), obtains actual
body through its virtual method, calls Start at37cfc, then ApplyForces37d04.
The up argument is by value in PDB and already copied before Start. It is not
the deck up axis, world +Y, or a per-wheel fresh Reckoning read.
Outer UpdatePostPhysics calls End at37d48 before bad-data/strange-acceleration
checks and before body PostPhysics37e5c. Start and End span the solve; the
same retained batch/results must survive it.

## Start: one batch, four thin wheel lines, one fat deck line

1. Read batch+378 at24550; nonnull asserts (source line448). A host error must
   not silently release/restart the prior batch.
2. Acquire via actual IIsland virtual+4c, label
   `SkateboardBody::StartWheelLineTest`, flags **7** (`24588`). Store returned
   batch+378 immediately (`2458c`), then Begin virtual+4 (`2459c`). Native has
   no separate capacity-null guard before this dereference.
3. Iterate parts **0,1,2,3**, ascending stride60. Each iteration reloads actual
   assembly/list then calls existing Part::GetTransform (`245c4`) and takes
   its full translation vector. Do not use Part's cached/local translation in
   place of the actual transform calculation.
4. Start is that full position. End is **position - up*MAX_HEIGHT**, where
   original `MAX_HEIGHT8225d610` is word **3e4ccccd**. Multiply then subtract
   are separate full-four-lane VMX operations (`24610/24614`), not an FMA.
   No radius offset, normalization, or velocity extrapolation occurs.
5. AddTest(start,end,+0 fatness) at24628. The caller ignores each wheel bool;
   it does not stop the remaining additions when a child has no capacity.
6. Reload assembly/list, obtain actual **part6** transform (list+240) at24664.
   Clear deck collision-valid (`24670`) before storing full start position to
   deck-info+0 (`24680`). End is full position plus **[+0,-100,+0,+0]**;
   this is world-down, independent of the passed up. `-100` word c2c80000.
7. Add deck test with fatness **3e19999a** (0.15000001), call246b8. Test its
   bool at246bc; false asserts (source line469) before End. It is not a miss.
8. End virtual+8 at246ec starts the work. Batch pointer remains retained.

Unknown W must remain unknown if actual Part transform cannot supply it. The
four-lane expressions are source operations; do not fabricate matrix W for
convenience. Query geometry may use only XYZ, but retained deck start is full4.

## Actual query route and batch semantics

The direct IIsland operation is `Island::AcquireLineBatch827b8728`: flags1
acquires World ground pool's line helper first; flags2 acquires this island's
DMO pool second; when island flags equal FLAG_Both and flags4 is set, its
vehicle helper is third. Flags7 therefore retains ground+DMO and optionally
vehicle in this order. One child returns directly; otherwise it constructs an
AggregateLineBatch. This direct island call does not use World::AcquireLineBatch
827ab918's visual-island flags override.
There is no skater collection in flags7; own board/skeleton exclusion is
structural, not an extra same-Entity filter applied to query hits.

Aggregate AddTest827bbe50 visits **every child** and ANDs bools without
short-circuiting. ClusteredMeshLineBatch::AddTest827bd9a0 returns false on
count==capacity before state/Next checks. Accepted requests retain their own
full start/end/fatness and reset result: lineParam max-finite, normal +Y,
position0, surface0, ownernull, hitcount0. The result's `hitcount`, not a
fabricated sentinel distance, determines hit/miss in GetResult827bda68.

Aggregate GetResult827bbec8 first tests index against EACH child's count,
queries admitted children, ignores nulls, and retains the nearest using
ordered strict parameter-less. Equal or unordered candidates preserve the
earlier ground/DMO/vehicle result. This matters if ignored wheel AddTest bools
leave children with different counts.

The existing `collision_query_batches.rs` already has actual capacity,
acquisition identity, Begin/add/End/sync/get/release and incomplete-job
failure retention. Reuse it. Existing `simulation_world_line_query.rs` has
the actual ground borrowing, retained constructor-empty dynamic collections,
DMO occurrence/membership validation and ordered child aggregation. Its
`try_line_test` currently acquires/begins/adds ONE test/ends/syncs/releases in
one call. That is not this five-test acquisition held across physics.

Expose a retained aggregate lease through that SAME query owner, with staged
Begin, Add, End, Sync, indexed GetResult and Release. Preserve acquisitions
and earlier child effects on errors; no implicit Drop release. Its End can
reuse the established synchronous local-job policy while retaining results
until actual EndWheelLineTest after the solve. Do not construct five separate
World single-query batches, because capacities, child effects and lifetime
would differ. Ground collection must be the actual retained world geometry;
existing nonempty-unpublished DMO refusal must remain. No broader DMO expansion
is required or authorized by this packet.

## End: defaults are legitimate only after the real batch

Require nonnull batch (assert line499), then Sync virtual+c at24750. Only
after Sync returns does it set **wheelHeightToGround body+358 = 0.2** (24764).
For each wheel index0..3:

1. Store full line normal `[0,1,0,0]` to body+2a0+i*10 (247a0), then distance
   +0 to body+2e0+i*4 (247a4), BEFORE GetResult(i) at247b8.
2. Null result retains these defaults. It does not directly change
   contactNormalValid or inContact.
3. Hit reads lineParam+20, reloads MAX_HEIGHT, multiplies (`247cc`). Read old
   wheelHeight, subtract `distance-oldHeight` (`247d4`), then fsel247d8 selects
   old when subtraction is ordered >=0 (including either signed zero), else
   new distance. NaN therefore selects the new distance; this is not Rust min.
4. Store selected wheelHeight247dc, then full hit normal+10 to wheel normal
   (`247e0..e4`), then distance247e8. Hit count does not publish a contact flag.

After all four, GetResult(4) at24814. On hit: deck-info valid=true first
(2482c), full collision position+10 (24834), full normal+20 (2483c), surface
ID+30 (24844). On miss: clear valid only (2484c); prior collision position,
normal and surface stay retained. Start-position was already stored in Start.
Finally Release virtual+10 at24860; clear batch pointer ONLY after successful
return (24864). A failed get/release retains the exact earlier field prefix.

PDB `VolumeLineResult` is size30: position+0, normal+10, lineParam+20,
surfaceId+24, owner+28. Deck LineTestInfo size40: start+0, hitpos+10,
normal+20, surface+30, valid+34. Relative CollisionInfo offsets are normal
array+260, distance+2a0, wheelHeight+318; add body CollisionInfo base40.

## Required geometry continuation

`collision_triangle_line.rs::try_triangle_line` currently supports only zero
sum fatness and returns explicit UnsupportedFatness otherwise. The deck test
necessarily asks for nonzero line fatness; success cannot be claimed by
dropping that test, substituting zero radius, or swallowing its refusal.

Original TriangleLineSegIntersect82adf658 nonzero branch first computes and
publishes normalized triangle normal (`82adf724..7ac`), then calls
FatTriangleLineSegIntersect82adeb48 at82adf7b0. On success the wrapper subtracts
normal times **lineFatness** from result position (`7c8..7f4`); f31 captures
original f1 at82adf668 before the summed radius replaces f1 atf684. Recover
this branch and its inner face/edge/vertex tests with the existing geometry
numeric boundary before implementing the full batch. This packet identifies
the dependency; it does not claim that larger leaf audited or implemented.

## Minimal integration and focused checks

Retain a board-owned wheel-line state whose batch lease is absent only before
Acquire or after Release; individual output fields need readiness rather than
default construction advertised as sampled. The session wrapper should validate
actual board authority then read Part0..3/6 through existing real Part transform
services. Capture actual Reckoning up at the outer caller. The same query
owner handles all children and the same board owner receives End writes.

Feed those real wheel fields into the already-audited PostPhysics fallback:
for a wheel not in actual contact, ordered distance < live threshold copies
line normal; otherwise clears validity. A real miss has distance0/+Y by source
and may participate in this fallback; do not replace it with infinite distance.
It still does not set inContact. See `skateboard-body-ground-normal-source.md`.

Tests should distinguish Start/End lifetime and replay, per-wheel actual Part
read order, full4 tilted-up versus world-down deck construction, ignored wheel
capacity bools versus asserted deck bool, child tie order, Sync failure before
defaults, GetResult failure after current-wheel defaults, hit/miss deck retained
fields, NaN/signedzero height fsel, and Release failure before pointer clearing.
Original corpus needs actual thin and fat geometry and at least one independently
checked miss/hit; no unconditional result service is adequate. Root owns Cargo
and eventual complete prephysics/postphysics integration.
