# Solved contact export and the actual skeleton feedback route

2026-09-07. Source recovery and XYZ exporter checkpoint2554. High structural
confidence; no native per-tick trace or complete Rust feedback implementation claimed.
Independent source audits checked the raw instructions and PDB layouts.

## Producer: Simulation::SpyContactJacobians

`82ae4a88..82ae4bf8`, SHA-256
`c2de62f301c95192427e27a393d1aed2fd540e35d922ec37c6fc0eb331f58cf8`.
Simulation fields: CJ stack+10, Contact count+64, ContactSpy count+68,
frequency+a4, spy flags+b4. Native compacts selected 256-byte Jacobians into
112-byte ContactSpy records in the SAME backing, in dense input order.

Read and square current frequency once, even when Contact count is zero.
Select only `(jacobian.nb & 8) != 0` and ordered normal impulse `> 0`.
Important independent correction: raw branch409900d4 at82ae4b0c branches when
CR6.GT is NOT set. NaN is skipped, just like zero/negative values. Reading the
printed `ble` as only an ordered less/equal condition would be incorrect.

For each selected record, read CURRENT body B COM before current body A COM.
Use compiled original A/B bodies, not solver-side/RF role order. XYZ equations
(native operations execute all four packed lanes independently):

```text
pointB = currentCOM_B + attachmentB
pointA = currentCOM_A + attachmentA
weightedB = pointB * inverseMassB
numerator = pointA.mul_add(inverseMassA, weightedB)
inverse = 1 / (inverseMassB + inverseMassA)
position = numerator * inverse
normalForce = (right * normalImpulse) * frequencySquared
frictionForce = up.mul_add(impulseY, at * impulseZ) * frequencySquared
```

No finite-value, zero-denominator, state or self-body special case. Preserve
the distinct multiplies and fused operations. Copy frame right/up/at and
original tag; retain actual body A/B identity. Store body identities/tag before
vector output, then increment existing unsigned spy count with wrapping.
This function does NOT reset the count. A checked host one-export-per-frame
boundary must be distinguished from arbitrary repeated destructive native calls.
StartFrame82e0bd80 clears CS_Count at82e0bd98 and82e0bdb8, CT_Count at82e0bd9c,
and stores frequency from `1 / managerTimeStep` at82e0bdac.

IDA VMX128 VA aliases are unreliable. Corrected VA registers at4ab0/4b44/
4b48/4b64/4b68/4b74/4ba0 are62/59/56/55/51/50/49, not94/91/88/87/83/82/81.
Raw standard vmaddfp operands at4b78/4b98 give the fused equations above.

ContactSpy layout: frame00..3f, normal force40, friction50, BodyA60, BodyB64,
packed tag68, total70hex. Frame right.W/up.W contain native Body pointer bits;
at.W contains tag bits. Point and force W arithmetic therefore has nonsemantic
packed provenance. Host XYZ output must explicitly omit unavailable native W,
not turn host tokens into fake native addresses or claim canonical-W parity.

## List creation

`Island::CreateContactSpyList827ba710..827bab14` consumes compacted ContactSpy
records, not raw Contacts. Under Island lock, clear ALL existing Body vectors
without erasing map entries/capacity, including when spy count is zero. Visit
spies in dense order; look up registered A then B and their actual Entity owners
before attempting output. Self-collision is same registered Body OR equal Entity
owner references; missing target supplies native null. Unknown host ownership
must NOT be substituted with null (native null/null equality is true).

Attempt A then B. Each owning vector has a hard16-contact cap. Same-Body A/B
share that cap; at length15 only A fits. Capacity is checked BEFORE live velocity
reads. A reads B then A and subtracts A-B; B independently rereads A then B and
subtracts B-A. No point/angular velocity. A normal=frame.right*+1, B=-1;
position=frame.position; A surface=tag low16, B=tag high16. Original registered
Part tag, this/other Body, self-collision, side flag and retained Spy reference
are kept. No force computation/filtering occurs here. Native velocity.W is an
intrusive pointer word; XYZ host semantics must disclose that boundary too.

ContactInfo size60hex: Spy reference00; normal10, position20, relative velocity30;
Part tag40, surface44, thisBody48, otherBody4c, selfColliding50, isBodyA51.
Unwritten native padding is not a source zero-initialization claim.

## Scheduling and actual consumer

PopulateJobTree82e0b410 schedules contact spies only with active bodies and
`Simulation.spyFlags & 4`; its contact-spy job depends on DynamicUpdate, and
the end job depends on contact spies. Step_Solver3 joins the end job, resets
static reaction forces, optionally freezes bodies, THEN creates the contact
list at827bae74. Thus exporter COM reads and later list velocity reads occur
at different source stages.

World::Update3 subsequently runs Adjust/Output jobs, conditioners, PhysicsOutput
update and collision-flag reset BEFORE Resolve (visual827afa30, simulation827afa58).
The apparent GetEditTake condition is a nonnull function-address test, not a call.
Resolve iterates nonempty Body vectors in native pointer-map order without
locking, clearing or draining; repeated calls redispatch. Host identity/hash
iteration must not be claimed to match native pointer ordering.

Both actual board/skeleton GetContacts callbacks are bare825187c0 blr. There is
no callback-owned collision-data copy. Skeleton::UpdatePostPhysics82b575d0:
bad-data check and board error; UpdatePhysicsRecords82b576d8;
GetCollisions82b57714; fill current inputs; SkeletonCollision::Update82b57888;
UpdateVolumesStates82b57890; UpdateRecords82b5789c. GetCollisions returns the
Island vector itself, creating an empty entry when absent.

SkeletonCollision::Update82b4b8f0..82b4c6fc resets/ticks state first, uses current
history, contacts in list order and live bodies. Foot-hit-board detection comes
before self-collision rejection; unrideable-surface classification precedes ignored
body filtering. Normal capacity20 bypasses subsequent response for that contact,
not merely normal insertion. Force/slide and final drive-strength work need full
raw-numeric review before implementation. Current input/state/cvar/Vault sampling
and later gameplay consumers remain required; no complete freeroam feedback yet.

## Other independently checked hashes

```text
CreateContactSpyList 878a6bd2d4d1d5130904f54e3e75ded9eb00d9a26fdf737799652ddc7c7b428a
ResolveContactSpyList 096f75180f00ee297f3e66fc550cf45e40826a9890b3a567b10bfe37acfae570
GetCollisions d1de68b506912f011f53bf2409ae611210efe9bfb8470fd4830213c0c8f61bb1
GetContacts no-op f332ea5b5437103cbb6f1508679da89eec9288ad775c96c439a17fccabe3de8e
Step_Solver3 75123e715cc569fb78a6664b57d7ea3667ed785b6424d6cb147a000ec27df9a6
World::Update3 86800f83fca5858894bf70e9dcf8a2999280930bbcfb2f1d00086f11c59a0522
UpdatePostPhysics 75202f926bc7035f871f25d4c332fb8ab29d80f80a1de98e9010596279fc896f
SkeletonCollision::Update 4785efa3f4b358a9791877426f4e5beae73dc42698a65bf5020081ecbf05b0ae
```

## Typed implementation and boundaries

`simulation_contact_spy.rs` implements the XYZ numeric graph and retained
original Contact/body provenance. The actual sealed Contact batch consumes a
private completed-Horus receipt and itself to export. The receipt retains a
read-only borrow of the SAME body pool after actual integration, plus frame,
phase and an opaque identity issued at that exact compiled batch's creation.
This prevents mutation/freezing before export and refuses mismatched batches,
including zero-iteration completion. No caller-supplied Jacobian creates output.

The existing serial solver API delegates to the same completion implementation;
no second solver is introduced. Empty Active phases have no body effects and
skip spies. Zero iterations still integrate, but freshly compiled zero normal
impulses do not emit. The actual authored-ground manager accepts an explicit
current spy-flags snapshot, exports after solving, and retains the resulting
frame. Readout checks manager, body pool, current frame and body occurrences.
It is NOT Island::GetCollisions, which still requires the actual Body map.

Separate immutable host storage replaces native in-place CJ/Spy aliasing.
Allocation is preflighted before deriving output. A later ownership/read refusal
drops earlier derived spies and publishes no successful frame, whereas native
has already compacted earlier records and incremented its count. Completed body
integration is not rolled back. This is explicit host failure policy, not native
partial-fault parity. A consumed batch/receipt disallows native's unsupported
repeated destructive call; output Vec length represents the once-reset count.

Actual rigid-body state-prefix methods now use the owned allocation capability
to set `state |= 8` and remove with `state &= 7`, without ring/body replacement.
Existing host integrity admits only supported base states1/2/4 and9/10/12;
unsupported high-bit state corruption is refused before this mutation. These
are source state prefixes, NOT complete Island registration/removal: Body/Entity
ownership, target map, constructor ordering and cleanup remain required.

The existing original-data board and mixed cases keep their actual states
unchanged. Their exporter is enabled with explicit flag4 but correctly emits
no spies until native target registration is connected. Positive actual compiled/
solved coverage is a separately labeled controlled network-contact fixture,
not a completed Island map or original-data humanoid response.

Independent downstream audit found only dot3 reductions/Y scalar decisions and
componentwise or YZXW cross operations in the recovered normal/world collision
branch and helpers: unknown W cannot affect those XYZ/branch results. Stored
normal/position/slide W still differs, and arbitrary later consumers remain
unaudited. See `skeleton-contact-response-next-source.md`.

## Validation

Strict67020 EXIT0: fmt/check, workspace tests (2326 core +187 formats +38 app
+3 symbols =2554 unit,6 doc), all-target Clippy `-D warnings`, all7 ignored
original-data core tests and git diff whitespace check. Thirteen new unit tests:
6 numeric/frame tests,2 actual state-prefix tests,5 actual solver/export tests.
Independent author-separated source, implementation and test review PASS.

Initial lib164088 passed; focused83758 passed8 kernel/state tests; integrated
author74389 passed7 matching (5 integrated +2 state). Initial strict16995 passed
the full workspace but stopped on doc-markdown and intentional consumed-receipt
lint; fixed with documentation formatting and a scoped ownership explanation,
no numeric change. Final67020 includes all production/test changes.

Positive fixture inputs are controlled real network requests/RF/state prefixes,
not native trace captures or completed registration. Dense source0/3 emit;
registered zero-impulse1 and positive unregistered2 skip. Tests distinguish current
post-integration COM from pre-integration, exact body/contact/tag identity,
frequency-squared force arithmetic, disabled flags, empty Active/zero-iteration
gates, exact-batch/phase refusal and stale/reused bodies/frames. No RF-reordered
producer coverage is claimed by this fixture.

All original core checks pass with unchanged Rob's Park C/J/D6/6/3,6/6/3,
6/6/3,4/6/3 and deck COM:
[588.9575,-54.351295,996.21686], [588.9615,-54.343952,996.2112],
[588.9654,-54.340397,996.2055], [588.9701,-54.33957,996.1999].
DMO8 frame hash0e7c704e6a10f08a and final[588.8619,-54.40114,996.5048]
unchanged. Original mounted output validates the enabled exporter with empty
selection, not positive skater feedback. No app ignored probe rerun.

## Post-export Static RF reset and conditional freezing

2026-09-07 continuation. The actual authored-ground path now continues past
export through this required prefix of `Island::Step_Solver3`; it still does
not create the Island contact vectors or run the complete scheduler.

Source order: join end job, call ResetStaticReactionForces at827bae4c, sample
CURRENT freezing target byte at827bae5c, conditionally call FreezeIfNecessary
at827bae6c, then CreateContactSpyList at827bae74. The new service input samples
the byte only AFTER reset; it samples the manager's current unsigned threshold
only for a nonzero byte. It does not cache either value at frame entry.

`ResetStaticReactionForces827b26d8..827b2738` follows the Static ring's right
links, clears all64bytes (16 words) at each nonnull Body.mId RF pointer, then
reads the next link. It changes no Body motion, force, inertia, state or cooldown.
Registered Static9 remains included by ring membership. Native Initialize binds
the final published bodies to permanent RF rows at82ae7c94/82ae7cc8; transient
constructor null pointers are not exposed by the host pool. Host anchors have
no Body/RF payload and empty rings are inert. This does not claim the native
uninitialized tiny-capacity sentinel anomaly is portable behavior.

`FreezeIfNecessary82e0be40..82e0bec8` samples manager Params.numFramesBeforeFreezing
(Params+24, manager+cc), captures the first at most100 qualifying Active bodies
in actual right-link order using unsigned `body.cooldown >= threshold`, and only
then mutates them. The 100th capture breaks before reading that node's next link.
`FreezeRigidBody82ae3298..82ae3304` unlinks Active, appends Frozen tail, writes
`state=(old&8)|2`, copies CURRENT Simulation.cooldown into the body and performs
wrapping Active--/Frozen++. No velocity, inertia, RF or occurrence writes occur.
The capture array has fixed capacity100 and introduces no allocation failure.

The threshold is not inferred from current pool cooldown. Manager ctor copies
Params+24 into Simulation.cooldown at82e0c71c, but these are separate later reads.
World Init sets BOTH manager thresholds to2 (li827b0ef0, stores827b0f20/827b0f7c,
constructor calls827b0fbc/827b1018). `kEnableFreezing`, category Physics_Simulation,
initializer82f6a5fc, defaults to1. Existing isolated board/mixed corpus frames
explicitly supply controlled zero, not the shipped default.

Implementation: `simulation_contact_feedback_phase.rs` owns actual pool writes;
`simulation_assembly_contact_feedback.rs` orders them after the manager's actual
completed/exported batch. Current manager/pool/frame/occurrence validation and
whole-ring topology preflight are explicit safe-host policy, not native lazy
fault behavior. The no-100th-next-read property applies to capture, not preflight.
Host progress records NotStarted/Started/StaticForcesReset/Complete. A later
input refusal keeps reset RF and completed integration/export, refuses replay,
and does not roll back or silently re-read a partial phase. The next actual
authored-ground frame resets that host progress and invalidates old spy output.

Positive freezing leaves the solved-spy frame valid: owner, frame and body
occurrences are unchanged. It invalidates SkaterPool publication/active stamps
as host safety policy, preventing same-frame direct consumers from accepting
pre-freeze frozen/header/active data. It does NOT eagerly recompute or clear
actual bounds/volume values or native retained island labels. The ordinary next
ground frame already refreshes all members. Missing thaw, assembly pair jobs,
Island target ownership/lists and the skeleton response remain explicit.

Independently checked SHA-256:

```text
ResetStaticReactionForces cf87487218e1d28e0988c8cde7c744a1eb9d03dfd126f7359dd28963ba9b532a
FreezeIfNecessary e54930731d6b09eb11f5e57ee3c2cd3240002defad223d9a14a7aa3d61b7fb0e
FreezeRigidBody 90d34ed0940a8b05143f282dde4272944ca14f0a40964699eaeabe56fcf23d30
```

Independent author-separated raw source and production review PASS, including
post-export read timing and freshness policy. Initial focused83500 EXIT0 passes
all6 pool tests: empty rings, exact Static RF words including registered9 and
NaN/-0 sentinels, unsigned threshold versus replacement cooldown, actual reuse/
right-order/100-cap, full preserved slot and RF state, and topology pre-effect
refusal. This first compile reported two diagnostic-field dead-code warnings;
the Display implementation now reads the contained errors. Bridge tests and
final strict gate are recorded in the subsequent validation addendum.

### Post-export validation checkpoint2567

Focused67550 EXIT0 passes12 pool/bridge tests. Six new bridge tests use real
empty Contact/Joint/Drive compilation, actual completion and export: disabled
lazy threshold, noncanonical true0x80, current replacement cooldown73 versus
threshold0, enabled/no-qualified-body, missing/stale output pre-effect refusal,
configuration failure retaining reset prefix/replay refusal, and consecutive
actual authored-ground frames with different current enable settings.

A thirteenth test extends the real positive solved-contact fixture: two actual
exported records retain all fields and pass endpoint validation after Static
RF reset and freezing their actual Active bodies. Threshold0 is an explicit
controlled selection, not a native sleep trace. Bridge freshness tests use an
empty SkaterPool and verify active stamps; populated member stamp invalidation
is verified by independent inspection of the existing all-member method, not
claimed as new populated-freeze test coverage.

Initial strict17752 passed2567 unit and6 doc tests, then stopped on five
doc-markdown and two assertion-style lints. Documentation backticks and
assert_eq formatting fixes changed no source behavior. Final strict12384 EXIT0
includes fmt/check, workspace2339core+187formats+38app+3symbols=2567unit,6doc,
all-target Clippy-Dwarnings, ALL7ignored original-data core tests, git diff
whitespace check. No app ignored probe rerun. Rob's Park's four C/J/D counts,
centers and DMO8 hash/final center listed above are unchanged. All production
and test changes precede12384; later edits only record evidence/status.

## Actual Island contact vectors and GetCollisions (after2592)

The actual registered physical board now reaches retained Island contact vectors
through mounted terrain collision, the real compiler/Horus/integration, solved
spy export, Static RF reset and conditional freezing. This closes the list
producer/getter path, NOT SkeletonCollision::Update or whole WorldUpdate3.
Independent author-separated source/production/tests review PASS, high structural
confidence, no native tick trace. Final validation is recorded in RESEARCH_LOG.

NEW physical_player_contact_list.rs owns the actual world's Body-vector entries.
Each ContactInfo retains an Arc to the EXACT sealed exported SpyFrame and its
dense record index, original component handle/other handle/Part tag, source
surface side, self-collision/side flags, XYZ normal/position/relative velocity.
Cloned arbitrary Spy records cannot construct these values or publish a frame.
The world's existing exporter now retains Arc<SpyFrame>; no second export or
numeric graph is introduced. The actual authored-ground caller invokes list
creation AFTER successful reset/freezing at the source827bae74 position.

Whole current manager/pool/frame/occurrence validation and completed feedback
are checked host preconditions. Creation then clears ALL existing vectors,
retaining entries/capacity even for zero spies. It visits the actual dense spies,
resolves BOTH A/B targets before reading BOTH owners (even same Body), then
attempts A followed by B. Actual missing target is native-null; found target
with invalid/unknown owner is refused, never coerced to null. Valid partial
registration works through a narrow member/owner lookup, not full-coverage
diagnostics. Both missing targets yield self=true but emit no output. Self is
same registered Body OR same meaningful player-owner Arc, not matching IDs.

Each Body vector has cap16 BEFORE checked reserve/live velocity reads. Native
A readsB827ba954 thenA95c, subtractsA-Ba964; B separately readsA827baa54 thenBaa5c,
subtractsB-Aaa64. A missing opposite target still requires its actual body read.
The new narrow pool XYZ-velocity accessor validates the same current occurrence
and rings as COM, without state filtering or unrelated serialized-field reads.
Normals use ordinary componentwise multiply+1/-1, not normalization/unary
semantic shortcuts; surfaceAlo16/Bhi16 and spy frame position are retained.
No force recomputation, additional impulse filter or invented callback occurs.

Native GetCollisions is lock/map[Body]/unlock with no membership/frame predicate.
The host getter obtains/creates the SAME vector and allows empty access before
first frame. Nonempty successful access additionally requires completed list
creation, exact current SpyFrame Arc and current occurrence validity. Those are
explicit safe-host restrictions. A separate readonly prefix diagnostic exposes
retained incomplete/old values without a current-output claim. StartFrame
invalidates success but does NOT prematurely clear native vector contents.
Checked later failures retain the clear/append prefix and disallow creation
replay until the next frame. Entry/vector reservations follow explicit host
allocation policy. Native pointer-map iteration order is not inferred from
host entry storage; later Resolve/callback scheduling remains outside this work.

Actual Body cleanup now erases its vector AFTER ascending spy-target removal
and BEFORE actual assembly removal, at the formerly documented empty site.
No Body/Entity tokens or separate collision-state owner were invented.

Ten new tests: six shared production-kernel/cap tests (independent endpoint
rereads, signed zero, full16 no-read, shared15 A-only, first/later read failure),
two real-pool velocity tests over all allocated rings/raw values/reuse, and two
actual empty compiler/completion/export feedback/freshness/replay gates.
Focused80478 passed6kernels+1bridge; focused69745 passed9 matching new tests
and the original-body mounted test before the final stronger clear discriminator.
The complete workspace gate includes the second differently named bridge test.

NEW physical_player_contact_list_corpus.rs extends the EXISTING original-body
ignored test via the paired factory's real board-ready subpath: actual retained
Vault gravity, original RobPark terrain,7 registered board Parts,6 joints/3drives,
source50 iterations and explicit controlled no-freezing. Positive first-frame
spies produce a nonempty actual vector. Checks exact exporter Arc/record address,
source side/Part/surface/normal/position/current relative velocity, known owner
and unmapped real terrain endpoint. A diagnostic epoch change refuses old
successful output while preserving readonly old data. The NEXT actual frame
has spyflags0 and MUST clear this proven nonempty vector while preserving its
entry; two subsequent spy4 frames validate current output if selected, else
empty output. Explicit factory teardown erases the actual Body vector.

Reviewer caught the earlier potentially vacuous clearing test (an intervening
physics frame could already have emptied it); root moved flags0 immediately
after the guaranteed-positive prefix. This changes only controlled test order.
The actual same-Entity board/skeleton two-sided contact is source reviewed but
NOT newly exercised by this terrain test. Same-vector cap timing is a shared
kernel test, not a fabricated actual contact producer. Host XYZ/exceptional
float, controlled spawn/schedule/cvars and missing live Actor/input remain
explicit. Skeleton's real response is the next consumer, not already executed.

Final strict33141 EXIT0:fmt/check, workspace2374core+187formats+38app+3symbols=
2602unit,6doc, all-target Clippy-Dwarnings, ALL7ignored original core checks,
gitdiffcheck. Initialstrict7733 passed workspace then stopped on two style lints;
root made the internal reference-only ContactSide Copy and used explicit raw
pointer syntax for the host record-address comparison, changing no behavior.
The stronger disabled-clear test is included in33141. Prior RobPark C/J/D and
centers unchanged; DMO8hash0e7c704e6a10f08a/final[588.8619,-54.40114,996.5048]
unchanged. No app ignored probe rerun. All production/tests precede33141;
subsequent edits only document the checkpoint and next-source requirements.
