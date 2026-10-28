# Per-frame Drive normalization before Build

2026-09-07, source recovery followed by implementation and validation below.
Confidence **high** for the live PPC/PDB traversal,
mutation targets and operation graph. Exact Xenon dot-reduction rounding,
estimate bits, denormal handling and exceptional results remain an explicit
numeric boundary, not a host-libm parity claim.

## Source identity and actual callsite

`Sk8::Physics::DriveNormalizer::operator()(rw::physics::Drive const&)`,
decorated `??RDriveNormalizer@Physics@Sk8@@QBAXABVDrive@physics@rw@@@Z`,
range **827b2738..827b27d0**, SHA-256
`83da34eec111e2897045fbf1b08bd63d112517bdc579db1fdf77504e4e4f14ee`.

The concrete `eastl::for_each<LinkedListConstraintIterator<Drive>,DriveNormalizer>`
is 827b30c8..827b3124, SHA-256
`9b80bc687a4e873b68a1b25b8db714aa03024ff6510bc96ebdc2f4e01d40dce4`.
It calls the functor at 827b30f8.

Actual caller: `Sk8::Physics::Island::Step_Solver2(float)`,
`?Step_Solver2@Island@Physics@Sk8@@QAAXM@Z`,
827b35b8..827b36c0, SHA-256
`2a64a1437ff7506fbd80026491929ce51b9141e8bf58007e9dc0ec48a7b134fb`.
The normalization for_each call is **827b3610**.

Its source order is:

1. Check mFlags at +8 against FLAG_Both=3. At 827b35dc call
   SetupSimulationIslandsForSinglePlayer for equality; otherwise call
   SetupSimulationIslandsForOnline at 827b35d4.
2. Obtain DrivesEnd at 827b35f4, then DrivesBegin at 827b3604.
3. Normalize the resulting Drive range at 827b3610.
4. Obtain/assert the job scheduler.
5. For FLAG_Both, optionally ThawAsNecessary at 827b3690 according to the
   enable-freezing byte, then PartitionAssembliesInIslands at 827b3698.
6. CreateSimulationIslandTables with separateContactsAndContactJacobians=0
   at 827b36a8.
7. Add the job tree at 827b36b4, through which later constraint Build/solve
   consumes the now-normalized retained frame data.

Normalization runs for both online and single-player Step_Solver2 branches;
it is not conditional on the freezing toggle or number of Active bodies.
It is after island setup and before thawing/partition/table creation, not an
arbitrary late fixup inside the Drive numerical composer.

Higher actual symbols: World::Update1 at 827ae008 calls Step_Solver2 for
simulation/visual islands at 827ae1e4/827ae1fc in its visual-island scheduling
route. World::Update2 at 827ae210 calls it at 827ae2a4 for the simulation
island when the visual route was not selected and update mask bit1 is set.
The preceding call is Step_Collision3. Simulation::SimUpdate1 at 8289f410
calls World::Update1 at 8289f758; SimController::Update1 calls SimUpdate1 at
828a7690. The exact recovered normalization owner is Island::Step_Solver2,
not an inferred new function named TickSinglePlayerPhysics.

## Active ring, not Active-body filtering

SimulationManager::DrivesBegin is 82e0bd58..82e0bd6c, SHA-256
`000cb23971e1df6c65b9575a4436a62a503320282850b69d474e17451bf8fc94`:
load manager.mSimulation at +0, load Simulation.m_ActiveDR_Anchor at +38,
load anchor.m_right at +8, return that iterator.

DrivesEnd is 82e0bd70..82e0bd80, SHA-256
`1b0f4d3afa1155d179caee47854f0600a5ae602a77cb654ce41563aaec18418d`:
load the same simulation and +38 anchor and return the anchor itself.

The for_each loop compares current against end before entering. For each
entry, invoke DriveNormalizer first; only afterward load current.m_right
at +8 (827b3100), advance and compare against end again. Empty range has
no numeric calls or writes. Ring order is current Active right-link order,
not numeric slot order or body order.

PDB Drive layout (size20): m_skel +00, m_crtl +04, m_right +08, m_left +0c,
m_bodyA +10, m_bodyB +14, m_tag +18, m_spy +1c. The functor reads only
m_skel. It does not inspect bodies, Active-body bits, island labels, dynamics
type, spy, or tag. Consequently it normalizes Active-ring drives even when
both endpoints would fail the later Build body's Active-bit predicate.

`SimulationDrivePool::try_active_build_order` and Build snapshot traversal
are NOT appropriate producers for this operation: they validate/read bodies
and filter on ((stateB|stateA)&4)!=0. Normalization requires its own narrow
owner-internal traversal over every Active Drive occurrence.

## Exact payload and numeric schedule

827b2738 loads drive.m_skel. The only conditional gate is pointer null:
827b273c compare zero, 827b2740 return when equal. A nonnull frames pointer
permits all remaining operations unconditionally.

Payload addresses: quaternion_a at frames+00, quaternion_b at frames+20.
position_a +10 and position_b +30 are untouched; m_crtl and all dynamics
fields are untouched. No frame pointer, list link or Drive metadata changes.

The following is the exact numerical dependency/callback order:

1. Load QA at 827b2748; vmsum4fp(QA,QA) at 827b274c produces squared norm A.
2. Load QB at 827b2754; vmsum4fp(QB,QB) at 827b275c produces squared norm B.
3. vrsqrtefp estimate A at 827b2764.
4. vrsqrtefp estimate B at 827b2770.
5. First refinement operations interleaved A/B: square A, half A, square B,
   half B, residual A, residual B, refined A, refined B (827b277c..827b2798).
6. Second refinement repeats that order at 827b279c..827b27b8.
7. Multiply original QA by refined A at 827b27bc; original QB by refined B
   at 827b27c0.
8. Store normalized QA at 827b27c4, then QB at 827b27c8.

For each vector lane, each refinement round is:

```text
square   = estimate * estimate
half     = estimate * 0.5
residual = (-squared_norm).mul_add(square, 1.0)
estimate = half.mul_add(residual, estimate)
```

One and half are generated by integer-vector-to-float conversion, not Vault
tuning. Two rounds occur for both quaternions. There is no final additional
rsqrt, scalar division, epsilon, sign canonicalization, zero-length selection,
unit-length early exit, or nonfinite-value guard. In particular do not adopt
an unrelated normalize-or-identity helper or return zero for a zero vector.

The four-lane squared norm includes W. Do not replace it with XYZ length.
The instruction-level dot reduction and estimate results should remain
explicit numeric-provider boundaries. A portable host may use the existing
world quaternion helper's nested host dot policy and sqrt().recip() seeds,
but must label these as host models, not exact Xenon vmsum4/vrsqrtefp bits.
The explicit fused refinement graph should remain implementation-owned.

Under ordinary host IEEE modeling a zero quaternion will reach exceptional
estimate/refinement results rather than a source guard. NaN, infinity and
underflow are not rejected by the original code. A numerical port must not
silently sanitize them; exact target exceptional bits are still unvalidated.

## Alias visibility and safe-host failure boundary

Each iteration rereads its current frame values. If two Drive occurrences
retain the same frame owner, the second occurrence normalizes the result
written by the first. Do not deduplicate owners, snapshot every frame before
starting, or precompute every result before publishing all writes. Even
near-unit quaternion normalization can change bits on a repeated pass.

The original reads both quaternion inputs and completes both calculations
before either store. Thus a fallible host numeric provider should not publish
QA when a later B estimate operation fails. Previously completed Drive
occurrences may already have changed: an all-range rollback is not native.
Position vectors and dynamics must survive byte-for-byte, including signed
zero and uninterpreted NaN payloads.

The existing pool retains Arc<RwLock<SimulationDriveFrames>> even after
Drive removal. Free-ring entries are not visited merely because that owner
remains retained. A separate active occurrence sharing that owner may still
change it through the alias, as native pointers would.

Implement structural validation and any lock/poison handling as explicit
safe-host policy. This path needs no body-pool reference, body occurrence
validation or fabricated frame capability. An owner-internal traversal may
hold stable topology while it accesses frame owners and then reads each next
right link. Current public Add always retains a nonnull frame owner, but
the source null-frame early return must not be mistaken for a zero quaternion
or missing dynamics check.

## Smallest next implementation and tests

Add a narrow normalization method inside SimulationDrivePool, using its
actual Active ring and retained frame owners. Expose only what the real
backend/manager frame owner requires; do not export arbitrary raw Drive rows.
Invoke it at the recovered pre-Build scheduling point. Keep repeated source
calls observable rather than inserting an unsupported normalize-once cache.

Use a small numeric port for ordered normA, normB, seedA, seedB. Preserve both
rounds and A/B publication order in authored Rust. The existing immutable
fresh truck-drive values or their construction normalization do not remove
this per-frame obligation: source setters produce unnormalized candidates.

Focused tests: empty Active ring; nonunit A/B with W contribution; all four
numeric calls in source order; 0/1/2-refinement discriminators with injected
seeds; both body endpoints inactive still normalized; dynamics type NO_DRIVE
still normalized; positions/dynamics untouched bitwise; freed occurrences
not visited; repeated aliased owner normalized twice in ring order; FIFO
remove/readd order; null frame skip in a clearly synthetic private fixture;
zero/nonfinite inputs without default identity; later numeric/poison failure
retains earlier complete occurrence writes but not a half-published A/B pair.

The original source investigation modified no game assets and performed no
Rust or original-runtime execution. The later implementation gate follows.

## Implemented and independently reviewed

simulation_drive_normalization.rs is a private child of SimulationDrivePool.
Its own topology-only validator checks both rings' links, membership, counts,
bounded traversal and total coverage without reading frame/dynamics/body/spy
payloads. Existing Add/Build validate_integrity is unchanged: its stricter
payload requirements would incorrectly defeat the source null-frame skip.

The actual Active traversal preserves per-occurrence alias rereads and ordered
normA/normB/seedA/seedB callbacks, two interleaved refinement rounds, both output
multiplications before stores, A then B stores, then the next-right-link read.
One per-owner write guard spans numeric callbacks and publication. Callbacks
are explicitly pure/nonreentrant and must not reacquire that owner. This host
locking policy is not a claim about native concurrency. Errors leave the
current pair unchanged and preserve earlier completed occurrences; panic
poisons the current owner. No batch snapshot or alias deduplication is used.

Twelve tests cover the discriminators above. The extra root FMA fixture uses
norm68d392ce and host sqrt-recip seed: both A/B refine to2ac71ea9, whereas split
operations yield2ac71ea8. These are controlled numeric fixtures, not native
estimate oracles. Separate live PPC/code/test review passes.

simulation_assembly_constraint_session.rs now calls normalization before the
existing checked Build producers. The actual authored board corpus executes
four retained contactless/pair-free/non-Hook frames with six Joint and two Drive
records and actual RF effects/integration. See authored-constraint-solver-path.md.
Final root gate71989:2325unit tests,6doctests, fmt and warnings-denied Clippy,
plus all4core original-data tests pass. No native whole-Tick parity is claimed.
