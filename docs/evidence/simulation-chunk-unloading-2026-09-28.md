# Moving simulation collection unloading — September 28

Question: do visited chunks remain loaded forever? Audit found a real
simulation-side retention policy, not a missing visual unload implementation.

## Existing paths checked before editing

- `apps/skate2-pc/src/city_streaming.rs` transfers the existing scene to its
  resource worker. `move_runtime_presentation_selection` and
  `move_runtime_texture_selection` in `main.rs` already remove departed
  collections. Presentation dependencies are recomputed for the selected set.
- `render_inspection.rs` already prunes mesh, texture and material caches to
  each replacement generation. Submitted GPU work retains its own references.
- `proxy_world` already re-registers a proxy when its high collection leaves.
  The globally loaded low-detail proxy world is intentional, not a visited-chunk
  leak. These working renderer paths were left intact.
- `SelectedSessionTerrain` selected moving Sim focus correctly, but
  `SelectedTerrainStartup` only added collections. `SelectedResidentStartup`
  kept every hold and every requested ID forever. Its GC collaborator refused
  all calls. `SelectedSimulationAssetServices` also retained all processed
  wrappers/plans, independently of the terrain vectors.
- Existing resident streamer, normal/grind/environment deregistration and
  DMO arena adapter were reused. No second collision world or stream loader.

Read-only original-data check, using `tools/extract_big.py` functions over a
read-only mapping of `worldbam.big`: `data\content\world\stream\BAM\BAM_Sim.xsm`
contains **284 collections; all 284 parent IDs = 0xe171e1abfedb69ed** (the
source invalid-parent sentinel). No new parent dependency machinery is needed
for this shipped simulation map. Presentation has its own existing parents.

## Source checked in connected IDA

| Address | Evidence used |
| --- | --- |
| 8277ebb8 | SimThreadMsgMan.ReceiveDRegCollectionMsg: all DRegDMOArena calls first, then normal DRegArena in collection order, then location deregistration |
| 82d59a00 | NotifyCollectionSimDReg sends the simulation deregistration message before completion notification |
| 82d3d6e8 | LWDynamicObjectMan.RemoveStreamedLocatorData supplies force=false, allowPersistence=true |
| 82d3d618 | LocatorMan.RemoveStreamedLocatorData removes chunk membership after successful locator processing |
| 82d3c0c8 | RemoveLocatorsForStreamedLocatorChunk: flag0x80 forces cull; flag0x20 suppresses persistence; attached locators normally survive; passive persistence branch |
| 82d2a278 | RequestExplicitCullOfUserObject resolves the live object and requests its ordinary explicit cull |
| 82e22cb8 | RemoveTriggerVolume visits all three standard groups, then erases local GUID mapping |
| 82e23758 | Managed octree pointer removal searches for the exact record allocation |
| 82e232b0 | Clear slot, push reusable index, remove tree entry |
| 82acc210 | RW Octree.Remove unlinks the entry and frees only its empty owning node, not recursive ancestors |
| 82acc120 | DecPushCount decrement, including saturated255 recount before unlink |
| 82d557c8 | GarbageCollectAsset queues deadline releaseTick+3, capacity9000 |
| 82d5cc40 | UpdateGarbageCollector drains at most20 entries; deadline must be strictly less than releaseTick |

## Connected behavior

Terrain now tracks collection-to-arena ranges beside its existing ordered plan
and registration vectors. After an in-flight load completes, departed desired
focus members execute DMO deregistration first, normal deregistration second,
then release the resident load reference and occurrence-bound simulation hold.
Their IDs leave the active request inventory so revisiting can load them again.
The existing loader remains serialized; an already started request completes
before its now-obsolete registration is removed. Unloads precede the next load.

Trigger groups now release exact pointer entries and reuse LIFO free slots.
The existing RW octree gained in-place removal, preserving survivor query order;
host removal finds the owning node/chain rather than adding native pointer fields.

DMO removal uses the existing locator persistence map and census cull queue.
The existing explicit-cull body moved to DmoObjectMan so frame and streaming
callers share it. Ordinary unloading does not blindly destroy attached objects.
The cached streamed-locator set is updated from remaining chunk membership,
including IDs shared with a remaining chunk. Retired DMO data handles are removed
by exact backing identity; normal registration arrays and trigger admission
plan references are also released.

The resident garbage collaborator queues actual plan owners before registry
teardown. Matured entries release processed wrappers/plans by Arc identity,
so an old entry cannot erase a same-GUID reload. The selected host advances the
release clock once at its simulation-frame boundary, independently of focus
moves/teleports. No new normal runtime logging was added.

## Limits and validation

This connects the selected live host, not the entire native multithreaded asset
manager. Existing resident registry teardown removes inner/outer publication
atomically; physical plan/processed-wrapper lifetime now has delayed release.
Native cross-thread message timing and the global asset-manager GC clock are
not reproduced exactly by the host frame clock. The existing overlap refusal
for different collections decoding the same arena ID remains explicit.
Location-manager registrations are not added by this selected terrain owner,
so there is no matching location registration here to remove. Other existing
selected-profile exclusions remain unchanged.

Added regression cases for repeated octree entry/node reuse and survivor order;
passive versus attached DMO unload/reload; the native nonpersistent-locator
branch; delayed release versus same-ID reload; and the GC deadline/batch limit.
Updated the external-asset terrain corpus to require exact selected membership,
bounded processed-plan ownership after GC, and matching counts on return to spawn.

Scoped rustfmt syntax parsing passed for all edited Rust files. **No Cargo build,
check, test, shader validation or game launch was run. Tests are added, unexecuted.**
The read-only 284-entry original Sim collection-parent audit passed.

Validation: build-release.bat, then play-quiet.bat. Travel well beyond the
spawn focus into several city sections and back; check collision/grinds and
DMOs after revisiting. Move a DMO, leave its area and return to exercise the
existing persistence path with actual unloads. A flat memory reading is not
guaranteed: global proxies, the DMO model bank, active DMOs/persistence and
allocator high-water reservations intentionally outlive individual chunks.
