# Grounded hook positions: next source packet

2026-09-08 source audit and implementation following accepted17769. Author and
separate reviewer audited the complete bounded graphs. Code is now
being validated; no new hook-position checkpoint is accepted by this paragraph.

## Required caller and retained inputs

The current grounded root entry stops after UpdateRootTransforms82b5e490.
Native UpdateGroundSkateboard subsequently validates predicted/root XYZ,
composes the processed output transform and clears passed flags8c4 bit80000.
It calls the actual skateboard BODY Part6 getter at82b5e984 and copies the transform into
PhysicsSkateboardMtx+2fc0 and SkateRootTransform+3c80 at e998..e9d4. Only then
does it call GeneralUpdate at e9d8. Hook1 needs this retained physical transform.
The adjusted animation deck is not an equivalent producer.

GeneralUpdate[82b5ba88,82b5bc58) has SHA256
6f9035906e5fc80631410e91625af3b7369602c7bb4c51e6b110cfdb0e8f8693.
Its prefix computes full4 (AnimToWorld.P - LastAnimWorldPos+3db0), multiplied
by the two-refinement reciprocal of processed timestep+93c. It stores velocity
at+3da0/bb0c, then current translation at+3db0/bb14. Both retained vectors are
constructor-zero at573ac/b8. These commits precede hook placement.

## Hook placement and reset decision

UpdateHookPositions[82b4d380,82b4d600), SHA256
c94377d2ada951ca786f7e13e476b9cc97e5fcfa91625bc30db5361222689353.

- Hook0 transform is AnimToWorld(+2d80) outer * adjusted Hips(+2d00).
- Hook1 is retained SkateRootTransform(+3c80).
- AnimBoardToPhysicsBoard(+2ec0) is InverseSkateboard(+2f40) outer *
  (AnimToWorld outer * adjusted Skate(+2cc0)). All four columns commit before
  reading old hook0 transform atd56c.
- Set hook0 atd58c, hook1 atd5a4, then write proxy target Skeleton+2c80 from
  SkateRoot.P atd5b4.
- Finally return false if teleporting+3e44 or squared old/new hips distance>1.
  Rawd5e8=41990008 is ordered GT. NaN distance does not force reset; an ordered
  `distance <= 1` return test would change that behavior.

GeneralUpdate calls this helper atbb18. False immediately calls ResetSkeleton
atbb2c. Within that same failed-hook branch it then sets+3e42 when processed
state is neither0 nor2be; success skips both reset and this flag assignment.
State2be skips
UpdateIKData atbb64 and IK.Update atbb6c. All states reach Drives.Update atbb7c.
Existing actual hook-pool/Part setters can be reused. Current AnimToWorld W
uncertainty must remain explicit through the full4 cache and setters.

## Conditional reset is a complete operation

ResetSkeleton[82b45bf0,82b45dd4), SHA256
4105b4ae7a9094f55f033a43fb8bacbde2b8f26095077222a5f62ca95384dc64.
It first clears AddSkateboardError+3dd4. Then ascending24 parts compose
AnimToWorld with current mapped volumes and call actual Part::SetTransform.
Each body clears velocity/omega/torque XYZ and copies force XYZ from stasis+90,
preserving embedded W.

Only afterward SkeletonState::Reset82b3d238 rereads the actual24 Part transforms
into the physical transform and position arrays and zeros velocity. Its tail
zeros four animation COM/deck vectors and proxy target in source store order.
It preserves velocityDelta, physical COM/COMVel, resetScalar and landing state.
The existing narrow animation reset helper alone cannot stand in for this body
and history reset.

Smallest proposed complete scope: grounded suffix, retained GeneralUpdate
prefix, actual hook writes and conditional full reset, stopping explicitly at
unreconstructed UpdateIKData. Full IK solving was outside this read-only audit.
Recheck numerical/store ordering and all failure prefixes before implementation;
then independently review code and validate actual-body/corpus consumers.

## Full graph refinements and accepted implementation

Additional raw spans: grounded suffix[82b5e490,82b5e9d8), SHA256
0976a2065bb2532d3a311bf672b849728b34ac7878519cd5590d74fc0983def9;
General velocity[82b5ba88,82b5bb18), SHA256
6d5fd1e876fc046224876a52a18b6f7eb05cf3943281d96610178182ad5fd2d5.
StateReset[82b3d238,82b3d408), SHA256
01112c81d3b355c743f57bc0340cce6fc9ed0a304b3abfd1ecad1248be2bc82e.

Ground output is PASSED ProcessedPhysIn.mSkateboardOrientation+0, not deck+40.
It recomposes Translate(predictedP) outer * (raw AnimToBoard+2d40 outer *
current mapped0), not normalized AnimToWorld*mapped0. Both full4 composition
graphs execute, including explicit identity zero products. Three sequential
XYZ-only NaN assertion stages check predictedP, retained AnimToWorld, and final
output. Infinity passes each assertion unless arithmetic creates later NaN.
Old passed flags8c4 are read at e938; orientation X/Y/Z/P stores e93c/e940/e948/
e94c precede bit80000 clear e950 and board getter e984. A late board failure
retains output/flags but neither physical/skate-root copies nor velocity writes.

GeneralUpdate reads retained processed timestep+93c and current-state+8e8,
distinct from passed output and root-update timestep. Reciprocal-service failure
precedes both retained velocity/history commits. No dt-zero guard is invented.
After full GeneralUpdate, native resets NextTrajectoryTransform+2e80 at ea04..10;
the implemented prefix stops before IK/drives and must not execute that tail.

Hook cache X/Y/Z/P stores are d548/d54c/d554/d55c. The old-minus-new position
delta precedes hook0 set, then hook1/proxy writes. Teleport is read d5b8 but
dot3 at d5c8 still executes unconditionally afterward, even if teleporting.
Unknown delta W is not a dot3 input. Normalized AnimToWorld has known basis W
but potentially unknown P.W, inherited by hook0 and reset requests. The
animation-to-physics-board cache regains known W because its outer rigid inverse
is known and intermediate inner W is never an affine coefficient.

Reset motion stores are velocity45d58, omega45d64, stasis read45d68/70,
force45d78 and torque45d84. PDB stasis is the owning Simulation and its+90 is
current gravity. The Rust pool retains this immutable identity; it does not
provide a nullable/mutable stasis service. Narrow XYZ writes preserve links,
kinematic/cooldown, inertia, allocation, rings and RF rows. History reset checks
all XYZ columns before a row matrix copy, then validates/stores position and
zero velocity. It preserves physical proxy_position as well as physical COM,
COM velocity and velocity deltas. The separate proxy_target clear is between
animation COM-to-deck and animation COMdelta stores.

`SkeletonGeneralUpdate` owns three retained matrices, two vectors and existing
source flags (all ctor identity/zero/false; AddSkateboardError false573cc).
The normal owner's child adapters use the same current animation, roots,
physics history, drives-owned two-Part hook and existing proxy target. New
paired/world entries validate owner authority, invalidate collision publication
before possible Part movement, and retain all subsequent source prefixes.
Missing hook ownership fails lazily at the first actual hook read, after the
animation-to-physics cache commit. Reset completes before sticky WantRespawn.
Return states explicitly identify the pending UpdateIKData or UpdateDrives stage.

The actual Part cache now stores Known([16]) or Partial(typed matrix), with each
lane represented once. Raw cache iteration keeps every Part ordinal and returns
None for incomplete lanes. Typed attached setters use private scratch only for
the existing XYZ body math, then retain the original typed request; scratch W
never enters cache storage. Attached getters choose the actual body path before
examining cache completeness. Detach copies back actual body pose and restores
known cache before generic reattachment. Partial detached writes are explicitly
outside this attached-hook/reset seam. Existing attached-body canonical-W and
ordinary-finite numeric limitations remain; this is not recovered packed native W.

Independent review passed the General kernel/18 tests, reset owner/history/9
tests and shared cache/body changes. Three additional shared tests check typed
cache/body equivalence and refusal/detach behavior, plus exact narrow motion
stores with unchanged packed fields and RF. Full adapters and original-data
corpus also passed separate independent review.

Accepted **30738/56467d EXIT0**: 2,873 units (2,645 core, 187 formats,
38 app, three symbols), six docs, all nine original core corpus tests, fmt,
all-target Clippy with warnings denied and diff check pass. Thirty unit tests
were added beyond checkpoint 17769. Initial corpus compilation required the
existing read-only `observation.identity().id()` accessor; initial lint findings
were formatting, comments, a method reference and an unused lint expectation.
No source arithmetic changed during those fixes.

`physical_player_grounded_corpus` runs after the second post-records check on
the same solved board/skeleton and retained Vault Reckoning, before teardown.
Two deliberate component calls use root dt=0 and retained dt=1/8: the first
naturally resets; the unchanged second call accepts. Independent full-four-lane
consumer equations check passed output, velocity and cache, requested unknown
hook W, actual live hook XYZ, all 24 reset Part caches/body motion/history, and
preserved records/animation/IK data. Root outputs remain inputs to this consumer
oracle; existing root tests supply separate coverage. There is no extra solve
or native gameplay scheduler. Current-state 0x2be correctly reports Drives next;
other states report UpdateIKData. Neither continuation nor the later trajectory
tail is executed by this checkpoint.
