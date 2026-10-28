# HiLOD state and suffix integration, September 12

The same retained physical owner now constructs all26 state allocations in the
order of PhysicalPlayerHiLOD82e03c50. The22 previously absent states interleave
Sleeping, Air, WipeoutGround and Ground. Source-selected Vault classes and live
Pushing cvar44 are mounted, with unknown PhysicsSkater fields preserved.
RemainingStates::check_original runs against the actual original-data pair.

Root independently checked base82d83aa8, Revert82d912a8, GroundAnimation82d83dd0,
KnownAir82d8b578, GrindCommon82d84fa8, HandPlant82d972c0, FootPlant82d975d8,
Boneless82d977e0, OffBoard82d916d8, Locomotion82d91b18, Landing82d97f48,
OffAir82d94040 and OffJump82d98428. Fresh raw Grind confirms three J vectors,
I/J/K/zero precontact orientation and zero jump velocity. KnownAir preserves
unwritten heading/speed/startY/comMaxY. Independent ground-agent review verified
Skitch/Pushing, actual late attribute reads, PID histories and nested GrabSpline
reset. The author corrected Teleport numeric bit fields to1/0/0 after PDB review.
Detailed addresses and keys: remaining-physical-state-constructors.md.

Root mounted the early inline HiLOD stores before ProcessedPhysIn. Actual AI
deck transform is zero, trajectory time is four lanes of-1, high flag writes
producee0000000 in cleared Sim storage. Collision bit01000000 has a separate
single authoritative tail owner. Existing prepared velocity and physics-state
owners receive their source fields, including framesSinceJump100. Three line
results reset both early and late. PhysicsDebugSystem retains a real one-byte
uninitialized allocation before RemainOnReset; independent review caught and
removed an initial host zero-sized representation.

The selected World startup retains actual GameInfo network-field ownership and
the source empty nine-pointer physical collection. Suffix orchestration uses
the same Actor/pair: capsule, Ground entry, Wipeout invincibility0.04, jump vector
zero, live network query, flag-before-board-before-skeleton collision changes,
World.Add of the actual registered allocation, JPhy zero. Board publications
now use actual per-session collision groups and reject stale group revisions.
See hilod-constructor-tail.md for raw/PDB proof.

**Completion limit:** suffix return is not full constructor completion. Closure
review found omitted Skeleton HeadTracking/SkateboardWobble owners, outer
constructor fields and a conflated Skeleton constructor/Init schedule. Actor
cached physical publication therefore also requires a separate actual Skeleton
completion proof, currently false. Early registered IPhysical remains usable;
cached Actor+200 remains unavailable. No continuous skating or full Actor
admission is claimed. The dedicated suffix test checks this guard.

Validation before the suffix fixture and later animation advancement edits:
%TEMP%/skate2-hilod-prefix-workspace.log passes3671units (3426core+245other) and
6doc tests; skate2-hilod-prefix-original.log passes17/17 original-data tests.
This includes mounted22 state checks and constructed NewChannel/PlayAnim trees.
It does not validate subsequent suffix-fixture, completion-guard or AddTime
edits. A fresh gate is running; strict Clippy/fmt are still required for this
batch. No per-tick reference trace or playable launch result is claimed.
