# Pedestrian navigation is carried by the Presentation stream

The release diagnostic build completed successfully (pedestrian-nav-build.log).
The 2400-completion offboard-city-walk capture exited 0, but every sampled
MeshStore query reported tiles=0 and presentation remained zero pedestrians.
Resident recipe candidates were rejected by the native absent-cell spawn gate.
See artifacts/pedestrian-nav-run.log. This is not successful pedestrian playback.

The existing navigation audit only scanned BAM_Sim. That entire stream has
no NavMesh2 records. BAM_Pres contains 101 NavMesh2 records with 133326 cells
(original audit scan; payload-window-corrected scan saved separately in
artifacts/pedestrian-pres-nav-inventory.log). The audit now accepts Sim/Pres
and uses each CMAP payload offset rather than the SFIL first-block offset.

IDA confirms:

- cProcessArenaAsset constructor 82d60830..85c registers ProcessNavMesh2Data
  for raw type 00eb0022. The host type constant is correct.
- ProcessNavMesh2Data82d5f230 calls AddSimRegistryAsset with kind23, even when
  the containing collection is a Presentation stream collection.
- NotifyCollectionReg82d5c920 calls NotifyCollectionSimReg82d5c950 before
  NotifyCollectionPresReg82d5c96c. Overload82d5c8c0 has the same dual dispatch.
- AddNavMeshTile82e63968 resolves the simulation registry asset before
  inserting the tile and linking its neighbours. Keep this existing owner.
- Presentation::RegArena827f1430 does not handle kind23; merely adding nav
  records to the visual presentation owner cannot register navigation.

Current missing host connection:
apps/skate2-pc/src/main.rs load_presentation_collection decodes every arena
then calls own_presentation_arena, retaining visual data only. CityStreaming
documents that full processed Presentation registration is absent. Meanwhile
selected_terrain_startup registers normal Simulation plans only for Sim
collections. The navigation MeshStore and its asset service are shared
correctly, but no Presentation-stream nav records reach them.

Next implementation must retain/process the actual decoded navigation arenas,
publish their inner assets to the SAME simulation registry, then register
kind23 in native order. Deregister on the matching presentation collection's
departure before releasing its backing assets. Reuse normal plans, existing
registry publication/occurrence identity, and MeshStore add/remove; do not add
another nav owner, independently reload all city tiles, or force spawn.

Initial registration bridge compiled successfully in release (7m19s): the presentation
collection retains an Arc-backed normal plan only for decoded NavMesh2 arenas,
using original XST metadata. Before each live tick the installed collections
synchronize their nav assets into the existing registry and MeshStore; matching
departures remove tiles before unpublishing the exact inner wrapper. Normal
play does not reload/decode tiles independently or change spawn eligibility.
Presentation loading now honors CMAP payload offsets with CollectionPayload.

Follow-up source check: MeshMan::RemoveNavMeshData82ce6ac0 calls the pedestrian
manager's ReleaseNavMeshTile before MeshStore removal. The existing host rig
release helper is not wired, and MeshNavigation path-release needs connecting.
ReleaseNavMeshTile82e6c060 scans path edges/cells for the exact tile; on a match
it clears validity/counts/start/end and sets should-AStar. Complete this before
claiming moving-world navigation lifecycle parity. This hook is now implemented
in the follow-up build: shared rig release, MeshNavigation path reset, then
MeshStore removal, then exact inner-wrapper unpublication. Original manager
82e84cc8 iterates all retained pedestrians and entity82e6e0f8 calls both helpers.

First live result: artifacts/pedestrian-pres-nav-run.log reports 10 registered
tiles and valid cost-zero cells. At completion784 a resident candidate passed
the nav gate, then failed `pedestrian SimLOD bone assertion: Head` before roster
publication. The input was offboard-city-walk, target1600; exit1, no screenshot.

IDA InitSimLODBoneLookupIDs82e8d650 uses Head, RightForeArm/Hand/HandProp,
LeftForeArm/Hand/HandProp, Hips, Spine1. GetBone82590fc0 compares five packed
FastString words. Host erroneously compared capitalized text against decoded
lowercase display names. Fixed both SimLodPose lookup and PostAdjust's matching
bone lookup to use existing PackedFastString<5>::encode and packed_bone_names.
No alternate skeleton, missing-bone fallback, or spawn bypass was added.

Follow-up release build artifacts/pedestrian-pres-navigation-bones-build.log
passed (7m01s). Capture pedestrian-pres-bones-run.log passed the bone lookup,
then stopped at completion772 on constructed Members d5e1267e2d715124.
DoReverseAttribCategoryLookup82e7f4e0 reads the same RefSpec Members array
already handled by the census. The factory now reuses try_reference_members
instead of the generic POD field projection; retains original leaf/member order,
full64 key match and later-match overwrite. No new generic constructed bypass.
Release rebuild artifacts/pedestrian-category-build.log passed. Capture
pedestrian-category-run.log reached completion741, passed category lookup and
then failed on hand-prop usage864a7084ca220523, exactly the pending audited type.

Read-only follow-up audit found another upcoming constructed projection:
hand-prop usage type d6778895b505d1db stride56 flags25, key864a7084ca220523.
Its vtable82167afc Construct/Destruct slots are both no-op825187c0;
Assign828b56a0 copies RefSpec0, floats24/28, RefSpec32. Existing usage consumer
reads class/key0/8,24/28 floats, class/key32/40. Typed-field allowance for just
these fields is now implemented (caches16/48 and padding remain rejected).
Extended the existing constructed-projection test to cover the second RefSpec
and reject both caches; not yet run. Build pedestrian-handprop-build.log passed.
The runtime capture passed hand-prop construction and reached four presentation
entities/eight meshes at frame900, then failed at924 on initial global pose.
See pedestrian-initial-global-pose-2026-10-01.md for the source-backed pool-zero
initialization correction and remaining runtime verification.
Other examined post-adjust/takedown/hand-prop action arrays have flags5 (POD).
Shared namespace/duplicate exact-removal regression added to
selected_simulation_registry_bridge_tests.rs, not yet run. Still need visible,
moving and collidable pedestrian runtime proof and map visual checks.

Map arrow/wide-background
changes and the preference typed-field fix remain in source; actual visible,
moving, collidable pedestrians and map visual parity remain unverified.
