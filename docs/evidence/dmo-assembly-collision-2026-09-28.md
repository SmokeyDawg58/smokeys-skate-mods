# DMO assembly collision continuation, September 28

Status: generic contact entry, DMO-DMO CPU driver, pair/thaw lists and
single-player island partitioning connected. No compilation or tests executed.
Read dmo-grab-runtime-2026-09-28.md for the preceding grab/Pushing work.

## Confirmed source, current IDA database

- Step_UpdatePrePhysics2: DMO-ground Run827b2954 precedes DMO-DMO
  AssemblyAssemblyCollisionPipeline.Run827b2978 (argument false).
- Step_Collision3: DMO-DMO Sync827b4198 follows skater-ground, DMO-ground,
  skater-DMO, skater-vehicle Sync.
- InitWorld827b42c0, DMO configuration827b4e64..4ea8: Params constructor,
  assembliesPerJob=5 (r26 initialized827b492c), allowMultipleContactWrites=true
  (r23=1 at827b45b0), additionalPadding=0x3e19999a (0.15). Pool6cc8,
  pipeline6cf8. Do not use the unmodified Params defaults (1/false/0).
- Pipeline Run82e0bef8: contiguous A groups; intra-pool B starts at the group's
  A start and runs to pool end. Skip a whole job only when all its A headers
  have num_static==num_rigid_bodies. Frozen is not this test.

## AssemblyAssemblyCollisionJob827cc410

Hex-Rays fails at827cca98 because RunIntersectionTests has a bad inferred
prototype. Raw disassembly was used; no database type changes were made.
Its actual mangled signature is two sets of (VolumeData*, ExtraVolumeData*,
AABB*, count), TempContactBuffer&, ConstantData by value, float padding,
BitTable const*, CollisionType. Extra pointers both refer to the real body.

For every A, prepare Extra and swept bounds for every non-Aggregate volume,
including disabled/static/frozen volumes. Existing body_motion and
try_calculate_primitive_part_bbox provide this operation; the outer ground
run constructor cannot be used unchanged because it skips frozen assemblies.

For every B:

- Intra-pool skips A>B (827cc6b4), allowing the self case separately.
- Different assemblies: XYZ AABB rejection uses strict > (827cc6c4..700).
  Skip both frozen, both all-static, frozen A against all-static B, or
  all-static A against frozen B (827cc704..768).
- Pipeline culling tests assembly groups, then registers the sorted actual
  assembly-pointer pair BEFORE narrow phase (827cc76c..830), even if no
  contact results. Do not create pair membership only from accepted contacts.
- Prepare B extras/bounds, call RunIntersectionTests with CollisionType0.
  Its returned found flag appends each frozen participant to the thaw list,
  A then B (827cc950..9f0).
- Self case: only when assembly.cullingTable is nonnull. Register (A,A), then
  test A against itself with CollisionType1 and that assembly culling table
  (827cc9f8..ca98). No distinct-pair frozen/static/AABB gates on this branch.
- TempContactBuffer.GetNextFreeContact827cb7e8: 50 local records. Full buffer
  flushes when permitted; otherwise returns null. Invalid contact decrements
  the local count. Final job flush at827ccb08. Existing
  simulation_contact_append.rs already implements the reservation semantics.

## RunIntersectionTests827cbdc8

Volume A outer, B inner. Skip Aggregate operands. A must be enabled even if
it is the only volume; B may be disabled only when numVolumesB==1.
Self collision skips equal volume indices and equal rigid-body indices;
otherwise test assembly self-culling by body indices. Distinct pairs use the
pipeline culling table by volume groups, with null table admitting the pair.

Material selection, preserving ordered/unordered branches:

- restitution = A.res >= B.res ? B.res : A.res;
- dynamic friction = A.mud <= B.mud ? B.mud : A.mud;
- static friction = A.mus > B.mus ? A.mus : B.mus.

Skip only if BOTH raw body states equal STATIC_BODY=1 (827cc004..018).
Padding=(B.padding+A.padding)+additionalPadding, two fadds827cc02c/03c.
Construct GP A then B, then test their prepared AABBs. Call the generic
ComputeContactPoints82ad5d80 with isObject=false (r9=0 at827cc0d4).
Contact tag=(A.surfaceID<<16)|(B.surfaceID as u16).

For each point, GetNextFreeContact; set found=true immediately after success,
before Contact.IsValid. Preserve actual A/B live and table body observations;
unlike the skater-DMO job, no forced-active B word. Common row construction
uses points, normal, live velocities/COM/inertia, selected materials and tag.
IsValid827cc398 uses 0x37800000. Invalid removes the slot but leaves found=true.

## Implemented generic narrow phase

narrow_phase.rs now has try_compute_contact_points for82ad5d80. It shares the
existing GP feature/prism construction, normal selection, triangle fixup and
packing; no competing geometry implementation. Two intentional differences
from the skater function:

- Generic contact limit is (B.fatness+A.fatness)+padding, raw fadds82ad5e0c/24.
  Sk8 uses (B.fatness+padding)+A.fatness, raw827cce94/a4.
- Generic checks first triangle with rev=false, THEN second with rev=true
  (82ad6040..60b8), returning before packing on rejection. Sk8 checks only
  second and preserves scratch packing even after rejection.

Both call FixUpTriangleResult82acb5d0. Generic direct separating-function
dispatch is the same table called by FindBestSeparatingDirInline82acc050,
which Sk8 calls. Final normal negation/fatness offsets share the existing
serializer (82ad60bc..6160 versus827cd140 onwards).

Added unexecuted regression tests for one-sided triangles in either operand
position and a rounding-boundary pair that distinguishes the two padding
orders. Scoped rustfmt passed; this is not type checking or test evidence.
Current numerical limitations of the existing GP backend still apply.

## Integration requirements identified in the first trace

- Connect run_dmo_dmo in skateboard_ground_frame.rs with original settings,
  real contacts and thaw results. The flag currently has no runtime consumer.
- Preserve primitive-pair provenance: SimulationGroundContactSource presently
  requires an AssemblyGroundTriangleSource. Do not fabricate a triangle for
  DMO primitive contacts. Extend the shared producer provenance instead.
- Reuse existing TempContactBuffer and common contact compositor; preserve
  combined frame capacity and native job grouping/flush behavior. Native job
  ordering is concurrent; host serial ordering must remain an explicit limit.
- Assembly-pair partitioning is missing even from the existing skater-DMO
  path. simulation_island_state.rs only implements StartFrame labels and hook
  exclusion. The frame snapshots those labels BEFORE collision/thaw today.
  Trace this before relying on newly awakened DMO solver participation.
- PartitionAssembliesInIslands82e0e2d0 consumes registered pairs in order:
  neither known -> assign same next island and advance modulo maxIslands;
  one known -> inherit; both known/different -> replace EVERY occurrence of
  second's island with first's island. Then write each assembly's Part labels
  in pointer-map order. This is not generic union-find: modulo island reuse
  can merge otherwise unrelated groups. Need real owner/pair capacity/order
  and full Step_Solver2 scheduling before integrating.

Broader goal remains load/render/collide/grab/move plus build/playtest
validation. No end-to-end completion claim.

## Driver and solver integration continuation

simulation_dmo_dmo_contact.rs now implements the five-assembly jobs and pair
traversal above, with common GP, Extra/BBox, material compositor and existing
TempContactBuffer. The latter exposes its already-implemented discard-last
operation through reserve/compose/validate; invalid points still set found.
It flushes only at job end (and capacity-driven internal flushes).
Disabled A, singleton disabled B, aggregates, frozen/static combinations,
self culling and distinct volume/assembly culling use their separate native
tests. No fabricated triangle source: the shared record's target provenance
now distinguishes an actual ripped triangle from an actual primitive pair.

skateboard_ground_frame.rs consumes run_dmo_dmo after DMO-ground, reserves
remaining Contact capacity, then runs skater-ground and skater-DMO. Four
ordered record slices enter the same sealed contact compiler. Native workers
race; this serial prefix preserves one valid scheduling order, not a claim
to reproduce native inter-job atomic timing.

New simulation_assembly_pairs.rs is a frame-local bounded pair/thaw owner.
World.Init827b0ee8/0f0c supplies maxAssemblyPairs500 in Params+14; manager
ctor82e0c778..7a0 consumes it. Thaw cap1000 from82e0c7a8..7d4. Pair identities
are the borrowed retained publication owners and actual Part body bindings,
not guessed arena object IDs (multiple instances can share those). Pointer
keys sort actual host owners in the same manner as native Assembly pointers;
host allocation addresses and ordering are not claimed Xenon-identical.

Skater-DMO now emits its admitted pairs before narrow phase too, using its
native827cf434..494 registration position after assembly culling. Duplicate
thaw pushes are retained; each visit checks CURRENT body state before waking,
then clears cooldown for every Part. Both producers share remaining500/1000
capacity. No fabricated contact is required to admit an overlapping pair.

Step_Solver2 verified live827b35b8: SetupSinglePlayer (hook exclusion), drive
normalization, conditional ThawAsNecessary827b3690, unconditional single-player
PartitionAssembliesInIslands827b3698, then CreateSimulationIslandTables.
Step_Collision3 sets maxIslands10 for flags3 at827b40b0..b8 (online8/1 are
outside this selected schedule). New partition_pair retains native modulo
reuse, one-sided inheritance, full-label replacement and self-pair handling.
It validates real occurrences before publishing Part labels. The solver map
snapshot now follows thaw/partition; rows retain their pre-thaw contents and
are rebound to the new integration phase, as the previous thaw path did.

Shared preparation correction: native SkaterDMOCollisionJob has no frozen-A
header test; its sole lbz(allPartsFrozen) is B's thaw check827cf614. Ground
preparation's frozen-A skip is now an explicit caller choice, retained for
ground and disabled for skater-DMO. Whole-static A remains a pipeline skip.

Static review caught the shared compositor's surface packing requirement:
it extracts B surface from triangle_user_tag HIGH16. Primitive B surface is
adapted to that word before the shared compositor; emitted tag is exactly
(A.surfaceID<<16)|(B.surfaceID as u16), not an always-zero B tag.

Added unexecuted tests: partition inheritance/self/duplicates and modulo reuse
merging unrelated groups; invalid contact slot release; material unordered
and signed-zero selections. Existing ground fixture provenance updated.
Only scoped rustfmt/static call-site inspection performed, no Rust type check.

Adjacent skater-pair audit: Step_Collision1 `827b386c..3998` appends each dense
SkaterPool pair (board/Skeleton), with an even-count assertion, before the
skater-ground Run and independently of its cvar. The live frame now appends
these actual publication pairs to the same bounded buffer and reuses one
retained publication vector for ground and skater-DMO. No intersection or
Contact is fabricated. The old September 23 source-gap note had its later
pipeline branch reversed: `r27=0` for flags3, so `bne827b39f4` does NOT skip
skater-DMO. That note is corrected from the raw instructions.

Next: the live DMO state/animation flow, then remaining rendering omissions. The
existing broader skater-skater pipeline is not represented by the new DMO
driver; confirm the required grouping and cvar paths instead of inventing
extra pair connections. Compile/runtime evidence is still required for all
new code. Do not declare the full DMO goal achieved.
