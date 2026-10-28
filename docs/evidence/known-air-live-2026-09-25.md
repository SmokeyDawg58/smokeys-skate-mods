# KnownAir and controller batch ? 2026-09-25

Status superseded by docs/evidence/known-air-trajectory-binding-2026-09-25.md.
The first runtime check exposed an omitted trajectory class; that binding is now
fixed and quick/high/spinning ollie release replays enter KnownAir and land.
The original implementation/compile record below is retained as history.
Builds use `build-release.bat`; controller tests are hands-on.
Rustfmt parsing/formatting passed on 46 touched Rust files. Static checks verified
533 cvar definitions and a complete, nonduplicated 533-entry registration order.
These checks are not a Rust typecheck or evidence of gameplay parity.

## Reported failures

| Report | Source-backed change |
| --- | --- |
| Motion state update2190429208 / 828f4c18 | RollingForwardsOrStationary1328, TailManual/Cycle ancestry; all36 packed update slots, once each. |
| Motion expression371 | Pre82917294 preserves repeated conditions51/50/48 and short circuit order; Sustain uses native true default829221c8. |
| Motion callback2173 / 825187c0 | Admit the existing ScoringGrabsUpdate callback implementation, alongside2174. |
| Motion expression154 | Pre8290ec0c negates condition225; Sustain uses native true default. |
| Motion callback643 / 82ca7628 | Extend existing FilterMotionGraphIntent lifecycle dispatch alongside642; reuse its actual filtering effects. |
| Motion expression363 | Pre8291682c and Sustain8292140c evaluate395 then396 with short circuit. |
| Terrain SelectedTriggerPending | Register retained trigger records in standard group0/1/2 octrees and the local-GUID map; camera and terrain now share the same retained manager. |

For1328 the callback sequence is:
`0,1,2,3,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,90,91,92,177,178,722,723,724,1413,1928,1929,1930,2176,2340,2341,2419`.
The native Motion descriptor table82195fa0 (stride32) only associates1328 with
828f4c18. Ancestry includes folded Random and Grabbing nodes; calling their
ancestors again would duplicate updates.

## KnownAir201, actual existing owner

The constructor/payload, trajectory selector, Reckoning, Skeleton.UpdateKnownAir,
PhysOut.Air, controller, and post-solve publication already existed. They are
reused. No replacement physical player, fake trajectory, tuning override, or
second camera was introduced.

New leaf consumers live in physical_player_known_air_trajectory.rs,
physical_player_known_air_rotation.rs and physical_player_known_air_fill.rs.
physical_player_known_air_state.rs connects them to the retained constructor
payload and live world. The preexisting physical_player_known_air.rs is the
shared Skeleton.UpdateKnownAir adapter used by both Air200's COM mode and201.

Source followed in IDA:

- Enter82d9f828: angular drive, air collision, truck tilt/IK, flipping and footplant
  resets, height/COM capture, trajectory initialization, closest COM index/error,
  and entry travel direction.
- InitTrajectoryInfo82d992a0: winning landing normal, apex, collision time and
  position, flip speed, original selector start-board position, finite-difference
  landing velocity and authored minimum heading speed.
- Update82d99820: selector changes, collision processing, flip/spin, alignment
  frames, trajectory following, shared Skeleton.UpdateKnownAir, head target,
  steering reset and height/time tracking.
- Follow82d8c078; closest-COM82db9b30/82db99f0: incremented fixed60Hz trajectory
  samples and fading entry error; retain first-distance-increase stopping rule.
- CalculateBodySpinSpeed82d8bbd0 and flipping82d84400: original attributes,
  fakie handling, angle graph, speed bounds and opposite-spin response.
- FillPhysOut82d8c348: real trajectory metadata, landing countdown/normal/heading,
  and25 COM-height samples measured along the landing normal (not world Y).
  GetLocalPosOfCOM82db6410 includes the orientation translation.
- PostPhysics82d99b78 and UpsideDownWipeout82d8c1a0: apex latch, actual COM-velocity
  bad-landing branch of the existing wipeout pipeline, upside-down request15.
- Exit82d99728 / RestoreVelocity82d8b928: flip reset, actual landing tangential
  velocity correction with authored slope graph, normal-speed preservation,
  processed local Z speed, selector and spin reset.
- StateChanger82ddc88c..82ddcaa8: known-air selection priorities, contact timing,
  expiry and loss-of-trajectory decisions. Published type201/category200 now
  reaches Update3, physics/controller, PostPhysics and Update5.

Supporting existing services extended rather than replaced:

- Reckoning.ResetFlipping82dd4080, StartFlipping82dd87d0 and UpdateBodyFlip82dd8410.
- HeadTracking.SetTarget82dca790 and Update82dca850 valid-target orientation:
  target frame, angular admission/clamp, quaternion/axis-angle and existing PID.
  Head target now retains optional W through SkeletonRootVector; the native
  calculation reads XYZ, so a missing scratch W is not fabricated or rejected.
- Ten native cvars registered using original defaults and initializer ordering.
- Air.CheckForAirWipeout's existing false/true argument now chooses deck/COM
  velocity in CheckForBadLandingWipeout82dd9528; other checks remain shared.

Raw source notes are in artifacts/knownair-complete-source-20260925.txt.
Do NOT use the earlier knownair-source-20260925.txt as complete: it is truncated.
Existing host numeric services retain their documented estimate/exceptional
boundaries; this work does not establish bit-identical console numerics.

## Terrain source and boundaries

AddTriggerVolume82e24810 routes type+216 (1 -> group1, 2 -> group2, otherwise0).
Group.Insert82e21aa8 / ManagedOctree.Insert82e21860 allocate an index, retain the
same record and insert its authored bounds before assigning the local-GUID map.
Manager.Init82e23500 initializes standard groups in0,1,2 order. The selected
retention policy never unloads these records, so no removed/free slots exist yet.
Capacity remains the original256 entries per tree.

Camera group collision testing is a SEPARATE still-pending boundary:
GetCollisions82e23da8 ->82e237d8 -> CollisionValid82e220b8 ->
IsColldingWithTriggerVolume82e21f70/82e213b8. Native uses the live kDoOBBTest cvar
and trigger CModel primitive intersection, then sorted indices and case-insensitive
name-prefix comparison in IsSubjectInVolume82e38460. The selected camera owner
still fails explicitly on a nonempty candidate query (`PendingCollision`); it
must not report every AABB candidate as an actual hit. Registration is implemented,
but this does NOT claim complete trigger-manager/camera-volume functionality.

## Remaining limits and first check

- Active FootPlant.UpdateFootplantAir is still explicit pending. The source reset
  path is implemented; active footplant launch/state601 is not claimed complete.
- Grind-targeted KnownAir adjustment remains parked with grinds.
- Enabled debug drawing remains subject to existing debug-overlay boundaries.
- Native terrain unloading/shared-arena ownership remains incomplete. This
  change does not repair every long-distance streaming/trigger/camera case.
- Intermittent pushing after turning has not been proved fixed by these changes.
- Added a head-target orientation regression and extended transition/trigger
  expectations. None were run in this handoff.

Compile with build-release.bat. First check a flat ollie/flip and landing, then an
immediate second crouch; compare prelanding leg extension and camera behavior.
Also retry the reported manual/gesture/grab controls and the streaming location.
A successful compile alone does not establish that these runtime paths complete.

## Compile follow-up

The handoff build failed. Compilation reproduced17
E0616 errors: KnownAir's world adapter was declared under physical_player_assembly,
but directly accessed the private backend/skaters fields owned by
SimulationAssemblyConstraintSession. Moved the single module declaration into
simulation_assembly_constraint_session.rs and re-exported it through the original
assembly path. This preserves existing callers and field privacy; no gameplay
math or source ordering changed.

`cargo build --release -p skate2-pc -j 1` then succeeded (exit0,4m35s), producing
`target/release/skate2-pc.exe`. Cargo reported224 core warnings and4 app warnings.
No game launch, runtime replay or controller validation was performed.
