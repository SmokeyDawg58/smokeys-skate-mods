# Pedestrian waypoints and patrol, September 30

Source and original-asset review only. No compilation, tests, or game runtime.
Rustfmt parsed the waypoint, patrol, targeting, and physical-services files
with stdout output and skip_children; this does not establish type correctness.

The sole PropMan retains EntityMan256 with the shared entity map/KD kernel.
Root verified constructor82e67710, bucket prime257 and Init82e67788 with
80-byte KD blocks256. Prop storage follows LWPropMemoryMan.Init82e64740:
entity/core256, waypoint group255, conversation32, gathering8, actor tracker1,
point1020. Point allocation exhaustion skips that source point, preserving
its source index separately from the resulting native vector index.

The prior unpopulated physical-services waypoint GUID/chunk vectors had no
consumers. They are now typed maps referring to this sole Prop roster and
retained original waypoint assets. No second group or pedestrian roster exists.

WaypointData.Fixup829c6090 resolves record-relative arrays: header group
count0, groups offset12; group96 bytes; points48 bytes. Group centroid0,
global GUID48, LOCAL GUID56, entity RefSpec collection72, count80, points84.
WaypointGroup.GetGUID82e68e98 returns LOCAL GUID. Both stream notifications
and group lookup use this key. Pedestrian.NotifyOfWaypointDeRegister82e79ac8
uses a full-width cmpld, not the truncated comparison shown by decompiler.

WaypointGroup.SetData82e68b08 frees old point slots before assigning data,
clears centroid and vector, reserves source count, and constructs each
successfully allocated point from source location/orientation. Lock82e64c10
asserts unlocked; Unlock82b983b0 ignores caller identity. ClosestUnlocked
82e64a10 uses full XYZ squared distance and chooses the last equal-distance
entry. FirstUnlocked82e64b00 follows vector order.

The retained factory prefix implements Prop82e67150 then WaypointGroup
82e68d90. Actual original skater.vlt/bin inspection shows waypoint_patrol
entity RefSpec a81526f5cf7bd9c1 selects props ef4d99e1503b75a2, class
5cac37845e0c4722. Its UseBehaviourGraph cached byte20 is zero and optional
AI-enable9c946211beb7673c absent (zero default). Its actual no-graph constructor
therefore completes. Graph-bearing props stop explicitly at their missing
concrete loader and retain the partial allocations; parsed metadata is not
claimed to be a live behaviour controller. Model key must come from actual
census selection, never an invented zero. Construction does not publish.

Patrol Begin826fe358 switches navigation0, targets waypoint1 (including real
Seek), enables navigation, saves arrival radius, applies authored locomotion
attribute b90aaf6849740864, and unlocks current point. Update826ec230 sets
Auto then seek speed8. End826fe478 restores saved radius. PatrolExecution
belongs to each graph behaviour instance, not a shared entity-global payload.

LockFirstWaypoint.Begin82708020 and LockClosestWaypoint.Begin8270ad88 set
arrival radius0.2 before checking current pointer. The actual behaviour source
supplies the Prop ID. First selects vector entry zero only, even when later
points are free. Closest uses the shared actual group query. Successful
selection assigns the current pointer, locks with the pedestrian's actual ID,
then sets entity byte1a44 bit80. No iterator-index reset is invented.

Stream integration remains incomplete. RegisterWaypointData82719e60 inserts
the chunk first, checks actual factory callback!=default, calls SpawnProp
8271c430 using original recipe/model selection and load gates, then SetData
and inserts LOCALguid only if absent. attach_waypoint_data preserves the
last two operations. RecipeMan.select_model/model_asset_id already provide
the shared model selection kernels, but the Prop census/spawn/cull owner and
selected stream caller must still be connected. No synthetic publication was
added to bypass them. Existing selected arena waypoint registration remains
a caller limitation until that work is completed.

DeRegisterWaypointData82722088 removes the chunk before notifying every
pedestrian per authored group. The new typed notification helper traverses
the actual EntityMan40 bucket order and unlocks/clears matching current
pointers before source points can be freed. Native code then scans actual
Props, emits the optional removal message, removes GUID only when mapped to
that exact Prop, notifies remaining point lockers, clears group data, marks
the Props for culling, and runs PropCensus.Update. That final census/event
pipeline is still required; this change does not pretend removal completed.

The next implementation adds selected_living_world_props.rs under the real
SelectedRecipeFrame. SpawnProp8271c430 now uses its SAME RecipeMan for parent
model selection, original Model.Load_Model and ModelMan presence gates, and
live kMAXPROPS. It calls PropMan.request_spawn_waypoint, whose native
LWPropCensusMan.Spawn82e64f18 sets target transform, then ONLY current
translation/XZ through SetPropPosition82e66a38/SetPosition82e57770, then culling
and contained-prop initialization. Nonempty contained arrays explicitly need
the actual recursive census route; original patrol arrays are empty.
Completed objects enter the shared EntityMan first, then shared
ExecutionMan256 per AttachToManagers82e67a00. Unfinished objects remain owned.
Execution initialization must sample the actual newly registered Prop rates.

Prop execution rates30/10/3 are registered from82fa5050/50a8/5100; original
swapped NoFi/LoFi display labels retained. Patrol callback cvar82fa4ce0 is
true; MAXPROPS82f5d798 is255, range0..255. No fake default rate is supplied to
the execution owner. The cvar table additions occupy1113..1117.

EvaluativePropDrivenSpawnPedestrians827215f8 now has a concrete bounded
no-plugin path over the actual Prop KD tree: maximum256, original minimum
distance filtering, actual group/active/spawner/timer gates. Raw
CanTransferBehaviourPlugin82e653a8 reads IBehaviourSource+30 (=Prop+b4),
and returns false for the constructor's null transferable pointer BEFORE
DecideToSpawnPedestrian or probability RNG. Currently publishable no-graph
props therefore correctly produce no prop-driven pedestrians. A graph-bearing
candidate errors at its missing concrete plugin owner rather than being
silently omitted. This is not a fake empty-roster branch.

The normal arena dispatcher now exposes synchronous scoped waypoint callbacks
at the original operation index, preserving fault-prefix retention. A live
occurrence records whether waypoint callbacks ran; deregistration refuses to
drop that owner silently. Initial terrain, moving focus, and teleport all use
the same callback path. No deferred successful-registration queue is used.

RecipeMan construction is now before terrain: native ctor82727140 is called by
singleton Construct8272b0e4 from Sk8AppMan.InitApp82734ff4. The selected world
retains one Arc/Mutex owner, shared with LivingWorld and the later player owner.
Initialization uses the original Vault/cvars once; callbacks borrow that same
RecipeMan and retained Vault. This removes the former impossible dependency on
player startup, which follows initial terrain registration. PropMan Init now
captures actual factory callbacks and execution rates (82e69ac0); Prop spawn
retains the original shared CensusTargetCache entry before publication.

Register emits WaypointGroupAdded when streamed=true; Prop.OnCullRequested
82e65bf0 emits kind1004304572 through the retained Simulation GameSim queue.
Concrete FE MapBackEnd, ChallengeCharacter, and Lua subscriber ownership and
dispatch are still incomplete; see living-world-gamesim-events-2026-09-30.md.
Graph-bearing Prop construction and nonempty contained-prop recursive census
also remain explicit errors. Syntax parsing passed; no type check or runtime
was performed, and this does not establish working NPCs.


## Contained Props and reachable descriptor audit

InitContainedProps82e615a0 now executes recursive CensusMan SpawnProp through
one PropRecipeFrame/RecipeMan/PropMan. Source raw82e616a4..16c8 supplies the
same transform, initial=false, allowCull=false, force=true, worldCollision=true.
The current Prop spawn kernel has no force/world-collision-dependent branch;
those arguments are not replaced with fabricated transforms. Each typed RefSpec
is24 bytes, collection at+8. The native8-entry assertion and repeated length
reads are preserved. Child SetContainingEntity82e66b28 (folded82d15968) retains
the actual parent ID; successful children enter managers before the parent.
Failures retain both the failed child and any unfinished ancestor allocations.
Default no-graph Props now share the common constructor, without allocating a
WaypointGroup. Graph-bearing and specialized gathering/actortracker callbacks
remain explicit failures.

RequestExplicitCull recursively follows LWBehaviourSink.OnCullRequested82e4c500
and DeInitContainedProps82e57098: clear each child's containing entity, request
its cull before the parent event, detach through the existing managers, then
free the retained census list. Optional waypoint storage is destroyed only for
an actual WaypointGroup. No behaviour-source removal is fabricated: presently
admitted no-graph Props have never entered that source list.

Read-only original skater.vlt/bin audit confirms these authored bindings:

| Entity | Prop characteristics | Descriptor |
|---|---|---|
| waypoint_spectator |2924ca7eb1671f9a|spectate.xml|
| waypoint_sit |e25fa5ecc1166af1|sit.xml|
| waypoint_waterfountain |039fe06a97f0908f|usewaterfountain.xml|
| waypoint_vendingmachine |519852a53b91d8d9|usevendingmachine.xml|
| waypoint_usetrashbin |a5b3f3acd828805b|usetrashbin.xml|
| waypoint_newspaperbox |810d90d62f23a51d|usenewspaperbox.xml|
| waypoint_lookat |1e08eb95c66b442c|lookat.xml|
| waypoint_atm |31d6497ad6475cb9|useatm.xml|
| gathering_01 |412d658e75ea0d3d|gatheringplace_01.xml|
| actortracker_spectator |a56da08964fc985e|actortracker_spectate.xml|

All descriptors above are under livingworld/PluginDescriptor and have cached
UseBehaviourGraph byte20=true. waypoint_patrol ef4d99e1503b75a2 has false.
Optional separate UseAIGraph attribute9c946211beb7673c is absent on all these
collections. Thus the immediate reachable blocker is the transferable plugin
controller family, not a Prop-specific ordinary AI graph.

TransferableBehaviour.Process82cf0a40 asserts no previous controller, allocates
a HierarchicalController with zero24-byte context, sets controller-self and
root hash0, and Load(file,true). BehaviourMan.Load82cf6ee0 uses its shared
BehaviourGraph cache. Existing host TransferableBehaviourGraph only constructs
unprocessed metadata, and the sole current dynamic StateGraph cache contains
Camera payloads. Neither is an executable plugin controller. No metadata-only
wrapper has been published as a processed plugin. These missing generated
plugin controllers must be implemented before graph-bearing Prop admission.

Syntax-only rustfmt parsing passed. No compilation, tests, or game run.

## Gathering factory reachability audit (final source review)

The enabled `gathering_01` callback still requires its specialized constructor; it must not be aliased to WaypointGroup. Native `GatheringArea::GatheringArea82e69078` calls that base, allocates separate PropMan pool4 data and initializes its own state; its interface overrides include compatibility and gatherer-count behavior.

Read-only inspection of the original Jan17 assets finds no selection of this callback by the current freeroam content. `Attrib::StringHash64("gathering_01") = 31f4aab5e5090670` occurs exactly once in skater.vlt, at302168 (its own entity collection), and zero times in skater.bin. Thus there are no authored census/contained-Prop references or child collections that select it. All405 decompressed worldmisc.big members and920 worldbam.big members contain zero instances of this entity key. The BAM simulation entries provide positive controls:19 waypoint_sit keys and24 waypoint_usetrashbin keys, including cSim_-100_300_high.xsf and cSim_-100_900_high.xsf. Additional scans find zero gathering keys in156 worldproxy,580 livingworld and7494 missions archive members; mission members do contain8 waypoint_spectator references.

Dynamic DMO hotpoint synthesis cannot introduce this key either: native InitializeHotpoints82d38ec0 maps type0 to a hand grip, type1 to waypoint_usetrashbin, type2 to waypoint_newspaperbox, and every other nonzero type to waypoint_sit. The existing source uses that same mapping. Current host source has no direct gathering spawn call. Consequently the specialized gathering constructor is an uninstantiated optional path in the inspected freeroam corpus, not a newly found normal-freeroam blocker. No spawn exclusion or fallback constructor was added. No build/tests/runtime were performed.
