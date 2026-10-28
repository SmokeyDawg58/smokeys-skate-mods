# Pedestrian default-world ground/jump/taze probes

Source reviewed 2026-09-30. Confidence **high** for the local batch and geometry
translation; not a claim of live pedestrian integration or numeric trace parity.
No cargo, rustc, tests or runtime were launched.

## Native evidence

IDA symbols examined:

- `LWPedestrianExecutionMan::CreateGroundDMOLineTestBatch` `82e72b08`.
- `ProcessGroundDMOLineTestBatch` `82e72c50`.
- `ReceiveGroundDMOLineTestBatch` `82e72d70`.
- `Pedestrian::AddTestsToGroundDMOBatch` `82e789c8`.
- `LWPedestrianExecutionMan::Init` `82e9dc50`.
- `Pedestrian::ForcePositionToGroundPlane` `82e795e0`.
- `Pedestrian::IsIgnoreNavMeshInGroundTest` `82e94e80`.
- `NavRig::GetStartingJumpForwardDistance` `82e4d730`,
  `GetStartingJumpHeightDistance` `82e4d748`, `SetJumpDistance` `82e4d848`.
- `geotest::ComputeBarycentricY` `82e54878`.

Despite the GroundDMO name, Init acquires **one BF_Default=7 world batch per
buffer**, not five ground batches and not BF_GroundDMO=3. Raw instruction
`82e9dd98: li r5,7` confirms the enum. The existing WorldLineGroup retains
those leases; no new collision registry, spatial index, entity collection or
DMO owner is introduced. The native hookup capacity is80, independent of the
single world's line-test capacity. A rejected AddTest marks that request
unsubmitted and prevents later submissions and visits to later entities.

AddTests emits in this order:

1. Positioning(type4), always. Select up distance from
   kGroundPlaneTestDistanceUpKnockDown when state byte3241 bit04 is set,
   otherwise kGroundPlaneTestDistanceUpDefault. Save it into
   mPositioningTestStartHeight even when hookup capacity is exhausted.
   Begin from LWEntityCoreData+128. Unless pedestrian byte6724 bit08 says
   ignore navmesh, borrow NavRig::mMeshInfo; when valid and absolute centroid
   Y minus GetPosition Y is strictly below downDistance+upDistance, replace
   base Y by ComputeBarycentricY if valid. Start=base+Y*up, end=base-Y*down,
   zero fatness. The shared MeshCell::height_at already implements that helper.
2. Jump(type3), only when IsAbleToJump and pedestrian byte6724 bit80.
   Forward start=position+Y*(forwardFatness+forwardHeight)
   +facing*testStartOffset. End=start+facing*forwardDistance. Initial miss
   distance is the forwardDistance cvar, not a recomputed endpoint length.
   Height start=position+Y*heightHeight+facing*heightLead;
   end=start-(Y*heightHeight)*2. Initial miss distance is heightDistance,
   independently of the endpoint extent. Ray IDs0 then1.
3. Taze(type5), only when state byte3244 bit80 and TazerState_Cyc, the
   wants-table result target ID is valid and resolves to ITargetable. Start
   is GetAnimData()+25984 plus facing*kTazeLineStartOffset; end is the actual
   target's GetTargetableCentreOfMass, zero fatness.

All cvars remain caller-provided runtime values. No corpus tuning is hardcoded.
Miss position is endpoint; miss normal is world up. Process calls Sync then
GetResult for each hookup index, copying actual hit position/normal/surface/
owner and computing distance from start with shared native numeric operations.
The typed existing SimulationDmoAssemblyHandle is retained, not converted into
an invented integer ID. Taze's native miss surface/owner are not initialized in
AddTests; the host initializes them but the taze consumer does not read them.

Receive only considers submitted requests, **including submitted misses**:

- Jump writes NavRig jumpDistances[ray] directly; SetJumpDistance asserts ray<2.
- Positioning resolves the original pedestrian ID through the existing manager
  and calls ForcePositionToGroundPlane(hitPosition, hitNormal, surface,
  hitOwner==null). Missing pedestrian IDs are skipped. A miss has null owner
  and still calls this callback; it must not be turned into a no-op.
- Taze resolves the same ID and writes state byte3244 bit40 when **none** of
  the XYZ endpoint/hit-position absolute differences exceeds EPSILON_192.
  `vcmpgtfp128.`82e72e9c writes CR6, `mfocrf r10,2; srwi r9,r10,5`
  extracts CR6 EQ (all comparisons false); `insrwi` writes the flag. Thus
  this is a clear-line flag, the inverse of taze_obstructed(). The epsilon's
  raw float at823953d0 is34000000; W is excluded by Vector3 comparison.

ForcePositionToGroundPlane does not immediately teleport the pedestrian. It
validates finite/bounded vectors, flips a downward normal, tests the actual
MeshStore cell score<800 using kDefaultNavMeshCheckYTolerance when owner is null
and surface water mask `(surface & 0xf80)==0x300` is false, then updates the
existing SpatialAdjust ground normal/position/surface, valid-ground flag,
last-good position and conditional water/offmesh flag. Its implementation and
frame consumer must remain with the retained spatial-adjust owner.

## Integration contract and remaining work

New module `pedestrian_ground_dmo_batches.rs` is intentionally not registered
by this delegated change; the root integrator owns shared modules. Register it
beside pedestrian_ground_batches, retain one instance in the execution owner,
initialize/release with the existing selected world query owner, begin/add/end
in native preframe order, and swap/process in current-frame order before
receiving through EntityMan40. No newly manufactured roster is allowed.

Required facts are explicit GroundDmoProbeFacts inputs. In particular,
`mesh_cell` is **NavRig::mMeshInfo**, not MeshNavigation's seek/target cell.
Existing NavRig lacks this field at the time of this change: recover its native
producer before wiring it. Caller must save positioning_start_height on the
entity each visited frame. Jump and taze eligibility, animation origin and
target centre must come from their real retained owners. No false default may
silently suppress those branches. The separate GroundDmoTuning struct names
all required cvars for loading through the existing LW configuration owner.

Results expose GroundDmoTest, pedestrian_id, submitted and native hit fields;
the root receiver must perform the three dispatches described above. The
`taze_obstructed()` helper implements the source comparison only; it does not
own security state. SpatialAdjust implementation, entity manager delivery,
loop scheduling, module registration, native ownership lifetime verification
and compiler/runtime validation are integration obligations, not completed by
this file alone. Source inspection and diff-whitespace check were performed;
there is no build/test success claim.

## Follow-up: native current mesh-cell producer

Recovered `Task_UpdatePositionalData82e72fd8`,
`Pedestrian::SetPathogenCell82e6e190`, `ReleaseNavMeshTile82e6e0f8` and the
GetPathogenCell/GetLastValidPathogenCell/GetLastValidPathogenCellPosition
accessors82e94e10/20/30. New child `pedestrian_nav_rig_mesh.rs` supplies
NavRigMeshInfo and NavRig::update_mesh_cell, using the existing MeshStore and
weak streamed MeshCell identities. This supersedes the missing-producer
limitation above; registration and calling remain root integration work.

Task_UpdatePositionalData reads GetPosition, queries the real MeshStore using
runtime kDefaultNavMeshCheckYTolerance, calls SetPathogenCell and then the
existing LWBasicEntity positional-data callback (the latter remains outside
this helper). On a cell/handle identity change only, SetPathogenCell fetches
the current **MeshNavigation** target iterator's cost threshold, or zero if
there is no iterator. It computes new and previous cell scores with that same
threshold, copies current cell into previous, installs new current cell,
updates last-valid cell/position only if new cell is valid, and records the
current transform translation as previous-cell position. Even an unchanged
cell is assigned again at function end. Two absent cells compare equal;
the existing MeshCell::same intentionally reports only present matches, so
the helper explicitly handles the null/null case.

IDA type layout: NavRig::mPosition+16, directions+32, distances+144/+172,
jump distances+228, mMeshInfo+240, current group+304, batch indices+308/+312,
cache counter+316, cache bool+320, do-jump bool+321. MeshInfo offsets are
current cell0, previous cell8, previous-cell-position16, cell score32,
previous score36, last-valid cell40, last-valid-position48, total64 bytes.
Ctor82e51870 nulls the three cell/handle pairs in otherwise cleared storage.
ReleaseNavMeshTile clears matching current and previous handle pairs only;
last-valid and scores/positions remain retained. Caller must separately
forward the native release to MeshNavigation, as the original does.

Required registration in pedestrian_navigation_rig.rs:
`#[path = "pedestrian_nav_rig_mesh.rs"] pub(crate) mod mesh;`, field
`pub(crate) mesh_info: mesh::NavRigMeshInfo`, initialized by
`mesh::NavRigMeshInfo::new()`. GroundDMO borrows `&rig.mesh_info.cell`.

NavRig::mPosition is **not** either MeshInfo position field. It is read by
TransformRig physics-cache distance and GetNavRigDistance. Decompiled
TransformRig82e51f38, AdjustNavRigVerticalAngleToNavMesh82e58720,
Pedestrian Transform/Validate/Orient callbacks, SetPathogenCell,
TargetProvider path/physics validation and all named Pedestrian methods
were inspected; none establishes a verified mPosition assignment. The new
helper deliberately does not manufacture one. An inlined/untyped writer
through GetNavRig or a native retained-zero behavior remains to be proved.
No source-parity claim should depend on an invented every-frame assignment.

## Follow-up: entity result application

New entity child `pedestrian_ground_dmo_receive.rs` adds
PedestrianConstruction::receive_ground_dmo_result. It ignores unsubmitted
requests; directly writes existing NavRig.distances.jump[ray] for jump;
calls the existing entity's SpatialAdjust::force_position_to_ground_plane
for positioning (including misses); and updates only state.flags[4] bit40
for taze using the **all-false** comparison condition described above.
Root registers it under living_world_pedestrian_entity.rs and supplies the
current actual MeshStore and kDefaultNavMeshCheckYTolerance.

Positioning/taze missing-ID skip belongs to the manager caller. Native jump
entries retain a NavRig pointer and do not perform an ID lookup, so their
owning entity must survive until receive; a missing jump owner is not
equivalent to the positioning/taze absent-ID skip. No parallel roster or
unsafe pointer is added to emulate this lifetime. The root execution owner
must retain the existing pedestrian's lifetime until all requests complete.
